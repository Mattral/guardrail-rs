//! Detection evaluation: how often the default pipeline flags prompt
//! injection, measured on labelled datasets in `eval/`.
//!
//! Every case goes through the real pipeline (`Pipeline::run`) built from a
//! default configuration; a case counts as flagged when the decision is a
//! block with code `prompt_injection`. The test compares which cases were
//! flagged with `eval/baseline.json` and fails on any change:
//!
//! - an injection no longer caught, or a benign case newly flagged, is a
//!   regression;
//! - an injection newly caught, or a false alarm gone, is an improvement
//!   that has to be recorded, so the published numbers stay current.
//!
//! To record the current results (baseline and `eval/RESULTS.md`):
//!
//! ```text
//! GUARDRAIL_EVAL_UPDATE=1 cargo test -p guardrail-test-suite --test detection_eval
//! ```
//!
//! Datasets under `eval/data/external/` (fetched on demand, not committed)
//! are reported but not compared.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use guardrail_core::decision::{BlockCode, Decision};
use guardrail_core::request::{ChatMessage, GuardrailRequest, MessageContent, Provider, Role};
use serde_json::{json, Value};

/// The detector being measured. One today; the ONNX stages are measured by
/// an ignored test in `guardrail-classifiers` because they need real models.
const DETECTOR: &str = "regex_injection";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Label {
    Injection,
    Benign,
}

struct Case {
    id: String,
    label: Label,
    messages: Vec<ChatMessage>,
}

struct Dataset {
    name: String,
    /// Shown next to the results: where the data came from and what to keep
    /// in mind when reading them.
    note: &'static str,
    cases: Vec<Case>,
    /// Committed datasets are compared with the baseline; fetched ones are
    /// only reported.
    gated: bool,
}

#[derive(Default)]
struct Outcome {
    injections: usize,
    benign: usize,
    caught: BTreeSet<String>,
    false_alarms: BTreeSet<String>,
}

fn eval_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval")
}

fn user(text: &str) -> ChatMessage {
    ChatMessage {
        role: Role::User,
        content: MessageContent::Text(text.to_string()),
    }
}

fn parse_label(label: &str, id: &str) -> Label {
    match label {
        "injection" => Label::Injection,
        "benign" => Label::Benign,
        other => panic!("{id}: unknown label {other:?}"),
    }
}

/// A JSONL file of `{"id", "label": "injection" | "benign", "text"}`.
fn load_jsonl(path: &Path) -> Vec<Case> {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let record: Value = serde_json::from_str(line).unwrap();
            let id = record["id"].as_str().unwrap().to_string();
            let label = parse_label(record["label"].as_str().unwrap(), &id);
            Case {
                messages: vec![user(record["text"].as_str().unwrap())],
                id,
                label,
            }
        })
        .collect()
}

/// The hand-written cases in `tests/fixtures/`: blocks are injections,
/// allows are benign.
fn load_fixtures() -> Vec<Case> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let mut cases = Vec::new();
    for file in ["injection_prompts.json", "clean_prompts.json"] {
        let data: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join(file)).unwrap()).unwrap();
        for case in data["cases"].as_array().unwrap() {
            let id = format!("fixture-{}", case["name"].as_str().unwrap());
            let label = match case["expected_decision"].as_str().unwrap() {
                "block" => Label::Injection,
                "allow" => Label::Benign,
                other => panic!("{id}: unexpected decision {other:?}"),
            };
            let messages = case["messages"]
                .as_array()
                .unwrap()
                .iter()
                .map(|m| ChatMessage {
                    role: match m["role"].as_str().unwrap() {
                        "system" => Role::System,
                        "assistant" => Role::Assistant,
                        "tool" => Role::Tool,
                        _ => Role::User,
                    },
                    content: MessageContent::Text(m["content"].as_str().unwrap().to_string()),
                })
                .collect();
            cases.push(Case {
                id,
                label,
                messages,
            });
        }
    }
    cases
}

fn datasets() -> Vec<Dataset> {
    let mut sets = vec![
        Dataset {
            name: "deepset-prompt-injections".into(),
            note: "deepset/prompt-injections (Apache-2.0), train and test splits. \
                   Independent of the rules. Labels are the dataset's own; some are \
                   debatable, and part of it is in German.",
            cases: load_jsonl(&eval_dir().join("data/deepset-prompt-injections.jsonl")),
            gated: true,
        },
        Dataset {
            name: "handwritten-fixtures".into(),
            note: "tests/fixtures/ (this repository). Written alongside the rules, \
                   so these show intended behaviour, not detection quality.",
            cases: load_fixtures(),
            gated: true,
        },
    ];
    let external = eval_dir().join("data/external/jackhhao-jailbreak-classification.jsonl");
    if external.exists() {
        sets.push(Dataset {
            name: "jackhhao-jailbreak-classification".into(),
            note: "jackhhao/jailbreak-classification (Apache-2.0), fetched with \
                   eval/fetch-jackhhao.py. Mostly long role-play jailbreaks.",
            cases: load_jsonl(&external),
            gated: false,
        });
    }
    sets
}

