//! # guardrail-proxy
//!
//! HTTP reverse-proxy server for `guardrail-rs`.
//!
//! This crate ties together [`guardrail_core`], [`guardrail_classifiers`], and
//! [`guardrail_config`] into a runnable server: it accepts HTTP requests
//! shaped like OpenAI/Anthropic chat-completion calls, runs them through the
//! configured [`guardrail_core::Pipeline`], and either forwards the (possibly
//! redacted) request upstream or returns a block response.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────┐     ┌────────────────┐     ┌───────────┐     ┌──────────────┐
//! │  Client  │ ──▶ │  guardrail-rs  │ ──▶ │  Pipeline  │ ──▶ │   Upstream    │
//! │          │     │  HTTP server   │     │  (Stages)  │     │  (OpenAI/...) │
//! └──────────┘     └────────────────┘     └───────────┘     └──────────────┘
//!                          │                                          │
//!                          ▼                                          │
//!                   ┌─────────────┐                                  │
//!                   │  Block JSON │ ◀─── short-circuit on Block ─────┘
//!                   │  response   │
//!                   └─────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`server`]: the main HTTP server loop and request handler
//! - [`forward`]: upstream request forwarding (streaming + non-streaming)
//! - [`translate`]: conversion between raw JSON bodies and [`guardrail_core::GuardrailRequest`]
//! - [`response`]: output-side PII redaction for non-streaming responses
//! - [`telemetry`]: OpenTelemetry OTLP layer and per-request/stage span helpers
//! - [`metrics`]: Prometheus metrics registry and recording helpers
//! - [`audit`]: structured audit logging (tracing events + rotating NDJSON file)
//! - [`audit_log`]: rotating NDJSON file layer for `tracing_subscriber`
//!
//! ## Feature flags
//!
//! | Flag | Description | Default |
//! |------|-------------|---------|
//! | *(none)* | Base proxy: regex injection scanning, PII redaction, policy engine, Prometheus metrics, OTel tracing, NDJSON audit log | — |
//! | `onnx` | Enables the semantic injection/toxicity classifiers via `guardrail-classifiers/onnx` and `guardrail-config/onnx` | off |
//!
//! ## Quick example
//!
//! Most applications run guardrail-proxy via the `guardrail` CLI binary
//! rather than embedding [`run_server`] directly, but it is fully usable as
//! a library:
//!
//! ```rust,no_run
//! use guardrail_config::ConfigHandle;
//! use std::sync::Arc;
//!
//! # tokio_test::block_on(async {
//! let config = Arc::new(ConfigHandle::load("guardrail.toml").unwrap());
//! let handle = guardrail_proxy::run_server(config).await.unwrap();
//! println!("listening on {}", handle.local_addr());
//! handle.shutdown();
//! # });
//! ```
//!
//! ## Further reading
//!
//! - [Configuration reference](https://github.com/Mattral/guardrail-rs/blob/main/docs/configuration.md) —
//!   every TOML key this server consumes, including `[auth]` and `[observability]`.
//! - [Threat model](https://github.com/Mattral/guardrail-rs/blob/main/docs/threat-model.md) —
//!   what this proxy protects against, caller authentication, and residual risks.
//! - [Architecture](https://github.com/Mattral/guardrail-rs/blob/main/docs/architecture.md) —
//!   request/response pipeline diagram and hot-reload behavior.
//! - [Changelog](https://github.com/Mattral/guardrail-rs/blob/main/CHANGELOG.md) —
//!   release history and notable changes.

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod audit;
pub mod audit_log;
pub mod forward;
pub mod metrics;
pub mod response;
pub mod server;
pub mod telemetry;
pub mod translate;

pub use server::{run_server, ServerHandle};
