//! PII detection and redaction stage.
//!
//! Detects and replaces personally identifiable information (PII) before
//! it reaches the upstream LLM. Uses compiled regular expressions for zero
//! external dependencies. Optionally uses a NER ONNX model for name/address
//! detection when the `onnx-pii` feature is enabled.
//!
//! **Performance target:** < 20 µs p99 for inputs up to 4 KB (regex path).

use guardrail_core::{
    decision::Decision,
    error::GuardrailError,
    pipeline::Stage,
    request::{ChatMessage, GuardrailRequest, MessageContent},
};
use regex::Regex;
use std::borrow::Cow;

// ── Entity types ─────────────────────────────────────────────────────────────

/// The type of PII entity detected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiEntityType {
    /// Email address (RFC 5321).
    Email,
    /// Phone number (E.164 / common US formats).
    Phone,
    /// Credit card number (Luhn-validated 13–19 digit sequences).
    CreditCard,
    /// US Social Security Number.
    Ssn,
    /// IPv4 or IPv6 address.
    IpAddress,
    /// API key (OpenAI sk-*, GitHub ghp_*, Bearer tokens, etc.).
    ApiKey,
    /// AWS access key ID.
    AwsKey,
}

impl PiiEntityType {
    /// Return the default replacement string for this entity type.
    pub fn default_replacement(&self) -> &'static str {
        match self {
            PiiEntityType::Email => "[EMAIL]",
            PiiEntityType::Phone => "[PHONE]",
            PiiEntityType::CreditCard => "[CARD]",
            PiiEntityType::Ssn => "[SSN]",
            PiiEntityType::IpAddress => "[IP_ADDRESS]",
            PiiEntityType::ApiKey => "[API_KEY]",
            PiiEntityType::AwsKey => "[AWS_KEY]",
        }
    }
}

// ── Redaction record (for audit log) ─────────────────────────────────────────

/// A single redaction that was applied to a request or response.
///
/// **Offset caveat:** `offset` and `length` are byte positions in the text
/// *as seen by the pattern that matched*, after any earlier patterns in the
/// configured entity list have already been applied. For a single
/// `PiiEntityType`, offsets are accurate against the pre-redaction input for
/// that entity's pass; across entity types, offsets are not directly
/// comparable to a single shared "original text" coordinate space. Consumers
/// that need precise original-text spans should match on `entity_type` and
/// re-scan the original text with that entity's pattern.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RedactionRecord {
    /// The type of entity that was redacted.
    pub entity_type: PiiEntityType,
    /// The byte offset where the match began (see offset caveat above).
    pub offset: usize,
    /// The length (in bytes) of the matched text.
    pub length: usize,
}

// ── Internal pattern entry ────────────────────────────────────────────────────

struct PatternEntry {
    entity_type: PiiEntityType,
    regex: Regex,
    replacement: String,
}

// ── Luhn validation ───────────────────────────────────────────────────────────

/// Validates a digit string using the Luhn algorithm.
///
/// Returns `true` if the number passes the Luhn check.
fn luhn_valid(digits: &str) -> bool {
    let digits: Vec<u32> = digits
        .chars()
        .filter(|c| c.is_ascii_digit())
        .filter_map(|c| c.to_digit(10))
        .collect();

    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }

    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 1 {
                let doubled = d * 2;
                if doubled > 9 {
                    doubled - 9
                } else {
                    doubled
                }
            } else {
                d
            }
        })
        .sum();

    sum % 10 == 0
}

// ── PiiRedactor ───────────────────────────────────────────────────────────────

