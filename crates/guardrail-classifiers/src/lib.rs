//! Classifier implementations for `guardrail-rs`.
//!
//! This crate provides the concrete [`guardrail_core::Stage`] implementations that perform
//! threat detection and PII redaction. All stages implement the
//! [`guardrail_core::Stage`] trait and can be composed into a [`guardrail_core::Pipeline`].
//!
//! ## Available stages
//!
//! (`OnnxInjectionClassifier` and `ToxicityClassifier` below are plain code
//! text, not links: they only exist under the `onnx` feature, and CI builds
//! docs with default features, so a link to either would be a broken-link
//! build error.)
//!
//! | Stage | Feature flag | Description |
//! |-------|-------------|-------------|
//! | [`RegexInjectionScanner`] | *(always)* | Fast regex-based prompt injection detection |
//! | [`PiiRedactor`] | *(always)* | Regex-based PII detection and redaction |
//! | `OnnxInjectionClassifier` | `onnx` | DeBERTa-based semantic injection detection |
//! | `ToxicityClassifier` | `onnx` | RoBERTa-based toxicity detection |
//!
//! ## Quick example
//!
//! ```rust
//! use guardrail_core::Pipeline;
//! use guardrail_classifiers::{RegexInjectionScanner, PiiRedactor};
//!
//! # tokio_test::block_on(async {
//! let pipeline = Pipeline::builder()
//!     .stage(RegexInjectionScanner::default())
//!     .stage(PiiRedactor::default())
//!     .build();
//!
//! let req = guardrail_core::test_helpers::injection_request();
//! let (decision, _req) = pipeline.run(req).await.unwrap();
//! assert!(matches!(decision, guardrail_core::Decision::Block { .. }));
//! # });
//! ```
//!
//! ## Further reading
//!
//! - [Configuration reference](https://github.com/Mattral/guardrail-rs/blob/main/docs/configuration.md) —
//!   how `[stages.regex_injection]`, `[stages.pii_redactor]`, etc. map to these types.
//! - [Threat model](https://github.com/Mattral/guardrail-rs/blob/main/docs/threat-model.md) —
//!   detection coverage and residual risk for each classifier.
//! - [Stage API reference](https://github.com/Mattral/guardrail-rs/blob/main/docs/stage-api.md) —
//!   implementing your own stage alongside these.
//! - [Changelog](https://github.com/Mattral/guardrail-rs/blob/main/CHANGELOG.md) —
//!   release history and notable changes.

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod classifier;
pub mod injection;
pub mod pii;

#[cfg(feature = "onnx")]
pub mod onnx;

pub use classifier::{Classifier, ClassifierScore, RegexBackend, RegexMatchResult};
pub use injection::RegexInjectionScanner;
pub use pii::{PiiEntityType, PiiRedactor, RedactionRecord};

#[cfg(feature = "onnx")]
pub use onnx::{OnnxInjectionClassifier, ToxicityClassifier};
