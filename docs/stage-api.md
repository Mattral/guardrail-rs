# Stage API Reference

This document is the definitive reference for implementing custom pipeline
stages. All built-in stages in `guardrail-classifiers` implement the same
`Stage` trait documented here.

## `guardrail_core::Stage`

```rust
#[async_trait::async_trait]
pub trait Stage: Send + Sync + 'static {
    /// Human-readable identifier used in logs, metrics, and OTel spans.
    /// Must be lowercase snake_case and stable across process restarts
    /// (used as a Prometheus label value).
    fn name(&self) -> &'static str;

    /// Evaluate a request and return a `Decision`.
    async fn evaluate(
        &self,
        req: &GuardrailRequest,
    ) -> Result<Decision, GuardrailError>;
}
```

## Decision variants

| Variant | Meaning | Effect on pipeline |
|---------|---------|-------------------|
| `Decision::Allow` | Request is clean | Pipeline continues to next stage |
| `Decision::Redact { reason, mutated, entities }` | PII or sensitive data replaced | `mutated` replaces the request for all subsequent stages; `entities` is a machine-readable list of redacted entity-type names (e.g. `["email", "phone"]`) for the audit trail |
| `Decision::Block { reason, code }` | Request must be stopped | Pipeline short-circuits; 403 returned immediately |

### Block codes

| `BlockCode` | `as_str()` | When to use |
|-------------|-----------|-------------|
| `PromptInjection` | `"prompt_injection"` | Regex or semantic injection match |
| `Toxicity` | `"toxicity"` | Toxicity classifier threshold exceeded |
| `PolicyViolation` | `"policy_violation"` | User-defined policy rule matched |
| `RateLimit` | `"rate_limit"` | Reserved for future rate-limiting stages |
| `Custom(String)` | *(your string)* | Custom stage-defined code |

## Contract

Every stage implementation **must** satisfy these invariants:

### 1. No in-place mutation
Stages must not mutate the `GuardrailRequest` in place. Express modifications
by returning `Decision::Redact { mutated: modified_copy, .. }`.

### 2. Concurrency safety
Stages must be `Send + Sync + 'static`. All per-stage state (compiled regexes,
ONNX sessions, configuration) must be held behind `Arc` or be `Copy`. The same
stage instance is shared across all concurrent requests.

### 3. Fail-open errors
On recoverable errors (e.g. a model returns an unexpected output shape),
prefer returning `Ok(Decision::Allow)` with a `tracing::warn!` log. Only
return `Err(GuardrailError)` for unrecoverable initialization failures that
make the stage fundamentally non-functional.

### 4. Non-blocking async
Never call blocking I/O or CPU-intensive code directly inside `evaluate`.
Use `tokio::task::spawn_blocking` for CPU-bound work (ONNX inference,
heavy regex on very large inputs). The pipeline runs on the Tokio async
executor — blocking it will starve all other requests.

### 5. Idempotency of `Redact`
If a stage returns `Redact`, the `mutated` request it returns must produce
the same `Redact` decision if passed through the stage again (idempotency).
This is required for the audit log to accurately describe what was changed.

### 6. `entities` is best-effort, not mandatory
A redacting stage with no typed taxonomy to report against (e.g. a custom
stage redacting based on a free-form policy match rather than a known PII
entity type) may return `entities: Vec::new()`. The pipeline aggregates
`entities` from every redacting stage that ran, de-duplicating across
stages — an empty `Vec` from one stage simply contributes nothing to that
union, it never causes an error.

## Minimal example

