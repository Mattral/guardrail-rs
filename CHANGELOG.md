# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

**True size-based audit log rotation**
- `crates/guardrail-proxy/src/audit_log.rs`: replaced
  `tracing_appender::rolling::never` (which ignored `max_size_mb` entirely)
  with a custom `SizeRotatingWriter` — a `Write` implementation that checks
  the configured size threshold before each write and, on exceeding it,
  flushes, renames the current file to `<path>.<unix_timestamp>` (with a
  collision-avoiding numeric suffix), and reopens a fresh file at `path`.
  Resumes the correct running size on restart against a pre-existing file.
  The file handle field is `Option<File>` specifically so the old handle
  can be flushed and dropped via `.take()` *before* the rename syscall —
  renaming a file while a handle is still open is fine on POSIX but can
  fail with access-denied on Windows, since `std::fs::File` doesn't
  request `FILE_SHARE_DELETE` by default. 6 new unit tests covering
  rotation timing, content preservation across rotation, never rotating an
  empty file, and the `max_size_mb = 0` edge case.

**`GUARDRAIL_CONFIG` environment variable (spec §14)**
- `guardrail-cli`'s `--config` flag on all three subcommands (`run`,
  `validate`, `check`) now falls back to `GUARDRAIL_CONFIG` via clap's `env`
  feature, then to `guardrail.toml`, matching the spec's documented
  `GUARDRAIL_CONFIG=examples/minimal.toml cargo run -p guardrail-cli` usage
  exactly. This had previously only been mentioned as a caveat in a code
  comment, never implemented.

**Crate-level documentation completeness (spec §17)**
- All four library crates (`guardrail-core`, `guardrail-classifiers`,
  `guardrail-config`, `guardrail-proxy`) now have explicit "Further
  reading" links to the configuration reference, threat model, and
  changelog in their top-level rustdoc, closing a gap where none of them
  had this despite otherwise meeting the spec's other three crate-doc
  requirements.
- `guardrail-classifiers` and `guardrail-config` gained a working doctest
  example and (for `guardrail-config`) a feature-flags table, neither of
  which existed before.
- `guardrail-proxy` gained `#![deny(missing_docs)]`, matching the other
  three crates, after a heuristic scan confirmed no undocumented public
  items would break the build under the new lint.

### Changed
- `examples/minimal.py` renamed to `examples/python_client.py` to match the
  spec's literal filename (§14); all cross-references updated.
- `crates/guardrail-classifiers/examples/custom_stage.rs` moved to
  `crates/guardrail-cli/examples/custom_stage.rs` to match the spec's
  directory tree (§14), which lists it alongside `minimal.rs` under
  `crates/guardrail-cli/examples/`. Costs nothing — `guardrail-cli` already
  depends on both `guardrail-core` and `guardrail-classifiers` directly.

### Fixed
- Test-isolation race condition: the new `GUARDRAIL_CONFIG` tests in
  `cli.rs` were initially written as 5 separate `#[test]` functions each
  mutating the process-global env var — consolidated into one sequential
  test, since `cargo test`/`nextest` run different test functions in
  parallel by default. The same pre-existing pattern (5 separate
  `#[test]` functions sharing `GUARDRAIL_UPSTREAM`/`GUARDRAIL_PORT` across
  pairs of tests) was found in `guardrail-config/src/loader.rs` from an
  earlier session and fixed identically.
- Stale `examples/README.md` Python/Node.js sections didn't link to the
  actual runnable `python_client.py`/`node_client.js`/`curl_test.sh`
  files, only showing inline code snippets — added explicit file pointers
  to all three.

[Unreleased]: https://github.com/Mattral/guardrail-rs/compare/v0.1.0...HEAD
- `crates/guardrail-test-suite/benches/pipeline.rs`: `bench_regex_stage`,
  `bench_full_pipeline_regex_only` (mirrors the spec's example almost
  verbatim), `bench_full_pipeline_by_size` (512B/4KB/8KB scaling),
  `bench_full_pipeline_blocked_short_circuit`. Lives in
  `guardrail-test-suite` rather than a root `benches/` directory since the
  workspace root is virtual and has no crate to attach a `[[bench]]` target
  to.