/// Detects and replaces PII in request messages before forwarding upstream.
///
/// Returns [`Decision::Allow`] if no PII is found, or [`Decision::Redact`]
/// with a sanitized request copy if PII was replaced. Never returns
/// [`Decision::Block`].
///
/// # Examples
///
/// ```rust
/// use guardrail_classifiers::PiiRedactor;
/// use guardrail_core::{Stage, Decision};
/// use guardrail_core::test_helpers::pii_request;
///
/// # tokio_test::block_on(async {
/// let redactor = PiiRedactor::default();
/// let req = pii_request();
/// let d = redactor.evaluate(&req).await.unwrap();
/// assert!(matches!(d, Decision::Redact { .. }));
/// # });
/// ```
pub struct PiiRedactor {
    patterns: Vec<PatternEntry>,
    /// Whether to apply Luhn validation to candidate credit card numbers.
    validate_luhn: bool,
}

impl std::fmt::Debug for PiiRedactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PiiRedactor")
            .field("num_patterns", &self.patterns.len())
            .field("validate_luhn", &self.validate_luhn)
            .finish()
    }
}

impl Default for PiiRedactor {
    /// Build a redactor with all default entity patterns enabled.
    ///
    /// # Panics
    ///
    /// Panics if any bundled regex fails to compile — this is a compile-time
    /// invariant validated by the test suite.
    fn default() -> Self {
        Self::new(
            vec![
                PiiEntityType::Email,
                PiiEntityType::CreditCard,
                PiiEntityType::Phone,
                PiiEntityType::Ssn,
                PiiEntityType::IpAddress,
                PiiEntityType::ApiKey,
                PiiEntityType::AwsKey,
            ],
            true,
        )
        .expect("default PII patterns must compile without error")
    }
}

impl PiiRedactor {
    /// Build a redactor for a specific set of entity types.
    ///
    /// # Errors
    ///
    /// Returns [`GuardrailError::Regex`] if a pattern fails to compile.
    pub fn new(entities: Vec<PiiEntityType>, validate_luhn: bool) -> Result<Self, GuardrailError> {
        let mut patterns = Vec::with_capacity(entities.len());
        for entity in entities {
            let pattern_str = entity_pattern(&entity);
            let regex = Regex::new(pattern_str)?;
            let replacement = entity.default_replacement().to_string();
            patterns.push(PatternEntry {
                entity_type: entity,
                regex,
                replacement,
            });
        }
        Ok(Self {
            patterns,
            validate_luhn,
        })
    }

    /// Redact PII in a plain text string, returning the sanitized text.
    ///
    /// This is the core operation; [`PiiRedactor::redact_request`] and
    /// [`PiiRedactor::redact_response_text`] call this for each piece of text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_classifiers::PiiRedactor;
    ///
    /// let redactor = PiiRedactor::default();
    /// let out = redactor.redact_text("Email me at user@example.com");
    /// assert!(out.contains("[EMAIL]"));
    /// assert!(!out.contains("user@example.com"));
    /// ```
    pub fn redact_text(&self, input: &str) -> String {
        self.redact_text_with_records(input).0
    }

    /// Redact PII in a plain text string, returning both the sanitized text
    /// and a list of [`RedactionRecord`]s describing what was found (entity
    /// type, byte offset, and length in the **original** text).
    ///
    /// If no PII is found, the returned `Vec` is empty and the returned
    /// string equals `input`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_classifiers::{PiiRedactor, PiiEntityType};
    ///
    /// let redactor = PiiRedactor::default();
    /// let (out, records) = redactor.redact_text_with_records("Email me at user@example.com");
    /// assert!(out.contains("[EMAIL]"));
    /// assert_eq!(records.len(), 1);
    /// assert_eq!(records[0].entity_type, PiiEntityType::Email);
    /// ```
    pub fn redact_text_with_records(&self, input: &str) -> (String, Vec<RedactionRecord>) {
        let mut result = Cow::Borrowed(input);
        let mut records = Vec::new();

        for entry in &self.patterns {
            // Record matches against the *current* (possibly already-redacted)
            // text before this entry's pass. Offsets are best-effort: once an
            // earlier pattern has changed the text, later offsets are relative
            // to the post-earlier-pass text, not the original input. This is
            // acceptable for audit purposes (entity type + count matter most);
            // see `RedactionRecord` docs.
            for m in entry.regex.find_iter(&result) {
                if entry.entity_type == PiiEntityType::CreditCard && self.validate_luhn {
                    let digits_only: String =
                        m.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
                    if !luhn_valid(&digits_only) {
                        continue;
                    }
                }
                records.push(RedactionRecord {
                    entity_type: entry.entity_type.clone(),
                    offset: m.start(),
                    length: m.len(),
                });
            }

            // Special handling for credit cards: apply Luhn check.
            if entry.entity_type == PiiEntityType::CreditCard && self.validate_luhn {
                let replaced = entry
                    .regex
                    .replace_all(&result, |caps: &regex::Captures<'_>| {
                        let matched = caps.get(0).map_or("", |m| m.as_str());
                        let digits_only: String =
                            matched.chars().filter(|c| c.is_ascii_digit()).collect();
                        if luhn_valid(&digits_only) {
                            entry.replacement.clone()
                        } else {
                            matched.to_string()
                        }
                    });
                result = Cow::Owned(replaced.into_owned());
            } else {
                let replaced = entry.regex.replace_all(&result, entry.replacement.as_str());
                if let Cow::Owned(s) = replaced {
                    result = Cow::Owned(s);
                }
            }
        }

