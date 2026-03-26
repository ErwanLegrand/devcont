//! Shared utilities for xtask operations.

use std::path::PathBuf;

/// Get the workspace root directory by searching upward for a `Cargo.toml`
/// containing a `[workspace]` section.
///
/// # Errors
///
/// Returns an error if the current directory cannot be determined.
pub fn workspace_root() -> anyhow::Result<PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }

        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    Ok(std::env::current_dir()?)
}
