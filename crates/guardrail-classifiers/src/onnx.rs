//! ONNX-backed classifier implementations.
//!
//! Enabled by the `onnx` feature flag. Provides:
//! - [`OnnxInjectionClassifier`]: DeBERTa-v3-base fine-tuned on prompt injection
//! - [`ToxicityClassifier`]: unbiased-toxic-roberta for hate speech / harm detection
//!
//! Both classifiers load their ONNX sessions **once at startup** and share the
//! session across all concurrent requests (ONNX Runtime sessions are internally
//! thread-safe). Inference runs in `tokio::task::spawn_blocking` to avoid
//! blocking the async executor.
//!
//! An inference failure (including a model whose output doesn't have the
//! expected shape) is returned as `Err`, never a panic, so the pipeline can
//! apply the stage's configured fail mode (`on_error`).

#![cfg(feature = "onnx")]

use guardrail_core::{
    decision::{BlockCode, Decision},
    error::GuardrailError,
    pipeline::Stage,
    request::{GuardrailRequest, Role},
};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

type SharedSession = Arc<Mutex<ort::session::Session>>;

/// Longest token sequence sent to the model in one inference, special tokens
/// included. Both recommended models accept at most 512 positions (RoBERTa
/// fails outright beyond that; DeBERTa runs, but its cost grows
/// quadratically and it was trained on ≤512).
pub const MAX_WINDOW_TOKENS: usize = 512;

/// Tokens shared by consecutive windows, so a phrase that straddles a
/// window boundary is still seen whole by one of them.
pub const WINDOW_STRIDE_TOKENS: usize = 128;

/// Most windows scored for one request. Longer input is an inference error
/// (handled by the stage's `on_error`) rather than an unbounded number of
/// inferences: at the default 10 MiB body limit an attacker could otherwise
/// force thousands of model runs per request.
pub const MAX_WINDOWS: usize = 16;

fn internal(e: impl std::fmt::Display) -> GuardrailError {
    GuardrailError::Internal(e.to_string())
}

fn load_session_and_tokenizer(
    model_path: impl AsRef<Path>,
    tokenizer_path: impl AsRef<Path>,
) -> Result<(SharedSession, Arc<tokenizers::Tokenizer>), GuardrailError> {
    let session = ort::session::Session::builder()
        .map_err(internal)?
        .with_intra_threads(1)
        .map_err(internal)?
        .commit_from_file(model_path)
        .map_err(internal)?;

    let mut tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path).map_err(internal)?;
    // Split long input into overlapping windows instead of sending one
    // over-long sequence. Overrides whatever truncation tokenizer.json sets.
    tokenizer
        .with_truncation(Some(tokenizers::TruncationParams {
            max_length: MAX_WINDOW_TOKENS,
            stride: WINDOW_STRIDE_TOKENS,
            strategy: tokenizers::TruncationStrategy::LongestFirst,
            direction: tokenizers::TruncationDirection::Right,
        }))
        .map_err(internal)?;

    Ok((Arc::new(Mutex::new(session)), Arc::new(tokenizer)))
}

/// Lock the shared session. If a previous holder panicked (poisoning the
/// lock), the session itself is still usable, so recover it: otherwise one
/// panic would turn every later request into a panic too, and with fail-open
/// the stage would silently stop checking anything.
fn lock_session(session: &Mutex<ort::session::Session>) -> MutexGuard<'_, ort::session::Session> {
    session
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Tokenize `text` into windows of at most [`MAX_WINDOW_TOKENS`], run the
/// model on each, and return the highest score `score` gives any window.
///
/// Scoring every window and keeping the maximum means padding a message
/// with filler can't push the harmful part out of view.
pub(crate) fn max_window_score(
    session: &Mutex<ort::session::Session>,
    tokenizer: &tokenizers::Tokenizer,
    text: &str,
    score: fn(&[f32]) -> Result<f32, GuardrailError>,
) -> Result<f32, GuardrailError> {
    let first = tokenizer.encode(text, true).map_err(internal)?;
    let windows = 1 + first.get_overflowing().len();
    if windows > MAX_WINDOWS {
        return Err(internal(format!(
            "input needs {windows} windows of {MAX_WINDOW_TOKENS} tokens; the limit is {MAX_WINDOWS}"
        )));
    }

    let mut best = f32::NEG_INFINITY;
    for encoding in std::iter::once(&first).chain(first.get_overflowing()) {
        best = best.max(score(&run_encoding(session, encoding)?)?);
    }
    Ok(best)
}

