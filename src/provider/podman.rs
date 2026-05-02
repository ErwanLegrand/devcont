use super::options::ContainerOptions;
use super::print_command;
use super::utils::{
    apply_common_create_args, check_container_status, inject_ssh_agent, run_step,
};
use super::{ExecOutput, IMAGE_NAMESPACE, Provider, output_to_exec_result};
use crate::provider::docker::BuildSource;
use std::{collections::HashMap, io, process::Command};
/// Allowed values for the `--userns` flag.
const VALID_USERNS_MODES: &[&str] = &["keep-id", "host", "auto", "private", ""];

/// Podman provider -- uses the `podman` CLI to manage dev containers.
#[derive(Debug)] // Podman
pub struct Podman {
    // NOLINT(too_many_fields)
    /// Build-time `--build-arg` key/value pairs passed to `podman build`.
    pub(crate) build_args: HashMap<String, String>,
    /// Path sent to the Podman daemon as the build context.
    /// Defaults to the workspace root; overridden by `build.context` in devcontainer.json.
    pub(crate) build_context: String,
    /// Image source -- either a Dockerfile path or a pre-built image name.
    pub(crate) build_source: BuildSource,
    /// CLI binary name (typically `"podman"`).
    pub(crate) command: String,
    /// Host directory containing the project workspace.
    pub(crate) directory: String,
    /// Ports to forward from the container to the host.
    pub(crate) forward_ports: Vec<u16>,
    /// Additional bind/volume mounts from devcontainer.json.
    pub(crate) mounts: Option<Vec<HashMap<String, String>>>,
    /// Container name used for lifecycle commands.
    pub(crate) name: String,
    /// Additional arguments passed to `podman create`.
    pub(crate) run_args: Vec<String>,
    /// When `true`, override the container entrypoint with a sleep loop.
    pub(crate) override_command: bool,
    /// User to run commands as inside the container.
    pub(crate) user: String,
    /// Path inside the container where the workspace is mounted.
    pub(crate) workspace_folder: String,
    /// User namespace mode for Podman containers.
    /// Defaults to "keep-id" for better security in rootless mode.
    /// Must be one of: "keep-id", "host", "auto", "private", or empty.
    pub(crate) userns_mode: String,
    /// Whether to disable `SELinux` labeling.
    ///
    /// Defaults to `true` for better compatibility, especially in rootless mode.
    pub(crate) disable_selinux: bool,
} // Podman

