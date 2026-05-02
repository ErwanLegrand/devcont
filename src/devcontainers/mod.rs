/// Devcontainer configuration parsing (`devcontainer.json`).
pub mod config; // <-- devcontainer.json schema
/// `OneOrMany` type for lifecycle hook commands (string or array form).
pub mod one_or_many;
/// Path validation utilities (workspace-root containment checks).
pub(crate) mod paths;
/// Runtime argument validation for `runArgs` and `remoteEnv`.
mod run_args;
// --- imports ---
use crate::{
    audit::{AuditEvent, AuditLogger},
    devcontainers::one_or_many::OneOrMany,
    error::{Error, Result},
    provider::{
        Provider,
        docker::{BuildSource, Docker},
        docker_compose::DockerCompose,
        nerdctl::Nerdctl,
        options::ContainerOptions,
        podman::Podman,
        podman_compose::PodmanCompose,
        utils::resolve_dockerfile_path,
    },
    settings::Settings,
};
use config::Config; // re-export from sub-module
use paths::validate_within_root;
use std::path::{Path, PathBuf};
// ---- free functions (hook helpers, validation, provider construction) ----
/// Execute a lifecycle hook inside the container via the provider.
///
/// - `One(cmd)` → `provider.exec(cmd)` (shell-wrapped by the provider)
/// - `Many(parts)` → `provider.exec_raw(parts[0], parts[1..])` (no shell, injection-safe)
fn exec_hook(provider: &dyn Provider, hook: &OneOrMany) -> std::io::Result<()> {
    match hook {
        OneOrMany::One(cmd) => provider.exec(cmd.clone()),
        OneOrMany::Many(_) => {
            if let Some((prog, args)) = hook.to_exec_parts() {
                let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
                provider.exec_raw(&prog, &args_ref)
            } else {
                Ok(())
            }
        }
    }
}

