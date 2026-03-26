use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::io::Result;
use std::path::PathBuf;
use std::process::Command;
use tinytemplate::TinyTemplate;

use super::print_command;

/// Returns true if any name in `output` (newline-separated) exactly matches `name`.
///
/// Container CLI commands like `docker ps --format {{.Names}}` produce
/// newline-delimited output; this function guards against substring matches
/// (e.g. `my-ctr` must not match `my-ctr-2`).
pub(crate) fn exact_name_match(output: Vec<u8>, name: &str) -> bool {
    String::from_utf8(output)
        .unwrap_or_default()
        .lines()
        .any(|line| line.trim() == name)
}

/// Check whether a container exists or is running by invoking a container
/// CLI's `ps` subcommand.
///
/// This is the shared implementation behind `exists()` and `running()` for
/// Docker, Nerdctl, Apple Container, and Podman.
///
/// * `include_stopped` — when `true` the `-a` flag is added so that stopped
///   containers are included (used by `exists()`).  When `false` only running
///   containers are listed (used by `running()`).
///
/// # Errors
///
/// Returns an error if the command fails to spawn or the output cannot be read.
pub(crate) fn check_container_status(
    command: &mut Command,
    name: &str,
    include_stopped: bool,
) -> std::io::Result<bool> {
    command.arg("ps");
    if include_stopped {
        command.arg("-a");
    }
    command
        .arg("--filter")
        .arg(format!("name={name}"))
        .arg("--format")
        .arg("{{.Names}}");

    let output = command.output()?.stdout;
    Ok(exact_name_match(output, name))
}

/// Forward the host's SSH agent socket into the container.
///
/// When `SSH_AUTH_SOCK` is set, adds `--volume <sock>:/ssh-agent` and
/// `--env SSH_AUTH_SOCK=/ssh-agent` to the command.
///
/// All four direct providers (Docker, Podman, Nerdctl, Apple) use
/// identical forwarding — no provider-specific suffixes (e.g. `:z`) are
/// applied here; `SELinux` relabelling is only relevant for Compose
/// overrides (see [`create_compose_override`]).
pub(crate) fn inject_ssh_agent(command: &mut Command) {
    if let Ok(ssh_auth_sock) = env::var("SSH_AUTH_SOCK") {
        command.arg("--volume");
        command.arg(format!("{ssh_auth_sock}:/ssh-agent"));
        command.arg("--env");
        command.arg("SSH_AUTH_SOCK=/ssh-agent");
    }
}

/// Append the common container-create arguments shared by all direct providers.
///
/// This consolidates port forwarding, environment variable injection, extra
/// run-args, additional mounts, the container identity flags (`-it`, `--name`,
/// `-u`, `-w`), the image reference, and the optional sleep-loop override
/// command that every direct provider (`docker create`, `podman create`, etc.)
/// emits in the same order.
///
/// The override command (sleep loop) is placed after the image argument because
/// it is a positional argument to the container entrypoint.
///
/// # Arguments
///
/// * `command`          - The partially-built `Command` (already has `create`,
///                        workspace mount, SSH injection, and any
///                        provider-specific flags like `--userns`).
/// * `forward_ports`    - Ports to publish (`--publish <port>:<port>`).
/// * `remote_env`       - Environment variables from `ContainerOptions`.
/// * `run_args`         - Extra CLI arguments from `devcontainer.json`.
/// * `mounts`           - Additional bind/volume mounts.
/// * `name`             - Container name.
/// * `user`             - User to run as inside the container.
/// * `workspace_folder` - Working directory inside the container.
/// * `image`            - Image reference (tag or name).
/// * `override_command` - When `true`, append the sleep-loop entrypoint.
/// * `use_bin_sh`       - `true` for Docker (`/bin/sh`), `false` for others
///                        (`sh`).  Docker uses the absolute path because the
///                        Docker runtime guarantees `/bin/sh` exists.  Other
///                        providers use bare `sh` for better compatibility
///                        with rootless/minimal containers where `/bin` may
///                        not be on `PATH`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_common_create_args(
    command: &mut Command,
    forward_ports: &[u16],
    remote_env: &[(String, String)],
    run_args: &[String],
    mounts: Option<&Vec<HashMap<String, String>>>,
    name: &str,
    user: &str,
    workspace_folder: &str,
    image: &str,
    override_command: bool,
    use_bin_sh: bool,
) {
    for port in forward_ports {
        command.arg("--publish").arg(format!("{port}:{port}"));
    }

    for (key, value) in remote_env {
        command.arg("--env").arg(format!("{key}={value}"));
    }

    for arg in run_args {
        command.arg(arg);
    }

    if let Some(mounts) = mounts {
        for mount in mounts {
            command.arg("--mount");
            let m = mount
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<String>>()
                .join(",");
            command.arg(m);
        }
    }

    command.arg("-it");
    command.arg("--name").arg(name);
    command.arg("-u").arg(user);
    command.arg("-w").arg(workspace_folder);
    command.arg(image);

    if override_command {
        let shell = if use_bin_sh { "/bin/sh" } else { "sh" };
        command
            .arg(shell)
            .arg("-c")
            .arg("while sleep 1000; do :; done");
    }
}

