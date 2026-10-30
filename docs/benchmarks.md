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

### ONNX throughput under concurrent requests

The numbers above are one request at a time. This is the `onnx_injection`
stage under concurrent load, from the `onnx_concurrency` benchmark: a fixed
number of requests (32 per row) through `Stage::evaluate`, issued by
`concurrency` tasks in parallel, with a pool of `sessions` sessions and
`intra_threads = 1`. Model: `protectai/deberta-v3-base-prompt-injection-v2`
(`onnx/model.onnx`, fp32). Prompt: 1 KB, one window. One local run.

| sessions | concurrency | req/s | p50 (ms) | p99 (ms) | RSS after load (MiB) |
|---------:|------------:|------:|---------:|---------:|---------------------:|
| 1 | 1 | 2.9 | 337 | 468 | 1081 |
| 1 | 4 | 3.0 | 1353 | 1397 | 1081 |
| 1 | 8 | 3.0 | 2727 | 2816 | 1081 |
| 2 | 1 | 3.0 | 331 | 395 | 1964 |
| 2 | 4 | 5.7 | 686 | 779 | 1964 |
| 2 | 8 | 5.7 | 1381 | 1493 | 1964 |
| 4 | 1 | 3.1 | 319 | 490 | 3600 |
| 4 | 4 | 9.7 | 402 | 506 | 3600 |
| 4 | 8 | 9.9 | 804 | 821 | 3600 |

**Finding:** with one session (the behavior before the pool), throughput is
flat at about 3 req/s whatever the load, and latency grows linearly with the
queue. Each added session adds about 2.4–3 req/s until the cores run out,
and each costs about 0.85 GB of memory. Requests beyond the pool wait
asynchronously, so latency still grows with load: put a limit in front of
the proxy (or set `pipeline.stage_timeout_ms` with `on_error = "block"`)
for overload.

```bash
GUARDRAIL_BENCH_MODEL_DIR=models/prompt-injection \
GUARDRAIL_BENCH_SESSIONS=1,2,4 GUARDRAIL_BENCH_CONCURRENCY=1,4,8 GUARDRAIL_BENCH_REQUESTS=32 \
  cargo bench -p guardrail-classifiers --features onnx --bench onnx_concurrency
```

To reproduce the single-request figures (models laid out as in `docs/onnx-models.md`):

```bash
GUARDRAIL_BENCH_MODEL_DIR=models \
  cargo bench -p guardrail-classifiers --features onnx --bench classifier_benchmarks -- 'onnx_injection|toxicity'
```

or run the manual `ONNX real models` workflow, which downloads and exports
the same pinned model commits and uploads the Criterion output.

### ONNX on 8 cores: pool size, long input, both stages

A separate set of measurements, on a different machine from the tables
above. Read them on their own; the two sets aren't comparable.

- Machine: 8 physical cores / 16 threads, 23 GiB of memory available.
- Software: Rust 1.99.1, `ort` 2.0.0-rc.13, ONNX Runtime 1.28.0.
- Models: `protectai/deberta-v3-base-prompt-injection-v2` at `90c9989b`
  (`onnx/model.onnx`, fp32) and `unitary/unbiased-toxic-roberta` at
  `36295dd8` (exported with `optimum-onnx` 0.1.0, fp32).
- Method: `onnx_concurrency`, one process per row, after a warm-up. Each
  figure is the median of 3 runs, with the range in brackets where it
  matters. Every run measured at least 20 s, and nothing was swapped out.
  The prompt is 1 KB (less than one window) unless a window count is given.

#### Pool size (`onnx_injection`, `intra_threads = 1`)

| sessions | concurrency | req/s | p50 (ms) | p99 (ms) | RSS after load (MiB) |
|---------:|------------:|------:|---------:|---------:|---------------------:|
| 1 | 1 | 3.1 | 321 | 359 | 1237 |
| 1 | 16 | 3.3 | 4792 | 5073 | 1228 |
| 2 | 4 | 5.9 | 665 | 754 | 2040 |
| 4 | 4 | 9.7 | 409 | 503 | 3595 |
| 4 | 16 | 9.5 | 1678 | 1863 | 3637 |
| 8 | 8 | 12.1 (10.7 – 12.2) | 657 | 825 | 6902 |
| 8 | 32 | 11.9 | 2656 | 3038 | 6901 |
| 16 | 8 | 12.0 | 664 | 780 | 13493 |
| 16 | 16 | 10.8 | 1473 | 1595 | 13516 |