- `Pipeline::builder()` convenience constructor in `guardrail-core`
  (equivalent to `PipelineBuilder::default()`), added so the new benchmark
  and example code can match the spec's example syntax exactly.
- New CI job `pipeline-latency-gate` in `benchmarks.yml`: hard-fails if any
  full-pipeline benchmark case exceeds 5ms (5x the 1ms p99 target),
  separate from the existing soft 150%-regression alert on classifier
  microbenchmarks.
- `just bench-pipeline` / `just bench-all` recipes.

**Rust embedding example (spec §14)**
- `crates/guardrail-cli/examples/minimal.rs`: demonstrates embedding the
  pipeline directly as a library with zero HTTP/network usage — the Rust
  counterpart to `examples/minimal.py`/`node_client.js`, which both talk to
  a *running proxy* over HTTP rather than embedding the library directly.

### Fixed

**crates.io publish workflow — found and fixed two real release-blocking bugs**
- `release.yml`'s `publish` job claimed "Trusted Publishing" but actually
  used a long-lived `CARGO_REGISTRY_TOKEN` secret via
  `cargo publish --no-verify` — the opposite of Trusted Publishing. Fixed to
  use `rust-lang/crates-io-auth-action@v1` to mint a short-lived
  OIDC-derived token per run; `id-token: write` permission moved from the
  workflow level to the `publish` job only (least privilege); removed
  `--no-verify` since the `verify` job already gates this job via `needs:`.
- **Every internal `guardrail-* = { path = "..", ... }` dependency across
  all 5 publishable crates (14 occurrences) was missing the
  `version = "..."` field crates.io requires for path dependencies.** This
  would have made the very first `cargo publish` step fail on a real
  release tag. Added explicit version pins matching the workspace version,
  plus a new CI job (`version-pin-check`) that fails the build if any pin
  drifts from `[workspace.package].version` in future version bumps — this
  cannot be expressed via `version.workspace = true` (that shorthand only
  applies to a crate's own `[package].version`, not to dependency
  requirements), so an automated guard replaces what would otherwise be a
  manual, easy-to-forget sync step.
- Fixed remaining stale `server.upstream_url`/`server.listen_addr` and
  `pii_redaction` references in `README.md` and `examples/README.md`.

[Unreleased]: https://github.com/Mattral/guardrail-rs/compare/v0.1.0...HEAD
- Config schema fully rewritten: `[server] host/port/workers/max_body_size_bytes`
  with `.listen_addr()` helper, `[upstream] url/timeout_secs/connect_timeout`,
  `[auth] require_key/keys`, `[pipeline] request_stages/response_stages/on_error`
  ordering arrays, per-stage `action: block|redact|log_only`,
  `[stages.pii_redactor.replacements]` custom tokens, `[[policy.rules]]` using
  the spec's `when`/`then` TOML nesting, `[observability.audit_log] path/max_size_mb`.
- `validate.rs` rewritten with full coverage: unknown stage IDs, unknown PII
  entity types, auth key requirements, ONNX model path existence, policy
  rule "no condition set" detection, OTLP scheme validation, audit log
  path/size checks.

**Caller authentication (`[auth]`) — now enforced at runtime**
- `server.rs` checks `X-Guardrail-Key` against `config.auth.keys` before
  reading the request body (fail fast). `/healthz` and `/metrics` are
  exempt. The header is stripped before forwarding upstream — the LLM
  provider never sees it.
- 7 new tests: missing key, wrong key, correct key (proves request reaches
  the pipeline), health/metrics exemption, default-disabled passthrough,
  and a contract test confirming `x-guardrail-key` is in
  `STRIPPED_REQUEST_HEADERS`.
- `docs/threat-model.md` updated: caller authentication moved from
  "out of scope" to in-scope item 6, with residual-risk notes (not
  constant-time, no revocation list).

**`GuardrailError::Upstream` error-type fix (spec §11)**
- Now `#[from] reqwest::Error` behind an optional `reqwest-errors` feature
  (enabled by default for `guardrail-proxy`), giving `?`-ergonomics and
  structured error inspection (`.is_timeout()`, `.is_connect()`) while
  preserving `guardrail-core`'s minimal dependency footprint for consumers
  who don't need `reqwest` at all.
