//! Output-side PII redaction for non-streaming chat-completion responses.
//!
//! `guardrail-rs` inspects **requests** with the full pipeline
//! ([`guardrail_core::Pipeline`]). Responses are forwarded back to the client
//! largely unmodified — except that, when [`PiiRedactor`] is enabled, this
//! module scans known assistant-message text fields in the JSON response body
//! and redacts any PII the model echoed back (e.g. from retrieved documents,
//! tool outputs, or training-data leakage) before the response reaches the
//! caller.
//!
//! # Supported shapes
//!
//! | Provider | Field(s) scanned |
//! |----------|-------------------|
//! | OpenAI `/v1/chat/completions` | `choices[].message.content` (string) |
//! | Anthropic `/v1/messages` | `content[]` blocks where `type == "text"`, field `text` |
//!
//! # Known limitation: streaming responses
//!
//! When the original request had `"stream": true`, the upstream returns a
//! `text/event-stream` (SSE) body, not a single JSON document. This module
//! does **not** parse or redact SSE chunks — streaming responses are passed
//! through byte-for-byte unmodified. [`is_redactable_response`] returns
//! `false` for such responses so callers can skip this stage entirely. See
//! `docs/architecture.md` for the planned streaming redaction design.

use guardrail_classifiers::PiiRedactor;
use serde_json::Value;

/// Summary of redactions applied to a response body, for the audit log.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ResponseRedactionSummary {
    /// Total number of redactions applied across all text fields.
    pub total_redactions: usize,
    /// Distinct entity types redacted (e.g. `["email", "phone"]`).
    pub entity_types: Vec<String>,
}

impl ResponseRedactionSummary {
    fn is_empty(&self) -> bool {
        self.total_redactions == 0
    }
}

/// Returns `true` if `content_type` and `is_streaming` indicate a response
/// body that [`redact_response_body`] can process (a single JSON document).
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::response::is_redactable_response;
///
/// assert!(is_redactable_response(Some("application/json"), false));
/// assert!(!is_redactable_response(Some("text/event-stream"), true));
/// assert!(!is_redactable_response(Some("text/event-stream"), false));
/// assert!(!is_redactable_response(None, false));
/// ```
pub fn is_redactable_response(content_type: Option<&str>, is_streaming: bool) -> bool {
    if is_streaming {
        return false;
    }
    matches!(content_type, Some(ct) if ct.starts_with("application/json"))
}

