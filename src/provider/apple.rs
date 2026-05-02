use std::collections::HashMap;
use std::io::Result;
use std::process::Command;

use super::IMAGE_NAMESPACE;
use super::Provider;
use super::options::ContainerOptions;
use super::print_command;
use super::utils::{apply_common_create_args, check_container_status, inject_ssh_agent, run_step};
use super::{ExecOutput, output_to_exec_result};
use crate::provider::docker::BuildSource;

/// Apple Container provider — uses the `container` CLI to manage dev containers on macOS.
#[derive(Debug)]
pub struct AppleContainer {
    /// Build-time `--build-arg` key/value pairs passed to `container image build`.
    pub(crate) build_args: HashMap<String, String>,
    /// Path sent to the Apple container runtime as the build context.
    /// Defaults to the workspace root; overridden by `build.context` in devcontainer.json.
    pub(crate) build_context: String,
    /// Image source — either a Dockerfile path or a pre-built image name.
    pub(crate) build_source: BuildSource,
    /// CLI binary name (typically `"container"`).
    pub(crate) command: String,
    /// Host directory containing the project workspace.
    pub(crate) directory: String,
    /// Ports to forward from the container to the host.
    pub(crate) forward_ports: Vec<u16>,
    /// Container name used for lifecycle commands.
    pub(crate) name: String,
    /// Additional arguments passed to `container create`.
    pub(crate) run_args: Vec<String>,
    /// Additional bind/volume mounts from devcontainer.json.
    pub(crate) mounts: Option<Vec<HashMap<String, String>>>,
    /// User to run commands as inside the container.
    pub(crate) user: String,
    /// Path inside the container where the workspace is mounted.
    pub(crate) workspace_folder: String,
    /// When `true`, override the container entrypoint with a sleep loop.
    pub(crate) override_command: bool,
}

impl AppleContainer {
    /// Create a new Apple Container provider instance.
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
    ) -> Self {
        Self {
            build_args,
            build_context,
            build_source,
            command,
            directory,
            forward_ports,
            name,
            run_args,
            mounts,
            user,
            workspace_folder,
            override_command,
        }
    }

    /// Check if the Apple container CLI is available and the system is running.
    ///
    /// Returns `Ok(true)` if available, `Ok(false)` if not available, or `Err` if detection fails.
    ///
    /// # Errors
    /// Returns `Err` if the container command execution fails or if there are permission issues.
    pub fn is_available() -> Result<bool> {
        // Check if we're on macOS
        if std::env::consts::OS != "macos" {
            return Ok(false);
        }

        // Check if the container command exists
        let status = Command::new("container").arg("--version").output();

        match status {
            Ok(output) if output.status.success() => {
                // Container CLI exists, check if system is running
                let system_status = Command::new("container")
                    .arg("system")
                    .arg("status")
                    .output();

                Ok(system_status.as_ref().is_ok_and(|s| s.status.success()))
            }
            _ => Ok(false), // Container CLI not available
        }
    }

    /// Ensure the Apple container system service is running.
    /// This is required before any container operations can be performed.
    fn ensure_system_running(&self) -> Result<bool> {
        // Check if system is already running first
        let status_check = Command::new(&self.command)
            .arg("system")
            .arg("status")
            .output()?;

        if status_check.status.success() {
            return Ok(true);
        }

        // System not running — start it
        let mut start_cmd = Command::new(&self.command);
        start_cmd.arg("system").arg("start");
        print_command(&start_cmd);
        Ok(start_cmd.status()?.success())
    }

    /// Build the `Command` for `container create` without executing it.
    fn build_create_command(&self, opts: &ContainerOptions) -> Command {
        let image = match &self.build_source {
            BuildSource::Dockerfile(_) => format!("{IMAGE_NAMESPACE}/{}", &self.name),
            BuildSource::Image(name) => name.clone(),
        };

        let mut command = Command::new(&self.command);
        command.arg("create");
        command.arg("--mount");
        command.arg(format!(
            "type=bind,source={},target={}",
            &self.directory, &self.workspace_folder
        ));

        inject_ssh_agent(&mut command);

        apply_common_create_args(
            &mut command,
            &self.forward_ports,
            &opts.remote_env,
            &self.run_args,
            self.mounts.as_ref(),
            &self.name,
            &self.user,
            &self.workspace_folder,
            &image,
            self.override_command,
            false,
        );

        command
    }

    /// Build the `Command` for `container image build/pull` without executing it.
    fn build_build_command(&self, use_cache: bool) -> Command {
        match &self.build_source {
            BuildSource::Dockerfile(path) => {
                let tag = format!("{IMAGE_NAMESPACE}/{}", &self.name);

                let mut command = Command::new(&self.command);
                command
                    .arg("image")
                    .arg("build")
                    .arg("-t")
                    .arg(&tag)
                    .arg("-f")
                    .arg(path);

                if !use_cache {
                    command.arg("--no-cache");
                }

                for (key, value) in &self.build_args {
                    command.arg("--build-arg").arg(format!("{key}={value}"));
                }

                command.arg(&self.build_context);

                command
            }
            BuildSource::Image(image) => {
                let mut command = Command::new(&self.command);
                command.arg("image").arg("pull").arg(image);
                command
            }
        }
    }
}

