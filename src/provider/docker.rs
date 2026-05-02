use super::options::ContainerOptions;
use super::print_command;
use super::utils::{apply_common_create_args, check_container_status, inject_ssh_agent, run_step};
use super::{ExecOutput, IMAGE_NAMESPACE, Provider, output_to_exec_result};
use std::{collections::HashMap, io, process::Command};
/// Source used to obtain the container image for a provider.
#[derive(Debug, Clone)]
pub enum BuildSource {
    /// Build from a Dockerfile at the given path.
    Dockerfile(String),
    /// Pull a pre-built image by name.
    Image(String),
} // BuildSource

/// Docker provider -- uses the `docker` CLI to manage dev containers.
#[derive(Debug)] // Docker
pub struct Docker {
    // NOLINT(too_many_fields)
    /// Build-time `--build-arg` key/value pairs passed to `docker build`.
    pub(crate) build_args: HashMap<String, String>,
    /// Path sent to the Docker daemon as the build context.
    /// Defaults to the workspace root; overridden by `build.context` in devcontainer.json.
    pub(crate) build_context: String,
    /// Image source -- either a Dockerfile path or a pre-built image name.
    pub(crate) build_source: BuildSource,
    /// CLI binary name (typically `"docker"`).
    pub(crate) command: String,
    /// Host directory containing the project workspace.
    pub(crate) directory: String,
    /// Ports to forward from the container to the host.
    pub(crate) forward_ports: Vec<u16>,
    /// Container name used for lifecycle commands.
    pub(crate) name: String,
    /// Additional arguments passed to `docker create`.
    pub(crate) run_args: Vec<String>,
    /// Additional bind/volume mounts from devcontainer.json.
    pub(crate) mounts: Option<Vec<HashMap<String, String>>>,
    /// User to run commands as inside the container.
    pub(crate) user: String,
    /// Path inside the container where the workspace is mounted.
    pub(crate) workspace_folder: String,
    /// When `true`, override the container entrypoint with a sleep loop.
    pub(crate) override_command: bool,
} // Docker

impl Docker {
    /// Create a new Docker provider instance.
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
    } // new
    /// Check if the Docker CLI is available and the daemon is running.
    ///
    /// Returns `Ok(true)` if available, `Ok(false)` if not available.
    ///
    /// # Errors
    /// Returns `Err` if command execution fails due to permission issues.
    pub fn is_available() -> io::Result<bool> {
        let ver = Command::new("docker").arg("--version").output();
        match ver {
            Ok(out) if out.status.success() => {
                let daemon_check = Command::new("docker").arg("info").output();
                Ok(daemon_check.is_ok_and(|o| o.status.success()))
            }
            _ => Ok(false),
        }
    } // is_available

    /// Build the `Command` for `docker build` or `docker pull` without executing it.
    fn build_build_command(&self, use_cache: bool) -> Command {
        match &self.build_source {
            BuildSource::Dockerfile(dockerfile_path) => {
                let image_tag = format!("{IMAGE_NAMESPACE}/{}", &self.name);
                let mut cmd = Command::new(&self.command);
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
                let mut cmd = Command::new(&self.command);
                cmd.arg("pull").arg(img);
                cmd
            }
        }
    } // build_build_command
    /// Build the `Command` for `docker create` without executing it.
    fn build_create_command(&self, opts: &ContainerOptions) -> Command {
        let image_ref = match &self.build_source {
            BuildSource::Dockerfile(_) => format!("{IMAGE_NAMESPACE}/{}", &self.name),
            BuildSource::Image(img_name) => img_name.clone(),
        };
        let mount_spec = format!(
            "type=bind,source={},target={}",
            &self.directory, &self.workspace_folder
        );
        let mut cmd = Command::new(&self.command);
        cmd.arg("create").arg("--mount").arg(&mount_spec);
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
            // Docker uses `/bin/sh` (absolute path) because the Docker runtime
            // guarantees it exists. Other providers (Podman, Apple, Nerdctl) use
            // bare `sh` for better compatibility with rootless/minimal containers
            // where `/bin` may not be on PATH.
            true,
        );
        cmd
    } // build_create_command
} // impl Docker

