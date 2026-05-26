# Architecture

## Overview

`guardrail-rs` is a reverse proxy structured as a five-crate Cargo workspace.
Requests flow through a single in-process **request pipeline** of **stages**
before being forwarded upstream. For non-streaming responses, an optional
**response redaction pass** runs before the response is returned to the caller.

```text
                        ┌──────────────────────────────────────────────────┐
                        │                 guardrail-proxy                    │
                        │                                                     │
  Client ── HTTP ──────▶│ 1. Read & size-limit body                          │
                        │ 2. translate::parse_request → GuardrailRequest     │
                        │ 3. pipeline.run(req)  ◀──────── guardrail-core     │
                        │ 4a. Block  → JSON 403, audit event, return early   │
                        │ 4b. Allow/Redact → forward::forward_request        │
                        │ 5. [non-streaming] response::redact_response_body  │
                        │ 6. Return (possibly redacted) response to client   │
                        └──────────────────────────────────────────────────┘
```

## Crate responsibilities

### `guardrail-core`

The dependency-free heart of the system. Contains:

- [`GuardrailRequest`][req] — a normalized, provider-agnostic representation
  of a chat-completion request. Constructed once at ingress; passed by
  reference through the pipeline; mutated copies are produced by `Redact`
  decisions.
- [`Decision`][dec] — the three-way outcome of a stage: `Allow`, `Redact`
  (carries a sanitized request), or `Block` (carries a reason and machine
  -readable `BlockCode`).
- [`Stage`][stage] — the trait every classifier and the policy engine
  implement. `async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError>`.
- [`Pipeline`][pipeline] / [`PipelineBuilder`][builder] — runs stages in order,
  short-circuiting on the first `Block`.
- [`PolicyEngine`][policy] — a `Stage` implementation that evaluates
  user-defined `PolicyRule`s.

[req]: ../crates/guardrail-core/src/request.rs
[dec]: ../crates/guardrail-core/src/decision.rs
[stage]: ../crates/guardrail-core/src/pipeline.rs
[pipeline]: ../crates/guardrail-core/src/pipeline.rs
[builder]: ../crates/guardrail-core/src/pipeline.rs
[policy]: ../crates/guardrail-core/src/policy.rs

This crate has **no I/O** and **no async runtime dependency** beyond the
`async-trait` macro — `Stage::evaluate` is `async fn` so that ONNX-backed
stages can use `tokio::task::spawn_blocking` internally, but `guardrail-core`
itself never starts a runtime.

### `guardrail-classifiers`

Concrete `Stage` implementations:

- [`RegexInjectionScanner`](../crates/guardrail-classifiers/src/injection.rs) —
  wraps a `regex::RegexSet` over the bundled (or custom) rule file. Tests all
  patterns in one pass; `regex`'s automaton-based engine guarantees
  linear-time matching (no ReDoS).
- [`PiiRedactor`](../crates/guardrail-classifiers/src/pii.rs) — per-entity
  compiled regexes, with Luhn validation for credit card candidates. Returns
  `Redact` with a deep-cloned, sanitized `GuardrailRequest`.
- `OnnxInjectionClassifier` / `ToxicityClassifier` (behind the `onnx` feature)
  — load an ONNX session + HuggingFace tokenizer once at startup; inference
  runs in `spawn_blocking`.

### `guardrail-config`

- [`schema`](../crates/guardrail-config/src/schema.rs) — `serde`-deserializable
  TOML structure mirroring `guardrail.example.toml`.
- [`validate`](../crates/guardrail-config/src/validate.rs) — semantic
  validation beyond what `serde` can express (valid socket addresses, known
  PII entity names, threshold ranges, non-empty policy conditions, etc.).
- [`loader`](../crates/guardrail-config/src/loader.rs) —
  - `load_config()`: read + parse + validate.
  - `build_pipeline()`: construct a `Pipeline` from a validated `Config`, in
    the fixed stage order documented below.
  - `ConfigHandle`: wraps both `Config` and `Pipeline` in `ArcSwap` for
    lock-free reads on the hot path and atomic hot-reload.

### `guardrail-proxy`

- [`server`](../crates/guardrail-proxy/src/server.rs) — hyper 1.x + Tokio HTTP
  server. Routes `/healthz`, `/metrics`, and everything else through
  `proxy_request`. Tracks `active_connections` gauge around each served connection.