```rust
use async_trait::async_trait;
use guardrail_core::{
    decision::{BlockCode, Decision},
    error::GuardrailError,
    pipeline::Stage,
    request::GuardrailRequest,
};

/// Blocks requests longer than a configurable byte threshold.
pub struct MaxLengthGuard {
    max_bytes: usize,
}

impl MaxLengthGuard {
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }
}

#[async_trait]
impl Stage for MaxLengthGuard {
    fn name(&self) -> &'static str {
        "max_length_guard"
    }

    async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError> {
        let total_bytes = req.all_text().len();
        if total_bytes > self.max_bytes {
            return Ok(Decision::Block {
                reason: format!(
                    "Request content is {total_bytes} bytes, exceeds limit of {}.",
                    self.max_bytes
                ),
                code: BlockCode::PolicyViolation,
            });
        }
        Ok(Decision::Allow)
    }
}
```

## Adding to the pipeline

### Via `PipelineBuilder`

```rust
use guardrail_core::PipelineBuilder;

let pipeline = PipelineBuilder::default()
    .stage(guardrail_classifiers::RegexInjectionScanner::default())
    .stage(MaxLengthGuard::new(4096))
    .build();
```

### Via config (custom stages require code changes)

Custom stages must be compiled into a binary. They cannot be loaded as plugins
at runtime. Wire them into `guardrail_config::loader::build_pipeline`:

```rust
// In crates/guardrail-config/src/loader.rs

if config.stages.my_custom_stage.enabled {
    let guard = MaxLengthGuard::new(config.stages.my_custom_stage.max_bytes);
    builder = builder.stage(guard);
}
```

and add the corresponding `MyCustomStageConfig` to `StagesConfig` in
`crates/guardrail-config/src/schema.rs`.

## Using the `Classifier` backend trait

For stages that have swappable execution backends (regex vs ONNX), implement
[`guardrail_classifiers::Classifier`] instead of `Stage` directly, then wrap
it in a stage:

```rust
use guardrail_classifiers::classifier::{Classifier, ClassifierScore};
use guardrail_core::{error::GuardrailError, pipeline::Stage, request::GuardrailRequest,
    decision::{BlockCode, Decision}};

pub struct MyClassifierStage<C> {
    classifier: C,
    threshold: f32,
}

impl<C: Classifier<Input = String, Output = ClassifierScore>> Stage
    for MyClassifierStage<C>
{
    fn name(&self) -> &'static str { "my_classifier" }

    async fn evaluate(&self, req: &GuardrailRequest) -> Result<Decision, GuardrailError> {
        let score = self.classifier.classify(req.user_text()).await?;
        if score.is_above_threshold(self.threshold) {
            return Ok(Decision::Block {
                reason: format!("score {:.3}", score.score),
                code: BlockCode::Custom("my_check".into()),
            });
        }
        Ok(Decision::Allow)
    }
}
```

## Testing a stage

Use the helpers in `guardrail_core::test_helpers`:

```rust
use guardrail_core::{
    test_helpers::{clean_request, injection_request},
    Stage, Decision,
};

#[tokio::test]
async fn test_my_stage_allows_clean_request() {
    let stage = MaxLengthGuard::new(10_000);
    let d = stage.evaluate(&clean_request()).await.unwrap();
    assert_eq!(d, Decision::Allow);
}

#[tokio::test]
async fn test_my_stage_blocks_long_request() {
    use guardrail_core::request::{ChatMessage, GuardrailRequest, MessageContent, Provider, Role};

    let stage = MaxLengthGuard::new(10);
    let req = GuardrailRequest::new(
        vec![ChatMessage {
            role: Role::User,
            content: MessageContent::Text("this text is definitely longer than 10 bytes".into()),
        }],
        "gpt-4o".into(),
        Provider::OpenAI,
    );

    let d = stage.evaluate(&req).await.unwrap();
    assert!(matches!(d, Decision::Block { .. }));
}
```

## Observability

Each stage automatically gets:

- A Prometheus histogram bucket in `guardrail_stage_duration_seconds{stage="<name>"}`.
- An OpenTelemetry child span `guardrail.stage.<name>` (when OTel is enabled).

Both are populated by `Pipeline::run_with_observer`, which the proxy calls
internally. No additional instrumentation is required in the stage itself.
