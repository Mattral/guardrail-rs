# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

**Caller authentication (`[auth]`) — enforced at runtime (spec §8/§9)**
- `server.rs` checks `X-Guardrail-Key` against `config.auth.keys` before
  reading the request body (fail fast). `/healthz` and `/metrics` are
  exempt. The header is stripped before forwarding upstream — the LLM
  provider never sees it.
- 7 new tests: missing key, wrong key, correct key (proves the request
  reaches the pipeline), health/metrics exemption, default-disabled
  passthrough, and a contract test confirming `x-guardrail-key` is in
  `STRIPPED_REQUEST_HEADERS`.
- `docs/threat-model.md` updated: caller authentication moved from
  "out of scope" to in-scope item 6, with residual-risk notes (not
  constant-time, no revocation list).

**`Classifier` backend trait (spec §7)**
- `guardrail_classifiers::classifier::Classifier<Input, Output>` —
  low-level, backend-agnostic interface for classification backends,
  decoupling stage implementations from execution environment.
- `RegexBackend` — wraps `RegexSet`; returns `RegexMatchResult` with
  matched indices and rule names. Always available, zero extra dependencies.
- `OnnxCpuBackend` — ONNX Runtime CPU execution provider, behind the
  `onnx` feature.
- `OnnxCudaBackend` — ONNX Runtime CUDA execution provider, behind the
  `onnx-cuda` feature.
- `ClassifierScore` — output type for binary (safe/unsafe) backends, with
  an `is_above_threshold(threshold)` helper.
- Comprehensive tests including rstest table-driven threshold cases and a
  proptest no-panic property.

**OpenTelemetry distributed tracing (spec §10)**
- `guardrail_proxy::telemetry` module: `build_otel_layer` returns a
  layered `tracing_subscriber` OTel OTLP exporter; `shutdown_tracer_provider`
  flushes buffered spans on exit.
- `request_span`, `stage_span`, `upstream_span` helpers produce
  correctly-named OTel spans with the appropriate field schema.
- `observability.otlp_endpoint` config field; validated as `http://`,
  `https://`, or `grpc://` if set.
- `guardrail-cli`'s `init_tracing` composes three layers: fmt (filtered
  by `log_level`), audit-log (target-filtered, `log_level`-independent),
  and OTel OTLP (endpoint-gated). Returns both a `WorkerGuard` and an
  `SdkTracerProvider` for correct process-lifetime ownership.
- `OtelError` error type with `ExporterBuild` and `ProviderInstall` variants.

**Audit record shape corrected to match spec §10 exactly**
- `AuditRecord` rewritten with: `timestamp` (ISO 8601, `Z`-suffix,
  millisecond precision, allocation-free — no `chrono` dependency),
  `stage`, `score`, `code`, `pii_entities_found`, `latency_pipeline_ms`,
  `latency_total_ms`.
- Builder-pattern `.with_score(f32)` / `.with_stage(&str)` for ONNX-stage
  enrichment.
- `PiiEntityType` and `RedactionRecord` derive `serde::Serialize` for
  audit-log JSON inclusion.

**Response-side PII redaction (output pipeline)**
- `PiiRedactor::redact_text_with_records` — unified implementation shared
  by both the request- and response-side redaction paths.
- `PiiRedactor::redact_response_text` — response-side entry point;
  returns `Option<(String, Vec<RedactionRecord>)>`.
- `guardrail_proxy::response` module — `redact_response_body` walks
  OpenAI `choices[].message.content` and Anthropic `content[]` text
  blocks; `is_redactable_response` gates redaction to non-streaming JSON
  responses only (streaming is explicitly out of scope — see
  `docs/architecture.md`).
- `stages.pii_redactor.redact_responses = true` config toggle.
- `maybe_redact_response` in `server.rs` wires response redaction into
  the request lifecycle.

**Structured NDJSON audit log with true size-based rotation**
- `guardrail_proxy::audit_log::build_layer` — a `tracing_subscriber::Layer`
  filtered to `target = "guardrail::audit"`, backed by a custom
  `SizeRotatingWriter`.
- `SizeRotatingWriter` checks `observability.audit_log.max_size_mb`
  before each write and, on exceeding it, flushes, renames the current
  file to `<path>.<unix_timestamp>` (with a collision-avoiding numeric
  suffix), and reopens a fresh file at `path`. Resumes the correct
  running size on restart against a pre-existing file. The file-handle
  field is `Option<File>` specifically so the old handle can be flushed
  and dropped via `.take()` *before* the rename syscall — renaming a file
  while a handle is open is fine on POSIX but can fail with
  access-denied on Windows, since `std::fs::File` doesn't request
  `FILE_SHARE_DELETE` by default. 6 unit tests cover rotation timing,
  content preservation across rotation, never rotating an empty file,
  resuming size on restart, and the `max_size_mb = 0` edge case.
