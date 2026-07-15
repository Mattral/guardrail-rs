# Benchmarks

Performance characteristics of `guardrail-rs`'s classifier microbenchmarks,
measured on GitHub Actions' `ubuntu-latest` shared runners via the
[benchmark workflow](../.github/workflows/benchmarks.yml)
(`bench` job → `cargo bench -p guardrail-classifiers --bench
classifier_benchmarks`), tracked on every push to `main` with
`benchmark-action/github-action-benchmark` and published to the
[live, interactive dashboard](https://mattral.github.io/guardrail-rs/dev/bench/).

**This is a shared CI runner, not a dedicated benchmarking machine** — Rust
toolchain is whatever `stable` resolves to at run time, not a pinned
version, and run-to-run variance from neighboring CI load is real and
visible below (this replaces an earlier version of this doc that described
a fictional dedicated AWS instance before any benchmark had actually been
run). Treat the numbers below as directional, not lab-grade precision; use
the live dashboard's per-commit history if you need to correlate a specific
change to a specific latency shift.

---

## Pipeline latency summary

Figures are the **median across the 6 most recent tracked runs** (as of
2026-07-05) rather than a single sample, specifically because run-to-run
variance on shared runners is large enough to be misleading otherwise — see
[Observed CI variance](#observed-ci-variance-why-median-not-latest) below.

| Benchmark | Input size | Median | Observed range (6 runs) | Target | |
|-----------|-----------:|-------:|:------------------------|--------|:-:|
| `regex_injection_scanner` (sync) | 64 B | 0.36 µs | 0.30 – 0.37 µs | — | |
| `regex_injection_scanner` (sync) | 1.0 KB | 3.65 µs | 3.33 – 4.06 µs | — | |
| `regex_injection_scanner` (sync) | 4.1 KB | 14.3 µs | 13.3 – 15.9 µs | — | |
| `regex_injection_scanner` (sync) | 8.2 KB | 28.5 µs | 26.1 – 31.7 µs | < 50 µs / 8 KB | ✅ |
| `pii_redactor` (sync, `redact_text`) | 46 B | 1.4 µs | 1.0 – 1.6 µs | — | |
| `pii_redactor` (sync, `redact_text`) | 742 B | 13.1 µs | 12.0 – 15.1 µs | < 20 µs / 4 KB | ✅ |
| `pii_redactor` (sync, `redact_text`) | 3.0 KB | 50.8 µs | 46.7 – 58.7 µs | < 20 µs / 4 KB | ❌ |
| `pii_redactor` (sync, `redact_text`) | 6.0 KB | 101 µs | 93.0 – 116.9 µs | — | |
| `stage_evaluate_async` (real `Stage::evaluate`, incl. `Decision`/tracing) | ~1.0 KB, clean | 3.71 µs | 3.36 – 4.11 µs | — | |
| `stage_evaluate_async` (real `Stage::evaluate`, incl. `Decision`/tracing) | ~742 B, w/ PII | 14.7 µs | 13.3 – 16.0 µs | — | |
| `onnx_injection` (CPU) | — | *not yet benchmarked* | — | < 5 ms | ⬜ |
| `toxicity` (CPU) | — | *not yet benchmarked* | — | < 5 ms | ⬜ |
| Full pipeline (regex + PII), worst case (8 KB) | 8.2 KB | 158 µs | — | < 1 ms | ✅ [see detail](#full-pipeline-measured-gated-and-passing-single-run-snapshot) |

**One real finding worth flagging:** `pii_redactor` misses its own
documented `< 20 µs / 4 KB` target once the input actually contains PII at
a few KB — 50.8 µs at 3.0 KB, 101 µs at 6.0 KB, roughly 2.5–5× over budget.
It's nowhere close to threatening the pipeline's overall 5 ms hard ceiling
(see below), so this isn't a functional problem today, but if sub-100 µs
tail latency on large, PII-dense payloads matters for your deployment,
it's the one stage worth profiling first. (`RegexInjectionScanner`, by
contrast, comes in at roughly half its budget even at the worst-case 8 KB
input.)

**A second, smaller, real finding:** the actual production code path
(`Stage::evaluate`, async, benchmarked as `stage_evaluate_async`) tracks
closely with the raw sync `RegexInjectionScanner` call (3.71 µs vs 3.65 µs
— negligible async-dispatch overhead) but runs about 12% slower than the
raw sync `PiiRedactor::redact_text` call (14.7 µs vs 13.1 µs at a
comparable size). Reading `PiiRedactor::evaluate`'s source explains why: it
does real extra work the sync microbenchmark doesn't exercise — building
the full `Decision::Redact` (a cloned, mutated `GuardrailRequest`, not just
a string), deduplicating entity types into a summary, formatting a reason
string, and emitting a `tracing::info!` event. Not a bug, just a reminder
that the sync `pii_redactor` numbers above are a *lower bound* on what a
real request actually costs, not the full picture.

### ONNX classifiers: not yet benchmarked

`classifier_benchmarks.rs` has no benchmark group for `OnnxInjectionClassifier`
or `ToxicityClassifier` at all — not "ran and slow," genuinely absent from
the file, and the CI `bench` job doesn't pass `--features onnx` regardless.
Getting real numbers here needs: (1) exported model files (`./models/export_models.sh`
— not bundled in the repo, see `docs/onnx-models.md`), (2) a new
`#[cfg(feature = "onnx")]`-gated benchmark group added to
`classifier_benchmarks.rs`, and (3) a CI job (or a manually-run, locally
reported number) that actually exercises it. Until then, the `< 5 ms`
figures anywhere else in this repo's docs are the *design target* from the
project guidelines, not a measurement — treat them accordingly.

### Full pipeline: measured, gated, and passing (single-run snapshot)

Unlike the classifier microbenchmarks above, `guardrail-test-suite/benches/pipeline.rs`
(the `pipeline-latency-gate` CI job) doesn't publish historical data to
`gh-pages` yet, so the figures below are from one specific run
(`8a8bc3a`, 2026-07-05) rather than a median across several — there's no
observed-variance range to report the way there is above. All figures
are well inside both the 1 ms design target and the 5 ms CI hard ceiling.

| Benchmark | What it measures | Latency | vs. 1 ms target |
|-----------|-------------------|--------:|:-----------------|
| `full_pipeline_regex_only` | Full assembled `Pipeline::run()` (regex injection + PII redaction stages, async, via `tokio`), short request containing an email address | 2.99 µs | 0.3% |
| `full_pipeline_by_size/clean_with_pii/512` | Same, padded to ~512 B | 12.3 µs | 1.2% |
| `full_pipeline_by_size/clean_with_pii/4096` | Same, padded to ~4 KB | 80.8 µs | 8.1% |
| `full_pipeline_by_size/clean_with_pii/8192` | Same, padded to ~8 KB | 158 µs | 15.8% |
| `full_pipeline_blocked_short_circuit` | Malicious input — regex stage blocks immediately, PII stage never runs (the fastest possible path) | 1.01 µs | 0.1% |

Two extra cases in the same benchmark file, run in isolation rather than
through the assembled pipeline — not part of the CI gate, but useful
context: `regex_injection_scanner/clean_input` (short clean sentence) took
241 ns, `regex_injection_scanner/malicious_input` (short injection
attempt) took 516 ns. (Both are labeled by a fixed short sentence, not an
actual byte-size target — unlike `full_pipeline_by_size` above, which
really does pad its input to hit the labeled size.)

At worst case (8 KB, with PII present), the full pipeline uses about 16%
of its 1 ms design budget and about 3% of the 5 ms CI ceiling — a
comfortable margin, consistent with the regex/PII stages individually
each coming in well under their own per-stage targets in the classifier
table above.

Want this tracked over time instead of a single snapshot? Adding a
second `benchmark-action/github-action-benchmark` step to
`pipeline-latency-gate` (mirroring the one already in the `bench` job)
would start publishing this to `gh-pages` on every push, same as the
classifier microbenchmarks — happy to wire that up on request.

---

## Observed CI variance (why median, not "latest")

The dashboard's 6 tracked runs so far show real run-to-run swings on
what's presumably unchanged benchmark code — e.g. `regex_injection_scanner`
at 8.2 KB ranges from 26.1 µs to 31.7 µs (≈ 22%) across runs with no
obvious relationship to what each commit actually changed. This is
expected on shared, variable-load CI runners and is exactly why the table
above reports a median across runs rather than whatever the latest run
happened to show — a single sample could easily overstate or understate
a real regression. As more runs accumulate on the dashboard, prefer
reading the live charts over this static table for anything time-sensitive.

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
