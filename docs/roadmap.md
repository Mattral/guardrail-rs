# Roadmap

Open work, grouped by what it needs to run. Measurements go in
[`benchmarks.md`](benchmarks.md) when they are done; remove the item here.

## Needs a GPU (A100 or H100, for example in Colab)

- [ ] **Make `onnx-cuda` actually use the GPU.** The feature builds ONNX
  Runtime with CUDA support, but the stages never register the CUDA
  execution provider, so they still run on the CPU. Register it when the
  sessions are built (`SessionPool::load` in
  `crates/guardrail-classifiers/src/onnx.rs`), log which provider is in use,
  and decide whether a missing GPU is a startup error or a CPU fallback.
- [ ] **Verify on a GPU.** Build with `--features guardrail-cli/onnx-cuda`,
  run `onnx_real_models` and confirm the CUDA provider is used.
- [ ] **GPU benchmarks.** `onnx_concurrency` on the GPU for both models:
  single-request latency (64 B, 1 KB, a full 512-token window), throughput by
  pool size, and GPU memory per session. Fill in "With ONNX classifiers
  (CUDA): not measured" in [`benchmarks.md`](benchmarks.md#throughput-model).
- [ ] **Batching windows.** On a GPU, scoring all windows of a long input in
  one batched inference should be far cheaper than one inference per
  window. Prototype it and measure against the 16-window case.

## Planned changes (any machine)

- [ ] Size the automatic ONNX pools for all enabled stages together. Each
  stage takes `available CPUs / intra_threads` sessions (at most 4) as if it
  were alone, so with both stages enabled their threads add up: on
  4 CPUs with `intra_threads = 1`, 8 inference threads and 5.7 GiB; on
  8 cores × 2 threads with
  `intra_threads = 2`, 16 threads, which lost throughput at full load
  (6.9 → 6.1 requests/s) where 2 × 2 per stage held 6.6–6.7 with half the
  memory (see [benchmarks](benchmarks.md#through-the-proxy-both-stages)).
  Divide by the number of enabled ONNX stages, together with the decision
  on the `intra_threads` default.
- [ ] Admission control for each ONNX stage. Today requests wait for a
  session without limit, so a few long requests stall everyone behind them
  (see [long input](threat-model.md#long-input-holds-onnx-sessions)). Bound
  the wait queue per stage and answer `503` with `Retry-After` when it is
  full, ideally with permits weighted by window count, so that one 16-window
  request takes as much of the pool as 16 short ones.
- [ ] Reject input over `max_windows` without holding a session. The
  stage takes a session, then tokenizes all of the input to count windows,
  so a 10 MiB message holds a session for about 3 s and about 0.8 GB of
  memory before it is blocked (see
  [long input](threat-model.md#long-input-holds-onnx-sessions); one run).
  Stop tokenizing once the window count is over the limit, or check a cheap
  bound on the input length first.
- [ ] Edition 2024 and `resolver = "3"` (the env-override tests no longer
  call `set_var`, so the migration has nothing left to break), and update
  `.rustfmt.toml`'s `edition`.
- [ ] `[workspace.lints]` (`missing_docs = "deny"`, `unsafe_code = "forbid"`)
  with `[lints] workspace = true` in each crate, replacing the per-crate
  `#![deny(missing_docs)]`.
- [ ] `.rustfmt.toml`: remove the nightly-only `imports_granularity` and
  `group_imports` (ignored on stable).
- [ ] `[profile.release]` with `lto = "thin"`, `codegen-units = 1`,
  `strip = "symbols"`; re-measure the binary size quoted in the README.
- [ ] Dockerfile: correct the comment on `pkg-config` and `libssl-dev`.
  The default build is rustls-only, but ONNX builds need both:
  `ort-sys` → `ureq` → `native-tls` → `openssl-sys`, a build dependency
  that downloads ONNX Runtime (`cargo tree -i openssl-sys --features
  guardrail-cli/onnx`). Keep installing them.
- [ ] Repo artifacts: delete `.github/NEXT_PUSH_ISSUE.md`; rewrite the
  "the spec" / "§N" references in `CHANGELOG.md`, `audit.rs`,
  `audit_log.rs`, `error.rs`, `schema.rs`, `benches/pipeline.rs`, `justfile`
  and `guardrail-core/Cargo.toml` as plain rationale; remove machine details
  from `README.md`, `docs/benchmarks.md` and `docs/architecture.md`.
- [ ] Release 0.4.0 (version bump, CHANGELOG date, tag) once the above is in.
- [ ] Dependency upgrades: `prometheus` 0.14, `criterion` 0.8, `toml` 1.x,
  `rstest` 0.28, `ort` rc.15; evaluate `reqwest` 0.13 separately (its TLS
  defaults change).
- [ ] Redact streamed `/v1/responses` events. Today, with response
  redaction on, `stream: true` on that route is refused with `400`.
- [ ] Detection quality: normalize text before matching (NFKC, remove
  zero-width and bidi characters, map common look-alike characters) and
  broaden the injection phrasings ("prior", "preceding", "every" …), with an
  offset map so PII redaction still lands on the original text; add an
  evaluation harness over `tests/fixtures/*.json` that reports recall and
  false-positive rate per rule set and fails CI on regressions.
