# Roadmap

Open work, grouped by what it needs to run. Measurements go in
[`benchmarks.md`](benchmarks.md) when they are done; remove the item here.

## Needs a large machine (16 GB+ free RAM, 8+ cores)

Each ONNX session holds its own copy of the model (about 0.85 GB for the
DeBERTa injection model), so these don't fit on a small machine.

- [ ] **ONNX throughput past 4 sessions.** Re-run `onnx_concurrency` with
  `GUARDRAIL_BENCH_SESSIONS=1,2,4,8` and `GUARDRAIL_BENCH_CONCURRENCY=1,4,8,16`
  to show where scaling stops (cores or memory bandwidth). If it keeps
  scaling, revisit the automatic cap of 4 sessions
  (`onnx::MAX_DEFAULT_SESSIONS`).
- [ ] **`sessions` × `intra_threads` trade-off.** Same total threads, split
  differently (for example 8×1, 4×2, 2×4), to give tuning guidance in
  `configuration.md`: more sessions for throughput, more intra threads for
  latency.
- [ ] **Toxicity model under load.** Export
  `unitary/unbiased-toxic-roberta` to ONNX (`optimum-onnx` needs PyTorch,
  several GB; see [`onnx-models.md`](onnx-models.md)) and run the same
  concurrency benchmark for the `toxicity` stage.
- [ ] **Long-input cost.** Time a 16-window (about 6k-token) request on each
  real model. This is the worst case one request can cost, and the number
  operators need to choose `pipeline.stage_timeout_ms`.
- [ ] **Both ONNX stages through the proxy.** End-to-end load test with
  `onnx_injection` and `toxicity` enabled: requests/s, p50/p99, and resident
  memory with two session pools.
- [ ] **Real-model smoke tests.** Run `onnx_real_models` with both models:
  `GUARDRAIL_TEST_MODEL_DIR=… cargo test -p guardrail-classifiers --features onnx --test onnx_real_models -- --ignored`.

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

- [ ] Workspace and Cargo cleanup: internal crates as workspace
  dependencies, unused dependencies removed, per-crate `tokio` features,
  edition 2024, `[workspace.lints]`, a release profile, a smaller Dockerfile.
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
