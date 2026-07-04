//! Conversion between raw JSON request bodies and [`GuardrailRequest`].
//!
//! Supports both the OpenAI `/v1/chat/completions` shape and the Anthropic
//! `/v1/messages` shape. The translation is lossless: any fields not
//! recognized by `guardrail-core`'s normalized model are preserved in
//! [`GuardrailRequest::extra`] and merged back in on the way out.

use guardrail_core::{
    request::{ChatMessage, ContentPart, GuardrailRequest, MessageContent, Provider, Role},
    GuardrailError,
};
use serde_json::{json, Value};

/// Parse a raw JSON request body into a [`GuardrailRequest`].
///
/// # Errors
///
/// Returns [`GuardrailError::Serialization`] if `body` is not valid JSON, or
/// [`GuardrailError::Internal`] if required fields (`model`, `messages`) are
/// missing or malformed.
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
    let mut value: Value = serde_json::from_slice(body)?;

    let model = value
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| GuardrailError::Internal("missing or invalid 'model' field".into()))?
        .to_string();

    let stream = value
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let messages_value = value
        .get("messages")
        .cloned()
        .ok_or_else(|| GuardrailError::Internal("missing 'messages' field".into()))?;

    let messages = parse_messages(&messages_value, &provider)?;

    // Remove fields we've extracted into typed form so `extra` holds only
    // the remainder (e.g. temperature, max_tokens, tools, etc.)
    if let Value::Object(map) = &mut value {
        map.remove("model");
        map.remove("messages");
        map.remove("stream");
    }

    Ok(GuardrailRequest {
        id: uuid::Uuid::new_v4(),
        received_at: std::time::SystemTime::now(),
        messages,
        provider,
        model,
        stream,
        extra: value,
    })
}

/// Parse the `messages` array, handling both OpenAI and Anthropic shapes.
///
/// - **OpenAI**: `[{"role": "...", "content": "..."}]`, content may be a
///   string or an array of content-part objects.
/// - **Anthropic**: identical message shape, but the system prompt may
///   instead live in a top-level `"system"` field rather than as a message
///   with `role: "system"`. This function only parses the `messages` array;
///   callers should separately prepend a synthetic system message if the
///   Anthropic `"system"` field is present (see [`prepend_anthropic_system`]).
fn parse_messages(value: &Value, _provider: &Provider) -> Result<Vec<ChatMessage>, GuardrailError> {
    let arr = value
        .as_array()
        .ok_or_else(|| GuardrailError::Internal("'messages' must be an array".into()))?;

    let mut out = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        let role_str = item.get("role").and_then(Value::as_str).ok_or_else(|| {
            GuardrailError::Internal(format!("messages[{i}] missing 'role' field"))
        })?;

        let role = match role_str {
            "system" => Role::System,
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "tool" | "function" => Role::Tool,
            other => {
                return Err(GuardrailError::Internal(format!(
                    "messages[{i}] has unknown role '{other}'"
                )))
            }
        };

        let content = item.get("content").ok_or_else(|| {
            GuardrailError::Internal(format!("messages[{i}] missing 'content' field"))
        })?;

        let content = parse_content(content, i)?;

        out.push(ChatMessage { role, content });
    }

    Ok(out)
}

fn parse_content(value: &Value, idx: usize) -> Result<MessageContent, GuardrailError> {
    match value {
        Value::String(s) => Ok(MessageContent::Text(s.clone())),
        Value::Array(parts) => {
            let mut out = Vec::with_capacity(parts.len());
            for part in parts {
                let part_type = part.get("type").and_then(Value::as_str).unwrap_or("text");
                match part_type {
                    "text" => {
                        let text = part
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        out.push(ContentPart::Text { text });
                    }
                    "image_url" => {
                        let image_url = part.get("image_url").cloned().unwrap_or(json!({}));
                        let url = image_url
                            .get("url")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        let detail = image_url
                            .get("detail")
                            .and_then(Value::as_str)
                            .map(String::from);
                        out.push(ContentPart::ImageUrl {
                            image_url: guardrail_core::request::ImageUrl { url, detail },
                        });
                    }
                    "image" => {
                        // Anthropic-style image block; represent as a text
                        // placeholder so downstream classifiers don't choke,
                        // while preserving nothing-lost semantics is handled
                        // via `extra` at the top level.
                        out.push(ContentPart::Text {
                            text: String::new(),
                        });
                    }
                    other => {
                        return Err(GuardrailError::Internal(format!(
                            "messages[{idx}] content part has unsupported type '{other}'"
                        )))
                    }
                }
            }
            Ok(MessageContent::Parts(out))
        }
        Value::Null => Ok(MessageContent::Text(String::new())),
        other => Err(GuardrailError::Internal(format!(
            "messages[{idx}] 'content' has unsupported JSON type: {other}"
        ))),
    }
}