/// Run the model on one encoded window and return the first output's values
/// (the logits for a batch of one).
fn run_encoding(
    session: &Mutex<ort::session::Session>,
    encoding: &tokenizers::Encoding,
) -> Result<Vec<f32>, GuardrailError> {
    let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
    let mask: Vec<i64> = encoding
        .get_attention_mask()
        .iter()
        .map(|&m| m as i64)
        .collect();

    let len = ids.len();

    let ids_tensor =
        ort::value::Tensor::from_array((vec![1_i64, len as i64], ids)).map_err(internal)?;
    let mask_tensor =
        ort::value::Tensor::from_array((vec![1_i64, len as i64], mask)).map_err(internal)?;

    let mut s = lock_session(session);
    let outputs = s
        .run(ort::inputs! {
            "input_ids" => ids_tensor,
            "attention_mask" => mask_tensor
        })
        .map_err(internal)?;

    let (_shape, data) = outputs[0].try_extract_tensor::<f32>().map_err(internal)?;
    Ok(data.to_vec())
}

fn softmax_positive(logits: &[f32]) -> f32 {
    // Subtract the max for numerical stability with large logits.
    let max = logits[0].max(logits[1]);
    let e0 = (logits[0] - max).exp();
    let e1 = (logits[1] - max).exp();
    e1 / (e0 + e1)
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Score for a 2-class `[negative, positive]` model: softmax probability of
/// the positive class.
pub(crate) fn two_class_score(logits: &[f32]) -> Result<f32, GuardrailError> {
    if logits.len() != 2 {
        return Err(internal(format!(
            "expected 2 logits [SAFE, INJECTION], model returned {}",
            logits.len()
        )));
    }
    Ok(softmax_positive(logits))
}

/// Toxicity score from either supported output layout:
///
/// - exactly 2 logits: a 2-class `[non-toxic, toxic]` model, scored with
///   softmax over the two;
/// - more than 2 logits: a multi-label model whose label 0 is `toxicity`
///   (as in `unitary/unbiased-toxic-roberta`), scored with a sigmoid on that
///   label alone. Softmax across labels would be wrong here: the labels are
///   independent, not mutually exclusive.
fn toxicity_score(logits: &[f32]) -> Result<f32, GuardrailError> {
    match logits.len() {
        2 => Ok(softmax_positive(logits)),
        n if n > 2 => Ok(sigmoid(logits[0])),
        n => Err(internal(format!(
            "expected 2 logits (2-class) or more (multi-label, label 0 = toxicity), model returned {n}"
        ))),
    }
}

// ── OnnxInjectionClassifier ───────────────────────────────────────────────────

/// DeBERTa-v3-base semantic prompt injection classifier.
///
/// Uses `ProtectAI/deberta-v3-base-prompt-injection-v2` (Apache-2.0) exported
/// to ONNX format. Catches semantic injection attacks that evade regex patterns.
///
/// The model must take `input_ids` and `attention_mask` and return two logits,
/// `[SAFE, INJECTION]`.
///
/// # Feature
///
/// Requires the `onnx` feature flag.
pub struct OnnxInjectionClassifier {
    session: SharedSession,
    tokenizer: Arc<tokenizers::Tokenizer>,
    /// Decision threshold: inputs scoring at or above this are blocked. Default: 0.85.
    threshold: f32,
    /// Log positives instead of blocking them (`action = "log_only"`).
    log_only: bool,
}

impl std::fmt::Debug for OnnxInjectionClassifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxInjectionClassifier")
            .field("threshold", &self.threshold)
            .finish()
    }
}

