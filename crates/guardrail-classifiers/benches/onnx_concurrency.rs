//! Throughput and latency of an ONNX stage under concurrent requests, for
//! several session-pool sizes.
//!
//! ```text
//! GUARDRAIL_BENCH_MODEL_DIR=/path/to/prompt-injection \
//!   cargo bench -p guardrail-classifiers --features onnx --bench onnx_concurrency
//! ```
//!
//! The directory must hold `model.onnx` and `tokenizer.json` for one model.
//! Without it the tiny test fixture is used, which only checks that the
//! harness runs. Optional:
//!
//! - `GUARDRAIL_BENCH_STAGE`: `injection` (default) or `toxicity`.
//! - `GUARDRAIL_BENCH_SESSIONS` (e.g. `1,2,4`), `GUARDRAIL_BENCH_INTRA_THREADS`
//!   (e.g. `1,4`, default `1`) and `GUARDRAIL_BENCH_CONCURRENCY` (e.g.
//!   `1,4,16`). Every combination of sessions and intra threads is loaded in
//!   turn and run at each concurrency.
//! - `GUARDRAIL_BENCH_REQUESTS`: requests per row (default 64).
//! - `GUARDRAIL_BENCH_PROMPT_BYTES`: prompt length (default 1024), or
//!   `GUARDRAIL_BENCH_WINDOWS`: a prompt that is exactly this many windows,
//!   each one full, built with the model's own tokenizer. The run fails if
//!   the stage counts a different number of windows.
//!
//! Before measuring, each pool is checked to score the bench request (so a
//! stage that ignores its role can't report a fast no-op), and its window
//! count is taken from the stage itself.
//!
//! Memory columns (Linux only): RSS right after the pool is loaded, and the
//! peak RSS (`VmHWM`) from the start of that load to the end of the row.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use guardrail_classifiers::onnx::{
    OnnxRuntimeOptions, MAX_WINDOWS, MAX_WINDOW_TOKENS, WINDOW_STRIDE_TOKENS,
};
use guardrail_classifiers::{OnnxInjectionClassifier, ToxicityClassifier};
use guardrail_core::pipeline::Stage;
use guardrail_core::request::{ChatMessage, GuardrailRequest, MessageContent, Provider, Role};

const SENTENCE: &str = "Please summarize the attached quarterly report for the team. ";

fn env_list(name: &str, default: &[usize]) -> Vec<usize> {
    std::env::var(name)
        .ok()
        .map(|v| v.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_else(|| default.to_vec())
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}

/// A field of `/proc/self/status`, in MiB (Linux only).
fn status_mib(field: &str) -> Option<f64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let kb: f64 = status
        .lines()
        .find(|l| l.starts_with(field))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()?;
    Some(kb / 1024.0)
}

fn mib(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".into(), |v| format!("{v:.0}"))
}

/// Restart the peak-RSS counter (`VmHWM`) from the current RSS (Linux only).
fn reset_peak_rss() {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[i]
}

fn request(text: &str) -> GuardrailRequest {
    GuardrailRequest::new(
        vec![ChatMessage {
            role: Role::User,
            content: MessageContent::Text(text.into()),
        }],
        "bench".into(),
        Provider::OpenAI,
    )
}

#[derive(Clone, Copy)]
enum StageKind {
    Injection,
    Toxicity,
}

impl StageKind {
    fn from_env() -> Self {
        match std::env::var("GUARDRAIL_BENCH_STAGE").as_deref() {
            Err(_) | Ok("injection") => Self::Injection,
            Ok("toxicity") => Self::Toxicity,
            Ok(other) => panic!("GUARDRAIL_BENCH_STAGE={other}: expected injection or toxicity"),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Injection => "injection",
            Self::Toxicity => "toxicity",
        }
    }
}

/// A stage with its default settings, before it is shared between tasks.
enum Loaded {
    Injection(OnnxInjectionClassifier),
    Toxicity(ToxicityClassifier),
}

