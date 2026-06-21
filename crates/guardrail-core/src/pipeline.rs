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

    /// Start building a pipeline fluently.
    ///
    /// Equivalent to [`PipelineBuilder::default()`]; provided as a
    /// convenience entry point so call sites can write `Pipeline::builder()`
    /// without importing `PipelineBuilder` directly.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use guardrail_core::Pipeline;
    /// use guardrail_core::test_helpers::PassthroughStage;
    ///
    /// let pipeline = Pipeline::builder()
    ///     .stage(PassthroughStage)
    ///     .build();
    /// assert_eq!(pipeline.len(), 1);
    /// ```
    pub fn builder() -> PipelineBuilder {
        PipelineBuilder::default()
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
    /// Run every stage in order against `req`, short-circuiting on the
    /// first `Decision::Block`.
    ///
    /// If one or more stages return `Decision::Redact`, evaluation
    /// continues (subsequent stages see the mutated request), and the
    /// **final returned decision is `Decision::Redact`** — not `Allow` —
    /// with `reason` being every redacting stage's reason joined by `"; "`
    /// and `entities` being the de-duplicated union of every redacting
    /// stage's entity list. Only if no stage redacted *and* no stage
    /// blocked does the pipeline return `Decision::Allow`.
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
    ///
    /// A pipeline with a redacting stage returns `Decision::Redact` as its
    /// **final** decision, not `Allow` — this is the behavior callers such
    /// as `guardrail-proxy`'s audit log and `redacted_total` metric depend
    /// on:
    ///
    /// ```rust
    /// use guardrail_core::{Pipeline, Decision};
    /// # use guardrail_core::{Stage, error::GuardrailError, request::GuardrailRequest};
    /// # use async_trait::async_trait;
    /// #
    /// # struct AlwaysRedacts;
    /// # #[async_trait]
    /// # impl Stage for AlwaysRedacts {
    /// #     fn name(&self) -> &'static str { "always_redacts" }
    /// #     async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError> {
    /// #         Ok(Decision::Redact {
    /// #             reason: "test redaction".into(),
    /// #             mutated: req.clone(),
    /// #             entities: vec!["email".into()],
    /// #         })
    /// #     }
    /// # }
    /// # tokio_test::block_on(async {
    /// let pipeline = Pipeline::builder().stage(AlwaysRedacts).build();
    /// let req = guardrail_core::test_helpers::clean_request();
    /// let (decision, _req) = pipeline.run(req).await.unwrap();
    /// match decision {
    ///     Decision::Redact { entities, .. } => assert_eq!(entities, vec!["email"]),
    ///     other => panic!("expected Redact, got {other:?}"),
    /// }
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
        // Accumulated across all stages that redact (not just the first or
        // last). A pipeline with both `pii_redactor` and a custom
        // redacting stage must report the union of both as the final
        // decision — previously this state was discarded entirely and the
        // pipeline always returned `Decision::Allow` even after a
        // successful redaction, which meant `Decision::Redact` could never
        // actually reach callers (the audit log's "redact" case, the
        // `guardrail_redacted_total` metric, and `guardrail check`'s
        // redact output were all unreachable dead code).
        let mut redaction_reasons: Vec<String> = Vec::new();
        let mut redaction_entities: Vec<String> = Vec::new();

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
                Decision::Redact { reason, mutated, entities } => {
                    tracing::info!(stage = stage_name, reason = %reason, "request redacted");
                    req = mutated;
                    redaction_reasons.push(reason);
                    for entity in entities {
                        if !redaction_entities.contains(&entity) {
                            redaction_entities.push(entity);
                        }
                    }
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

        if redaction_reasons.is_empty() {
            Ok((Decision::Allow, req))
        } else {
            Ok((
                Decision::Redact {
                    reason: redaction_reasons.join("; "),
                    mutated: req.clone(),
                    entities: redaction_entities,
                },
                req,
            ))
        }
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
    async fn test_single_redacting_stage_returns_redact_not_allow() {
        // Regression test: the pipeline previously always collapsed a
        // successful redaction down to `Decision::Allow` by the time
        // `run()` returned, even though the stage itself correctly
        // returned `Decision::Redact`. This made `Decision::Redact`
        // unreachable from any caller of `Pipeline::run`.
        use crate::test_helpers::RedactingStage;

        let pipeline = PipelineBuilder::default()
            .stage(RedactingStage::new(
                "PII detected and redacted: Email",
                vec!["email".to_string()],
            ))
            .build();

        let req = clean_request();
        let (decision, _) = pipeline.run(req).await.unwrap();

        match decision {
            Decision::Redact { reason, entities, .. } => {
                assert_eq!(reason, "PII detected and redacted: Email");
                assert_eq!(entities, vec!["email".to_string()]);
            }
            other => panic!("expected Decision::Redact, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_multiple_redacting_stages_accumulate_reasons_and_entities() {
        use crate::test_helpers::RedactingStage;

        let pipeline = PipelineBuilder::default()
            .stage(RedactingStage::with_name(
                "stage_a",
                "redacted email",
                vec!["email".to_string()],
            ))
            .stage(RedactingStage::with_name(
                "stage_b",
                "redacted phone",
                vec!["phone".to_string()],
            ))
            .build();

        let req = clean_request();
        let (decision, _) = pipeline.run(req).await.unwrap();

        match decision {
            Decision::Redact { reason, entities, .. } => {
                assert_eq!(reason, "redacted email; redacted phone");
                assert_eq!(entities, vec!["email".to_string(), "phone".to_string()]);
            }
            other => panic!("expected Decision::Redact, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_duplicate_entities_across_stages_are_deduplicated() {
        use crate::test_helpers::RedactingStage;

        // Two stages both report "email" — the final entity list must not
        // contain it twice.
        let pipeline = PipelineBuilder::default()
            .stage(RedactingStage::with_name(
                "stage_a",
                "first pass",
                vec!["email".to_string()],
            ))
            .stage(RedactingStage::with_name(
                "stage_b",
                "second pass",
                vec!["email".to_string(), "phone".to_string()],
            ))
            .build();

        let req = clean_request();
        let (decision, _) = pipeline.run(req).await.unwrap();

        match decision {
            Decision::Redact { entities, .. } => {
                assert_eq!(entities, vec!["email".to_string(), "phone".to_string()]);
            }
            other => panic!("expected Decision::Redact, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_redact_then_block_returns_block_not_redact() {
        // A request that gets redacted by an earlier stage and then blocked
        // by a later stage must surface as Block — blocking always takes
        // priority since it's a stronger guarantee (the request never
        // reaches the upstream either way).
        use crate::test_helpers::RedactingStage;

        let pipeline = PipelineBuilder::default()
            .stage(RedactingStage::new("redacted first", vec!["email".to_string()]))
            .stage(BlockingStage)
            .build();

        let req = clean_request();
        let (decision, _) = pipeline.run(req).await.unwrap();

        assert!(matches!(decision, Decision::Block { .. }));
    }

    #[tokio::test]
    async fn test_redacted_request_is_passed_to_subsequent_stages() {
        // The mutated request from a redacting stage must be visible to
        // stages that run after it (already correctly implemented before
        // this fix, but verified here alongside the new accumulation logic
        // to guard against a future regression).
        use crate::test_helpers::RedactingStage;

        let pipeline = PipelineBuilder::default()
            .stage(RedactingStage::new("redact", vec!["email".to_string()]))
            .stage(PassthroughStage)
            .build();

        let req = clean_request();
        let (decision, final_req) = pipeline.run(req.clone()).await.unwrap();

        assert!(matches!(decision, Decision::Redact { .. }));
        // RedactingStage's `mutated` is `req.clone()` (identical content in
        // this test helper), but the key invariant is that the SAME final
        // request flows through to PassthroughStage and out the other end
        // without being silently reset to the original.
        assert_eq!(final_req.user_text(), req.user_text());
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