impl OnnxInjectionClassifier {
    /// Load the classifier from an ONNX model file and a HuggingFace `tokenizer.json`.
    ///
    /// # Errors
    ///
    /// Returns an error if the model file cannot be opened, the ONNX session
    /// cannot be created, or the tokenizer file is invalid.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use guardrail_classifiers::OnnxInjectionClassifier;
    ///
    /// let classifier = OnnxInjectionClassifier::load(
    ///     "models/prompt-injection/model.onnx",
    ///     "models/prompt-injection/tokenizer.json",
    ///     0.85,
    /// ).unwrap();
    /// ```
    pub fn load(
        model_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        threshold: f32,
    ) -> Result<Self, GuardrailError> {
        let (session, tokenizer) = load_session_and_tokenizer(model_path, tokenizer_path)?;
        Ok(Self {
            session,
            tokenizer,
            threshold,
            log_only: false,
        })
    }

    /// When `true`, a score at or above the threshold is logged with
    /// `tracing::warn!` and the request is allowed (dry-run mode).
    #[must_use]
    pub fn with_log_only(mut self, log_only: bool) -> Self {
        self.log_only = log_only;
        self
    }

    /// Run inference on `text` and return the injection probability in
    /// `[0.0, 1.0]`. Blocks the calling thread; [`Stage::evaluate`] runs it
    /// inside `spawn_blocking`.
    ///
    /// # Errors
    ///
    /// Returns an error if tokenization or inference fails, or if the model
    /// doesn't return exactly two logits.
    pub fn score(&self, text: &str) -> Result<f32, GuardrailError> {
        max_window_score(&self.session, &self.tokenizer, text, two_class_score)
    }
}

#[async_trait::async_trait]
impl Stage for OnnxInjectionClassifier {
    fn name(&self) -> &'static str {
        "onnx_injection"
    }

    /// Evaluate the request via DeBERTa inference.
    ///
    /// # Errors
    ///
    /// Returns an error if inference fails. The pipeline then applies this
    /// stage's fail mode (`on_error`).
    async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError> {
        let text = req.user_text();
        if text.trim().is_empty() {
            return Ok(Decision::Allow);
        }

        let threshold = self.threshold;
        let session = self.session.clone();
        let tokenizer = self.tokenizer.clone();

        let score = tokio::task::spawn_blocking(move || {
            max_window_score(&session, &tokenizer, &text, two_class_score)
        })
        .await
        .map_err(internal)??;

        tracing::debug!(
            score,
            threshold,
            stage = "onnx_injection",
            "inference complete"
        );

        if score < threshold {
            Ok(Decision::Allow)
        } else if self.log_only {
            tracing::warn!(
                score,
                threshold,
                stage = "onnx_injection",
                "prompt injection detected (log_only: request allowed)"
            );
            Ok(Decision::Allow)
        } else {
            Ok(Decision::Block {
                reason: format!(
                    "ONNX injection classifier score {score:.3} >= threshold {threshold:.3}"
                ),
                code: BlockCode::PromptInjection,
            })
        }
    }
}

// ── ToxicityClassifier ────────────────────────────────────────────────────────

/// Toxicity classifier backed by `unitary/unbiased-toxic-roberta` (Apache-2.0).
///
/// Detects hate speech, harassment, and self-harm requests. Accepts either a
/// 2-class `[non-toxic, toxic]` model or a multi-label model whose label 0 is
/// `toxicity`; see the [module docs](self) for how each is scored.
///
/// # Feature
///
/// Requires the `onnx` feature flag.
pub struct ToxicityClassifier {
    session: SharedSession,
    tokenizer: Arc<tokenizers::Tokenizer>,
    /// Decision threshold. Default: 0.90.
    threshold: f32,
    /// Log positives instead of blocking them (`action = "log_only"`).
    log_only: bool,
    /// Message roles whose text is scored. Default: user messages only.
    scan_roles: Vec<Role>,
}

impl std::fmt::Debug for ToxicityClassifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToxicityClassifier")
            .field("threshold", &self.threshold)
            .finish()
    }
}