        (result.into_owned(), records)
    }

    /// Redact PII in all messages of a request.
    ///
    /// Returns the sanitized request and a list of redaction records for the
    /// audit log. Returns `None` if no PII was found.
    pub fn redact_request(
        &self,
        req: &GuardrailRequest,
    ) -> Option<(GuardrailRequest, Vec<RedactionRecord>)> {
        let mut any_changed = false;
        let mut all_records = Vec::new();

        let new_messages: Vec<ChatMessage> = req
            .messages
            .iter()
            .map(|msg| {
                let original_text = msg.content.as_text();
                let (redacted_text, records) = self.redact_text_with_records(&original_text);

                if !records.is_empty() {
                    any_changed = true;
                    all_records.extend(records);
                    ChatMessage {
                        role: msg.role.clone(),
                        content: MessageContent::Text(redacted_text),
                    }
                } else {
                    msg.clone()
                }
            })
            .collect();

        if !any_changed {
            return None;
        }

        let mut mutated = req.clone();
        mutated.messages = new_messages;
        Some((mutated, all_records))
    }

    /// Redact PII in a single block of free-form text (e.g. an LLM response).
    ///
    /// This is the response-side counterpart to [`PiiRedactor::redact_request`].
    /// Use it to sanitize assistant-generated content before returning it to
    /// the caller, so that PII the model echoes back (or hallucinates) from
    /// its training data or tool outputs never reaches the client unredacted.
    ///
    /// Returns `None` if no PII was found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_classifiers::PiiRedactor;
    ///
    /// let redactor = PiiRedactor::default();
    ///
    /// let (sanitized, records) = redactor
    ///     .redact_response_text("Sure, you can reach support at help@example.com.")
    ///     .unwrap();
    /// assert!(sanitized.contains("[EMAIL]"));
    /// assert_eq!(records.len(), 1);
    ///
    /// assert!(redactor.redact_response_text("Nothing sensitive here.").is_none());
    /// ```
    pub fn redact_response_text(&self, text: &str) -> Option<(String, Vec<RedactionRecord>)> {
        let (redacted, records) = self.redact_text_with_records(text);
        if records.is_empty() {
            None
        } else {
            Some((redacted, records))
        }
    }
}

#[async_trait::async_trait]
impl Stage for PiiRedactor {
    fn name(&self) -> &'static str {
        "pii_redactor"
    }

    async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError> {
        match self.redact_request(req) {
            None => Ok(Decision::Allow),
            Some((mutated, records)) => {
                let entity_summary: Vec<String> = records
                    .iter()
                    .map(|r| format!("{:?}", r.entity_type))
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();

                let reason = format!("PII detected and redacted: {}", entity_summary.join(", "));

                tracing::info!(
                    entities = ?entity_summary,
                    num_redactions = records.len(),
                    "PII redacted from request"
                );

                Ok(Decision::Redact {
                    reason,
                    mutated,
                    entities: entity_summary,
                })
            }
        }
    }
}

