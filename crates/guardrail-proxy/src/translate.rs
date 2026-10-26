//! Conversion between raw JSON request bodies and [`GuardrailRequest`].
//!
//! The original JSON body is the source of truth. [`parse_body`] walks it
//! and pulls out every string the stages should scan, remembering where
//! each one came from (a JSON pointer). Stages see those strings as a
//! [`GuardrailRequest`]. [`ScannedBody::apply`] then writes the (possibly
//! redacted) strings back to the same places in the original body.
//! Everything else in the body (tool calls, images, `cache_control`,
//! unknown fields and part types) is never rebuilt, so it reaches the
//! upstream exactly as the client sent it.
//!
//! # What is scanned
//!
//! | Provider | Strings |
//! |----------|---------|
//! | OpenAI-compatible | message `content` strings, `text` and `refusal` parts, assistant `refusal`, `tool_calls[].function.arguments`, legacy `function_call.arguments` |
//! | Anthropic | top-level `system` (string or text blocks), message `content` strings, `text` blocks, `tool_use` input (every string value), `tool_result` content (string or text blocks), plain-text `document` sources |
//!
//! Images, audio and files are not scanned (there is no text to scan), and
//! Anthropic `thinking` blocks are left alone: they are signed, and
//! changing them would make the upstream reject the request.

use guardrail_core::{
    request::{
        ChatMessage, ContentPart, GuardrailRequest, ImageUrl, MessageContent, Provider, Role,
    },
    GuardrailError,
};
use serde_json::{json, Value};

/// A parsed request body, with the location of every string the stages
/// scan. Produced by [`parse_body`].
#[derive(Debug, Clone)]
pub struct ScannedBody {
    body: Value,
    /// For each message of the [`GuardrailRequest`], the JSON pointers of
    /// its text, in the order the message's text parts appear.
    pointers: Vec<Vec<String>>,
}

impl ScannedBody {
    /// The original body.
    pub fn body(&self) -> &Value {
        &self.body
    }

    /// Write the text of `req` (normally the pipeline's final, possibly
    /// redacted request) back into the original body and return it.
    ///
    /// # Errors
    ///
    /// Returns [`GuardrailError::Internal`] if `req` no longer lines up
    /// with the body: a different number of messages, or of text parts in a
    /// message. Built-in stages never do that; the caller should refuse to
    /// forward rather than send text that may not have been redacted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_core::request::{MessageContent, Provider};
    /// use guardrail_proxy::translate::parse_body;
    ///
    /// let body = br#"{"model":"gpt-4o","messages":[{"role":"user","name":"al","content":"hi al@example.com"}]}"#;
    /// let (mut req, scanned) = parse_body(body, Provider::OpenAI).unwrap();
    /// req.messages[0].content = MessageContent::Text("hi [EMAIL]".into());
    ///
    /// let out = scanned.apply(&req).unwrap();
    /// assert_eq!(out["messages"][0]["content"], "hi [EMAIL]");
    /// assert_eq!(out["messages"][0]["name"], "al");
    /// ```
    pub fn apply(mut self, req: &GuardrailRequest) -> Result<Value, GuardrailError> {
        if req.messages.len() != self.pointers.len() {
            return Err(GuardrailError::Internal(format!(
                "request has {} messages after the pipeline, expected {}",
                req.messages.len(),
                self.pointers.len()
            )));
        }
        for (i, (message, pointers)) in req.messages.iter().zip(&self.pointers).enumerate() {
            let texts = message_texts(&message.content);
            if texts.len() != pointers.len() {
                // A message with no text (e.g. only an image) is `Text("")`.
                if pointers.is_empty() && texts.iter().all(|t| t.is_empty()) {
                    continue;
                }
                return Err(GuardrailError::Internal(format!(
                    "message {i} has {} text parts after the pipeline, expected {}",
                    texts.len(),
                    pointers.len()
                )));
            }
            for (pointer, text) in pointers.iter().zip(texts) {
                let slot = self.body.pointer_mut(pointer).ok_or_else(|| {
                    GuardrailError::Internal(format!("request body has no {pointer}"))
                })?;
                if slot.as_str() != Some(text) {
                    *slot = Value::String(text.to_string());
                }
            }
        }
        Ok(self.body)
    }
}

