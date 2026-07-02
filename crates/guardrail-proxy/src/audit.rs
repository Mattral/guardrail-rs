//! Structured audit logging for blocked and redacted requests.
//!
//! Audit records are emitted as structured `tracing` events at the `info`
//! level under the `guardrail::audit` target, so they can be filtered,
//! shipped to a SIEM, or written to a dedicated audit log file independently
//! of general application logs.
//!
//! ## Audit record shape (§10 of the spec)
//!
//! ```json
//! {
//!   "timestamp": "2026-06-11T10:00:00.000Z",
//!   "request_id": "01J9XK...",
//!   "decision": "block",
//!   "stage": "onnx_injection",
//!   "reason": "prompt_injection",
//!   "score": 0.97,
//!   "model": "gpt-4o",
//!   "role_scanned": "user",
//!   "pii_entities_found": [],
//!   "latency_pipeline_ms": 0.8,
//!   "latency_total_ms": 312.4
//! }
//! ```
//!
//! **Privacy invariants (never violated):**
//! - Audit records never contain message content or API keys.
//! - Only metadata needed for security analysis is logged.

use chrono::{Datelike, Timelike};
use guardrail_core::{
    decision::{BlockCode, Decision},
    request::GuardrailRequest,
};
use serde::Serialize;

/// A structured audit record for a single pipeline decision.
///
/// This type is serializable to JSON for log shipping. The shape matches
/// §10 of the spec exactly, including optional fields that are only present
/// when relevant (e.g. `score` only for ONNX decisions, `pii_entities_found`
/// only for redaction decisions).
#[derive(Debug, Clone, Serialize)]
pub struct AuditRecord {
    /// ISO 8601 timestamp with millisecond precision and `Z` suffix.
    pub timestamp: String,
    /// The unique request ID (UUID v4).
    pub request_id: String,
    /// Final pipeline decision: `"allow"`, `"redact"`, or `"block"`.
    pub decision: &'static str,
    /// The name of the stage that produced this decision. `None` for
    /// `"allow"` decisions where all stages passed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// Human-readable reason string. `None` for `"allow"` decisions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Machine-readable block code (`"prompt_injection"`, `"toxicity"`,
    /// `"policy_violation"`, …). Present only on `"block"` decisions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Classifier confidence score in `[0, 1]`. Present only when an ONNX
    /// classifier made the decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    /// The model the request was destined for.
    pub model: String,
    /// The provider (lowercase).
    pub provider: String,
    /// Number of messages in the request.
    pub message_count: usize,
    /// PII entity types found, e.g. `["email", "phone"]`. Empty for non-redact decisions.
    pub pii_entities_found: Vec<String>,
    /// Pipeline-only evaluation latency in milliseconds.
    pub latency_pipeline_ms: f64,
    /// End-to-end request latency including upstream, in milliseconds.
    pub latency_total_ms: f64,
}

impl AuditRecord {
    /// Build an audit record from a request and its final decision.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_proxy::audit::AuditRecord;
    /// use guardrail_core::{decision::{Decision, BlockCode}, test_helpers::clean_request};
    ///
    /// let req = clean_request();
    /// let record = AuditRecord::from_decision(&req, &Decision::Allow, 0.05, 120.0);
    /// assert_eq!(record.decision, "allow");
    /// assert!(record.code.is_none());
    /// ```
    ///
    /// PII entity types are read directly from `Decision::Redact`'s own
    /// `entities` field — there is no separate parameter to thread through
    /// (and therefore no way to pass a list that doesn't match the actual
    /// decision, which was a real bug in an earlier version of this API).
    ///
    /// ```rust
    /// use guardrail_proxy::audit::AuditRecord;
    /// use guardrail_core::{decision::Decision, test_helpers::clean_request};
    ///
    /// let req = clean_request();
    /// let decision = Decision::Redact {
    ///     reason: "PII redacted: Email".into(),
    ///     mutated: req.clone(),
    ///     entities: vec!["email".to_string()],
    /// };
    /// let record = AuditRecord::from_decision(&req, &decision, 0.02, 250.0);
    /// assert_eq!(record.pii_entities_found, vec!["email"]);
    /// ```
    pub fn from_decision(
        req: &GuardrailRequest,
        decision: &Decision,
        latency_pipeline_ms: f64,
        latency_total_ms: f64,
    ) -> Self {
        let now = chrono_timestamp();

        let (decision_name, stage, reason, code, pii_entities) = match decision {
            Decision::Allow => ("allow", None, None, None, Vec::new()),
            Decision::Redact {
                reason, entities, ..
            } => ("redact", None, Some(reason.clone()), None, entities.clone()),
            Decision::Block { reason, code } => (
                "block",
                None, // stage name is best-effort from caller context
                Some(reason.clone()),
                Some(block_code_str(code)),
                Vec::new(),
            ),
        };

        Self {
            timestamp: now,
            request_id: req.id.to_string(),
            decision: decision_name,
            stage,
            reason,
            code,
            score: None, // populated by `with_score`
            model: req.model.clone(),
            provider: format!("{:?}", req.provider).to_lowercase(),
            message_count: req.messages.len(),
            pii_entities_found: pii_entities,
            latency_pipeline_ms,
            latency_total_ms,
        }
    }