/// Print a command and run it, returning whether it exited successfully.
///
/// This is the pattern used by `start()`, `stop()`, `restart()`, `attach()`,
/// `rm()`, `build()`, `create()`, and `cp()` across all direct providers.
///
/// # Errors
///
/// Returns an error if the command fails to spawn.
pub(crate) fn run_and_check(command: &mut Command) -> Result<bool> {
    print_command(command);
    Ok(command.status()?.success())
}

#[derive(Serialize, Debug)]
struct TemplateContext {
    service: String,
    envs: Vec<TemplateEntry>,
    volumes: Vec<TemplateEntry>,
    build_args: Vec<TemplateEntry>,
}

#[derive(Serialize, Debug)]
struct TemplateEntry {
    source: String,
    dest: String,
    suffix: String,
}

/// Return `true` if `SELinux` is currently in enforcing mode.
///
/// Reads `/sys/fs/selinux/enforce`; any error (file absent, unreadable)
/// is treated as "not enforcing".
pub(crate) fn selinux_enforcing() -> bool {
    std::fs::read_to_string("/sys/fs/selinux/enforce")
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}

static TEMPLATE: &str = include_str!("../../templates/docker-compose.yml");

/// RAII guard that deletes the compose override file when dropped.
pub(crate) struct ComposeOverrideGuard(pub(crate) PathBuf);

impl Drop for ComposeOverrideGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Write a temporary docker-compose override file that forwards the SSH agent
/// socket into the named `service` container and injects any additional
/// environment variables, returning a guard that holds the path and deletes
/// the file on drop.
///
/// When `selinux_relabel` is `true`, a `:z` label is appended to the SSH socket
/// volume mount so that `SELinux` allows the container to access the socket.
///
/// The file is created with mode 0o600 (owner read/write only).
///
/// # Errors
/// Returns an error if the template cannot be rendered or the file cannot be written.
pub(crate) fn create_compose_override(
    service: &str,
    env_vars: &[(String, String)],
    selinux_relabel: bool,
    build_args: &HashMap<String, String>,
) -> Result<ComposeOverrideGuard> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::fs::PermissionsExt;

    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let dir = env::temp_dir();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = dir.join(format!("docker-compose-{}-{count}.yml", std::process::id()));
    let mut volumes = vec![];
    let mut envs: Vec<TemplateEntry> = env_vars
        .iter()
        .map(|(k, v)| TemplateEntry {
            source: k.clone(),
            dest: v.clone(),
            suffix: String::new(),
        })
        .collect();

    if let Ok(ssh_auth_sock) = env::var("SSH_AUTH_SOCK") {
        let volume_suffix = if selinux_relabel {
            ":z".to_string()
        } else {
            String::new()
        };
        volumes.push(TemplateEntry {
            source: ssh_auth_sock,
            dest: "/ssh-agent".to_string(),
            suffix: volume_suffix,
        });
        envs.push(TemplateEntry {
            source: "SSH_AUTH_SOCK".to_string(),
            dest: "/ssh-agent".to_string(),
            suffix: String::new(),
        });
    }

    let build_args_entries: Vec<TemplateEntry> = build_args
        .iter()
        .map(|(k, v)| TemplateEntry {
            source: k.clone(),
            dest: v.clone(),
            suffix: String::new(),
        })
        .collect();

    let context = TemplateContext {
        service: service.to_string(),
        envs,
        volumes,
        build_args: build_args_entries,
    };

    let mut tt = TinyTemplate::new();
    tt.add_template("docker-compose.yml", TEMPLATE)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let rendered = tt
        .render("docker-compose.yml", &context)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)?;
    file.write_all(rendered.as_bytes())?;
    drop(file);

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;

    Ok(ComposeOverrideGuard(path))
}