/// Scan a non-streaming JSON response body for PII in assistant-message text
/// fields and redact it in place.
///
/// Returns `None` if the body is not valid JSON, doesn't match a known
/// response shape, or contains no PII — in all of these cases the caller
/// should forward the original bytes unchanged.
///
/// On success, returns the re-serialized JSON body (as bytes) and a
/// [`ResponseRedactionSummary`] for the audit log.
///
/// # Examples
///
/// ```rust
/// use guardrail_classifiers::PiiRedactor;
/// use guardrail_proxy::response::redact_response_body;
///
/// let redactor = PiiRedactor::default();
/// let body = br#"{
///     "id": "chatcmpl-123",
///     "choices": [{
///         "index": 0,
///         "message": {"role": "assistant", "content": "Reach us at help@example.com."},
///         "finish_reason": "stop"
///     }]
/// }"#;
///
/// let (new_body, summary) = redact_response_body(body, &redactor).unwrap();
/// let text = String::from_utf8(new_body).unwrap();
/// assert!(text.contains("[EMAIL]"));
/// assert_eq!(summary.total_redactions, 1);
/// ```
pub fn redact_response_body(
    body: &[u8],
    redactor: &PiiRedactor,
) -> Option<(Vec<u8>, ResponseRedactionSummary)> {
    let mut value: Value = serde_json::from_slice(body).ok()?;
    let mut summary = ResponseRedactionSummary::default();
    let mut entity_set = std::collections::HashSet::new();

    let mut changed = false;

    // ── OpenAI shape: choices[].message.content ─────────────────────────────
    if let Some(choices) = value.get_mut("choices").and_then(Value::as_array_mut) {
        for choice in choices.iter_mut() {
            if let Some(content) = choice.get_mut("message").and_then(|m| m.get_mut("content")) {
                if let Some(text) = content.as_str() {
                    if let Some((sanitized, records)) = redactor.redact_response_text(text) {
                        for r in &records {
                            entity_set.insert(format!("{:?}", r.entity_type).to_lowercase());
                        }
                        summary.total_redactions += records.len();
                        *content = Value::String(sanitized);
                        changed = true;
                    }
                }
            }
        }
    }

    // ── Anthropic shape: content[] blocks with type == "text" ───────────────
    if let Some(blocks) = value.get_mut("content").and_then(Value::as_array_mut) {
        for block in blocks.iter_mut() {
            let is_text_block = block.get("type").and_then(Value::as_str) == Some("text");
            if !is_text_block {
                continue;
            }
            if let Some(text_field) = block.get_mut("text") {
                if let Some(text) = text_field.as_str() {
                    if let Some((sanitized, records)) = redactor.redact_response_text(text) {
                        for r in &records {
                            entity_set.insert(format!("{:?}", r.entity_type).to_lowercase());
                        }
                        summary.total_redactions += records.len();
                        *text_field = Value::String(sanitized);
                        changed = true;
                    }
                }
            }
        }
    }

    if !changed || summary.is_empty() {
        return None;
    }

    summary.entity_types = entity_set.into_iter().collect();
    summary.entity_types.sort();

    let new_body = serde_json::to_vec(&value).ok()?;
    Some((new_body, summary))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_redactable_response() {
        assert!(is_redactable_response(Some("application/json"), false));
        assert!(is_redactable_response(
            Some("application/json; charset=utf-8"),
            false
        ));
        assert!(!is_redactable_response(Some("text/event-stream"), true));
        assert!(!is_redactable_response(Some("text/event-stream"), false));
        assert!(!is_redactable_response(Some("application/json"), true));
        assert!(!is_redactable_response(None, false));
    }

    #[test]
    fn test_redact_openai_response_with_pii() {
        let redactor = PiiRedactor::default();
        let body = serde_json::to_vec(&json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "You can reach our support team at help@example.com or 555-867-5309."
                },
                "finish_reason": "stop"
            }]
        }))
        .unwrap();

        let (new_body, summary) = redact_response_body(&body, &redactor).unwrap();
        let value: Value = serde_json::from_slice(&new_body).unwrap();

        let content = value["choices"][0]["message"]["content"].as_str().unwrap();
        assert!(content.contains("[EMAIL]"), "content: {content}");
        assert!(content.contains("[PHONE]"), "content: {content}");
        assert!(!content.contains("help@example.com"));

        assert_eq!(summary.total_redactions, 2);
        assert!(summary.entity_types.contains(&"email".to_string()));
        assert!(summary.entity_types.contains(&"phone".to_string()));

        // Other fields must be preserved unchanged.
        assert_eq!(value["id"], "chatcmpl-123");
        assert_eq!(value["choices"][0]["finish_reason"], "stop");
    }

    #[test]
    fn test_redact_openai_response_multiple_choices() {
        let redactor = PiiRedactor::default();
        let body = serde_json::to_vec(&json!({
            "choices": [
                {"index": 0, "message": {"role": "assistant", "content": "Email alice@example.com"}},
                {"index": 1, "message": {"role": "assistant", "content": "No PII in this one."}}
            ]
        }))
        .unwrap();

        let (new_body, summary) = redact_response_body(&body, &redactor).unwrap();
        let value: Value = serde_json::from_slice(&new_body).unwrap();

        assert!(value["choices"][0]["message"]["content"]
            .as_str()
            .unwrap()
            .contains("[EMAIL]"));
        assert_eq!(
            value["choices"][1]["message"]["content"],
            "No PII in this one."
        );
        assert_eq!(summary.total_redactions, 1);
    }

    #[test]
    fn test_redact_anthropic_response_with_pii() {
        let redactor = PiiRedactor::default();
        let body = serde_json::to_vec(&json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Contact us at help@example.com for assistance."}
            ]
        }))
        .unwrap();

        let (new_body, summary) = redact_response_body(&body, &redactor).unwrap();
        let value: Value = serde_json::from_slice(&new_body).unwrap();

        let text = value["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("[EMAIL]"), "text: {text}");
        assert_eq!(summary.total_redactions, 1);
        assert_eq!(summary.entity_types, vec!["email".to_string()]);
    }

    #[test]
    fn test_anthropic_non_text_blocks_untouched() {
        let redactor = PiiRedactor::default();
        let body = serde_json::to_vec(&json!({
            "content": [
                {"type": "tool_use", "id": "toolu_1", "name": "lookup", "input": {"email": "alice@example.com"}},
                {"type": "text", "text": "Here is the result."}
            ]
        }))
        .unwrap();

        // No PII in the text block, and tool_use blocks are not scanned.
        assert!(redact_response_body(&body, &redactor).is_none());
    }

    #[test]
    fn test_clean_response_returns_none() {
        let redactor = PiiRedactor::default();
        let body = serde_json::to_vec(&json!({
            "id": "chatcmpl-123",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "The capital of France is Paris."},
                "finish_reason": "stop"
            }]
        }))
        .unwrap();

        assert!(redact_response_body(&body, &redactor).is_none());
    }

    #[test]
    fn test_invalid_json_returns_none() {
        let redactor = PiiRedactor::default();
        assert!(redact_response_body(b"not json", &redactor).is_none());
    }

    #[test]
    fn test_empty_choices_returns_none() {
        let redactor = PiiRedactor::default();
        let body = serde_json::to_vec(&json!({"choices": []})).unwrap();
        assert!(redact_response_body(&body, &redactor).is_none());
    }

    #[test]
    fn test_response_without_choices_or_content_returns_none() {
        let redactor = PiiRedactor::default();
        let body = serde_json::to_vec(&json!({"error": {"message": "not found"}})).unwrap();
        assert!(redact_response_body(&body, &redactor).is_none());
    }

    #[test]
    fn test_preserves_unicode_content() {
        let redactor = PiiRedactor::default();
        let body = serde_json::to_vec(&json!({
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "こんにちは、email: user@example.com です。"}
            }]
        }))
        .unwrap();

        let (new_body, summary) = redact_response_body(&body, &redactor).unwrap();
        let value: Value = serde_json::from_slice(&new_body).unwrap();
        let content = value["choices"][0]["message"]["content"].as_str().unwrap();

        assert!(content.contains("こんにちは"));
        assert!(content.contains("[EMAIL]"));
        assert_eq!(summary.total_redactions, 1);
    }
}
