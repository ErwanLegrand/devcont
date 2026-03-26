pub mod rebuild; // rebuild sub-command handler
pub mod start; // start sub-command handler

use std::path::{Path, PathBuf};

/// Resolve the project directory from an optional user-supplied path, falling back to cwd.
pub(crate) fn get_project_directory(dir: Option<&str>) -> std::io::Result<PathBuf> {
    if let Some(path) = dir {
        let expanded = shellexpand::env(path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Could not expand dir '{path}': {e}"),
            )
        })?;

        Path::new(expanded.as_ref()).canonicalize()
    } else {
        std::env::current_dir()
    }
}
