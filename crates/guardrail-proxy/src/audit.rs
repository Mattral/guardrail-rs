//! Structured audit logging for blocked and redacted requests.
//!
//! Audit records are emitted as structured `tracing` events at the `info`
//! level under the `guardrail::audit` target, so they can be filtered,
//! shipped to a SIEM, or written to a dedicated audit log file independently
//! of general application logs.
//!
//! **Privacy note:** audit records never include the raw request content.
//! Only metadata (request ID, model, decision, reason, entity types, byte
//! offsets) is logged. This avoids the audit trail itself becoming a PII
//! exposure vector.

use guardrail_core::{
    decision::{BlockCode, Decision},
    request::GuardrailRequest,
};
use serde::Serialize;

/// A structured audit record for a single pipeline decision.
///
/// Serializable to JSON for log shipping (e.g. to a SIEM via the `json_logs`
/// observability option).
#[derive(Debug, Clone, Serialize)]
pub struct AuditRecord<'a> {
    /// The unique request ID.
    pub request_id: String,
    /// The model requested.
    pub model: &'a str,
    /// The provider the request was destined for.
    pub provider: String,
    /// The final pipeline decision: `"allow"`, `"redact"`, or `"block"`.
    pub decision: &'static str,
    /// The block code, if `decision == "block"`.
    pub block_code: Option<String>,
    /// The human-readable reason, if `decision != "allow"`.
    pub reason: Option<String>,
    /// Number of messages in the request.
    pub message_count: usize,
}

impl<'a> AuditRecord<'a> {
    /// Build an audit record from a request and its final decision.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_proxy::audit::AuditRecord;
    /// use guardrail_core::{decision::{Decision, BlockCode}, test_helpers::clean_request};
    ///
    /// let req = clean_request();
    /// let record = AuditRecord::from_decision(&req, &Decision::Allow);
    /// assert_eq!(record.decision, "allow");
    /// assert!(record.block_code.is_none());
    /// ```
    pub fn from_decision(req: &'a GuardrailRequest, decision: &Decision) -> Self {
        let (decision_name, block_code, reason) = match decision {
            Decision::Allow => ("allow", None, None),
            Decision::Redact { reason, .. } => ("redact", None, Some(reason.clone())),
            Decision::Block { reason, code } => {
                ("block", Some(block_code_str(code)), Some(reason.clone()))
            }
        };

        Self {
            request_id: req.id.to_string(),
            model: &req.model,
            provider: format!("{:?}", req.provider).to_lowercase(),
            decision: decision_name,
            block_code,
            reason,
            message_count: req.messages.len(),
        }
    }

    /// Emit this record as a structured `tracing` event at the `info` level.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_proxy::audit::AuditRecord;
    /// use guardrail_core::{decision::Decision, test_helpers::clean_request};
    ///
    /// let req = clean_request();
    /// let record = AuditRecord::from_decision(&req, &Decision::Allow);
    /// record.emit();
    /// ```
    pub fn emit(&self) {
        tracing::info!(
            target: "guardrail::audit",
            request_id = %self.request_id,
            model = %self.model,
            provider = %self.provider,
            decision = %self.decision,
            block_code = ?self.block_code,
            reason = ?self.reason,
            message_count = self.message_count,
            "pipeline decision"
        );
    }
}

fn block_code_str(code: &BlockCode) -> String {
    code.as_str().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use guardrail_core::test_helpers::clean_request;

    #[test]
    fn test_audit_record_allow() {
        let req = clean_request();
        let record = AuditRecord::from_decision(&req, &Decision::Allow);
        assert_eq!(record.decision, "allow");
        assert!(record.block_code.is_none());
        assert!(record.reason.is_none());
    }

    #[test]
    fn test_audit_record_block() {
        let req = clean_request();
        let decision = Decision::Block {
            reason: "Prompt injection detected.".into(),
            code: BlockCode::PromptInjection,
        };
        let record = AuditRecord::from_decision(&req, &decision);
        assert_eq!(record.decision, "block");
        assert_eq!(record.block_code, Some("prompt_injection".to_string()));
        assert_eq!(record.reason, Some("Prompt injection detected.".to_string()));
    }

    #[test]
    fn test_audit_record_redact() {
        let req = clean_request();
        let decision = Decision::Redact {
            reason: "PII redacted: Email".into(),
            mutated: req.clone(),
        };
        let record = AuditRecord::from_decision(&req, &decision);
        assert_eq!(record.decision, "redact");
        assert_eq!(record.reason, Some("PII redacted: Email".to_string()));
    }

    #[test]
    fn test_audit_record_serializes_to_json() {
        let req = clean_request();
        let record = AuditRecord::from_decision(&req, &Decision::Allow);
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"decision\":\"allow\""));
        assert!(json.contains("\"model\":\"gpt-4o\""));
    }

    #[test]
    fn test_audit_record_emit_does_not_panic() {
        let req = clean_request();
        let record = AuditRecord::from_decision(&req, &Decision::Allow);
        record.emit();
    }
}
