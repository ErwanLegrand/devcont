/// Apple Container provider (macOS only).
///
/// The module compiles on all platforms so that tests and type references work
/// everywhere. Runtime availability is gated by [`apple::AppleContainer::is_available`],
/// which returns `Ok(false)` on non-macOS hosts.
pub mod apple;
pub mod docker;
pub mod docker_compose;
pub mod nerdctl;
pub mod options;
pub mod podman;
pub mod podman_compose;
pub(crate) mod utils;
use colored::Colorize as _;
use options::ContainerOptions;
use std::io;
pub(crate) const IMAGE_NAMESPACE: &str = "devcont";

/// Abstraction over container engine backends (Docker, Podman, Compose variants).
pub trait Provider {
    /// Build the container image. Pass `cache_enabled = false` to add `--no-cache`.
    ///
    /// # Errors
    /// Fails when the underlying build command cannot be spawned or exits with an error.
    fn build(&self, cache_enabled: bool) -> io::Result<bool>;

    /// Create the container with the given options.
    ///
    /// # Errors
    /// Fails when the underlying create command cannot be spawned or exits with an error.
    fn create(&self, opts: &ContainerOptions) -> io::Result<bool>;

    /// Start a stopped container.
    ///
    /// # Errors
    /// Fails when the underlying start command cannot be spawned or exits with an error.
    fn start(&self) -> io::Result<bool>;

    /// Stop a running container.
    ///
    /// # Errors
    /// Fails when the underlying stop command cannot be spawned or exits with an error.
    fn stop(&self) -> io::Result<bool>;

    /// Restart the container.
    ///
    /// # Errors
    /// Fails when the underlying restart command cannot be spawned or exits with an error.
    fn restart(&self) -> io::Result<bool>;

    /// Attach an interactive shell session to the container.
    ///
    /// # Errors
    /// Fails when the underlying attach command cannot be spawned or exits with an error.
    fn attach(&self) -> io::Result<bool>;

    /// Remove the container.
    ///
    /// # Errors
    /// Fails when the underlying remove command cannot be spawned or exits with an error.
    fn rm(&self) -> io::Result<bool>;

    /// Return `true` if the container exists (running or stopped).
    ///
    /// # Errors
    /// Fails when the underlying inspect command cannot be spawned.
    fn exists(&self) -> io::Result<bool>;

    /// Return `true` if the container is currently running.
    ///
    /// # Errors
    /// Fails when the underlying inspect command cannot be spawned.
    fn running(&self) -> io::Result<bool>;

    /// Copy `source` (host path) into the container at `destination`.
    ///
    /// # Errors
    /// Fails when the underlying copy command cannot be spawned or exits with an error.
    fn cp(&self, source: String, destination: String) -> io::Result<bool>;

    /// Execute a shell command inside the container via `sh -c`.
    ///
    /// Commands from `devcontainer.json` lifecycle hooks are passed here and
    /// are expected to be shell syntax (pipes, redirects, etc. are supported).
    /// Callers constructing commands programmatically should shell-quote any
    /// path arguments to prevent word-splitting.
    ///
    /// # Errors
    /// Returns an error if the underlying exec command fails to spawn or exits with a non-zero
    /// status. The exit code is included in the error message.
    fn exec(&self, cmd: String) -> io::Result<()>;

    /// Execute a program directly inside the container without a shell wrapper.
    ///
    /// `prog` is the executable to run; `args` are its arguments passed as
    /// separate tokens (no shell interpretation, no word-splitting, no injection).
    /// Use this for the array form of lifecycle hooks (`Many` variant).
    ///
    /// # Errors
    /// Returns an error if the underlying exec command fails to spawn or exits with a non-zero
    /// status. The exit code is included in the error message.
    fn exec_raw(&self, prog: &str, args: &[&str]) -> io::Result<()>;

    /// Execute a shell command inside the container and capture its output.
    ///
    /// Unlike [`exec`](Provider::exec), which inherits the terminal, this method
    /// captures stdout and stderr as raw bytes and returns them in an
    /// [`ExecOutput`]. The command is run via `sh -c`, same as `exec`.
    ///
    /// # Returns
    ///
    /// - `Ok(ExecOutput)` when the command exits with code 0.
    /// - `Err(Error::ExecCaptureFailed { .. })` when the command exits with a
    ///   non-zero code. The stdout, stderr, and exit code are preserved in the
    ///   error so callers can inspect them via [`crate::error::Error::into_exec_output`].
    /// - `Err(Error::Io(..))` on spawn or pipe failure.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails to spawn or exits with a non-zero
    /// status.
    fn exec_capture(&self, shell_cmd: &str) -> crate::error::Result<ExecOutput>;
} // trait Provider
/// Raw output captured from a container command execution.
///
/// `stdout` and `stderr` are raw bytes matching [`std::process::Output`].
/// `exit_code` is the process exit code; on Unix signal termination it is
/// `128 + signal_number`.
pub struct ExecOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
}

