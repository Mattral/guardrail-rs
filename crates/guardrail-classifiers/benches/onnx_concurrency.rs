//! Throughput and latency of the ONNX injection stage under concurrent
//! requests, for several session-pool sizes.
//!
//! ```text
//! GUARDRAIL_BENCH_MODEL_DIR=/path/to/prompt-injection \
//!   cargo bench -p guardrail-classifiers --features onnx --bench onnx_concurrency
//! ```
//!
//! The directory must hold `model.onnx` and `tokenizer.json`. Without it the
//! tiny test fixture is used, which only checks that the harness runs.
//! Optional: `GUARDRAIL_BENCH_SESSIONS` (e.g. `1,2,4`),
//! `GUARDRAIL_BENCH_CONCURRENCY` (e.g. `1,4,16`), `GUARDRAIL_BENCH_REQUESTS`
//! (requests per run, default 64) and `GUARDRAIL_BENCH_PROMPT_BYTES`
//! (default 1024).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use guardrail_classifiers::onnx::OnnxRuntimeOptions;
use guardrail_classifiers::OnnxInjectionClassifier;
use guardrail_core::pipeline::Stage;
use guardrail_core::request::{ChatMessage, GuardrailRequest, MessageContent, Provider, Role};

fn env_list(name: &str, default: &[usize]) -> Vec<usize> {
    std::env::var(name)
        .ok()
        .map(|v| v.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_else(|| default.to_vec())
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Resident memory of this process, in MiB (Linux only).
fn rss_mib() -> Option<f64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let kb: f64 = status
        .lines()
        .find(|l| l.starts_with("VmRSS:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()?;
    Some(kb / 1024.0)
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

fn main() {
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
    let concurrency = env_list("GUARDRAIL_BENCH_CONCURRENCY", &[1, 4, 16]);
    let requests = env_usize("GUARDRAIL_BENCH_REQUESTS", 64);
    let prompt_bytes = env_usize("GUARDRAIL_BENCH_PROMPT_BYTES", 1024);

    let sentence = "Please summarize the attached quarterly report for the team. ";
    let prompt: String = sentence.repeat(prompt_bytes / sentence.len() + 1)[..prompt_bytes].into();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    println!(
        "model: {}, prompt: {prompt_bytes} B, {requests} requests per run",
        if real { "real" } else { "test fixture" }
    );
    println!("| sessions | concurrency | req/s | p50 (ms) | p99 (ms) | RSS after load (MiB) |");
    println!("|---------:|------------:|------:|---------:|---------:|---------------------:|");

    for &n_sessions in &sessions {
        let before = rss_mib();
        let stage = Arc::new(
            OnnxInjectionClassifier::load_with_options(
                &model,
                &tokenizer,
                0.85,
                OnnxRuntimeOptions {
                    sessions: n_sessions,
                    intra_threads: 1,
                },
            )
            .expect("model loads"),
        );
        let loaded = rss_mib();
        let rss = match (before, loaded) {
            (Some(_), Some(after)) => format!("{after:.0}"),
            _ => "n/a".into(),
        };

        // Warm up every session.
        runtime.block_on(async {
            let warm: Vec<_> = (0..n_sessions)
                .map(|_| {
                    let stage = stage.clone();
                    let req = request(&prompt);
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
                        let req = request(&prompt);
                        tokio::spawn(async move {
                            let mut lat = Vec::new();
                            while next.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < requests
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
                "| {n_sessions} | {workers} | {:.1} | {:.0} | {:.0} | {rss} |",
                requests as f64 / elapsed.as_secs_f64(),
                percentile(&latencies, 0.50).as_secs_f64() * 1000.0,
                percentile(&latencies, 0.99).as_secs_f64() * 1000.0,
            );
        }
        drop(stage);
    }
}
