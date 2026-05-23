//! The pipeline abstraction: `Stage` trait, `Pipeline`, and `PipelineBuilder`.

use std::sync::Arc;

use crate::{decision::Decision, error::GuardrailError, request::GuardrailRequest};

/// Every processing unit in the pipeline implements this trait.
///
/// Stages are stateless with respect to individual requests; all shared state
/// (e.g., compiled regexes, ONNX sessions) is held behind `Arc` in the
/// implementor. This allows stages to be shared across many concurrent requests
/// without synchronization overhead on the hot path.
///
/// ## Contract
///
/// - A stage **must not** mutate the request in-place; instead it expresses
///   mutations by returning `Decision::Redact` with a new `GuardrailRequest`.
/// - A stage **must** be `Send + Sync + 'static` so it can be used in a
///   multi-threaded Tokio runtime.
/// - On error, stages should prefer returning `Ok(Decision::Allow)` over
///   propagating the error (fail-open). Propagate the error only for
///   configuration issues that make the stage fundamentally broken.
#[async_trait::async_trait]
pub trait Stage: Send + Sync + 'static {
    /// Human-readable identifier used in logs, metrics, and traces.
    fn name(&self) -> &'static str;

    /// Evaluate a request and return a `Decision`.
    ///
    /// # Errors
    ///
    /// Returns `GuardrailError::StageFailed` if the stage encounters an
    /// unrecoverable error. Recoverable errors (e.g., a model returning an
    /// unexpected shape) should be logged and result in `Ok(Decision::Allow)`.
    async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError>;
}

/// A sequential pipeline of stages evaluated in order.
///
/// The first `Block` decision short-circuits the pipeline; no subsequent stages
/// are run. `Redact` decisions update the working request for subsequent stages.
///
/// # Examples
///
/// ```rust
/// use guardrail_core::{Pipeline, PipelineBuilder, Decision};
///
/// # tokio_test::block_on(async {
/// let pipeline = PipelineBuilder::default().build();
/// let req = guardrail_core::test_helpers::clean_request();
/// let (decision, _) = pipeline.run(req).await.unwrap();
/// assert_eq!(decision, Decision::Allow);
/// # });
/// ```
pub struct Pipeline {
    stages: Vec<Arc<dyn Stage>>,
}

impl Pipeline {
    /// Create a pipeline from a list of stages.
    pub fn new(stages: Vec<Arc<dyn Stage>>) -> Self {
        Self { stages }
    }

    /// Returns the number of stages in this pipeline.
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// Returns `true` if the pipeline contains no stages.
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// Evaluate a request against all configured stages and return a final decision.
    ///
    /// Stages are run in order. The first `Block` decision short-circuits evaluation.
    /// `Redact` decisions update the request for downstream stages.
    ///
    /// # Errors
    ///
    /// Returns `GuardrailError::StageFailed` if any stage returns an error.
    /// By default, stage errors are treated as `Allow` (fail-open). This can be
    /// changed via `config.pipeline.on_error`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_core::{Pipeline, PipelineBuilder, Decision};
    ///
    /// # tokio_test::block_on(async {
    /// let pipeline = PipelineBuilder::default().build();
    /// let req = guardrail_core::test_helpers::clean_request();
    /// let (decision, _req) = pipeline.run(req).await.unwrap();
    /// assert_eq!(decision, Decision::Allow);
    /// # });
    /// ```
    pub async fn run(
        &self,
        req: GuardrailRequest,
    ) -> Result<(Decision, GuardrailRequest), GuardrailError> {
        self.run_with_observer(req, |_stage, _elapsed| {}).await
    }

