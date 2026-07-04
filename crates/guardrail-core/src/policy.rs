//! User-defined policy engine: the last line of defense.

use crate::{
    decision::{BlockCode, Decision},
    error::GuardrailError,
    pipeline::Stage,
    request::GuardrailRequest,
};

/// A single policy rule.
///
/// Rules are evaluated in order. The first matching rule's action is returned.
#[derive(Debug, Clone)]
pub struct PolicyRule {
    /// Human-readable name for the rule (used in logs and audit records).
    pub name: String,
    /// Whether this rule is active.
    pub enabled: bool,
    /// The condition that triggers this rule.
    pub condition: PolicyCondition,
    /// The action to take when the condition is met.
    pub action: PolicyAction,
    /// Optional message to include in the block response.
    pub message: Option<String>,
}

/// Conditions that can trigger a policy rule.
#[derive(Debug, Clone)]
pub enum PolicyCondition {
    /// Matches if any of the given strings appear in any message content.
    ContentContains(Vec<String>),
    /// Matches if there is no system prompt in the request.
    SystemPromptAbsent,
    /// Matches if the approximate token count (word count * 1.3) exceeds the threshold.
    TokenCountExceeds(usize),
    /// Always matches (useful for a default-deny rule at the end of a list).
    Always,
}

/// Actions taken when a policy rule matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyAction {
    /// Allow the request through.
    Allow,
    /// Redact the request (not yet fully supported; treated as Allow).
    Redact,
    /// Block the request entirely.
    Block,
    /// Log the match but allow the request through.
    LogOnly,
}

/// Evaluates a list of `PolicyRule` objects in order.
///
/// Returns the first matching rule's action. Returns `Allow` if no rule matches.
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
}

impl PolicyEngine {
    /// Create a new engine from a list of rules.
    pub fn new(rules: Vec<PolicyRule>) -> Self {
        Self { rules }
    }

    /// Create an engine with no rules (always allows).
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }
}

#[async_trait::async_trait]
impl Stage for PolicyEngine {
    fn name(&self) -> &'static str {
        "policy_engine"
    }

    /// Evaluate the request against all rules and return the first matching action.
    ///
    /// # Errors
    ///
    /// This stage does not return errors; it always returns `Ok`.
    async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError> {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            let matched = match &rule.condition {
                PolicyCondition::ContentContains(keywords) => {
                    let all_text = req.all_text().to_lowercase();
                    keywords.iter().any(|kw| all_text.contains(kw.as_str()))
                }
                PolicyCondition::SystemPromptAbsent => !req
                    .messages
                    .iter()
                    .any(|m| matches!(m.role, crate::request::Role::System)),
                PolicyCondition::TokenCountExceeds(limit) => {
                    let approx_tokens =
                        (req.all_text().split_whitespace().count() as f64 * 1.3) as usize;
                    approx_tokens > *limit
                }
                PolicyCondition::Always => true,
            };

            if matched {
                let reason = rule
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("Policy rule '{}' matched.", rule.name));

                tracing::info!(rule = %rule.name, action = ?rule.action, "policy rule matched");

                return Ok(match rule.action {
                    PolicyAction::Allow | PolicyAction::Redact | PolicyAction::LogOnly => {
                        Decision::Allow
                    }
                    PolicyAction::Block => Decision::Block {
                        reason,
                        code: BlockCode::PolicyViolation,
                    },
                });
            }
        }

        Ok(Decision::Allow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request::{ChatMessage, MessageContent, Provider, Role},
        test_helpers::clean_request,
    };

    fn make_req_with_content(content: &str) -> GuardrailRequest {
        GuardrailRequest::new(
            vec![ChatMessage {
                role: Role::User,
                content: MessageContent::Text(content.into()),
            }],
            "gpt-4o".into(),
            Provider::OpenAI,
        )
    }

    #[tokio::test]
    async fn test_no_rules_allows() {
        let engine = PolicyEngine::empty();
        let req = clean_request();
        let decision = engine.evaluate(&req).await.unwrap();
        assert_eq!(decision, Decision::Allow);
    }

    #[tokio::test]
    async fn test_content_contains_blocks() {
        let engine = PolicyEngine::new(vec![PolicyRule {
            name: "no-competitors".into(),
            enabled: true,
            condition: PolicyCondition::ContentContains(vec!["competitor-x".into()]),
            action: PolicyAction::Block,
            message: Some("Competitor mentions are not allowed.".into()),
        }]);

        let req = make_req_with_content("Tell me about Competitor-X products.");
        let decision = engine.evaluate(&req).await.unwrap();
        assert!(matches!(decision, Decision::Block { .. }));
    }

    #[tokio::test]
    async fn test_disabled_rule_ignored() {
        let engine = PolicyEngine::new(vec![PolicyRule {
            name: "disabled-rule".into(),
            enabled: false,
            condition: PolicyCondition::Always,
            action: PolicyAction::Block,
            message: None,
        }]);

        let req = clean_request();
        let decision = engine.evaluate(&req).await.unwrap();
        assert_eq!(decision, Decision::Allow);
    }

    #[tokio::test]
    async fn test_system_prompt_absent_blocks() {
        let engine = PolicyEngine::new(vec![PolicyRule {
            name: "require-system-prompt".into(),
            enabled: true,
            condition: PolicyCondition::SystemPromptAbsent,
            action: PolicyAction::Block,
            message: Some("A system prompt is required.".into()),
        }]);

        let req = make_req_with_content("Hello!");
        let decision = engine.evaluate(&req).await.unwrap();
        assert!(matches!(decision, Decision::Block { .. }));
    }

    #[tokio::test]
    async fn test_token_count_blocks() {
        let engine = PolicyEngine::new(vec![PolicyRule {
            name: "max-tokens".into(),
            enabled: true,
            condition: PolicyCondition::TokenCountExceeds(5),
            action: PolicyAction::Block,
            message: Some("Input too long.".into()),
        }]);

        let long_text = "one two three four five six seven eight".repeat(3);
        let req = make_req_with_content(&long_text);
        let decision = engine.evaluate(&req).await.unwrap();
        assert!(matches!(decision, Decision::Block { .. }));
    }
}
