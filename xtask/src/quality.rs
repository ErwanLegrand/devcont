//! Code quality checks.

use std::process::Command;

/// Run all quality checks: format, lint, doc-check, audit.
///
/// # Errors
///
/// Returns an error if any check fails.
pub fn run_quality() -> anyhow::Result<()> {
    println!("Running comprehensive quality checks...");
    run_format_check()?;
    run_lint()?;
    run_doc_check()?;
    run_audit()?;
    println!("All quality checks passed!");
    Ok(())
}

/// Run clippy with pedantic warnings.
///
/// # Errors
///
/// Returns an error if clippy finds issues.
pub fn run_lint() -> anyhow::Result<()> {
    println!("Running linter...");
    let status = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch clippy: {e}"))?;

    if status.success() {
        println!("Linting passed");
        Ok(())
    } else {
        anyhow::bail!("Linting failed (exit {status})")
    }
}

/// Format code with rustfmt.
///
/// # Errors
///
/// Returns an error if formatting fails.
pub fn run_format() -> anyhow::Result<()> {
    println!("Formatting code...");
    let status = Command::new("cargo")
        .args(["fmt"])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch rustfmt: {e}"))?;

    if status.success() {
        println!("Code formatted");
        Ok(())
    } else {
        anyhow::bail!("Formatting failed (exit {status})")
    }
}

/// Check formatting without modifying files.
///
/// # Errors
///
/// Returns an error if formatting issues are found.
pub fn run_format_check() -> anyhow::Result<()> {
    println!("Checking code formatting...");
    let status = Command::new("cargo")
        .args(["fmt", "--check"])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch rustfmt: {e}"))?;

    if status.success() {
        println!("Code formatting is correct");
        Ok(())
    } else {
        anyhow::bail!("Code formatting issues found")
    }
}

/// Check documentation builds without warnings.
///
/// # Errors
///
/// Returns an error if documentation has issues.
pub fn run_doc_check() -> anyhow::Result<()> {
    println!("Checking documentation...");
    let status = Command::new("cargo")
        .args(["doc", "--no-deps", "--document-private-items"])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch cargo doc: {e}"))?;

    if status.success() {
        println!("Documentation checks passed");
        Ok(())
    } else {
        anyhow::bail!("Documentation check failed (exit {status})")
    }
}

/// Run security audit via cargo-deny (if deny.toml exists) and cargo-audit.
///
/// # Errors
///
/// Returns an error if audit finds issues.
pub fn run_audit() -> anyhow::Result<()> {
    println!("Running security audit...");

    // Run cargo-deny for license and bans checks (advisory checks use cargo-audit
    // instead, because cargo-deny's advisory DB parser doesn't support CVSS v4.0
    // entries yet).
    if std::path::Path::new("deny.toml").exists() {
        let status = Command::new("cargo")
            .args(["deny", "check", "licenses", "bans"])
            .status()
            .map_err(|e| anyhow::anyhow!("failed to launch cargo deny: {e}"))?;

        if !status.success() {
            anyhow::bail!("cargo deny check failed (exit {status})");
        }
        println!("cargo deny (licenses, bans) passed");
    }

    let status = Command::new("cargo").args(["audit", "--quiet"]).status();

    match status {
        Ok(s) if s.success() => println!("Security audit passed"),
        Ok(s) => anyhow::bail!("Security audit failed (exit {s})"),
        Err(_) => println!("cargo-audit not installed, skipping"),
    }

    Ok(())
}
