# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial public release of `guardrail-rs`.
- `guardrail-core`: pipeline abstraction, `Stage` trait, `Decision` enum,
  normalized `GuardrailRequest` model, and policy engine.
- `guardrail-classifiers`: regex-based prompt injection scanner with bundled
  rule set; PII redactor covering email, phone, credit card (Luhn-validated),
  SSN, IP address, API key, and AWS key entities.
- `guardrail-classifiers`: optional `onnx` feature providing
  `OnnxInjectionClassifier` (DeBERTa-based) and `ToxicityClassifier`
  (RoBERTa-based) stages.
- `guardrail-config`: TOML configuration schema, validation, and hot-reload
  via `ConfigHandle`.
- `guardrail-proxy`: hyper-based HTTP reverse proxy with OpenAI- and
  Anthropic-shaped request translation, Prometheus metrics endpoint,
  structured audit logging, and graceful shutdown.
- `guardrail-cli`: `guardrail run`, `guardrail validate`, and `guardrail check`
  subcommands.
- End-to-end integration test suite (`guardrail-test-suite`) using `wiremock`
  to simulate upstream providers.
- CI workflows for build, test, lint (clippy + rustfmt), license/advisory
  checks (cargo-deny), and benchmark tracking.

[Unreleased]: https://github.com/Mattral/guardrail-rs/compare/v0.1.0...HEAD
