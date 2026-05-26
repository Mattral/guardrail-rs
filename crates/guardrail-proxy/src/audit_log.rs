//! Rotating NDJSON audit log file.
//!
//! In addition to the `tracing`-event-based audit trail emitted by
//! [`crate::audit::AuditRecord::emit`], `guardrail-rs` can write the same
//! records as newline-delimited JSON (NDJSON) to a dedicated, rotating log
//! file — suitable for direct ingestion by log shippers (Filebeat, Vector,
//! Fluent Bit) without needing to parse general application logs.
//!
//! This is implemented as a `tracing_subscriber::Layer` filtered to events
//! with `target = "guardrail::audit"`, writing through a
//! [`tracing_appender::rolling::RollingFileAppender`]. The returned
//! [`tracing_appender::non_blocking::WorkerGuard`] **must be kept alive** for
//! the lifetime of the process — dropping it stops the background writer
//! thread and any buffered records may be lost.
//!
//! # Example
//!
//! ```rust,no_run
//! use guardrail_config::schema::AuditLogConfig;
//! use guardrail_proxy::audit_log;
//! use tracing_subscriber::prelude::*;
//!
//! let audit_config = AuditLogConfig {
//!     enabled: true,
//!     directory: "/var/log/guardrail".into(),
//!     file_name_prefix: "audit".into(),
//!     rotation: "daily".into(),
//! };
//!
//! let (layer, _guard) = audit_log::build_layer(&audit_config).unwrap();
//!
//! tracing_subscriber::registry()
//!     .with(tracing_subscriber::fmt::layer())
//!     .with(layer)
//!     .init();
//!
//! // `_guard` must be held for the lifetime of the program.
//! ```

use std::path::Path;

use guardrail_config::schema::AuditLogConfig;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::Layer;

/// The `tracing` target that audit records are emitted under.
///
/// Matches [`crate::audit::AuditRecord::emit`].
pub const AUDIT_TARGET: &str = "guardrail::audit";

/// Errors that can occur while setting up the audit log file.
#[derive(Debug, thiserror::Error)]
pub enum AuditLogError {
    /// `rotation` was not one of the recognized values.
    #[error(
        "invalid observability.audit_log.rotation '{0}'; expected 'minutely', 'hourly', 'daily', or 'never'"
    )]
    InvalidRotation(String),

    /// The log directory could not be created.
    #[error("failed to create audit log directory '{path}': {source}")]
    CreateDir {
        /// The directory that could not be created.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Build a `tracing_subscriber::Layer` that writes audit events as NDJSON to
/// a rotating file, and the [`WorkerGuard`] that keeps its background writer
/// thread alive.
///
/// Returns `Ok(None)` if `config.enabled` is `false` — callers should skip
/// adding the layer in that case.
///
/// # Errors
///
/// Returns [`AuditLogError::InvalidRotation`] if `config.rotation` is not a
/// recognized value, or [`AuditLogError::CreateDir`] if `config.directory`
/// cannot be created.
///
/// # Panics
///
/// Does not panic.
pub fn build_layer<S>(
    config: &AuditLogConfig,
) -> Result<Option<(Box<dyn Layer<S> + Send + Sync + 'static>, WorkerGuard)>, AuditLogError>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a> + Send + Sync,
{
    if !config.enabled {
        return Ok(None);
    }

    let rotation = match config.rotation.as_str() {
        "minutely" => Rotation::MINUTELY,
        "hourly" => Rotation::HOURLY,
        "daily" => Rotation::DAILY,
        "never" => Rotation::NEVER,
        other => return Err(AuditLogError::InvalidRotation(other.to_string())),
    };

    std::fs::create_dir_all(&config.directory).map_err(|source| AuditLogError::CreateDir {
        path: config.directory.clone(),
        source,
    })?;

    let appender = RollingFileAppender::new(
        rotation,
        Path::new(&config.directory),
        &config.file_name_prefix,
    );

    let (non_blocking, guard) = tracing_appender::non_blocking(appender);

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
    use std::io::Read;
    use tracing_subscriber::prelude::*;

    #[test]
    fn test_disabled_config_returns_none() {
        let config = AuditLogConfig {
            enabled: false,
            directory: "/tmp/does-not-matter".into(),
            file_name_prefix: "audit".into(),
            rotation: "daily".into(),
        };

        let result = build_layer::<tracing_subscriber::Registry>(&config).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_rotation_errors() {
        let dir = tempfile::tempdir().unwrap();
        let config = AuditLogConfig {
            enabled: true,
            directory: dir.path().to_string_lossy().into_owned(),
            file_name_prefix: "audit".into(),
            rotation: "weekly".into(), // not a valid value
        };

        let result = build_layer::<tracing_subscriber::Registry>(&config);
        assert!(matches!(result, Err(AuditLogError::InvalidRotation(_))));
    }

    #[test]
    fn test_writes_ndjson_for_audit_target_only() {
        let dir = tempfile::tempdir().unwrap();
        let config = AuditLogConfig {
            enabled: true,
            directory: dir.path().to_string_lossy().into_owned(),
            file_name_prefix: "audit-test".into(),
            rotation: "never".into(),
        };

        let (layer, _guard) = build_layer::<tracing_subscriber::Registry>(&config)
            .unwrap()
            .unwrap();

        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            // This should be written (audit target).
            let req = clean_request();
            AuditRecord::from_decision(&req, &Decision::Allow).emit();

            // This should NOT be written (different target).
            tracing::info!(target: "some::other::target", "irrelevant event");
        });

        // Give the non-blocking writer a moment to flush.
        std::thread::sleep(std::time::Duration::from_millis(200));

        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1, "expected exactly one audit log file");

        let mut contents = String::new();
        std::fs::File::open(entries[0].path())
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();

        assert!(contents.contains("\"decision\":\"allow\""));
        assert!(!contents.contains("irrelevant event"));

        // Each non-empty line must be valid JSON.
        for line in contents.lines().filter(|l| !l.trim().is_empty()) {
            let parsed: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("line is not valid JSON: {line:?}: {e}"));
            assert!(parsed.get("decision").is_some() || parsed.get("fields").is_some());
        }
    }
}
