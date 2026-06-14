//! # guardrail-config
//!
//! Configuration types, TOML loading, and validation for `guardrail-rs`.
//!
//! The proxy reads a single TOML file (default: `guardrail.toml`) at startup.
//! Configuration can be reloaded at runtime via [`ConfigHandle::reload`] without
//! dropping in-flight connections — stages constructed from the new config are
//! swapped in atomically using [`arc_swap::ArcSwap`].
//!
//! ## Example configuration
//!
//! ```toml
//! [server]
//! listen_addr = "0.0.0.0:8080"
//! upstream_url = "https://api.openai.com"
//!
//! [pipeline]
//! on_error = "allow"
//!
//! [stages.regex_injection]
//! enabled = true
//!
//! [stages.pii_redaction]
//! enabled = true
//! entities = ["email", "phone", "credit_card", "ssn"]
//!
//! [[policy.rules]]
//! name = "block-competitor-mentions"
//! enabled = true
//! action = "block"
//! condition.type = "content_contains"
//! condition.keywords = ["competitor-x"]
//! message = "Mentions of Competitor X are not permitted."
//! ```

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod loader;
pub mod schema;
pub mod validate;

pub use loader::{ConfigHandle, ConfigLoadError};
pub use schema::{
    AuditLogConfig, Config, ObservabilityConfig, PiiEntityList, PipelineConfig, PolicyRuleConfig,
    ServerConfig, StagesConfig,
};
pub use validate::validate_config;
