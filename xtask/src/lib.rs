// Copyright (c) 2025 Erwan Patrick Legrand

//! Shared utilities for xtask operations

use std::path::{Path, PathBuf};
use std::process::Output;

/// Result type alias for xtask operations
pub type Result<T> = anyhow::Result<T>;

/// Get the workspace root directory
///
/// # Errors
///
/// Returns an error if the current directory cannot be retrieved or if
/// a `Cargo.toml` file cannot be read.
pub fn workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        if current.join("Cargo.toml").exists() {
            // Check if this is a workspace root by looking for [workspace] section
            let cargo_toml = std::fs::read_to_string(current.join("Cargo.toml"))?;
            if cargo_toml.contains("[workspace]") {
                return Ok(current);
            }
        }

        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    // Fallback to current directory if no workspace found
    Ok(std::env::current_dir()?)
}

/// Run a command and return the output
///
/// # Errors
///
/// Returns an error if the command fails to execute.
pub fn run_command(cmd: &mut duct::Expression) -> Result<Output> {
    let output = cmd.run()?;
    Ok(output)
}

/// Check if a command succeeded
///
/// # Errors
///
/// Returns an error if the command exit code is not zero.
pub fn check_success(output: &Output) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "Command failed with exit code {}\nstdout: {}\nstderr: {}",
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        );
    }
}

/// Get all crate directories in the workspace
///
/// # Errors
///
/// Returns an error if the workspace root cannot be found or if
/// a `Cargo.toml` file cannot be read.
pub fn get_crate_dirs() -> Result<Vec<PathBuf>> {
    let root = workspace_root()?;
    let mut crates = Vec::new();

    // Check if this is a workspace
    let cargo_toml_path = root.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let cargo_toml = std::fs::read_to_string(&cargo_toml_path)?;
        if cargo_toml.contains("[workspace]") {
            // Parse workspace members
            if let Some(members_line) = cargo_toml.lines().find(|line| line.contains("members"))
                && let Some(members_value) = members_line.split('=').nth(1)
            {
                let members_str = members_value.trim().trim_matches(&['[', ']', '"'] as &[_]);
                for member in members_str.split(',') {
                    let member = member.trim().trim_matches('"');
                    if !member.is_empty() {
                        crates.push(root.join(member));
                    }
                }
            }
        }
    }

    // If no workspace members found, check current directory
    if crates.is_empty() && cargo_toml_path.exists() {
        crates.push(root);
    }

    Ok(crates)
}

/// Check if a file exists and is recent (within last hour)
#[must_use]
pub fn is_file_recent(path: &Path) -> bool {
    if let Ok(metadata) = std::fs::metadata(path)
        && let Ok(modified) = metadata.modified()
        && let Ok(duration) = modified.elapsed()
    {
        return duration.as_secs() < 3600; // 1 hour
    }
    false
}

/// Format duration for display
#[must_use]
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

/// Get the current git branch name
///
/// # Errors
///
/// Returns an error if the `git` command fails to execute.
pub fn get_git_branch() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok("unknown".to_string())
    }
}

/// Check if we're in a git repository
#[must_use]
pub fn is_git_repo() -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
