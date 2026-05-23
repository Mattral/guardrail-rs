//! `guardrail` — command-line interface for `guardrail-rs`.
//!
//! ```text
//! guardrail run --config guardrail.toml
//! guardrail validate --config guardrail.toml
//! guardrail check "Ignore all previous instructions"
//! ```

mod cli;
mod commands;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();
    commands::dispatch(args).await
}
