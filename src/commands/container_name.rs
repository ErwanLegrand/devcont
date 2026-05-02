use devcont::devcontainers::Devcontainer;

/// Print the deterministic container name for the devcontainer in `dir`
/// (or the current directory when `None`).
///
/// Writes the container name followed by `\n` to stdout and exits 0.
///
/// # Exit codes
/// - `0` — name was determined and printed.
/// - `2` — `devcontainer.json` could not be loaded (diagnostic on stderr).
/// - `3` — config loaded but the container name could not be derived.
///
/// This function never returns — it always calls [`std::process::exit`] on
/// error paths to emit the documented exit codes. Normal completion falls
/// through to `Ok(())` which the caller converts to exit code 0.
pub fn run(dir: Option<&str>) {
    let directory = match super::get_project_directory(dir) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };

    let inspection = match Devcontainer::load_for_inspection(&directory) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };

    println!("{}", inspection.container_name());
} // end container_name::run
