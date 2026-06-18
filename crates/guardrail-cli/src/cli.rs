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
        ///
        /// Falls back to the `GUARDRAIL_CONFIG` environment variable if the
        /// flag is not given, then to `guardrail.toml` in the current
        /// directory if neither is set.
        #[arg(short, long, env = "GUARDRAIL_CONFIG", default_value = "guardrail.toml")]
        config: PathBuf,
    },

    /// Validate a configuration file without starting the server.
    Validate {
        /// Path to the TOML configuration file.
        ///
        /// Falls back to the `GUARDRAIL_CONFIG` environment variable if the
        /// flag is not given, then to `guardrail.toml` in the current
        /// directory if neither is set.
        #[arg(short, long, env = "GUARDRAIL_CONFIG", default_value = "guardrail.toml")]
        config: PathBuf,
    },

    /// Run a single text payload through the pipeline and print the decision.
    ///
    /// Useful for testing rules and policies without running a full server.
    Check {
        /// The text to evaluate (treated as a single user message).
        text: String,

        /// Path to the TOML configuration file.
        ///
        /// Falls back to the `GUARDRAIL_CONFIG` environment variable if the
        /// flag is not given, then to `guardrail.toml` in the current
        /// directory if neither is set.
        #[arg(short, long, env = "GUARDRAIL_CONFIG", default_value = "guardrail.toml")]
        config: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All `GUARDRAIL_CONFIG` behavior is tested in a single function rather
    /// than separate `#[test]` functions, because `std::env::set_var` /
    /// `remove_var` mutate process-global state and `cargo test`/`nextest`
    /// run tests in parallel by default — separate functions touching the
    /// same env var would race against each other.
    #[test]
    fn test_guardrail_config_env_var_behavior() {
        // 1. No env var, no flag: falls back to the hardcoded default.
        std::env::remove_var("GUARDRAIL_CONFIG");
        let cli = Cli::parse_from(["guardrail", "run"]);
        match cli.command {
            Command::Run { config } => assert_eq!(config, PathBuf::from("guardrail.toml")),
            _ => panic!("expected Run command"),
        }

        // 2. Env var set, no flag: env var wins over the hardcoded default.
        std::env::set_var("GUARDRAIL_CONFIG", "/from/env.toml");
        let cli = Cli::parse_from(["guardrail", "run"]);
        match cli.command {
            Command::Run { config } => assert_eq!(config, PathBuf::from("/from/env.toml")),
            _ => panic!("expected Run command"),
        }

        // 3. Env var set AND flag given: explicit flag wins over the env var.
        let cli = Cli::parse_from(["guardrail", "run", "--config", "/from/flag.toml"]);
        match cli.command {
            Command::Run { config } => assert_eq!(config, PathBuf::from("/from/flag.toml")),
            _ => panic!("expected Run command"),
        }

        // 4. Same env-var fallback applies to `validate`.
        let cli = Cli::parse_from(["guardrail", "validate"]);
        match cli.command {
            Command::Validate { config } => assert_eq!(config, PathBuf::from("/from/env.toml")),
            _ => panic!("expected Validate command"),
        }

        // 5. Same env-var fallback applies to `check`.
        let cli = Cli::parse_from(["guardrail", "check", "hello world"]);
        match cli.command {
            Command::Check { text, config } => {
                assert_eq!(text, "hello world");
                assert_eq!(config, PathBuf::from("/from/env.toml"));
            }
            _ => panic!("expected Check command"),
        }

        // Clean up so other tests in this process don't observe a stale value.
        std::env::remove_var("GUARDRAIL_CONFIG");
    }
}
