use super::options::ContainerOptions;
use super::print_command;
use super::utils::{ComposeOverrideGuard, create_compose_override, run_and_check};
use super::{ExecOutput, Provider, output_to_exec_result};
use std::{collections::HashMap, io, path::Path, process::Command};
/// Docker Compose provider -- manages containers via `docker compose`.
#[derive(Debug)] // DockerCompose
pub struct DockerCompose {
    // compose provider
    /// Build-time `--build-arg` key/value pairs.
    pub(crate) build_args: HashMap<String, String>,
    /// CLI binary name (typically `"docker"`).
    pub(crate) command: String,
    /// Environment variables to inject into the compose override.
    pub(crate) env_vars: Vec<(String, String)>,
    /// Path to the docker-compose.yml file.
    pub(crate) file: String,
    /// Compose project name used for lifecycle commands.
    pub(crate) name: String,
    /// Service name within the compose file.
    pub(crate) service: String,
    /// Shell to use for attach (typically `"sh"`).
    pub(crate) shell: String,
    /// User to run commands as inside the container.
    pub(crate) user: String,
    /// Path inside the container where the workspace is mounted.
    pub(crate) workspace_folder: String,
} // DockerCompose
impl DockerCompose {
    // methods
    /// Create a new Docker Compose provider instance.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        build_args: HashMap<String, String>,
        command: String,
        env_vars: Vec<(String, String)>,
        file: String,
        name: String,
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
            service,
            shell,
            user,
            workspace_folder,
        }
    } // new
    fn create_docker_compose(&self) -> io::Result<ComposeOverrideGuard> {
        create_compose_override(&self.service, &self.env_vars, false, &self.build_args)
    } // create_docker_compose
    /// Build the base compose args: `compose -f <file> -f <override> -p <name>`.
    fn compose_base_args(&self, override_path: &Path) -> Vec<String> {
        vec![
            "compose".to_string(),
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
    fn rm_args(&self, override_path: &Path) -> Vec<String> {
        let mut tokens = self.compose_base_args(override_path);
        tokens.extend([
            "down".to_string(),
            "--remove-orphans".to_string(),
            "--rmi".to_string(),
            "all".to_string(),
        ]);
        tokens
    } // rm_args
    fn cp_args(&self, override_path: &Path, src: &str, dst: &str) -> Vec<String> {
        let mut tokens = self.compose_base_args(override_path);
        tokens.extend([
            "cp".to_string(),
            src.to_string(),
            format!("{}:{dst}", &self.service),
        ]);
        tokens
    } // cp_args
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
    fn running_args(&self) -> Vec<String> {
        vec![
            "compose".to_string(),
            "-f".to_string(),
            self.file.clone(),
            "-p".to_string(),
            self.name.clone(),
            "ps".to_string(),
            "-q".to_string(),
            "--status=running".to_string(),
        ]
    } // running_args
    fn exists_args(&self) -> Vec<String> {
        vec![
            "compose".to_string(),
            "-f".to_string(),
            self.file.clone(),
            "-p".to_string(),
            self.name.clone(),
            "ps".to_string(),
            "-aq".to_string(),
            "--format".to_string(),
            "json".to_string(),
            self.service.clone(),
        ]
    } // exists_args
} // impl DockerCompose

impl Provider for DockerCompose {
    fn build(&self, use_cache: bool) -> io::Result<()> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.build_command_args(&guard.0, use_cache));
        run_and_check(&mut cmd)
    } // build
    fn create(&self, _opts: &ContainerOptions) -> io::Result<()> {
        Ok(())
    }
    fn start(&self) -> io::Result<()> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.start_command_args(&guard.0));
        run_and_check(&mut cmd)
    } // start
    fn stop(&self) -> io::Result<()> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.stop_command_args(&guard.0));
        run_and_check(&mut cmd)
    } // stop
    fn restart(&self) -> io::Result<()> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.restart_command_args(&guard.0));
        run_and_check(&mut cmd)
    } // restart
    fn attach(&self) -> io::Result<()> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.attach_command_args(&guard.0));
        run_and_check(&mut cmd)
    } // attach
    fn rm(&self) -> io::Result<()> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.rm_args(&guard.0));
        run_and_check(&mut cmd)
    } // rm
    fn exists(&self) -> io::Result<bool> {
        let out = Command::new(&self.command)
            .args(self.exists_args())
            .output()?;
        Ok(compose_service_exists(out.stdout))
    } // exists
    fn running(&self) -> io::Result<bool> {
        let out = Command::new(&self.command)
            .args(self.running_args())
            .output()?;
        let text = String::from_utf8(out.stdout).unwrap_or_default();
        Ok(!text.trim().is_empty())
    } // running
    fn cp(&self, source: String, destination: String) -> io::Result<()> {
        let guard = self.create_docker_compose()?;
        let mut cmd = Command::new(&self.command);
        cmd.args(self.cp_args(&guard.0, &source, &destination));
        run_and_check(&mut cmd)
    } // cp
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
    fn exec_capture(&self, cmd: &str) -> crate::error::Result<ExecOutput> {
        let guard = self
            .create_docker_compose()
            .map_err(crate::error::Error::Io)?;
        let mut proc = Command::new(&self.command);
        proc.args(self.exec_command_args(&guard.0, cmd));
        print_command(&proc);
        output_to_exec_result(proc.output().map_err(crate::error::Error::Io)?)
    } // exec_capture
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
} // impl Provider for DockerCompose

