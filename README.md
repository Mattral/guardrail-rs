<div align="center">

# guardrail-rs

[![CI](https://img.shields.io/github/actions/workflow/status/Mattral/guardrail-rs/ci.yml?style=for-the-badge&logo=github&logoColor=white)](https://github.com/Mattral/guardrail-rs/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/guardrail-cli?style=for-the-badge&logo=rust&logoColor=white)](https://crates.io/crates/guardrail-cli)
[![docs.rs](https://img.shields.io/docsrs/guardrail-core?style=for-the-badge&logo=read-the-docs&logoColor=white)](https://docs.rs/guardrail-core)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue?style=for-the-badge&logo=opensourceinitiative&logoColor=white)](https://github.com/Mattral/guardrail-rs/blob/main/LICENSE-APACHE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange?style=for-the-badge&logo=rust&logoColor=white)](https://github.com/Mattral/guardrail-rs/blob/main/rust-toolchain.toml)
[![Open in Colab](https://img.shields.io/badge/Open%20in%20Colab-Notebook-yellow?style=for-the-badge&logo=googlecolab&logoColor=white)](https://colab.research.google.com/github/Mattral/guardrail-rs/blob/main/examples/notebooks/quickstart_colab.ipynb)

</div>

**A zero-Python LLM security proxy written in Rust.**

`guardrail-rs` is a reverse-proxy that sits between your application and an
LLM provider (OpenAI, Anthropic, Azure OpenAI, or any OpenAI-compatible
endpoint). It inspects every chat-completion request before it leaves your
infrastructure, blocking prompt injection attempts, redacting PII, and
enforcing custom policy rules. The default regex + PII pipeline took
47 µs on an 8 KB request in our measurement (see [Latency](#at-a-glance)).

## Installation

```bash
cargo install guardrail-cli
```

This builds and installs the `guardrail` binary from
[crates.io](https://crates.io/crates/guardrail-cli) (Rust 1.88+ required).
Every crate in this workspace — [`guardrail-core`](https://crates.io/crates/guardrail-core),
[`guardrail-classifiers`](https://crates.io/crates/guardrail-classifiers),
[`guardrail-config`](https://crates.io/crates/guardrail-config),
[`guardrail-proxy`](https://crates.io/crates/guardrail-proxy), and
[`guardrail-cli`](https://crates.io/crates/guardrail-cli) — is published
independently, so you can also depend on just `guardrail-core` to embed the
pipeline in your own Rust application with no HTTP server at all.

**Alternatives:**

```bash
# Docker (build locally from the Dockerfile in this repo)
docker build -t guardrail-rs .
docker run -p 8080:8080 -v $(pwd)/guardrail.toml:/etc/guardrail/guardrail.toml guardrail-rs

# ...or the full local stack (guardrail + Ollama + Prometheus + Grafana):
docker compose up -d

# Build from source
git clone https://github.com/Mattral/guardrail-rs.git && cd guardrail-rs
cargo install just  # one-time
just build
```

Want to try it in a couple of minutes with zero local setup? See
[Try it in Colab](#try-it-in-colab) below — it installs Rust, runs
`cargo install guardrail-cli` against the real crates.io release, and
walks through blocking a prompt injection and redacting PII live.

## At a glance

| | |
|---|---|
| **Threat model scope** | Prompt injection, PII leakage, toxic content, custom policy violations. PII is redacted from requests by default. Redacting PII from model *responses* is opt-in (`redact_responses = true`) and covers both complete and streamed (`stream: true`) responses; streams stay streamed, with PII redacted as tokens flow. Does *not* cover adversarial-ML evasion of the classifiers or post-model LLM behavior — full details in [`docs/threat-model.md`](docs/threat-model.md). |
| **Latency** | Default regex + PII pipeline, measured end to end (`Pipeline::run`): 4.7 µs at 512 B, 26.5 µs at 4 KB, 47.3 µs at 8 KB. The optional ONNX stages are far slower: 36–42 ms per stage for a 64 B prompt, 0.38–0.43 s at 1 KB. These figures are Criterion means from one local run (Intel Core Ultra 5 226V, Rust 1.99, Windows 11); CI's shared-runner medians are in [`docs/benchmarks.md`](docs/benchmarks.md), along with how to reproduce both. CI hard-fails if the regex + PII pipeline exceeds 5 ms. |
| **Fails open** | By default, a stage that errors on a request (e.g. ONNX inference fails) is skipped and the request continues without that check. Set `pipeline.on_error = "block"`, or `on_error` on an individual ONNX stage, to block instead. A model that fails to load stops startup rather than failing open. |
| **Observability** | Prometheus metrics at `/metrics` on the proxy's own port, OpenTelemetry traces (OTLP), and an NDJSON audit log — which never contains message content or API keys, only decision metadata. |

```mermaid
flowchart LR
    A["Your app<br/>(OpenAI/Anthropic SDK)"] -->|"POST /v1/chat/completions"| B["guardrail-rs<br/>(drop-in proxy)"]
    B --> C{{"Pipeline<br/>(stages)"}}
    C -->|"allow / redact"| D["Upstream LLM<br/>OpenAI · Anthropic · Azure · …"]
    C -->|"block"| E["403 + JSON error<br/>(never leaves your network)"]
```

## Why guardrail-rs?

- **No Python, no PyTorch, no GPU required.** A single static binary
  (~15 MB), built from the Rust source in this repository.
- **Fast without ONNX.** In the default configuration (regex + PII), the
  pipeline took 4.7 µs for a 512 B request and 47 µs for 8 KB in our
  measurement. Cost grows with input size, mostly from PII redaction. The
  optional ONNX classifiers cost tens to hundreds of milliseconds per
  request; see [Latency](#at-a-glance).
- **Drop-in.** Point your existing OpenAI/Anthropic SDK's `base_url` at
  `guardrail-rs` — no application code changes required.
- **Fails open by default.** A stage that errors on a request doesn't take
  down your production traffic; switch to fail-closed with
  `pipeline.on_error = "block"` or per ONNX stage with `on_error`.
- **Hot-reloadable configuration.** Update rules and policies without
  dropping connections.
- **Observable.** Structured audit logs (never logging raw PII) and
  Prometheus metrics out of the box.

## Quick start

Once installed (see [Installation](#installation) above):

```bash
# 1. Copy and edit the example configuration
cp guardrail.example.toml guardrail.toml
# edit guardrail.toml: set [upstream].url

# 2. Validate the configuration
guardrail validate --config guardrail.toml
# (from source: just validate / cargo run -p guardrail-cli -- validate --config guardrail.toml)

# 3. Run the proxy
guardrail run --config guardrail.toml
# (from source: just run / cargo run -p guardrail-cli -- run --config guardrail.toml)
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
| `onnx_injection` *(optional)* | DeBERTa-based semantic injection detection | target < 5 ms; **measured 41.9 ms at 64 B, 426 ms at 1 KB** (one local run, 1 CPU thread — [details](docs/benchmarks.md#onnx-classifiers-measured-and-far-over-target)) |
| `pii_redactor` | Detects & redacts emails, phone numbers, credit cards (Luhn-validated), SSNs, IPs, API keys | < 20 µs / 4 KB |
| `toxicity` *(optional)* | RoBERTa-based toxicity/harassment detection | target < 5 ms; **measured 36.4 ms at 64 B, 378 ms at 1 KB** (same run) |
| `policy_engine` | Your custom rules: keyword blocks, token-count limits, required-system-prompt checks | negligible |

The `onnx_injection` and `toxicity` stages are **experimental**. They
require building with `--features onnx` and providing ONNX model files (see
[`docs/onnx-models.md`](docs/onnx-models.md)). CI runs them against tiny
fixture models, which checks the code path but not detection quality. The
real models are only smoke-tested, manually. Everything else works with zero
external dependencies.

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

### Try it in Colab

[![Open In Colab](https://colab.research.google.com/assets/colab-badge.svg)](https://colab.research.google.com/github/Mattral/guardrail-rs/blob/main/examples/notebooks/quickstart_colab.ipynb)

No local setup required. The notebook installs Rust, runs
`cargo install guardrail-cli` against the real published crate on
crates.io, starts the proxy against a small local mock upstream (no API
key needed), and then sends three requests to show:

1. A clean request passing through untouched.
2. A prompt-injection attempt getting blocked with a `403` and the
   `prompt_injection` error code.
3. A message containing an email address getting redacted to `[EMAIL]`
   before it ever reaches the (mock) upstream.

First run takes a few minutes while Rust compiles the crate — that's the
cost of testing the real crates.io artifact rather than a pre-built binary.


---

## Related: GuardRail Studio (optional)

A separate control-plane app — dashboard, policy editor, audit log, metrics —
that drives this proxy. It does not reimplement detection.

- Repo: https://github.com/Mattral/GuardRail-Studio
- Talks to a running `guardrail` process (`/v1/chat/completions`, `/healthz`,
  `/metrics` on the same port, `guardrail.toml` + `SIGHUP`)
- Not required to use `guardrail-cli`

---

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