impl Loaded {
    fn load(kind: StageKind, model: &Path, tokenizer: &Path, options: OnnxRuntimeOptions) -> Self {
        match kind {
            StageKind::Injection => Self::Injection(
                OnnxInjectionClassifier::load_with_options(model, tokenizer, 0.85, options)
                    .expect("model loads"),
            ),
            StageKind::Toxicity => Self::Toxicity(
                ToxicityClassifier::load_with_options(model, tokenizer, 0.9, options)
                    .expect("model loads"),
            ),
        }
    }

    fn with_max_windows(self, max_windows: usize) -> Self {
        match self {
            Self::Injection(s) => Self::Injection(s.with_max_windows(max_windows)),
            Self::Toxicity(s) => Self::Toxicity(s.with_max_windows(max_windows)),
        }
    }

    fn as_stage(&self) -> &dyn Stage {
        match self {
            Self::Injection(s) => s,
            Self::Toxicity(s) => s,
        }
    }

    fn into_stage(self) -> Arc<dyn Stage> {
        match self {
            Self::Injection(s) => Arc::new(s),
            Self::Toxicity(s) => Arc::new(s),
        }
    }
}

/// The tokenizer from `path`, splitting into windows the way the stages do.
fn window_tokenizer(path: &Path) -> tokenizers::Tokenizer {
    let mut tokenizer = tokenizers::Tokenizer::from_file(path).expect("tokenizer loads");
    tokenizer
        .with_truncation(Some(tokenizers::TruncationParams {
            max_length: MAX_WINDOW_TOKENS,
            stride: WINDOW_STRIDE_TOKENS,
            strategy: tokenizers::TruncationStrategy::LongestFirst,
            direction: tokenizers::TruncationDirection::Right,
        }))
        .expect("truncation is valid");
    tokenizer
}

fn count_windows(tokenizer: &tokenizers::Tokenizer, text: &str) -> usize {
    1 + tokenizer
        .encode(text, true)
        .expect("prompt tokenizes")
        .get_overflowing()
        .len()
}

