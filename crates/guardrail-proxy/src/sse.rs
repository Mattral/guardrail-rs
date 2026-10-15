//! Output-side PII redaction for streamed (`text/event-stream`) responses.
//!
//! [`SseRedactor`] sits between the upstream byte stream and the client.
//! It reassembles server-sent events however the bytes are split, and
//! rewrites the text deltas:
//!
//! | Provider | Text field rewritten | Text released before |
//! |----------|----------------------|----------------------|
//! | OpenAI-compatible | `choices[].delta.content` | that choice's `finish_reason`, or `data: [DONE]` |
//! | Anthropic | `content_block_delta` → `delta.text` (`text_delta`) | that block's `content_block_stop` |
//!
//! Each choice / content block gets its own [`StreamingPiiRedactor`], so a
//! value split across deltas (`"user@"`, `"example.com"`) is still caught.
//! The redactor holds back the last few words of each stream until it knows
//! they're safe, so a delta may be forwarded with less text (or none) than
//! the upstream sent; the held-back text follows in a later event.
//! Everything else (other fields, other event types, tool-call arguments,
//! comments, unparseable data) is forwarded byte for byte.

use std::collections::BTreeMap;

use guardrail_classifiers::{PiiRedactor, StreamingPiiRedactor};
use guardrail_core::request::Provider;
use serde_json::{json, Value};

/// Incremental SSE rewriter. Feed it upstream bytes with [`push`](Self::push)
/// and call [`finish`](Self::finish) when the upstream ends.
pub struct SseRedactor {
    format: Format,
    buf: Vec<u8>,
    /// Per choice index (OpenAI) or content block index (Anthropic).
    streams: BTreeMap<u64, StreamingPiiRedactor>,
    redactions: usize,
    entity_types: Vec<String>,
    /// `id`/`model`/`created`/… from the last OpenAI chunk, copied into
    /// synthetic chunks so strict client parsers accept them.
    openai_meta: serde_json::Map<String, Value>,
}

#[derive(Clone, Copy)]
enum Format {
    OpenAi,
    Anthropic,
}

impl SseRedactor {
    /// A rewriter for responses to a request sent to `provider`.
    pub fn new(provider: &Provider) -> Self {
        Self {
            format: match provider {
                Provider::Anthropic => Format::Anthropic,
                _ => Format::OpenAi,
            },
            buf: Vec::new(),
            streams: BTreeMap::new(),
            redactions: 0,
            entity_types: Vec::new(),
            openai_meta: serde_json::Map::new(),
        }
    }

