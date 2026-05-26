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

pub mod audit;
pub mod audit_log;
pub mod forward;
pub mod metrics;
pub mod response;
pub mod server;
pub mod telemetry;
pub mod translate;

pub use server::{run_server, ServerHandle};
