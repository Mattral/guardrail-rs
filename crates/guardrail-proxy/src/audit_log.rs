//! NDJSON audit log writer (spec §10).
//!
//! When `observability.audit_log.enabled = true`, every pipeline decision is
//! written as a newline-delimited JSON record to `observability.audit_log.path`.
//! When the file would exceed `observability.audit_log.max_size_mb`, it is
//! rotated: the current file is renamed to `<path>.<unix_timestamp>` and a
//! fresh file is opened at `path`. Rotation is checked before each write
//! (not on a timer), so the file never grows meaningfully past the
//! configured limit.
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
//! let _ = audit_log::build_layer(&config).unwrap();
//! ```

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use guardrail_config::schema::AuditLogConfig;
use tracing_appender::non_blocking::WorkerGuard;
// filter_fn/FmtSpan are composed by callers when constructing concrete
// `fmt` layers; this module only builds the writer+guard pair.

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
    /// The audit log file could not be opened.
    #[error("failed to open audit log file '{path}': {source}")]
    OpenFile {
        /// The file path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// A [`Write`] implementation that rotates the underlying file by renaming
/// it to `<path>.<unix_timestamp>` once it would exceed `max_size_bytes`,
/// then opening a fresh file at `path`.
///
/// Rotation is checked before each `write_all` call rather than on a timer,
/// so a burst of writes can momentarily exceed the threshold within a single
/// write (a single audit record is always written atomically, never split
/// across the rotation boundary) but never by more than one record's worth.
///
/// Wrapped in a [`Mutex`] so it satisfies `Write + Send + 'static` as
/// required by [`tracing_appender::non_blocking`] — the non-blocking
/// appender already serializes writes onto a single background thread, so
/// the lock is never contended in practice, but the type system requires
/// `Sync` regardless since `non_blocking` stores the writer behind an `Arc`.
struct SizeRotatingWriter {
    inner: Mutex<SizeRotatingState>,
}

struct SizeRotatingState {
    path: PathBuf,
    /// `None` only transiently during `rotate()`, between dropping the old
    /// handle and opening the new one — never observable outside this type.
    file: Option<File>,
    current_size: u64,
    max_size_bytes: u64,
}

impl SizeRotatingWriter {
    /// Open (or create) the file at `path` and prepare to rotate it once it
    /// would exceed `max_size_bytes`.
    fn open(path: &Path, max_size_bytes: u64) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let current_size = file.metadata()?.len();

        Ok(Self {
            inner: Mutex::new(SizeRotatingState {
                path: path.to_path_buf(),
                file: Some(file),
                current_size,
                max_size_bytes,
            }),
        })
    }
}

impl SizeRotatingState {
    /// Rotate the current file: rename it to `<path>.<unix_timestamp>` and
    /// open a fresh file at `path`. If a file with the timestamped name
    /// already exists (clock resolution collision), append an incrementing
    /// suffix until a free name is found.
    ///
    /// The old file handle is flushed and dropped (via `self.file.take()`)
    /// *before* the rename syscall runs. On POSIX this would work either way
    /// (rename operates on the inode regardless of open handles), but on
    /// Windows `std::fs::File` does not request `FILE_SHARE_DELETE` by
    /// default, so renaming a file that is still open elsewhere in the
    /// process can fail with an access-denied error. Dropping the handle
    /// first avoids that failure mode on every platform.
    ///
    /// `self.file` is `None` only for the brief window between the
    /// `.take()` and the reassignment a few lines later; if anything in
    /// between returns early via `?`, the writer is left in a permanently
    /// broken state (every subsequent `write()` call returns an error)
    /// rather than silently losing data or writing to a stale handle —
    /// this is the only sound choice once the rename itself has failed,
    /// since there is no original file left to fall back to write into.
    fn rotate(&mut self) -> io::Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let rotated_extension = |suffix: Option<u32>| {
            let base_ext = self
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("ndjson");
            match suffix {
                Some(n) => format!("{base_ext}.{timestamp}.{n}"),
                None => format!("{base_ext}.{timestamp}"),
            }
        };

        let mut rotated_path = self.path.with_extension(rotated_extension(None));
        let mut suffix = 1u32;
        while rotated_path.exists() {
            rotated_path = self.path.with_extension(rotated_extension(Some(suffix)));
            suffix += 1;
        }

        // Flush any buffered bytes, then explicitly drop the handle (via
        // `.take()`) so no file-share restriction (most relevant on
        // Windows, where `std::fs::File` does not request
        // `FILE_SHARE_DELETE` by default) blocks the rename below.
        if let Some(file) = self.file.as_mut() {
            file.flush()?;
        }
        self.file.take(); // drops the old handle here

        std::fs::rename(&self.path, &rotated_path)?;

        self.file = Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?,
        );
        self.current_size = 0;

        Ok(())
    }
}

impl Write for SizeRotatingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Rotate first if this write would push us over the threshold and
        // we've already written at least one byte (never rotate an empty
        // file just because a single record is itself huge).
        if state.current_size > 0
            && state.current_size + buf.len() as u64 > state.max_size_bytes
        {
            state.rotate()?;
        }

        let file = state.file.as_mut().ok_or_else(|| {
            io::Error::other("audit log file handle unexpectedly absent after rotation")
        })?;
        let written = file.write(buf)?;
        state.current_size += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match state.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()), // transient mid-rotation state; nothing to flush
        }
    }
}

