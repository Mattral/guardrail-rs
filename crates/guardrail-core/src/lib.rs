//! # guardrail-core
//!
//! Core types, pipeline, and stage trait for `guardrail-rs` — a zero-Python,
//! production-grade LLM security layer written in Rust.
//!
//! This crate contains no I/O and no async runtime. It is purely the data model
//! and the pipeline abstraction that all other crates build upon.
//!
//! ## Feature flags
//!
//! | Flag | Description | Default |
//! |------|-------------|---------|
//! | *(none)* | Base functionality, always enabled | — |
//!
//! ## Quick example
//!
//! ```rust
//! use guardrail_core::{Pipeline, PipelineBuilder, Decision};
//!
//! // Build a pipeline (stages added by guardrail-classifiers)
//! let pipeline = PipelineBuilder::default().build();
//!
//! // A pipeline with no stages always allows
//! # tokio_test::block_on(async {
//! let req = guardrail_core::test_helpers::clean_request();
//! let (decision, _req) = pipeline.run(req).await.unwrap();
//! assert_eq!(decision, Decision::Allow);
//! # });
//! ```

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod decision;
pub mod error;
pub mod pipeline;
pub mod policy;
pub mod request;

#[allow(missing_docs)]
pub mod test_helpers;

pub use decision::{BlockCode, Decision};
pub use error::GuardrailError;
pub use pipeline::{Pipeline, PipelineBuilder, Stage};
pub use request::{ChatMessage, ContentPart, GuardrailRequest, MessageContent, Provider, Role};
