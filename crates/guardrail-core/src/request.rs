//! Request and response types that flow through the guardrail pipeline.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// The normalized, provider-agnostic representation of an LLM request.
///
/// Constructed from a raw HTTP body regardless of provider (OpenAI, Anthropic, etc.).
/// This is the primary type passed through every pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailRequest {
    /// Unique ID for this request, generated at ingress.
    pub id: Uuid,
    /// Wall-clock time the request arrived at the proxy.
    #[serde(with = "system_time_serde")]
    pub received_at: SystemTime,
    /// The full message list, including system prompt.
    pub messages: Vec<ChatMessage>,
    /// Original provider (parsed from Host header or config).
    pub provider: Provider,
    /// Model identifier as sent by the caller.
    pub model: String,
    /// Whether the caller requested streaming.
    pub stream: bool,
    /// All other fields from the original payload, preserved verbatim.
    pub extra: serde_json::Value,
}

impl GuardrailRequest {
    /// Create a new request with the given messages and model.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_core::request::{GuardrailRequest, ChatMessage, Role, MessageContent, Provider};
    ///
    /// let req = GuardrailRequest::new(
    ///     vec![ChatMessage { role: Role::User, content: MessageContent::Text("Hello".into()) }],
    ///     "gpt-4o".into(),
    ///     Provider::OpenAI,
    /// );
    /// assert_eq!(req.model, "gpt-4o");
    /// ```
    pub fn new(messages: Vec<ChatMessage>, model: String, provider: Provider) -> Self {
        Self {
            id: Uuid::new_v4(),
            received_at: SystemTime::now(),
            messages,
            provider,
            model,
            stream: false,
            extra: serde_json::Value::Null,
        }
    }

    /// Extract all text content from user-role messages as a single string.
    ///
    /// Useful for classifiers that operate on the combined user input.
    pub fn user_text(&self) -> String {
        self.messages
            .iter()
            .filter(|m| matches!(m.role, Role::User))
            .map(|m| m.content.as_text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Extract all text content from all messages.
    pub fn all_text(&self) -> String {
        self.messages
            .iter()
            .map(|m| m.content.as_text())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A single message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message author.
    pub role: Role,
    /// The message content (text or multi-part).
    pub content: MessageContent,
}

/// The content of a message — either plain text or a list of content parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Plain text content (most common).
    Text(String),
    /// Multi-part content (text + images).
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    /// Extract all text from the content as a single string.
    pub fn as_text(&self) -> String {
        match self {
            MessageContent::Text(t) => t.clone(),
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| {
                    if let ContentPart::Text { text } = p {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

/// A single content part in a multi-part message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Plain text part.
    Text {
        /// The text content.
        text: String,
    },
    /// Image URL part.
    ImageUrl {
        /// The image URL details.
        image_url: ImageUrl,
    },
}

/// An image referenced by URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    /// The URL of the image.
    pub url: String,
    /// The detail level (`"low"`, `"high"`, `"auto"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// The role of a message author.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System-level instruction.
    System,
    /// End-user turn.
    User,
    /// Model turn.
    Assistant,
    /// Tool / function call result.
    Tool,
}

/// The upstream LLM provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// OpenAI (api.openai.com).
    OpenAI,
    /// Anthropic (api.anthropic.com).
    Anthropic,
    /// Azure OpenAI Service.
    Azure,
    /// Any other provider (e.g., Ollama, vLLM).
    Other(String),
}

/// Serde helper for `SystemTime`.
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S: Serializer>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
        let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs_f64();
        secs.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SystemTime, D::Error> {
        let secs = f64::deserialize(d)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs_f64(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_text_extraction() {
        let req = GuardrailRequest::new(
            vec![
                ChatMessage {
                    role: Role::System,
                    content: MessageContent::Text("You are a helpful assistant.".into()),
                },
                ChatMessage {
                    role: Role::User,
                    content: MessageContent::Text("Hello!".into()),
                },
            ],
            "gpt-4o".into(),
            Provider::OpenAI,
        );

        let text = req.user_text();
        assert_eq!(text, "Hello!");
    }

    #[test]
    fn test_message_content_as_text_parts() {
        let content = MessageContent::Parts(vec![
            ContentPart::Text { text: "Hello".into() },
            ContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "https://example.com/img.png".into(),
                    detail: None,
                },
            },
            ContentPart::Text { text: "World".into() },
        ]);
        assert_eq!(content.as_text(), "Hello\nWorld");
    }

    #[test]
    fn test_request_roundtrip_json() {
        let req = GuardrailRequest::new(
            vec![ChatMessage {
                role: Role::User,
                content: MessageContent::Text("Test".into()),
            }],
            "gpt-4o".into(),
            Provider::OpenAI,
        );
        let json = serde_json::to_string(&req).unwrap();
        let decoded: GuardrailRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.model, req.model);
        assert_eq!(decoded.id, req.id);
    }
}
