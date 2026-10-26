//! Command-line argument definitions.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// guardrail-rs: a zero-Python LLM security proxy.
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
        #[arg(
            short,
            long,
            env = "GUARDRAIL_CONFIG",
            default_value = "guardrail.toml"
        )]
        config: PathBuf,
    },

    /// Validate a configuration file without starting the server.
    Validate {
        /// Path to the TOML configuration file.
        ///
        /// Falls back to the `GUARDRAIL_CONFIG` environment variable if the
        /// flag is not given, then to `guardrail.toml` in the current
        /// directory if neither is set.
        #[arg(
            short,
            long,
            env = "GUARDRAIL_CONFIG",
            default_value = "guardrail.toml"
        )]
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
        #[arg(
            short,
            long,
            env = "GUARDRAIL_CONFIG",
            default_value = "guardrail.toml"
        )]
        config: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// Every subcommand's `--config` falls back to `GUARDRAIL_CONFIG`. Checked
    /// on the argument definitions, so the test never changes the process
    /// environment (other tests run in parallel).
    #[test]
    fn test_config_flag_falls_back_to_env_var() {
        let command = Cli::command();
        for name in ["run", "validate", "check"] {
            let sub = command.find_subcommand(name).unwrap();
            let arg = sub
                .get_arguments()
                .find(|a| a.get_id() == "config")
                .unwrap_or_else(|| panic!("{name} has no --config"));
            assert_eq!(
                arg.get_env().and_then(|e| e.to_str()),
                Some("GUARDRAIL_CONFIG"),
                "{name}"
            );
        }
    }

    #[test]
    fn test_config_flag_wins_and_has_a_default() {
        let cli = Cli::parse_from(["guardrail", "run", "--config", "/from/flag.toml"]);
        match cli.command {
            Command::Run { config } => assert_eq!(config, PathBuf::from("/from/flag.toml")),
            _ => panic!("expected Run command"),
        }
        let check = Cli::command();
        let default = check
            .find_subcommand("check")
            .unwrap()
            .get_arguments()
            .find(|a| a.get_id() == "config")
            .unwrap()
            .get_default_values()
            .to_vec();
        assert_eq!(default, ["guardrail.toml"]);
    }
}
