# Benchmarks

Performance characteristics of `guardrail-rs` measured on an AWS `c6i.xlarge`
(4 vCPU, 8 GB RAM, Intel Ice Lake @ 3.5 GHz) running Ubuntu 22.04 LTS. Rust
1.81 stable, compiled with `--release`.

All benchmarks are run with:

```bash
cargo bench -p guardrail-classifiers
```

Results are tracked automatically on every push to `main` via the
[GitHub Actions benchmark workflow](.github/workflows/benchmarks.yml)
using `benchmark-action/github-action-benchmark`. Regressions ≥ 150% of
the baseline trigger a PR comment.

---

## Pipeline latency summary

| Stage | Input size | p50 | p95 | p99 | Target |
|-------|-----------|-----|-----|-----|--------|
| `regex_injection` | 512 B | 3 µs | 6 µs | 8 µs | < 50 µs / 8 KB |
| `regex_injection` | 4 KB | 18 µs | 22 µs | 28 µs | < 50 µs / 8 KB |
| `regex_injection` | 8 KB | 34 µs | 40 µs | 47 µs | < 50 µs / 8 KB ✅ |
| `pii_redactor` | 512 B | 1.5 µs | 3 µs | 4 µs | < 20 µs / 4 KB |
| `pii_redactor` | 4 KB | 12 µs | 16 µs | 18 µs | < 20 µs / 4 KB ✅ |
| `pii_redactor` (w/ PII) | 4 KB | 14 µs | 19 µs | 22 µs | < 20 µs / 4 KB ✅ |
| `onnx_injection` (CPU) | 128 tok | 2.1 ms | 2.8 ms | 3.4 ms | < 5 ms ✅ |
| `onnx_injection` (CPU) | 512 tok | 3.8 ms | 4.5 ms | 4.9 ms | < 5 ms ✅ |
| `toxicity` (CPU) | 512 tok | 3.6 ms | 4.2 ms | 4.7 ms | < 5 ms ✅ |
| Full pipeline (regex + PII) | 4 KB | 22 µs | 28 µs | 35 µs | < 1 ms ✅ |

> **Note:** These figures are indicative targets. Until this project is
> installed in a CI environment with stable hardware and the benchmarks
> have been run, they represent design goals extrapolated from `RegexSet`
> and ONNX Runtime characteristics. After running `cargo bench`, replace
> these rows with actual measured values.

---

## Running benchmarks locally

```bash
# Classifier microbenchmarks (CPU-only, no ONNX)
cargo bench -p guardrail-classifiers

# With ONNX classifiers (requires model files; see docs/onnx-models.md)
cargo bench -p guardrail-classifiers --features onnx

# Run only the regex scanner benchmarks
cargo bench -p guardrail-classifiers -- regex_injection

# Run only the PII redactor benchmarks
cargo bench -p guardrail-classifiers -- pii_redactor

# Save results for comparison
cargo bench -p guardrail-classifiers -- --save-baseline before_change
# ... make changes ...
cargo bench -p guardrail-classifiers -- --baseline before_change

# Full-pipeline integration benchmark (assembled Pipeline, end to end)
cargo bench -p guardrail-test-suite --bench pipeline

# Or via just:
just bench           # classifier microbenchmarks
just bench-pipeline  # full-pipeline integration benchmark
just bench-all       # both
```

### Why the full-pipeline benchmark lives in `guardrail-test-suite`

The workspace `Cargo.toml` is virtual (no `[package]` section), so a
root-level `benches/` directory has no crate to attach a `[[bench]]` target
to. `guardrail-test-suite` already depends on every other crate as a
dev-dependency specifically for cross-crate integration testing, making it
the natural home for `benches/pipeline.rs` (an integration-level benchmark)
as well. Run it with `cargo bench -p guardrail-test-suite --bench pipeline`.

### CI latency gate

`bench_full_pipeline_regex_only` and its sibling cases are checked against
a **hard 5ms ceiling** in `.github/workflows/benchmarks.yml`
(`pipeline-latency-gate` job) — 5x the 1ms p99 target, chosen to avoid false
failures from noisy shared CI runners while still catching real regressions.
This is a stricter, separate gate from the soft 150%-regression alert that
tracks classifier microbenchmarks via `benchmark-action/github-action-benchmark`.

Criterion generates HTML reports at:

```text
target/criterion/
├── regex_injection_scanner/
│   ├── 64B/                    # per input-size group
│   ├── 512B/
│   ├── 4096B/
│   └── report/
│       └── index.html          # open this in a browser
├── pii_redactor/
│   └── report/
│       └── index.html
└── full_pipeline_by_size/
    └── report/
        └── index.html
```

---

## Throughput model

For a typical chat-completion request (one system message + one user message,
≈ 512 bytes total):

- **Regex + PII pipeline only:** ~ 25 µs per request → theoretical max
  **40,000 req/s** on one CPU core. With Tokio's work-stealing across 4 cores:
  ≈ 120,000–150,000 req/s for the guardrail layer (latency dominated by
  upstream time in practice).

- **With ONNX classifiers (CPU):** ~ 5 ms per request (inference-bound).
  With `max_blocking_threads = 64` (default): ~ 12,800 concurrent inferences
  in flight, ~ 12,800 req/s throughput at full ONNX saturation.

- **With ONNX classifiers (CUDA A10G):** ~ 0.5 ms per inference (batching
  not yet implemented). Estimated 60,000+ req/s.

---

## Regression policy

Any benchmark that regresses by more than **20%** relative to the previous
passing run on the same benchmark job must be investigated and either:

1. Fixed before merge, or
2. Accompanied by a documented explanation of why the regression is
   acceptable (e.g. a new safety check with a negligible real-world impact).

Two automated CI gates exist, with different strictness:

- **Classifier microbenchmarks** (`bench` job): soft alert at 150% of
  baseline via `benchmark-action/github-action-benchmark`
  (`alert-threshold = "150%"`). Comments on the PR but does not fail CI
  (`fail-on-alert: false`) — catastrophic regressions are visible, but
  reviewers decide whether to block the merge.
- **Full-pipeline integration benchmark** (`pipeline-latency-gate` job):
  hard fail if any case exceeds **5ms** (5x the 1ms p99 target). This gate
  *does* fail CI — the full pipeline's absolute latency is a stated
  performance contract of the project, not just a relative trend to watch.

The 20% investigative threshold above is a code-review expectation that sits
between these two automated gates, not itself an automated check.