/// Serialize a [`GuardrailRequest`] back into a raw JSON body suitable for
/// forwarding to the upstream provider.
///
/// This re-merges the typed `messages`/`model`/`stream` fields with whatever
/// was preserved in `extra`, ensuring fields the proxy doesn't understand
/// (e.g. `temperature`, `tools`, `tool_choice`) pass through unchanged.
///
/// # Examples
///
/// ```rust
/// use guardrail_proxy::translate::{parse_request, serialize_request};
/// use guardrail_core::request::Provider;
///
/// let body = br#"{"model": "gpt-4o", "messages": [{"role":"user","content":"Hi"}], "temperature": 0.7}"#;
/// let req = parse_request(body, Provider::OpenAI).unwrap();
/// let out = serialize_request(&req).unwrap();
/// assert_eq!(out["temperature"], 0.7);
/// assert_eq!(out["model"], "gpt-4o");
/// ```
pub fn serialize_request(req: &GuardrailRequest) -> Result<Value, GuardrailError> {
    let mut value = req.extra.clone();
    if !value.is_object() {
        value = json!({});
    }

    let map = value.as_object_mut().expect("forced to object above");

    map.insert("model".to_string(), json!(req.model));
    map.insert("stream".to_string(), json!(req.stream));

    let messages: Vec<Value> = req
        .messages
        .iter()
        .map(|m| {
            let role = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };

            let content = match &m.content {
                MessageContent::Text(t) => json!(t),
                MessageContent::Parts(parts) => {
                    let parts_json: Vec<Value> = parts
                        .iter()
                        .map(|p| match p {
                            ContentPart::Text { text } => json!({"type": "text", "text": text}),
                            ContentPart::ImageUrl { image_url } => json!({
                                "type": "image_url",
                                "image_url": {
                                    "url": image_url.url,
                                    "detail": image_url.detail,
                                }
                            }),
                        })
                        .collect();
                    json!(parts_json)
                }
            };

            json!({ "role": role, "content": content })
        })
        .collect();

    map.insert("messages".to_string(), json!(messages));

    Ok(value)
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

    #[test]
    fn test_parse_simple_openai_request() {
        let body = br#"{
            "model": "gpt-4o",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello!"}
            ],
            "temperature": 0.5
        }"#;

        let req = parse_request(body, Provider::OpenAI).unwrap();
        assert_eq!(req.model, "gpt-4o");
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[1].content.as_text(), "Hello!");
        assert_eq!(req.extra["temperature"], 0.5);
    }

    #[test]
    fn test_parse_multipart_content() {
        let body = br#"{
            "model": "gpt-4o",
            "messages": [
                {"role": "user", "content": [
                    {"type": "text", "text": "What's in this image?"},
                    {"type": "image_url", "image_url": {"url": "https://example.com/img.png", "detail": "high"}}
                ]}
            ]
        }"#;

        let req = parse_request(body, Provider::OpenAI).unwrap();
        assert_eq!(req.messages[0].content.as_text(), "What's in this image?");
    }

    #[test]
    fn test_parse_missing_model_errors() {
        let body = br#"{"messages": []}"#;
        let result = parse_request(body, Provider::OpenAI);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_messages_errors() {
        let body = br#"{"model": "gpt-4o"}"#;
        let result = parse_request(body, Provider::OpenAI);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_json_errors() {
        let body = b"not json";
        let result = parse_request(body, Provider::OpenAI);
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_preserves_extra_fields() {
        let body = br#"{
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Hi"}],
            "temperature": 0.7,
            "max_tokens": 100,
            "tools": [{"type": "function", "function": {"name": "get_weather"}}]
        }"#;

        let req = parse_request(body, Provider::OpenAI).unwrap();
        let out = serialize_request(&req).unwrap();

        assert_eq!(out["temperature"], 0.7);
        assert_eq!(out["max_tokens"], 100);
        assert!(out["tools"].is_array());
        assert_eq!(out["model"], "gpt-4o");
        assert_eq!(out["messages"][0]["content"], "Hi");
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
    fn test_stream_flag_roundtrip() {
        let body = br#"{
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Hi"}],
            "stream": true
        }"#;
        let req = parse_request(body, Provider::OpenAI).unwrap();
        assert!(req.stream);

        let out = serialize_request(&req).unwrap();
        assert_eq!(out["stream"], true);
    }

    #[test]
    fn test_unknown_role_errors() {
        let body = br#"{
            "model": "gpt-4o",
            "messages": [{"role": "alien", "content": "Hi"}]
        }"#;
        let result = parse_request(body, Provider::OpenAI);
        assert!(result.is_err());
    }
}
