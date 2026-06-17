//! # guardrail-config
//!
//! Configuration types, TOML loading, environment-variable overlay, and
//! validation for `guardrail-rs`.
//!
//! See [`guardrail.example.toml`][example] for the full annotated schema.
//!
//! [example]: https://github.com/Mattral/guardrail-rs/blob/main/guardrail.example.toml

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod loader;
pub mod schema;
pub mod validate;

pub use loader::{build_response_redactor, ConfigHandle, ConfigLoadError};
pub use schema::{
    AuditLogConfig, AuthConfig, Config, ObservabilityConfig, PiiEntityList,
    PipelineConfig, PolicyRuleConfig, ServerConfig, StagesConfig, UpstreamConfig,
    OnErrorBehavior,
};
pub use validate::validate_config;
