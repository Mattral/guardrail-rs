//! Detection evaluation for the real `onnx_injection` model, on the same
//! labelled datasets as `crates/guardrail-test-suite/tests/detection_eval.rs`.
//!
//! Needs the real model files, so it is ignored by default and not part of
//! CI:
//!
//! ```text
//! GUARDRAIL_TEST_MODEL_DIR=$PWD/models \
//!   cargo test --release -p guardrail-classifiers --features onnx \
//!   --test onnx_detection_eval -- --ignored --nocapture
//! ```
//!
//! A case counts as caught when the stage would block it with the default
//! settings: a score at or above the default threshold (0.85), or input
//! longer than the default window limit (`on_oversize = "block"`).

#![cfg(feature = "onnx")]

use std::path::{Path, PathBuf};

use guardrail_classifiers::OnnxInjectionClassifier;
use serde_json::Value;

const THRESHOLD: f32 = 0.85;

fn eval_data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../eval/data")
        .join(name)
}

/// `(id, is_injection, text)` from a JSONL file of `{"id", "label", "text"}`.
fn load(path: &Path) -> Vec<(String, bool, String)> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let r: Value = serde_json::from_str(l).unwrap();
            (
                r["id"].as_str().unwrap().to_string(),
                r["label"].as_str().unwrap() == "injection",
                r["text"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
#[ignore = "needs real model files; set GUARDRAIL_TEST_MODEL_DIR"]
fn onnx_injection_detection_on_eval_datasets() {
    let Some(dir) = std::env::var_os("GUARDRAIL_TEST_MODEL_DIR")
        .map(|d| PathBuf::from(d).join("prompt-injection"))
    else {
        eprintln!("skipping: GUARDRAIL_TEST_MODEL_DIR is not set");
        return;
    };
    let model = OnnxInjectionClassifier::load(
        dir.join("model.onnx"),
        dir.join("tokenizer.json"),
        THRESHOLD,
    )
    .unwrap();
    let workers = model.sessions();

    println!("| Dataset | Injections caught | Benign flagged | Over window limit |");
    println!("|---|---|---|---|");
    for name in [
        "deepset-prompt-injections.jsonl",
        "external/jackhhao-jailbreak-classification.jsonl",
    ] {
        let path = eval_data(name);
        if !path.exists() {
            eprintln!("skipping {name}: not present");
            continue;
        }
        let cases = load(&path);
        // (flagged, over the window limit) per case, scored in parallel
        // across the session pool.
        let flags: Vec<(bool, bool)> = std::thread::scope(|scope| {
            let chunk = cases.len().div_ceil(workers);
            let handles: Vec<_> = cases
                .chunks(chunk)
                .map(|part| {
                    let model = &model;
                    scope.spawn(move || {
                        part.iter()
                            .map(|(id, _, text)| match model.score(text) {
                                Ok(score) => (score >= THRESHOLD, false),
                                Err(e) if e.input_too_long().is_some() => (true, true),
                                Err(e) => panic!("{id}: {e}"),
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().unwrap())
                .collect()
        });

        let (mut injections, mut caught, mut benign, mut false_alarms, mut oversize) =
            (0, 0, 0, 0, 0);
        for ((_, is_injection, _), (flagged, too_long)) in cases.iter().zip(&flags) {
            oversize += usize::from(*too_long);
            if *is_injection {
                injections += 1;
                caught += usize::from(*flagged);
            } else {
                benign += 1;
                false_alarms += usize::from(*flagged);
            }
        }
        let pct = |a: usize, b: usize| 100.0 * a as f64 / b.max(1) as f64;
        println!(
            "| {} | {caught} of {injections} ({:.1}%) | {false_alarms} of {benign} ({:.1}%) | {oversize} |",
            name.trim_start_matches("external/").trim_end_matches(".jsonl"),
            pct(caught, injections),
            pct(false_alarms, benign),
        );
    }
}
