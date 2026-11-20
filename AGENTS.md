# AGENTS.md

Instructions for AI coding agents working in this repository. Humans are
welcome to read it too; [CONTRIBUTING.md](CONTRIBUTING.md) is the longer
guide.

## What this is

guardrail-rs is a reverse proxy, written in Rust, that sits between an app
and an LLM provider. It checks each chat request (pattern rules for prompt
injection, PII redaction, user policy rules, optional ONNX classifiers),
then forwards, redacts or refuses it. It is experimental: detection rates
have not been measured, so never describe it as a reliable security
control.

## Crates

| Crate | What lives there |
|---|---|
| `guardrail-core` | `Stage` trait, `Pipeline`, request and `Decision` types, policy engine |
| `guardrail-classifiers` | regex injection scanner (`injection.rs`, rules in `src/rules/injection.rules`), PII redactor (`pii.rs`, `pii_stream.rs`), ONNX classifiers (`onnx.rs`, `classifier.rs`) |
| `guardrail-config` | TOML schema (`schema.rs`), validation (`validate.rs`), loading, hot reload and pipeline building (`loader.rs`) |
| `guardrail-proxy` | HTTP server, routing and forwarding, SSE, metrics, audit log, telemetry |
| `guardrail-cli` | the `guardrail` binary: `run`, `validate`, `check` |
| `guardrail-test-suite` | end-to-end tests against a mock upstream (`tests/`), pipeline benchmark |

## Commands

CI runs these; run the ones your change touches before calling it done.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --doc
cargo deny check                       # needs cargo-deny
```

With the ONNX feature (CI uses tiny fixture models; nothing to download):

```bash
cargo clippy --workspace --all-targets --features guardrail-cli/onnx -- -D warnings
cargo test --workspace --features guardrail-cli/onnx
```

Try the binary:

```bash
cargo run -p guardrail-cli -- validate --config guardrail.example.toml
cargo run -p guardrail-cli -- check "some text" --config guardrail.example.toml
```

Real ONNX models are optional and slow; the smoke tests are `#[ignore]`d:
`GUARDRAIL_TEST_MODEL_DIR=$PWD/models cargo test -p guardrail-classifiers --features onnx --test onnx_real_models -- --ignored`
(see `models/README.md` to get the models).

## Where to make common changes

- **New injection rule:** a line in
  `crates/guardrail-classifiers/src/rules/injection.rules`, plus positive
  and benign tests in `injection.rs`. Use ASCII word boundaries
  (`(?-u:\b)`): a Unicode `\b` makes scans of non-ASCII text about 100
  times slower, and user-supplied rules that use it are refused at load.
- **New PII entity:** see "Adding a new PII entity type" in CONTRIBUTING.md
  (four places must agree).
- **New config key:** the struct in `guardrail-config/src/schema.rs`,
  validation in `validate.rs`, the table in `docs/configuration.md`, and
  `guardrail.example.toml`, in the same change. Unknown keys are rejected
  (`deny_unknown_fields`), so a key missing from the schema fails every
  config that uses it.
- **New stage:** implement `guardrail_core::Stage` and wire it into
  `build_pipeline` in `guardrail-config/src/loader.rs`; see
  `docs/stage-api.md`.
- **Detection test data:** `tests/fixtures/*.json`.

## Rules for changes

- Don't change a default (for example `pipeline.on_error`, which is
  `"allow"`) without an "Upgrading" note in `CHANGELOG.md`.
- Add a `CHANGELOG.md` entry under `## [Unreleased]` for user-visible
  changes.
- Audit records must never carry message text, PII values or API keys.
  `crates/guardrail-test-suite/tests/audit_log_content.rs` checks this;
  keep it passing and extend it for new request paths.
- Docs claim only what is measured or tested. Numbers in docs say where
  they came from (`docs/benchmarks.md`).
- MSRV is Rust 1.88 (`rust-version` in `Cargo.toml`); CI checks it.
- Commit subjects follow Conventional Commits (`feat:`, `fix:`, `docs:` …).

## Known gotchas

- Tests that install a thread-local `tracing` subscriber need their own
  test binary: alongside other tests, the callsite interest cache can miss
  the subscriber and the test sees no events.
- `ort` allows one ONNX Runtime environment per process;
  `guardrail-classifiers` creates it once in `onnx.rs` (`environment()`).
- ONNX inference runs on blocking threads; never call it from async code
  directly.
