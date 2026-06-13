//! Unified error type for all guardrail-rs crates.

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

    /// The upstream LLM request failed.
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
}