Against one session, 2 sessions give 1.8× the throughput, 4 give 3.0× and
8 give 3.7×. 16 sessions (one per hardware thread) are no faster than 8.
Each session adds about 810 MiB.

#### Pool size (`toxicity`, `intra_threads = 1`)

| sessions | concurrency | req/s | p50 (ms) | p99 (ms) | RSS after load (MiB) |
|---------:|------------:|------:|---------:|---------:|---------------------:|
| 1 | 1 | 3.9 | 251 | 293 | 649 |
| 2 | 4 | 7.4 | 530 | 588 | 1255 |
| 4 | 4 | 12.5 | 317 | 405 | 2373 |
| 8 | 8 | 16.1 | 486 | 626 | 4715 |
| 16 | 8 | 16.5 | 483 | 594 | 9342 |
| 16 | 16 | 15.5 | 1007 | 1254 | 9333 |

The same shape: throughput stops growing at 8 sessions. Each session adds
about 580 MiB.

#### `sessions` × `intra_threads` (`onnx_injection`)

Single-request latency is at concurrency 1; throughput is at concurrency
equal to `sessions`.

| sessions × intra_threads | p50 alone (ms) | req/s, pool full | p50, pool full (ms) | RSS after load (MiB) |
|---|---:|---:|---:|---:|
| 1 × 8 | 133 | 7.5 | 133 | 1219 |
| 2 × 4 | 129 | 11.4 | 173 | 2028 |
| 4 × 2 | 176 | 12.9 | 310 | 3630 |
| 8 × 1 | 296 | 12.1 | 656 | 6891 |
| 4 × 4 | 129 | 12.6 | 318 | 3611 |
| 8 × 2 | 178 | 12.5 | 642 | 6865 |
| 16 × 1 | 301 | 10.9 | 1452 | 13513 |

With 8 threads in total, 4 × 2 has the most throughput, half the latency
of 8 × 1 and half its memory. A second inference thread cuts the latency of one
request by 40% (296 → 176 ms); beyond 4 there is nothing left to gain. Using all 16 hardware threads (bottom three rows) is no faster than
using 8.

#### Why throughput stops at 8 sessions

What was measured, with
[`contrib/load-test/cpu_probe.c`](../contrib/load-test/cpu_probe.c) beside
the benchmark:

| busy probe threads | 1 | 2 | 4 | 8 | 16 |
|---|---:|---:|---:|---:|---:|
| read bandwidth, total (GB/s) | 19.8 | 29.8 | 34.9 | 34.7 | 34.1 |
| fused multiply-add rate, per thread (GFLOP/s) | 56.8 | 50.5 | 46.5 | 37.8 | 23.3 |

- **Memory bandwidth tops out at about 35 GB/s, reached with 4 threads.** In
  `bandwidth` mode each thread sums its own 256 MiB buffer (far larger than
  any cache) with AVX2 loads, over and over, for 6 s; the figure is the
  total bytes read divided by the elapsed time.
- **Compute per core falls as more cores are busy:** a third lower with 8
  threads than with 1, consistent with lower all-core clocks. At 16 threads two
  share each core. In `compute` mode each thread runs fused multiply-adds
  on registers only, so this doesn't touch memory.
- **Interference:** 4 sessions at concurrency 4, with 4 more threads busy
  beside them:

  | beside the benchmark | req/s | p50 (ms) |
  |---|---:|---:|
  | nothing | 9.7 | 409 |
  | `cpu_probe compute 4` | 7.5 | 529 |
  | `cpu_probe bandwidth 4` (28.4 GB/s) | 5.2 | 758 |

  Threads that only compute cost 23% of the throughput. The same number of
  threads streaming memory cost 46%, so the extra loss is memory traffic.
- **No drop over time.** 4 × 1 at concurrency 4 and 8 × 1 at concurrency 8,
  run for 120 s each and counted every 10 s, held 8.7–9.6 and
  10.9–12.0 req/s with no downward trend. That rules out a sustained
  power or thermal limit setting in after a short boost.
