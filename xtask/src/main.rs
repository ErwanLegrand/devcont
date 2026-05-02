//! Development task runner for devcont.

use clap::{Parser, Subcommand};
use std::process::{self, Command};

mod ci;
mod quality;
mod test;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Devcont development tasks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all quality checks (format + lint + test + audit)
    Quality,
    /// Run format-check + lint + test
    Check,
    /// Run clippy linter
    Lint,
    /// Format code
    Format,
    /// Check formatting without changes
    FormatCheck,
    /// Check documentation builds
    DocCheck,
    /// Run security audit (cargo-deny + cargo-audit)
    Audit,
    /// Run test suite
    Test,
    /// Run tests with coverage
    Coverage,
    /// Run CI pipeline
    Ci,
    /// Clean build artifacts
    Clean,
    /// Generate documentation
    Docs,
    /// Setup development environment (hooks, tools, pre-commit)
    Setup,
    /// Pre-release verification (dry-run publish, doc warnings, metadata)
    ReleaseCheck,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Quality => quality::run_quality(),
        Commands::Check => run_check(),
        Commands::Lint => quality::run_lint(),
        Commands::Format => quality::run_format(),
        Commands::FormatCheck => quality::run_format_check(),
        Commands::DocCheck => quality::run_doc_check(),
        Commands::Audit => quality::run_audit(),
        Commands::Test => test::run(),
        Commands::Coverage => test::run_coverage(),
        Commands::Ci => ci::run(),
        Commands::Clean => run_clean(),
        Commands::Docs => run_docs(),
        Commands::Setup => run_setup(),
        Commands::ReleaseCheck => run_release_check(),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run_check() -> anyhow::Result<()> {
    println!("Running comprehensive checks...");
    quality::run_format_check()?;
    quality::run_lint()?;
    test::run()?;
    println!("All checks passed!");
    Ok(())
}

fn run_clean() -> anyhow::Result<()> {
    println!("Cleaning build artifacts...");
    let status = Command::new("cargo")
        .args(["clean"])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch cargo clean: {e}"))?;

    if status.success() {
        println!("Clean completed");
        Ok(())
    } else {
        anyhow::bail!("Clean failed (exit {status})")
    }
}

fn run_docs() -> anyhow::Result<()> {
    println!("Generating documentation...");
    let status = Command::new("cargo")
        .args(["doc", "--no-deps", "--document-private-items"])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch cargo doc: {e}"))?;

    if status.success() {
        println!("Documentation generated. Open with: cargo doc --open");
        Ok(())
    } else {
        anyhow::bail!("Documentation generation failed (exit {status})")
    }
}

fn run_setup() -> anyhow::Result<()> {
    println!("Setting up development environment...");

    // Install pre-commit hooks if config exists
    if std::path::Path::new(".pre-commit-config.yaml").exists() {
        println!("Installing pre-commit hooks...");
        let status = Command::new("pre-commit").args(["install"]).status();

        match status {
            Ok(s) if s.success() => println!("Pre-commit hooks installed"),
            _ => println!(
                "pre-commit not available, skipping (install with: pip install pre-commit)"
            ),
        }

        drop(
            Command::new("pre-commit")
                .args(["install", "--hook-type", "commit-msg"])
                .status(),
        );
    }

    // Check Rust components
    for component in &["rustfmt", "clippy"] {
        let check = Command::new("rustup")
            .args(["component", "list", "--installed"])
            .output();

        if let Ok(output) = check {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.contains(component) {
                println!("Installing {component}...");
                drop(
                    Command::new("rustup")
                        .args(["component", "add", component])
                        .status(),
                );
            }
        }
    }

    println!("Development environment ready!");
    println!("Run 'cargo xtask check' to verify everything works");
    Ok(())
}

fn run_release_check() -> anyhow::Result<()> {
    println!("Running pre-release checks...");

    // 1. cargo publish --dry-run
    println!("release-check: cargo publish --dry-run");
    let status = Command::new("cargo")
        .args(["publish", "--dry-run"])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch cargo publish: {e}"))?;
    if !status.success() {
        anyhow::bail!("cargo publish --dry-run failed (exit {status})");
    }

    // 2. cargo doc with warnings denied
    println!("release-check: cargo doc --no-deps (warnings denied)");
    let status = Command::new("cargo")
        .args(["doc", "--no-deps"])
        .env("RUSTDOCFLAGS", "-D warnings")
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch cargo doc: {e}"))?;
    if !status.success() {
        anyhow::bail!("cargo doc with -D warnings failed (exit {status})");
    }

    // 3. Verify Cargo.toml metadata
    println!("release-check: verifying Cargo.toml metadata");
    verify_cargo_toml_metadata()?;

    println!("All pre-release checks passed!");
    Ok(())
}

fn verify_cargo_toml_metadata() -> anyhow::Result<()> {
    let content = std::fs::read_to_string("Cargo.toml")
        .map_err(|e| anyhow::anyhow!("could not read Cargo.toml: {e}"))?;

    let doc: toml::Value =
        toml::from_str(&content).map_err(|e| anyhow::anyhow!("invalid Cargo.toml: {e}"))?;

    let package = doc
        .get("package")
        .ok_or_else(|| anyhow::anyhow!("Cargo.toml missing [package] table"))?;

    let required = ["license", "description"];
    let mut missing = Vec::new();

    for field in &required {
        match package.get(field) {
            Some(toml::Value::String(s)) if !s.is_empty() => {}
            Some(toml::Value::Table(_)) => {} // { workspace = true }
            _ => missing.push(*field),
        }
    }

    if missing.is_empty() {
        println!("release-check: metadata check passed");
        Ok(())
    } else {
        anyhow::bail!("Cargo.toml missing required fields: {}", missing.join(", "))
    }
}