impl Podman {
    /// Create a new Podman provider instance.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        build_args: HashMap<String, String>,
        build_context: String,
        build_source: BuildSource,
        command: String,
        directory: String,
        forward_ports: Vec<u16>,
        name: String,
        run_args: Vec<String>,
        mounts: Option<Vec<HashMap<String, String>>>,
        user: String,
        workspace_folder: String,
        override_command: bool,
        userns_mode: String,
        disable_selinux: bool,
    ) -> Self {
        Self {
            build_args,
            build_context,
            build_source,
            command,
            directory,
            forward_ports,
            mounts,
            name,
            run_args,
            override_command,
            user,
            workspace_folder,
            userns_mode,
            disable_selinux,
        }
    } // new
    /// Check if the Podman CLI is available and the service is running.
    ///
    /// Returns `Ok(true)` if available, `Ok(false)` if not available, or `Err` if detection fails.
    ///
    /// # Errors
    /// Returns `Err` if the podman command execution fails or if there are permission issues.
    pub fn is_available() -> io::Result<bool> {
        // Check if the podman command exists and is executable
        let ver = Command::new("podman").arg("--version").output();
        match ver {
            Ok(out) if out.status.success() => {
                // Podman CLI exists, check if the service/daemon is accessible
                // Try a simple command that requires the daemon to be running
                let daemon_check = Command::new("podman").arg("info").output();
                Ok(daemon_check.as_ref().is_ok_and(|o| o.status.success()))
            }
            _ => Ok(false), // Podman CLI not available or not executable
        }
    } // is_available
    /// Check if Podman is running in rootless mode.
    ///
    /// Returns `true` if rootless, `false` if rootful or if detection fails.
    pub(crate) fn is_rootless() -> bool {
        let info_out = Command::new("podman")
            .arg("info")
            .arg("--format")
            .arg("{{.Host.Security.Rootless}}")
            .output();
        match info_out {
            Ok(out) if out.status.success() => {
                let val = String::from_utf8_lossy(&out.stdout);
                val.trim() == "true"
            }
            // Conservative fallback: assume rootful when detection fails
            _ => false,
        }
    } // is_rootless
    /// Get the appropriate Podman socket path based on rootless detection.
    ///
    /// Returns `Some(path)` when a rootless socket exists, `None` otherwise.
    pub(crate) fn get_socket_path() -> Option<String> {
        if !Self::is_rootless() {
            return None;
        }
        let info_out = Command::new("podman")
            .arg("info")
            .arg("--format")
            .arg("{{.Host.RemoteSocket.Path}}")
            .output();
        if let Ok(out) = info_out {
            if out.status.success() {
                let sock = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !sock.is_empty() && std::path::Path::new(&sock).exists() {
                    return Some(sock);
                }
            }
        }
        None
    } // get_socket_path
    /// Validate that `userns_mode` is an allowed value.
    ///
    /// # Errors
    /// Returns `Err` if the mode is not in the allowlist.
    pub(crate) fn validate_userns_mode(mode: &str) -> io::Result<()> {
        if VALID_USERNS_MODES.contains(&mode) {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Invalid userns_mode '{mode}', must be one of: keep-id, host, auto, private"
                ),
            ))
        }
    } // validate_userns_mode
    /// Apply Podman-specific environment variables to a command.
    ///
    /// Sets `CONTAINER_HOST` if a rootless socket is detected, and
    /// `PODMAN_ROOTLESS=1` when running in rootless mode.
    fn apply_podman_env(cmd: &mut Command) {
        if let Some(sock_path) = Self::get_socket_path() {
            cmd.env("CONTAINER_HOST", format!("unix://{sock_path}"));
        }
        if Self::is_rootless() {
            cmd.env("PODMAN_ROOTLESS", "1");
        }
    } // apply_podman_env
    /// Create a new `Command` for the Podman CLI with environment pre-applied.
    fn podman_command(&self) -> Command {
        let mut cmd = Command::new(&self.command);
        Self::apply_podman_env(&mut cmd);
        cmd
    } // podman_command
    /// Build the `Command` for `podman create` without executing it.
    fn build_create_command(&self, opts: &ContainerOptions) -> Command {
        let image_ref = match &self.build_source {
            BuildSource::Dockerfile(_) => format!("{IMAGE_NAMESPACE}/{}", &self.name),
            BuildSource::Image(img_name) => img_name.clone(),
        };
        let mut cmd = self.podman_command();
        cmd.arg("create");
        // User namespace configuration - now configurable
        if !self.userns_mode.is_empty() {
            cmd.arg("--userns").arg(&self.userns_mode);
        }
        // SELinux labeling - now configurable
        if self.disable_selinux {
            cmd.arg("--security-opt").arg("label=disable");
        }
        let mount_spec = format!(
            "type=bind,source={},target={}",
            &self.directory, &self.workspace_folder
        );
        cmd.arg("--mount").arg(&mount_spec);
        inject_ssh_agent(&mut cmd);
        apply_common_create_args(
            &mut cmd,
            &self.forward_ports,
            &opts.remote_env,
            &self.run_args,
            self.mounts.as_ref(),
            &self.name,
            &self.user,
            &self.workspace_folder,
            &image_ref,
            self.override_command,
            false,
        );
        cmd
    } // build_create_command
    /// Build the `Command` for `podman build` or `podman pull` without executing it.
    fn build_build_command(&self, use_cache: bool) -> Command {
        match &self.build_source {
            BuildSource::Dockerfile(dockerfile_path) => {
                let image_tag = format!("{IMAGE_NAMESPACE}/{}", &self.name);
                let mut cmd = self.podman_command();
                cmd.arg("build")
                    .arg("-t")
                    .arg(&image_tag)
                    .arg("-f")
                    .arg(dockerfile_path);
                if !use_cache {
                    cmd.arg("--no-cache");
                }
                for (k, v) in &self.build_args {
                    cmd.arg("--build-arg").arg(format!("{k}={v}"));
                }
                cmd.arg(&self.build_context);
                cmd
            }
            BuildSource::Image(img) => {
                let mut cmd = self.podman_command();
                cmd.arg("pull").arg(img);
                cmd
            }
        }
    } // build_build_command
} // impl Podman