    /// Add upstream bytes; returns the bytes to send to the client now.
    pub fn push(&mut self, redactor: &PiiRedactor, bytes: &[u8]) -> Vec<u8> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        while let Some((end, sep_len)) = find_event_end(&self.buf) {
            let event: Vec<u8> = self.buf.drain(..end + sep_len).collect();
            self.rewrite_event(redactor, &event[..end], &event[end..], &mut out);
        }
        out
    }

    /// Upstream ended: release anything still held back, then any trailing
    /// partial event as-is.
    pub fn finish(&mut self, redactor: &PiiRedactor) -> Vec<u8> {
        let mut out = Vec::new();
        let indices: Vec<u64> = self.streams.keys().copied().collect();
        for index in indices {
            self.flush_into(redactor, index, &mut out);
        }
        out.extend(std::mem::take(&mut self.buf));
        self.collect_stats();
        out
    }

    /// Total redactions applied so far.
    pub fn redactions(&self) -> usize {
        self.redactions
    }

    /// Distinct entity types redacted so far (e.g. `"Email"`).
    pub fn entity_types(&self) -> &[String] {
        &self.entity_types
    }

    fn rewrite_event(
        &mut self,
        redactor: &PiiRedactor,
        event: &[u8],
        sep: &[u8],
        out: &mut Vec<u8>,
    ) {
        let Ok(text) = std::str::from_utf8(event) else {
            out.extend_from_slice(event);
            out.extend_from_slice(sep);
            return;
        };

        // Split into `data:` payload and everything else, keeping order.
        let mut data_lines = Vec::new();
        let mut other_lines = Vec::new();
        for line in text.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.strip_prefix(' ').unwrap_or(rest));
            } else {
                other_lines.push(line);
            }
        }
        let data = data_lines.join("\n");

        if data.trim() == "[DONE]" {
            let indices: Vec<u64> = self.streams.keys().copied().collect();
            for index in indices {
                self.flush_into(redactor, index, out);
            }
            pass_through(event, sep, out);
            return;
        }

        let Ok(mut value) = serde_json::from_str::<Value>(&data) else {
            pass_through(event, sep, out);
            return;
        };

        let changed = match self.format {
            Format::OpenAi => self.rewrite_openai(redactor, &mut value),
            Format::Anthropic => {
                if value["type"] == "content_block_stop" {
                    if let Some(index) = value["index"].as_u64() {
                        self.flush_into(redactor, index, out);
                    }
                    false
                } else {
                    self.rewrite_anthropic(redactor, &mut value)
                }
            }
        };

        if changed {
            write_event(&other_lines, &value, sep, out);
        } else {
            pass_through(event, sep, out);
        }
    }

    /// Returns whether `value` was modified.
    fn rewrite_openai(&mut self, redactor: &PiiRedactor, value: &mut Value) -> bool {
        for key in ["id", "object", "created", "model", "system_fingerprint"] {
            if let Some(v) = value.get(key) {
                self.openai_meta.insert(key.into(), v.clone());
            }
        }
        let Some(choices) = value["choices"].as_array_mut() else {
            return false;
        };
        let mut changed = false;
        for (pos, choice) in choices.iter_mut().enumerate() {
            let index = choice["index"].as_u64().unwrap_or(pos as u64);
            let finished = !choice["finish_reason"].is_null();
            let incoming = choice["delta"]["content"].as_str().map(str::to_owned);

            let stream = self.streams.entry(index).or_default();
            let mut text = match &incoming {
                Some(t) => stream.push(redactor, t),
                None => String::new(),
            };
            if finished {
                text.push_str(&stream.finish(redactor));
            }

            if incoming.is_some() || !text.is_empty() {
                if incoming.as_deref() != Some(text.as_str()) {
                    changed = true;
                }
                if let Some(delta) = choice.get_mut("delta").and_then(Value::as_object_mut) {
                    delta.insert("content".into(), Value::String(text));
                } else {
                    choice["delta"] = json!({ "content": text });
                }
            }
        }
        changed
    }

    /// Returns whether `value` was modified.
    fn rewrite_anthropic(&mut self, redactor: &PiiRedactor, value: &mut Value) -> bool {
        if value["type"] != "content_block_delta" || value["delta"]["type"] != "text_delta" {
            return false;
        }
        let (Some(index), Some(incoming)) =
            (value["index"].as_u64(), value["delta"]["text"].as_str())
        else {
            return false;
        };
        let incoming = incoming.to_owned();
        let text = self
            .streams
            .entry(index)
            .or_default()
            .push(redactor, &incoming);
        if text == incoming {
            return false;
        }
        value["delta"]["text"] = Value::String(text);
        true
    }

    /// Emit whatever stream `index` still holds as a synthetic delta event.
    fn flush_into(&mut self, redactor: &PiiRedactor, index: u64, out: &mut Vec<u8>) {
        let Some(stream) = self.streams.get_mut(&index) else {
            return;
        };
        let text = stream.finish(redactor);
        if text.is_empty() {
            return;
        }
        let (other, value) = match self.format {
            Format::OpenAi => {
                let mut chunk = self.openai_meta.clone();
                chunk
                    .entry("object")
                    .or_insert_with(|| json!("chat.completion.chunk"));
                chunk.insert(
                    "choices".into(),
                    json!([{ "index": index, "delta": { "content": text }, "finish_reason": null }]),
                );
                (Vec::new(), Value::Object(chunk))
            }
            Format::Anthropic => (
                vec!["event: content_block_delta"],
                json!({
                    "type": "content_block_delta",
                    "index": index,
                    "delta": { "type": "text_delta", "text": text },
                }),
            ),
        };
        write_event(&other, &value, b"\n\n", out);
    }

    fn collect_stats(&mut self) {
        self.redactions = 0;
        for stream in self.streams.values() {
            for r in stream.records() {
                self.redactions += 1;
                let name = format!("{:?}", r.entity_type);
                if !self.entity_types.contains(&name) {
                    self.entity_types.push(name);
                }
            }
        }
    }
}

/// Position and length of the first event terminator (a blank line).
fn find_event_end(buf: &[u8]) -> Option<(usize, usize)> {
    let mut i = 0;
    while i < buf.len() {
        if buf[i] == b'\n' {
            if buf.get(i + 1) == Some(&b'\n') {
                return Some((i, 2));
            }
            if buf.get(i + 1) == Some(&b'\r') && buf.get(i + 2) == Some(&b'\n') {
                return Some((i, 3));
            }
        }
        i += 1;
    }
    None
}

fn pass_through(event: &[u8], sep: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(event);
    out.extend_from_slice(sep);
}

