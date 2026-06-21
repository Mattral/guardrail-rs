//! Decision types returned by pipeline stages.

use crate::request::GuardrailRequest;

/// The outcome of a single stage evaluation.
///
/// Stages return a `Decision` to indicate whether the request should proceed,
/// be modified, or be stopped entirely.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Request is clean; proceed to the next stage.
    Allow,
    /// Request contained sensitive content; `mutated` holds the sanitized version.
    Redact {
        /// Human-readable reason for the redaction.
        reason: String,
        /// The sanitized request to pass to subsequent stages.
        mutated: GuardrailRequest,
        /// Machine-readable list of entity-type names that were redacted
        /// (e.g. `["email", "phone"]`), for structured logging and the
        /// audit trail. Mirrors how [`Decision::Block`] pairs a
        /// human-readable `reason` with a machine-readable `code`. Empty
        /// for stages that redact without a typed entity taxonomy (e.g. a
        /// custom stage that redacts based on a free-form policy match).
        entities: Vec<String>,
    },
    /// Request must be blocked; do not forward to the upstream.
    Block {
        /// Human-readable reason for blocking.
        reason: String,
        /// Machine-readable block code for programmatic handling.
        code: BlockCode,
    },
}

impl Decision {
    /// Returns `true` if the request should be forwarded upstream (`Allow` or `Redact`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_core::decision::{Decision, BlockCode};
    ///
    /// assert!(Decision::Allow.should_forward());
    /// assert!(!Decision::Block {
    ///     reason: "injection".into(),
    ///     code: BlockCode::PromptInjection
    /// }.should_forward());
    /// ```
    pub fn should_forward(&self) -> bool {
        !matches!(self, Decision::Block { .. })
    }

    /// Returns the name of this decision variant as a static string, for logging.
    pub fn name(&self) -> &'static str {
        match self {
            Decision::Allow => "allow",
            Decision::Redact { .. } => "redact",
            Decision::Block { .. } => "block",
        }
    }
}

/// Machine-readable codes for blocked requests.
///
/// Used in the JSON error response body and audit log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockCode {
    /// A prompt injection pattern was detected.
    PromptInjection,
    /// Toxic or harmful content was detected.
    Toxicity,
    /// A user-defined policy rule was violated.
    PolicyViolation,
    /// The request was rate-limited.
    RateLimit,
    /// A custom block code defined by a user stage.
    Custom(String),
}

impl BlockCode {
    /// Returns the snake_case string representation for use in JSON/logs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_core::decision::BlockCode;
    ///
    /// assert_eq!(BlockCode::PromptInjection.as_str(), "prompt_injection");
    /// assert_eq!(BlockCode::Custom("my_rule".into()).as_str(), "my_rule");
    /// ```
    pub fn as_str(&self) -> &str {
        match self {
            BlockCode::PromptInjection => "prompt_injection",
            BlockCode::Toxicity => "toxicity",
            BlockCode::PolicyViolation => "policy_violation",
            BlockCode::RateLimit => "rate_limit",
            BlockCode::Custom(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for BlockCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_forward_allow() {
        assert!(Decision::Allow.should_forward());
    }

    #[test]
    fn test_should_forward_block() {
        let d = Decision::Block {
            reason: "test".into(),
            code: BlockCode::PromptInjection,
        };
        assert!(!d.should_forward());
    }

    #[test]
    fn test_block_code_as_str() {
        assert_eq!(BlockCode::PromptInjection.as_str(), "prompt_injection");
        assert_eq!(BlockCode::Toxicity.as_str(), "toxicity");
        assert_eq!(BlockCode::PolicyViolation.as_str(), "policy_violation");
        assert_eq!(BlockCode::RateLimit.as_str(), "rate_limit");
        assert_eq!(BlockCode::Custom("foo_bar".into()).as_str(), "foo_bar");
    }

    #[test]
    fn test_decision_name() {
        assert_eq!(Decision::Allow.name(), "allow");
        assert_eq!(
            Decision::Block {
                reason: "x".into(),
                code: BlockCode::Toxicity
            }
            .name(),
            "block"
        );
    }
}
