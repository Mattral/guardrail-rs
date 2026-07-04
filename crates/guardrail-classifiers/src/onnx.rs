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

#![cfg(feature = "onnx")]

use guardrail_core::{
    decision::{BlockCode, Decision},
    error::GuardrailError,
    pipeline::Stage,
    request::GuardrailRequest,
};
use std::path::Path;
use std::sync::Arc;

// ── OnnxInjectionClassifier ───────────────────────────────────────────────────

/// DeBERTa-v3-base semantic prompt injection classifier.
///
/// Uses `ProtectAI/deberta-v3-base-prompt-injection-v2` (Apache-2.0) exported
/// to ONNX format. Catches semantic injection attacks that evade regex patterns.
///
/// **Performance target:** < 5 ms on CPU for inputs up to 512 tokens.
///
/// # Feature
///
/// Requires the `onnx` feature flag.
pub struct OnnxInjectionClassifier {
    session: Arc<std::sync::Mutex<ort::session::Session>>,
    tokenizer: Arc<tokenizers::Tokenizer>,
    /// Decision threshold: inputs scoring above this are blocked. Default: 0.85.
    threshold: f32,
}

impl std::fmt::Debug for OnnxInjectionClassifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxInjectionClassifier")
            .field("threshold", &self.threshold)
            .finish()
    }
}