/// The text parts of `content`, in order.
fn message_texts(content: &MessageContent) -> Vec<&str> {
    match content {
        MessageContent::Text(text) => vec![text.as_str()],
        MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => Some(text.as_str()),
                ContentPart::ImageUrl { .. } => None,
            })
            .collect(),
    }
}

/// Parse a raw JSON request body into a [`GuardrailRequest`].
///
/// Shorthand for [`parse_body`] when the body itself isn't needed.
///
/// # Errors
///
/// As for [`parse_body`].
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::translate::parse_request;
/// use guardrail_core::request::Provider;
///
/// let body = br#"{
///     "model": "gpt-4o",
///     "messages": [{"role": "user", "content": "Hello"}]
/// }"#;
///
/// let req = parse_request(body, Provider::OpenAI).unwrap();
/// assert_eq!(req.model, "gpt-4o");
/// assert_eq!(req.messages.len(), 1);
/// ```
pub fn parse_request(body: &[u8], provider: Provider) -> Result<GuardrailRequest, GuardrailError> {
    parse_body(body, provider).map(|(req, _)| req)
}

/// Parse a raw JSON request body into the [`GuardrailRequest`] the stages
/// see, plus a [`ScannedBody`] to write their changes back with.
///
/// `GuardrailRequest::extra` holds the body without `model`, `messages` and
/// `stream`, for stages that want other fields. An Anthropic top-level
/// `system` becomes a leading system message.
///
/// # Errors
///
/// Returns [`GuardrailError::Serialization`] if `body` is not valid JSON, or
/// [`GuardrailError::Internal`] if `model` or `messages` is missing or
/// malformed, or a message has an unknown role.
pub fn parse_body(
    body: &[u8],
    provider: Provider,
) -> Result<(GuardrailRequest, ScannedBody), GuardrailError> {
    let value: Value = serde_json::from_slice(body)?;

    let model = value
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| GuardrailError::Internal("missing or invalid 'model' field".into()))?
        .to_string();
    let stream = value
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let messages = value
        .get("messages")
        .ok_or_else(|| GuardrailError::Internal("missing 'messages' field".into()))?
        .as_array()
        .ok_or_else(|| GuardrailError::Internal("'messages' must be an array".into()))?;

    let mut extracted = Vec::with_capacity(messages.len() + 1);
    if provider == Provider::Anthropic {
        if let Some(system) = extract_anthropic_system(value.get("system"))? {
            extracted.push(system);
        }
    }
    for (i, message) in messages.iter().enumerate() {
        extracted.push(extract_message(message, i, &provider)?);
    }

    let mut extra = value.clone();
    if let Value::Object(map) = &mut extra {
        map.remove("model");
        map.remove("messages");
        map.remove("stream");
    }

    let (messages, pointers) = extracted.into_iter().map(Extracted::finish).unzip();
    let req = GuardrailRequest {
        id: uuid::Uuid::new_v4(),
        received_at: std::time::SystemTime::now(),
        messages,
        provider,
        model,
        stream,
        extra,
    };
    Ok((
        req,
        ScannedBody {
            body: value,
            pointers,
        },
    ))
}

/// One message's scannable text, with where each piece came from.
struct Extracted {
    role: Role,
    segments: Vec<Segment>,
    /// The message's content is a single string with nothing else
    /// extracted: it is presented as `MessageContent::Text`.
    plain: bool,
}

enum Segment {
    Text { pointer: String, text: String },
    Image(ImageUrl),
}

impl Extracted {
    fn new(role: Role) -> Self {
        Self {
            role,
            segments: Vec::new(),
            plain: false,
        }
    }

    /// Text from a content field that is a plain string.
    fn plain_text(&mut self, pointer: String, text: &str) {
        self.plain = self.segments.is_empty();
        self.text(pointer, text);
    }

    fn text(&mut self, pointer: String, text: &str) {
        if !self.segments.is_empty() {
            self.plain = false;
        }
        self.segments.push(Segment::Text {
            pointer,
            text: text.to_string(),
        });
    }

    fn finish(self) -> (ChatMessage, Vec<String>) {
        let mut pointers = Vec::new();
        let mut parts = Vec::with_capacity(self.segments.len());
        for segment in self.segments {
            match segment {
                Segment::Text { pointer, text } => {
                    pointers.push(pointer);
                    parts.push(ContentPart::Text { text });
                }
                Segment::Image(image_url) => parts.push(ContentPart::ImageUrl { image_url }),
            }
        }
        let content = match parts.as_slice() {
            [] => MessageContent::Text(String::new()),
            [ContentPart::Text { text }] if self.plain => MessageContent::Text(text.clone()),
            _ => MessageContent::Parts(parts),
        };
        (
            ChatMessage {
                role: self.role,
                content,
            },
            pointers,
        )
    }
}

