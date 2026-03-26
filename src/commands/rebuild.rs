use devcont::devcontainers::Devcontainer; // container lifecycle entry point
/// Stop, remove, and restart the dev container in `dir` (or the current directory when `None`).
///
/// When `use_cache` is `true` the Docker layer cache is used; pass `false` to force a clean
/// image rebuild.
///
/// # Errors
/// Returns an error if the directory cannot be resolved, the devcontainer config cannot be
/// loaded, or any lifecycle operation fails.
#[allow(clippy::fn_params_excessive_bools)]
pub fn run(
    dir: Option<&str>,
    use_cache: bool,
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
    dc.rebuild(use_cache, trust, no_root_check, no_audit_log)?;
    Ok(()) // rebuild::run result
} // end rebuild::run