- [`translate`](../crates/guardrail-proxy/src/translate.rs) — bidirectional
  conversion between raw JSON bodies and `GuardrailRequest`. Fields not
  understood by the normalized model (e.g. `temperature`, `tools`) are
  preserved in `GuardrailRequest::extra` and merged back on serialization —
  **lossless passthrough**.
- [`response`](../crates/guardrail-proxy/src/response.rs) — output-side PII
  redaction. `redact_response_body` walks OpenAI `choices[].message.content`
  and Anthropic `content[]` text blocks. Gated by `is_redactable_response`
  (non-streaming, `application/json` only). Streaming responses are not yet
  scanned — see `docs/architecture.md` streaming roadmap note.
- [`forward`](../crates/guardrail-proxy/src/forward.rs) — sends the
  (possibly redacted) request upstream via a pooled `reqwest::Client`,
  forwarding the original `Authorization` header. Records
  `upstream_errors_total{error_class}` on failure.
- [`metrics`](../crates/guardrail-proxy/src/metrics.rs) — Prometheus
  collectors: request counts by decision/provider, block counts by code,
  request- and response-side redaction counts, pipeline-only and full-round-trip
  latency histograms (p50/p95/p99 via Grafana), per-stage latency, upstream
  error counts by class, and active connection gauge.
- [`audit`](../crates/guardrail-proxy/src/audit.rs) — structured `tracing`
  events at `target: "guardrail::audit"`. **Never logs raw request content**
  — only metadata (request ID, model, decision, reason, entity types).
- [`audit_log`](../crates/guardrail-proxy/src/audit_log.rs) — rotating NDJSON
  file writer. `build_layer` returns a `tracing_subscriber::Layer` filtered to
  `target = "guardrail::audit"` backed by a
  `tracing_appender::rolling::RollingFileAppender`. The caller must hold the
  returned `WorkerGuard` alive for the process lifetime.

### `guardrail-cli`

Thin `clap`-based binary (`guardrail`) wrapping `guardrail-config` and
`guardrail-proxy`: `run`, `validate`, `check`. The `run` subcommand installs
a layered tracing subscriber (fmt + optional NDJSON audit-log file layer) and,
on Unix, spawns a SIGHUP listener for automatic configuration hot-reload.

## Stage ordering and rationale

`build_pipeline()` constructs stages in this fixed order:

1. **`regex_injection`** — cheapest check, catches the bulk of attacks
   before any other work is done.
2. **`onnx_injection`** *(optional)* — semantic detection for attacks that
   evade regex. Runs before PII redaction so injection payloads containing
   fake PII don't waste redaction work on requests about to be blocked.
3. **`pii_redaction`** — sanitizes the request *before* the toxicity
   classifier sees it, so PII never appears in classifier logs/traces, and
   before forwarding upstream.
4. **`toxicity`** *(optional)* — slowest classifier; runs last among
   automated stages.
5. **`policy_engine`** — user-defined rules run last, after the request has
   been sanitized, so policy conditions like `content_contains` operate on
   (e.g.) the redacted text rather than raw PII.

## Error handling: fail-open vs fail-closed

Each `Stage::evaluate()` returns `Result<Decision, GuardrailError>`. The
`Pipeline::run()` loop catches stage errors and, by default
(`pipeline.on_error = "allow"`), converts them to `Decision::Allow` with a
`tracing::warn!`. Setting `pipeline.on_error = "block"` instead converts
top-level pipeline errors (e.g. an unexpected internal error in
`proxy_request` itself) to a `403 policy_violation` response.

This two-level design means: **individual stage failures always fail open**
(a broken ONNX model doesn't take down the proxy), while **catastrophic
pipeline failures** are configurable per-deployment.

## Hot reload

`ConfigHandle` holds three `ArcSwap`s: one for `Config`, one for `Pipeline`,
and one for `Option<PiiRedactor>` (the response-side redactor). `ConfigHandle::reload()`:

1. Re-reads and re-validates the TOML file (errors leave the existing config active).
2. Rebuilds the `Pipeline` and the response `PiiRedactor`.
3. Atomically swaps all three `ArcSwap`s.

On **Unix**, `guardrail run` spawns a task that listens for SIGHUP and calls
`reload()` automatically — no process restart required and no connections are
dropped. The tracing subscriber stack (fmt layer + audit-log layer) is
**not** hot-reloaded; changing `observability.*` settings still requires a
restart.

In-flight requests continue using their snapshot `Arc<Pipeline>` /
`Arc<Option<PiiRedactor>>` to completion; only new requests see the updated
configuration. There is no lock contention on the request hot path.
