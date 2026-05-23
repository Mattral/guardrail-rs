# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

**Response pipeline (output PII redaction)**
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
