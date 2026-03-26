//! Structured audit log for container lifecycle events.
//!
//! Writes append-only NDJSON entries to `$XDG_DATA_HOME/devcont/audit.log`
//! (mode 0o600). Write failures are printed to stderr and do not abort the
//! calling operation.

use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;

use directories::ProjectDirs;
use serde::Serialize;

/// A single auditable lifecycle event.
#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub(crate) enum AuditEvent {
    /// The `start` command was invoked and is beginning container setup.
    ContainerStart {
        /// Derived container name from the devcontainer config.
        container: String,
    },
    /// The `rebuild` command was invoked.
    ContainerRebuild {
        /// Derived container name from the devcontainer config.
        container: String,
    },
    /// The container was stopped at the end of the session (shutdownAction).
    ContainerStop {
        /// Container name.
        container: String,
    },
    /// A lifecycle hook was executed.
    HookExecuted {
        /// Hook name, e.g. `"postCreateCommand"`.
        hook: String,
        /// The command string (or joined argv for array hooks).
        command: String,
    },
}

/// Wrapper that writes one event per call to the audit log.
///
/// When disabled (via `--no-audit-log`) all `log()` calls are no-ops.
pub(crate) struct AuditLogger {
    path: Option<PathBuf>,
}

impl AuditLogger {
    /// Create a logger. When `disabled` is `true`, all writes are suppressed.
    pub fn new(disabled: bool) -> Self {
        if disabled {
            return Self { path: None };
        }
        Self {
            path: audit_log_path(),
        }
    }

    /// Log `event`. Write failures are printed to stderr; they do not propagate.
    pub fn log(&self, event: &AuditEvent) {
        let Some(path) = &self.path else {
            return;
        };
        if let Some(dir) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                tracing::warn!("audit: could not create log directory: {e}");
                return;
            }
        }
        if let Err(e) = append_entry(path, event) {
            tracing::warn!("audit: could not write log entry: {e}");
        }
    }
}

#[derive(Serialize)]
struct LogEntry<'a> {
    timestamp: u64,
    #[serde(flatten)]
    event: &'a AuditEvent,
}

/// Resolve the audit log path via XDG data directory.
fn audit_log_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "devcont").map(|d| d.data_dir().join("audit.log"))
}

/// Append a single NDJSON entry to `path`.
fn append_entry(path: &PathBuf, event: &AuditEvent) -> std::io::Result<()> {
    let ts = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let entry = serde_json::to_string(&LogEntry {
        timestamp: ts,
        event,
    })
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let mut file = open_log_file(path)?;
    writeln!(file, "{entry}")
}

/// Open (or create) the log file in append mode with restricted permissions.
fn open_log_file(path: &PathBuf) -> std::io::Result<std::fs::File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .mode(0o600)
            .open(path)
    }
    #[cfg(not(unix))]
    {
        std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn log_to_temp_file(event: &AuditEvent) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "devcont_audit_test_{}.ndjson",
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let logger = AuditLogger {
            path: Some(path.clone()),
        };
        logger.log(event);
        path
    }

    #[test]
    fn log_entry_is_valid_ndjson() {
        let event = AuditEvent::ContainerStart {
            container: "test-container".to_string(),
        };
        let path = log_to_temp_file(&event);
        let content = std::fs::read_to_string(&path).expect("log file should exist");
        // Each non-empty line must parse as valid JSON
        for line in content.lines().filter(|l| !l.is_empty()) {
            let v: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("log line is not valid JSON: {e}\nline: {line}"));
            assert_eq!(v["event"], "container_start");
            assert_eq!(v["container"], "test-container");
            assert!(v["timestamp"].is_number(), "timestamp must be a number");
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn hook_executed_event_serializes_hook_and_command() {
        let event = AuditEvent::HookExecuted {
            hook: "postCreateCommand".to_string(),
            command: "npm install".to_string(),
        };
        let path = log_to_temp_file(&event);
        let content = std::fs::read_to_string(&path).expect("log file should exist");
        let line = content.lines().next().expect("at least one log line");
        let v: serde_json::Value = serde_json::from_str(line).expect("valid JSON");
        assert_eq!(v["event"], "hook_executed");
        assert_eq!(v["hook"], "postCreateCommand");
        assert_eq!(v["command"], "npm install");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_failure_does_not_panic() {
        // Point the logger at an unwritable path (directory instead of file).
        let tmp = std::env::temp_dir().join("devcont_audit_test_dir_as_file");
        std::fs::create_dir_all(&tmp).ok();
        let logger = AuditLogger {
            path: Some(tmp.clone()),
        };
        // Must not panic; failure is emitted to stderr.
        logger.log(&AuditEvent::ContainerStart {
            container: "x".to_string(),
        });
        let _ = std::fs::remove_dir(&tmp);
    }

    #[test]
    fn disabled_logger_writes_nothing() {
        let logger = AuditLogger::new(true);
        // disabled logger has no path
        assert!(logger.path.is_none());
        // Calling log must not panic
        logger.log(&AuditEvent::ContainerStart {
            container: "x".to_string(),
        });
    }

    #[test]
    #[cfg(unix)]
    fn log_file_has_restricted_mode() {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!(
            "devcont_audit_mode_test_{}.ndjson",
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        append_entry(
            &path,
            &AuditEvent::ContainerStart {
                container: "test".to_string(),
            },
        )
        .expect("append should succeed");
        let meta = std::fs::metadata(&path).expect("file should exist");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "log file must be owner-only (0o600)");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn multiple_events_each_on_own_line() {
        let path = std::env::temp_dir().join(format!(
            "devcont_audit_multi_{}.ndjson",
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let logger = AuditLogger {
            path: Some(path.clone()),
        };
        logger.log(&AuditEvent::ContainerStart {
            container: "c1".to_string(),
        });
        logger.log(&AuditEvent::ContainerStop {
            container: "c1".to_string(),
        });
        let content = std::fs::read_to_string(&path).expect("log file should exist");
        let lines: Vec<_> = content.lines().collect();
        assert_eq!(lines.len(), 2, "two events → two lines");
        let _ = std::fs::remove_file(&path);
    }
}