- `observability.audit_log` config block: `enabled`, `path`, `max_size_mb`.
- `guardrail-cli` installs a layered tracing subscriber at startup: fmt
  layer + audit-log layer (target-filtered, independent of `log_level`)
  + OTel layer, returning a `WorkerGuard` held for the process lifetime.

**SIGHUP hot-reload (spec §14)**
- On Unix, `guardrail run` spawns a task that listens for SIGHUP and
  calls `ConfigHandle::reload()` without dropping any in-flight
  connections. Reload failures are logged; the previous configuration
  stays active.

**Environment variable overlay and `GUARDRAIL_CONFIG` (spec §14)**
- `GUARDRAIL_UPSTREAM`, `GUARDRAIL_PORT`, `GUARDRAIL_LOG_LEVEL`,
  `GUARDRAIL_OTLP_ENDPOINT` override the corresponding TOML fields at
  startup and on reload.
- `guardrail-cli`'s `--config` flag on all three subcommands (`run`,
  `validate`, `check`) falls back to the `GUARDRAIL_CONFIG` environment
  variable (via clap's `env` feature) if the flag isn't given, then to
  `guardrail.toml`, matching the spec's documented usage exactly. This
  had previously only been mentioned as a caveat in a code comment, never
  implemented.

**New Prometheus metrics**
- `guardrail_response_redacted_total`, `guardrail_request_duration_seconds`
  (full round-trip including upstream wait), `guardrail_upstream_errors_total{error_class}`
  (`timeout` / `connect` / `other`), `guardrail_active_connections` gauge.
- `guardrail_pipeline_duration_seconds` now accurately measures only
  pipeline evaluation time, not upstream wait.
- Grafana dashboard updated with panels for all new metrics, including an
  `active_connections` stat panel.

**Config schema rewritten to match spec §9 exactly**
- `[server] host/port/workers/max_body_size_bytes` with a
  `.listen_addr()` helper, `[upstream] url/timeout_secs/connect_timeout`,
  `[auth] require_key/keys`,
  `[pipeline] request_stages/response_stages/on_error` ordering arrays,
  per-stage `action: block|redact|log_only`,
  `[stages.pii_redactor.replacements]` custom tokens, `[[policy.rules]]`
  using the spec's `when`/`then` TOML nesting,
  `[observability.audit_log] path/max_size_mb`.
- `validate.rs` rewritten with full coverage: unknown stage IDs, unknown
  PII entity types, auth key requirements, ONNX model path existence,
  policy-rule "no condition set" detection, `log_format`/OTLP-scheme
  validation, audit-log path/size checks.
- `ConfigHandle::response_redactor() -> Arc<Option<PiiRedactor>>` for
  hot-reloadable response-side redaction; `loader::build_response_redactor`
  constructs it from the same entity list as the request-side stage.

**`GuardrailError::Upstream` error-type fix (spec §11)**
- Now `#[from] reqwest::Error` behind an optional `reqwest-errors`
  feature (enabled by default for `guardrail-proxy`), giving
  `?`-ergonomics and structured error inspection (`.is_timeout()`,
  `.is_connect()`) while preserving `guardrail-core`'s minimal dependency
  footprint for consumers who don't need `reqwest` at all.
- `classify_upstream_error` rewritten to use structured `reqwest::Error`
  inspection instead of `Display`-text matching; tests trigger real
  `reqwest` errors (non-routable address, connection-refused) instead of
  constructing synthetic string-based ones.

**Workspace-level full-pipeline benchmark (spec §13)**
- `crates/guardrail-test-suite/benches/pipeline.rs`: `bench_regex_stage`,
  `bench_full_pipeline_regex_only` (mirrors the spec's example almost
  verbatim), `bench_full_pipeline_by_size` (512B/4KB/8KB scaling),
  `bench_full_pipeline_blocked_short_circuit`. Lives in
  `guardrail-test-suite` rather than a root `benches/` directory since the
  workspace root is virtual and has no crate to attach a `[[bench]]`
  target to.
- `Pipeline::builder()` convenience constructor in `guardrail-core`
  (equivalent to `PipelineBuilder::default()`), added so benchmark and
  example code can match the spec's example syntax exactly.
- New CI job `pipeline-latency-gate` in `benchmarks.yml`: hard-fails if
  any full-pipeline benchmark case exceeds 5ms (5x the 1ms p99 target),
  separate from the existing soft 150%-regression alert on classifier
  microbenchmarks.
- `just bench-pipeline` / `just bench-all` recipes.

**Developer experience and examples (spec §14)**
- `tests/fixtures/`: `clean_prompts.json`, `injection_prompts.json`,
  `pii_prompts.json`, `policy_cases.json`, plus a `README.md` documenting
  shape and manual smoke-test usage.
- `docker-compose.yml`: added an Ollama service (zero-API-key local LLM
  backend for dev/CI) and a Grafana service wired to the existing dashboard.
- `examples/python_client.py`, `examples/node_client.js` — OpenAI SDK
  drop-in client examples.
- `examples/curl_test.sh` — bash smoke-test script with pass/fail assertions.
- `crates/guardrail-cli/examples/minimal.rs` — embeds the pipeline
  directly as a library with zero HTTP/network usage, distinct from the
  Python/Node examples (which both talk to a *running proxy* over HTTP).
- `crates/guardrail-cli/examples/custom_stage.rs` — full worked example
  of implementing and composing a custom `Stage`.
- `justfile` — comprehensive `just` task runner (build, test, lint,
  coverage, bench, security, run, docker, examples, docs, CI).
- `.config/nextest.toml` — `default` and `ci` profiles with retry,
  thread-count, and slow-timeout settings.
- `codecov.yml` — 80% project threshold, 70% patch threshold.
- `book.toml` + `docs/SUMMARY.md` — mdBook configuration.

**Documentation (spec §17)**
- `docs/threat-model.md` — in-scope threats and mitigations, residual
  risks, out-of-scope threats, and security properties.
- `docs/stage-api.md` — complete Stage API reference: contract,
  `Decision` table, block codes, minimal example, custom-stage wiring,
  `Classifier` backend integration, testing recipes.
- `docs/benchmarks.md` — performance targets, CI regression policy,
  latency tables, throughput model, benchmark instructions.
- `docs/configuration.md`, `docs/policy-rules.md`,
  `guardrail.example.toml` — fully rewritten to the new schema.
- All four library crates (`guardrail-core`, `guardrail-classifiers`,
  `guardrail-config`, `guardrail-proxy`) now have explicit "Further
  reading" links to the configuration reference, threat model, and
  changelog in their top-level rustdoc — closing a gap where none of
  them had this despite otherwise meeting the spec's other three
  crate-doc requirements. `guardrail-classifiers` and `guardrail-config`
  also gained a working doctest example and (for `guardrail-config`) a
  feature-flags table, neither of which existed before. `guardrail-proxy`
  gained `#![deny(missing_docs)]`, matching its sibling crates.

**CI/CD (spec §15)**
- `ci.yml`: `cargo nextest --profile ci`, JUnit artifact upload,
  Windows + macOS + beta-Rust matrix, `no-default-features` job, a fixed
  coverage job (single tarpaulin run → Codecov upload → 80% gate), a
  `docs` job (`rustdoc -D warnings` + mdBook build), and a
  `version-pin-check` job (see crates.io fixes below).
- `.github/workflows/audit.yml` — nightly `cargo audit` with automatic
  GitHub issue creation on findings.

### Changed
- `[stages.pii_redaction]` renamed to `[stages.pii_redactor]`; the old
  name is still accepted via `#[serde(alias = "pii_redaction")]` for
  backward-compatible TOML files.
- `observability.json_logs` (bool) replaced by `observability.log_format`
  (`"pretty"` | `"json"`).
- `AuditRecord::from_decision` signature now takes
  `pii_entities: &[String]`, `latency_pipeline_ms: f64`,
  `latency_total_ms: f64`.
- `forward.rs`'s `forward_request`/`read_body` use
  `.map_err(GuardrailError::from)` instead of manual string formatting.
- `PiiEntityType` and `RedactionRecord` now derive `serde::Serialize`
  (enables audit-log JSON inclusion); `RedactionRecord` documents the
  byte-offset caveat across multi-entity redaction passes.
- `examples/minimal.py` renamed to `examples/python_client.py` to match
  the spec's literal filename (§14); all cross-references updated.
- `crates/guardrail-classifiers/examples/custom_stage.rs` moved to
  `crates/guardrail-cli/examples/custom_stage.rs` to match the spec's
  directory tree (§14), which lists it alongside `minimal.rs` under
  `crates/guardrail-cli/examples/`. Costs nothing — `guardrail-cli`
  already depends on both `guardrail-core` and `guardrail-classifiers`
  directly.

### Fixed

**crates.io publish workflow — two real release-blocking bugs found and fixed**
- `release.yml`'s `publish` job claimed "Trusted Publishing" but actually
  used a long-lived `CARGO_REGISTRY_TOKEN` secret via
  `cargo publish --no-verify` — the opposite of Trusted Publishing. Fixed
  to use `rust-lang/crates-io-auth-action@v1` to mint a short-lived
  OIDC-derived token per run; `id-token: write` moved from the workflow
  level to the `publish` job only (least privilege); removed
  `--no-verify` since the `verify` job already gates this job via `needs:`.
- **Every internal `guardrail-* = { path = "..", ... }` dependency across
  all 5 publishable crates (14 occurrences) was missing the
  `version = "..."` field crates.io requires for path dependencies.**
  This would have made the very first `cargo publish` step fail on a real
  release tag. Added explicit version pins matching the workspace
  version, plus a new CI job (`version-pin-check`) that fails the build
  if any pin drifts from `[workspace.package].version` in future version
  bumps — this can't be expressed via `version.workspace = true` (that
  shorthand only applies to a crate's own `[package].version`, not to
  dependency requirements), so an automated guard replaces what would
  otherwise be an easy-to-forget manual sync step.