async fn evaluate(pipeline: &guardrail_core::pipeline::Pipeline, cases: &[Case]) -> Outcome {
    let mut outcome = Outcome::default();
    for case in cases {
        let request =
            GuardrailRequest::new(case.messages.clone(), "gpt-4o".into(), Provider::OpenAI);
        let (decision, _) = pipeline.run(request).await.unwrap();
        let flagged = matches!(
            decision,
            Decision::Block {
                code: BlockCode::PromptInjection,
                ..
            }
        );
        match case.label {
            Label::Injection => {
                outcome.injections += 1;
                if flagged {
                    outcome.caught.insert(case.id.clone());
                }
            }
            Label::Benign => {
                outcome.benign += 1;
                if flagged {
                    outcome.false_alarms.insert(case.id.clone());
                }
            }
        }
    }
    outcome
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        "n/a".into()
    } else {
        format!("{:.1}%", 100.0 * part as f64 / whole as f64)
    }
}

fn results_markdown(sets: &[(Dataset, Outcome)]) -> String {
    let mut out = String::from(
        "# Detection results\n\n\
         Generated by `crates/guardrail-test-suite/tests/detection_eval.rs`; do not edit by hand.\n\
         Detector: `regex_injection` with the bundled rules, default configuration.\n\
         A case counts as caught when the pipeline blocks it with code `prompt_injection`.\n\
         Rows for fetched datasets (`eval/data/external/`) appear only when they were present.\n\
         How to read these numbers: [README.md](README.md).\n\n\
         | Dataset | Injections caught | Benign flagged |\n\
         |---|---|---|\n",
    );
    for (set, outcome) in sets {
        out += &format!(
            "| {} | {} of {} ({}) | {} of {} ({}) |\n",
            set.name,
            outcome.caught.len(),
            outcome.injections,
            percent(outcome.caught.len(), outcome.injections),
            outcome.false_alarms.len(),
            outcome.benign,
            percent(outcome.false_alarms.len(), outcome.benign),
        );
    }
    out += "\n";
    for (set, _) in sets {
        out += &format!("- **{}**: {}\n", set.name, set.note);
    }
    out
}

fn to_json(outcome: &Outcome) -> Value {
    json!({
        "injections": outcome.injections,
        "benign": outcome.benign,
        "caught": outcome.caught,
        "false_alarms": outcome.false_alarms,
    })
}

fn ids(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .map(|a| a.iter().map(|v| v.as_str().unwrap().to_string()).collect())
        .unwrap_or_default()
}

/// Every difference from the baseline, worded for the person who caused it.
fn compare(name: &str, outcome: &Outcome, baseline: Option<&Value>) -> Vec<String> {
    let Some(base) = baseline else {
        return vec![format!("{name}: not in the baseline")];
    };
    let mut changes = Vec::new();
    if base["injections"] != outcome.injections || base["benign"] != outcome.benign {
        changes.push(format!(
            "{name}: dataset changed ({} injections and {} benign, baseline has {} and {})",
            outcome.injections, outcome.benign, base["injections"], base["benign"]
        ));
    }
    let (caught, alarms) = (ids(&base["caught"]), ids(&base["false_alarms"]));
    for id in caught.difference(&outcome.caught) {
        changes.push(format!(
            "{name}: REGRESSION, injection no longer caught: {id}"
        ));
    }
    for id in outcome.false_alarms.difference(&alarms) {
        changes.push(format!("{name}: REGRESSION, benign case now flagged: {id}"));
    }
    for id in outcome.caught.difference(&caught) {
        changes.push(format!("{name}: improvement, injection now caught: {id}"));
    }
    for id in alarms.difference(&outcome.false_alarms) {
        changes.push(format!("{name}: improvement, false alarm gone: {id}"));
    }
    changes
}

#[tokio::test]
async fn detection_matches_recorded_baseline() {
    let config: guardrail_config::Config = toml::from_str(
        "[server]\n[upstream]\nurl = \"http://127.0.0.1:9\"\n[pipeline]\non_error = \"block\"\n",
    )
    .unwrap();
    let pipeline = guardrail_config::loader::build_pipeline(&config).unwrap();

    let mut results = Vec::new();
    for set in datasets() {
        let outcome = evaluate(&pipeline, &set.cases).await;
        results.push((set, outcome));
    }
    let report = results_markdown(&results);
    println!("{report}");

    let baseline_path = eval_dir().join("baseline.json");
    if std::env::var_os("GUARDRAIL_EVAL_UPDATE").is_some() {
        let gated: BTreeMap<_, _> = results
            .iter()
            .filter(|(set, _)| set.gated)
            .map(|(set, outcome)| (set.name.clone(), to_json(outcome)))
            .collect();
        let baseline = json!({ DETECTOR: gated });
        std::fs::write(
            &baseline_path,
            serde_json::to_string_pretty(&baseline).unwrap() + "\n",
        )
        .unwrap();
        std::fs::write(eval_dir().join("RESULTS.md"), report).unwrap();
        return;
    }

    let baseline: Value = serde_json::from_str(
        &std::fs::read_to_string(&baseline_path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", baseline_path.display())),
    )
    .unwrap();
    let changes: Vec<String> = results
        .iter()
        .filter(|(set, _)| set.gated)
        .flat_map(|(set, outcome)| {
            compare(
                &set.name,
                outcome,
                baseline[DETECTOR].get(set.name.as_str()),
            )
        })
        .collect();
    assert!(
        changes.is_empty(),
        "detection results differ from eval/baseline.json:\n  {}\n\n\
         If the change is intended, record it with\n  \
         GUARDRAIL_EVAL_UPDATE=1 cargo test -p guardrail-test-suite --test detection_eval\n\
         and commit eval/baseline.json and eval/RESULTS.md.",
        changes.join("\n  ")
    );
}
