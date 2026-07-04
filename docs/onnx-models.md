# Enabling ONNX Semantic Classifiers

The `onnx_injection` and `toxicity` stages provide semantic (embedding-based)
detection that catches attacks evading the bundled regex rules — paraphrased
jailbreaks, unusual phrasings, and non-English injection attempts. They are
**optional** and require:

1. Building `guardrail-rs` with the `onnx` Cargo feature.
2. Providing ONNX-exported model files and tokenizer configs.

## Building with ONNX support

```bash
cargo build --release -p guardrail-cli --features onnx
```

This pulls in the `ort` crate (ONNX Runtime bindings) and `tokenizers`
(HuggingFace tokenizers). On Linux, `ort` will download a prebuilt ONNX
Runtime shared library at build time unless you set
`ORT_LIB_LOCATION` to point at a system-installed copy.

For CUDA acceleration, use the `onnx-cuda` feature instead (requires the
CUDA toolkit and a compatible GPU):

```bash
cargo build --release -p guardrail-cli --features onnx-cuda
```

## Model recommendations

`guardrail-rs` does not bundle model weights (they're hundreds of MB and
under their own licenses). We recommend the following, both Apache-2.0
licensed:

| Stage | Recommended model | Source |
|-------|--------------------|--------|
| `onnx_injection` | `protectai/deberta-v3-base-prompt-injection-v2` | [HuggingFace](https://huggingface.co/protectai/deberta-v3-base-prompt-injection-v2) |
| `toxicity` | `unitary/unbiased-toxic-roberta` | [HuggingFace](https://huggingface.co/unitary/unbiased-toxic-roberta) |

## Exporting to ONNX

Use the HuggingFace `optimum` CLI to export a PyTorch model to ONNX:

```bash
pip install optimum[exporters]

optimum-cli export onnx \
  --model protectai/deberta-v3-base-prompt-injection-v2 \
  --task text-classification \
  models/prompt-injection/

optimum-cli export onnx \
  --model unitary/unbiased-toxic-roberta \
  --task text-classification \
  models/toxicity/
```

Each output directory contains `model.onnx` and `tokenizer.json` (among other
files). Point your configuration at these:

```toml
[stages.onnx_injection]
enabled = true
model_path = "models/prompt-injection/model.onnx"
tokenizer_path = "models/prompt-injection/tokenizer.json"
threshold = 0.85

[stages.toxicity]
enabled = true
model_path = "models/toxicity/model.onnx"
tokenizer_path = "models/toxicity/tokenizer.json"
threshold = 0.90
```

> **Note on `pip`/`optimum`:** this export step is a one-time, offline
> operation performed when *preparing* model artifacts — it is not part of
> the `guardrail-rs` runtime, which remains pure Rust with zero Python
> dependencies. See [`models/README.md`](../models/README.md) for a
> ready-to-use export script.

## Expected ONNX model interface

Both classifiers expect a sequence-classification model with:

- **Inputs:** `input_ids` (`int64[1, seq_len]`), `attention_mask` (`int64[1, seq_len]`)
- **Output:** `logits` (`float32[1, 2]`) — a 2-class logit pair, where index
  `1` is the positive class (`"injection"` or `"toxic"`). The classifier
  applies softmax internally to compute a probability in `[0, 1]`, compared
  against `threshold`.

If your model has a different output shape (e.g. multi-label toxicity with
more than 2 classes), you'll need a small wrapper script during export to
reduce it to this binary shape, or adjust
`crates/guardrail-classifiers/src/onnx.rs` accordingly (see
[`CONTRIBUTING.md`](../CONTRIBUTING.md) for the stage-implementation
contract).

## Tuning thresholds

Both `threshold` values default conservatively (`0.85` / `0.90`) to minimize
false positives. To tune for your traffic:

1. Set `log_only`-equivalent behavior by setting the threshold to `1.01`
   temporarily (never blocks) and reviewing `tracing::debug!` score logs
   (set `observability.log_level = "debug"`).
2. Collect a sample of scores on representative traffic.
3. Choose a threshold that separates your known-bad and known-good samples
   with acceptable false-positive/false-negative tradeoffs for your use case.

## Performance

Both stages run inference inside `tokio::task::spawn_blocking`, so they don't
block the async executor, but each call does consume a blocking-pool thread
for ~1–5 ms (CPU, 512 tokens). Under sustained high concurrency, monitor the
`guardrail_stage_duration_seconds{stage="onnx_injection"}` and
`{stage="toxicity"}` histograms (exposed at `/metrics`) and size
`tokio::runtime::Builder::max_blocking_threads` accordingly.