impl ExecOutput {
    /// Return stdout decoded as a lossy UTF-8 string.
    #[must_use]
    pub fn stdout_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    /// Return stderr decoded as a lossy UTF-8 string.
    #[must_use]
    pub fn stderr_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

/// Redact the values of `--env` / `-e` arguments in a list of command-line tokens.
///
/// For two-token forms (`--env KEY=VALUE` or `-e KEY=VALUE`), the token following
/// the flag is replaced with `KEY=***`. For single-token forms (`--env=KEY=VALUE`),
/// the value after `=` is replaced with `***`.
pub(crate) fn redact_env_args(args: &[&str]) -> Vec<String> {
    let mut redacted: Vec<String> = Vec::with_capacity(args.len());
    let mut redact_next = false;

    for arg in args {
        if redact_next {
            // Previous arg was --env or -e; redact value in KEY=VALUE.
            let display = if let Some(eq) = arg.find('=') {
                format!("{}=***", &arg[..eq])
            } else {
                "***".to_string()
            };
            redacted.push(display);
            redact_next = false;
        } else if *arg == "--env" || *arg == "-e" {
            redacted.push((*arg).to_string());
            redact_next = true;
        } else if let Some(suffix) = arg.strip_prefix("--env=") {
            // --env=KEY=VALUE single-token form.
            let display = if let Some(eq) = suffix.find('=') {
                format!("--env={}=***", &suffix[..eq])
            } else {
                format!("--env={suffix}")
            };
            redacted.push(display);
        } else if let Some(suffix) = arg.strip_prefix("-e=") {
            let display = if let Some(eq) = suffix.find('=') {
                format!("-e={}=***", &suffix[..eq])
            } else {
                format!("-e={suffix}")
            };
            redacted.push(display);
        } else {
            redacted.push((*arg).to_string());
        }
    }

    redacted
}

/// Convert a [`std::process::Output`] into an [`ExecOutput`], returning
/// `Ok` on exit code 0 and [`crate::error::Error::ExecCaptureFailed`] otherwise.
///
/// On Unix, if the process was killed by a signal, the exit code is
/// `128 + signal_number` (standard shell convention).
pub(crate) fn output_to_exec_result(
    output: std::process::Output,
) -> crate::error::Result<ExecOutput> {
    #[cfg(unix)]
    let exit_code = {
        use std::os::unix::process::ExitStatusExt;
        output
            .status
            .code()
            .unwrap_or_else(|| 128 + output.status.signal().unwrap_or(0))
    };
    #[cfg(not(unix))]
    let exit_code = output.status.code().unwrap_or(-1);

    if output.status.success() {
        Ok(ExecOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code,
        })
    } else {
        Err(crate::error::Error::ExecCaptureFailed {
            exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

/// Print a formatted, redacted command line to stdout.
///
/// `--env KEY=VALUE` values are replaced with `KEY=***` to avoid leaking secrets in terminal
/// output. The actual subprocess arguments are not modified.
pub(crate) fn print_command(cmd: &std::process::Command) {
    let binary = cmd.get_program();
    let tokens: Vec<&str> = cmd
        .get_args()
        .map(|a| a.to_str().unwrap_or("<non-utf8>"))
        .collect();
    let sanitized_args = redact_env_args(&tokens);
    let line = format!(
        "{} {}",
        binary.to_str().unwrap_or("<non-utf8>"),
        sanitized_args.join(" ")
    );
    println!("{}", line.bold().blue());
} // print_command

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_two_token_env() {
        let args = vec!["run", "-e", "SECRET=hunter2", "--name", "c"];
        let result = redact_env_args(&args);
        assert_eq!(result, vec!["run", "-e", "SECRET=***", "--name", "c"]);
    }

    #[test]
    fn redact_two_token_env_long() {
        let args = vec!["run", "--env", "MY_TOKEN=abc123"];
        let result = redact_env_args(&args);
        assert_eq!(result, vec!["run", "--env", "MY_TOKEN=***"]);
    }

    #[test]
    fn redact_single_token_env_equals() {
        let args = vec!["run", "--env=MY_TOKEN=abc123"];
        let result = redact_env_args(&args);
        assert_eq!(result, vec!["run", "--env=MY_TOKEN=***"]);
    }

    #[test]
    fn non_env_args_unchanged() {
        let args = vec!["run", "--name", "foo", "--network", "host"];
        let result = redact_env_args(&args);
        assert_eq!(result, vec!["run", "--name", "foo", "--network", "host"]);
    }

    #[test]
    fn multiple_env_args_all_redacted() {
        let args = vec!["run", "-e", "A=1", "-e", "B=2"];
        let result = redact_env_args(&args);
        assert_eq!(result, vec!["run", "-e", "A=***", "-e", "B=***"]);
    }

    #[test]
    fn exec_output_stdout_lossy_valid_utf8() {
        let output = ExecOutput {
            stdout: b"hello world\n".to_vec(),
            stderr: Vec::new(),
            exit_code: 0,
        };
        assert_eq!(output.stdout_lossy(), "hello world\n");
    }

    #[test]
    fn exec_output_stderr_lossy_valid_utf8() {
        let output = ExecOutput {
            stdout: Vec::new(),
            stderr: b"error: something broke".to_vec(),
            exit_code: 1,
        };
        assert_eq!(output.stderr_lossy(), "error: something broke");
    }

    #[test]
    fn exec_output_stdout_lossy_invalid_utf8() {
        let output = ExecOutput {
            stdout: vec![0xFF, 0xFE, b'o', b'k'],
            stderr: Vec::new(),
            exit_code: 0,
        };
        let result = output.stdout_lossy();
        assert!(result.contains("ok"));
        assert!(result.contains('\u{FFFD}'));
    }

    #[test]
    fn exec_output_stderr_lossy_invalid_utf8() {
        let output = ExecOutput {
            stdout: Vec::new(),
            stderr: vec![b'e', b'r', b'r', 0xFF],
            exit_code: 2,
        };
        let result = output.stderr_lossy();
        assert!(result.starts_with("err"));
        assert!(result.contains('\u{FFFD}'));
    }

    #[test]
    fn exec_output_empty_streams() {
        let output = ExecOutput {
            stdout: Vec::new(),
            stderr: Vec::new(),
            exit_code: 0,
        };
        assert_eq!(output.stdout_lossy(), "");
        assert_eq!(output.stderr_lossy(), "");
    }
}
