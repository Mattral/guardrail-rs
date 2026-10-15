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
| `onnx_injection` (CPU, 1 thread) | 64 B | 41.9 ms † | 41.9 – 52.9 ms (2 local runs) | < 5 ms | ❌ |
| `onnx_injection` (CPU, 1 thread) | 1.0 KB | 426 ms † | 426 ms (2 local runs) | — | |
| `toxicity` (CPU, 1 thread) | 64 B | 36.4 ms † | 33.6 – 36.4 ms (2 local runs) | < 5 ms | ❌ |
| `toxicity` (CPU, 1 thread) | 1.0 KB | 378 ms † | 378 – 437 ms (2 local runs) | — | |
| Full pipeline (regex + PII), worst case (8 KB) | 8.2 KB | 158 µs | — | < 1 ms | ✅ [see detail](#full-pipeline-measured-gated-and-passing-single-run-snapshot) |

**One real finding worth flagging:** in the CI rows above, `pii_redactor`
misses its own documented `< 20 µs / 4 KB` target once the input actually
contains PII at a few KB: 50.8 µs at 3.0 KB and 101 µs at 6.0 KB, roughly
2.5–5× over budget. (`RegexInjectionScanner`, by contrast, comes in at
roughly half its budget even at the worst-case 8 KB input.)

**Update, single-pass rewrite.** Those CI rows predate a rewrite of
`PiiRedactor::redact_text_with_records`. It used to scan the text twice per
pattern (once for audit records, once to replace). Now it scans once per
pattern, and skips patterns that a combined `RegexSet` scan says can't
match. The output is identical, which is checked by property tests against
the old implementation. Measured back to back in one session on one machine
(Intel Core Ultra 5 226V, Rust 1.99.0, Windows 11; Criterion point
estimates):

| Input | Before | After | Change |
|------:|-------:|------:|-------:|
| 46 B | 810 ns | 363 ns | −55% |
| 742 B | 8.36 µs | 3.77 µs | −55% |
| 3.0 KB | 33.6 µs | 15.7 µs | −55% |
| 4.1 KB | 44.4 µs | 23.2 µs | −48% |
| 6.0 KB | 66.0 µs | 29.6 µs | −54% |

**Target status: still not reliably met.** Three runs of the new code at
4.1 KB on this machine gave 19.2, 19.7 and 23.2 µs, so it sits right at the
20 µs line on a fast laptop, and CI's shared runners are slower in every
other row. The CI row stays ❌ until the dashboard shows otherwise. The next
step would be to avoid re-running the `RegexSet` after every pattern that
changes the text. The 4.1 KB size (`pii_redactor/4079B`) was added to the
benchmark for this, since the target is stated at 4 KB and no benchmark
measured that size.

To reproduce the comparison:

```bash
git stash push crates/guardrail-classifiers/src/pii.rs   # or check out the parent commit
cargo bench -p guardrail-classifiers --bench classifier_benchmarks -- pii_redactor --save-baseline before
git stash pop
cargo bench -p guardrail-classifiers --bench classifier_benchmarks -- pii_redactor --baseline before
```

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

### ONNX classifiers: measured, and far over target

† Unlike the rest of the table, these rows are **not** from the CI dashboard.
They come from two local runs of the `onnx_injection` and `toxicity` groups
in `classifier_benchmarks.rs`, on 2026-10-14 and 2026-10-15 (Criterion, 30
samples each, point estimate shown). The README quotes run 2.

- Machine: Intel Core Ultra 5 226V laptop (8 cores), 15.5 GB RAM, Windows 11,
  Balanced power plan.
- Toolchain and code: Rust 1.99.0, the code from the PR that added these
  rows.
- Models: `protectai/deberta-v3-base-prompt-injection-v2` at commit
  `90c9989b`, using its published `onnx/model.onnx` (fp32). And
  `unitary/unbiased-toxic-roberta` at `36295dd8`, exported with
  `optimum-onnx` 0.1.0 (fp32).
- What's timed: `score()`, i.e. tokenization plus one ONNX Runtime
  inference with `intra_threads = 1`, which is how the stages configure
  their sessions. The inputs are under 512 tokens, so each is a single
  window.

| Input | `onnx_injection` run 1 | `onnx_injection` run 2 | `toxicity` run 1 | `toxicity` run 2 |
|------:|---:|---:|---:|---:|
| 64 B | 52.9 ms (47.5 – 58.9) | 41.9 ms (41.6 – 42.3) | 33.6 ms (32.9 – 34.7) | 36.4 ms (35.8 – 37.1) |
| 1.0 KB | 426 ms (418 – 434) | 426 ms (425 – 429) | 437 ms (397 – 482) | 378 ms (376 – 381) |
| 1.8 KB | 845 ms (824 – 872) | 828 ms (821 – 835) | 1.25 s (1.23 – 1.26) | 702 ms (695 – 709) |

Ranges are Criterion's 95% confidence interval within each run. The gap
between runs, up to 44% for `toxicity` at 1.8 KB, is larger than those
intervals. On this laptop, run-to-run variance dominates, so treat any single
figure as approximate. Run 2 was on code that also includes the PII rewrite
and the config-wiring changes, which don't touch the timed `score()` path.

**Finding:** the `< 5 ms` design target is missed by 7–250×. Latency grows
with input length, and a 1 KB prompt costs about 0.4 s per stage on one
core. That makes the ONNX stages unsuitable for latency-sensitive traffic
as currently configured. Each window of a long input is another inference
of up to this cost, run sequentially, so a request near the 16-window limit
costs many seconds. Likely improvements, none measured yet: more
intra-op threads, int8-quantized models, smaller distilled models, and
batching the windows of a long input into one inference.

To reproduce (models laid out as in `docs/onnx-models.md`):

```bash
GUARDRAIL_BENCH_MODEL_DIR=models \
  cargo bench -p guardrail-classifiers --features onnx --bench classifier_benchmarks -- 'onnx_injection|toxicity'
```

or run the manual `ONNX real models` workflow, which downloads and exports
the same pinned model commits and uploads the Criterion output.

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

### Local reference run (the README's figures)

The latency numbers quoted in the README come from this one local run, not
from CI. The CI figures above are medians over shared runners; this run is a
quieter machine, so it reads somewhat faster.

- Machine: Intel Core Ultra 5 226V laptop (8 cores), 15.5 GB RAM, Windows 11,
  Balanced power plan, on AC power.
- Code: `main` at `b610ed6`, which includes the single-pass PII redactor.
  Rust 1.99.0.
- Date: 2026-10-15.
- Values are Criterion's point estimate (mean), with the 95% confidence
  interval in parentheses.

| Benchmark | Input | Time |
|-----------|------:|-----:|
| `full_pipeline_regex_only` (regex + PII, short request with an email) | short | 1.89 µs (1.88 – 1.90) |
| `full_pipeline_by_size/clean_with_pii` | 512 B | 4.67 µs (4.64 – 4.71) |
| `full_pipeline_by_size/clean_with_pii` | 4 KB | 26.5 µs (25.9 – 27.6) |
| `full_pipeline_by_size/clean_with_pii` | 8 KB | 47.3 µs (47.0 – 47.7) |
| `full_pipeline_blocked_short_circuit` | short | 1.28 µs (1.22 – 1.35) |
| `regex_injection_scanner` (sync) | 8.2 KB | 18.7 µs (18.6 – 18.8) |
| `pii_redactor` (sync, `redact_text`) | 742 B | 3.68 µs (3.66 – 3.70) |
| `pii_redactor` (sync, `redact_text`) | 3.0 KB | 14.3 µs (14.2 – 14.4) |
| `pii_redactor` (sync, `redact_text`) | 4.1 KB | 19.0 µs (18.9 – 19.1) |
| `pii_redactor` (sync, `redact_text`) | 6.0 KB | 28.4 µs (28.2 – 28.6) |

For comparison, the same pipeline benchmark on this machine the day before,
with the old two-pass PII redactor: 11.8 µs at 512 B, 67.8 µs at 4 KB, and
139 µs at 8 KB.

The ONNX figures in the README are run 2 of
[ONNX classifiers](#onnx-classifiers-measured-and-far-over-target), from the
same session.

To reproduce:

```bash
cargo bench -p guardrail-test-suite --bench pipeline
cargo bench -p guardrail-classifiers --bench classifier_benchmarks
```

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

- **With ONNX classifiers (CPU):** inference-bound. In the two local runs
  above, a 64 B prompt took 34–53 ms per stage and a 1 KB prompt about
  0.4 s per stage, each on one core. Each inference occupies one Tokio
  blocking-pool thread for that long. Throughput under load hasn't been
  measured.

- **With ONNX classifiers (CUDA):** not measured.

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
