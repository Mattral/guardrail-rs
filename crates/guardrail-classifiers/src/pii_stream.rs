//! Incremental PII redaction for text that arrives in pieces (e.g. the
//! token deltas of a streamed LLM response).
//!
//! [`StreamingPiiRedactor`] buffers just enough text to never emit part of
//! a PII value before it knows whether the value is complete: a match can
//! span pieces (`"user@"` then `"example.com"`). Its contract is that the
//! concatenation of everything it emits equals
//! [`PiiRedactor::redact_text`] of the concatenated input, however the
//! input was split.
//!
//! How much it holds back:
//!
//! - Text is only released up to a whitespace boundary at least
//!   [`HOLD_BACK_BYTES`] before the end of what has arrived, and never in
//!   the middle of something a PII pattern currently matches. Every pattern
//!   either can't contain whitespace (emails, keys) or is much shorter than
//!   the hold-back (phones, SSNs, card numbers).
//! - A run of more than [`MAX_CARRY_BYTES`] without whitespace is released
//!   anyway so memory stays bounded; a PII value straddling that forced cut
//!   could then be missed. Real model output practically never contains
//!   8 KB without a space or newline.
//!
//! The delay this adds is roughly the last few words of output, released as
//! soon as more text (or the end of the stream) arrives.

use crate::pii::{PiiRedactor, RedactionRecord};

/// Minimum number of trailing bytes held back from each release.
pub const HOLD_BACK_BYTES: usize = 64;

/// Longest run held back while waiting for a whitespace boundary.
pub const MAX_CARRY_BYTES: usize = 8 * 1024;

/// Redacts a text stream piece by piece. See the [module docs](self).
///
/// # Examples
///
/// ```rust
/// use guardrail_classifiers::{PiiRedactor, StreamingPiiRedactor};
///
/// let redactor = PiiRedactor::default();
/// let mut stream = StreamingPiiRedactor::new();
/// let mut out = String::new();
/// for piece in ["Write to user@", "example.com", " for help."] {
///     out.push_str(&stream.push(&redactor, piece));
/// }
/// out.push_str(&stream.finish(&redactor));
/// assert_eq!(out, "Write to [EMAIL] for help.");
/// ```
#[derive(Debug, Default)]
pub struct StreamingPiiRedactor {
    carry: String,
    records: Vec<RedactionRecord>,
    /// Don't look for a cut again until `carry` reaches this length. Finding
    /// a cut costs a few small redactions, so it's done once per
    /// [`RETRY_AFTER_BYTES`] of new text rather than on every token.
    next_attempt_len: usize,
}

/// New bytes needed before trying to release text again.
const RETRY_AFTER_BYTES: usize = 32;

impl StreamingPiiRedactor {
    /// An empty stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add the next piece of text and return whatever can now be released,
    /// already redacted. May return an empty string.
    pub fn push(&mut self, redactor: &PiiRedactor, piece: &str) -> String {
        self.carry.push_str(piece);
        if self.carry.len() < self.next_attempt_len.max(HOLD_BACK_BYTES + 1) {
            return String::new();
        }
        let cut = self.safe_cut(redactor);
        let released = if cut == 0 {
            String::new()
        } else {
            let head: String = self.carry.drain(..cut).collect();
            self.redact(redactor, &head)
        };
        self.next_attempt_len = self.carry.len() + RETRY_AFTER_BYTES;
        released
    }

    /// End of stream: release (and redact) everything still held back.
    pub fn finish(&mut self, redactor: &PiiRedactor) -> String {
        let rest = std::mem::take(&mut self.carry);
        self.redact(redactor, &rest)
    }

    /// Bytes currently held back.
    pub fn pending_len(&self) -> usize {
        self.carry.len()
    }

    /// Everything redacted so far. Offsets are relative to the piece of
    /// text each redaction was applied to, not to the whole stream; use the
    /// entity types and count.
    pub fn records(&self) -> &[RedactionRecord] {
        &self.records
    }

    fn redact(&mut self, redactor: &PiiRedactor, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        let (out, records) = redactor.redact_text_with_records(text);
        self.records.extend(records);
        out
    }

    /// The largest prefix of `carry` that is safe to release now.
    fn safe_cut(&self, redactor: &PiiRedactor) -> usize {
        let buf = self.carry.as_str();
        if buf.len() <= HOLD_BACK_BYTES {
            return 0;
        }
        let limit = floor_char_boundary(buf, buf.len() - HOLD_BACK_BYTES);

        // Candidate cuts are word starts (just after whitespace, before
        // non-whitespace), nearest to the hold-back limit first. The released
        // part then ends in whitespace, so `\b` and optional separators see
        // the same boundary they would in the whole text.
        let candidates = word_starts_at_or_before(buf, limit);
        if candidates.is_empty() {
            return if buf.len() > MAX_CARRY_BYTES {
                limit
            } else {
                0
            };
        }

        // A cut is accepted only if redacting the two sides separately gives
        // exactly what redacting the whole buffer gives. That covers matches
        // that only exist after an earlier pattern's replacement (e.g. an
        // SSN that gains a word boundary once a neighbouring email becomes
        // "[EMAIL]"). Cuts touching a raw match are skipped first, since such
        // a match could still grow with text that hasn't arrived.
        let raw: Vec<(usize, usize)> = redactor.raw_matches(buf).collect();
        let whole = redactor.redact_text(buf);
        for &cut in candidates.iter().take(MAX_CUT_ATTEMPTS) {
            if raw.iter().any(|&(start, end)| start < cut && end >= cut) {
                continue;
            }
            let mut split = redactor.redact_text(&buf[..cut]);
            split.push_str(&redactor.redact_text(&buf[cut..]));
            if split == whole {
                return cut;
            }
        }
        0
    }
}