impl Provider for Docker {
    fn build(&self, use_cache: bool) -> io::Result<()> {
        let mut cmd = self.build_build_command(use_cache);
        run_step("build", &mut cmd)
    } // build
    fn create(&self, opts: &ContainerOptions) -> io::Result<()> {
        let mut cmd = self.build_create_command(opts);
        run_step("create", &mut cmd)
    } // create
    fn start(&self) -> io::Result<()> {
        let mut cmd = Command::new(&self.command);
        cmd.arg("start").arg(&self.name);
        run_step("start", &mut cmd)
    } // start
    fn stop(&self) -> io::Result<()> {
        let mut cmd = Command::new(&self.command);
        cmd.arg("stop").arg(&self.name);
        run_step("stop", &mut cmd)
    } // stop
    fn restart(&self) -> io::Result<()> {
        let mut cmd = Command::new(&self.command);
        cmd.arg("restart").arg(&self.name);
        run_step("restart", &mut cmd)
    } // restart
    fn attach(&self) -> io::Result<()> {
        let mut cmd = Command::new(&self.command);
        cmd.arg("attach").arg(&self.name);
        run_step("attach", &mut cmd)
    } // attach
    fn rm(&self) -> io::Result<()> {
        let mut cmd = Command::new(&self.command);
        cmd.arg("rm").arg(&self.name);
        run_step("rm", &mut cmd)
    } // rm
    fn exists(&self) -> io::Result<bool> {
        check_container_status(&mut Command::new(&self.command), &self.name, true)
    } // exists
    fn running(&self) -> io::Result<bool> {
        check_container_status(&mut Command::new(&self.command), &self.name, false)
    } // running
    fn cp(&self, source: String, destination: String) -> io::Result<()> {
        let target = format!("{}:{}", &self.name, destination);
        let mut cmd = Command::new(&self.command);
        cmd.arg("cp").arg(source).arg(&target);
        run_step("cp", &mut cmd)
    } // cp
    fn exec(&self, cmd: String) -> io::Result<()> {
        let mut proc = Command::new(&self.command);
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
            super::utils::format_exec_error(exit_code, &err_text),
        ))
    } // exec
    fn exec_capture(&self, cmd: &str) -> crate::error::Result<ExecOutput> {
        let mut proc = Command::new(&self.command);
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
        let mut child = Command::new(&self.command);
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
            super::utils::format_exec_error(exit_code, &err_text),
        ))
    } // exec_raw
} // impl Provider for Docker

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::utils::exact_name_match;

    #[test]
    fn build_command_dockerfile_includes_tag_and_context() {
        let d = Docker::new(
            HashMap::from([("K".to_string(), "V".to_string())]),
            "/ctx".to_string(),
            BuildSource::Dockerfile("/path/Dockerfile".to_string()),
            "docker".to_string(),
            "/dir".to_string(),
            vec![],
            "test-ctr".to_string(),
            vec![],
            None,
            "root".to_string(),
            "/workspace".to_string(),
            false,
        );
        let cmd = d.build_build_command(true);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert!(args.contains(&std::ffi::OsStr::new("build")));
        assert!(args.contains(&std::ffi::OsStr::new("-t")));
        assert!(args.contains(&std::ffi::OsStr::new("devcont/test-ctr")));
        assert!(args.contains(&std::ffi::OsStr::new("-f")));
        assert!(args.contains(&std::ffi::OsStr::new("/path/Dockerfile")));
        assert!(args.contains(&std::ffi::OsStr::new("--build-arg")));
        assert!(args.contains(&std::ffi::OsStr::new("/ctx")));
        assert!(!args.contains(&std::ffi::OsStr::new("--no-cache")));
    }

    #[test]
    fn build_command_no_cache_flag() {
        let d = Docker::new(
            HashMap::new(),
            "/ctx".to_string(),
            BuildSource::Dockerfile("Dockerfile".to_string()),
            "docker".to_string(),
            "/dir".to_string(),
            vec![],
            "t".to_string(),
            vec![],
            None,
            "root".to_string(),
            "/w".to_string(),
            false,
        );
        let cmd = d.build_build_command(false);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert!(args.contains(&std::ffi::OsStr::new("--no-cache")));
    }

    #[test]
    fn build_command_image_uses_pull() {
        let d = Docker::new(
            HashMap::new(),
            "/ctx".to_string(),
            BuildSource::Image("alpine:latest".to_string()),
            "docker".to_string(),
            "/dir".to_string(),
            vec![],
            "t".to_string(),
            vec![],
            None,
            "root".to_string(),
            "/w".to_string(),
            false,
        );
        let cmd = d.build_build_command(true);
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert!(args.contains(&std::ffi::OsStr::new("pull")));
        assert!(args.contains(&std::ffi::OsStr::new("alpine:latest")));
        assert!(!args.contains(&std::ffi::OsStr::new("build")));
    }

    #[test]
    fn create_command_includes_workspace_mount_and_name() {
        let d = Docker::new(
            HashMap::new(),
            "/ctx".to_string(),
            BuildSource::Image("alpine:latest".to_string()),
            "docker".to_string(),
            "/mydir".to_string(),
            vec![8080],
            "my-ctr".to_string(),
            vec!["--net=host".to_string()],
            None,
            "dev".to_string(),
            "/workspace".to_string(),
            true,
        );
        let opts = ContainerOptions {
            remote_env: vec![("FOO".to_string(), "bar".to_string())],
        };
        let cmd = d.build_create_command(&opts);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();

        assert!(args.contains(&"create".to_string()));
        assert!(args.contains(&"--mount".to_string()));
        assert!(
            args.iter()
                .any(|a| a.contains("type=bind,source=/mydir,target=/workspace"))
        );
        assert!(args.contains(&"--publish".to_string()));
        assert!(args.contains(&"8080:8080".to_string()));
        assert!(args.contains(&"--env".to_string()));
        assert!(args.contains(&"FOO=bar".to_string()));
        assert!(args.contains(&"--net=host".to_string()));
        assert!(args.contains(&"-it".to_string()));
        assert!(args.contains(&"--name".to_string()));
        assert!(args.contains(&"my-ctr".to_string()));
        assert!(args.contains(&"-u".to_string()));
        assert!(args.contains(&"dev".to_string()));
        assert!(args.contains(&"-w".to_string()));
        assert!(args.contains(&"/workspace".to_string()));
        // override_command = true
        assert!(args.contains(&"/bin/sh".to_string()));
        assert!(args.contains(&"while sleep 1000; do :; done".to_string()));

        // Verify argument ordering for flag-value pairs
        let name_pos = args.iter().position(|a| a == "--name").unwrap();
        assert_eq!(
            args[name_pos + 1],
            "my-ctr",
            "--name must be followed by container name"
        );
        let user_pos = args.iter().position(|a| a == "-u").unwrap();
        assert_eq!(args[user_pos + 1], "dev", "-u must be followed by username");
        let workdir_pos = args.iter().position(|a| a == "-w").unwrap();
        assert_eq!(
            args[workdir_pos + 1],
            "/workspace",
            "-w must be followed by workspace path"
        );
    }

    #[test]
    fn create_command_no_override_skips_sleep() {
        let d = Docker::new(
            HashMap::new(),
            "/ctx".to_string(),
            BuildSource::Image("alpine".to_string()),
            "docker".to_string(),
            "/dir".to_string(),
            vec![],
            "t".to_string(),
            vec![],
            None,
            "root".to_string(),
            "/w".to_string(),
            false,
        );
        let opts = ContainerOptions { remote_env: vec![] };
        let cmd = d.build_create_command(&opts);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(!args.contains(&"/bin/sh".to_string()));
    }

    #[test]
    fn docker_availability_detection_does_not_panic() {
        let result = Docker::is_available();
        assert!(result.is_ok());
    }

    #[test]
    fn exact_name_match_matches_exact() {
        let output = b"foo\n".to_vec();
        assert!(exact_name_match(output, "foo"));
    }

    #[test]
    fn exact_name_match_no_prefix_match() {
        let output = b"foobar\n".to_vec();
        assert!(!exact_name_match(output, "foo"));
    }

    #[test]
    fn exact_name_match_no_suffix_match() {
        let output = b"barfoo\n".to_vec();
        assert!(!exact_name_match(output, "foo"));
    }

    #[test]
    fn exact_name_match_empty_output() {
        assert!(!exact_name_match(vec![], "foo"));
    }

    #[test]
    fn exact_name_match_multiple_names_one_matches() {
        let output = b"foobar\nfoo\nbaz\n".to_vec();
        assert!(exact_name_match(output, "foo"));
    }

    #[test]
    fn exact_name_match_multiple_names_none_match() {
        let output = b"foobar\nbaz\n".to_vec();
        assert!(!exact_name_match(output, "foo"));
    }
}
