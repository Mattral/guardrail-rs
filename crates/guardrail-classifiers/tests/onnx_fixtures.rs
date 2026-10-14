//! Real ONNX Runtime inference against the tiny fixture models in
//! `tests/fixtures/onnx/` (see `make_fixtures.py` there for what each model
//! computes). Run with:
//!
//! ```text
//! cargo test -p guardrail-classifiers --features onnx --test onnx_fixtures
//! ```

#![cfg(feature = "onnx")]

use std::path::PathBuf;

use guardrail_classifiers::{OnnxInjectionClassifier, ToxicityClassifier};
use guardrail_core::{
    decision::{BlockCode, Decision},
    pipeline::Stage,
    request::{ChatMessage, GuardrailRequest, MessageContent, Provider, Role},
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/onnx")
        .join(name)
}

fn request(user_text: &str) -> GuardrailRequest {
    GuardrailRequest::new(
        vec![ChatMessage {
            role: Role::User,
            content: MessageContent::Text(user_text.into()),
        }],
        "gpt-4o".into(),
        Provider::OpenAI,
    )
}

fn injection(model: &str, threshold: f32) -> OnnxInjectionClassifier {
    OnnxInjectionClassifier::load(fixture(model), fixture("tokenizer.json"), threshold)
        .expect("fixture model loads")
}

fn toxicity(model: &str, threshold: f32) -> ToxicityClassifier {
    ToxicityClassifier::load(fixture(model), fixture("tokenizer.json"), threshold)
        .expect("fixture model loads")
}

// ── OnnxInjectionClassifier ──────────────────────────────────────────────────

#[tokio::test]
async fn injection_blocks_when_score_is_over_threshold() {
    let stage = injection("keyword_2class.onnx", 0.85);
    let decision = stage
        .evaluate(&request("hello trigger world"))
        .await
        .unwrap();
    match decision {
        Decision::Block { code, .. } => assert_eq!(code, BlockCode::PromptInjection),
        other => panic!("expected Block, got {other:?}"),
    }
}

#[tokio::test]
async fn injection_allows_when_score_is_under_threshold() {
    let stage = injection("keyword_2class.onnx", 0.85);
    let decision = stage.evaluate(&request("hello world")).await.unwrap();
    assert_eq!(decision, Decision::Allow);
}

#[test]
fn injection_score_matches_model_logits() {
    let stage = injection("keyword_2class.onnx", 0.85);
    // logits [0, +10] -> softmax ~ 0.99995; [0, -10] -> ~ 0.0000454
    assert!(stage.score("trigger").unwrap() > 0.999);
    assert!(stage.score("hello").unwrap() < 0.001);
}

#[tokio::test]
async fn injection_threshold_is_respected() {
    // With the threshold above the trigger score, nothing blocks.
    let stage = injection("keyword_2class.onnx", 1.0);
    let decision = stage.evaluate(&request("trigger")).await.unwrap();
    assert_eq!(decision, Decision::Allow);
}

#[tokio::test]
async fn injection_skips_requests_without_user_text() {
    // Before the fix this ran inference on a zero-length input, which errors;
    // with on_error = "block" that would block every request that has no
    // user text.
    let stage = injection("keyword_2class.onnx", 0.85);
    let decision = stage.evaluate(&request("   ")).await.unwrap();
    assert_eq!(decision, Decision::Allow);
}

#[tokio::test]
async fn injection_wrong_output_shape_is_an_error_not_a_panic() {
    // Before the fix, indexing logits[1] on a [1, 1] output panicked while
    // holding the session lock, poisoning it for every later request.
    let stage = injection("wrong_output.onnx", 0.85);
    for _ in 0..2 {
        let err = stage
            .evaluate(&request("hello"))
            .await
            .expect_err("a model with one logit must be reported as an error");
        assert!(err.to_string().contains("expected 2 logits"), "{err}");
    }
}

// ── ToxicityClassifier ───────────────────────────────────────────────────────

