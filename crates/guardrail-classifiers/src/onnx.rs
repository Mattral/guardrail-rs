//! ONNX-backed classifier implementations.
//!
//! Enabled by the `onnx` feature flag. Provides:
//! - [`OnnxInjectionClassifier`]: DeBERTa-v3-base fine-tuned on prompt injection
//! - [`ToxicityClassifier`]: unbiased-toxic-roberta for hate speech / harm detection
//!
//! Each classifier loads a pool of identical ONNX Runtime sessions once at
//! startup ([`OnnxRuntimeOptions`]). A session runs one inference at a time,
//! so the pool size is how many requests a stage scores in parallel. A
//! request first waits (asynchronously) for a free slot, then runs in
//! `tokio::task::spawn_blocking` with a session to itself, so waiting
//! requests never hold a blocking thread.
//!
//! An inference failure (including a model whose output doesn't have the
//! expected shape) is returned as `Err`, never a panic, so the pipeline can
//! apply the stage's configured fail mode (`on_error`).

#![cfg(feature = "onnx")]

use guardrail_core::{
    decision::{BlockCode, Decision},
    error::GuardrailError,
    pipeline::{InputTooLong, OversizeAction, Stage},
    request::{GuardrailRequest, Role},
};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};

/// Longest token sequence sent to the model in one inference, special tokens
/// included. Both recommended models accept at most 512 positions (RoBERTa
/// fails outright beyond that; DeBERTa runs, but its cost grows
/// quadratically and it was trained on ≤512).
pub const MAX_WINDOW_TOKENS: usize = 512;

/// Tokens shared by consecutive windows, so a phrase that straddles a
/// window boundary is still seen whole by one of them.
pub const WINDOW_STRIDE_TOKENS: usize = 128;

/// Default for the most windows scored for one request (about 6k tokens).
/// Longer input is not scored at all: at the default 10 MiB body limit an
/// attacker could otherwise force thousands of model runs per request.
/// What happens to it instead is the stage's [`OversizeAction`] (blocked by
/// default). Change both with `with_max_windows` / `with_on_oversize`.
pub const MAX_WINDOWS: usize = 16;

/// How much input a stage scores, and what happens to input past that.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowLimit {
    pub(crate) max_windows: usize,
    pub(crate) on_oversize: OversizeAction,
}

impl Default for WindowLimit {
    fn default() -> Self {
        Self {
            max_windows: MAX_WINDOWS,
            on_oversize: OversizeAction::Block,
        }
    }
}

/// How a classifier runs its model.
///
/// # Examples
///
/// ```rust
/// use guardrail_classifiers::onnx::OnnxRuntimeOptions;
///
/// let options = OnnxRuntimeOptions { sessions: 2, intra_threads: 1 };
/// assert_eq!(options.normalized().sessions, 2);
/// assert!(OnnxRuntimeOptions::default().sessions >= 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnnxRuntimeOptions {
    /// Sessions in the pool: how many requests are scored in parallel.
    /// Each session holds its own copy of the model in memory. `0` means
    /// the default, [`default_sessions`].
    pub sessions: usize,
    /// Threads ONNX Runtime uses inside one inference. `0` is treated as 1.
    pub intra_threads: usize,
}

impl OnnxRuntimeOptions {
    /// Replace `0`s with the defaults.
    pub fn normalized(self) -> Self {
        let intra_threads = self.intra_threads.max(1);
        let sessions = match self.sessions {
            0 => default_sessions(intra_threads),
            n => n,
        };
        Self {
            sessions,
            intra_threads,
        }
    }
}

impl Default for OnnxRuntimeOptions {
    fn default() -> Self {
        Self {
            sessions: 0,
            intra_threads: 1,
        }
        .normalized()
    }
}

/// Most sessions the automatic default picks. Each session holds its own
/// copy of the model (about 0.85 GB for the DeBERTa injection model), so one
/// per core would run many-core machines out of memory. Set `sessions`
/// explicitly to go higher.
pub const MAX_DEFAULT_SESSIONS: usize = 4;

/// Default pool size: one session per available core (divided by
/// `intra_threads`), at least 1 and at most [`MAX_DEFAULT_SESSIONS`].
pub fn default_sessions(intra_threads: usize) -> usize {
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    (cores / intra_threads.max(1)).clamp(1, MAX_DEFAULT_SESSIONS)
}

fn internal(e: impl std::fmt::Display) -> GuardrailError {
    GuardrailError::Internal(e.to_string())
}

