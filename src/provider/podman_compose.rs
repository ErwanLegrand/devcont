use super::options::ContainerOptions;
use super::print_command;
use super::utils::{ComposeOverrideGuard, create_compose_override, run_and_check};
use super::{ExecOutput, Provider, output_to_exec_result};
use std::{collections::HashMap, io, path::Path, process::Command};
/// Podman Compose provider -- manages containers via `podman-compose`.
#[derive(Debug)] // PodmanCompose
pub struct PodmanCompose {
    // compose provider
    /// Build-time `--build-arg` key/value pairs.
    pub(crate) build_args: HashMap<String, String>,
    /// CLI binary name (typically `"podman-compose"`).
    pub(crate) command: String,
    /// Environment variables to inject into the compose override.
    pub(crate) env_vars: Vec<(String, String)>,
    /// Path to the docker-compose.yml file.
    pub(crate) file: String,
    /// Container/project name used for lifecycle commands.
    pub(crate) name: String,
    /// CLI binary name for direct podman commands (typically `"podman"`).
    pub(crate) podman_command: String,
    /// When `true`, appends `:z` to the SSH agent socket volume mount so that
    /// `SELinux` allows the container to access it.
    pub(crate) selinux_relabel: bool,
    /// Service name within the compose file.
    pub(crate) service: String,
    /// Shell to use for attach (typically `"sh"`).
    pub(crate) shell: String,
    /// User to run commands as inside the container.
    pub(crate) user: String,
    /// Path inside the container where the workspace is mounted.
    pub(crate) workspace_folder: String,
} // PodmanCompose
impl PodmanCompose {
    // methods
    /// Create a new Podman Compose provider instance.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        build_args: HashMap<String, String>,
        command: String,
        env_vars: Vec<(String, String)>,
        file: String,
        name: String,
        podman_command: String,
        selinux_relabel: bool,
        service: String,
        shell: String,
        user: String,
        workspace_folder: String,
    ) -> Self {
        Self {
            build_args,
            command,
            env_vars,
            file,
            name,
            podman_command,
            selinux_relabel,
            service,
            shell,
            user,
            workspace_folder,
        }
    } // new
    fn create_docker_compose(&self) -> io::Result<ComposeOverrideGuard> {
        create_compose_override(
            &self.service,
            &self.env_vars,
            self.selinux_relabel,
            &self.build_args,
        )
    } // create_docker_compose
    fn extract_container_id(raw: &str) -> &str {
        raw.lines()
            .find(|l| !l.trim().is_empty())
            .map_or("", str::trim)
    } // extract_container_id
    fn running_args(&self) -> Vec<String> {
        vec![
            "ps".to_string(),
            "-q".to_string(),
            "--filter".to_string(),
            "status=running".to_string(),
            "--filter".to_string(),
            format!("label=io.podman.compose.project={}", &self.name),
        ]
    } // running_args
    fn rm_args(&self, override_path: &Path) -> Vec<String> {
        vec![
            "-f".to_string(),
            self.file.clone(),
            "-f".to_string(),
            override_path.to_string_lossy().into_owned(),
            "-p".to_string(),
            self.name.clone(),
            "down".to_string(),
            "--remove-orphans".to_string(),
        ]
    } // rm_args
    /// Build the base compose args: `-f <file> -f <override> -p <name>`.
    fn compose_base_args(&self, override_path: &Path) -> Vec<String> {
        vec![
            "-f".to_string(),
            self.file.clone(),
            "-f".to_string(),
            override_path.to_string_lossy().into_owned(),
            "-p".to_string(),
            self.name.clone(),
        ]
    } // compose_base_args
    fn build_command_args(&self, override_path: &Path, use_cache: bool) -> Vec<String> {
        let mut tokens = self.compose_base_args(override_path);
        tokens.push("build".to_string());
        if !use_cache {
            tokens.push("--no-cache".to_string());
        }
        for (k, v) in &self.build_args {
            tokens.push("--build-arg".to_string());
            tokens.push(format!("{k}={v}"));
        } // build_args
        tokens
    } // build_command_args
    fn start_command_args(&self, override_path: &Path) -> Vec<String> {
        let mut tokens = self.compose_base_args(override_path);
        tokens.extend(["up".to_string(), "--detach".to_string()]);
        tokens
    } // start_command_args
    fn stop_command_args(&self, override_path: &Path) -> Vec<String> {
        let mut tokens = self.compose_base_args(override_path);
        tokens.push("stop".to_string());
        tokens
    } // stop_command_args
    fn restart_command_args(&self, override_path: &Path) -> Vec<String> {
        let mut tokens = self.compose_base_args(override_path);
        tokens.push("restart".to_string());
        tokens
    } // restart_command_args
    fn attach_command_args(&self, override_path: &Path) -> Vec<String> {
        let mut tokens = self.compose_base_args(override_path);
        tokens.extend([
            "exec".to_string(),
            "-u".to_string(),
            self.user.clone(),
            "-w".to_string(),
            self.workspace_folder.clone(),
            self.service.clone(),
            self.shell.clone(),
        ]);
        tokens
    } // attach_command_args
    fn exec_command_args(&self, override_path: &Path, shell_cmd: &str) -> Vec<String> {
        let mut tokens = self.compose_base_args(override_path);
        tokens.extend([
            "exec".to_string(),
            "-u".to_string(),
            self.user.clone(),
            "-w".to_string(),
            self.workspace_folder.clone(),
            self.service.clone(),
            "sh".to_string(),
            "-c".to_string(),
            shell_cmd.to_string(),
        ]);
        tokens
    } // exec_command_args
    fn exec_raw_command_args(
        &self,
        override_path: &Path,
        prog: &str,
        extra: &[&str],
    ) -> Vec<String> {
        let mut tokens = self.compose_base_args(override_path);
        tokens.extend([
            "exec".to_string(),
            "-u".to_string(),
            self.user.clone(),
            "-w".to_string(),
            self.workspace_folder.clone(),
            self.service.clone(),
            prog.to_string(),
        ]);
        tokens.extend(extra.iter().map(|s| (*s).to_string()));
        tokens
    } // exec_raw_command_args
} // impl PodmanCompose

