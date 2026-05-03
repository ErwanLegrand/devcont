use thiserror::Error;

/// Application-level errors for `devcont`.
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse devcontainer config: {0}")]
    ConfigParse(String),

    #[error("Failed to load settings: {0}")]
    SettingsLoad(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// A file path escapes a required root boundary.
    #[error("Path traversal blocked: {0}")]
    PathTraversal(String),

    /// A lifecycle hook command exited with a non-zero status.
    #[error("Lifecycle hook failed — {0}")]
    HookFailed(String),

    /// A container engine operation failed.
    #[error("Provider error: {0}")]
    ProviderError(String),

    /// The container did not enter the `running` state after `start`.
    ///
    /// Typically caused by a Dockerfile whose main process exits immediately
    /// (e.g., `CMD ["/bin/false"]`). Surfaced by `ensure_up` so callers don't
    /// receive a false-positive success.
    #[error("Container '{0}' is not running after start")]
    ContainerNotRunning(String),

    /// A captured command exited with a non-zero status.
    ///
    /// The [`ExecOutput`](crate::provider::ExecOutput) is preserved so callers
    /// can inspect stdout, stderr, and the exit code even on failure.
    #[error("exec_capture failed (exit code {exit_code}){}", format_truncated_stderr(.stderr))]
    ExecCaptureFailed {
        exit_code: i32,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
}

/// Format the first 200 bytes of stderr for error display, if non-empty.
fn format_truncated_stderr(stderr: &[u8]) -> String {
    if stderr.is_empty() {
        return String::new();
    }
    let lossy = String::from_utf8_lossy(if stderr.len() > 200 {
        &stderr[..200]
    } else {
        stderr
    });
    let suffix = if stderr.len() > 200 { "…" } else { "" };
    format!(": {lossy}{suffix}")
}

impl Error {
    /// If this is an [`ExecCaptureFailed`](Error::ExecCaptureFailed), convert it
    /// into the corresponding [`ExecOutput`](crate::provider::ExecOutput).
    /// Returns `None` for all other variants.
    #[must_use]
    pub fn into_exec_output(self) -> Option<crate::provider::ExecOutput> {
        match self {
            Self::ExecCaptureFailed {
                exit_code,
                stdout,
                stderr,
            } => Some(crate::provider::ExecOutput {
                stdout,
                stderr,
                exit_code,
            }),
            _ => None,
        }
    }
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::Io(io_err) => io_err,
            other => std::io::Error::new(std::io::ErrorKind::Other, other.to_string()),
        }
    }
}

/// Convenience alias for `std::result::Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_capture_failed_display_with_stderr() {
        let err = Error::ExecCaptureFailed {
            exit_code: 1,
            stdout: Vec::new(),
            stderr: b"file not found".to_vec(),
        };
        let msg = err.to_string();
        assert!(msg.contains("exit code 1"), "got: {msg}");
        assert!(msg.contains("file not found"), "got: {msg}");
    }

    #[test]
    fn exec_capture_failed_display_empty_stderr() {
        let err = Error::ExecCaptureFailed {
            exit_code: 127,
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        let msg = err.to_string();
        assert!(msg.contains("exit code 127"), "got: {msg}");
        assert!(
            !msg.contains(':'),
            "should have no colon when stderr empty: {msg}"
        );
    }

    #[test]
    fn exec_capture_failed_display_truncates_long_stderr() {
        let long_stderr = vec![b'x'; 300];
        let err = Error::ExecCaptureFailed {
            exit_code: 2,
            stdout: Vec::new(),
            stderr: long_stderr,
        };
        let msg = err.to_string();
        assert!(msg.contains("exit code 2"), "got: {msg}");
        assert!(msg.contains('…'), "should contain ellipsis: {msg}");
        // The displayed stderr should be roughly 200 chars, not 300
        assert!(
            msg.len() < 260,
            "message too long ({} chars): {msg}",
            msg.len()
        );
    }

    #[test]
    fn into_exec_output_returns_some_for_exec_capture_failed() {
        let err = Error::ExecCaptureFailed {
            exit_code: 42,
            stdout: b"out".to_vec(),
            stderr: b"err".to_vec(),
        };
        let output = err.into_exec_output().expect("should be Some");
        assert_eq!(output.exit_code, 42);
        assert_eq!(output.stdout, b"out");
        assert_eq!(output.stderr, b"err");
    }

    #[test]
    fn into_exec_output_returns_none_for_other_variants() {
        let err = Error::ProviderError("boom".into());
        assert!(err.into_exec_output().is_none());
    }
}