/// Identical ONNX Runtime sessions for one stage, each used by one
/// inference at a time.
pub(crate) struct SessionPool {
    idle: Mutex<Vec<ort::session::Session>>,
    returned: Condvar,
    /// One permit per session. Taken (asynchronously) before a request
    /// moves to a blocking thread, so requests queue here rather than on
    /// blocked threads.
    permits: Arc<tokio::sync::Semaphore>,
    size: usize,
}

impl SessionPool {
    fn load(model_path: &Path, options: OnnxRuntimeOptions) -> Result<Arc<Self>, GuardrailError> {
        let options = options.normalized();
        let sessions = (0..options.sessions)
            .map(|_| {
                ort::session::Session::builder()
                    .map_err(internal)?
                    .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
                    .map_err(internal)?
                    .with_intra_threads(options.intra_threads)
                    .map_err(internal)?
                    .commit_from_file(model_path)
                    .map_err(internal)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Arc::new(Self {
            idle: Mutex::new(sessions),
            returned: Condvar::new(),
            permits: Arc::new(tokio::sync::Semaphore::new(options.sessions)),
            size: options.sessions,
        }))
    }

    /// Wait for a free slot. Hold the permit while using a session from
    /// [`SessionPool::checkout`].
    async fn reserve(&self) -> tokio::sync::OwnedSemaphorePermit {
        self.permits
            .clone()
            .acquire_owned()
            .await
            .expect("the pool's semaphore is never closed")
    }

    /// Take an idle session, blocking until one is returned. With a permit
    /// from [`SessionPool::reserve`] in hand, one is always idle.
    fn checkout(&self) -> PooledSession<'_> {
        // A panic while holding the lock can't leave the Vec inconsistent,
        // so recover from poisoning instead of failing every later request.
        let mut idle = self.idle.lock().unwrap_or_else(|p| p.into_inner());
        loop {
            if let Some(session) = idle.pop() {
                return PooledSession {
                    pool: self,
                    session: Some(session),
                };
            }
            idle = self.returned.wait(idle).unwrap_or_else(|p| p.into_inner());
        }
    }
}

/// A session checked out of a [`SessionPool`]; returned to it on drop,
/// including when inference panics.
pub(crate) struct PooledSession<'a> {
    pool: &'a SessionPool,
    session: Option<ort::session::Session>,
}

impl PooledSession<'_> {
    fn get(&mut self) -> &mut ort::session::Session {
        self.session.as_mut().expect("present until drop")
    }
}

impl Drop for PooledSession<'_> {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            let mut idle = self.pool.idle.lock().unwrap_or_else(|p| p.into_inner());
            idle.push(session);
            self.pool.returned.notify_one();
        }
    }
}

fn load_tokenizer(tokenizer_path: &Path) -> Result<Arc<tokenizers::Tokenizer>, GuardrailError> {
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
    Ok(Arc::new(tokenizer))
}

/// Run one request's scoring on a session from `pool`, in a blocking
/// thread, once a slot is free.
async fn score_in_pool(
    stage: &'static str,
    pool: &Arc<SessionPool>,
    tokenizer: &Arc<tokenizers::Tokenizer>,
    text: String,
    limit: WindowLimit,
    score: fn(&[f32]) -> Result<f32, GuardrailError>,
) -> Result<f32, GuardrailError> {
    let permit = pool.reserve().await;
    let pool = pool.clone();
    let tokenizer = tokenizer.clone();
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let mut session = pool.checkout();
        max_window_score(stage, session.get(), &tokenizer, &text, limit, score)
    })
    .await
    .map_err(internal)?
}

