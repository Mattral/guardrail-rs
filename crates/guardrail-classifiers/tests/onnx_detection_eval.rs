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
//! With `GUARDRAIL_EVAL_UPDATE=1` it also writes its tables into
//! `docs/benchmarks.md`, between the `onnx-detection` markers, so the
//! published numbers come from this test rather than from hand edits.
//!
//! A case counts as caught when the stage would block it with the default
//! settings: a score at or above the default threshold (0.85), or input
//! longer than the default window limit (`on_oversize = "block"`).

#![cfg(feature = "onnx")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use guardrail_classifiers::OnnxInjectionClassifier;
use serde_json::Value;

const THRESHOLD: f32 = 0.85;
const START: &str = "<!-- onnx-detection:start -->";
const END: &str = "<!-- onnx-detection:end -->";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Case {
    id: String,
    injection: bool,
    lang: Option<String>,
    text: String,
}

fn load(path: &Path) -> Vec<Case> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let r: Value = serde_json::from_str(l).unwrap();
            Case {
                id: r["id"].as_str().unwrap().to_string(),
                injection: r["label"].as_str().unwrap() == "injection",
                lang: r["lang"].as_str().map(str::to_string),
                text: r["text"].as_str().unwrap().to_string(),
            }
        })
        .collect()
}

/// 95% Wilson score interval, as fractions. Same as in `detection_eval.rs`.
fn wilson(k: usize, n: usize) -> (f64, f64) {
    let (k, n, z) = (k as f64, n as f64, 1.959_964);
    let p = k / n;
    let denom = 1.0 + z * z / n;
    let centre = (p + z * z / (2.0 * n)) / denom;
    let half = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt() / denom;
    ((centre - half).max(0.0), (centre + half).min(1.0))
}

fn rate(k: usize, n: usize) -> String {
    if n == 0 {
        return "no cases".into();
    }
    let (lo, hi) = wilson(k, n);
    format!(
        "{k} of {n}: {:.1}% ({:.1}–{:.1}%)",
        100.0 * k as f64 / n as f64,
        100.0 * lo,
        100.0 * hi
    )
}

#[derive(Default, Clone, Copy)]
struct Counts {
    injections: usize,
    caught: usize,
    benign: usize,
    flagged: usize,
    oversize: usize,
}

impl Counts {
    fn add(&mut self, injection: bool, flagged: bool, oversize: bool) {
        self.oversize += usize::from(oversize);
        if injection {
            self.injections += 1;
            self.caught += usize::from(flagged);
        } else {
            self.benign += 1;
            self.flagged += usize::from(flagged);
        }
    }
}

/// `(flagged, over the window limit)` per case, scored in parallel across
/// the model's session pool.
fn score_all(model: &OnnxInjectionClassifier, cases: &[Case]) -> Vec<(bool, bool)> {
    let chunk = cases.len().div_ceil(model.sessions()).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = cases
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|case| match model.score(&case.text) {
                            Ok(score) => (score >= THRESHOLD, false),
                            Err(e) if e.input_too_long().is_some() => (true, true),
                            Err(e) => panic!("{}: {e}", case.id),
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    })
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

    let datasets = [
        (
            "deepset/prompt-injections (not in the model's training data)",
            "deepset-prompt-injections.jsonl",
        ),
        (
            "jackhhao/jailbreak-classification (in the model's training data)",
            "external/jackhhao-jailbreak-classification.jsonl",
        ),
    ];
    let mut table =
        String::from("| Dataset | Injections caught | Benign flagged |\n|---|---|---|\n");
    let mut by_lang_tables = String::new();
    let mut oversize = 0;
    for (title, file) in datasets {
        let path = repo().join("eval/data").join(file);
        if !path.exists() {
            eprintln!("skipping {file}: not present");
            continue;
        }
        let cases = load(&path);
        let flags = score_all(&model, &cases);

        let mut total = Counts::default();
        let mut by_lang: BTreeMap<String, Counts> = BTreeMap::new();
        for (case, (flagged, too_long)) in cases.iter().zip(&flags) {
            total.add(case.injection, *flagged, *too_long);
            if let Some(lang) = &case.lang {
                by_lang
                    .entry(lang.clone())
                    .or_default()
                    .add(case.injection, *flagged, *too_long);
            }
        }
        oversize += total.oversize;
        table += &format!(
            "| {title} | {} | {} |\n",
            rate(total.caught, total.injections),
            rate(total.flagged, total.benign)
        );
        if !by_lang.is_empty() {
            by_lang_tables += &format!(
                "\nBy language, {}:\n\n| Language | Injections caught | Benign flagged |\n|---|---|---|\n",
                title.split(' ').next().unwrap()
            );
            for (lang, c) in &by_lang {
                by_lang_tables += &format!(
                    "| {lang} | {} | {} |\n",
                    rate(c.caught, c.injections),
                    rate(c.flagged, c.benign)
                );
            }
        }
    }
    let generated = format!(
        "{table}{by_lang_tables}\nBrackets give the 95% Wilson score interval. \
         Cases over the window limit (counted as caught): {oversize}.\n"
    );
    println!("{generated}");

    if std::env::var_os("GUARDRAIL_EVAL_UPDATE").is_some() {
        let doc_path = repo().join("docs/benchmarks.md");
        let doc = std::fs::read_to_string(&doc_path).unwrap();
        let (Some(start), Some(end)) = (doc.find(START), doc.find(END)) else {
            panic!("docs/benchmarks.md has no {START} … {END} markers");
        };
        let updated = format!(
            "{}{START}\n<!-- Generated by crates/guardrail-classifiers/tests/onnx_detection_eval.rs; do not edit by hand. -->\n\n{generated}\n{}",
            &doc[..start],
            &doc[end..]
        );
        std::fs::write(doc_path, updated).unwrap();
    }
}