impl OnnxInjectionClassifier {
    /// Load the classifier from an ONNX model file and a HuggingFace tokenizer directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the model file cannot be opened, the ONNX session
    /// cannot be created, or the tokenizer directory is invalid.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use guardrail_classifiers::OnnxInjectionClassifier;
    ///
    /// # tokio_test::block_on(async {
    /// let classifier = OnnxInjectionClassifier::load(
    ///     "models/prompt-injection-v1.onnx",
    ///     "models/deberta-tokenizer/",
    ///     0.85,
    /// ).unwrap();
    /// # });
    /// ```
    pub fn load(
        model_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        threshold: f32,
    ) -> Result<Self, GuardrailError> {
        // fully-qualify ort types below; no local import needed
        let session = ort::session::Session::builder()
            .map_err(|e| GuardrailError::Internal(e.to_string()))?
            .with_intra_threads(1)
            .map_err(|e| GuardrailError::Internal(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        Ok(Self {
            session: Arc::new(std::sync::Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
            threshold,
        })
    }

    /// Run inference synchronously.
    ///
    /// Called inside `spawn_blocking` by [`Stage::evaluate`].
    fn infer_sync(&self, text: &str) -> Result<f32, GuardrailError> {
        // reference ort::inputs! macro fully-qualified below

        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();

        let len = ids.len();

        let ids_tensor = ort::value::Tensor::from_array((vec![1_i64, len as i64], ids))
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;
        let mask_tensor = ort::value::Tensor::from_array((vec![1_i64, len as i64], mask))
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        let mut s = self.session.lock().expect("session mutex poisoned");
        let outputs = s
            .run(ort::inputs! {
                "input_ids" => ids_tensor,
                "attention_mask" => mask_tensor
            })
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        // Output shape: [1, 2] — logits for [SAFE, INJECTION]
        let (_shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        // Softmax over the two classes
        let exp0 = data[0].exp();
        let exp1 = data[1].exp();
        let injection_score = exp1 / (exp0 + exp1);

        Ok(injection_score)
    }
}

#[async_trait::async_trait]
impl Stage for OnnxInjectionClassifier {
    fn name(&self) -> &'static str {
        "onnx_injection"
    }

    /// Evaluate the request via DeBERTa inference.
    ///
    /// Runs inference inside `tokio::task::spawn_blocking` to avoid blocking
    /// the async executor.
    ///
    /// # Errors
    ///
    /// Returns an error only for internal ONNX failures. On such failures the
    /// pipeline fails open (allow).
    async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError> {
        let text = req.user_text();
        let threshold = self.threshold;
        let session = self.session.clone();
        let tokenizer = self.tokenizer.clone();

        let classifier = OnnxInjectionClassifier {
            session,
            tokenizer,
            threshold,
        };

        let score = tokio::task::spawn_blocking(move || classifier.infer_sync(&text))
            .await
            .map_err(|e| GuardrailError::Internal(e.to_string()))??;

        tracing::debug!(
            score,
            threshold,
            stage = "onnx_injection",
            "inference complete"
        );

        if score >= threshold {
            Ok(Decision::Block {
                reason: format!(
                    "ONNX injection classifier score {score:.3} >= threshold {threshold:.3}"
                ),
                code: BlockCode::PromptInjection,
            })
        } else {
            Ok(Decision::Allow)
        }
    }
}

// ── ToxicityClassifier ────────────────────────────────────────────────────────

/// Toxicity classifier backed by `unitary/unbiased-toxic-roberta` (Apache-2.0).
///
/// Detects hate speech, harassment, and self-harm requests. By default, only
/// user-role messages are scanned; system prompts are exempt.
///
/// **Performance target:** < 5 ms on CPU for inputs up to 512 tokens.
///
/// # Feature
///
/// Requires the `onnx` feature flag.
pub struct ToxicityClassifier {
    session: Arc<std::sync::Mutex<ort::session::Session>>,
    tokenizer: Arc<tokenizers::Tokenizer>,
    /// Decision threshold. Default: 0.90.
    threshold: f32,
}

impl std::fmt::Debug for ToxicityClassifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToxicityClassifier")
            .field("threshold", &self.threshold)
            .finish()
    }
}

impl ToxicityClassifier {
    /// Load the toxicity classifier from an ONNX model file.
    ///
    /// # Errors
    ///
    /// Returns an error if the model or tokenizer cannot be loaded.
    pub fn load(
        model_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        threshold: f32,
    ) -> Result<Self, GuardrailError> {
        // fully-qualify ort types below; no local import needed
        let session = ort::session::Session::builder()
            .map_err(|e| GuardrailError::Internal(e.to_string()))?
            .with_intra_threads(1)
            .map_err(|e| GuardrailError::Internal(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        Ok(Self {
            session: Arc::new(std::sync::Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
            threshold,
        })
    }

    fn infer_sync(&self, text: &str) -> Result<f32, GuardrailError> {
        // reference ort::inputs! macro fully-qualified below

        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();

        let len = ids.len();

        let ids_tensor = ort::value::Tensor::from_array((vec![1_i64, len as i64], ids))
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;
        let mask_tensor = ort::value::Tensor::from_array((vec![1_i64, len as i64], mask))
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        let mut s = self.session.lock().expect("session mutex poisoned");
        let outputs = s
            .run(ort::inputs! {
                "input_ids" => ids_tensor,
                "attention_mask" => mask_tensor
            })
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        let (_shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| GuardrailError::Internal(e.to_string()))?;

        let exp0 = data[0].exp();
        let exp1 = data[1].exp();
        Ok(exp1 / (exp0 + exp1))
    }
}

#[async_trait::async_trait]
impl Stage for ToxicityClassifier {
    fn name(&self) -> &'static str {
        "toxicity"
    }

    async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError> {
        let text = req.user_text();
        if text.trim().is_empty() {
            return Ok(Decision::Allow);
        }

        let threshold = self.threshold;
        let session = self.session.clone();
        let tokenizer = self.tokenizer.clone();

        let classifier = ToxicityClassifier {
            session,
            tokenizer,
            threshold,
        };

        let score = tokio::task::spawn_blocking(move || classifier.infer_sync(&text))
            .await
            .map_err(|e| GuardrailError::Internal(e.to_string()))??;

        tracing::debug!(score, threshold, stage = "toxicity", "inference complete");

        if score >= threshold {
            Ok(Decision::Block {
                reason: format!("Toxicity score {score:.3} >= threshold {threshold:.3}"),
                code: BlockCode::Toxicity,
            })
        } else {
            Ok(Decision::Allow)
        }
    }
}