/// The Anthropic top-level `system`: a string, or an array of text blocks.
fn extract_anthropic_system(system: Option<&Value>) -> Result<Option<Extracted>, GuardrailError> {
    let mut out = Extracted::new(Role::System);
    match system {
        None | Some(Value::Null) => {}
        Some(Value::String(s)) => out.plain_text("/system".into(), s),
        Some(Value::Array(blocks)) => {
            for (j, block) in blocks.iter().enumerate() {
                if let Some(text) = text_block(block) {
                    out.text(format!("/system/{j}/text"), text);
                }
            }
        }
        Some(other) => {
            return Err(GuardrailError::Internal(format!(
                "'system' has unsupported JSON type: {other}"
            )))
        }
    }
    Ok((!out.segments.is_empty()).then_some(out))
}

fn parse_role(message: &Value, i: usize) -> Result<Role, GuardrailError> {
    let role = message
        .get("role")
        .and_then(Value::as_str)
        .ok_or_else(|| GuardrailError::Internal(format!("messages[{i}] missing 'role' field")))?;
    match role {
        "system" | "developer" => Ok(Role::System),
        "user" => Ok(Role::User),
        "assistant" => Ok(Role::Assistant),
        "tool" | "function" => Ok(Role::Tool),
        other => Err(GuardrailError::Internal(format!(
            "messages[{i}] has unknown role '{other}'"
        ))),
    }
}

fn extract_message(
    message: &Value,
    i: usize,
    provider: &Provider,
) -> Result<Extracted, GuardrailError> {
    let mut out = Extracted::new(parse_role(message, i)?);
    let base = format!("/messages/{i}");
    let anthropic = *provider == Provider::Anthropic;

    match message.get("content") {
        None | Some(Value::Null) => {}
        Some(Value::String(s)) => out.plain_text(format!("{base}/content"), s),
        Some(Value::Array(parts)) => {
            for (j, part) in parts.iter().enumerate() {
                let at = format!("{base}/content/{j}");
                if anthropic {
                    extract_anthropic_block(part, &at, &mut out);
                } else {
                    extract_openai_part(part, &at, &mut out);
                }
            }
        }
        Some(other) => {
            return Err(GuardrailError::Internal(format!(
                "messages[{i}] 'content' has unsupported JSON type: {other}"
            )))
        }
    }

    if !anthropic {
        if let Some(refusal) = message.get("refusal").and_then(Value::as_str) {
            out.text(format!("{base}/refusal"), refusal);
        }
        if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
            for (k, call) in calls.iter().enumerate() {
                if let Some(args) = call.pointer("/function/arguments").and_then(Value::as_str) {
                    out.text(format!("{base}/tool_calls/{k}/function/arguments"), args);
                }
            }
        }
        if let Some(args) = message
            .pointer("/function_call/arguments")
            .and_then(Value::as_str)
        {
            out.text(format!("{base}/function_call/arguments"), args);
        }
    }
    Ok(out)
}

/// An OpenAI content part. Parts with no text (audio, files, unknown
/// types) are skipped.
fn extract_openai_part(part: &Value, at: &str, out: &mut Extracted) {
    match part.get("type").and_then(Value::as_str).unwrap_or("text") {
        "text" => {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                out.text(format!("{at}/text"), text);
            }
        }
        "refusal" => {
            if let Some(text) = part.get("refusal").and_then(Value::as_str) {
                out.text(format!("{at}/refusal"), text);
            }
        }
        "image_url" => {
            if let Some(url) = part.pointer("/image_url/url").and_then(Value::as_str) {
                out.segments.push(Segment::Image(ImageUrl {
                    url: url.to_string(),
                    detail: part
                        .pointer("/image_url/detail")
                        .and_then(Value::as_str)
                        .map(String::from),
                }));
            }
        }
        _ => {}
    }
}

