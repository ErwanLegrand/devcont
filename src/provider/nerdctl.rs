use std::collections::HashMap;
use std::io::Result;
use std::process::Command;

use super::IMAGE_NAMESPACE;
use super::Provider;
use super::options::ContainerOptions;
use super::print_command;
use super::utils::{
    apply_common_create_args, check_container_status, inject_ssh_agent, run_and_check,
};
use super::{ExecOutput, output_to_exec_result};
use crate::provider::docker::BuildSource;

/// Nerdctl provider — uses the `nerdctl` CLI to manage dev containers with containerd.
#[derive(Debug)]
pub struct Nerdctl {
    /// Build-time `--build-arg` key/value pairs passed to `nerdctl build`.
    pub(crate) build_args: HashMap<String, String>,
    /// Path sent to the build context.
    /// Defaults to the workspace root; overridden by `build.context` in devcontainer.json.
    pub(crate) build_context: String,
    /// Image source — either a Dockerfile path or a pre-built image name.
    pub(crate) build_source: BuildSource,
    /// CLI binary name (typically `"nerdctl"`).
    pub(crate) command: String,
    /// Host directory containing the project workspace.
    pub(crate) directory: String,
    /// Ports to forward from the container to the host.
    pub(crate) forward_ports: Vec<u16>,
    /// Container name used for lifecycle commands.
    pub(crate) name: String,
    /// Additional arguments passed to `nerdctl create`.
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

impl Nerdctl {
    /// Create a new Nerdctl provider instance.
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

    /// Check if the nerdctl CLI is available and the containerd service is running.
    /// Returns `Ok(true)` if available, `Ok(false)` if not available, or `Err` if detection fails.
    ///
    /// # Errors
    /// Returns `Err` if the nerdctl command execution fails or if there are permission issues.
    pub fn is_available() -> Result<bool> {
        // Check if the nerdctl command exists and is executable
        let version_check = Command::new("nerdctl").arg("--version").output();

        match version_check {
            Ok(output) if output.status.success() => {
                // nerdctl CLI exists, check if containerd is accessible
                // Try a simple command that requires containerd to be running
                let info_check = Command::new("nerdctl").arg("info").output();

                Ok(info_check.is_ok_and(|info| info.status.success()))
            }
            _ => Ok(false), // nerdctl CLI not available or not executable
        }
    }

    /// Build the `Command` for `nerdctl create` without executing it.
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

    /// Build the `Command` for `nerdctl build/pull` without executing it.
    fn build_build_command(&self, use_cache: bool) -> Command {
        match &self.build_source {
            BuildSource::Dockerfile(path) => {
                let tag = format!("{IMAGE_NAMESPACE}/{}", &self.name);

                let mut command = Command::new(&self.command);
                command.arg("build").arg("-t").arg(&tag).arg("-f").arg(path);

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
                command.arg("pull").arg(image);
                command
            }
        }
    }
}

impl Provider for Nerdctl {
    fn build(&self, use_cache: bool) -> Result<()> {
        let mut command = self.build_build_command(use_cache);
        run_and_check(&mut command)
    }

    fn create(&self, opts: &ContainerOptions) -> Result<()> {
        let mut command = self.build_create_command(opts);
        run_and_check(&mut command)
    }

    fn start(&self) -> Result<()> {
        let mut command = Command::new(&self.command);
        command.arg("start").arg(&self.name);
        run_and_check(&mut command)
    }

    fn stop(&self) -> Result<()> {
        let mut command = Command::new(&self.command);
        command.arg("stop").arg(&self.name);
        run_and_check(&mut command)
    }

    fn restart(&self) -> Result<()> {
        let mut command = Command::new(&self.command);
        command.arg("restart").arg(&self.name);
        run_and_check(&mut command)
    }

    fn attach(&self) -> Result<()> {
        let mut command = Command::new(&self.command);
        command.arg("attach").arg(&self.name);
        run_and_check(&mut command)
    }

    fn rm(&self) -> Result<()> {
        let mut command = Command::new(&self.command);
        command.arg("rm").arg(&self.name);
        run_and_check(&mut command)
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
        run_and_check(&mut command)
    }

