# guardrail-rs

**A zero-Python, production-grade LLM security layer written in Rust.**

`guardrail-rs` is a reverse-proxy that sits between your application and an
LLM provider (OpenAI, Anthropic, Azure OpenAI, or any OpenAI-compatible
endpoint). It inspects every chat-completion request before it leaves your
infrastructure, blocking prompt injection attempts, redacting PII, and
enforcing custom policy rules — all with single-digit-millisecond overhead.

```text
┌──────────┐     ┌──────────────────┐     ┌────────────┐     ┌───────────────┐
│  Your    │ ──▶│   guardrail-rs   │ ──▶ │  Pipeline  │ ──▶│   OpenAI /    │
│  App     │     │  (drop-in proxy) │     │  (Stages)  │     │  Anthropic /… │
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
# edit guardrail.toml: set [upstream].url

# 2. Validate the configuration
just validate
# or: cargo run -p guardrail-cli -- validate --config guardrail.toml

# 3. Run the proxy
just run
# or: cargo run -p guardrail-cli -- run --config guardrail.toml
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

### Environment variable overrides

Configuration can be overridden without editing `guardrail.toml`:

| Variable | Overrides |
|----------|-----------|
| `GUARDRAIL_UPSTREAM` | `upstream.url` |
| `GUARDRAIL_PORT` | `server.port` |
| `GUARDRAIL_LOG_LEVEL` | `observability.log_level` |
| `GUARDRAIL_OTLP_ENDPOINT` | `observability.otlp_endpoint` |

### Hot reload (Unix)

Send `SIGHUP` to reload configuration without dropping connections:

```bash
pkill -HUP guardrail
# or: just reload
```

## What gets checked

| Stage | What it does | Performance target |
|-------|--------------|---------------------|
| `regex_injection` | Fast regex scan for jailbreaks, prompt-extraction attempts, delimiter injection | < 50 µs / 8 KB |
| `onnx_injection` *(optional)* | DeBERTa-based semantic injection detection | < 5 ms / 512 tokens |
| `pii_redactor` | Detects & redacts emails, phone numbers, credit cards (Luhn-validated), SSNs, IPs, API keys | < 20 µs / 4 KB |
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
- [`docs/threat-model.md`](docs/threat-model.md) — what guardrail-rs protects against (and what it doesn't)
- [`docs/stage-api.md`](docs/stage-api.md) — implementing custom pipeline stages
- [`docs/benchmarks.md`](docs/benchmarks.md) — performance targets and how to run benchmarks

## Examples

See [`examples/README.md`](examples/README.md) for client examples (curl,
Python, Node.js, Anthropic SDK) against a running proxy. To embed
guardrail-rs as a library with no HTTP server at all:

```bash
cargo run --example minimal -p guardrail-cli
```

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
# Install just (task runner)
cargo install just

# Build
just build

# Test (requires cargo-nextest: cargo install cargo-nextest)
just test

# Lint
just lint

# Format
just fmt

# Full CI check locally
just ci

# Generate coverage report (requires cargo-tarpaulin)
just coverage
```

See `justfile` for all available recipes (`just --list`).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

## Contributing

Contributions are welcome! Please see [`CONTRIBUTING.md`](CONTRIBUTING.md)
for guidelines on submitting issues and pull requests, including how to add
new prompt-injection rules or PII entity types.
