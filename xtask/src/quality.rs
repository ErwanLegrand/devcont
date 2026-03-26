// Copyright (c) 2025 Erwan Patrick Legrand

//! Code quality checks

/// Run comprehensive quality checks
///
/// # Errors
///
/// Returns an error if any of the quality checks fail.
pub fn run_quality() -> anyhow::Result<()> {
    println!("Running comprehensive quality checks...");

    run_format_check()?;
    run_lint()?;
    run_doc_check()?;
    run_audit()?;

    println!("All quality checks passed!");
    Ok(())
}

/// Run clippy linter
///
/// # Errors
///
/// Returns an error if linting fails.
pub fn run_lint() -> anyhow::Result<()> {
    println!("Running linter...");

    let output = duct::cmd!(
        "cargo",
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
        "-W",
        "clippy::pedantic",
        "-W",
        "clippy::nursery"
    )
    .stdout_capture()
    .stderr_capture()
    .run()?;

    if output.status.success() {
        println!("Linting passed");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Linting failed: {stderr}");
    }
}

/// Run code formatting
///
/// # Errors
///
/// Returns an error if formatting fails.
pub fn run_format() -> anyhow::Result<()> {
    println!("Formatting code...");

    let output = duct::cmd!("cargo", "fmt")
        .stdout_capture()
        .stderr_capture()
        .run()?;

    if output.status.success() {
        println!("Code formatted");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Formatting failed: {stderr}");
    }
}

/// Check code formatting without making changes
///
/// # Errors
///
/// Returns an error if formatting issues are found.
pub fn run_format_check() -> anyhow::Result<()> {
    println!("Checking code formatting...");

    let output = duct::cmd!("cargo", "fmt", "--check")
        .stdout_capture()
        .stderr_capture()
        .run()?;

    if output.status.success() {
        println!("Code formatting is correct");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Code formatting issues found: {stderr}");
    }
}

/// Check documentation
///
/// # Errors
///
/// Returns an error if documentation check fails.
pub fn run_doc_check() -> anyhow::Result<()> {
    println!("Checking documentation...");

    let output = duct::cmd!("cargo", "doc", "--no-deps", "--document-private-items")
        .stdout_capture()
        .stderr_capture()
        .run()?;

    if output.status.success() {
        println!("Documentation checks passed");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Documentation check failed: {stderr}");
    }
}

/// Run security audit
///
/// # Errors
///
/// Returns an error if security audit fails.
pub fn run_audit() -> anyhow::Result<()> {
    println!("Running security audit...");

    let output = duct::cmd!("cargo", "audit", "--quiet")
        .stdout_capture()
        .stderr_capture()
        .run()?;

    if output.status.success() {
        println!("Security audit passed");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Security audit found issues:");
        println!("{stderr}");
        anyhow::bail!("Security audit failed");
    }
}