- `classify_upstream_error` rewritten to use structured `reqwest::Error`
  inspection instead of `Display`-text matching; tests now trigger real
  `reqwest` errors (non-routable address, connection-refused) instead of
  constructing synthetic string-based errors.

**Documentation rewrites to match new schema**
- `docs/configuration.md` — full field-by-field rewrite covering every new
  section (`[auth]`, `[upstream]`, ordering arrays, replacements, audit log).
- `docs/policy-rules.md` — all examples converted to `when`/`then` shape.
- `guardrail.example.toml` — fully rewritten, annotated for every new field.

**Developer experience**
- `tests/fixtures/`: `clean_prompts.json`, `injection_prompts.json`,
  `pii_prompts.json`, `policy_cases.json`, plus a `README.md` documenting
  shape and manual smoke-test usage.
- `docker-compose.yml`: added Ollama service (zero-API-key local LLM
  backend for dev/CI) and a Grafana service wired to the existing dashboard.

### Changed
- `[stages.pii_redaction]` renamed to `[stages.pii_redactor]`; the old name
  is still accepted via `#[serde(alias = "pii_redaction")]` for
  backward-compatible TOML files.
- `forward.rs`'s `forward_request`/`read_body` use `.map_err(GuardrailError::from)`
  instead of manual string formatting.

### Fixed
- Removed ~250 lines of orphaned, duplicate test code in
  `guardrail-config/src/loader.rs` left over from an earlier incomplete
  schema migration (caused a 3-brace structural imbalance, now verified
  clean via a string/comment-aware brace-balance scan across the entire
  codebase).
- Fixed a sed-mangled doctest in `build_response_redactor` that had broken
  out of its TOML string literal.
- Fixed all stale `[server].listen_addr`/`upstream_url` and
  `[stages.pii_redaction]` references across `server.rs`, `commands.rs`,
  and the `guardrail-test-suite` integration tests.