/// Build a `tracing_subscriber::Layer` writing audit events as NDJSON to
/// `config.path`, rotating by size per `config.max_size_mb`. Returns
/// `Ok(None)` when `config.enabled = false`.
///
/// # Errors
///
/// Returns [`AuditLogError::CreateDir`] if the parent directory cannot be
/// created, or [`AuditLogError::OpenFile`] if the audit log file itself
/// cannot be opened (e.g. permissions).
pub fn build_layer(
    config: &AuditLogConfig,
) -> Result<Option<(tracing_appender::non_blocking::NonBlocking, WorkerGuard)>, AuditLogError>
where
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

    let max_size_bytes = config.max_size_mb.saturating_mul(1024 * 1024);
    let writer = SizeRotatingWriter::open(audit_path, max_size_bytes).map_err(|source| {
        AuditLogError::OpenFile {
            path: config.path.clone(),
            source,
        }
    })?;

    let (non_blocking, guard) = tracing_appender::non_blocking(writer);

    Ok(Some((non_blocking, guard)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditRecord;
    use guardrail_core::{decision::Decision, test_helpers::clean_request};
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::filter::filter_fn;

    fn temp_config() -> (tempfile::TempDir, AuditLogConfig) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit-test.ndjson").to_string_lossy().into_owned();
        let config = AuditLogConfig { enabled: true, path, max_size_mb: 100 };
        (dir, config)
    }

    #[test]
    fn test_disabled_returns_none() {
        let config = AuditLogConfig { enabled: false, path: "/tmp/x.ndjson".into(), max_size_mb: 100 };
        assert!(build_layer(&config).unwrap().is_none());
    }

    #[test]
    fn test_builds_with_valid_config() {
        let (_dir, config) = temp_config();
        assert!(build_layer(&config).unwrap().is_some());
    }

    #[test]
    fn test_creates_parent_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("sub").join("dir").join("audit.ndjson")
            .to_string_lossy().into_owned();
        let config = AuditLogConfig { enabled: true, path, max_size_mb: 100 };
        assert!(build_layer(&config).is_ok());
    }

    #[test]
    fn test_writes_audit_events_as_ndjson() {
        let (_dir, config) = temp_config();
        let path = config.path.clone();

        let (non_blocking, _guard) = build_layer(&config)
            .unwrap().unwrap();
        let layer = tracing_subscriber::fmt::layer()
            .json()
            .with_writer(non_blocking)
            .with_filter(filter_fn(|meta| meta.target().starts_with(AUDIT_TARGET)));

        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            let req = clean_request();
            AuditRecord::from_decision(&req, &Decision::Allow, 0.05, 120.0).emit();
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

    // ── SizeRotatingWriter unit tests ───────────────────────────────────────
    // These exercise the rotation logic directly (no tracing machinery, no
    // async flushing delay) for fast, precise coverage of the size threshold.

    #[test]
    fn test_rotating_writer_no_rotation_under_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.ndjson");

        let mut writer = SizeRotatingWriter::open(&path, 1024).unwrap();
        writer.write_all(b"a short line\n").unwrap();
        writer.flush().unwrap();

        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1, "expected exactly one file, found: {entries:?}");

        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "a short line\n");
    }

    #[test]
    fn test_rotating_writer_rotates_when_threshold_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.ndjson");

        let mut writer = SizeRotatingWriter::open(&path, 20).unwrap();
        writer.write_all(b"0123456789012345\n").unwrap();
        writer.flush().unwrap();
        writer.write_all(b"this write pushes us over\n").unwrap();
        writer.flush().unwrap();

        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();

        assert_eq!(entries.len(), 2, "expected original + 1 rotated file, found: {entries:?}");
        assert!(entries.iter().any(|n| n == "audit.ndjson"));
        assert!(
            entries.iter().any(|n| n.starts_with("audit.") && n != "audit.ndjson"),
            "expected a rotated backup file, found: {entries:?}"
        );

        let current_contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(current_contents, "this write pushes us over\n");
    }

    #[test]
    fn test_rotating_writer_preserves_old_content_in_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.ndjson");

        let mut writer = SizeRotatingWriter::open(&path, 10).unwrap();
        writer.write_all(b"first\n").unwrap();
        writer.flush().unwrap();
        writer.write_all(b"second\n").unwrap();
        writer.flush().unwrap();

        let backup = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name().unwrap().to_string_lossy().starts_with("audit.")
                    && p.file_name().unwrap() != "audit.ndjson"
            })
            .expect("expected a rotated backup file");

        let backup_contents = std::fs::read_to_string(&backup).unwrap();
        assert_eq!(backup_contents, "first\n");

        let current_contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(current_contents, "second\n");
    }

    #[test]
    fn test_rotating_writer_never_rotates_an_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.ndjson");

        let mut writer = SizeRotatingWriter::open(&path, 5).unwrap();
        writer.write_all(b"this single line is longer than 5 bytes\n").unwrap();
        writer.flush().unwrap();

        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1, "must not rotate an empty file: {entries:?}");
    }

    #[test]
    fn test_rotating_writer_resumes_existing_file_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.ndjson");

        std::fs::write(&path, b"pre-existing content (20 bytes)\n").unwrap();
        let preexisting_size = std::fs::metadata(&path).unwrap().len();
        assert!(preexisting_size > 0);

        let mut writer = SizeRotatingWriter::open(&path, preexisting_size + 5).unwrap();
        writer.write_all(b"more than 5 extra bytes here\n").unwrap();
        writer.flush().unwrap();

        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(
            entries.len(),
            2,
            "expected rotation on resumed file size, found: {entries:?}"
        );
    }

    #[test]
    fn test_max_size_mb_zero_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.ndjson");

        let mut writer = SizeRotatingWriter::open(&path, 0).unwrap();
        writer.write_all(b"first\n").unwrap();
        writer.flush().unwrap();
        writer.write_all(b"second\n").unwrap();
        writer.flush().unwrap();

        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert!(!entries.is_empty());
    }
}
