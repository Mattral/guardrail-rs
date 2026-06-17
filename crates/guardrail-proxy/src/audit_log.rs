//! NDJSON audit log writer (spec §10).
//!
//! When `observability.audit_log.enabled = true`, every pipeline decision is
//! written as a newline-delimited JSON record to `observability.audit_log.path`.
//!
//! The returned [`tracing_appender::non_blocking::WorkerGuard`] must be kept
//! alive for the process lifetime; dropping it stops the background writer.
//!
//! # Example
//!
//! ```rust,no_run
//! use guardrail_config::schema::AuditLogConfig;
//! use guardrail_proxy::audit_log;
//! use tracing_subscriber::prelude::*;
//!
//! let config = AuditLogConfig {
//!     enabled: true,
//!     path: "/var/log/guardrail/audit.ndjson".into(),
//!     max_size_mb: 100,
//! };
//!
//! if let Some((layer, _guard)) = audit_log::build_layer::<tracing_subscriber::Registry>(
//!     &config
//! ).unwrap() {
//!     tracing_subscriber::registry().with(layer).init();
//! }
//! ```

use std::path::Path;

use guardrail_config::schema::AuditLogConfig;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::Layer;

/// The tracing target audit records are emitted under.
pub const AUDIT_TARGET: &str = "guardrail::audit";

/// Errors that can occur while setting up the audit log writer.
#[derive(Debug, thiserror::Error)]
pub enum AuditLogError {
    /// The parent directory could not be created.
    #[error("failed to create audit log directory '{path}': {source}")]
    CreateDir {
        /// The directory path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Build a `tracing_subscriber::Layer` writing audit events as NDJSON to
/// `config.path`. Returns `Ok(None)` when `config.enabled = false`.
///
/// # Errors
///
/// Returns [`AuditLogError::CreateDir`] if the parent directory cannot be created.
pub fn build_layer<S>(
    config: &AuditLogConfig,
) -> Result<Option<(Box<dyn Layer<S> + Send + Sync + 'static>, WorkerGuard)>, AuditLogError>
where
    S: tracing::Subscriber
        + for<'a> tracing_subscriber::registry::LookupSpan<'a>
        + Send
        + Sync,
{
    if !config.enabled {
        return Ok(None);
    }

    let audit_path = Path::new(&config.path);
    if let Some(parent) = audit_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|source| AuditLogError::CreateDir {
                path: parent.to_string_lossy().into_owned(),
                source,
            })?;
        }
    }

    let dir = audit_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let prefix = audit_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "audit.ndjson".into());

    let file_appender = tracing_appender::rolling::never(dir, &prefix);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let layer = tracing_subscriber::fmt::layer()
        .json()
        .with_span_events(FmtSpan::NONE)
        .with_target(false)
        .with_writer(non_blocking)
        .with_filter(filter_fn(|metadata| metadata.target() == AUDIT_TARGET))
        .boxed();

    Ok(Some((layer, guard)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditRecord;
    use guardrail_core::{decision::Decision, test_helpers::clean_request};
    use tracing_subscriber::prelude::*;

    fn temp_config() -> (tempfile::TempDir, AuditLogConfig) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit-test.ndjson").to_string_lossy().into_owned();
        let config = AuditLogConfig { enabled: true, path, max_size_mb: 100 };
        (dir, config)
    }

    #[test]
    fn test_disabled_returns_none() {
        let config = AuditLogConfig { enabled: false, path: "/tmp/x.ndjson".into(), max_size_mb: 100 };
        assert!(build_layer::<tracing_subscriber::Registry>(&config).unwrap().is_none());
    }

    #[test]
    fn test_builds_with_valid_config() {
        let (_dir, config) = temp_config();
        assert!(build_layer::<tracing_subscriber::Registry>(&config).unwrap().is_some());
    }

    #[test]
    fn test_creates_parent_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("sub").join("dir").join("audit.ndjson")
            .to_string_lossy().into_owned();
        let config = AuditLogConfig { enabled: true, path, max_size_mb: 100 };
        assert!(build_layer::<tracing_subscriber::Registry>(&config).is_ok());
    }

    #[test]
    fn test_writes_audit_events_as_ndjson() {
        let (_dir, config) = temp_config();
        let path = config.path.clone();

        let (layer, _guard) = build_layer::<tracing_subscriber::Registry>(&config)
            .unwrap().unwrap();
        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            let req = clean_request();
            AuditRecord::from_decision(&req, &Decision::Allow, &[], 0.05, 120.0).emit();
            tracing::info!(target: "unrelated::module", "should not appear");
        });

        std::thread::sleep(std::time::Duration::from_millis(250));

        let contents = std::fs::read_to_string(&path).unwrap_or_default();

        // Must contain an audit event.
        assert!(
            contents.contains("pipeline decision") || contents.contains("decision"),
            "missing audit event in: {contents:?}"
        );
        // Must NOT contain the unrelated event.
        assert!(!contents.contains("should not appear"));

        // Every non-empty line must be valid JSON.
        for line in contents.lines().filter(|l| !l.trim().is_empty()) {
            serde_json::from_str::<serde_json::Value>(line)
                .unwrap_or_else(|e| panic!("not valid JSON: {line:?}: {e}"));
        }
    }
}