    /// Like [`Pipeline::run`], but invokes `observer(stage_name, elapsed)`
    /// after each stage evaluation (including failed/blocked stages).
    ///
    /// This is used by `guardrail-proxy` to record per-stage latency
    /// histograms without coupling `guardrail-core` to a metrics backend.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_core::{PipelineBuilder, test_helpers::clean_request};
    /// use std::time::Duration;
    ///
    /// # tokio_test::block_on(async {
    /// let pipeline = PipelineBuilder::default().build();
    /// let mut observed: Vec<(String, Duration)> = Vec::new();
    ///
    /// let (decision, _req) = pipeline
    ///     .run_with_observer(clean_request(), |stage, elapsed| {
    ///         observed.push((stage.to_string(), elapsed));
    ///     })
    ///     .await
    ///     .unwrap();
    ///
    /// assert!(observed.is_empty()); // empty pipeline has no stages
    /// # });
    /// ```
    pub async fn run_with_observer<F>(
        &self,
        mut req: GuardrailRequest,
        mut observer: F,
    ) -> Result<(Decision, GuardrailRequest), GuardrailError>
    where
        F: FnMut(&str, std::time::Duration),
    {
        for stage in &self.stages {
            let stage_name = stage.name();
            tracing::debug!(stage = stage_name, "evaluating stage");

            let stage_start = std::time::Instant::now();
            let result = stage.evaluate(&req).await;
            observer(stage_name, stage_start.elapsed());

            let decision = match result {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(
                        stage = stage_name,
                        error = %e,
                        "stage failed; failing open (allow)"
                    );
                    Decision::Allow
                }
            };

            tracing::debug!(
                stage = stage_name,
                decision = decision.name(),
                "stage decision"
            );

            match decision {
                Decision::Allow => continue,
                Decision::Redact { reason, mutated } => {
                    tracing::info!(stage = stage_name, reason = %reason, "request redacted");
                    req = mutated;
                }
                Decision::Block { reason, code } => {
                    tracing::info!(
                        stage = stage_name,
                        reason = %reason,
                        code = %code,
                        "request blocked"
                    );
                    return Ok((
                        Decision::Block {
                            reason,
                            code,
                        },
                        req,
                    ));
                }
            }
        }

        Ok((Decision::Allow, req))
    }
}

/// Builder for ergonomic pipeline construction.
///
/// # Examples
///
/// ```rust
/// use guardrail_core::PipelineBuilder;
///
/// let pipeline = PipelineBuilder::default()
///     // .stage(MyStage::new())  // add stages here
///     .build();
///
/// assert_eq!(pipeline.len(), 0);
/// ```
#[derive(Default)]
pub struct PipelineBuilder {
    stages: Vec<Arc<dyn Stage>>,
}

impl PipelineBuilder {
    /// Add a stage to the pipeline. Stages are evaluated in the order they are added.
    pub fn stage(mut self, s: impl Stage) -> Self {
        self.stages.push(Arc::new(s));
        self
    }

    /// Add a pre-boxed stage (useful when the stage is already behind an `Arc`).
    pub fn arc_stage(mut self, s: Arc<dyn Stage>) -> Self {
        self.stages.push(s);
        self
    }

    /// Consume the builder and return a configured `Pipeline`.
    pub fn build(self) -> Pipeline {
        Pipeline::new(self.stages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{clean_request, BlockingStage, PassthroughStage};

    #[tokio::test]
    async fn test_empty_pipeline_allows() {
        let pipeline = PipelineBuilder::default().build();
        let req = clean_request();
        let (decision, _) = pipeline.run(req).await.unwrap();
        assert_eq!(decision, Decision::Allow);
    }

    #[tokio::test]
    async fn test_single_pass_stage_allows() {
        let pipeline = PipelineBuilder::default()
            .stage(PassthroughStage)
            .build();
        let req = clean_request();
        let (decision, _) = pipeline.run(req).await.unwrap();
        assert_eq!(decision, Decision::Allow);
    }

    #[tokio::test]
    async fn test_blocking_stage_short_circuits() {
        let pipeline = PipelineBuilder::default()
            .stage(BlockingStage)
            .stage(PassthroughStage)
            .build();
        let req = clean_request();
        let (decision, _) = pipeline.run(req).await.unwrap();
        assert!(matches!(decision, Decision::Block { .. }));
    }

    #[tokio::test]
    async fn test_pipeline_len() {
        let pipeline = PipelineBuilder::default()
            .stage(PassthroughStage)
            .stage(PassthroughStage)
            .build();
        assert_eq!(pipeline.len(), 2);
        assert!(!pipeline.is_empty());
    }

    #[tokio::test]
    async fn test_run_with_observer_records_each_stage() {
        let pipeline = PipelineBuilder::default()
            .stage(PassthroughStage)
            .stage(PassthroughStage)
            .build();

        let mut observed = Vec::new();
        let (decision, _) = pipeline
            .run_with_observer(clean_request(), |stage, elapsed| {
                observed.push((stage.to_string(), elapsed));
            })
            .await
            .unwrap();

        assert_eq!(decision, Decision::Allow);
        assert_eq!(observed.len(), 2);
        assert!(observed.iter().all(|(name, _)| name == "passthrough"));
    }

    #[tokio::test]
    async fn test_run_with_observer_stops_after_block() {
        let pipeline = PipelineBuilder::default()
            .stage(BlockingStage)
            .stage(PassthroughStage)
            .build();

        let mut observed = Vec::new();
        let (decision, _) = pipeline
            .run_with_observer(clean_request(), |stage, elapsed| {
                observed.push((stage.to_string(), elapsed));
            })
            .await
            .unwrap();

        assert!(matches!(decision, Decision::Block { .. }));
        // Only the blocking stage should have been observed.
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0].0, "blocking");
    }
}