impl ToxicityClassifier {
    /// Load the toxicity classifier from an ONNX model file and a HuggingFace
    /// `tokenizer.json`.
    ///
    /// # Errors
    ///
    /// Returns an error if the model or tokenizer cannot be loaded.
    pub fn load(
        model_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        threshold: f32,
    ) -> Result<Self, GuardrailError> {
        let (session, tokenizer) = load_session_and_tokenizer(model_path, tokenizer_path)?;
        Ok(Self {
            session,
            tokenizer,
            threshold,
            log_only: false,
            scan_roles: vec![Role::User],
        })
    }

    /// When `true`, a score at or above the threshold is logged with
    /// `tracing::warn!` and the request is allowed (dry-run mode).
    #[must_use]
    pub fn with_log_only(mut self, log_only: bool) -> Self {
        self.log_only = log_only;
        self
    }

    /// Score the text of messages with these roles (joined in message
    /// order) instead of only user messages.
    #[must_use]
    pub fn with_scan_roles(mut self, roles: Vec<Role>) -> Self {
        self.scan_roles = roles;
        self
    }

    /// Run inference on `text` and return the toxicity probability in
    /// `[0.0, 1.0]`. Blocks the calling thread.
    ///
    /// # Errors
    ///
    /// Returns an error if tokenization or inference fails, or if the model
    /// returns fewer than two logits.
    pub fn score(&self, text: &str) -> Result<f32, GuardrailError> {
        max_window_score(&self.session, &self.tokenizer, text, toxicity_score)
    }
}

#[async_trait::async_trait]
impl Stage for ToxicityClassifier {
    fn name(&self) -> &'static str {
        "toxicity"
    }

    async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError> {
        let text = req.text_for_roles(&self.scan_roles);
        if text.trim().is_empty() {
            return Ok(Decision::Allow);
        }

        let threshold = self.threshold;
        let session = self.session.clone();
        let tokenizer = self.tokenizer.clone();

        let score = tokio::task::spawn_blocking(move || {
            max_window_score(&session, &tokenizer, &text, toxicity_score)
        })
        .await
        .map_err(internal)??;

        tracing::debug!(score, threshold, stage = "toxicity", "inference complete");

        if score < threshold {
            Ok(Decision::Allow)
        } else if self.log_only {
            tracing::warn!(
                score,
                threshold,
                stage = "toxicity",
                "toxic content detected (log_only: request allowed)"
            );
            Ok(Decision::Allow)
        } else {
            Ok(Decision::Block {
                reason: format!("Toxicity score {score:.3} >= threshold {threshold:.3}"),
                code: BlockCode::Toxicity,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_class_score_rejects_wrong_shape() {
        assert!(two_class_score(&[0.5]).is_err());
        assert!(two_class_score(&[0.0, 1.0, 2.0]).is_err());
        assert!((two_class_score(&[0.0, 0.0]).unwrap() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_softmax_is_stable_for_large_logits() {
        let s = softmax_positive(&[0.0, 1000.0]);
        assert!(s.is_finite() && (s - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_toxicity_score_multilabel_uses_sigmoid_of_label_zero() {
        // Toxic input on a multi-label model: "toxicity" (label 0) high,
        // "severe_toxicity" (label 1) lower. The old 2-class softmax returned
        // P(index 1) = sigmoid(logit1 - logit0) ~ 0 here, so toxic text was
        // allowed. The right score is sigmoid(logit0).
        let mut toxic = vec![-9.0_f32; 16];
        toxic[0] = 5.0;
        toxic[1] = -1.0;
        let s = toxicity_score(&toxic).unwrap();
        assert!((s - sigmoid(5.0)).abs() < 1e-6);
        assert!(s > 0.99);

        let mut clean = vec![-9.0_f32; 16];
        clean[0] = -4.0;
        assert!(toxicity_score(&clean).unwrap() < 0.05);
    }

    #[test]
    fn test_toxicity_score_two_class_uses_softmax() {
        assert!((toxicity_score(&[0.0, 0.0]).unwrap() - 0.5).abs() < 1e-6);
        assert!(toxicity_score(&[1.0]).is_err());
    }
}