**Schema-migration cleanup**
- Removed ~250 lines of orphaned, duplicate test code in
  `guardrail-config/src/loader.rs` left over from an earlier incomplete
  schema migration (caused a 3-brace structural imbalance, verified clean
  via a string/comment-aware brace-balance scan across the entire codebase).
- Fixed a sed-mangled doctest in `build_response_redactor` that had
  broken out of its TOML string literal.
- Fixed all stale `server.upstream_url`/`server.listen_addr` and
  `[stages.pii_redaction]` references across `server.rs`, `commands.rs`,
  `README.md`, `examples/README.md`, and the `guardrail-test-suite`
  integration tests.
- Removed dead imports (`PolicyActionConfig`, `PolicyConditionConfig` at
  the loader's crate-level `use`) and a redundant shadowed `use` inside
  `convert_policy_rules`.
- Duplicate `AuditRecord` struct definition in `audit.rs` (old
  lifetime-based version) removed.

**Runtime correctness**
- `active_connections` gauge is now correctly incremented/decremented
  even when `service_fn` moves the `state` clone (fixed by pre-cloning
  `service_state`).
- `pipeline_duration_seconds` no longer double-observed at the end of
  `proxy_request`.
- `forward_to_upstream` now records `upstream_errors_total` on failure.

**Test-isolation race conditions**
- The `GUARDRAIL_CONFIG` tests in `cli.rs` were initially written as 5
  separate `#[test]` functions each mutating the process-global env
  var — consolidated into one sequential test, since `cargo
  test`/`nextest` run different test functions in parallel by default.
  The same pre-existing pattern (5 separate `#[test]` functions sharing
  `GUARDRAIL_UPSTREAM`/`GUARDRAIL_PORT` across pairs of tests) was found
  in `guardrail-config/src/loader.rs` and fixed identically.

**Repository hygiene**
- Removed two orphaned, empty directory trees left over from the
  project's very first scaffolding command: a literal directory named
  `{crates` (containing nested literal-brace subdirectories like
  `{guardrail-core/src,guardrail-classifiers/src,...}`) created when an
  early `mkdir -p {crates/{...},...}` brace-expansion command was run
  under a shell that doesn't perform brace expansion, and an empty
  `tests/integration/` directory whose real counterpart is
  `crates/guardrail-test-suite/tests/proxy_e2e.rs`. Neither contained any
  files; both were pure dead weight in the tree.
- Added `.dockerignore` (previously absent entirely) — without it, every
  `docker build` invocation tarred up and transmitted the entire
  repository, including `target/` (potentially gigabytes after a local
  `cargo build`), to the Docker daemon before the `Dockerfile`'s `COPY`
  instructions got a chance to ignore anything. Exclusions are anchored
  to the build-context root and verified against every `COPY`
  instruction in the `Dockerfile` to ensure nothing actually needed by
  the build was excluded.
- Added audit-log output patterns (`guardrail-audit.ndjson*`,
  `audit-logs/`) to `.gitignore` — the default
  `observability.audit_log.path` writes to the project root, and its
  rotated `<path>.<unix_timestamp>` backups would otherwise be one
  `git add .` away from being committed by a developer testing the
  feature locally.
- Stale `examples/README.md` Python/Node.js sections didn't link to the
  actual runnable `python_client.py`/`node_client.js`/`curl_test.sh`
  files, only showing inline code snippets — added explicit file
  pointers to all three.

[Unreleased]: https://github.com/Mattral/guardrail-rs/compare/v0.1.0...HEAD