/// Tokenize `text` into windows of at most [`MAX_WINDOW_TOKENS`], run the
/// model on each, and return the highest score `score` gives any window.
///
/// Scoring every window and keeping the maximum means padding a message
/// with filler can't push the harmful part out of view. Input needing more
/// than `limit.max_windows` windows is not scored: it returns
/// [`InputTooLong`], which the pipeline resolves with `limit.on_oversize`.
pub(crate) fn max_window_score(
    stage: &'static str,
    session: &mut ort::session::Session,
    tokenizer: &tokenizers::Tokenizer,
    text: &str,
    limit: WindowLimit,
    score: fn(&[f32]) -> Result<f32, GuardrailError>,
) -> Result<f32, GuardrailError> {
    let first = tokenizer.encode(text, true).map_err(internal)?;
    let windows = 1 + first.get_overflowing().len();
    if windows > limit.max_windows {
        return Err(InputTooLong {
            size: windows,
            limit: limit.max_windows,
            action: limit.on_oversize,
        }
        .into_error(stage));
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
    session: &mut ort::session::Session,
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

    let outputs = session
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
    pool: Arc<SessionPool>,
    tokenizer: Arc<tokenizers::Tokenizer>,
    /// Decision threshold: inputs scoring at or above this are blocked. Default: 0.85.
    threshold: f32,
    /// Log positives instead of blocking them (`action = "log_only"`).
    log_only: bool,
    /// Most input scored, and what happens to longer input.
    limit: WindowLimit,
}

impl std::fmt::Debug for OnnxInjectionClassifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxInjectionClassifier")
            .field("threshold", &self.threshold)
            .field("sessions", &self.pool.size)
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
        Self::load_with_options(
            model_path,
            tokenizer_path,
            threshold,
            OnnxRuntimeOptions::default(),
        )
    }

    /// Like [`OnnxInjectionClassifier::load`], with an explicit session pool size and thread
    /// count.
    ///
    /// # Errors
    ///
    /// As for [`OnnxInjectionClassifier::load`].
    pub fn load_with_options(
        model_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        threshold: f32,
        options: OnnxRuntimeOptions,
    ) -> Result<Self, GuardrailError> {
        Ok(Self {
            pool: SessionPool::load(model_path.as_ref(), options)?,
            tokenizer: load_tokenizer(tokenizer_path.as_ref())?,
            threshold,
            log_only: false,
            limit: WindowLimit::default(),
        })
    }

    /// Number of sessions in the pool.
    pub fn sessions(&self) -> usize {
        self.pool.size
    }

    /// When `true`, a score at or above the threshold is logged with
    /// `tracing::warn!` and the request is allowed (dry-run mode).
    #[must_use]
    pub fn with_log_only(mut self, log_only: bool) -> Self {
        self.log_only = log_only;
        self
    }

    /// Score at most `max_windows` windows of [`MAX_WINDOW_TOKENS`] tokens
    /// per request (default [`MAX_WINDOWS`]). Values below 1 are treated
    /// as 1.
    #[must_use]
    pub fn with_max_windows(mut self, max_windows: usize) -> Self {
        self.limit.max_windows = max_windows.max(1);
        self
    }

    /// What to do with input longer than `max_windows` windows: block it
    /// (the default) or let it through unchecked.
    #[must_use]
    pub fn with_on_oversize(mut self, on_oversize: OversizeAction) -> Self {
        self.limit.on_oversize = on_oversize;
        self
    }

    /// Run inference on `text` and return the injection probability in
    /// `[0.0, 1.0]`. Blocks the calling thread; [`Stage::evaluate`] runs it
    /// inside `spawn_blocking`.
    ///
    /// # Errors
    ///
    /// Returns an error if tokenization or inference fails, if the model
    /// doesn't return exactly two logits, or ([`InputTooLong`]) if `text`
    /// needs more windows than the limit.
    pub fn score(&self, text: &str) -> Result<f32, GuardrailError> {
        let mut session = self.pool.checkout();
        max_window_score(
            "onnx_injection",
            session.get(),
            &self.tokenizer,
            text,
            self.limit,
            two_class_score,
        )
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
    /// Returns an error if inference fails; the pipeline then applies this
    /// stage's fail mode (`on_error`). Input over the window limit returns
    /// [`InputTooLong`] instead, which the pipeline resolves with the
    /// stage's `on_oversize`.
    async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError> {
        // User messages and tool results: tool output is the main route for
        // indirect injection.
        let text = req.text_for_roles(&[Role::User, Role::Tool]);
        if text.trim().is_empty() {
            return Ok(Decision::Allow);
        }

        let threshold = self.threshold;
        let score = score_in_pool(
            "onnx_injection",
            &self.pool,
            &self.tokenizer,
            text,
            self.limit,
            two_class_score,
        )
        .await?;

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
    pool: Arc<SessionPool>,
    tokenizer: Arc<tokenizers::Tokenizer>,
    /// Decision threshold. Default: 0.90.
    threshold: f32,
    /// Log positives instead of blocking them (`action = "log_only"`).
    log_only: bool,
    /// Message roles whose text is scored. Default: user messages only.
    scan_roles: Vec<Role>,
    /// Most input scored, and what happens to longer input.
    limit: WindowLimit,
}

impl std::fmt::Debug for ToxicityClassifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToxicityClassifier")
            .field("threshold", &self.threshold)
            .field("sessions", &self.pool.size)
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
        Self::load_with_options(
            model_path,
            tokenizer_path,
            threshold,
            OnnxRuntimeOptions::default(),
        )
    }

    /// Like [`ToxicityClassifier::load`], with an explicit session pool size and thread
    /// count.
    ///
    /// # Errors
    ///
    /// As for [`ToxicityClassifier::load`].
    pub fn load_with_options(
        model_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        threshold: f32,
        options: OnnxRuntimeOptions,
    ) -> Result<Self, GuardrailError> {
        Ok(Self {
            pool: SessionPool::load(model_path.as_ref(), options)?,
            tokenizer: load_tokenizer(tokenizer_path.as_ref())?,
            threshold,
            log_only: false,
            scan_roles: vec![Role::User],
            limit: WindowLimit::default(),
        })
    }

    /// Number of sessions in the pool.
    pub fn sessions(&self) -> usize {
        self.pool.size
    }

    /// When `true`, a score at or above the threshold is logged with
    /// `tracing::warn!` and the request is allowed (dry-run mode).
    #[must_use]
    pub fn with_log_only(mut self, log_only: bool) -> Self {
        self.log_only = log_only;
        self
    }

    /// Score at most `max_windows` windows of [`MAX_WINDOW_TOKENS`] tokens
    /// per request (default [`MAX_WINDOWS`]). Values below 1 are treated
    /// as 1.
    #[must_use]
    pub fn with_max_windows(mut self, max_windows: usize) -> Self {
        self.limit.max_windows = max_windows.max(1);
        self
    }

    /// What to do with input longer than `max_windows` windows: block it
    /// (the default) or let it through unchecked.
    #[must_use]
    pub fn with_on_oversize(mut self, on_oversize: OversizeAction) -> Self {
        self.limit.on_oversize = on_oversize;
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
    /// Returns an error if tokenization or inference fails, if the model
    /// returns fewer than two logits, or ([`InputTooLong`]) if `text` needs
    /// more windows than the limit.
    pub fn score(&self, text: &str) -> Result<f32, GuardrailError> {
        let mut session = self.pool.checkout();
        max_window_score(
            "toxicity",
            session.get(),
            &self.tokenizer,
            text,
            self.limit,
            toxicity_score,
        )
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
        let score = score_in_pool(
            "toxicity",
            &self.pool,
            &self.tokenizer,
            text,
            self.limit,
            toxicity_score,
        )
        .await?;

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

    fn fixture_pool(sessions: usize) -> Arc<SessionPool> {
        let model = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/onnx/keyword_2class.onnx");
        SessionPool::load(
            &model,
            OnnxRuntimeOptions {
                sessions,
                intra_threads: 1,
            },
        )
        .unwrap()
    }

    #[test]
    fn test_pool_hands_out_each_session_once() {
        let pool = fixture_pool(2);
        let a = pool.checkout();
        let b = pool.checkout();
        assert!(pool.idle.lock().unwrap().is_empty());

        // A third checkout waits until a session comes back.
        let (tx, rx) = std::sync::mpsc::channel();
        let waiter = {
            let pool = pool.clone();
            std::thread::spawn(move || {
                let _c = pool.checkout();
                tx.send(()).unwrap();
            })
        };
        assert!(rx
            .recv_timeout(std::time::Duration::from_millis(100))
            .is_err());
        drop(a);
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        waiter.join().unwrap();
        drop(b);
        assert_eq!(pool.idle.lock().unwrap().len(), 2);
    }

    #[test]
    fn test_session_is_returned_after_a_panic() {
        let pool = fixture_pool(1);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _session = pool.checkout();
            panic!("inference blew up");
        }));
        assert!(result.is_err());
        assert_eq!(pool.idle.lock().unwrap_or_else(|p| p.into_inner()).len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_permits_bound_concurrent_inference() {
        let pool = fixture_pool(2);
        let p1 = pool.reserve().await;
        let _p2 = pool.reserve().await;
        // Both permits are taken: a third reservation has to wait.
        let pending = tokio::time::timeout(std::time::Duration::from_millis(50), pool.reserve());
        assert!(pending.await.is_err());
        drop(p1);
        let _p3 = pool.reserve().await;
    }

    #[test]
    fn test_runtime_options_defaults() {
        let options = OnnxRuntimeOptions {
            sessions: 0,
            intra_threads: 0,
        }
        .normalized();
        assert_eq!(options.intra_threads, 1);
        assert_eq!(options.sessions, default_sessions(1));
        assert!(default_sessions(1) >= 1);
        assert_eq!(default_sessions(usize::MAX), 1);
    }

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