/// Returns true if the `docker compose ps --format json` output indicates at least one container.
///
/// Handles empty output, JSON `[]`, and `null` as "no container". Any other non-empty content
/// means a container record was returned. Also handles newline-delimited JSON (one object per line).
pub(crate) fn compose_service_exists(raw_output: Vec<u8>) -> bool {
    let text = String::from_utf8(raw_output).unwrap_or_default();
    let stripped = text.trim();
    !matches!(stripped, "" | "[]" | "null")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_provider(name: &str, service: &str) -> DockerCompose {
        DockerCompose::new(
            HashMap::new(),
            "docker".to_string(),
            vec![],
            "docker-compose.yml".to_string(),
            name.to_string(),
            service.to_string(),
            "/bin/bash".to_string(),
            "vscode".to_string(),
            "/workspace".to_string(),
        )
    }

    // --- compose_service_exists ---

    #[test]
    fn compose_service_exists_empty_output_is_false() {
        assert!(!compose_service_exists(vec![]));
    }

    #[test]
    fn compose_service_exists_empty_array_is_false() {
        assert!(!compose_service_exists(b"[]".to_vec()));
    }

    #[test]
    fn compose_service_exists_null_is_false() {
        assert!(!compose_service_exists(b"null".to_vec()));
    }

    #[test]
    fn compose_service_exists_populated_array_is_true() {
        let json = br#"[{"ID":"abc","Name":"proj-svc-1","State":"running"}]"#;
        assert!(compose_service_exists(json.to_vec()));
    }

    #[test]
    fn compose_service_exists_newline_delimited_object_is_true() {
        let json = br#"{"ID":"abc","Name":"proj-svc-1","State":"running"}"#;
        assert!(compose_service_exists(json.to_vec()));
    }

    // --- compose_base_args ---

    #[test]
    fn compose_base_args_starts_with_compose_subcommand() {
        let p = make_provider("proj", "svc");
        let args = p.compose_base_args(Path::new("/override.yml"));
        assert_eq!(args[0], "compose");
    }

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

    // --- rm_args ---

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
    fn rm_args_includes_rmi_all() {
        let p = make_provider("myproject", "app");
        let args = p.rm_args(Path::new("/tmp/override.yml"));
        let idx = args
            .iter()
            .position(|a| a == "--rmi")
            .expect("--rmi missing");
        assert_eq!(args[idx + 1], "all");
    }

    #[test]
    fn rm_args_includes_project_name() {
        let p = make_provider("projname", "app");
        let args = p.rm_args(Path::new("/override.yml"));
        let p_idx = args.iter().position(|a| a == "-p").expect("-p missing");
        assert_eq!(args[p_idx + 1], "projname");
    }

    // --- cp_args ---

    #[test]
    fn cp_args_includes_cp_subcommand() {
        let p = make_provider("proj", "svc");
        let args = p.cp_args(Path::new("/override.yml"), "/src/file", "/dst/file");
        assert!(args.iter().any(|a| a == "cp"), "must include 'cp'");
    }

    #[test]
    fn cp_args_formats_destination_with_service() {
        let p = make_provider("proj", "svc");
        let args = p.cp_args(Path::new("/override.yml"), "/src/file", "/dst/file");
        assert!(
            args.iter().any(|a| a == "svc:/dst/file"),
            "must format destination as service:path, got: {args:?}"
        );
    }

    // --- exec_command_args ---

    #[test]
    fn exec_command_args_passes_cmd_via_sh_c() {
        let p = make_provider("proj", "svc");
        let args = p.exec_command_args(Path::new("/override.yml"), "npm install");
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
        assert!(!args.iter().any(|a| a == "-c"), "exec_raw must not use -c");
    }

    // --- running_args ---

    #[test]
    fn running_args_includes_status_running() {
        let p = make_provider("myproject", "app");
        let args = p.running_args();
        assert!(
            args.iter().any(|a| a == "--status=running"),
            "running_args must contain --status=running, got: {args:?}"
        );
    }

    #[test]
    fn running_args_starts_with_compose() {
        let p = make_provider("myproject", "app");
        let args = p.running_args();
        assert_eq!(args[0], "compose");
    }

    // --- exists_args ---

    #[test]
    fn exists_args_includes_format_json() {
        let p = make_provider("myproject", "app");
        let args = p.exists_args();
        let idx = args
            .iter()
            .position(|a| a == "--format")
            .expect("--format missing");
        assert_eq!(args[idx + 1], "json");
    }

    #[test]
    fn exists_args_includes_service_name() {
        let p = make_provider("proj", "mysvc");
        let args = p.exists_args();
        assert_eq!(args.last().unwrap(), "mysvc");
    }
}