- Removed dead imports (`PolicyActionConfig`, `PolicyConditionConfig` at the
  loader's crate-level `use`) and a redundant shadowed `use` inside
  `convert_policy_rules`.

[Unreleased]: https://github.com/Mattral/guardrail-rs/compare/v0.1.0...HEAD
- `guardrail_classifiers::classifier::Classifier<Input, Output>` — low-level, backend-agnostic interface for classification backends, enabling stage implementations to be decoupled from execution environment.
- `RegexBackend` — wraps `RegexSet`; returns `RegexMatchResult` with matched indices and rule names. Always available, zero extra dependencies.
- `OnnxCpuBackend` — ONNX Runtime CPU execution provider, behind `onnx` feature.
- `OnnxCudaBackend` — ONNX Runtime CUDA execution provider, behind `onnx-cuda` feature.
- `ClassifierScore` — output type for binary (safe/unsafe) backends, with `is_above_threshold(threshold)` helper.
- Comprehensive tests including rstest table-driven threshold cases and proptest no-panic property.

**OpenTelemetry distributed tracing (§10)**
- `guardrail_proxy::telemetry` module: `build_otel_layer` returns a layered `tracing_subscriber` OTel OTLP exporter; `shutdown_tracer_provider` for graceful span flush on exit.
- `request_span`, `stage_span`, `upstream_span` helpers produce correctly-named OTel spans with appropriate field schema.
- `observability.otlp_endpoint` config field; validated as `http://`, `https://`, or `grpc://` if set.
- `guardrail-cli::commands::init_tracing` now composes three layers: fmt (with per-layer env-filter), audit-log (target-filtered), OTel OTLP (endpoint-gated). Returns both `WorkerGuard` and `SdkTracerProvider` for proper lifecycle management.
- `OtelError` error type with `ExporterBuild` and `ProviderInstall` variants.

**Corrected audit record shape (§10)**
- `AuditRecord` completely rewritten to match spec §10 exactly: `timestamp` (ISO 8601 `Z`-suffix), `stage`, `score`, `code`, `pii_entities_found`, `latency_pipeline_ms`, `latency_total_ms`.
- Builder-pattern `.with_score(f32)` and `.with_stage(&str)` for ONNX stage enrichment.
- Custom allocation-free timestamp implementation (`unix_secs_to_datetime`) — no `chrono` dependency.
- `PiiEntityType` and `RedactionRecord` derive `serde::Serialize` for JSON audit log inclusion.

**Response pipeline (output PII redaction)**
- `PiiRedactor::redact_text_with_records` — unified implementation shared by request and response paths.
- `PiiRedactor::redact_response_text` — response-side entry point, returns `Option<(String, Vec<RedactionRecord>)>`.
- `guardrail_proxy::response` — `redact_response_body` (OpenAI + Anthropic shape), `is_redactable_response`.
- `stages.pii_redaction.redact_responses = true` toggle.
- `maybe_redact_response` in `server.rs` integrates response redaction into the request lifecycle.

**NDJSON audit log (§10)**
- `guardrail_proxy::audit_log::build_layer` — NDJSON file layer filtered to `guardrail::audit` target.
- `observability.audit_log` config block with `enabled`, `directory`, `file_name_prefix`, `rotation`.
- `AuditLogConfig` with full validation in `validate_config`.

**SIGHUP hot-reload (§14)**
- `guardrail run` spawns a SIGHUP listener on Unix that calls `ConfigHandle::reload()` atomically.

**Environment variable overlay (§14)**
- `GUARDRAIL_UPSTREAM`, `GUARDRAIL_PORT`, `GUARDRAIL_LOG_LEVEL`, `GUARDRAIL_OTLP_ENDPOINT` override TOML values at startup and on reload.

**New Prometheus metrics (§10)**
- `guardrail_response_redacted_total`, `guardrail_request_duration_seconds`, `guardrail_upstream_errors_total{error_class}`, `guardrail_active_connections` gauge.
- `pipeline_duration_seconds` now accurately measures only pipeline evaluation (not upstream wait).
- Grafana dashboard updated with panels for all new metrics including `active_connections` stat panel.

**Configuration schema additions**
- `observability.log_format` (`"pretty"` | `"json"`; replaces `json_logs` bool).
- `observability.otlp_endpoint`, `observability.metrics_port`.
- Validation tests for all new fields.

**Developer experience (§14)**
- `justfile` — comprehensive `just` task runner with build, test, lint, coverage, bench, security, run, docker, model, example, docs, and CI recipes.
- `.config/nextest.toml` — nextest `default` and `ci` profiles with retry, thread limits, and slow-timeout settings.
- `codecov.yml` — 80% project threshold, 70% patch threshold, ignores test helpers and examples.
- `examples/minimal.py` — Python OpenAI SDK drop-in example.
- `examples/node_client.js` — Node.js OpenAI SDK example.
- `examples/curl_test.sh` — bash smoke-test script with pass/fail assertions.
- `crates/guardrail-classifiers/examples/custom_stage.rs` — full worked example of a custom `Stage` implementation.
- `book.toml` + `docs/SUMMARY.md` — mdBook configuration for publishing documentation.

**Documentation (§17)**
- `docs/threat-model.md` — in-scope threats, mitigations, residual risks, out-of-scope threats, and security properties.
- `docs/stage-api.md` — complete Stage API reference with contract, `Decision` table, block codes, minimal example, custom stage wiring, `Classifier` backend integration, and testing recipes.
- `docs/benchmarks.md` — performance targets, CI regression policy, latency tables, throughput model, and benchmark instructions.

**CI/CD (§15)**
- `ci.yml` updated: nextest `--profile ci`, JUnit artifact upload, Windows + macOS + beta Rust matrix, `no-default-features` job, fixed coverage job (single tarpaulin run → Codecov upload → 80% gate), `docs` job (rustdoc `-D warnings` + mdBook build).
- `.github/workflows/audit.yml` — nightly `cargo audit` with automatic GitHub issue creation on findings.

### Changed
- `observability.json_logs` (bool) replaced by `observability.log_format` (`"pretty"` | `"json"`).
- `AuditRecord::from_decision` signature now takes `pii_entities: &[String]`, `latency_pipeline_ms: f64`, `latency_total_ms: f64`.
- `guardrail_proxy::audit_log::build_layer` test fixed to use new 5-arg `from_decision`.

### Fixed
- Duplicate `AuditRecord` struct definition in `audit.rs` (old lifetime-based version removed).
- `active_connections` gauge correctly decremented even when `service_fn` moves the `state` clone (fixed by pre-cloning `service_state`).
- `pipeline_duration_seconds` no longer double-observed at end of `proxy_request`.
- `AuditRecord::from_decision` call in `audit_log.rs` test updated to match new signature.

[Unreleased]: https://github.com/Mattral/guardrail-rs/compare/v0.1.0...HEAD

- `PiiRedactor::redact_response_text` — scan free-form LLM output for PII before returning it to callers.
- `PiiRedactor::redact_text_with_records` — unified implementation that returns both the sanitized text and detailed `RedactionRecord`s; used by both request- and response-side redaction paths.
- `guardrail-proxy::response` module — `redact_response_body` walks OpenAI `choices[].message.content` and Anthropic `content[]` text blocks; `is_redactable_response` gates redaction to non-streaming JSON responses.
- `stages.pii_redaction.redact_responses = true` config toggle to enable response-side PII redaction.

**NDJSON audit log with rotation**
- `guardrail-proxy::audit_log` module — `build_layer` constructs a `tracing_subscriber::Layer` filtered to `target = "guardrail::audit"` that writes NDJSON records to a `tracing_appender::rolling::RollingFileAppender`.
- `observability.audit_log` config block: `enabled`, `directory`, `file_name_prefix`, `rotation` (`minutely` / `hourly` / `daily` / `never`).
- `AuditLogConfig` struct in `guardrail-config::schema`, with full validation in `validate_config`.
- `guardrail-cli` installs a **layered** tracing subscriber at startup: fmt layer (filtered by `log_level`) + audit-log layer (target-filtered, `env_filter`-independent), returning a `WorkerGuard` held for process lifetime.

**SIGHUP hot-reload**
- On Unix, `guardrail run` now spawns a dedicated task that listens for SIGHUP and calls `ConfigHandle::reload()` without dropping any connections. Reload failures are logged and the previous configuration stays active.

**New Prometheus metrics**
- `guardrail_response_redacted_total` — response-side PII redaction counter.
- `guardrail_request_duration_seconds{decision}` — end-to-end latency including upstream wait time.
- `guardrail_upstream_errors_total{error_class}` — upstream failures labeled `timeout` / `connect` / `other`.
- `guardrail_active_connections` — in-flight connection gauge.
- `guardrail_pipeline_duration_seconds` now accurately measures only pipeline evaluation time (not upstream); `request_duration_seconds` measures the full round-trip.
- Grafana dashboard updated with panels for all new metrics.

**Config schema additions**
- `stages.pii_redaction.redact_responses` — opt-in response PII redaction.
- `observability.audit_log` block.
- Validation for both fields; tests for all new validation paths.

**`ConfigHandle` additions**
- `ConfigHandle::response_redactor() -> Arc<Option<PiiRedactor>>` — hot-reloadable response redactor.
- `loader::build_response_redactor(config)` — constructs the response-side `PiiRedactor` from the same entity list as the request-side stage.

### Changed
- `PiiEntityType` now derives `serde::Serialize` (enables `RedactionRecord` serialization for audit log).
- `RedactionRecord` now derives `serde::Serialize` and documents the offset caveat across multi-entity passes.
- `init_tracing` in `guardrail-cli` now uses a layered `Registry`-based subscriber instead of `fmt::Subscriber::builder()`, enabling layer composition.

### Fixed
- `forward_to_upstream` now records `upstream_errors_total` on failure.
- `active_connections` gauge is incremented/decremented correctly even when `service_fn` moves the `state` clone.
- `pipeline_duration_seconds` no longer double-counts (was erroneously re-observed at end of `proxy_request`).

[Unreleased]: https://github.com/Mattral/guardrail-rs/compare/v0.1.0...HEAD
