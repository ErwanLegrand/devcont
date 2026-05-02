use devcont::devcontainers::Devcontainer;
use serde::Serialize;

/// Serializable JSON output for the `info` subcommand.
///
/// Fields appear in a fixed order matching the spec. When `--no-probe` is
/// passed, `exists` and `running` are `None` and omitted from the output.
#[derive(Serialize)]
pub(crate) struct InfoOutput {
    /// Deterministic container name (from `safe_name()`).
    container: String,
    /// Pre-built image name, or `null` when the container is built from a Dockerfile.
    image: Option<String>,
    /// Working directory inside the container (`workspaceFolder`).
    workspace: String,
    /// Directory containing the resolved `devcontainer.json`.
    config_dir: String,
    /// Container engine name (e.g. `"docker"`, `"podman"`).
    engine: String,
    /// Whether the container currently exists (omitted with `--no-probe`).
    #[serde(skip_serializing_if = "Option::is_none")]
    exists: Option<bool>,
    /// Whether the container is currently running (omitted with `--no-probe`).
    #[serde(skip_serializing_if = "Option::is_none")]
    running: Option<bool>,
}

/// Print a JSON document describing the dev container in `dir`.
///
/// When `no_probe` is `true`, engine probes are skipped and `exists`/`running`
/// are omitted from the output.
///
/// # Exit codes
/// - `0` — info printed successfully.
/// - `2` — `devcontainer.json` could not be loaded (diagnostic on stderr).
/// - Non-zero — engine probe failed (unless `--no-probe` was passed).
pub fn run(dir: Option<&str>, no_probe: bool) {
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

    let config = inspection.config();

    let (exists, running) = if no_probe {
        (None, None)
    } else {
        // Build a full Devcontainer to access the provider for probing.
        let dc = match Devcontainer::load(&directory) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("error: engine probe failed: {e}");
                std::process::exit(1);
            }
        };
        let e = match dc.probe_exists() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("error: engine probe (exists) failed: {e}");
                std::process::exit(1);
            }
        };
        let r = match dc.probe_running() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("error: engine probe (running) failed: {e}");
                std::process::exit(1);
            }
        };
        (Some(e), Some(r))
    };

    let output = InfoOutput {
        container: inspection.container_name(),
        image: config.image.clone(),
        workspace: config.workspace_folder.clone(),
        config_dir: inspection.config_dir().to_string_lossy().into_owned(),
        engine: inspection.engine().to_string(),
        exists,
        running,
    };

    match serde_json::to_string_pretty(&output) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("error: failed to serialize info output: {e}");
            std::process::exit(1);
        }
    }
} // end info::run