#[tokio::test]
async fn toxicity_multilabel_blocks_toxic_input() {
    // Toxic input: toxicity logit +10, severe_toxicity -9. The old 2-class
    // softmax scored P(index 1) = sigmoid(-9 - 10) ~ 0 and allowed it, so
    // with a multi-label model like unitary/unbiased-toxic-roberta the stage
    // almost never blocked anything.
    let stage = toxicity("keyword_multilabel.onnx", 0.90);
    let decision = stage.evaluate(&request("trigger")).await.unwrap();
    match decision {
        Decision::Block { code, .. } => assert_eq!(code, BlockCode::Toxicity),
        other => panic!("expected Block, got {other:?}"),
    }
}

#[tokio::test]
async fn toxicity_multilabel_allows_clean_input() {
    // Clean input: toxicity logit -4, every other label -9 (the shape a real
    // multi-label model gives). Score is sigmoid(-4) ~ 0.018.
    let stage = toxicity("keyword_multilabel.onnx", 0.90);
    let score = stage.score("hello world").unwrap();
    assert!(score < 0.05, "score {score}");
    let decision = stage.evaluate(&request("hello world")).await.unwrap();
    assert_eq!(decision, Decision::Allow);
}

#[tokio::test]
async fn toxicity_two_class_model_still_works() {
    let stage = toxicity("keyword_2class.onnx", 0.90);
    assert!(matches!(
        stage.evaluate(&request("trigger")).await.unwrap(),
        Decision::Block { .. }
    ));
    assert_eq!(
        stage.evaluate(&request("hello")).await.unwrap(),
        Decision::Allow
    );
}

#[tokio::test]
async fn toxicity_wrong_output_shape_is_an_error() {
    let stage = toxicity("wrong_output.onnx", 0.90);
    assert!(stage.evaluate(&request("hello")).await.is_err());
}

// ── Long input ───────────────────────────────────────────────────────────────

/// `n` clean words followed by "trigger": with the fixture tokenizer (one
/// token per word, no special tokens) that's n + 1 tokens.
fn padded_trigger(n: usize) -> String {
    let mut s = "hello ".repeat(n);
    s.push_str("trigger");
    s
}

#[tokio::test]
async fn toxicity_long_input_is_scored_in_windows_not_failed() {
    // keyword_multilabel_max514 fails inference past 514 tokens, like
    // RoBERTa. Before windowing, padding a toxic message past that length
    // made the stage error, and with the default on_error = "allow" the
    // request went through unchecked.
    let stage = toxicity("keyword_multilabel_max514.onnx", 0.90);
    let decision = stage
        .evaluate(&request(&padded_trigger(1500)))
        .await
        .unwrap();
    assert!(
        matches!(decision, Decision::Block { .. }),
        "got {decision:?}"
    );

    // A long input with no trigger is still allowed.
    let clean = "hello ".repeat(1500);
    assert_eq!(
        stage.evaluate(&request(&clean)).await.unwrap(),
        Decision::Allow
    );
}

#[tokio::test]
async fn injection_trigger_anywhere_in_a_long_input_is_found() {
    let stage = injection("keyword_2class.onnx", 0.85);
    for n in [10, 511, 512, 513, 700, 2000] {
        let decision = stage.evaluate(&request(&padded_trigger(n))).await.unwrap();
        assert!(
            matches!(decision, Decision::Block { .. }),
            "n={n}: {decision:?}"
        );
    }
}

#[tokio::test]
async fn input_over_the_window_limit_is_an_error() {
    // Each window after the first adds 512 - 128 = 384 new tokens, so 16
    // windows cover 512 + 15 * 384 = 6272 tokens.
    use guardrail_classifiers::onnx::{MAX_WINDOWS, MAX_WINDOW_TOKENS, WINDOW_STRIDE_TOKENS};
    let limit = MAX_WINDOW_TOKENS + (MAX_WINDOWS - 1) * (MAX_WINDOW_TOKENS - WINDOW_STRIDE_TOKENS);

    let stage = injection("keyword_2class.onnx", 0.85);
    assert!(stage
        .evaluate(&request(&"hello ".repeat(limit)))
        .await
        .is_ok());
    let err = stage
        .evaluate(&request(&"hello ".repeat(limit + 1)))
        .await
        .expect_err("past the window limit must be an error, so on_error decides");
    assert!(err.to_string().contains("windows"), "{err}");
}

#[test]
fn loading_a_missing_model_fails() {
    assert!(OnnxInjectionClassifier::load(
        fixture("missing.onnx"),
        fixture("tokenizer.json"),
        0.85
    )
    .is_err());
}