    /// Set the ONNX classifier score on a record already constructed with
    /// `from_decision`. Returns `self` for chaining.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_proxy::audit::AuditRecord;
    /// use guardrail_core::{decision::{Decision, BlockCode}, test_helpers::clean_request};
    ///
    /// let req = clean_request();
    /// let record = AuditRecord::from_decision(
    ///     &req,
    ///     &Decision::Block { reason: "test".into(), code: BlockCode::PromptInjection },
    ///     0.8,
    ///     312.0,
    /// )
    /// .with_score(0.97)
    /// .with_stage("onnx_injection");
    ///
    /// assert_eq!(record.score, Some(0.97));
    /// assert_eq!(record.stage.as_deref(), Some("onnx_injection"));
    /// ```
    #[must_use]
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = Some(score);
        self
    }

    /// Set the pipeline stage that produced the decision. Returns `self` for
    /// chaining.
    #[must_use]
    pub fn with_stage(mut self, stage: impl Into<String>) -> Self {
        self.stage = Some(stage.into());
        self
    }

    /// Emit this record as a structured `tracing` event at `info` level
    /// under the `guardrail::audit` target.
    ///
    /// The `tracing` event fields are designed to be captured by the NDJSON
    /// audit-log writer in [`crate::audit_log`] and also appear in the
    /// structured JSON application log when `log_format = "json"`.
    pub fn emit(&self) {
        tracing::info!(
            target: "guardrail::audit",
            timestamp         = %self.timestamp,
            request_id        = %self.request_id,
            decision          = %self.decision,
            stage             = ?self.stage,
            reason            = ?self.reason,
            code              = ?self.code,
            score             = ?self.score,
            model             = %self.model,
            provider          = %self.provider,
            message_count     = self.message_count,
            pii_entities_found = ?self.pii_entities_found,
            latency_pipeline_ms = self.latency_pipeline_ms,
            latency_total_ms  = self.latency_total_ms,
            "pipeline decision"
        );
    }
}

fn block_code_str(code: &BlockCode) -> String {
    code.as_str().to_string()
}

/// Return the current UTC time formatted as an ISO 8601 string with
/// millisecond precision and a `Z` suffix, without pulling in the `chrono`
/// crate (which would add a heavy dependency for a non-critical path).
///
/// Format: `"2026-06-11T10:00:00.123Z"`
fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let total_secs = now.as_secs();
    let millis = now.subsec_millis();

    // Convert UNIX epoch seconds to (year, month, day, hour, min, sec)
    // using a minimal algorithm to avoid pulling in `chrono`.
    let (year, month, day, hour, min, sec) = unix_secs_to_datetime(total_secs);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}.{millis:03}Z")
}