/// An Anthropic content block. Blocks with no text (images, `thinking`,
/// unknown types) are skipped.
fn extract_anthropic_block(block: &Value, at: &str, out: &mut Extracted) {
    match block.get("type").and_then(Value::as_str) {
        Some("text") => {
            if let Some(text) = block.get("text").and_then(Value::as_str) {
                out.text(format!("{at}/text"), text);
            }
        }
        Some("tool_use") => {
            if let Some(input) = block.get("input") {
                extract_strings(input, &format!("{at}/input"), out);
            }
        }
        Some("tool_result") => match block.get("content") {
            Some(Value::String(s)) => out.text(format!("{at}/content"), s),
            Some(Value::Array(inner)) => {
                for (m, inner_block) in inner.iter().enumerate() {
                    if let Some(text) = text_block(inner_block) {
                        out.text(format!("{at}/content/{m}/text"), text);
                    }
                }
            }
            _ => {}
        },
        Some("document")
            if block.pointer("/source/type").and_then(Value::as_str) == Some("text") =>
        {
            if let Some(data) = block.pointer("/source/data").and_then(Value::as_str) {
                out.text(format!("{at}/source/data"), data);
            }
        }
        _ => {}
    }
}

/// Every string inside `value` (tool-call input), depth first.
fn extract_strings(value: &Value, at: &str, out: &mut Extracted) {
    match value {
        Value::String(s) => out.text(at.to_string(), s),
        Value::Array(items) => {
            for (n, item) in items.iter().enumerate() {
                extract_strings(item, &format!("{at}/{n}"), out);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                extract_strings(item, &format!("{at}/{}", escape_pointer(key)), out);
            }
        }
        _ => {}
    }
}

