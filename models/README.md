# Models Directory

This directory is a placeholder for ONNX model artifacts used by the
optional `onnx_injection` and `toxicity` stages (see
[`docs/onnx-models.md`](../docs/onnx-models.md)).

**No model weights are checked into this repository.** This directory is
`.gitignore`d except for this README and the export helper script below.

## Quick export

```bash
./models/export_models.sh
```

This creates:

```text
models/
├── prompt-injection/
│   ├── model.onnx
│   └── tokenizer.json
└── toxicity/
    ├── model.onnx
    └── tokenizer.json
```

Then enable the stages in your `guardrail.toml`:

```toml
[stages.onnx_injection]
enabled = true
model_path = "models/prompt-injection/model.onnx"
tokenizer_path = "models/prompt-injection/tokenizer.json"

[stages.toxicity]
enabled = true
model_path = "models/toxicity/model.onnx"
tokenizer_path = "models/toxicity/tokenizer.json"
```

and build with `cargo build --features onnx`.
