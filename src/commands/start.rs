use devcont::devcontainers::Devcontainer; // container lifecycle entry point
/// Start the dev container in `dir` (or the current directory when `None`).
///
/// # Errors
/// Returns an error if the directory cannot be resolved, the devcontainer config cannot be
/// loaded, or any lifecycle operation fails.
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
    dc.run(true, trust, no_root_check, no_audit_log)?;
    Ok(()) // start::run result
} // end start::run