impl Provider for AppleContainer {
    fn build(&self, use_cache: bool) -> Result<()> {
        self.ensure_system_running()?;
        let mut command = self.build_build_command(use_cache);
        run_step("build", &mut command)
    }

    fn create(&self, opts: &ContainerOptions) -> Result<()> {
        self.ensure_system_running()?;
        let mut command = self.build_create_command(opts);
        run_step("create", &mut command)
    }

    fn start(&self) -> Result<()> {
        self.ensure_system_running()?;
        let mut command = Command::new(&self.command);
        command.arg("start").arg(&self.name);
        run_step("start", &mut command)
    }

    fn stop(&self) -> Result<()> {
        let mut command = Command::new(&self.command);
        command.arg("stop").arg(&self.name);
        run_step("stop", &mut command)
    }

    fn restart(&self) -> Result<()> {
        let mut command = Command::new(&self.command);
        command.arg("restart").arg(&self.name);
        run_step("restart", &mut command)
    }

    fn attach(&self) -> Result<()> {
        let mut command = Command::new(&self.command);
        command.arg("attach").arg(&self.name);
        run_step("attach", &mut command)
    }

    fn rm(&self) -> Result<()> {
        let mut command = Command::new(&self.command);
        command.arg("rm").arg(&self.name);
        run_step("rm", &mut command)
    }

    fn exists(&self) -> Result<bool> {
        check_container_status(&mut Command::new(&self.command), &self.name, true)
    }

    fn running(&self) -> Result<bool> {
        check_container_status(&mut Command::new(&self.command), &self.name, false)
    }

    fn cp(&self, source: String, destination: String) -> Result<()> {
        let mut command = Command::new(&self.command);
        command
            .arg("cp")
            .arg(source)
            .arg(format!("{}:{}", &self.name, destination));
        run_step("cp", &mut command)
    }