fn write_event(other_lines: &[&str], value: &Value, sep: &[u8], out: &mut Vec<u8>) {
    for line in other_lines.iter().filter(|l| !l.is_empty()) {
        out.extend_from_slice(line.as_bytes());
        out.push(b'\n');
    }
    out.extend_from_slice(b"data: ");
    out.extend_from_slice(value.to_string().as_bytes());
    // `sep` started with the event's final newline; keep the terminator.
    out.extend_from_slice(sep);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn openai_chunk(index: u64, content: &str) -> String {
        format!(
            "data: {}\n\n",
            json!({"object": "chat.completion.chunk", "choices": [{"index": index, "delta": {"content": content}, "finish_reason": null}]})
        )
    }

    fn openai_finish(index: u64) -> String {
        format!(
            "data: {}\n\n",
            json!({"object": "chat.completion.chunk", "choices": [{"index": index, "delta": {}, "finish_reason": "stop"}]})
        )
    }

    /// Run the whole stream through the redactor in `chunk`-byte pieces and
    /// collect the text a client would assemble from the deltas.
    fn run(provider: Provider, stream: &str, chunk: usize) -> (String, String) {
        let r = PiiRedactor::default();
        let mut sse = SseRedactor::new(&provider);
        let mut out = Vec::new();
        for piece in stream.as_bytes().chunks(chunk.max(1)) {
            out.extend(sse.push(&r, piece));
        }
        out.extend(sse.finish(&r));
        let raw = String::from_utf8(out).unwrap();
        let mut text = String::new();
        for event in raw.split("\n\n") {
            for line in event.lines() {
                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };
                let Ok(v) = serde_json::from_str::<Value>(data) else {
                    continue;
                };
                if let Some(c) = v["choices"][0]["delta"]["content"].as_str() {
                    text.push_str(c);
                }
                if v["type"] == "content_block_delta" {
                    if let Some(t) = v["delta"]["text"].as_str() {
                        text.push_str(t);
                    }
                }
            }
        }
        (raw, text)
    }

    #[test]
    fn openai_email_split_across_deltas_is_redacted_for_any_byte_split() {
        let mut stream = String::new();
        for piece in [
            "Sure, write to ",
            "jane.doe@",
            "example",
            ".com and ",
            "I'll reply.",
        ] {
            stream.push_str(&openai_chunk(0, piece));
        }
        stream.push_str(&openai_finish(0));
        stream.push_str("data: [DONE]\n\n");

        for chunk in [1, 2, 3, 7, 16, 64, 4096] {
            let (raw, text) = run(Provider::OpenAI, &stream, chunk);
            assert_eq!(
                text, "Sure, write to [EMAIL] and I'll reply.",
                "chunk={chunk}"
            );
            assert!(!raw.contains("jane.doe"), "chunk={chunk}: {raw}");
            assert!(raw.trim_end().ends_with("data: [DONE]"), "chunk={chunk}");
        }
    }

    #[test]
    fn openai_held_back_text_is_released_by_done_without_finish_reason() {
        let stream = format!(
            "{}data: [DONE]\n\n",
            openai_chunk(0, "call 555-867-5309 now")
        );
        let (_, text) = run(Provider::OpenAI, &stream, 5);
        assert_eq!(text, "call [PHONE] now");
    }

    #[test]
    fn openai_choices_are_redacted_independently() {
        let mut stream = String::new();
        stream.push_str(&openai_chunk(0, "a@"));
        stream.push_str(&openai_chunk(1, "b@"));
        stream.push_str(&openai_chunk(0, "x.co "));
        stream.push_str(&openai_chunk(1, "y.co "));
        stream.push_str("data: [DONE]\n\n");
        let r = PiiRedactor::default();
        let mut sse = SseRedactor::new(&Provider::OpenAI);
        let mut out = sse.push(&r, stream.as_bytes());
        out.extend(sse.finish(&r));
        let raw = String::from_utf8(out).unwrap();
        assert!(!raw.contains("a@x.co") && !raw.contains("b@y.co"), "{raw}");
        assert_eq!(raw.matches("[EMAIL]").count(), 2, "{raw}");
        assert_eq!(sse.redactions(), 2);
    }

    #[test]
    fn anthropic_text_deltas_are_redacted_and_flushed_at_block_stop() {
        let delta = |t: &str| {
            format!(
                "event: content_block_delta\ndata: {}\n\n",
                json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": t}})
            )
        };
        let mut stream =
            String::from("event: message_start\ndata: {\"type\":\"message_start\"}\n\n");
        stream.push_str(&delta("My SSN is 123-"));
        stream.push_str(&delta("45-6789, keep it safe"));
        stream.push_str(
            "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        );
        stream.push_str("event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n");

        for chunk in [1, 4, 33, 4096] {
            let (raw, text) = run(Provider::Anthropic, &stream, chunk);
            assert_eq!(text, "My SSN is [SSN], keep it safe", "chunk={chunk}");
            assert!(!raw.contains("6789"), "chunk={chunk}");
            // The flushed text comes before the block stop.
            let flush = raw.find("keep it safe").unwrap();
            assert!(
                flush < raw.find("content_block_stop").unwrap(),
                "chunk={chunk}"
            );
            assert!(raw.contains("message_start") && raw.contains("message_stop"));
        }
    }

    #[test]
    fn non_text_events_and_comments_pass_through_unchanged() {
        let stream = ": keep-alive\n\nevent: ping\ndata: {\"type\":\"ping\"}\n\ndata: not json\n\n";
        let (raw, _) = run(Provider::Anthropic, stream, 3);
        assert_eq!(raw, stream);
    }

    #[test]
    fn crlf_framing_is_handled() {
        let stream = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"mail a@b.co ok\"},\"finish_reason\":\"stop\"}]}\r\n\r\ndata: [DONE]\r\n\r\n";
        let (_, text) = run(Provider::OpenAI, stream, 2);
        assert_eq!(text, "mail [EMAIL] ok");
    }
}
