//! Classifier implementations for `guardrail-rs`.
//!
//! This crate provides the concrete [`guardrail_core::Stage`] implementations that perform
//! threat detection and PII redaction. All stages implement the
//! [`guardrail_core::Stage`] trait and can be composed into a [`guardrail_core::Pipeline`].
//!
//! ## Available stages
//!
//! | Stage | Feature flag | Description |
//! |-------|-------------|-------------|
//! | [`RegexInjectionScanner`] | *(always)* | Fast regex-based prompt injection detection |
//! | [`PiiRedactor`] | *(always)* | Regex-based PII detection and redaction |
//! | [`OnnxInjectionClassifier`] | `onnx` | DeBERTa-based semantic injection detection |
//! | [`ToxicityClassifier`] | `onnx` | RoBERTa-based toxicity detection |

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
