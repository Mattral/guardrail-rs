//! Unified error type for all guardrail-rs crates.
//!
//! ## A note on `Upstream` and the dependency budget
//!
//! Per the project's dependency budget (spec §20), `guardrail-core` does not
//! depend on `reqwest` by default — it is the foundational crate and should
//! remain usable in constrained / embedded contexts without pulling in an
//! HTTP client. However, the spec's error model calls for
//! `Upstream(#[from] reqwest::Error)` so that `?` works ergonomically at the
//! call site in `guardrail-proxy`.
//!
//! We resolve this by making `reqwest` an **optional** dependency gated
//! behind the `reqwest-errors` feature. `guardrail-proxy` enables this
//! feature; consumers of `guardrail-core` alone (e.g. a future WASM build,
//! or someone embedding just the classifiers) get the lighter
//! `Upstream(String)` variant with no behavioral difference apart from the
//! `#[from]` conversion.

/// Top-level error type used throughout `guardrail-rs`.
///
/// All errors are variants of this enum so that callers can handle them
/// uniformly regardless of which crate or stage produced them.
#[derive(Debug, thiserror::Error)]
pub enum GuardrailError {
    /// A pipeline stage failed during evaluation.
    #[error("Pipeline stage '{stage}' failed: {source}")]
    StageFailed {
        /// The name of the stage that failed.
        stage: String,
        /// The underlying error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// A configuration value was invalid or missing.
    #[error("Configuration error: {0}")]
    Config(String),

    /// The upstream LLM request failed (`reqwest-errors` feature enabled).
    ///
    /// Carries the original [`reqwest::Error`] for inspection (status code,
    /// timeout vs. connect-error classification, etc.) via `?`.
    #[cfg(feature = "reqwest-errors")]
    #[error("Upstream request failed: {0}")]
    Upstream(#[from] reqwest::Error),

    /// The upstream LLM request failed (`reqwest-errors` feature disabled).
    ///
    /// Carries a pre-formatted message instead of the original error type,
    /// avoiding a hard dependency on `reqwest` in minimal builds.
    #[cfg(not(feature = "reqwest-errors"))]
    #[error("Upstream request failed: {0}")]
    Upstream(String),

    /// JSON serialization or deserialization failed.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A regex pattern failed to compile.
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// An internal invariant was violated (should never happen in production).
    #[error("Internal error: {0}")]
    Internal(String),
}

impl GuardrailError {
    /// Wrap any error as a `StageFailed` variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_core::GuardrailError;
    ///
    /// let err = GuardrailError::stage_failed(
    ///     "my_stage",
    ///     std::io::Error::new(std::io::ErrorKind::Other, "disk full"),
    /// );
    /// assert!(err.to_string().contains("my_stage"));
    /// ```
    pub fn stage_failed(
        stage: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        GuardrailError::StageFailed {
            stage: stage.into(),
            source: Box::new(source),
        }
    }

    /// Construct an `Upstream` error from a message string.
    ///
    /// Available regardless of the `reqwest-errors` feature, so call sites
    /// that just want to report a string don't need to feature-gate.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_core::GuardrailError;
    ///
    /// let err = GuardrailError::upstream("connection refused");
    /// assert!(err.to_string().contains("connection refused"));
    /// ```
    pub fn upstream(message: impl Into<String>) -> Self {
        #[cfg(feature = "reqwest-errors")]
        {
            // We can't construct a real reqwest::Error from a string, so in
            // this feature configuration we fall back to Internal for
            // string-only construction. Call sites with an actual
            // reqwest::Error should use `?` / `.into()` instead.
            GuardrailError::Internal(format!("upstream: {}", message.into()))
        }
        #[cfg(not(feature = "reqwest-errors"))]
        {
            GuardrailError::Upstream(message.into())
        }
    }
}
