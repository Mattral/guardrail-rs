# guardrail-rs

**A zero-Python, production-grade LLM security layer written in Rust.**

`guardrail-rs` is a reverse-proxy that sits between your application and an
LLM provider (OpenAI, Anthropic, Azure OpenAI, or any OpenAI-compatible
endpoint). It inspects every chat-completion request before it leaves your
infrastructure, blocking prompt injection attempts, redacting PII, and
enforcing custom policy rules — all with single-digit-millisecond overhead.

```text
┌──────────┐     ┌──────────────────┐     ┌────────────┐     ┌───────────────┐
│  Your    │ ──▶ │   guardrail-rs    │ ──▶ │  Pipeline   │ ──▶ │   OpenAI /     │
│  App     │     │  (drop-in proxy)  │     │  (Stages)   │     │  Anthropic /…  │
└──────────┘     └──────────────────┘     └────────────┘     └───────────────┘
                          │
                          ▼
                   403 + JSON error
                  (blocked requests
                   never leave your
                      network)
```

## Why guardrail-rs?

- **No Python, no PyTorch, no GPU required.** A single static binary
  (~15 MB), built from the Rust source in this repository.
- **Fast.** Regex injection scanning and PII redaction both run in
  single-digit microseconds; the full pipeline adds well under 1 ms p99 to
  request latency in the default (non-ONNX) configuration.
- **Drop-in.** Point your existing OpenAI/Anthropic SDK's `base_url` at
  `guardrail-rs` — no application code changes required.
- **Fails open by default.** A misbehaving stage never takes down your
  production traffic; configurable per-deployment via `pipeline.on_error`.
- **Hot-reloadable configuration.** Update rules and policies without
  dropping connections.
- **Observable.** Structured audit logs (never logging raw PII) and
  Prometheus metrics out of the box.

## Quick start

```bash
# 1. Copy and edit the example configuration
cp guardrail.example.toml guardrail.toml
# edit guardrail.toml: set server.upstream_url to your provider

# 2. Validate the configuration
cargo run -p guardrail-cli -- validate --config guardrail.toml

# 3. Run the proxy
cargo run -p guardrail-cli -- run --config guardrail.toml
```

Then point your application at `http://localhost:8080` instead of
`https://api.openai.com`:

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080/v1",
    api_key="sk-...",  # forwarded to the real upstream unchanged
)
```

## What gets checked

| Stage | What it does | Performance target |
|-------|--------------|---------------------|
| `regex_injection` | Fast regex scan for jailbreaks, prompt-extraction attempts, delimiter injection | < 50 µs / 8 KB |
| `onnx_injection` *(optional)* | DeBERTa-based semantic injection detection | < 5 ms / 512 tokens |
| `pii_redaction` | Detects & redacts emails, phone numbers, credit cards (Luhn-validated), SSNs, IPs, API keys | < 20 µs / 4 KB |
| `toxicity` *(optional)* | RoBERTa-based toxicity/harassment detection | < 5 ms / 512 tokens |
| `policy_engine` | Your custom rules: keyword blocks, token-count limits, required-system-prompt checks | negligible |

The `onnx_injection` and `toxicity` stages require building with
`--features onnx` and providing ONNX model files (see [`models/README.md`](models/README.md)).
Everything else works with zero external dependencies.

## CLI

```text
guardrail run --config guardrail.toml       # start the proxy
guardrail validate --config guardrail.toml  # check config without starting
guardrail check "some text" --config guardrail.toml
                                             # run text through the pipeline
                                             # and print the decision as JSON
```

## Configuration

See [`guardrail.example.toml`](guardrail.example.toml) for a fully-annotated
reference configuration, and [`docs/`](docs/) for detailed guides:

- [`docs/architecture.md`](docs/architecture.md) — pipeline design and stage contract
- [`docs/configuration.md`](docs/configuration.md) — full TOML schema reference
- [`docs/policy-rules.md`](docs/policy-rules.md) — writing custom policy rules
- [`docs/deployment.md`](docs/deployment.md) — Docker, Kubernetes, and bare-metal deployment
- [`docs/onnx-models.md`](docs/onnx-models.md) — enabling semantic classifiers

## Project layout

```text
crates/
  guardrail-core         # Pipeline trait, request/decision types, policy engine
  guardrail-classifiers  # Regex injection scanner, PII redactor, ONNX classifiers
  guardrail-config       # TOML config schema, validation, hot-reload
  guardrail-proxy        # HTTP server, request forwarding, metrics, audit log
  guardrail-cli          # `guardrail` binary
  guardrail-test-suite   # End-to-end integration tests
```

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo bench -p guardrail-classifiers
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

## Contributing

Contributions are welcome! Please see [`CONTRIBUTING.md`](CONTRIBUTING.md)
for guidelines on submitting issues and pull requests, including how to add
new prompt-injection rules or PII entity types.