impl Provider for Podman {
    fn build(&self, use_cache: bool) -> io::Result<()> {
        let mut cmd = self.build_build_command(use_cache);
        run_step("build", &mut cmd)
    } // build
    fn create(&self, opts: &ContainerOptions) -> io::Result<()> {
        let mut cmd = self.build_create_command(opts);
        run_step("create", &mut cmd)
    } // create
    fn start(&self) -> io::Result<()> {
        let mut cmd = self.podman_command();
        cmd.arg("start").arg(&self.name);
        run_step("start", &mut cmd)
    } // start
    fn stop(&self) -> io::Result<()> {
        let mut cmd = self.podman_command();
        cmd.arg("stop").arg(&self.name);
        run_step("stop", &mut cmd)
    } // stop
    fn restart(&self) -> io::Result<()> {
        let mut cmd = self.podman_command();
        cmd.arg("restart").arg(&self.name);
        run_step("restart", &mut cmd)
    } // restart
    fn attach(&self) -> io::Result<()> {
        let mut cmd = self.podman_command();
        cmd.arg("attach").arg(&self.name);
        run_step("attach", &mut cmd)
    } // attach
    fn rm(&self) -> io::Result<()> {
        let mut cmd = self.podman_command();
        cmd.arg("rm").arg(&self.name);
        run_step("rm", &mut cmd)
    } // rm
    fn exists(&self) -> io::Result<bool> {
        check_container_status(&mut self.podman_command(), &self.name, true)
    } // exists
    fn running(&self) -> io::Result<bool> {
        check_container_status(&mut self.podman_command(), &self.name, false)
    } // running
    fn cp(&self, source: String, destination: String) -> io::Result<()> {
        let target = format!("{}:{}", &self.name, destination);
        let mut cmd = self.podman_command();
        cmd.arg("cp").arg(source).arg(&target);
        run_step("cp", &mut cmd)
    } // cp
    fn exec(&self, cmd: String) -> io::Result<()> {
        let mut proc = self.podman_command();
        proc.arg("exec")
            .arg("-u")
            .arg(&self.user)
            .arg("-w")
            .arg(&self.workspace_folder)
            .arg(&self.name)
            .arg("sh")
            .arg("-c")
            .arg(cmd);
        print_command(&proc);
        let result = proc.output()?;
        if result.status.success() {
            return Ok(());
        }
        let err_text = String::from_utf8_lossy(&result.stderr);
        let exit_code = result.status.code().unwrap_or(-1);
        Err(io::Error::new(
            io::ErrorKind::Other,
            crate::provider::utils::format_exec_error(exit_code, &err_text),
        ))
    } // exec
    fn exec_capture(&self, cmd: &str) -> crate::error::Result<ExecOutput> {
        let mut proc = self.podman_command();
        proc.arg("exec")
            .arg("-u")
            .arg(&self.user)
            .arg("-w")
            .arg(&self.workspace_folder)
            .arg(&self.name)
            .arg("sh")
            .arg("-c")
            .arg(cmd);
        print_command(&proc);
        output_to_exec_result(proc.output()?)
    } // exec_capture
    fn exec_raw(&self, prog: &str, args: &[&str]) -> io::Result<()> {
        let mut child = self.podman_command();
        child
            .arg("exec")
            .arg("-u")
            .arg(&self.user)
            .arg("-w")
            .arg(&self.workspace_folder)
            .arg(&self.name)
            .arg(prog)
            .args(args);
        print_command(&child);
        let result = child.output()?;
        if result.status.success() {
            return Ok(());
        }
        let err_text = String::from_utf8_lossy(&result.stderr);
        let exit_code = result.status.code().unwrap_or(-1);
        Err(io::Error::new(
            io::ErrorKind::Other,
            crate::provider::utils::format_exec_error(exit_code, &err_text),
        ))
    } // exec_raw
} // impl Provider for Podman

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn podman_command_applies_env() {
        // Verify podman_command() constructs a Command with the right program
        let container = Podman {
            build_args: HashMap::new(),
            build_context: "/test".to_string(),
            build_source: BuildSource::Image("alpine:latest".to_string()),
            command: "podman".to_string(),
            directory: "/workspace".to_string(),
            forward_ports: vec![],
            mounts: None,
            name: "test".to_string(),
            run_args: vec![],
            override_command: false,
            user: "testuser".to_string(),
            workspace_folder: "/workspace".to_string(),
            userns_mode: "keep-id".to_string(),
            disable_selinux: true,
        };

        let cmd = container.podman_command();
        assert_eq!(cmd.get_program(), "podman");
    }

    #[test]
    fn validate_userns_mode_edge_cases() {
        // Valid modes
        assert!(Podman::validate_userns_mode("keep-id").is_ok());
        assert!(Podman::validate_userns_mode("host").is_ok());
        assert!(Podman::validate_userns_mode("auto").is_ok());
        assert!(Podman::validate_userns_mode("private").is_ok());
        assert!(Podman::validate_userns_mode("").is_ok());
        // Invalid modes
        assert!(Podman::validate_userns_mode("Keep-Id").is_err()); // case sensitive
        assert!(Podman::validate_userns_mode(" keep-id").is_err()); // leading space
        assert!(Podman::validate_userns_mode("keep-id ").is_err()); // trailing space
        assert!(Podman::validate_userns_mode("nomap").is_err()); // not in allowlist
        assert!(Podman::validate_userns_mode("../../etc").is_err()); // path traversal
        assert!(Podman::validate_userns_mode("; rm -rf /").is_err()); // injection attempt
    }

    #[test]
    fn new_propagates_all_fields() {
        let container = Podman::new(
            HashMap::from([("K".to_string(), "V".to_string())]),
            "/ctx".to_string(),
            BuildSource::Dockerfile("Dockerfile".to_string()),
            "podman".to_string(),
            "/dir".to_string(),
            vec![8080],
            "my-ctr".to_string(),
            vec!["--net=host".to_string()],
            Some(vec![HashMap::from([(
                "type".to_string(),
                "bind".to_string(),
            )])]),
            "dev".to_string(),
            "/work".to_string(),
            true,
            "keep-id".to_string(),
            true,
        );
        assert_eq!(container.name, "my-ctr");
        assert_eq!(container.command, "podman");
        assert_eq!(container.user, "dev");
        assert_eq!(container.workspace_folder, "/work");
        assert_eq!(container.build_context, "/ctx");
        assert_eq!(container.forward_ports, vec![8080]);
        assert!(container.override_command);
        assert_eq!(container.userns_mode, "keep-id");
        assert!(container.disable_selinux);
        assert!(container.mounts.is_some());
        assert_eq!(container.run_args, vec!["--net=host"]);
        assert_eq!(container.build_args.len(), 1);
    }

    #[test]
    fn build_source_dockerfile_stores_path() {
        let container = Podman {
            build_source: BuildSource::Dockerfile("/path/to/Dockerfile".to_string()),
            ..get_base_podman()
        };
        match &container.build_source {
            BuildSource::Dockerfile(p) => assert_eq!(p, "/path/to/Dockerfile"),
            BuildSource::Image(_) => panic!("Expected Dockerfile variant"),
        }
    }

    #[test]
    fn build_source_image_stores_name() {
        let container = Podman {
            build_source: BuildSource::Image("ubuntu:22.04".to_string()),
            ..get_base_podman()
        };
        match &container.build_source {
            BuildSource::Image(i) => assert_eq!(i, "ubuntu:22.04"),
            BuildSource::Dockerfile(_) => panic!("Expected Image variant"),
        }
    }

    #[test]
    fn create_command_includes_userns_and_selinux() {
        let p = Podman::new(
            HashMap::new(),
            "/ctx".to_string(),
            BuildSource::Image("alpine".to_string()),
            "podman".to_string(),
            "/dir".to_string(),
            vec![],
            "t".to_string(),
            vec![],
            None,
            "root".to_string(),
            "/w".to_string(),
            false,
            "keep-id".to_string(),
            true,
        );
        let opts = ContainerOptions { remote_env: vec![] };
        let cmd = p.build_create_command(&opts);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"--userns".to_string()));
        assert!(args.contains(&"keep-id".to_string()));
        assert!(args.contains(&"--security-opt".to_string()));
        assert!(args.contains(&"label=disable".to_string()));
    }

    #[test]
    fn create_command_skips_userns_when_empty() {
        let p = Podman::new(
            HashMap::new(),
            "/ctx".to_string(),
            BuildSource::Image("alpine".to_string()),
            "podman".to_string(),
            "/dir".to_string(),
            vec![],
            "t".to_string(),
            vec![],
            None,
            "root".to_string(),
            "/w".to_string(),
            false,
            String::new(),
            false,
        );
        let opts = ContainerOptions { remote_env: vec![] };
        let cmd = p.build_create_command(&opts);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(!args.contains(&"--userns".to_string()));
        assert!(!args.contains(&"--security-opt".to_string()));
    }

    #[test]
    fn create_command_includes_ports_and_env() {
        let p = Podman::new(
            HashMap::new(),
            "/ctx".to_string(),
            BuildSource::Image("alpine".to_string()),
            "podman".to_string(),
            "/dir".to_string(),
            vec![3000, 8080],
            "t".to_string(),
            vec![],
            None,
            "dev".to_string(),
            "/work".to_string(),
            true,
            "keep-id".to_string(),
            true,
        );
        let opts = ContainerOptions {
            remote_env: vec![("MY_VAR".to_string(), "val".to_string())],
        };
        let cmd = p.build_create_command(&opts);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"3000:3000".to_string()));
        assert!(args.contains(&"8080:8080".to_string()));
        assert!(args.contains(&"MY_VAR=val".to_string()));
        assert!(args.contains(&"while sleep 1000; do :; done".to_string()));
    }

    #[test]
    fn build_command_dockerfile_includes_args() {
        let p = Podman::new(
            HashMap::from([("K".to_string(), "V".to_string())]),
            "/ctx".to_string(),
            BuildSource::Dockerfile("Dockerfile".to_string()),
            "podman".to_string(),
            "/dir".to_string(),
            vec![],
            "t".to_string(),
            vec![],
            None,
            "root".to_string(),
            "/w".to_string(),
            false,
            "keep-id".to_string(),
            true,
        );
        let cmd = p.build_build_command(false);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"build".to_string()));
        assert!(args.contains(&"--no-cache".to_string()));
        assert!(args.contains(&"--build-arg".to_string()));
        assert!(args.iter().any(|a| a.contains("K=V")));
    }

    /// Helper function to create a base Podman configuration
    fn get_base_podman() -> Podman {
        Podman {
            build_args: HashMap::new(),
            build_context: "/test".to_string(),
            build_source: BuildSource::Image("alpine:latest".to_string()),
            command: "podman".to_string(),
            directory: "/workspace".to_string(),
            forward_ports: vec![],
            mounts: None,
            name: "test".to_string(),
            run_args: vec![],
            override_command: false,
            user: "testuser".to_string(),
            workspace_folder: "/workspace".to_string(),
            userns_mode: "keep-id".to_string(),
            disable_selinux: true,
        }
    }

    #[test]
    fn exact_name_match_verifies_format_output() {
        // Validates that the {{.Names}} format used by exists()/running()
        // produces output that exact_name_match correctly handles
        use crate::provider::utils::exact_name_match;
        let output = b"my-container\nother\n".to_vec();
        assert!(exact_name_match(output, "my-container"));
        assert!(!exact_name_match(
            b"my-container-2\n".to_vec(),
            "my-container"
        ));
    }

    #[test]
    fn parse_error_image_not_found() {
        let msg = crate::provider::utils::format_exec_error(
            125,
            "Error: no such image: alpine:nonexistent",
        );
        assert!(msg.starts_with("Image not found:"), "got: {msg}");
    }

    #[test]
    fn parse_error_image_not_known() {
        let msg = crate::provider::utils::format_exec_error(125, "Error: image not known");
        assert!(msg.starts_with("Image not found:"), "got: {msg}");
    }

    #[test]
    fn parse_error_permission_denied() {
        let msg = crate::provider::utils::format_exec_error(126, "Error: permission denied");
        assert!(msg.contains("Permission denied"), "got: {msg}");
    }

    #[test]
    fn parse_error_daemon_not_running() {
        let msg = crate::provider::utils::format_exec_error(
            125,
            "Cannot connect to Podman. Is the Podman machine running?",
        );
        assert!(msg.contains("daemon is not running"), "got: {msg}");
    }

    #[test]
    fn parse_error_container_exists() {
        let msg = crate::provider::utils::format_exec_error(
            125,
            "Error: container already exists: my-ctr",
        );
        assert!(msg.starts_with("Container already exists:"), "got: {msg}");
    }

    #[test]
    fn parse_error_fallback() {
        let msg = crate::provider::utils::format_exec_error(1, "some unknown error");
        assert_eq!(msg, "Exec failed with exit code 1");
    }
}