/// The longest repetition of [`SENTENCE`] that is `windows` windows, so
/// that every window is full.
fn prompt_with_windows(tokenizer: &tokenizers::Tokenizer, windows: usize) -> String {
    let count = |n: usize| count_windows(tokenizer, &SENTENCE.repeat(n));
    // Invariant: count(lo) <= windows < count(hi).
    let mut lo = 0;
    let mut hi = 1;
    while count(hi) <= windows {
        lo = hi;
        hi *= 2;
    }
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if count(mid) <= windows {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let got = count(lo);
    assert_eq!(
        got, windows,
        "no repetition of the bench sentence is exactly {windows} windows"
    );
    SENTENCE.repeat(lo)
}

/// Windows the stage itself counts for `req`: it is evaluated with a limit
/// of one window, which fails before inference with the actual count.
async fn stage_windows(stage: &dyn Stage, req: &GuardrailRequest) -> usize {
    match stage.evaluate(req).await {
        Ok(_) => 1,
        Err(e) => {
            e.input_too_long()
                .unwrap_or_else(|| panic!("stage failed: {e}"))
                .size
        }
    }
}

fn main() {
    let kind = StageKind::from_env();
    let (model, tokenizer, real) = match std::env::var("GUARDRAIL_BENCH_MODEL_DIR") {
        Ok(dir) => {
            let dir = PathBuf::from(dir);
            (dir.join("model.onnx"), dir.join("tokenizer.json"), true)
        }
        Err(_) => {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/onnx");
            (
                dir.join("keyword_2class.onnx"),
                dir.join("tokenizer.json"),
                false,
            )
        }
    };
    let sessions = env_list("GUARDRAIL_BENCH_SESSIONS", &[1, 2, 4]);
    let intra_threads = env_list("GUARDRAIL_BENCH_INTRA_THREADS", &[1]);
    let concurrency = env_list("GUARDRAIL_BENCH_CONCURRENCY", &[1, 4, 16]);
    let requests = env_usize("GUARDRAIL_BENCH_REQUESTS").unwrap_or(64);
    let target_windows = env_usize("GUARDRAIL_BENCH_WINDOWS");

    let window_tokenizer = window_tokenizer(&tokenizer);
    let prompt = match target_windows {
        Some(windows) => prompt_with_windows(&window_tokenizer, windows.max(1)),
        None => {
            let bytes = env_usize("GUARDRAIL_BENCH_PROMPT_BYTES").unwrap_or(1024);
            SENTENCE.repeat(bytes / SENTENCE.len() + 1)[..bytes].into()
        }
    };
    let windows = count_windows(&window_tokenizer, &prompt);
    let max_windows = MAX_WINDOWS.max(target_windows.unwrap_or(0));
    assert!(
        windows <= max_windows,
        "the prompt is {windows} windows, over the stage's limit of {max_windows}; \
         it would be resolved by on_oversize instead of scored"
    );
    // Clearly more than one window, to check the stage scores this request.
    let probe = request(&SENTENCE.repeat(MAX_WINDOW_TOKENS));
    let req = request(&prompt);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    println!(
        "stage: {}, model: {}, prompt: {} B, windows: {windows}, {requests} requests per run",
        kind.name(),
        if real { "real" } else { "test fixture" },
        prompt.len(),
    );
    println!(
        "| sessions | intra_threads | concurrency | req/s | p50 (ms) | p99 (ms) \
         | RSS after load (MiB) | peak RSS (MiB) |"
    );
    println!(
        "|---------:|--------------:|------------:|------:|---------:|---------:\
         |---------------------:|---------------:|"
    );

    for &n_intra in &intra_threads {
        for &n_sessions in &sessions {
            reset_peak_rss();
            let options = OnnxRuntimeOptions {
                sessions: n_sessions,
                intra_threads: n_intra,
            };
            let loaded = Loaded::load(kind, &model, &tokenizer, options).with_max_windows(1);
            let rss = mib(status_mib("VmRSS:"));

            let stage_count = runtime.block_on(async {
                let checked = loaded.as_stage();
                assert!(
                    checked
                        .evaluate(&probe)
                        .await
                        .is_err_and(|e| e.input_too_long().is_some()),
                    "the stage did not score the bench request's role"
                );
                stage_windows(checked, &req).await
            });
            assert_eq!(
                stage_count, windows,
                "the stage counts {stage_count} windows, the bench tokenizer {windows}"
            );
            if let Some(target) = target_windows {
                assert_eq!(stage_count, target, "the prompt is not {target} windows");
            }
            let stage = loaded.with_max_windows(max_windows).into_stage();

            // Warm up every session.
            runtime.block_on(async {
                let warm: Vec<_> = (0..n_sessions)
                    .map(|_| {
                        let stage = stage.clone();
                        let req = req.clone();
                        tokio::spawn(async move { stage.evaluate(&req).await })
                    })
                    .collect();
                for w in warm {
                    w.await.unwrap().unwrap();
                }
            });

            for &workers in &concurrency {
                let (elapsed, mut latencies) = runtime.block_on(async {
                    let next = Arc::new(std::sync::atomic::AtomicUsize::new(0));
                    let start = Instant::now();
                    let tasks: Vec<_> = (0..workers)
                        .map(|_| {
                            let stage = stage.clone();
                            let next = next.clone();
                            let req = req.clone();
                            tokio::spawn(async move {
                                let mut lat = Vec::new();
                                while next.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                                    < requests
                                {
                                    let t = Instant::now();
                                    stage.evaluate(&req).await.unwrap();
                                    lat.push(t.elapsed());
                                }
                                lat
                            })
                        })
                        .collect();
                    let mut all = Vec::new();
                    for t in tasks {
                        all.extend(t.await.unwrap());
                    }
                    (start.elapsed(), all)
                });
                latencies.sort();
                println!(
                    "| {n_sessions} | {n_intra} | {workers} | {:.1} | {:.0} | {:.0} | {rss} | {} |",
                    requests as f64 / elapsed.as_secs_f64(),
                    percentile(&latencies, 0.50).as_secs_f64() * 1000.0,
                    percentile(&latencies, 0.99).as_secs_f64() * 1000.0,
                    mib(status_mib("VmHWM:")),
                );
            }
            drop(stage);
        }
    }
}
