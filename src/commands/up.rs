use devcont::devcontainers::Devcontainer; // container lifecycle entry point
/// Ensure the dev container is up and all lifecycle hooks have run, then return.
///
/// Unlike [`start`](super::start), this command does **not** attach to the container.
/// It is designed for non-interactive callers (CI scripts, orchestrators, agent sandboxes)
/// that need "ensure running, then `docker exec` later" semantics.
///
/// Equivalent to running `start` up through `postCreateCommand`, then returning
/// without calling `restart`, `attach`, `postAttachCommand`, or honouring `shutdownAction`.
///
/// If your hooks may run for a long time, pass `--hook-timeout <seconds>` to avoid
/// blocking indefinitely.
///
/// # Errors
/// Returns an error if the directory cannot be resolved, the devcontainer config cannot be
/// loaded, any lifecycle operation fails, or the container is not running after start.
#[allow(clippy::fn_params_excessive_bools)]
pub fn run(
    dir: Option<&str>,
    trust: bool,
    no_root_check: bool,
    no_audit_log: bool,
    hook_timeout: Option<u32>,
) -> std::io::Result<()> {
    let directory = super::get_project_directory(dir)?;
    let mut dc = Devcontainer::load(&directory)?;
    if let Some(secs) = hook_timeout {
        dc = dc.with_hook_timeout(secs);
    }
    dc.up(true, trust, no_root_check, no_audit_log)?;
    Ok(()) // up::run result
} // end up::run