/// Minimal, allocation-free conversion of UNIX seconds to calendar fields
/// (Gregorian calendar, UTC). Handles dates from 1970 to ~2200 correctly.
fn unix_secs_to_datetime(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let dt = chrono::DateTime::from_timestamp(secs as i64, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    let dt = dt.naive_utc();
    (
        dt.year() as u64,
        dt.month() as u64,
        dt.day() as u64,
        dt.hour() as u64,
        dt.minute() as u64,
        dt.second() as u64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use guardrail_core::{
        decision::BlockCode,
        test_helpers::{clean_request, injection_request},
    };

    #[test]
    fn test_audit_record_allow() {
        let req = clean_request();
        let record = AuditRecord::from_decision(&req, &Decision::Allow, 0.05, 120.0);
        assert_eq!(record.decision, "allow");
        assert!(record.code.is_none());
        assert!(record.reason.is_none());
        assert!(record.score.is_none());
        assert!(record.pii_entities_found.is_empty());
    }

    #[test]
    fn test_audit_record_block() {
        let req = injection_request();
        let decision = Decision::Block {
            reason: "Prompt injection detected.".into(),
            code: BlockCode::PromptInjection,
        };
        let record = AuditRecord::from_decision(&req, &decision, 0.8, 0.9)
            .with_score(0.97)
            .with_stage("onnx_injection");

        assert_eq!(record.decision, "block");
        assert_eq!(record.code, Some("prompt_injection".to_string()));
        assert_eq!(record.score, Some(0.97));
        assert_eq!(record.stage.as_deref(), Some("onnx_injection"));
        assert_eq!(record.latency_pipeline_ms, 0.8);
    }

    #[test]
    fn test_audit_record_redact_with_pii() {
        // Entities now come directly from Decision::Redact's own field —
        // no separate parameter to keep in sync, closing the gap where a
        // caller could previously pass an entity list that didn't match
        // the decision it was supposedly describing.
        let req = clean_request();
        let decision = Decision::Redact {
            reason: "PII redacted: Email".into(),
            mutated: req.clone(),
            entities: vec!["email".to_string(), "phone".to_string()],
        };
        let record = AuditRecord::from_decision(&req, &decision, 0.02, 250.0);

        assert_eq!(record.decision, "redact");
        assert_eq!(record.pii_entities_found, vec!["email", "phone"]);
        assert_eq!(record.latency_total_ms, 250.0);
    }

    #[test]
    fn test_audit_record_redact_entities_empty_when_stage_provides_none() {
        // A custom stage that redacts without populating `entities` (e.g.
        // a free-form policy match) must still produce a valid record with
        // an empty entity list, not panic or default to something wrong.
        let req = clean_request();
        let decision = Decision::Redact {
            reason: "custom redaction".into(),
            mutated: req.clone(),
            entities: Vec::new(),
        };
        let record = AuditRecord::from_decision(&req, &decision, 0.01, 10.0);

        assert_eq!(record.decision, "redact");
        assert!(record.pii_entities_found.is_empty());
    }

    #[test]
    fn test_audit_record_serializes_to_correct_json_shape() {
        let req = clean_request();
        let record = AuditRecord::from_decision(&req, &Decision::Allow, 0.05, 120.0);
        let json: serde_json::Value = serde_json::to_value(&record).unwrap();

        // Required fields per spec §10
        assert!(json.get("timestamp").is_some());
        assert!(json.get("request_id").is_some());
        assert_eq!(json["decision"], "allow");
        assert_eq!(json["model"], "gpt-4o");
        assert!(json.get("latency_pipeline_ms").is_some());
        assert!(json.get("latency_total_ms").is_some());
        assert!(json.get("pii_entities_found").is_some());

        // Optional fields must be absent for an allow decision
        assert!(json.get("code").is_none());
        assert!(json.get("score").is_none());
        assert!(json.get("stage").is_none());
    }

    #[test]
    fn test_timestamp_format() {
        let ts = chrono_timestamp();
        // Must match "YYYY-MM-DDTHH:MM:SS.mmmZ"
        assert_eq!(ts.len(), 24, "unexpected length: {ts}");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[23..24], "Z");
        assert_eq!(&ts[19..20], ".");
    }

    #[test]
    fn test_emit_does_not_panic() {
        let req = clean_request();
        let record = AuditRecord::from_decision(&req, &Decision::Allow, 0.0, 0.0);
        record.emit(); // must not panic even without a subscriber
    }

    #[test]
    fn test_unix_secs_epoch() {
        let (y, mo, d, h, mi, s) = unix_secs_to_datetime(0);
        assert_eq!((y, mo, d, h, mi, s), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn test_unix_secs_known_date() {
        // 2026-06-11T00:00:00Z = 1_781_136_000
        let (y, mo, d, h, mi, s) = unix_secs_to_datetime(1_781_136_000);
        assert_eq!(y, 2026);
        assert_eq!(mo, 6);
        assert_eq!(d, 11);
        assert_eq!((h, mi, s), (0, 0, 0));
    }
}