impl Provider for PodmanCompose {
    fn build(&self, use_cache: bool) -> io::Result<bool> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.build_command_args(&guard.0, use_cache));
        run_and_check(&mut cmd)
    } // build
    fn create(&self, _opts: &ContainerOptions) -> io::Result<bool> {
        Ok(true)
    }
    fn start(&self) -> io::Result<bool> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.start_command_args(&guard.0));
        run_and_check(&mut cmd)
    } // start
    fn stop(&self) -> io::Result<bool> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.stop_command_args(&guard.0));
        run_and_check(&mut cmd)
    } // stop
    fn restart(&self) -> io::Result<bool> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.restart_command_args(&guard.0));
        run_and_check(&mut cmd)
    } // restart
    fn attach(&self) -> io::Result<bool> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.attach_command_args(&guard.0));
        run_and_check(&mut cmd)
    } // attach
    fn rm(&self) -> io::Result<bool> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.rm_args(&guard.0));
        run_and_check(&mut cmd)
    } // rm
    fn exists(&self) -> io::Result<bool> {
        // Use `podman ps -aq` with project + service label filters so this works even when
        // podman-compose is not on the PATH, catches stopped containers, and is scoped to
        // the specific service (not sibling services in the same project).
        let out = Command::new(&self.podman_command)
            .args([
                "ps",
                "-aq",
                "--filter",
                &format!("label=io.podman.compose.project={}", &self.name),
                "--filter",
                &format!("label=com.docker.compose.service={}", &self.service),
            ])
            .output()?;
        let text = String::from_utf8(out.stdout).unwrap_or_default();
        Ok(!text.trim().is_empty())
    } // exists
    fn running(&self) -> io::Result<bool> {
        let out = Command::new(&self.podman_command)
            .args(self.running_args())
            .output()?;
        let text = String::from_utf8(out.stdout).unwrap_or_default();
        Ok(!text.trim().is_empty())
    } // running
    fn cp(&self, source: String, destination: String) -> io::Result<bool> {
        // podman-compose has no native cp; find the service container via project label
        // and delegate to `podman cp`.
        let out = Command::new(&self.podman_command)
            .args([
                "ps",
                "-q",
                "--filter",
                &format!("label=io.podman.compose.project={}", &self.name),
                "--filter",
                &format!("label=io.podman.compose.service={}", &self.service),
            ])
            .output()?;
        let raw_text = String::from_utf8(out.stdout).unwrap_or_default();
        let container_id = Self::extract_container_id(&raw_text).to_string();
        if container_id.is_empty() {
            return Ok(false);
        }
        let mut cp_cmd = Command::new(&self.podman_command);
        cp_cmd
            .arg("cp")
            .arg(source)
            .arg(format!("{container_id}:{destination}"));
        run_and_check(&mut cp_cmd)
    } // cp
    fn exec_capture(&self, cmd: &str) -> crate::error::Result<ExecOutput> {
        let guard = self
            .create_docker_compose()
            .map_err(crate::error::Error::Io)?;
        let mut proc = Command::new(&self.command);
        proc.args(self.exec_command_args(&guard.0, cmd));
        print_command(&proc);
        output_to_exec_result(proc.output().map_err(crate::error::Error::Io)?)
    } // exec_capture
    fn exec(&self, cmd: String) -> io::Result<()> {
        let guard = self.create_docker_compose()?;
        let mut proc = Command::new(&self.command);
        proc.args(self.exec_command_args(&guard.0, &cmd));
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
    fn exec_raw(&self, prog: &str, args: &[&str]) -> io::Result<()> {
        let guard = self.create_docker_compose()?;
        let mut child = Command::new(&self.command);
        child.args(self.exec_raw_command_args(&guard.0, prog, args));
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
} // impl Provider for PodmanCompose

#[cfg(test)]
mod tests {
    use super::*;

    fn make_provider(name: &str, service: &str) -> PodmanCompose {
        PodmanCompose::new(
            HashMap::new(),
            "podman-compose".to_string(),
            vec![],
            "docker-compose.yml".to_string(),
            name.to_string(),
            "podman".to_string(),
            false,
            service.to_string(),
            "/bin/bash".to_string(),
            "vscode".to_string(),
            "/workspace".to_string(),
        )
    }

    // --- extract_container_id ---

    #[test]
    fn extract_container_id_single_line() {
        let output = "abc123\n";
        assert_eq!(PodmanCompose::extract_container_id(output), "abc123");
    }

    #[test]
    fn extract_container_id_blank_leading_lines() {
        let output = "\n\n  \ndeadbeef\n";
        assert_eq!(PodmanCompose::extract_container_id(output), "deadbeef");
    }

    #[test]
    fn extract_container_id_empty() {
        assert_eq!(PodmanCompose::extract_container_id(""), "");
        assert_eq!(PodmanCompose::extract_container_id("   \n  \n"), "");
    }

    // --- running_args ---

    #[test]
    fn running_args_includes_status_running_filter() {
        let p = make_provider("myproject", "app");
        let args = p.running_args();
        // Must contain --filter status=running
        let status_idx = args
            .iter()
            .position(|a| a == "--filter")
            .expect("--filter missing");
        assert_eq!(args[status_idx + 1], "status=running");
    }

    #[test]
    fn running_args_does_not_include_format_flag() {
        let p = make_provider("myproject", "app");
        let args = p.running_args();
        assert!(
            !args.iter().any(|a| a.contains("--format")),
            "running_args must not contain --format, got: {args:?}"
        );
    }

    #[test]
    fn running_args_includes_project_label_filter() {
        let p = make_provider("testproj", "svc");
        let args = p.running_args();
        assert!(
            args.iter()
                .any(|a| a.contains("io.podman.compose.project=testproj")),
            "running_args must filter by project label, got: {args:?}"
        );
    }

    // --- rm_args ---

    #[test]
    fn rm_args_does_not_include_rmi() {
        let p = make_provider("myproject", "app");
        let args = p.rm_args(Path::new("/tmp/override.yml"));
        assert!(
            !args.iter().any(|a| a.contains("--rmi")),
            "rm_args must not contain --rmi, got: {args:?}"
        );
    }

    #[test]
    fn rm_args_includes_down_and_remove_orphans() {
        let p = make_provider("myproject", "app");
        let args = p.rm_args(Path::new("/tmp/override.yml"));
        assert!(
            args.iter().any(|a| a == "down"),
            "rm_args must include 'down'"
        );
        assert!(
            args.iter().any(|a| a == "--remove-orphans"),
            "rm_args must include '--remove-orphans'"
        );
    }

    #[test]
    fn rm_args_includes_project_name() {
        let p = make_provider("projname", "app");
        let args = p.rm_args(Path::new("/override.yml"));
        let p_idx = args.iter().position(|a| a == "-p").expect("-p missing");
        assert_eq!(args[p_idx + 1], "projname");
    }

    // --- compose_base_args ---

    #[test]
    fn compose_base_args_includes_compose_file() {
        let p = make_provider("proj", "svc");
        let args = p.compose_base_args(Path::new("/override.yml"));
        let idx = args.iter().position(|a| a == "-f").expect("-f missing");
        assert_eq!(args[idx + 1], "docker-compose.yml");
    }

    #[test]
    fn compose_base_args_includes_override_file() {
        let p = make_provider("proj", "svc");
        let args = p.compose_base_args(Path::new("/my/override.yml"));
        let positions: Vec<usize> = args
            .iter()
            .enumerate()
            .filter(|(_, a)| *a == "-f")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(positions.len(), 2, "must have two -f flags");
        assert_eq!(args[positions[1] + 1], "/my/override.yml");
    }

    #[test]
    fn compose_base_args_includes_project_name() {
        let p = make_provider("myproj", "svc");
        let args = p.compose_base_args(Path::new("/override.yml"));
        let idx = args.iter().position(|a| a == "-p").expect("-p missing");
        assert_eq!(args[idx + 1], "myproj");
    }

    // --- build_command_args ---

    #[test]
    fn build_command_args_includes_build_subcommand() {
        let p = make_provider("proj", "svc");
        let args = p.build_command_args(Path::new("/override.yml"), true);
        assert!(args.iter().any(|a| a == "build"), "must include 'build'");
    }

    #[test]
    fn build_command_args_no_cache_flag() {
        let p = make_provider("proj", "svc");
        let args = p.build_command_args(Path::new("/override.yml"), false);
        assert!(
            args.iter().any(|a| a == "--no-cache"),
            "must include --no-cache"
        );
    }

    #[test]
    fn build_command_args_with_cache_no_no_cache_flag() {
        let p = make_provider("proj", "svc");
        let args = p.build_command_args(Path::new("/override.yml"), true);
        assert!(
            !args.iter().any(|a| a == "--no-cache"),
            "must not include --no-cache"
        );
    }

    #[test]
    fn build_command_args_includes_build_arg_flags() {
        let mut p = make_provider("proj", "svc");
        p.build_args.insert("FOO".to_string(), "bar".to_string());
        let args = p.build_command_args(Path::new("/override.yml"), true);
        let idx = args
            .iter()
            .position(|a| a == "--build-arg")
            .expect("--build-arg missing");
        assert_eq!(args[idx + 1], "FOO=bar");
    }

    // --- start_command_args ---

    #[test]
    fn start_command_args_includes_up_detach() {
        let p = make_provider("proj", "svc");
        let args = p.start_command_args(Path::new("/override.yml"));
        assert!(args.iter().any(|a| a == "up"), "must include 'up'");
        assert!(
            args.iter().any(|a| a == "--detach"),
            "must include '--detach'"
        );
    }

    // --- stop_command_args ---

    #[test]
    fn stop_command_args_includes_stop_subcommand() {
        let p = make_provider("proj", "svc");
        let args = p.stop_command_args(Path::new("/override.yml"));
        assert!(args.iter().any(|a| a == "stop"), "must include 'stop'");
    }

    // --- restart_command_args ---

    #[test]
    fn restart_command_args_includes_restart_subcommand() {
        let p = make_provider("proj", "svc");
        let args = p.restart_command_args(Path::new("/override.yml"));
        assert!(
            args.iter().any(|a| a == "restart"),
            "must include 'restart'"
        );
    }

    // --- attach_command_args ---

    #[test]
    fn attach_command_args_includes_exec_and_service() {
        let p = make_provider("proj", "app");
        let args = p.attach_command_args(Path::new("/override.yml"));
        assert!(args.iter().any(|a| a == "exec"), "must include 'exec'");
        assert!(args.iter().any(|a| a == "app"), "must include service name");
    }

    #[test]
    fn attach_command_args_includes_shell() {
        let p = make_provider("proj", "svc");
        let args = p.attach_command_args(Path::new("/override.yml"));
        assert!(args.iter().any(|a| a == "/bin/bash"), "must include shell");
    }

    #[test]
    fn attach_command_args_includes_user_and_workspace() {
        let p = make_provider("proj", "svc");
        let args = p.attach_command_args(Path::new("/override.yml"));
        let u_idx = args.iter().position(|a| a == "-u").expect("-u missing");
        assert_eq!(args[u_idx + 1], "vscode");
        let w_idx = args.iter().position(|a| a == "-w").expect("-w missing");
        assert_eq!(args[w_idx + 1], "/workspace");
    }

    // --- exec_command_args ---

    #[test]
    fn exec_command_args_passes_cmd_via_sh_c() {
        let p = make_provider("proj", "svc");
        let args = p.exec_command_args(Path::new("/override.yml"), "npm install");
        // Must end with sh -c <cmd>
        let len = args.len();
        assert!(len >= 3);
        assert_eq!(args[len - 3], "sh");
        assert_eq!(args[len - 2], "-c");
        assert_eq!(args[len - 1], "npm install");
    }

    // --- exec_raw_command_args ---

    #[test]
    fn exec_raw_command_args_places_prog_after_service() {
        let p = make_provider("proj", "svc");
        let args = p.exec_raw_command_args(Path::new("/override.yml"), "myprogram", &["--flag"]);
        // service comes before prog
        let svc_idx = args
            .iter()
            .position(|a| a == "svc")
            .expect("service missing");
        assert_eq!(args[svc_idx + 1], "myprogram");
        assert_eq!(args[svc_idx + 2], "--flag");
    }

    #[test]
    fn exec_raw_command_args_no_sh_wrapper() {
        let p = make_provider("proj", "svc");
        let args = p.exec_raw_command_args(Path::new("/override.yml"), "myprog", &[]);
        assert!(!args.iter().any(|a| a == "sh"), "exec_raw must not use sh");
        assert!(!args.iter().any(|a| a == "-c"), "exec_raw must not use -c");
    }
}