/// Candidate cut positions tried per release before holding back instead.
const MAX_CUT_ATTEMPTS: usize = 8;

/// Every `p <= max` (with `p > 0`) where `s[p - 1]` is whitespace and `s[p]`
/// is not, largest first.
fn word_starts_at_or_before(s: &str, max: usize) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut prev_ws = false;
    for (i, c) in s.char_indices() {
        if i > max {
            break;
        }
        if prev_ws && !c.is_whitespace() {
            starts.push(i);
        }
        prev_ws = c.is_whitespace();
    }
    starts.reverse();
    starts
}

fn floor_char_boundary(s: &str, mut i: usize) -> usize {
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn stream_all(redactor: &PiiRedactor, pieces: &[&str]) -> String {
        let mut s = StreamingPiiRedactor::new();
        let mut out = String::new();
        for p in pieces {
            out.push_str(&s.push(redactor, p));
        }
        out.push_str(&s.finish(redactor));
        out
    }

    /// Split `text` at the given byte offsets (snapped to char boundaries).
    fn split_at(text: &str, cuts: &[usize]) -> Vec<String> {
        let mut points: Vec<usize> = cuts
            .iter()
            .map(|&c| floor_char_boundary(text, c.min(text.len())))
            .collect();
        points.sort_unstable();
        points.dedup();
        let mut pieces = Vec::new();
        let mut last = 0;
        for p in points {
            pieces.push(text[last..p].to_string());
            last = p;
        }
        pieces.push(text[last..].to_string());
        pieces
    }

    #[test]
    fn email_split_across_pieces_is_redacted() {
        let r = PiiRedactor::default();
        assert_eq!(
            stream_all(&r, &["Write to user@", "example.com", " for help."]),
            "Write to [EMAIL] for help."
        );
    }

    #[test]
    fn every_split_of_a_card_number_is_redacted() {
        let r = PiiRedactor::default();
        let text = "My card is 4111 1111 1111 1111, thanks.";
        for i in 0..=text.len() {
            let pieces = split_at(text, &[i]);
            let refs: Vec<&str> = pieces.iter().map(String::as_str).collect();
            assert_eq!(stream_all(&r, &refs), r.redact_text(text), "split at {i}");
        }
    }

    #[test]
    fn text_is_released_as_it_arrives() {
        // Long output without PII must not sit in the buffer: after enough
        // text, most of it has been released.
        let r = PiiRedactor::default();
        let mut s = StreamingPiiRedactor::new();
        let mut released = 0;
        for _ in 0..50 {
            released += s.push(&r, "the quick brown fox jumps over ").len();
        }
        assert!(
            s.pending_len() < HOLD_BACK_BYTES + 40,
            "{}",
            s.pending_len()
        );
        assert!(released > 50 * 31 - (HOLD_BACK_BYTES + 40));
    }

    #[test]
    fn long_run_without_whitespace_is_bounded() {
        let r = PiiRedactor::default();
        let mut s = StreamingPiiRedactor::new();
        let mut out = String::new();
        for _ in 0..1000 {
            out.push_str(&s.push(&r, "abcdefghij"));
        }
        assert!(s.pending_len() <= MAX_CARRY_BYTES + 10);
        out.push_str(&s.finish(&r));
        assert_eq!(out.len(), 10_000);
    }

    #[test]
    fn records_are_collected() {
        let r = PiiRedactor::default();
        let mut s = StreamingPiiRedactor::new();
        s.push(&r, "mail a@b.co and call 555-867-5309 ");
        s.finish(&r);
        assert_eq!(s.records().len(), 2);
    }

    const FRAGMENTS: &[&str] = &[
        "user@example.com",
        "a@b.co",
        "4111111111111111",
        "4111 1111 1111 1111",
        "4111-1111-1111-1112",
        "555-867-5309",
        "(555) 867-5309",
        "+1 555.867.5309",
        "123-45-6789",
        "123 45 6789",
        "192.168.1.1",
        "fe80:0:0:0:200:f8ff:fe21:67cf",
        "sk-abcdefghijklmnopqrstuvwxyz",
        "Bearer abcdefghijklmnopqrstuvwxyz0123",
        "AKIAABCDEFGHIJKLMNOP",
        " ",
        "  ",
        "\n",
        "-",
        ".",
        "@",
        "x",
        "1",
        "12",
        "the quick brown fox ",
        "é",
        "[EMAIL]",
    ];

    proptest! {
        /// The contract: for any input and any way of splitting it, the
        /// streamed output equals redacting the whole text at once.
        #[test]
        fn streamed_output_equals_whole_text_redaction(
            parts in proptest::collection::vec(proptest::sample::select(FRAGMENTS), 0..60),
            cuts in proptest::collection::vec(0usize..2000, 0..40),
        ) {
            let r = PiiRedactor::default();
            let text: String = parts.concat();
            let pieces = split_at(&text, &cuts);
            let refs: Vec<&str> = pieces.iter().map(String::as_str).collect();
            prop_assert_eq!(stream_all(&r, &refs), r.redact_text(&text));
        }

        #[test]
        fn streamed_output_equals_whole_text_redaction_random_chars(
            text in "[a-zA-Z0-9@._%+\\-:()\\[\\] \\n]{0,600}",
            cuts in proptest::collection::vec(0usize..600, 0..40),
        ) {
            let r = PiiRedactor::default();
            let pieces = split_at(&text, &cuts);
            let refs: Vec<&str> = pieces.iter().map(String::as_str).collect();
            prop_assert_eq!(stream_all(&r, &refs), r.redact_text(&text));
        }
    }
}
