//! # guardrail-config
//!
//! Configuration types, TOML loading, environment-variable overlay, and
//! validation for `guardrail-rs`.
//!
//! See [`guardrail.example.toml`][example] for the full annotated schema.
//!
//! [example]: https://github.com/Mattral/guardrail-rs/blob/main/guardrail.example.toml
//!
//! ## Feature flags
//!
//! | Flag | Description | Default |
//! |------|-------------|---------|
//! | *(none)* | Base TOML loading, validation, and env-var overlay | — |
//! | `onnx` | Enables building ONNX-backed stages (`onnx_injection`, `toxicity`) when `[stages.onnx_injection]`/`[stages.toxicity]` are enabled in the TOML | off |
//!
//! ## Quick example
//!
//! ```rust
//! use guardrail_config::loader::{load_config, build_pipeline};
//! use std::io::Write;
//!
//! # let mut f = tempfile::NamedTempFile::new().unwrap();
//! # f.write_all(br#"
//! # [server]
//! # host = "0.0.0.0"
//! # port = 8080
//! #
//! # [upstream]
//! # url = "https://api.openai.com"
//! # "#).unwrap();
//! let config = load_config(f.path()).unwrap();
//! let pipeline = build_pipeline(&config).unwrap();
//! assert!(pipeline.len() > 0); // regex_injection + pii_redactor enabled by default
//! ```
//!
//! ## Further reading
//!
//! - [Configuration reference](https://github.com/Mattral/guardrail-rs/blob/main/docs/configuration.md) —
//!   field-by-field documentation of every TOML key this crate parses and validates.
//! - [Policy rules guide](https://github.com/Mattral/guardrail-rs/blob/main/docs/policy-rules.md) —
//!   writing `[[policy.rules]]` entries (the `when`/`then` shape).
//! - [Threat model](https://github.com/Mattral/guardrail-rs/blob/main/docs/threat-model.md) —
//!   security properties of `[auth]`, fail-open behavior, and audit logging.
//! - [Changelog](https://github.com/Mattral/guardrail-rs/blob/main/CHANGELOG.md) —
//!   release history and notable changes.

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod loader;
pub mod schema;
pub mod validate;

pub use loader::{build_response_redactor, ConfigHandle, ConfigLoadError};
pub use schema::{
    AuditLogConfig, AuthConfig, Config, ObservabilityConfig, OnErrorBehavior, PiiEntityList,
    PipelineConfig, PolicyRuleConfig, ServerConfig, StagesConfig, UpstreamConfig,
};
pub use validate::validate_config;
