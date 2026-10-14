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

fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();
    runtime_for(&args)?.block_on(commands::dispatch(args))
}

/// Build the Tokio runtime. For `guardrail run`, `server.workers` sets the
/// number of worker threads (`0`, the default, means one per CPU). The
/// runtime exists before the server starts, so a SIGHUP reload can't change
/// it.
fn runtime_for(args: &cli::Cli) -> std::io::Result<tokio::runtime::Runtime> {
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();
    if let Some(n) = worker_threads(args) {
        builder.worker_threads(n);
    }
    builder.build()
}

/// `Some(n)` if this is `guardrail run` and the config sets `server.workers`
/// to `n > 0`. A config that fails to load is reported by `run` itself.
fn worker_threads(args: &cli::Cli) -> Option<usize> {
    let cli::Command::Run { config } = &args.command else {
        return None;
    };
    let workers = guardrail_config::loader::load_config(config)
        .ok()?
        .server
        .workers;
    (workers > 0).then_some(workers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn config_with_workers(workers: usize) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(
            f,
            "[server]\nhost = \"127.0.0.1\"\nport = 0\nworkers = {workers}\n\n\
             [upstream]\nurl = \"https://api.openai.com\"\n"
        )
        .unwrap();
        f
    }

    fn run_args(path: &std::path::Path) -> cli::Cli {
        cli::Cli::parse_from(["guardrail", "run", "--config", path.to_str().unwrap()])
    }

    #[test]
    fn workers_setting_sets_runtime_worker_threads() {
        let f = config_with_workers(3);
        let args = run_args(f.path());
        assert_eq!(worker_threads(&args), Some(3));

        let rt = runtime_for(&args).unwrap();
        assert_eq!(rt.metrics().num_workers(), 3);
    }

    #[test]
    fn workers_zero_means_tokio_default() {
        let f = config_with_workers(0);
        assert_eq!(worker_threads(&run_args(f.path())), None);
    }
}