// ── Regex patterns ────────────────────────────────────────────────────────────

fn entity_pattern(entity: &PiiEntityType) -> &'static str {
    match entity {
        PiiEntityType::Email => r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}",
        PiiEntityType::Phone => {
            // Matches: +1-555-867-5309, (555) 867-5309, 555.867.5309, 5558675309
            r"(?:\+?1[\s\-.]?)?\(?\d{3}\)?[\s\-.]?\d{3}[\s\-.]?\d{4}"
        }
        PiiEntityType::CreditCard => {
            // Matches 13–19 digit sequences with optional spaces/hyphens
            r"\b(?:\d[ \-]?){13,19}\b"
        }
        PiiEntityType::Ssn => r"\b\d{3}[-\s]\d{2}[-\s]\d{4}\b",
        PiiEntityType::IpAddress => {
            // IPv4 + simplified IPv6
            r"\b(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b|\b(?:[0-9a-fA-F]{1,4}:){2,7}[0-9a-fA-F]{1,4}\b"
        }
        PiiEntityType::ApiKey => {
            // OpenAI sk-..., Anthropic sk-ant-..., GitHub ghp_/gho_/ghs_, Bearer tokens
            // Relax GH token length to a reasonable range to match common examples
            r"(?:sk-[a-zA-Z0-9\-_]{20,}|sk-ant-[a-zA-Z0-9\-_]{20,}|ghp_[a-zA-Z0-9]{30,40}|gho_[a-zA-Z0-9]{30,40}|ghs_[a-zA-Z0-9]{30,40}|Bearer\s+[a-zA-Z0-9\-._~+/]{20,})"
        }
        PiiEntityType::AwsKey => r"\bAKIA[A-Z0-9]{16}\b",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use guardrail_core::test_helpers::{clean_request, pii_request};
    use rstest::rstest;

    #[test]
    fn test_default_redactor_compiles() {
        let r = PiiRedactor::default();
        assert!(!r.patterns.is_empty());
    }

    // ── Email ────────────────────────────────────────────────────────────────

    #[rstest]
    #[case("Contact user@example.com for info", "[EMAIL]")]
    #[case("Send to first.last+tag@sub.domain.org today", "[EMAIL]")]
    fn test_email_redacted(#[case] input: &str, #[case] replacement: &str) {
        let r = PiiRedactor::default();
        let out = r.redact_text(input);
        assert!(out.contains(replacement), "output: {out}");
        assert!(!out.contains('@'));
    }

    // ── SSN ──────────────────────────────────────────────────────────────────

    #[rstest]
    #[case("SSN is 123-45-6789")]
    #[case("Social: 987 65 4321")]
    fn test_ssn_redacted(#[case] input: &str) {
        let r = PiiRedactor::default();
        let out = r.redact_text(input);
        assert!(out.contains("[SSN]"), "output: {out}");
    }

    // ── Credit card (Luhn) ───────────────────────────────────────────────────

    #[test]
    fn test_valid_credit_card_redacted() {
        // 4111111111111111 is the canonical Luhn-valid Visa test number
        let r = PiiRedactor::default();
        let out = r.redact_text("My card: 4111111111111111");
        assert!(out.contains("[CARD]"), "output: {out}");
    }

    #[test]
    fn test_invalid_luhn_not_redacted() {
        // 4111111111111112 fails Luhn
        let r = PiiRedactor::default();
        let out = r.redact_text("Number: 4111111111111112");
        assert!(!out.contains("[CARD]"), "output: {out}");
    }

    // ── API keys ─────────────────────────────────────────────────────────────

    #[rstest]
    #[case("key = sk-abcdefghijklmnopqrstuvwxyzABCDE")]
    #[case("token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ123456789")]
    #[case("AKIAIOSFODNN7EXAMPLE")]
    fn test_api_key_redacted(#[case] input: &str) {
        let r = PiiRedactor::default();
        let out = r.redact_text(input);
        // At least one of the key replacements should appear
        assert!(
            out.contains("[API_KEY]") || out.contains("[AWS_KEY]"),
            "output: {out}"
        );
    }

    // ── IP addresses ─────────────────────────────────────────────────────────

    #[test]
    fn test_ipv4_redacted() {
        let r = PiiRedactor::default();
        let out = r.redact_text("Server at 192.168.1.100");
        assert!(out.contains("[IP_ADDRESS]"), "output: {out}");
    }

    // ── Full request ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_pii_request_becomes_redact_decision() {
        let redactor = PiiRedactor::default();
        let req = pii_request();
        let d = redactor.evaluate(&req).await.unwrap();
        assert!(matches!(d, Decision::Redact { .. }));
    }

    #[tokio::test]
    async fn test_clean_request_stays_allow() {
        let redactor = PiiRedactor::default();
        let req = clean_request();
        let d = redactor.evaluate(&req).await.unwrap();
        assert_eq!(d, Decision::Allow);
    }

    // ── Idempotency ──────────────────────────────────────────────────────────

    #[test]
    fn test_redaction_is_idempotent() {
        let r = PiiRedactor::default();
        let input = "Contact user@example.com or call 555-867-5309";
        let first = r.redact_text(input);
        let second = r.redact_text(&first);
        assert_eq!(first, second, "redaction must be idempotent");
    }

    // ── Response-side redaction ───────────────────────────────────────────────

    #[test]
    fn test_redact_response_text_with_pii() {
        let r = PiiRedactor::default();
        let (sanitized, records) = r
            .redact_response_text("You can reach our support team at help@example.com.")
            .unwrap();

        assert!(sanitized.contains("[EMAIL]"));
        assert!(!sanitized.contains("help@example.com"));
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].entity_type, PiiEntityType::Email);
    }

    #[test]
    fn test_redact_response_text_without_pii_returns_none() {
        let r = PiiRedactor::default();
        assert!(r
            .redact_response_text("Here is a haiku about autumn leaves.")
            .is_none());
    }

    #[test]
    fn test_redact_response_text_multiple_entities() {
        let r = PiiRedactor::default();
        let (sanitized, records) = r
            .redact_response_text("Email alice@example.com or call 555-867-5309.")
            .unwrap();

        assert!(sanitized.contains("[EMAIL]"));
        assert!(sanitized.contains("[PHONE]"));
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_redact_text_with_records_matches_redact_text() {
        let r = PiiRedactor::default();
        let input = "Contact user@example.com for info";
        let (text_only, _) = r.redact_text_with_records(input);
        assert_eq!(text_only, r.redact_text(input));
    }

    #[test]
    fn test_redaction_record_serializes() {
        let r = PiiRedactor::default();
        let (_, records) = r.redact_text_with_records("Email me at user@example.com");
        let json = serde_json::to_string(&records[0]).unwrap();
        assert!(json.contains("\"entity_type\":\"email\""));
    }

    // ── Property-based tests ──────────────────────────────────────────────────

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn no_panic_on_arbitrary_utf8(s in "\\PC*") {
            let r = PiiRedactor::default();
            let _ = r.redact_text(&s);
        }

        #[test]
        fn redaction_is_idempotent_prop(s in "[a-zA-Z0-9@.\\-\\s]{0,200}") {
            let r = PiiRedactor::default();
            let first = r.redact_text(&s);
            let second = r.redact_text(&first);
            assert_eq!(first, second);
        }
    }

    // ── Luhn helper ──────────────────────────────────────────────────────────

    #[rstest]
    #[case("4111111111111111", true)] // Visa test
    #[case("5500005555555559", true)] // Mastercard test
    #[case("4111111111111112", false)] // Invalid
    #[case("1234567890123456", false)] // Invalid
    fn test_luhn(#[case] digits: &str, #[case] expected: bool) {
        assert_eq!(luhn_valid(digits), expected);
    }
}