/// Resolve the Dockerfile path from config, relative to `context` if provided,
/// otherwise relative to `workspace`.
///
/// Absolute paths are returned unchanged.
///
/// # Errors
/// Returns an error if the resulting path would escape the workspace root (validated
/// by the caller, e.g. via `validate_within_root`).
pub(crate) fn resolve_dockerfile_path(
    workspace: &std::path::Path,
    dockerfile: &str,
    context: Option<&str>,
) -> std::path::PathBuf {
    let dockerfile_path = std::path::Path::new(dockerfile);
    if dockerfile_path.is_absolute() {
        return dockerfile_path.to_path_buf();
    }
    let base = if let Some(ctx) = context {
        let ctx_path = std::path::Path::new(ctx);
        if ctx_path.is_absolute() {
            ctx_path.to_path_buf()
        } else {
            workspace.join(ctx_path)
        }
    } else {
        workspace.to_path_buf()
    };
    base.join(dockerfile_path)
}

/// Format a human-friendly error message from an exec failure.
///
/// Inspects `stderr` for common error patterns (image not found, permission
/// denied, daemon not running, container already exists) and returns a
/// descriptive message.  Falls back to a generic "Exec failed with exit
/// code N" when no pattern matches or `stderr` is empty.
pub(crate) fn format_exec_error(exit_code: i32, stderr: &str) -> String {
    let lower = stderr.to_lowercase();
    if lower.contains("no such image") || lower.contains("image not known") {
        format!(
            "Image not found: {}",
            stderr.lines().next().unwrap_or("unknown")
        )
    } else if lower.contains("permission denied") || lower.contains("not enough permissions") {
        format!(
            "Permission denied: {}",
            stderr.lines().next().unwrap_or("unknown")
        )
    } else if lower.contains("cannot connect")
        || lower.contains("connection refused")
        || lower.contains("daemon is not running")
        || (lower.contains("no such file or directory") && lower.contains(".sock"))
    {
        "Container daemon is not running — check that your container runtime is started".to_string()
    } else if lower.contains("container already exists") {
        format!(
            "Container already exists: {}",
            stderr.lines().next().unwrap_or("unknown")
        )
    } else {
        format!("Exec failed with exit code {exit_code}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn resolve_dockerfile_no_context_relative_to_workspace() {
        let ws = std::path::Path::new("/ws");
        let result = resolve_dockerfile_path(ws, "Dockerfile", None);
        assert_eq!(result, std::path::PathBuf::from("/ws/Dockerfile"));
    }

    #[test]
    fn resolve_dockerfile_with_relative_context() {
        let ws = std::path::Path::new("/ws");
        let result = resolve_dockerfile_path(ws, "Dockerfile", Some("subdir"));
        assert_eq!(result, std::path::PathBuf::from("/ws/subdir/Dockerfile"));
    }

    #[test]
    fn resolve_dockerfile_with_absolute_context() {
        let ws = std::path::Path::new("/ws");
        let result = resolve_dockerfile_path(ws, "Dockerfile", Some("/other/ctx"));
        assert_eq!(result, std::path::PathBuf::from("/other/ctx/Dockerfile"));
    }

    #[test]
    fn resolve_dockerfile_absolute_path_returned_unchanged() {
        let ws = std::path::Path::new("/ws");
        let result = resolve_dockerfile_path(ws, "/abs/Dockerfile", Some("ctx"));
        assert_eq!(result, std::path::PathBuf::from("/abs/Dockerfile"));
    }

    #[test]
    fn compose_override_file_has_mode_0o600() {
        let guard = create_compose_override("test-service", &[], false, &HashMap::new())
            .expect("create_compose_override should succeed");
        let mode = std::fs::metadata(&guard.0)
            .expect("metadata should be readable")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "expected 0o600, got 0o{mode:o}");
    }

    #[test]
    fn compose_override_file_removed_after_guard_drop() {
        let path = {
            let guard = create_compose_override("test-service", &[], false, &HashMap::new())
                .expect("create_compose_override should succeed");
            guard.0.clone()
        };
        assert!(
            !path.exists(),
            "file should be deleted after guard is dropped"
        );
    }

    #[test]
    fn compose_override_with_build_args_renders_args_section() {
        let mut build_args = HashMap::new();
        build_args.insert("FOO".to_string(), "bar".to_string());
        let guard = create_compose_override("svc", &[], false, &build_args)
            .expect("create_compose_override should succeed");
        let content = std::fs::read_to_string(&guard.0).expect("file should be readable");
        assert!(
            content.contains("FOO"),
            "override should contain build arg key 'FOO', got:\n{content}"
        );
        assert!(
            content.contains("bar"),
            "override should contain build arg value 'bar', got:\n{content}"
        );
        assert!(
            content.contains("build:"),
            "override should contain 'build:' section, got:\n{content}"
        );
    }

    #[test]
    fn compose_override_without_build_args_omits_build_section() {
        let guard = create_compose_override("svc", &[], false, &HashMap::new())
            .expect("create_compose_override should succeed");
        let content = std::fs::read_to_string(&guard.0).expect("file should be readable");
        assert!(
            !content.contains("build:"),
            "override without build_args should not contain 'build:' section, got:\n{content}"
        );
    }

    #[test]
    fn selinux_enforcing_returns_false_when_file_absent() {
        // On non-`SELinux` systems /sys/fs/selinux/enforce does not exist.
        // The function must return false rather than panic.
        // (On `SELinux` systems that are enforcing this test would return true —
        //  we only assert it doesn't panic.)
        let _ = selinux_enforcing();
    }

    #[test]
    fn compose_override_without_selinux_has_no_z_suffix() {
        let guard = create_compose_override("svc", &[], false, &HashMap::new())
            .expect("create_compose_override should succeed");
        let content = std::fs::read_to_string(&guard.0).expect("file should be readable");
        assert!(
            !content.contains(":z"),
            "non-`SELinux` override should not contain ':z', got:\n{content}"
        );
    }

    #[test]
    fn compose_override_with_selinux_has_z_suffix() {
        // SSH_AUTH_SOCK must be set for the volume entry to appear.
        let guard = std::env::var("SSH_AUTH_SOCK").ok().map(|sock| {
            let _ = sock; // use value
        });
        // Only run the assertion when SSH_AUTH_SOCK is set.
        if std::env::var("SSH_AUTH_SOCK").is_ok() {
            let file_guard = create_compose_override("svc", &[], true, &HashMap::new())
                .expect("create_compose_override should succeed");
            let content = std::fs::read_to_string(&file_guard.0).expect("file should be readable");
            assert!(
                content.contains(":z"),
                "`SELinux` override should contain ':z', got:\n{content}"
            );
        }
        let _ = guard;
    }

    // --- format_exec_error ---

    #[test]
    fn format_exec_error_image_not_found() {
        let msg = format_exec_error(125, "Error: no such image: alpine:nonexistent");
        assert!(msg.starts_with("Image not found:"), "got: {msg}");
    }

    #[test]
    fn format_exec_error_image_not_known() {
        let msg = format_exec_error(125, "Error: image not known");
        assert!(msg.starts_with("Image not found:"), "got: {msg}");
    }

    #[test]
    fn format_exec_error_permission_denied() {
        let msg = format_exec_error(126, "Error: permission denied");
        assert!(msg.contains("Permission denied"), "got: {msg}");
    }

    #[test]
    fn format_exec_error_not_enough_permissions() {
        let msg = format_exec_error(126, "Error: not enough permissions to access resource");
        assert!(msg.contains("Permission denied"), "got: {msg}");
    }

    #[test]
    fn format_exec_error_cannot_connect() {
        let msg = format_exec_error(
            125,
            "Cannot connect to Podman. Is the Podman machine running?",
        );
        assert!(msg.contains("daemon is not running"), "got: {msg}");
    }

    #[test]
    fn format_exec_error_connection_refused() {
        let msg = format_exec_error(1, "connection refused");
        assert!(msg.contains("daemon is not running"), "got: {msg}");
    }

    #[test]
    fn format_exec_error_daemon_not_running() {
        let msg = format_exec_error(
            1,
            "Cannot connect to the Docker daemon. Is the docker daemon running on this host?",
        );
        assert!(msg.contains("daemon is not running"), "got: {msg}");
    }

    #[test]
    fn format_exec_error_socket_not_found() {
        let msg = format_exec_error(
            1,
            "Error: no such file or directory: /run/podman/podman.sock",
        );
        assert!(msg.contains("daemon is not running"), "got: {msg}");
    }

    #[test]
    fn format_exec_error_container_exists() {
        let msg = format_exec_error(125, "Error: container already exists: my-ctr");
        assert!(msg.starts_with("Container already exists:"), "got: {msg}");
    }

    #[test]
    fn format_exec_error_fallback() {
        let msg = format_exec_error(1, "some unknown error");
        assert_eq!(msg, "Exec failed with exit code 1");
    }

    #[test]
    fn format_exec_error_empty_stderr() {
        let msg = format_exec_error(42, "");
        assert_eq!(msg, "Exec failed with exit code 42");
    }

    // --- inject_ssh_agent ---

    #[test]
    fn inject_ssh_agent_adds_volume_and_env_when_set() {
        temp_env::with_var("SSH_AUTH_SOCK", Some("/tmp/test-ssh-sock"), || {
            let mut cmd = Command::new("test");
            inject_ssh_agent(&mut cmd);

            let args: Vec<String> = cmd
                .get_args()
                .map(|a| a.to_string_lossy().to_string())
                .collect();
            assert!(args.contains(&"--volume".to_string()));
            assert!(args.contains(&"/tmp/test-ssh-sock:/ssh-agent".to_string()));
            assert!(args.contains(&"--env".to_string()));
            assert!(args.contains(&"SSH_AUTH_SOCK=/ssh-agent".to_string()));
        });
    }

    #[test]
    fn inject_ssh_agent_noop_when_unset() {
        temp_env::with_var_unset("SSH_AUTH_SOCK", || {
            let mut cmd = Command::new("test");
            inject_ssh_agent(&mut cmd);

            let args: Vec<String> = cmd
                .get_args()
                .map(|a| a.to_string_lossy().to_string())
                .collect();
            assert!(
                args.is_empty(),
                "No args should be added when SSH_AUTH_SOCK is unset"
            );
        });
    }

    // --- apply_common_create_args ---

    #[test]
    fn apply_common_create_args_adds_ports_env_runargs() {
        let mut cmd = Command::new("test");
        let env_vars = vec![("FOO".to_string(), "bar".to_string())];
        let run_args = vec!["--net=host".to_string()];
        apply_common_create_args(
            &mut cmd,
            &[8080, 3000],
            &env_vars,
            &run_args,
            None,
            "ctr",
            "dev",
            "/work",
            "alpine",
            false,
            false,
        );
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"8080:8080".to_string()));
        assert!(args.contains(&"3000:3000".to_string()));
        assert!(args.contains(&"FOO=bar".to_string()));
        assert!(args.contains(&"--net=host".to_string()));
        assert!(args.contains(&"-it".to_string()));
        assert!(args.contains(&"ctr".to_string()));
        assert!(args.contains(&"dev".to_string()));
        assert!(args.contains(&"/work".to_string()));
        assert!(args.contains(&"alpine".to_string()));
        // No override -- sleep loop shell should not appear after image
        assert!(!args.contains(&"while sleep 1000; do :; done".to_string()));
    }

    #[test]
    fn apply_common_create_args_override_uses_bin_sh() {
        let mut cmd = Command::new("test");
        apply_common_create_args(
            &mut cmd,
            &[],
            &[],
            &[],
            None,
            "c",
            "u",
            "/w",
            "img",
            true,
            true,
        );
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"/bin/sh".to_string()));
        assert!(args.contains(&"while sleep 1000; do :; done".to_string()));
        // /bin/sh must come after the image
        let img_pos = args.iter().position(|a| a == "img").unwrap();
        let sh_pos = args.iter().position(|a| a == "/bin/sh").unwrap();
        assert!(sh_pos > img_pos, "override shell must follow image arg");
    }

    #[test]
    fn apply_common_create_args_override_uses_bare_sh() {
        let mut cmd = Command::new("test");
        apply_common_create_args(
            &mut cmd,
            &[],
            &[],
            &[],
            None,
            "c",
            "u",
            "/w",
            "img",
            true,
            false,
        );
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        // bare `sh` appears as override shell (after image)
        let img_pos = args.iter().position(|a| a == "img").unwrap();
        let sh_pos = args.iter().rposition(|a| a == "sh").unwrap();
        assert!(sh_pos > img_pos, "override shell must follow image arg");
        assert!(!args.contains(&"/bin/sh".to_string()));
        assert!(args.contains(&"while sleep 1000; do :; done".to_string()));
    }

    #[test]
    fn apply_common_create_args_with_mounts() {
        let mut cmd = Command::new("test");
        let mounts = vec![HashMap::from([
            ("type".to_string(), "bind".to_string()),
            ("source".to_string(), "/host".to_string()),
            ("target".to_string(), "/ctr".to_string()),
        ])];
        apply_common_create_args(
            &mut cmd,
            &[],
            &[],
            &[],
            Some(&mounts),
            "c",
            "u",
            "/w",
            "img",
            false,
            false,
        );
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"--mount".to_string()));
        // The mount string is a comma-separated list of k=v pairs
        assert!(args.iter().any(|a| a.contains("type=bind")));
    }

    // --- run_and_check ---

    #[test]
    fn run_and_check_returns_true_on_success() {
        let mut cmd = Command::new("true");
        let result = run_and_check(&mut cmd);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn run_and_check_returns_false_on_failure() {
        let mut cmd = Command::new("false");
        let result = run_and_check(&mut cmd);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // --- exact_name_match ---

    #[test]
    fn exact_name_match_matches_exact() {
        assert!(exact_name_match(b"foo\n".to_vec(), "foo"));
    }

    #[test]
    fn exact_name_match_no_prefix_match() {
        assert!(!exact_name_match(b"foobar\n".to_vec(), "foo"));
    }

    #[test]
    fn exact_name_match_no_suffix_match() {
        assert!(!exact_name_match(b"barfoo\n".to_vec(), "foo"));
    }

    #[test]
    fn exact_name_match_empty_output() {
        assert!(!exact_name_match(vec![], "foo"));
    }

    #[test]
    fn exact_name_match_multiple_names_one_matches() {
        assert!(exact_name_match(b"foobar\nfoo\nbaz\n".to_vec(), "foo"));
    }

    #[test]
    fn exact_name_match_multiple_names_none_match() {
        assert!(!exact_name_match(b"foobar\nbaz\n".to_vec(), "foo"));
    }

    #[test]
    fn exact_name_match_trims_whitespace() {
        assert!(exact_name_match(b"  foo  \n".to_vec(), "foo"));
    }

    // --- check_container_status ---

    /// Helper: build a `Command` that ignores all arguments and prints
    /// a fixed string to stdout.  This simulates a container CLI whose
    /// `ps --format {{.Names}}` output contains the given names.
    fn fake_ps_command(stdout_content: &str) -> Command {
        let mut cmd = Command::new("sh");
        // `sh -c 'printf ...'` ignores any extra positional args that
        // check_container_status appends (ps, -a, --filter, etc.).
        cmd.arg("-c").arg(format!("printf '{stdout_content}'"));
        cmd
    }

    #[test]
    fn check_container_status_returns_true_when_name_matches() {
        let mut cmd = fake_ps_command("my-ctr\n");
        let result = check_container_status(&mut cmd, "my-ctr", true);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn check_container_status_returns_false_when_name_absent() {
        let mut cmd = fake_ps_command("other-ctr\n");
        let result = check_container_status(&mut cmd, "my-ctr", true);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn check_container_status_returns_false_on_empty_output() {
        let mut cmd = fake_ps_command("");
        let result = check_container_status(&mut cmd, "my-ctr", false);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn check_container_status_rejects_prefix_match() {
        let mut cmd = fake_ps_command("my-ctr-2\n");
        let result = check_container_status(&mut cmd, "my-ctr", true);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
