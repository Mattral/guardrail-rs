# Architecture

## Overview

`guardrail-rs` is a reverse proxy structured as a six-crate Cargo workspace
(five publishable library/binary crates plus `guardrail-test-suite`, an
internal, unpublished crate for cross-crate integration tests and
benchmarks). Requests flow through a single in-process **request
pipeline** of **stages** before being forwarded upstream. For
non-streaming responses, an optional **response redaction pass** runs
before the response is returned to the caller.

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
  (carries a sanitized request, a human-readable `reason`, and a
  machine-readable `entities: Vec<String>` list of redacted entity types),
  or `Block` (carries a reason and machine-readable `BlockCode`).
  `Pipeline::run` aggregates per-stage decisions: it returns `Block`
  immediately on the first stage that blocks, but if one or more stages
  redact (and none block), it returns `Redact` as the **final** decision —
  joining every redacting stage's reason and de-duplicating their entity
  lists — not `Allow`. Only a run where no stage redacted or blocked
  returns `Allow`.
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

Decomposed into single-responsibility modules — each one is independently
testable, and the split keeps any one file from becoming an
everything-in-one-place dumping ground as the proxy grows:

- [`server`](../crates/guardrail-proxy/src/server.rs) — the listener
  lifecycle only: bind, accept loop, graceful shutdown via
  [`ServerHandle`]. Hands every accepted connection to `handler::handle_request`.
- [`handler`](../crates/guardrail-proxy/src/handler.rs) — per-connection
  routing (`/healthz`, `/metrics`, everything else) and the core proxy
  flow (`proxy_request`): authenticate → strip/forward headers → read body
  with a size limit → parse → run the pipeline → forward upstream or
  return a block response. `pub(crate)` — this is an internal
  implementation detail, not part of the crate's public API.
- [`auth`](../crates/guardrail-proxy/src/auth.rs) — caller authentication
  (`[auth]`) as a standalone, pure function (`is_authorized`) with no I/O
  or state beyond what's passed in. Extracted specifically so the
  authorization *decision* is unit-testable without a real TCP listener;
  `server`'s own tests cover the end-to-end HTTP behavior on top of it.
- [`error`](../crates/guardrail-proxy/src/error.rs) — HTTP error-response
  construction (`error_response`, `error_body_response`,
  `internal_error_response`), the size-limited body reader
  (`read_limited_body`), and upstream-error classification
  (`classify_upstream_error`, used for the `upstream_errors_total` metric
  label). Grouped together because they're all "how do we describe a
  failure," reviewable independently of routing/business logic.
- [`state`](../crates/guardrail-proxy/src/state.rs) — `AppState` (crate-internal:
  config handle, HTTP client, metrics registry) and the public
  [`ServerHandle`]. Pure data, no behavior — kept separate so it's obvious
  at a glance what every connection handler has access to.
- [`translate`](../crates/guardrail-proxy/src/translate.rs) — bidirectional
  conversion between raw JSON bodies and `GuardrailRequest`. Fields not
  understood by the normalized model (e.g. `temperature`, `tools`) are
  preserved in `GuardrailRequest::extra` and merged back on serialization —
  **lossless passthrough**.
- [`response`](../crates/guardrail-proxy/src/response.rs) — output-side PII
  redaction. `redact_response_body` walks OpenAI `choices[].message.content`
  and Anthropic `content[]` text blocks. Gated by `is_redactable_response`
  (non-streaming, `application/json` only). Streaming responses are not
  scanned — see [Streaming responses](#streaming-responses).
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
- [`audit_log`](../crates/guardrail-proxy/src/audit_log.rs) — true
  size-based rotating NDJSON file writer (`SizeRotatingWriter`, a custom
  `Write` impl — `tracing-appender`'s built-in rolling only supports
  time-based rotation, not the `max_size_mb` threshold this needs).
  `build_layer` returns a `tracing_subscriber::Layer` filtered to
  `target = "guardrail::audit"`. The caller must hold the returned
  `WorkerGuard` alive for the process lifetime.
- [`telemetry`](../crates/guardrail-proxy/src/telemetry.rs) —
  OpenTelemetry OTLP layer and per-request/stage span helpers, gated on
  `observability.otlp_endpoint` being non-empty.

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

## Streaming responses

Requests with `"stream": true` go through the full request pipeline like
any other request. The response side differs:

- **No output redaction.** `response::is_redactable_response` returns
  `false` for streaming requests and for `text/event-stream` bodies, so
  `redact_responses = true` has no effect on them. PII the model writes in a
  streamed answer reaches the caller as-is.
- **Not streamed incrementally.** `forward::read_body` reads the whole
  upstream body before the proxy responds (the handler builds a single
  `Full` body), so the client receives every SSE event at once when the
  upstream finishes, not token by token. Clients still parse it as a normal
  SSE stream, but time-to-first-token equals total generation time.

Options, none implemented yet:

1. A setting that rejects `stream: true` requests, or rewrites them to
   `stream: false`, when `redact_responses` is on, so the redaction promise
   holds for all traffic.
2. Since the body is already fully buffered, redact the text deltas inside
   the buffered SSE events before returning them.
3. Real streaming passthrough, with redaction applied per chunk and a small
   carry-over buffer for matches that span chunk boundaries.

## Error handling: fail-open vs fail-closed

Each `Stage::evaluate()` returns `Result<Decision, GuardrailError>`. When a
stage returns `Err`, the pipeline applies that stage's `FailMode`:

- `FailMode::Open` (`on_error = "allow"`, the default): log a
  `tracing::warn!`, treat the stage as having allowed, and continue with the
  next stage. The request goes upstream without that stage's check.
- `FailMode::Closed` (`on_error = "block"`): log a `tracing::error!` and stop
  with `Decision::Block`, code `stage_error`. The caller gets a `403`.

`guardrail-config` sets each stage's mode from its own `on_error` (the ONNX
stages) or, if that's unset, from `pipeline.on_error`. The regex, PII and
policy stages always use `pipeline.on_error`.

This only covers errors while handling a request. A model or rule file that
fails to load is a startup error: `guardrail run` exits, and a SIGHUP reload
is rejected and keeps the previous configuration.

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