    fn exec(&self, cmd: String) -> Result<()> {
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
        output_to_exec_result(command.output()?)
    }

    fn exec_raw(&self, prog: &str, args: &[&str]) -> Result<()> {
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
    use std::collections::HashMap;

    #[test]
    fn new_propagates_all_fields() {
        let container = Nerdctl::new(
            HashMap::from([("K".to_string(), "V".to_string())]),
            "/ctx".to_string(),
            BuildSource::Dockerfile("Dockerfile".to_string()),
            "nerdctl".to_string(),
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
        );
        assert_eq!(container.name, "my-ctr");
        assert_eq!(container.command, "nerdctl");
        assert_eq!(container.user, "dev");
        assert_eq!(container.workspace_folder, "/work");
        assert_eq!(container.build_context, "/ctx");
        assert_eq!(container.forward_ports, vec![8080]);
        assert!(container.override_command);
        assert!(container.mounts.is_some());
        assert_eq!(container.run_args, vec!["--net=host"]);
        assert_eq!(container.build_args.len(), 1);
    }

    #[test]
    fn build_source_dockerfile_stores_path() {
        let container = Nerdctl {
            build_source: BuildSource::Dockerfile("/path/Dockerfile".to_string()),
            build_args: HashMap::new(),
            build_context: "/test".to_string(),
            command: "nerdctl".to_string(),
            directory: "/workspace".to_string(),
            forward_ports: vec![],
            name: "test".to_string(),
            run_args: vec![],
            mounts: None,
            user: "root".to_string(),
            workspace_folder: "/workspace".to_string(),
            override_command: false,
        };
        match &container.build_source {
            BuildSource::Dockerfile(p) => assert_eq!(p, "/path/Dockerfile"),
            BuildSource::Image(_) => panic!("Expected Dockerfile variant"),
        }
    }

    #[test]
    fn build_source_image_stores_name() {
        let container = Nerdctl {
            build_source: BuildSource::Image("ubuntu:22.04".to_string()),
            build_args: HashMap::new(),
            build_context: "/test".to_string(),
            command: "nerdctl".to_string(),
            directory: "/workspace".to_string(),
            forward_ports: vec![],
            name: "test".to_string(),
            run_args: vec![],
            mounts: None,
            user: "root".to_string(),
            workspace_folder: "/workspace".to_string(),
            override_command: false,
        };
        match &container.build_source {
            BuildSource::Image(i) => assert_eq!(i, "ubuntu:22.04"),
            BuildSource::Dockerfile(_) => panic!("Expected Image variant"),
        }
    }

    #[test]
    fn create_command_includes_workspace_mount() {
        let n = Nerdctl::new(
            HashMap::new(),
            "/ctx".to_string(),
            BuildSource::Image("alpine".to_string()),
            "nerdctl".to_string(),
            "/mydir".to_string(),
            vec![],
            "t".to_string(),
            vec![],
            None,
            "root".to_string(),
            "/workspace".to_string(),
            false,
        );
        let opts = ContainerOptions { remote_env: vec![] };
        let cmd = n.build_create_command(&opts);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"create".to_string()));
        assert!(
            args.iter()
                .any(|a| a.contains("type=bind,source=/mydir,target=/workspace"))
        );
    }

    #[test]
    fn exists_command_includes_format_names() {
        // This tests that exists() uses --format {{.Names}} which is load-bearing
        // We can't extract exists() to a builder easily, but we can verify the pattern
        // by checking exact_name_match which is the consumer of the format output
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
    fn build_command_dockerfile_args() {
        let n = Nerdctl::new(
            HashMap::from([("KEY".to_string(), "VAL".to_string())]),
            "/ctx".to_string(),
            BuildSource::Dockerfile("Dockerfile".to_string()),
            "nerdctl".to_string(),
            "/dir".to_string(),
            vec![],
            "t".to_string(),
            vec![],
            None,
            "root".to_string(),
            "/w".to_string(),
            false,
        );
        let cmd = n.build_build_command(false);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"build".to_string()));
        assert!(args.contains(&"--no-cache".to_string()));
        assert!(args.contains(&"--build-arg".to_string()));
    }
}
