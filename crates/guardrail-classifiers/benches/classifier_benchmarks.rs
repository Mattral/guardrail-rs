//! Criterion benchmarks for `guardrail-classifiers`.
//!
//! Run with:
//!
//! ```text
//! cargo bench -p guardrail-classifiers
//! ```
//!
//! ## Performance targets (from project guidelines)
//!
//! | Stage | Target (p99) |
//! |-------|--------------|
//! | `RegexInjectionScanner` | < 50 µs for 8 KB input |
//! | `PiiRedactor`           | < 20 µs for 4 KB input |

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use guardrail_classifiers::{PiiRedactor, RegexInjectionScanner};
use guardrail_core::{
    pipeline::Stage,
    request::{ChatMessage, GuardrailRequest, MessageContent, Provider, Role},
};

/// Build a synthetic request with `n` repetitions of a benign sentence.
fn build_request(repeats: usize) -> GuardrailRequest {
    let text = "The quick brown fox jumps over the lazy dog near the riverbank. ".repeat(repeats);

    GuardrailRequest::new(
        vec![ChatMessage {
            role: Role::User,
            content: MessageContent::Text(text),
        }],
        "gpt-4o".into(),
        Provider::OpenAI,
    )
}

/// Build a request containing PII for redaction benchmarks.
fn build_pii_request(repeats: usize) -> GuardrailRequest {
    let mut text = String::new();
    for i in 0..repeats {
        text.push_str(&format!(
            "Contact person{i}@example.com or call 555-{:04}.\n",
            i % 10000
        ));
    }

    GuardrailRequest::new(
        vec![ChatMessage {
            role: Role::User,
            content: MessageContent::Text(text),
        }],
        "gpt-4o".into(),
        Provider::OpenAI,
    )
}

fn bench_regex_injection(c: &mut Criterion) {
    let scanner = RegexInjectionScanner::default();
    let mut group = c.benchmark_group("regex_injection_scanner");

    // Roughly: each repeat of the sentence is ~65 bytes.
    for &repeats in &[1usize, 16, 64, 128] {
        let req = build_request(repeats);
        let size_bytes = req.user_text().len();
        group.throughput(Throughput::Bytes(size_bytes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size_bytes}B")),
            &req,
            |b, req| {
                b.iter(|| scanner.evaluate_sync(req));
            },
        );
    }

    group.finish();
}

fn bench_pii_redaction(c: &mut Criterion) {
    let redactor = PiiRedactor::default();
    let mut group = c.benchmark_group("pii_redactor");

    for &repeats in &[1usize, 16, 64, 128] {
        let req = build_pii_request(repeats);
        let text = req.user_text();
        let size_bytes = text.len();
        group.throughput(Throughput::Bytes(size_bytes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size_bytes}B")),
            &text,
            |b, text| {
                b.iter(|| redactor.redact_text(text));
            },
        );
    }

    group.finish();
}

fn bench_full_pipeline_async(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let scanner = RegexInjectionScanner::default();
    let redactor = PiiRedactor::default();

    let mut group = c.benchmark_group("stage_evaluate_async");

    let clean_req = build_request(16);
    group.bench_function("regex_injection_async_clean", |b| {
        b.to_async(&rt)
            .iter(|| async { scanner.evaluate(&clean_req).await.unwrap() });
    });

    let pii_req = build_pii_request(16);
    group.bench_function("pii_redactor_async_with_pii", |b| {
        b.to_async(&rt)
            .iter(|| async { redactor.evaluate(&pii_req).await.unwrap() });
    });

    group.finish();
}

/// ONNX classifier latency with the real models. Needs `--features onnx` and
/// `GUARDRAIL_BENCH_MODEL_DIR` pointing at a directory containing
/// `prompt-injection/{model.onnx,tokenizer.json}` and/or
/// `toxicity/{model.onnx,tokenizer.json}` (see docs/onnx-models.md). Each
/// model that isn't present is skipped with a message.
///
/// Measures `score()`, i.e. tokenization plus one ONNX Runtime inference on a
/// single CPU thread (`intra_threads = 1`, as the stages are configured).
#[cfg(feature = "onnx")]
fn bench_onnx(c: &mut Criterion) {
    use guardrail_classifiers::{OnnxInjectionClassifier, ToxicityClassifier};
    use std::path::PathBuf;

    let Some(dir) = std::env::var_os("GUARDRAIL_BENCH_MODEL_DIR").map(PathBuf::from) else {
        eprintln!("skipping ONNX benchmarks: GUARDRAIL_BENCH_MODEL_DIR is not set");
        return;
    };

    // ~65 B, ~1 KB and ~1.8 KB of English text: roughly 15, 210 and 380
    // tokens, all under the 512-token limit of both recommended models.
    let inputs: Vec<String> = [1usize, 16, 28]
        .iter()
        .map(|&n| build_request(n).user_text())
        .collect();

    type Scorer = Box<dyn Fn(&str) -> f32>;
    let mut models: Vec<(&str, Scorer)> = Vec::new();

    let pi = dir.join("prompt-injection");
    if pi.join("model.onnx").exists() {
        let m =
            OnnxInjectionClassifier::load(pi.join("model.onnx"), pi.join("tokenizer.json"), 0.85)
                .expect("load prompt-injection model");
        models.push(("onnx_injection", Box::new(move |t| m.score(t).unwrap())));
    } else {
        eprintln!("skipping onnx_injection: {} not found", pi.display());
    }

    let tox = dir.join("toxicity");
    if tox.join("model.onnx").exists() {
        let m = ToxicityClassifier::load(tox.join("model.onnx"), tox.join("tokenizer.json"), 0.90)
            .expect("load toxicity model");
        models.push(("toxicity", Box::new(move |t| m.score(t).unwrap())));
    } else {
        eprintln!("skipping toxicity: {} not found", tox.display());
    }

    for (name, score) in &models {
        let mut group = c.benchmark_group(*name);
        group.sample_size(30);
        for text in &inputs {
            group.throughput(Throughput::Bytes(text.len() as u64));
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{}B", text.len())),
                text,
                |b, text| b.iter(|| score(text)),
            );
        }
        group.finish();
    }
}

#[cfg(not(feature = "onnx"))]
fn bench_onnx(_c: &mut Criterion) {}

criterion_group!(
    benches,
    bench_regex_injection,
    bench_pii_redaction,
    bench_full_pipeline_async,
    bench_onnx
);
criterion_main!(benches);
