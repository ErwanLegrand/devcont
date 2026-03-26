// Copyright (c) 2025 Erwan Patrick Legrand

//! CI/CD pipeline operations
//!
//! Runs the full CI pipeline: format check, clippy, tests, doc build,
//! and optionally cargo-deny if `deny.toml` is present.

use std::process::Command;

/// Run a command, printing its stdout/stderr, and bail on non-zero exit.
fn run_step(step_name: &str, program: &str, args: &[&str]) -> anyhow::Result<()> {
    println!("CI: {step_name}");

    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch `{program}`: {e}"))?;

    if status.success() {
        println!("CI: {step_name} -- passed");
        Ok(())
    } else {
        anyhow::bail!("CI: {step_name} -- failed (exit {})", status)
    }
}

/// Execute the full CI pipeline, failing on the first non-zero exit.
pub fn run() -> anyhow::Result<()> {
    println!("Running CI pipeline...");

    run_step("cargo fmt --check", "cargo", &["fmt", "--check"])?;

    run_step(
        "cargo clippy",
        "cargo",
        &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    )?;

    run_step("cargo test", "cargo", &["test", "--workspace"])?;

    run_step("cargo doc --no-deps", "cargo", &["doc", "--no-deps"])?;

    // cargo deny check is conditional on deny.toml existing
    if std::path::Path::new("deny.toml").exists() {
        run_step("cargo deny check", "cargo", &["deny", "check"])?;
    } else {
        println!("CI: skipping cargo deny (no deny.toml found)");
    }

    println!("CI pipeline completed successfully");
    Ok(())
}