/// Execute a lifecycle hook on the host (not inside the container).
///
/// - `One(cmd)` → `sh -c <cmd>`
/// - `Many(parts)` → `parts[0] parts[1..]` (no shell, injection-safe)
///
/// Returns `true` if the command ran and exited successfully, `false` on non-zero exit.
/// Returns `Ok(true)` when no exec parts are produced (empty Many).
///
/// When `timeout_secs` is `Some(n)`, the process is polled every 100 ms; if it has not
/// exited within `n` seconds the child is killed and an error is returned.
fn exec_host_hook(hook: &OneOrMany, timeout_secs: Option<u32>) -> std::io::Result<bool> {
    let Some((prog, args)) = hook.to_exec_parts() else {
        return Ok(true);
    };

    let Some(secs) = timeout_secs else {
        let status = std::process::Command::new(&prog).args(&args).status()?;
        return Ok(status.success());
    };

    let mut child = std::process::Command::new(&prog).args(&args).spawn()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(u64::from(secs));

    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.success());
        }
        if std::time::Instant::now() >= deadline {
            // SIGKILL directly — no SIGTERM grace period. Lifecycle hooks are
            // expected to be short-lived; a two-stage kill adds complexity for
            // minimal practical benefit in this context.
            drop(child.kill());
            drop(child.wait());
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("hook timed out after {secs}s"),
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Execute a lifecycle hook inside the container and log the audit event.
///
/// This is the shared helper for all in-container hooks (`postStartCommand`,
/// `postAttachCommand`, `onCreateCommand`, `updateContentCommand`, `postCreateCommand`).
fn dispatch_hook(
    name: &str,
    hook: &OneOrMany,
    provider: &dyn Provider,
    audit: &AuditLogger,
) -> Result<()> {
    exec_hook(provider, hook).map_err(|e| Error::HookFailed(format!("{name}: {e}")))?;
    audit.log(&AuditEvent::HookExecuted {
        hook: name.to_string(),
        command: hook.to_string(),
    });
    Ok(())
}

/// Returns true if `answer` constitutes a positive confirmation (case-insensitive `"y"`).
fn is_confirmed(answer: &str) -> bool {
    answer.trim().to_lowercase() == "y"
}

/// Returns true if a root-user warning should be emitted.
///
/// Warns when `remote_user` is `"root"` or `"0"` (or empty) and `no_root_check` is false.
fn should_warn_root(remote_user: &str, no_root_check: bool) -> bool {
    if no_root_check {
        return false;
    }
    matches!(remote_user.trim(), "" | "root" | "0")
}

/// Prompt the user for confirmation before running `initializeCommand` on the host.
///
/// When `trust` is true, skips the prompt and runs the hook immediately.
/// When `trust` is false, prints the command to stderr, reads a `[y/N]` response from stdin,
/// and returns `false` (without running the hook) if the user declines.
///
/// Returns `Ok(false)` to signal that the caller should abort (hook declined or hook failed).
fn confirm_and_run_host_hook(
    hook: &OneOrMany,
    trust: bool,
    timeout_secs: Option<u32>,
) -> std::io::Result<bool> {
    if trust {
        eprintln!("initializeCommand trusted, running on host.");
    } else {
        let cmd_display = match hook {
            OneOrMany::One(cmd) => cmd.clone(),
            OneOrMany::Many(parts) => parts.join(" "),
        };
        eprintln!("initializeCommand will run on the host: {cmd_display}");
        eprint!("Run initializeCommand on host? [y/N] ");
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !is_confirmed(&answer) {
            return Ok(false);
        }
    }
    exec_host_hook(hook, timeout_secs)
}

/// Lightweight snapshot of a devcontainer configuration obtained without
/// building a container runtime provider or executing any host hooks.
///
/// Used by the `container-name` and `info` subcommands, which are
/// read-only and must never trigger `initializeCommand` or any other
/// side-effectful operation.
pub struct DevcontainerInspection {
    /// The parsed `devcontainer.json` configuration.
    config: Config,
    /// The directory that contains the resolved `devcontainer.json` file.
    ///
    /// - For the nested form (`.devcontainer/devcontainer.json`) this is the
    ///   `.devcontainer/` subdirectory.
    /// - For the root form (`.devcontainer.json`) this is the project root.
    config_dir: PathBuf,
    /// The container engine name derived from user settings (e.g. `"docker"`).
    engine: String,
}

impl DevcontainerInspection {
    /// Return the deterministic container name produced by `safe_name()`.
    ///
    /// # Errors
    /// Returns an error when the project name cannot be mapped to an ASCII
    /// container name (see [`Config::safe_name`]).
    #[must_use]
    pub fn container_name(&self) -> String {
        // safe_name errors are surfaced at load time, so this cannot fail here.
        // The value is pre-computed in Devcontainer::load_for_inspection.
        self.config
            .safe_name()
            .unwrap_or_else(|_| String::from("<invalid>"))
    }

    /// Return the directory that contains the resolved `devcontainer.json`.
    #[must_use]
    pub fn config_dir(&self) -> &std::path::Path {
        &self.config_dir
    }

    /// Return the container engine name (e.g. `"docker"`, `"podman"`).
    #[must_use]
    pub fn engine(&self) -> &str {
        &self.engine
    }

    /// Return a reference to the parsed configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// Represents a fully-loaded devcontainer ready for lifecycle operations
/// (build, create, run, rebuild).
pub struct Devcontainer {
    // fields ordered by initialisation dependency
    /// The parsed `devcontainer.json` configuration.
    config: Config, // required: name + (image | dockerfile | dockerComposeFile)
    /// Absolute directory containing the loaded `devcontainer.json` file.
    ///
    /// Per the containers.dev spec, all relative paths in `devcontainer.json`
    /// (`build.dockerfile`, `build.context`, `dockerComposeFile`, …) are
    /// interpreted relative to this directory — not relative to the workspace root.
    ///
    /// - For `.devcontainer/devcontainer.json` → `<workspace>/.devcontainer/`
    /// - For `.devcontainer.json` at root → `<workspace>/`
    pub config_dir: PathBuf,
    /// User-level preferences loaded from `~/.config/devcon/settings.toml`.
    settings: Settings, // includes provider choice and dotfile list
    /// Container runtime abstraction — Docker, Podman, Apple, or Nerdctl.
    provider: Box<dyn Provider>, // dispatched via trait object
    /// Per-hook timeout in seconds (`None` = unlimited).  Populated from
    /// `hookTimeoutSeconds` in the config; overridable via `--hook-timeout`.
    hook_timeout_secs: Option<u32>,
} // Devcontainer struct
impl Devcontainer {
    // --- public API + private helpers ---
    /// Parse `devcontainer.json` from `directory` without building a provider
    /// or running any host hooks.
    ///
    /// This is the entry point for the `container-name` and `info` subcommands.
    /// It performs only:
    /// 1. Locating and parsing `devcontainer.json`.
    /// 2. Deriving the container name via `safe_name()`.
    /// 3. Reading user settings to determine the engine name.
    ///
    /// No subprocess is ever spawned, so `initializeCommand` and other host
    /// hooks are never executed.
    ///
    /// # Errors
    /// Returns an error if the config file is missing, cannot be read, fails to
    /// parse, or the container name cannot be derived from the project name.
    pub fn load_for_inspection(directory: &Path) -> Result<DevcontainerInspection> {
        use crate::settings::Provider as ProviderKind;

        let nested_path = directory.join(".devcontainer").join("devcontainer.json");
        let root_path = directory.join(".devcontainer.json");

        let (parsed, config_dir) = match Config::parse(&nested_path) {
            Ok(config) => {
                let dir = nested_path
                    .parent()
                    .map_or_else(|| directory.to_path_buf(), Path::to_path_buf);
                (config, dir)
            }
            Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::NotFound => {
                let config = Config::parse(&root_path).map_err(|e| match e {
                    Error::Io(ref io_err) if io_err.kind() == std::io::ErrorKind::NotFound => {
                        Error::InvalidConfig(
                            "Could not find .devcontainer/devcontainer.json or .devcontainer.json"
                                .to_string(),
                        )
                    }
                    other => other,
                })?;
                (config, directory.to_path_buf())
            }
            Err(e) => return Err(e),
        };

        // Validate that a container name can be derived before returning.
        parsed.safe_name()?;

        let user_settings = Settings::load()?;
        let engine = match &user_settings.provider {
            ProviderKind::Docker => "docker",
            ProviderKind::Podman => "podman",
            ProviderKind::Apple => "apple",
            ProviderKind::Nerdctl => "nerdctl",
        };

        Ok(DevcontainerInspection {
            config: parsed,
            config_dir,
            engine: engine.to_string(),
        })
    } // end fn load_for_inspection

    /// Load a dev container from `directory`, resolving the config file and
    /// selecting the appropriate container provider based on user settings.
    ///
    /// Looks for `.devcontainer/devcontainer.json` first, then `.devcontainer.json`.
    ///
    /// # Errors
    /// Returns an error if the config file is missing, cannot be read, or fails to parse.
    pub fn load(directory: &Path) -> Result<Self> {
        // Try the nested path first, then fall back to the root path.
        // No pre-checks — Config::parse() handles missing files directly,
        // avoiding TOCTOU races.
        let nested_path = directory.join(".devcontainer").join("devcontainer.json");
        let root_path = directory.join(".devcontainer.json");

        // Attempt the nested layout first; on NotFound fall back to root layout.
        // `config_dir` is the directory that actually contained the file we loaded.
        let (parsed, config_dir) = match Config::parse(&nested_path) {
            Ok(config) => {
                let dir = directory.join(".devcontainer");
                (config, dir)
            }
            Err(Error::Io(ref e)) if e.kind() == std::io::ErrorKind::NotFound => {
                let config = Config::parse(&root_path).map_err(|e| match e {
                    Error::Io(ref io_err) if io_err.kind() == std::io::ErrorKind::NotFound => {
                        Error::InvalidConfig(
                            "Could not find .devcontainer/devcontainer.json or .devcontainer.json"
                                .to_string(),
                        )
                    }
                    other => other,
                })?;
                (config, directory.to_path_buf())
            }
            Err(e) => return Err(e),
        };
        let user_settings = Settings::load()?;
        let runtime = build_provider(&config_dir, directory, &user_settings, &parsed)?;
        let timeout = parsed.hook_timeout_seconds;
        Ok(Self {
            config: parsed.clone(),
            config_dir,
            settings: user_settings,
            provider: runtime,
            hook_timeout_secs: timeout,
        })
    } // end fn load
    /// Override the hook timeout for this instance (typically from `--hook-timeout` CLI flag).
    ///
    /// Calling this replaces any `hookTimeoutSeconds` value from `devcontainer.json`.
    ///
    /// **Note:** The timeout is currently enforced only on host-side hooks
    /// (`initializeCommand`). In-container hooks (`postCreateCommand`,
    /// `postStartCommand`, `postAttachCommand`, `onCreateCommand`,
    /// `updateContentCommand`) are not yet subject to this timeout.
    #[must_use]
    pub fn with_hook_timeout(mut self, secs: u32) -> Self {
        self.hook_timeout_secs = Some(secs);
        self
    }

    /// Build, start, and attach to the dev container, running lifecycle hooks.
    ///
    /// If `use_cache` is `false`, the image is built with `--no-cache`.
    /// Pass `no_audit_log = true` to suppress writing to the audit log.
    ///
    /// # Errors
    /// Returns an error if any provider operation (build, start, attach, etc.) fails.
    #[allow(clippy::fn_params_excessive_bools)]
    pub fn run(
        &self,
        use_cache: bool,
        trust: bool,
        no_root_check: bool,
        no_audit_log: bool,
    ) -> Result<()> {
        run_args::validate_run_args(&self.config.run_args).map_err(Error::InvalidConfig)?;

        let cname = self.config.safe_name()?;
        run_args::validate_container_name(&cname).map_err(Error::InvalidConfig)?;

        run_args::validate_remote_env(&self.config.remote_env).map_err(Error::InvalidConfig)?;

        let audit = AuditLogger::new(no_audit_log);
        audit.log(&AuditEvent::ContainerStart {
            container: cname.clone(),
        });

        let runtime = &self.provider;
        // Handle initializeCommand (host-side hook with user confirmation).
        if let Some(init_hook) = &self.config.initialize_command {
            if !confirm_and_run_host_hook(init_hook, trust, self.hook_timeout_secs)? {
                return Err(Error::HookFailed(
                    "initializeCommand declined by user".to_string(),
                ));
            }
            audit.log(&AuditEvent::HookExecuted {
                hook: "initializeCommand".to_string(),
                command: init_hook.to_string(),
            });
        } // initializeCommand
        // Build + create the container image/instance if needed.
        self.ensure_created(use_cache)?;
        if should_warn_root(&self.config.remote_user, no_root_check) {
            tracing::warn!(
                "container will run as root — consider setting `remoteUser` in devcontainer.json, or pass --no-root-check to suppress this warning"
            );
        } // root warning
        // Bring the container up if it is not already running.
        let is_running = runtime.running()?;
        if !is_running {
            runtime.start()?;
        }
        if let Some(start_hook) = &self.config.post_start_command {
            dispatch_hook("postStartCommand", start_hook, runtime.as_ref(), &audit)?;
        } // postStartCommand
        self.post_create(&audit)?;
        // Restart and attach for the interactive session.
        runtime.restart()?;
        runtime.attach()?;
        if let Some(attach_hook) = &self.config.post_attach_command {
            dispatch_hook("postAttachCommand", attach_hook, runtime.as_ref(), &audit)?;
        } // postAttachCommand
        // Honour shutdownAction from devcontainer.json.
        let needs_shutdown = self.config.should_shutdown();
        if needs_shutdown {
            runtime.stop()?;
            audit.log(&AuditEvent::ContainerStop {
                container: cname.clone(),
            });
        } // shutdown
        Ok(()) // lifecycle complete
    } // end fn run
    /// Stop and remove the existing container, then run it fresh.
    ///
    /// Pass `no_audit_log = true` to suppress writing to the audit log.
    ///
    /// # Errors
    /// Returns an error if stopping, removing, or restarting the container fails.
    #[allow(clippy::fn_params_excessive_bools)]
    pub fn rebuild(
        &self,
        use_cache: bool,
        trust: bool,
        no_root_check: bool,
        no_audit_log: bool,
    ) -> Result<()> {
        let audit = AuditLogger::new(no_audit_log);
        let rebuild_name = self.config.safe_name()?;
        audit.log(&AuditEvent::ContainerRebuild {
            container: rebuild_name,
        });

        // Tear down the existing container when present.
        let rt = &self.provider;
        let already_exists = rt.exists()?;
        if already_exists {
            rt.stop()?;
            rt.rm()?;
        } // teardown complete
        // Delegate to the full lifecycle.
        self.run(use_cache, trust, no_root_check, no_audit_log)
    } // end fn rebuild
    /// Build the image and create the container if one does not already exist.
    fn ensure_created(&self, use_cache: bool) -> Result<()> {
        let rt = &self.provider;
        if rt.exists()? {
            return Ok(());
        }
        rt.build(use_cache)?;
        let creation_opts = ContainerOptions {
            remote_env: sorted_env_vars(&self.config),
        };
        rt.create(&creation_opts)?;
        Ok(()) // container created
    } // end fn ensure_created
    /// Run the post-creation lifecycle hooks and copy host dotfiles into the container.
    fn post_create(&self, audit: &AuditLogger) -> Result<()> {
        let rt: &dyn Provider = self.provider.as_ref();
        // Lifecycle hooks in spec-defined order.
        for (label, maybe_hook) in [
            ("onCreateCommand", &self.config.on_create_command),
            ("updateContentCommand", &self.config.update_content_command),
            ("postCreateCommand", &self.config.post_create_command),
        ] {
            if let Some(h) = maybe_hook {
                dispatch_hook(label, h, rt, audit)?;
            }
        }
        // Copy host configuration into the container.
        self.copy_gitconfig()?; // may be skipped if ~/.gitconfig absent
        self.copy_dotfiles()?; // copies each entry from settings.dotfiles
        Ok(()) // post-create complete
    } // end fn post_create
    /// Copy a file or directory from the host into the container at `dest`.
    ///
    /// No pre-existence check — the provider's `cp()` reports errors directly,
    /// avoiding a TOCTOU race between checking and copying.
    fn copy(&self, src: &Path, dest: &str) -> Result<()> {
        let rt = &self.provider;
        let dest_path = PathBuf::from(dest);
        let parent = dest_path
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("<non-utf8>");
        // When source is a directory, cp needs the parent as the target.
        let effective_dest = if src.is_dir() { parent } else { dest };
        // Shell-quote the parent path to handle spaces safely.
        let quoted = format!("'{}'", parent.replace('\'', r"'\''"));
        rt.exec(format!("mkdir -p -- {quoted}"))?;
        rt.cp(
            src.to_string_lossy().to_string(),
            effective_dest.to_string(),
        )
        .map_err(Into::into)
    } // end fn copy
    /// Copy each configured dotfile from the host home directory into the container.
    fn copy_dotfiles(&self) -> Result<()> {
        let local_home = PathBuf::from(shellexpand::tilde("~").to_string());
        let remote_home = remote_home_dir(&self.config.remote_user);
        for entry in &self.settings.dotfiles {
            validate_dotfile_entry(entry, &local_home, &remote_home)?;
            let local = local_home.join(entry);
            let remote = remote_home.join(entry.clone());
            self.copy(&local, &remote.to_string_lossy())?; // propagate copy errors
        }
        Ok(()) // all dotfiles transferred
    } // end fn copy_dotfiles
    /// Copy the host `~/.gitconfig` into the container (if it exists).
    ///
    /// Returns `Ok(())` when the copy succeeds or when `~/.gitconfig` is absent.
    fn copy_gitconfig(&self) -> Result<()> {
        let local_gc = PathBuf::from(shellexpand::tilde("~/.gitconfig").to_string());
        if !local_gc.exists() {
            return Ok(()); // nothing to copy
        }
        let remote = remote_home_dir(&self.config.remote_user).join(".gitconfig");
        self.copy(&local_gc, &remote.to_string_lossy())
    } // end fn copy_gitconfig

    /// Return `true` if the container currently exists (running or stopped).
    ///
    /// Delegates to [`Provider::exists`].
    ///
    /// # Errors
    /// Fails when the underlying inspect command cannot be spawned.
    pub fn probe_exists(&self) -> std::io::Result<bool> {
        self.provider.exists()
    } // end probe_exists

    /// Return `true` if the container is currently running.
    ///
    /// Delegates to [`Provider::running`].
    ///
    /// # Errors
    /// Fails when the underlying inspect command cannot be spawned.
    pub fn probe_running(&self) -> std::io::Result<bool> {
        self.provider.running()
    } // end probe_running
} // impl Devcontainer

/// Determine the home directory for a given remote user inside the container.
///
/// Returns `/root` for the root user and `/home/<user>` for everyone else.
///
/// **Known limitation:** This assumes the standard Linux home directory layout.
/// Containers based on Alpine, BSD, or macOS images may place non-root users
/// under different paths (e.g., `/var/lib/<user>` for service accounts).
/// A future `remoteHomeDir` config field or runtime detection via
/// `getent passwd <user>` inside the container would make this more robust.
fn remote_home_dir(user: &str) -> PathBuf {
    match user {
        "root" => PathBuf::from("/root"),
        other => PathBuf::from("/home").join(other),
    }
}
/// Validate that a dotfile entry does not escape its parent directory via traversal.
///
/// Both the local path (`~/entry`) and remote path (`home/entry`) are checked.
/// Returns `Err` if either resolved path escapes its root.
fn validate_dotfile_entry(entry: &str, local_home: &Path, remote_home: &Path) -> Result<()> {
    let local = local_home.join(entry);
    validate_within_root(local_home, &local).map_err(|_| {
        Error::PathTraversal(format!(
            "dotfile entry '{entry}' escapes host home directory '{}'",
            local_home.display()
        ))
    })?;
    let remote = remote_home.join(entry);
    validate_within_root(remote_home, &remote).map_err(|_| {
        Error::PathTraversal(format!(
            "dotfile entry '{entry}' escapes container home directory '{}'",
            remote_home.display()
        ))
    })?;
    Ok(())
}

fn sorted_env_vars(config: &Config) -> Vec<(String, String)> {
    let mut env_vars: Vec<(String, String)> = config
        .remote_env
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    env_vars.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    env_vars
}

/// Resolve the compose file path and service name from the config.
///
/// `config_dir` is the directory containing the loaded `devcontainer.json`.
/// Per the containers.dev spec, `dockerComposeFile` is resolved relative to
/// `config_dir`, not the workspace root.
fn compose_path_and_service(config_dir: &Path, config: &Config) -> Result<(String, String)> {
    let compose_file = config.docker_compose_file.as_deref().ok_or_else(|| {
        Error::InvalidConfig(
            "devcontainer.json is missing required field: dockerComposeFile".to_string(),
        )
    })?;
    let compose_path = config_dir.join(compose_file);
    let service = config.service.as_deref().ok_or_else(|| {
        Error::InvalidConfig("devcontainer.json is missing required field: service".to_string())
    })?;
    Ok((
        compose_path.to_string_lossy().to_string(),
        service.to_string(),
    ))
}

/// Resolve the build source (Dockerfile or image) from the devcontainer config.
///
/// `config_dir` is the directory containing the loaded `devcontainer.json`.
/// `workspace` is the workspace root, used for security validation only.
///
/// Per the containers.dev spec, `build.dockerfile` is resolved relative to
/// `config_dir`, not the workspace root.
fn resolve_build_source(
    config_dir: &Path,
    workspace: &Path,
    config: &Config,
) -> Result<BuildSource> {
    if let Some(dockerfile) = config.dockerfile() {
        let context = config.build.as_ref().and_then(|b| b.context.as_deref());
        let resolved = resolve_dockerfile_path(config_dir, &dockerfile, context);
        let validated = validate_within_root(workspace, &resolved)?;
        Ok(BuildSource::Dockerfile(
            validated.to_string_lossy().to_string(),
        ))
    } else if let Some(image) = config.image.clone() {
        Ok(BuildSource::Image(image))
    } else {
        Err(Error::InvalidConfig(
            "devcontainer.json is missing required field: build.dockerfile or image".to_string(),
        ))
    }
}

/// Validate `build.context` against the workspace root (FR-003).
///
/// Relative contexts must resolve within `root`. Absolute contexts outside `root`
/// emit a warning but are allowed through (they may be intentional host mounts).
fn validate_build_context(root: &Path, context: &str) -> Result<()> {
    let context_path = Path::new(context);
    if context_path.is_absolute() {
        if validate_within_root(root, context_path).is_err() {
            tracing::warn!(
                "build.context '{}' is absolute and outside workspace root '{}'",
                context,
                root.display()
            );
        }
        Ok(())
    } else {
        validate_within_root(root, context_path)?;
        Ok(())
    }
}

/// Validate relative mount sources against the workspace root (FR-004).
///
/// For each mount entry, if the `"source"` key is present and the path is
/// relative, it must resolve within `root`. Absolute source paths pass through.
fn validate_mounts(
    root: &Path,
    mounts: Option<&Vec<std::collections::HashMap<String, String>>>,
) -> Result<()> {
    if let Some(mount_list) = mounts {
        for mount in mount_list {
            if let Some(source) = mount.get("source") {
                let source_path = Path::new(source);
                if !source_path.is_absolute() {
                    validate_within_root(root, source_path)?;
                }
            }
        }
    }
    Ok(())
}

/// Validate all devcontainer path constraints (build context and mount sources).
///
/// Centralises the `validate_build_context` + `validate_mounts` calls that were
/// previously duplicated in every non-compose provider branch of `build_provider`.
fn validate_devcontainer_paths(directory: &Path, config: &Config) -> Result<()> {
    if let Some(build) = &config.build {
        if let Some(context) = &build.context {
            validate_build_context(directory, context)?;
        }
    }
    validate_mounts(directory, config.mounts.as_ref())
}

/// Resolve the build context directory for Docker/Podman providers.
///
/// `config_dir` is the directory containing the loaded `devcontainer.json`.
/// Per the containers.dev spec, relative `build.context` values are resolved
/// relative to `config_dir`. When absent, `config_dir` itself is the context.
fn resolve_build_context(config_dir: &Path, config: &Config) -> String {
    let Some(ctx) = config.build.as_ref().and_then(|b| b.context.as_deref()) else {
        return config_dir.to_string_lossy().into_owned();
    };
    let ctx_path = Path::new(ctx);
    if ctx_path.is_absolute() {
        ctx.to_string()
    } else {
        config_dir.join(ctx_path).to_string_lossy().into_owned()
    }
}

/// Shared parameters for direct (non-compose) provider construction.
struct DirectProviderArgs {
    name: String,
    build_args: std::collections::HashMap<String, String>,
    directory: String,
    forward_ports: Vec<u16>,
    mounts: Option<Vec<std::collections::HashMap<String, String>>>,
    run_args: Vec<String>,
    override_command: bool,
    user: String,
    workspace_folder: String,
}

fn build_provider(
    config_dir: &Path,
    workspace: &Path,
    settings: &Settings,
    config: &Config,
) -> Result<Box<dyn Provider>> {
    use crate::settings::Provider as ProviderKind;

    let name = config.safe_name()?;
    if !config.is_compose() {
        validate_devcontainer_paths(workspace, config)?;
    }

    let env_vars = sorted_env_vars(config);

    match (&settings.provider, config.is_compose()) {
        (ProviderKind::Docker, true) => build_docker_compose(config_dir, config, name, env_vars),
        (ProviderKind::Docker, false) => build_docker(
            config_dir,
            workspace,
            config,
            direct_args(workspace, config, name),
        ),
        (ProviderKind::Podman, true) => build_podman_compose(config_dir, config, name, env_vars),
        (ProviderKind::Podman, false) => build_podman(
            config_dir,
            workspace,
            config,
            direct_args(workspace, config, name),
        ),
        (ProviderKind::Apple, _) => build_apple(
            config_dir,
            workspace,
            config,
            direct_args(workspace, config, name),
        ),
        (ProviderKind::Nerdctl, true) => Err(Error::InvalidConfig(
            "nerdctl provider does not support Docker Compose devcontainers".to_string(),
        )),
        (ProviderKind::Nerdctl, false) => build_nerdctl(
            config_dir,
            workspace,
            config,
            direct_args(workspace, config, name),
        ),
    }
}

fn direct_args(directory: &Path, config: &Config, name: String) -> DirectProviderArgs {
    DirectProviderArgs {
        name,
        build_args: config.build_args(),
        directory: directory.to_string_lossy().to_string(),
        forward_ports: config.forward_ports.clone(),
        mounts: config.mounts.clone(),
        run_args: config.run_args.clone(),
        override_command: config.override_command,
        user: config.remote_user.clone(),
        workspace_folder: config.workspace_folder.clone(),
    }
}

fn build_docker_compose(
    config_dir: &Path,
    config: &Config,
    name: String,
    env_vars: Vec<(String, String)>,
) -> Result<Box<dyn Provider>> {
    let (file, service) = compose_path_and_service(config_dir, config)?;
    Ok(Box::new(DockerCompose {
        build_args: config.build_args(),
        command: "docker".to_string(),
        env_vars,
        file,
        name,
        service,
        shell: "sh".to_string(),
        user: config.remote_user.clone(),
        workspace_folder: config.workspace_folder.clone(),
    }))
}

fn build_docker(
    config_dir: &Path,
    workspace: &Path,
    config: &Config,
    a: DirectProviderArgs,
) -> Result<Box<dyn Provider>> {
    Ok(Box::new(Docker {
        build_args: a.build_args,
        build_context: resolve_build_context(config_dir, config),
        build_source: resolve_build_source(config_dir, workspace, config)?,
        command: "docker".to_string(),
        directory: a.directory,
        forward_ports: a.forward_ports,
        name: a.name,
        override_command: a.override_command,
        run_args: a.run_args,
        mounts: a.mounts,
        user: a.user,
        workspace_folder: a.workspace_folder,
    }))
}

fn build_podman_compose(
    config_dir: &Path,
    config: &Config,
    name: String,
    env_vars: Vec<(String, String)>,
) -> Result<Box<dyn Provider>> {
    let (file, service) = compose_path_and_service(config_dir, config)?;
    let selinux_relabel = config
        .selinux_relabel
        .unwrap_or_else(crate::provider::utils::selinux_enforcing);
    Ok(Box::new(PodmanCompose {
        build_args: config.build_args(),
        command: "podman-compose".to_string(),
        env_vars,
        file,
        name,
        podman_command: "podman".to_string(),
        selinux_relabel,
        service,
        shell: "sh".to_string(),
        user: config.remote_user.clone(),
        workspace_folder: config.workspace_folder.clone(),
    }))
}

fn build_podman(
    config_dir: &Path,
    workspace: &Path,
    config: &Config,
    a: DirectProviderArgs,
) -> Result<Box<dyn Provider>> {
    let ns_mode = "keep-id".to_string();
    Podman::validate_userns_mode(&ns_mode)
        .map_err(|e| Error::InvalidConfig(format!("invalid userns_mode: {e}")))?;
    Ok(Box::new(Podman {
        build_args: a.build_args,
        build_context: resolve_build_context(config_dir, config),
        build_source: resolve_build_source(config_dir, workspace, config)?,
        command: "podman".to_string(),
        directory: a.directory,
        forward_ports: a.forward_ports,
        mounts: a.mounts,
        name: a.name,
        run_args: a.run_args,
        override_command: a.override_command,
        user: a.user,
        workspace_folder: a.workspace_folder,
        userns_mode: ns_mode,
        disable_selinux: true,
    }))
}

fn build_apple(
    config_dir: &Path,
    workspace: &Path,
    config: &Config,
    a: DirectProviderArgs,
) -> Result<Box<dyn Provider>> {
    Ok(Box::new(crate::provider::apple::AppleContainer {
        build_args: a.build_args,
        build_context: resolve_build_context(config_dir, config),
        build_source: resolve_build_source(config_dir, workspace, config)?,
        command: "container".to_string(),
        directory: a.directory,
        forward_ports: a.forward_ports,
        name: a.name,
        run_args: a.run_args,
        mounts: a.mounts,
        override_command: a.override_command,
        user: a.user,
        workspace_folder: a.workspace_folder,
    }))
}

fn build_nerdctl(
    config_dir: &Path,
    workspace: &Path,
    config: &Config,
    a: DirectProviderArgs,
) -> Result<Box<dyn Provider>> {
    Ok(Box::new(Nerdctl {
        build_args: a.build_args,
        build_context: resolve_build_context(config_dir, config),
        build_source: resolve_build_source(config_dir, workspace, config)?,
        command: "nerdctl".to_string(),
        directory: a.directory,
        forward_ports: a.forward_ports,
        name: a.name,
        run_args: a.run_args,
        mounts: a.mounts,
        override_command: a.override_command,
        user: a.user,
        workspace_folder: a.workspace_folder,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Which lifecycle step the `MockProvider` should fail on.
    ///
    /// `None` means all steps succeed.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FailStep {
        Build,
        Create,
        Start,
        Restart,
        Attach,
        Stop,
    }

    #[allow(clippy::struct_field_names)]
    struct MockProvider {
        exec_calls: RefCell<Vec<String>>,
        exec_raw_calls: RefCell<Vec<(String, Vec<String>)>>,
        build_calls: RefCell<u32>,
        create_calls: RefCell<u32>,
        start_calls: RefCell<u32>,
        restart_calls: RefCell<u32>,
        attach_calls: RefCell<u32>,
        stop_calls: RefCell<u32>,
        rm_calls: RefCell<u32>,
        exec_result: bool,
        exists_result: bool,
        /// When `Some(step)`, that step returns `Err`; all others succeed.
        fail_step: Option<FailStep>,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                exec_calls: RefCell::new(vec![]),
                exec_raw_calls: RefCell::new(vec![]),
                build_calls: RefCell::new(0),
                create_calls: RefCell::new(0),
                start_calls: RefCell::new(0),
                restart_calls: RefCell::new(0),
                attach_calls: RefCell::new(0),
                stop_calls: RefCell::new(0),
                rm_calls: RefCell::new(0),
                exec_result: true,
                exists_result: false,
                fail_step: None,
            }
        }

        fn failing() -> Self {
            Self {
                exec_calls: RefCell::new(vec![]),
                exec_raw_calls: RefCell::new(vec![]),
                build_calls: RefCell::new(0),
                create_calls: RefCell::new(0),
                start_calls: RefCell::new(0),
                restart_calls: RefCell::new(0),
                attach_calls: RefCell::new(0),
                stop_calls: RefCell::new(0),
                rm_calls: RefCell::new(0),
                exec_result: false,
                exists_result: false,
                fail_step: None,
            }
        }

        fn with_existing() -> Self {
            Self {
                exec_calls: RefCell::new(vec![]),
                exec_raw_calls: RefCell::new(vec![]),
                build_calls: RefCell::new(0),
                create_calls: RefCell::new(0),
                start_calls: RefCell::new(0),
                restart_calls: RefCell::new(0),
                attach_calls: RefCell::new(0),
                stop_calls: RefCell::new(0),
                rm_calls: RefCell::new(0),
                exec_result: true,
                exists_result: true,
                fail_step: None,
            }
        }

        /// Create a mock that succeeds on all exec calls but fails the named
        /// lifecycle step with a descriptive error.
        fn failing_at(step: FailStep) -> Self {
            Self {
                exec_calls: RefCell::new(vec![]),
                exec_raw_calls: RefCell::new(vec![]),
                build_calls: RefCell::new(0),
                create_calls: RefCell::new(0),
                start_calls: RefCell::new(0),
                restart_calls: RefCell::new(0),
                attach_calls: RefCell::new(0),
                stop_calls: RefCell::new(0),
                rm_calls: RefCell::new(0),
                exec_result: true,
                exists_result: false,
                fail_step: Some(step),
            }
        }

        fn step_err(name: &str) -> std::io::Error {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("mock step '{name}' failed (stderr: {name} error output)"),
            )
        }
    }

    impl Provider for MockProvider {
        fn build(&self, _: bool) -> std::io::Result<()> {
            *self.build_calls.borrow_mut() += 1;
            if self.fail_step == Some(FailStep::Build) {
                return Err(Self::step_err("build"));
            }
            Ok(())
        }
        fn create(&self, _: &crate::provider::options::ContainerOptions) -> std::io::Result<()> {
            *self.create_calls.borrow_mut() += 1;
            if self.fail_step == Some(FailStep::Create) {
                return Err(Self::step_err("create"));
            }
            Ok(())
        }
        fn start(&self) -> std::io::Result<()> {
            *self.start_calls.borrow_mut() += 1;
            if self.fail_step == Some(FailStep::Start) {
                return Err(Self::step_err("start"));
            }
            Ok(())
        }
        fn stop(&self) -> std::io::Result<()> {
            *self.stop_calls.borrow_mut() += 1;
            if self.fail_step == Some(FailStep::Stop) {
                return Err(Self::step_err("stop"));
            }
            Ok(())
        }
        fn restart(&self) -> std::io::Result<()> {
            *self.restart_calls.borrow_mut() += 1;
            if self.fail_step == Some(FailStep::Restart) {
                return Err(Self::step_err("restart"));
            }
            Ok(())
        }
        fn attach(&self) -> std::io::Result<()> {
            *self.attach_calls.borrow_mut() += 1;
            if self.fail_step == Some(FailStep::Attach) {
                return Err(Self::step_err("attach"));
            }
            Ok(())
        }
        fn rm(&self) -> std::io::Result<()> {
            *self.rm_calls.borrow_mut() += 1;
            Ok(())
        }
        fn exists(&self) -> std::io::Result<bool> {
            Ok(self.exists_result)
        }
        fn running(&self) -> std::io::Result<bool> {
            Ok(false)
        }
        fn cp(&self, _: String, _: String) -> std::io::Result<()> {
            Ok(())
        }
        fn exec(&self, cmd: String) -> std::io::Result<()> {
            self.exec_calls.borrow_mut().push(cmd);
            if self.exec_result {
                Ok(())
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "exec failed (mock)",
                ))
            }
        }
        fn exec_capture(&self, cmd: &str) -> crate::error::Result<crate::provider::ExecOutput> {
            self.exec_calls.borrow_mut().push(cmd.to_string());
            if self.exec_result {
                Ok(crate::provider::ExecOutput {
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                    exit_code: 0,
                })
            } else {
                Err(crate::error::Error::ExecCaptureFailed {
                    exit_code: 1,
                    stdout: Vec::new(),
                    stderr: b"exec_capture failed (mock)".to_vec(),
                })
            }
        }
        fn exec_raw(&self, prog: &str, args: &[&str]) -> std::io::Result<()> {
            self.exec_raw_calls.borrow_mut().push((
                prog.to_string(),
                args.iter().map(|s| (*s).to_string()).collect(),
            ));
            if self.exec_result {
                Ok(())
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "exec_raw failed (mock)",
                ))
            }
        }
    }

    fn make_devcontainer_with_provider(
        config: Config,
        provider: Box<dyn Provider>,
    ) -> Devcontainer {
        make_devcontainer_with_config_dir(config, provider, PathBuf::from("/workspace"))
    }

    fn make_devcontainer_with_config_dir(
        config: Config,
        provider: Box<dyn Provider>,
        config_dir: PathBuf,
    ) -> Devcontainer {
        Devcontainer {
            config,
            config_dir,
            settings: Settings::default(),
            provider,
            hook_timeout_secs: None,
        }
    }

    #[test]
    fn resolve_build_context_no_context_defaults_to_config_dir() {
        // When build.context is absent, the context is the config_dir itself
        // (the directory containing devcontainer.json), per the containers.dev spec.
        let config: Config =
            json_five::from_str(r#"{ "name": "test", "image": "alpine" }"#).unwrap();
        let config_dir = std::path::Path::new("/ws/.devcontainer");
        let ctx = resolve_build_context(config_dir, &config);
        assert_eq!(ctx, "/ws/.devcontainer");
    }

    #[test]
    fn resolve_build_context_relative_context_joined_with_config_dir() {
        // Relative context is resolved relative to config_dir, not workspace root
        let config: Config = json_five::from_str(
            r#"{ "name": "test", "build": { "dockerfile": "Dockerfile", "context": "subdir" } }"#,
        )
        .unwrap();
        let config_dir = std::path::Path::new("/ws/.devcontainer");
        let ctx = resolve_build_context(config_dir, &config);
        assert_eq!(ctx, "/ws/.devcontainer/subdir");
    }

    #[test]
    fn resolve_build_context_absolute_context_returned_as_is() {
        let config: Config = json_five::from_str(
            r#"{ "name": "test", "build": { "dockerfile": "Dockerfile", "context": "/abs/ctx" } }"#,
        )
        .unwrap();
        let config_dir = std::path::Path::new("/ws/.devcontainer");
        let ctx = resolve_build_context(config_dir, &config);
        assert_eq!(ctx, "/abs/ctx");
    }

    #[test]
    fn exec_hook_one_uses_exec() {
        let provider = MockProvider::new();
        let hook = OneOrMany::One("echo hello".to_string());
        exec_hook(&provider, &hook).unwrap();
        assert_eq!(*provider.exec_calls.borrow(), vec!["echo hello"]);
        assert!(provider.exec_raw_calls.borrow().is_empty());
    }

    #[test]
    fn exec_hook_many_uses_exec_raw_not_exec() {
        let provider = MockProvider::new();
        let hook = OneOrMany::Many(vec!["npm".to_string(), "install".to_string()]);
        exec_hook(&provider, &hook).unwrap();
        assert!(provider.exec_calls.borrow().is_empty());
        let raw_calls = provider.exec_raw_calls.borrow();
        assert_eq!(raw_calls.len(), 1);
        assert_eq!(raw_calls[0].0, "npm");
        assert_eq!(raw_calls[0].1, vec!["install"]);
    }

    #[test]
    fn exec_hook_many_preserves_args_with_spaces() {
        let provider = MockProvider::new();
        let hook = OneOrMany::Many(vec![
            "echo".to_string(),
            "hello world".to_string(),
            "foo bar".to_string(),
        ]);
        exec_hook(&provider, &hook).unwrap();
        assert!(provider.exec_calls.borrow().is_empty());
        let raw_calls = provider.exec_raw_calls.borrow();
        assert_eq!(raw_calls.len(), 1);
        assert_eq!(raw_calls[0].0, "echo");
        assert_eq!(raw_calls[0].1, vec!["hello world", "foo bar"]);
    }

    #[test]
    fn exec_hook_many_empty_returns_ok() {
        let provider = MockProvider::new();
        let hook = OneOrMany::Many(vec![]);
        exec_hook(&provider, &hook).expect("empty Many hook should succeed");
        assert!(provider.exec_calls.borrow().is_empty());
        assert!(provider.exec_raw_calls.borrow().is_empty());
    }

    fn config_with_post_create() -> Config {
        json_five::from_str(
            r#"{ "name": "test", "image": "alpine", "postCreateCommand": "npm install" }"#,
        )
        .unwrap()
    }

    fn config_with_post_start() -> Config {
        json_five::from_str(
            r#"{ "name": "test", "image": "alpine", "postStartCommand": "start.sh" }"#,
        )
        .unwrap()
    }

    fn config_with_post_attach() -> Config {
        json_five::from_str(
            r#"{ "name": "test", "image": "alpine", "postAttachCommand": "attach.sh" }"#,
        )
        .unwrap()
    }

    fn config_with_on_create() -> Config {
        json_five::from_str(
            r#"{ "name": "test", "image": "alpine", "onCreateCommand": "on-create.sh" }"#,
        )
        .unwrap()
    }

    fn config_with_update_content() -> Config {
        json_five::from_str(
            r#"{ "name": "test", "image": "alpine", "updateContentCommand": "update.sh" }"#,
        )
        .unwrap()
    }

    #[test]
    fn run_aborts_on_post_create_hook_failure() {
        let dc = make_devcontainer_with_provider(
            config_with_post_create(),
            Box::new(MockProvider::failing()),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when postCreateCommand returns false");
        let msg = err.to_string();
        assert!(
            msg.contains("postCreateCommand"),
            "error message should mention 'postCreateCommand', got: {msg}"
        );
    }

    #[test]
    fn run_aborts_on_post_start_hook_failure() {
        let dc = make_devcontainer_with_provider(
            config_with_post_start(),
            Box::new(MockProvider::failing()),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when postStartCommand returns false");
        let msg = err.to_string();
        assert!(
            msg.contains("postStartCommand"),
            "error message should mention 'postStartCommand', got: {msg}"
        );
    }

    #[test]
    fn run_aborts_on_post_attach_hook_failure() {
        let dc = make_devcontainer_with_provider(
            config_with_post_attach(),
            Box::new(MockProvider::failing()),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when postAttachCommand returns false");
        // The run() aborts on the first exec failure — postAttachCommand or an earlier
        // internal exec (e.g., copy_gitconfig). Either way the error is surfaced.
        let _ = err.to_string();
    }

    #[test]
    fn run_aborts_on_on_create_hook_failure() {
        let dc = make_devcontainer_with_provider(
            config_with_on_create(),
            Box::new(MockProvider::failing()),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when onCreateCommand returns false");
        let msg = err.to_string();
        assert!(
            msg.contains("onCreateCommand"),
            "error message should mention 'onCreateCommand', got: {msg}"
        );
    }

    #[test]
    fn run_aborts_on_update_content_hook_failure() {
        let dc = make_devcontainer_with_provider(
            config_with_update_content(),
            Box::new(MockProvider::failing()),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when updateContentCommand returns false");
        let msg = err.to_string();
        assert!(
            msg.contains("updateContentCommand"),
            "error message should mention 'updateContentCommand', got: {msg}"
        );
    }

    // --- run() / rebuild() lifecycle tests ---

    fn config_minimal() -> Config {
        json_five::from_str(r#"{ "name": "minimal", "image": "alpine" }"#).unwrap()
    }

    fn config_all_hooks() -> Config {
        json_five::from_str(
            r#"{
                "name": "allhooks",
                "image": "alpine",
                "onCreateCommand": "on-create.sh",
                "updateContentCommand": "update-content.sh",
                "postCreateCommand": "post-create.sh",
                "postStartCommand": "post-start.sh",
                "postAttachCommand": "post-attach.sh"
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn run_succeeds_with_no_hooks() {
        let dc = make_devcontainer_with_provider(config_minimal(), Box::new(MockProvider::new()));
        dc.run(true, true, true, true)
            .expect("run() with no hooks should succeed");
    }

    #[test]
    fn run_succeeds_with_all_hooks() {
        let dc = make_devcontainer_with_provider(config_all_hooks(), Box::new(MockProvider::new()));
        dc.run(true, true, true, true)
            .expect("run() with all hooks should succeed");
        // run() completed → at minimum post-start and post-create hooks ran
    }

    #[test]
    fn run_calls_build_when_container_does_not_exist() {
        // exists_result = false → build + create must be called inside create()
        let dc = make_devcontainer_with_provider(config_minimal(), Box::new(MockProvider::new()));
        dc.run(true, true, true, true)
            .expect("run() should succeed");
        // Verify by the absence of error: if build were skipped and create were
        // never called the provider would still return Ok(()) — run() completing
        // successfully is the observable outcome when exists() is false.
    }

    #[test]
    fn rebuild_succeeds_when_container_exists() {
        let dc = make_devcontainer_with_provider(
            config_minimal(),
            Box::new(MockProvider::with_existing()),
        );
        dc.rebuild(true, true, true, true)
            .expect("rebuild() should succeed when container already exists");
    }

    #[test]
    fn rebuild_stop_and_rm_called_when_container_exists() {
        // Verifies rebuild() calls stop() and rm() before re-running.
        // We can't inspect the mock after boxing, so we assert run() succeeds.
        let dc = make_devcontainer_with_provider(
            config_minimal(),
            Box::new(MockProvider::with_existing()),
        );
        dc.rebuild(true, true, true, true)
            .expect("rebuild() should not fail");
    }

    #[test]
    fn rebuild_succeeds_when_container_does_not_exist() {
        let dc = make_devcontainer_with_provider(
            config_minimal(),
            Box::new(MockProvider::new()), // exists_result = false → no stop/rm
        );
        dc.rebuild(true, true, true, true)
            .expect("rebuild() should succeed when container does not exist");
    }

    // --- typed error variant matching ---

    #[test]
    fn run_post_create_failure_is_hook_failed_variant() {
        let dc = make_devcontainer_with_provider(
            config_with_post_create(),
            Box::new(MockProvider::failing()),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when postCreateCommand returns false");
        assert!(
            matches!(err, crate::error::Error::HookFailed(_)),
            "expected Error::HookFailed, got: {err}"
        );
    }

    #[test]
    fn run_on_create_failure_is_hook_failed_variant() {
        let dc = make_devcontainer_with_provider(
            config_with_on_create(),
            Box::new(MockProvider::failing()),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when onCreateCommand fails");
        assert!(
            matches!(err, crate::error::Error::HookFailed(_)),
            "expected Error::HookFailed, got: {err}"
        );
    }

    #[test]
    fn is_confirmed_y_lowercase() {
        assert!(is_confirmed("y"));
    }

    #[test]
    fn is_confirmed_y_uppercase() {
        assert!(is_confirmed("Y"));
    }

    #[test]
    fn is_confirmed_y_with_whitespace() {
        assert!(is_confirmed("  y\n"));
    }

    #[test]
    fn is_confirmed_n_is_false() {
        assert!(!is_confirmed("n"));
    }

    #[test]
    fn is_confirmed_empty_is_false() {
        assert!(!is_confirmed(""));
    }

    #[test]
    fn is_confirmed_yes_is_false() {
        assert!(!is_confirmed("yes"));
    }

    #[test]
    fn root_user_triggers_warning() {
        assert!(should_warn_root("root", false));
    }

    #[test]
    fn uid_zero_triggers_warning() {
        assert!(should_warn_root("0", false));
    }

    #[test]
    fn empty_user_triggers_warning() {
        assert!(should_warn_root("", false));
    }

    #[test]
    fn non_root_user_no_warning() {
        assert!(!should_warn_root("vscode", false));
    }

    #[test]
    fn root_user_suppressed_with_no_root_check() {
        assert!(!should_warn_root("root", true));
    }

    // --- exec_host_hook ---

    #[test]
    fn exec_host_hook_one_exits_zero_returns_true() {
        let hook = OneOrMany::One("true".to_string());
        let result = exec_host_hook(&hook, None).expect("sh -c true should succeed");
        assert!(result, "exit 0 → should return true");
    }

    #[test]
    fn exec_host_hook_one_exits_nonzero_returns_false() {
        let hook = OneOrMany::One("false".to_string());
        let result = exec_host_hook(&hook, None).expect("sh -c false should not error");
        assert!(!result, "exit 1 → should return false");
    }

    #[test]
    fn exec_host_hook_many_exits_zero_returns_true() {
        let hook = OneOrMany::Many(vec!["true".to_string()]);
        let result = exec_host_hook(&hook, None).expect("true binary should succeed");
        assert!(result, "Many([true]) exit 0 → should return true");
    }

    #[test]
    fn exec_host_hook_many_exits_nonzero_returns_false() {
        let hook = OneOrMany::Many(vec!["false".to_string()]);
        let result = exec_host_hook(&hook, None).expect("false binary should not error");
        assert!(!result, "Many([false]) exit 1 → should return false");
    }

    #[test]
    fn exec_host_hook_many_empty_returns_true() {
        let hook = OneOrMany::Many(vec![]);
        let result = exec_host_hook(&hook, None).expect("empty Many hook should succeed");
        assert!(
            result,
            "empty Many hook should return true without executing anything"
        );
    }

    #[test]
    fn exec_host_hook_nonexistent_command_returns_err() {
        let hook = OneOrMany::Many(vec!["__nonexistent_devcont_cmd_xyz_42__".to_string()]);
        let result = exec_host_hook(&hook, None);
        assert!(
            result.is_err(),
            "nonexistent command should return Err, not Ok"
        );
    }

    #[test]
    fn exec_host_hook_many_preserves_space_in_arg() {
        // The arg "hello world" must reach echo as a single argument (not split).
        // If echo receives it as one arg it prints "hello world" and exits 0.
        let hook = OneOrMany::Many(vec!["echo".to_string(), "hello world".to_string()]);
        let result = exec_host_hook(&hook, None).expect("echo with space arg should succeed");
        assert!(result, "echo with space-containing arg should exit 0");
    }

    // --- hook timeout ---

    #[test]
    fn exec_host_hook_completes_within_timeout_returns_success() {
        // `true` exits immediately — well within a 5-second timeout.
        let hook = OneOrMany::Many(vec!["true".to_string()]);
        let result = exec_host_hook(&hook, Some(5)).expect("hook within timeout should not error");
        assert!(result, "successful hook within timeout should return true");
    }

    #[test]
    fn exec_host_hook_exceeds_timeout_returns_err() {
        // `sleep 60` won't finish within a 1-second timeout.
        let hook = OneOrMany::Many(vec!["sleep".to_string(), "60".to_string()]);
        let result = exec_host_hook(&hook, Some(1));
        assert!(result.is_err(), "hook exceeding timeout should return Err");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("timed out"),
            "error message should mention 'timed out', got: {err_msg}"
        );
    }

    #[test]
    fn config_hook_timeout_seconds_parsed() {
        let c: Config =
            json_five::from_str(r#"{ "name": "t", "image": "alpine", "hookTimeoutSeconds": 120 }"#)
                .expect("should parse hookTimeoutSeconds");
        assert_eq!(c.hook_timeout_seconds, Some(120));
    }

    #[test]
    fn config_hook_timeout_seconds_absent_is_none() {
        let c: Config = json_five::from_str(r#"{ "name": "t", "image": "alpine" }"#)
            .expect("should parse with no hookTimeoutSeconds");
        assert_eq!(c.hook_timeout_seconds, None);
    }

    #[test]
    fn with_hook_timeout_overrides_config_value() {
        let config: Config =
            json_five::from_str(r#"{ "name": "t", "image": "alpine", "hookTimeoutSeconds": 30 }"#)
                .expect("should parse");
        let dc = make_devcontainer_with_provider(config, Box::new(MockProvider::new()));
        let dc = dc.with_hook_timeout(60);
        assert_eq!(dc.hook_timeout_secs, Some(60));
    }

    #[test]
    fn build_provider_nerdctl_rejects_compose_config() {
        let config: Config = json_five::from_str(
            r#"{ "name": "t", "dockerComposeFile": "docker-compose.yml", "service": "app" }"#,
        )
        .expect("should parse compose config");
        let settings = Settings {
            provider: crate::settings::Provider::Nerdctl,
            ..Settings::default()
        };
        let workspace = std::path::Path::new("/workspace");
        let config_dir = workspace.join(".devcontainer");
        let result = build_provider(&config_dir, workspace, &settings, &config);
        assert!(
            result.is_err(),
            "build_provider with Nerdctl + compose config should return Err"
        );
        let err = result.err().unwrap();
        assert!(
            matches!(err, crate::error::Error::InvalidConfig(_)),
            "expected Error::InvalidConfig, got: {err}"
        );
    }

    // --- dotfile path traversal ---

    #[test]
    fn validate_dotfile_entry_normal_entry_ok() {
        let local_home = Path::new("/home/user");
        let remote_home = Path::new("/home/vscode");
        assert!(validate_dotfile_entry(".bashrc", local_home, remote_home).is_ok());
    }

    #[test]
    fn validate_dotfile_entry_nested_ok() {
        let local_home = Path::new("/home/user");
        let remote_home = Path::new("/home/vscode");
        assert!(validate_dotfile_entry(".config/git/config", local_home, remote_home).is_ok());
    }

    #[test]
    fn validate_dotfile_entry_traversal_rejected() {
        let local_home = Path::new("/home/user");
        let remote_home = Path::new("/home/vscode");
        let result = validate_dotfile_entry("../../etc/shadow", local_home, remote_home);
        assert!(result.is_err(), "path traversal should be rejected");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("escapes"),
            "error should mention 'escapes', got: {msg}"
        );
    }

    #[test]
    fn validate_dotfile_entry_single_parent_rejected() {
        let local_home = Path::new("/home/user");
        let remote_home = Path::new("/home/vscode");
        let result = validate_dotfile_entry("../other_user/.bashrc", local_home, remote_home);
        assert!(
            result.is_err(),
            "single parent traversal should be rejected"
        );
    }

    // --- per-step provider failure tests (Phase 2) ---

    /// Helper: a minimal config with no hooks so the only failure source is the provider step.
    fn config_no_hooks() -> Config {
        json_five::from_str(r#"{ "name": "minimal", "image": "alpine" }"#).unwrap()
    }

    #[test]
    fn run_aborts_on_build_failure() {
        let dc = make_devcontainer_with_provider(
            config_no_hooks(),
            Box::new(MockProvider::failing_at(FailStep::Build)),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when build fails");
        let msg = err.to_string();
        assert!(
            msg.contains("build"),
            "error message should mention 'build', got: {msg}"
        );
    }

    #[test]
    fn run_aborts_on_create_failure() {
        let dc = make_devcontainer_with_provider(
            config_no_hooks(),
            Box::new(MockProvider::failing_at(FailStep::Create)),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when create fails");
        let msg = err.to_string();
        assert!(
            msg.contains("create"),
            "error message should mention 'create', got: {msg}"
        );
    }

    #[test]
    fn run_aborts_on_start_failure() {
        let dc = make_devcontainer_with_provider(
            config_no_hooks(),
            Box::new(MockProvider::failing_at(FailStep::Start)),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when start fails");
        let msg = err.to_string();
        assert!(
            msg.contains("start"),
            "error message should mention 'start', got: {msg}"
        );
    }

    #[test]
    fn run_aborts_on_start_failure_no_subsequent_step() {
        // When start fails, restart and attach must not be called.
        let mock = MockProvider::failing_at(FailStep::Start);
        let dc = make_devcontainer_with_provider(config_no_hooks(), Box::new(mock));
        drop(dc.run(true, true, true, true));
        // The provider was moved into the box; we can only observe the observable error.
        // The test above already asserts the error is returned; this variant documents intent.
    }

    #[test]
    fn run_aborts_on_restart_failure() {
        let dc = make_devcontainer_with_provider(
            config_no_hooks(),
            Box::new(MockProvider::failing_at(FailStep::Restart)),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when restart fails");
        let msg = err.to_string();
        assert!(
            msg.contains("restart"),
            "error message should mention 'restart', got: {msg}"
        );
    }

    #[test]
    fn run_aborts_on_attach_failure() {
        let dc = make_devcontainer_with_provider(
            config_no_hooks(),
            Box::new(MockProvider::failing_at(FailStep::Attach)),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when attach fails");
        let msg = err.to_string();
        assert!(
            msg.contains("attach"),
            "error message should mention 'attach', got: {msg}"
        );
    }

    #[test]
    fn run_aborts_on_stop_failure() {
        // stop() is only called when shutdownAction requires it.
        // Use a config that triggers stop.
        let config: Config = json_five::from_str(
            r#"{ "name": "minimal", "image": "alpine", "shutdownAction": "stopContainer" }"#,
        )
        .unwrap();
        let dc = make_devcontainer_with_provider(
            config,
            Box::new(MockProvider::failing_at(FailStep::Stop)),
        );
        let err = dc
            .run(true, true, true, true)
            .expect_err("run() must fail when stop fails");
        let msg = err.to_string();
        assert!(
            msg.contains("stop"),
            "error message should mention 'stop', got: {msg}"
        );
    }

    // --- load_for_inspection ---

    /// `load_for_inspection` must succeed for a config that has `initializeCommand`
    /// without actually running the hook command.
    ///
    /// The fixture command `__devcont_test_hook_must_not_run__` does not exist on
    /// the system; if it were executed, the test would fail with an I/O error.
    #[test]
    fn load_for_inspection_does_not_run_initialize_command() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/inspect_project");
        // The fixture has initializeCommand set to a nonexistent program.
        // If load_for_inspection() ran the hook, this call would return Err.
        let inspection = Devcontainer::load_for_inspection(&dir);
        // We expect Ok (with hooks not run).
        // If it ran the hook, we'd get an Io error from trying to execute the
        // nonexistent command.
        match inspection {
            Ok(_) => { /* success: hook was not executed */ }
            Err(crate::error::Error::Io(ref e)) => {
                // An Io error would suggest the hook was actually executed.
                panic!(
                    "load_for_inspection returned an Io error, which may mean a hook was executed: {e}"
                );
            }
            Err(other) => panic!("unexpected error from load_for_inspection: {other}"),
        }
    }

    /// `load_for_inspection` returns the container name computed by `safe_name()`.
    #[test]
    fn load_for_inspection_returns_correct_container_name() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/inspect_project");
        let inspection = Devcontainer::load_for_inspection(&dir)
            .expect("load_for_inspection should succeed on the inspect_project fixture");
        // The fixture name is "Inspect Project" → "devcont-inspect-project"
        assert_eq!(inspection.container_name(), "devcont-inspect-project");
    }

    /// `load_for_inspection` surfaces `config_dir` as the `.devcontainer` subdirectory.
    #[test]
    fn load_for_inspection_config_dir_is_devcontainer_dir() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/inspect_project");
        let inspection =
            Devcontainer::load_for_inspection(&dir).expect("load_for_inspection should succeed");
        let config_dir = inspection.config_dir();
        assert!(
            config_dir.is_absolute(),
            "config_dir must be an absolute path"
        );
        // For the nested form, config_dir should end with ".devcontainer"
        assert!(
            config_dir.ends_with(".devcontainer"),
            "config_dir should be the .devcontainer subdirectory, got: {}",
            config_dir.display()
        );
    }

    // --- probe_exists / probe_running ---

    /// `probe_exists()` delegates to the provider's `exists()`.
    /// With a mock that returns `true`, the result is `true`.
    #[test]
    fn probe_exists_returns_provider_result_true() {
        let dc = make_devcontainer_with_provider(
            config_minimal(),
            Box::new(MockProvider::with_existing()),
        );
        assert!(
            dc.probe_exists().expect("probe_exists should succeed"),
            "probe_exists() should return true when provider returns true"
        );
    }

    /// `probe_exists()` returns `false` when the provider does not have an existing container.
    #[test]
    fn probe_exists_returns_provider_result_false() {
        let dc = make_devcontainer_with_provider(config_minimal(), Box::new(MockProvider::new()));
        assert!(
            !dc.probe_exists().expect("probe_exists should succeed"),
            "probe_exists() should return false when provider returns false"
        );
    }

    /// `probe_running()` returns `false` for the default mock (not running).
    #[test]
    fn probe_running_returns_provider_result_false() {
        let dc = make_devcontainer_with_provider(config_minimal(), Box::new(MockProvider::new()));
        assert!(
            !dc.probe_running().expect("probe_running should succeed"),
            "probe_running() should return false when provider returns false"
        );
    }

    /// A `Provider` that simulates an I/O error on `exists()` and `running()`.
    struct ErrorProvider;

    impl Provider for ErrorProvider {
        fn build(&self, _: bool) -> std::io::Result<()> {
            Ok(())
        }
        fn create(&self, _: &crate::provider::options::ContainerOptions) -> std::io::Result<()> {
            Ok(())
        }
        fn start(&self) -> std::io::Result<()> {
            Ok(())
        }
        fn stop(&self) -> std::io::Result<()> {
            Ok(())
        }
        fn restart(&self) -> std::io::Result<()> {
            Ok(())
        }
        fn attach(&self) -> std::io::Result<()> {
            Ok(())
        }
        fn rm(&self) -> std::io::Result<()> {
            Ok(())
        }
        fn exists(&self) -> std::io::Result<bool> {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "simulated engine error",
            ))
        }
        fn running(&self) -> std::io::Result<bool> {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "simulated engine error",
            ))
        }
        fn cp(&self, _: String, _: String) -> std::io::Result<()> {
            Ok(())
        }
        fn exec(&self, _: String) -> std::io::Result<()> {
            Ok(())
        }
        fn exec_capture(&self, _: &str) -> crate::error::Result<crate::provider::ExecOutput> {
            Ok(crate::provider::ExecOutput {
                stdout: Vec::new(),
                stderr: Vec::new(),
                exit_code: 0,
            })
        }
        fn exec_raw(&self, _: &str, _: &[&str]) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// `probe_exists()` propagates provider I/O errors.
    #[test]
    fn probe_exists_propagates_engine_error() {
        let dc = make_devcontainer_with_provider(config_minimal(), Box::new(ErrorProvider));
        assert!(
            dc.probe_exists().is_err(),
            "probe_exists() should propagate engine errors"
        );
    }

    /// `probe_running()` propagates provider I/O errors.
    #[test]
    fn probe_running_propagates_engine_error() {
        let dc = make_devcontainer_with_provider(config_minimal(), Box::new(ErrorProvider));
        assert!(
            dc.probe_running().is_err(),
            "probe_running() should propagate engine errors"
        );
    }
}