- **Compute grows faster than throughput.** From 4 to 8 busy threads, the
  probe's total multiply-add rate grows 1.6× (186 to 302 GFLOP/s), while
  `onnx_injection` throughput grows 1.25× (9.7 to 12.1 req/s).

Clocks are steady over time. Throughput grows less than the compute
available, and a neighbor streaming memory costs twice as much as one that
only computes. That is consistent with memory bandwidth as the limit, but
doesn't prove it: we didn't measure the models' own memory traffic, and
couldn't control which physical cores the threads ran on (see
[smaller machines](#smaller-machines)).

To reproduce the probe figures:

```bash
cc -O2 -mavx2 -mfma -pthread -o cpu_probe contrib/load-test/cpu_probe.c
for n in 1 2 4 8 16; do ./cpu_probe bandwidth $n 6; done
for n in 1 2 4 8 16; do ./cpu_probe compute $n 6; done
```

For the interference rows, start `./cpu_probe compute 4 75 &` (or
`bandwidth 4 75`) and then the 4-session benchmark.

#### Long input

A prompt of exactly 4, 8 or 16 full windows (`GUARDRAIL_BENCH_WINDOWS`),
`intra_threads = 1`. *Alone* is one session and one request at a time.
*Under load* is the automatic pool of 4 sessions with 4 such requests at
once, which is what 4 long requests sent together cost on the automatic
pool with `intra_threads = 1`, the default before 0.4.0.

| windows | tokens | prompt (this text) | `onnx_injection` alone | under load | `toxicity` alone | under load |
|---:|---:|---:|---:|---:|---:|---:|
| 1 (1 KB prompt) | < 512 | 1 KB | 0.32 s | 0.41 s | 0.25 s | 0.32 s |
| 4 | 1,664 | 10 KB | 4.8 s | 7.6 s | 3.3 s | 4.5 s |
| 8 | 3,200 | 19 KB | 9.5 s | 15.5 s | 6.7 s | 9.4 s |
| 16 | 6,272 | 38 KB | 21.0 s | 31.3 s | 14.7 s | 18.7 s |

Cost grows linearly with the window count, about 1.2–1.3 s per full window
for `onnx_injection` when alone. How many bytes fill a window depends on
the text: this is plain English, and text that tokenizes into more tokens
per byte (code, many non-Latin scripts) reaches the limit in fewer bytes.

More inference threads shorten one long request, but not in proportion
(16 windows, one session, one request):

| intra_threads | `onnx_injection` | `toxicity` |
|---:|---:|---:|
| 1 | 21.0 s | 14.7 s |
| 2 | 11.7 s | 7.6 s |
| 4 | 9.5 s | 5.1 s |
| 8 | 9.3 s | 4.3 s |

With the automatic pool at `intra_threads = 2` (4 × 2 here) and 4 such
requests at once, a 16-window request took 26.8 s in `onnx_injection`
(26.8–27.6) and 14.0 s in `toxicity` (13.7–14.2).

What this means for denial of service is in the
[threat model](threat-model.md#denial-of-service-resource-exhaustion).

#### Through the proxy, both stages

`contrib/load-test/onnx-proxy.sh`: the proxy with `onnx_injection` and
`toxicity` enabled, a mock upstream that answers at once, and `oha` sending
a benign 1.1 KB chat request, so every request runs both models (one after
the other). All responses were `200`. Memory is the proxy's peak resident
set.

| pool per stage | ONNX threads, both stages | concurrency | req/s | p50 (ms) | p99 (ms) | peak RSS (MiB) |
|---|---:|---:|---:|---:|---:|---:|
| 4 × 1 (automatic) | 8 | 4 | 5.1 | 783 | 922 | 5852 |
| 4 × 1 (automatic) | 8 | 16 | 5.8 | 2720 | 3204 | 5881 |
| 4 × 2 | 16 | 4 | 6.9 | 573 | 694 | 5861 |
| 4 × 2 | 16 | 16 | 6.1 | 2533 | 2984 | 5854 |
| 2 × 2 | 8 | 4 | 6.6 | 596 | 750 | 2994 |
| 2 × 2 | 8 | 16 | 6.7 | 2358 | 2632 | 3029 |

Each stage sizes its pool from the machine as if it were alone. With
4 × 2 per stage, a full load (concurrency 16) keeps both pools busy, which
is 16 inference threads on 8 cores, and throughput falls from 6.9 to
6.1 req/s. 2 × 2 per stage, 8 threads in all, holds 6.6–6.7 req/s at both
loads with half the memory. Size the pools by the total threads across
all enabled ONNX stages; see [configuration](configuration.md#onnx).

#### Smaller machines

The same runs restricted with `taskset` to 2, 4 and 8 logical CPUs, with
the automatic pool size, which then follows the CPUs available.

The OS lists logical CPUs 0–1, 2–3 and so on as the two threads of one core,
but `taskset` didn't keep the work on those pairs. Pinned to the same CPU
sets, `cpu_probe compute` got about as much per thread as threads on
separate cores (2 threads on CPUs 0–1: 47.5 GFLOP/s each, 95 in all; one
core does 57 at most). So by the probe, each run had one physical core per
logical CPU, with one thread on each. The table says so in its first
column. These are **not** like cloud instances whose vCPUs are SMT pairs,
where 2 vCPUs are one core: expect less from those.

| `taskset -c` | logical CPUs | `cpu_probe compute`, total (GFLOP/s) | physical cores × threads, by the probe |
|---|---:|---:|---|
| `0-1` | 2 | 95 | 2 × 1 |
| `0-3` | 4 | 175 | 4 × 1 |
| `0-7` | 8 | 293 | 8 × 1 |
| (none) | 16 | 302 | 8 × 2 |

`onnx_injection` alone. *Alone* is concurrency 1; the last three columns
are at concurrency 8.

| physical cores × threads per core | `intra_threads` | automatic pool (sessions × intra threads) | p50 alone (ms) | req/s | p50 (ms) | RSS after load (MiB) |
|---|---:|---|---:|---:|---:|---:|
| 2 × 1 | 1 | 2 × 1 | 307 | 5.8 | 1363 | 2026 |
| 2 × 1 | 2 | 1 × 2 | 183 | 5.5 | 1465 | 1228 |
| 4 × 1 | 1 | 4 × 1 | 306 | 9.3 | 851 | 3645 |
| 4 × 1 | 2 | 2 × 2 | 181 | 9.3 | 853 | 2037 |
| 8 × 1 | 1 | 4 × 1 | 305 | 9.5 | 828 | 3601 |
| 8 × 1 | 2 | 4 × 2 | 181 | 12.6 | 636 | 3620 |

Both stages through the proxy (`onnx-proxy.sh` with `PROXY_CPUS`; the mock
upstream and `oha` run on other CPUs). Memory is the proxy's peak resident
set.

| physical cores × threads per core | `intra_threads` | automatic pool per stage (sessions × intra threads) | p50 alone (ms) | req/s | p50 (ms) | peak RSS (MiB) |
|---|---:|---|---:|---:|---:|---:|
| 2 × 1 | 1 | 2 × 1 | 564 | 3.2 | 2506 | 2992 |
| 2 × 1 | 2 | 1 × 2 | 323 | 2.9 | 2762 | 1646 |
| 4 × 1 | 1 | 4 × 1 | 555 | 5.2 | 1529 | 5824 |
| 4 × 1 | 2 | 2 × 2 | 329 | 4.8 | 1689 | 3032 |
| 8 × 1 | 1 | 4 × 1 | 556 | 6.7 | 1179 | 5836 |
| 8 × 1 | 2 | 4 × 2 | 337 | 6.5 | 1228 | 5889 |

With `intra_threads = 2` the automatic pool has half as many sessions:

- the latency of one request is about 40% lower on every size;
- on 2 and 4 cores, memory is 40–48% lower;
- throughput at concurrency 8 is the same or up to 5% lower for one stage
  on 2 and 4 cores, and 33% higher on 8 cores; with both stages it is 3–9%
  lower.

With `intra_threads = 1` and both stages, 4 CPUs get the same pools as 8
(4 × 1 per stage, 8 threads), and so the same 5.7 GiB of memory.

The rows match the unrestricted runs with the same number of busy threads
(2 sessions: 5.8 and 5.9 req/s; 4 × 2: 12.6 and 12.9), which is what one
core per logical CPU predicts.

#### Reproducing

```bash
# One row: pool, threads, load, and optionally an exact window count
GUARDRAIL_BENCH_MODEL_DIR=models/prompt-injection \
GUARDRAIL_BENCH_SESSIONS=4 GUARDRAIL_BENCH_INTRA_THREADS=2 \
GUARDRAIL_BENCH_CONCURRENCY=4 GUARDRAIL_BENCH_REQUESTS=256 \
  cargo bench -p guardrail-classifiers --features onnx --bench onnx_concurrency

# toxicity, 16 windows, one at a time
GUARDRAIL_BENCH_MODEL_DIR=models/toxicity GUARDRAIL_BENCH_STAGE=toxicity \
GUARDRAIL_BENCH_SESSIONS=1 GUARDRAIL_BENCH_CONCURRENCY=1 \
GUARDRAIL_BENCH_REQUESTS=4 GUARDRAIL_BENCH_WINDOWS=16 \
  cargo bench -p guardrail-classifiers --features onnx --bench onnx_concurrency

# 120 s at a steady load, throughput every 10 s
GUARDRAIL_BENCH_MODEL_DIR=models/prompt-injection \
GUARDRAIL_BENCH_SESSIONS=4 GUARDRAIL_BENCH_CONCURRENCY=4 \
GUARDRAIL_BENCH_REQUESTS=1200 GUARDRAIL_BENCH_PROGRESS_SECS=10 \
  cargo bench -p guardrail-classifiers --features onnx --bench onnx_concurrency

# Both stages through the proxy (needs oha); the proxy on 4 CPUs
SESSIONS=0 INTRA=2 CONCURRENCY=8 PROXY_CPUS=0-3 LOAD_CPUS=8-15 just load-test-onnx
```

For a smaller machine, run the benchmark under `taskset -c 0-3 cargo bench …`;
the automatic pool size follows the CPUs the process is allowed. Check
with `taskset -c 0-3 ./cpu_probe compute 4 6` which cores that gives you:
about one core's rate per thread means separate cores, about half means
SMT pairs. Run one
configuration per process, so the memory figures belong to that row.

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

### Streaming PII redaction (`pii_stream`)

Response redaction for `stream: true` responses
(`StreamingPiiRedactor`, used by the proxy's SSE rewriter). One local run on
the same machine as above, 2026-10-15, Rust 1.99.0. A 2,058-byte answer
containing an email and a phone number per line was fed as 4-byte deltas
(about one model token each, 515 deltas):

| Benchmark | Time |
|-----------|-----:|
| `pii_stream/2058B_in_4B_deltas` (whole response, streamed) | 180 µs (178 – 183) |
| `pii_stream/2058B_whole_text_baseline` (same text, one `redact_text` call) | 11.5 µs (11.4 – 11.7) |

That's about 0.35 µs per delta. Streaming costs more than one-shot
redaction because each release first checks that cutting there gives the
same result as redacting the whole buffered text. It's still small next to
model token latency. A first version that retried on every delta measured
419 µs for the same input. The redactor now retries only after 32 new bytes.

```bash
cargo bench -p guardrail-classifiers --bench classifier_benchmarks -- pii_stream
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
  0.4 s per stage, each on one core. Throughput per stage is about
  `sessions / latency`: 3 req/s with one session and 9.9 req/s with four
  at 1 KB (see [above](#onnx-throughput-under-concurrent-requests)).
  On the 8-core machine ([measurements](#onnx-on-8-cores-pool-size-long-input-both-stages)),
  that holds up to about 4 sessions and then flattens: one stage peaks at
  12–13 req/s (`onnx_injection`) and 16 req/s (`toxicity`) at 1 KB, however
  the threads are split between sessions and intra threads, because all
  cores share one limit (most likely memory bandwidth). With both stages in
  series through the proxy, 6–7 req/s. Long input costs about 1.2 s per
  full 512-token window per stage on one thread, so one 16-window request
  takes 21 s in `onnx_injection` and 15 s in `toxicity`.

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
