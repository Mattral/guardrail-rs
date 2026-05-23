//! Command-line argument definitions.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// guardrail-rs: a zero-Python, production-grade LLM security proxy.
#[derive(Debug, Parser)]
#[command(name = "guardrail", version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the proxy server.
    Run {
        /// Path to the TOML configuration file.
        #[arg(short, long, default_value = "guardrail.toml")]
        config: PathBuf,
    },

    /// Validate a configuration file without starting the server.
    Validate {
        /// Path to the TOML configuration file.
        #[arg(short, long, default_value = "guardrail.toml")]
        config: PathBuf,
    },

    /// Run a single text payload through the pipeline and print the decision.
    ///
    /// Useful for testing rules and policies without running a full server.
    Check {
        /// The text to evaluate (treated as a single user message).
        text: String,

        /// Path to the TOML configuration file.
        #[arg(short, long, default_value = "guardrail.toml")]
        config: PathBuf,
    },
}