/// Escape an object key for use in a JSON pointer (RFC 6901).
fn escape_pointer(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

/// The `text` of a `{"type": "text", "text": ..}` block.
fn text_block(block: &Value) -> Option<&str> {
    if block.get("type").and_then(Value::as_str) != Some("text") {
        return None;
    }
    block.get("text").and_then(Value::as_str)
}

/// Build a JSON error body for a blocked request, matching the OpenAI error
/// envelope shape so client SDKs can parse it without special-casing.
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::translate::block_response_body;
/// use guardrail_core::decision::BlockCode;
///
/// let body = block_response_body("Prompt injection detected.", &BlockCode::PromptInjection, "req-123");
/// assert_eq!(body["error"]["code"], "prompt_injection");
/// assert_eq!(body["error"]["guardrail_request_id"], "req-123");
/// ```
pub fn block_response_body(
    reason: &str,
    code: &guardrail_core::decision::BlockCode,
    request_id: &str,
) -> Value {
    json!({
        "error": {
            "message": reason,
            "type": "guardrail_block",
            "code": code.as_str(),
            "guardrail_request_id": request_id,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn openai(body: &[u8]) -> (GuardrailRequest, ScannedBody) {
        parse_body(body, Provider::OpenAI).unwrap()
    }

    fn anthropic(body: &[u8]) -> (GuardrailRequest, ScannedBody) {
        parse_body(body, Provider::Anthropic).unwrap()
    }

    /// Run the default PII redactor over `req` the way the pipeline would,
    /// and write the result back.
    fn redact_and_apply(req: GuardrailRequest, scanned: ScannedBody) -> Value {
        let redactor = guardrail_classifiers::PiiRedactor::default();
        let req = match redactor.redact_request(&req) {
            Some((mutated, _)) => mutated,
            None => req,
        };
        scanned.apply(&req).unwrap()
    }

    #[test]
    fn test_parse_simple_openai_request() {
        let (req, _) = openai(
            br#"{
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello!"}
            ],
            "temperature": 0.5
        }"#,
        );
        assert_eq!(req.model, "gpt-4o");
        assert_eq!(req.messages.len(), 2);
        assert_eq!(
            req.messages[1].content,
            MessageContent::Text("Hello!".into())
        );
        assert_eq!(req.extra["temperature"], 0.5);
        assert!(req.extra.get("messages").is_none());
    }

    #[test]
    fn test_parse_errors() {
        for body in [
            &br#"{"messages": []}"#[..],
            br#"{"model": "gpt-4o"}"#,
            b"not json",
            br#"{"model": "gpt-4o", "messages": "not-an-array"}"#,
            br#"{"model": "gpt-4o", "messages": [{"content": "Hi"}]}"#,
            br#"{"model": "gpt-4o", "messages": [{"role": "alien", "content": "Hi"}]}"#,
            br#"{"model": "gpt-4o", "messages": [{"role": "user", "content": 42}]}"#,
        ] {
            assert!(
                parse_body(body, Provider::OpenAI).is_err(),
                "{}",
                String::from_utf8_lossy(body)
            );
        }
    }

    #[test]
    fn test_roles() {
        let (req, _) = openai(
            br#"{"model": "m", "messages": [
                {"role": "system", "content": "a"},
                {"role": "developer", "content": "b"},
                {"role": "user", "content": "c"},
                {"role": "assistant", "content": "d"},
                {"role": "tool", "tool_call_id": "t", "content": "e"},
                {"role": "function", "name": "f", "content": "f"}
            ]}"#,
        );
        let roles: Vec<Role> = req.messages.iter().map(|m| m.role.clone()).collect();
        assert_eq!(
            roles,
            [
                Role::System,
                Role::System,
                Role::User,
                Role::Assistant,
                Role::Tool,
                Role::Tool
            ]
        );
    }

    #[test]
    fn test_message_without_content_is_allowed() {
        let body = br#"{"model": "m", "messages": [{"role": "user"}]}"#;
        let (req, scanned) = openai(body);
        assert_eq!(req.messages[0].content, MessageContent::Text(String::new()));
        let original: Value = serde_json::from_slice(body).unwrap();
        assert_eq!(scanned.apply(&req).unwrap(), original);
    }

    #[test]
    fn test_multipart_text_and_images_are_extracted() {
        let (req, _) = openai(
            br#"{"model": "gpt-4o", "messages": [{"role": "user", "content": [
                {"type": "text", "text": "What's in this image?"},
                {"type": "image_url", "image_url": {"url": "https://example.com/img.png", "detail": "high"}},
                {"type": "input_audio", "input_audio": {"data": "AAAA", "format": "wav"}},
                {"type": "video", "video": "x"}
            ]}]}"#,
        );
        let MessageContent::Parts(parts) = &req.messages[0].content else {
            panic!("expected parts")
        };
        assert_eq!(parts.len(), 2);
        assert_eq!(req.messages[0].content.as_text(), "What's in this image?");
        assert!(
            matches!(&parts[1], ContentPart::ImageUrl { image_url } if image_url.detail.as_deref() == Some("high"))
        );
    }

    #[test]
    fn test_tool_call_arguments_and_refusals_are_scanned() {
        let (req, _) = openai(
            br#"{"model": "m", "messages": [{"role": "assistant", "content": "calling",
                "refusal": "no",
                "tool_calls": [{"id": "c", "type": "function", "function": {"name": "f", "arguments": "{\"q\":1}"}}],
                "function_call": {"name": "g", "arguments": "{\"r\":2}"}
            }]}"#,
        );
        let texts = message_texts(&req.messages[0].content);
        assert_eq!(texts, ["calling", "no", "{\"q\":1}", "{\"r\":2}"]);
    }

    #[test]
    fn test_body_is_returned_unchanged_when_nothing_is_redacted() {
        let body = br#"{
            "model": "gpt-4o",
            "messages": [
                {"role": "user", "name": "alice", "content": "Hi"},
                {"role": "assistant", "content": null, "tool_calls": [
                    {"id": "c1", "type": "function", "function": {"name": "f", "arguments": "{}"}}
                ]},
                {"role": "tool", "tool_call_id": "c1", "content": "ok"}
            ],
            "tools": [{"type": "function", "function": {"name": "f"}}],
            "temperature": 0.7
        }"#;
        let (req, scanned) = openai(body);
        let original: Value = serde_json::from_slice(body).unwrap();
        let out = redact_and_apply(req, scanned);
        assert_eq!(out, original);
        assert!(out.get("stream").is_none(), "no stream key is added");
    }

    #[test]
    fn test_redactions_land_on_their_own_pointers() {
        let (req, scanned) = openai(
            br#"{"model": "m", "messages": [{"role": "assistant", "content": [
                {"type": "text", "text": "a@example.com"},
                {"type": "image_url", "image_url": {"url": "https://example.com/x.png"}},
                {"type": "text", "text": "clean"}
            ], "tool_calls": [{"id": "c", "type": "function",
                "function": {"name": "f", "arguments": "{\"to\":\"b@example.com\"}"}}]}]}"#,
        );
        let out = redact_and_apply(req, scanned);
        let msg = &out["messages"][0];
        assert_eq!(msg["content"][0]["text"], "[EMAIL]");
        assert_eq!(
            msg["content"][1]["image_url"]["url"],
            "https://example.com/x.png"
        );
        assert_eq!(msg["content"][2]["text"], "clean");
        assert_eq!(
            msg["tool_calls"][0]["function"]["arguments"],
            "{\"to\":\"[EMAIL]\"}"
        );
    }

    #[test]
    fn test_anthropic_system_string_is_scanned_and_written_back() {
        let (req, scanned) = anthropic(
            br#"{"model": "m", "system": "mail bob@example.com", "messages": [{"role": "user", "content": "Hi"}]}"#,
        );
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].role, Role::System);
        let out = redact_and_apply(req, scanned);
        assert_eq!(out["system"], "mail [EMAIL]");
        assert_eq!(out["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_anthropic_system_blocks_keep_cache_control() {
        let (req, scanned) = anthropic(
            br#"{"model": "m", "system": [
                {"type": "text", "text": "You are a support bot."},
                {"type": "text", "text": "mail bob@example.com", "cache_control": {"type": "ephemeral"}}
            ], "messages": [{"role": "user", "content": "Hi"}]}"#,
        );
        assert!(req.all_text().contains("bob@example.com"));
        let out = redact_and_apply(req, scanned);
        assert_eq!(out["system"][0]["text"], "You are a support bot.");
        assert_eq!(out["system"][1]["text"], "mail [EMAIL]");
        assert_eq!(out["system"][1]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_anthropic_blocks() {
        let (req, scanned) = anthropic(
            br#"{"model": "m", "messages": [
                {"role": "user", "content": [
                    {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "iVBOR"}},
                    {"type": "document", "source": {"type": "text", "media_type": "text/plain", "data": "doc e@example.com"}},
                    {"type": "tool_result", "tool_use_id": "t", "content": "result f@example.com"}
                ]},
                {"role": "assistant", "content": [
                    {"type": "thinking", "thinking": "g@example.com", "signature": "sig"},
                    {"type": "tool_use", "id": "t", "name": "n", "input": {"a/b": ["h@example.com", 3], "c~d": {"e": "x"}}}
                ]}
            ]}"#,
        );
        assert_eq!(
            message_texts(&req.messages[0].content),
            ["doc e@example.com", "result f@example.com"]
        );
        // Thinking is not scanned; tool_use input strings are.
        assert_eq!(
            message_texts(&req.messages[1].content),
            ["h@example.com", "x"]
        );

        let out = redact_and_apply(req, scanned);
        let user = &out["messages"][0]["content"];
        assert_eq!(user[1]["source"]["data"], "doc [EMAIL]");
        assert_eq!(user[2]["content"], "result [EMAIL]");
        let assistant = &out["messages"][1]["content"];
        assert_eq!(assistant[0]["thinking"], "g@example.com");
        assert_eq!(assistant[1]["input"]["a/b"][0], "[EMAIL]");
        assert_eq!(assistant[1]["input"]["a/b"][1], 3);
        assert_eq!(assistant[1]["input"]["c~d"]["e"], "x");
    }

    #[test]
    fn test_apply_refuses_a_request_that_no_longer_lines_up() {
        let (mut req, scanned) = openai(
            br#"{"model": "m", "messages": [{"role": "user", "content": [
                {"type": "text", "text": "a"}, {"type": "text", "text": "b"}
            ]}]}"#,
        );
        req.messages[0].content = MessageContent::Text("ab".into());
        assert!(scanned.clone().apply(&req).is_err());
        req.messages.clear();
        assert!(scanned.apply(&req).is_err());
    }

    #[test]
    fn test_block_response_body_shape() {
        let body = block_response_body(
            "test reason",
            &guardrail_core::decision::BlockCode::PromptInjection,
            "req-abc",
        );
        assert_eq!(body["error"]["message"], "test reason");
        assert_eq!(body["error"]["code"], "prompt_injection");
        assert_eq!(body["error"]["guardrail_request_id"], "req-abc");
        assert_eq!(body["error"]["type"], "guardrail_block");
    }

    #[test]
    fn test_stream_flag() {
        let (req, _) = openai(br#"{"model": "m", "messages": [], "stream": true}"#);
        assert!(req.stream);
    }
}
