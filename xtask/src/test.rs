//! Testing operations.

use std::process::Command;

/// Run the test suite.
///
/// # Errors
///
/// Returns an error if tests fail.
pub fn run() -> anyhow::Result<()> {
    println!("Running tests...");
    let status = Command::new("cargo")
        .args(["test", "--workspace"])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch cargo test: {e}"))?;

    if status.success() {
        println!("Tests passed");
        Ok(())
    } else {
        anyhow::bail!("Tests failed (exit {status})")
    }
}

/// Run tests with coverage via cargo-llvm-cov.
///
/// # Errors
///
/// Returns an error if coverage generation fails.
pub fn run_coverage() -> anyhow::Result<()> {
    println!("Running tests with coverage...");
    let status = Command::new("cargo")
        .args(["llvm-cov", "--workspace", "--summary-only"])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch cargo llvm-cov: {e}"))?;

    if status.success() {
        println!("Coverage report generated");
        Ok(())
    } else {
        anyhow::bail!("Coverage failed (exit {status})")
    }
}