    fn exec(&self, cmd: String) -> Result<()> {
        self.ensure_system_running()?;

        let mut command = Command::new(&self.command);
        command
            .arg("exec")
            .arg("-u")
            .arg(&self.user)
            .arg("-w")
            .arg(&self.workspace_folder)
            .arg(&self.name)
            .arg("sh")
            .arg("-c")
            .arg(cmd);

        print_command(&command);

        let output = command.output()?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let code = output.status.code().unwrap_or(-1);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                super::utils::format_exec_error(code, &stderr),
            ))
        }
    }

    fn exec_capture(&self, cmd: &str) -> crate::error::Result<ExecOutput> {
        self.ensure_system_running()
            .map_err(crate::error::Error::Io)?;

        let mut command = Command::new(&self.command);
        command
            .arg("exec")
            .arg("-u")
            .arg(&self.user)
            .arg("-w")
            .arg(&self.workspace_folder)
            .arg(&self.name)
            .arg("sh")
            .arg("-c")
            .arg(cmd);

        print_command(&command);
        output_to_exec_result(command.output().map_err(crate::error::Error::Io)?)
    }

    fn exec_raw(&self, prog: &str, args: &[&str]) -> Result<()> {
        self.ensure_system_running()?;

        let mut command = Command::new(&self.command);
        command
            .arg("exec")
            .arg("-u")
            .arg(&self.user)
            .arg("-w")
            .arg(&self.workspace_folder)
            .arg(&self.name)
            .arg(prog)
            .args(args);

        print_command(&command);

        let output = command.output()?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let code = output.status.code().unwrap_or(-1);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                super::utils::format_exec_error(code, &stderr),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_apple() -> AppleContainer {
        AppleContainer {
            build_args: HashMap::new(),
            build_context: "/test".to_string(),
            build_source: BuildSource::Image("alpine:latest".to_string()),
            command: "container".to_string(),
            directory: "/workspace".to_string(),
            forward_ports: vec![],
            name: "test".to_string(),
            run_args: vec![],
            mounts: None,
            user: "root".to_string(),
            workspace_folder: "/workspace".to_string(),
            override_command: false,
        }
    }

    #[test]
    fn apple_container_build_source_handling() {
        // Test Dockerfile build source
        let dockerfile_container = AppleContainer {
            build_source: BuildSource::Dockerfile("custom.Dockerfile".to_string()),
            ..base_apple()
        };

        // Test Image build source
        let image_container = AppleContainer {
            build_source: BuildSource::Image("alpine:latest".to_string()),
            ..base_apple()
        };

        // Verify the build sources are stored correctly
        match dockerfile_container.build_source {
            BuildSource::Dockerfile(path) => assert_eq!(path, "custom.Dockerfile"),
            BuildSource::Image(_) => panic!("Expected Dockerfile build source"),
        }

        match image_container.build_source {
            BuildSource::Image(image) => assert_eq!(image, "alpine:latest"),
            BuildSource::Dockerfile(_) => panic!("Expected Image build source"),
        }
    }

    #[test]
    fn new_propagates_all_fields() {
        let container = AppleContainer::new(
            HashMap::from([("K".to_string(), "V".to_string())]),
            "/ctx".to_string(),
            BuildSource::Dockerfile("Dockerfile".to_string()),
            "container".to_string(),
            "/dir".to_string(),
            vec![8080, 3000],
            "my-ctr".to_string(),
            vec!["--net=host".to_string()],
            Some(vec![HashMap::from([(
                "type".to_string(),
                "bind".to_string(),
            )])]),
            "dev".to_string(),
            "/work".to_string(),
            true,
        );
        assert_eq!(container.name, "my-ctr");
        assert_eq!(container.command, "container");
        assert_eq!(container.user, "dev");
        assert_eq!(container.workspace_folder, "/work");
        assert_eq!(container.build_context, "/ctx");
        assert_eq!(container.forward_ports, vec![8080, 3000]);
        assert!(container.override_command);
        assert!(container.mounts.is_some());
        assert_eq!(container.run_args, vec!["--net=host"]);
        assert_eq!(container.build_args.len(), 1);
    }

    #[test]
    fn is_available_returns_false_on_non_macos() {
        // On Linux/CI this should always be false
        if std::env::consts::OS != "macos" {
            let result = AppleContainer::is_available();
            assert!(result.is_ok());
            assert!(
                !result.unwrap(),
                "Apple container should not be available on non-macOS"
            );
        }
    }

    #[test]
    fn build_command_uses_image_subcommand() {
        let a = AppleContainer::new(
            HashMap::new(),
            "/ctx".to_string(),
            BuildSource::Dockerfile("Dockerfile".to_string()),
            "container".to_string(),
            "/dir".to_string(),
            vec![],
            "t".to_string(),
            vec![],
            None,
            "root".to_string(),
            "/w".to_string(),
            false,
        );
        let cmd = a.build_build_command(true);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        // Apple uses "image build" subcommand (not just "build")
        assert!(args.contains(&"image".to_string()));
        assert!(args.contains(&"build".to_string()));
    }

    #[test]
    fn build_command_image_uses_image_pull() {
        let a = AppleContainer::new(
            HashMap::new(),
            "/ctx".to_string(),
            BuildSource::Image("ubuntu:22.04".to_string()),
            "container".to_string(),
            "/dir".to_string(),
            vec![],
            "t".to_string(),
            vec![],
            None,
            "root".to_string(),
            "/w".to_string(),
            false,
        );
        let cmd = a.build_build_command(true);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"image".to_string()));
        assert!(args.contains(&"pull".to_string()));
        assert!(args.contains(&"ubuntu:22.04".to_string()));
        assert!(!args.contains(&"build".to_string()));
    }

    #[test]
    fn exact_name_match_verifies_format_output() {
        // Validates that the {{.Names}} format used by exists()/running()
        // produces output that exact_name_match correctly handles
        let output = b"my-container\nother\n".to_vec();
        assert!(crate::provider::utils::exact_name_match(
            output,
            "my-container"
        ));
        assert!(!crate::provider::utils::exact_name_match(
            b"my-container-2\n".to_vec(),
            "my-container"
        ));
    }

    #[test]
    fn create_command_includes_override() {
        let a = AppleContainer::new(
            HashMap::new(),
            "/ctx".to_string(),
            BuildSource::Image("alpine".to_string()),
            "container".to_string(),
            "/dir".to_string(),
            vec![8080],
            "my-ctr".to_string(),
            vec![],
            None,
            "dev".to_string(),
            "/work".to_string(),
            true,
        );
        let opts = ContainerOptions { remote_env: vec![] };
        let cmd = a.build_create_command(&opts);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"create".to_string()));
        assert!(args.contains(&"8080:8080".to_string()));
        assert!(args.contains(&"my-ctr".to_string()));
        assert!(args.contains(&"sh".to_string()));
        assert!(args.contains(&"while sleep 1000; do :; done".to_string()));
    }
}
