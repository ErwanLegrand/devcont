// Copyright (c) 2025 Erwan Patrick Legrand

use clap::{Parser, Subcommand};
use std::process::{self, Command};

mod build;
mod ci;
mod dev;
mod quality;
mod test;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Rust Project Development Tasks")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build and dependency management
    Build(build::BuildArgs),
    /// Testing operations
    Test(test::TestArgs),
    /// Code quality checks
    Quality,
    /// Linting (subset of quality)
    Lint,
    /// Code formatting
    Format,
    /// Check formatting without changes
    FormatCheck,
    /// Documentation checks
    DocCheck,
    /// Security auditing
    Audit,
    /// Development server and utilities
    Dev(dev::DevArgs),
    /// CI/CD pipeline operations
    Ci,
    /// Clean build artifacts
    Clean,
    /// Generate documentation
    Docs,
    /// Update dependencies
    Update,
    /// Check for outdated dependencies
    Outdated,
    /// Setup development environment and tools
    Setup,
    /// Run comprehensive checks (format, lint, test)
    Check,
    /// Pre-release verification (dry-run publish, doc warnings, metadata)
    ReleaseCheck,
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "xtask=info".to_string()))
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Build(args) => build::run(&args),
        Commands::Test(args) => test::run(&args),
        Commands::Quality => quality::run_quality(),
        Commands::Lint => quality::run_lint(),
        Commands::Format => quality::run_format(),
        Commands::FormatCheck => quality::run_format_check(),
        Commands::DocCheck => quality::run_doc_check(),
        Commands::Audit => quality::run_audit(),
        Commands::Dev(args) => dev::run(args),
        Commands::Ci => ci::run(),
        Commands::Clean => run_clean(),
        Commands::Docs => run_docs(),
        Commands::Update => build::run_update(),
        Commands::Outdated => build::run_outdated(),
        Commands::Setup => run_setup(),
        Commands::Check => run_check(),
        Commands::ReleaseCheck => run_release_check(),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
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
    println!("release-check: publish dry-run passed");

    // 2. cargo doc --no-deps with warnings denied
    println!("release-check: cargo doc --no-deps (warnings denied)");
    let status = Command::new("cargo")
        .args(["doc", "--no-deps"])
        .env("RUSTDOCFLAGS", "-D warnings")
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch cargo doc: {e}"))?;
    if !status.success() {
        anyhow::bail!("cargo doc with -D warnings failed (exit {status})");
    }
    println!("release-check: documentation passed");

    // 3. Verify required Cargo.toml fields
    println!("release-check: verifying Cargo.toml metadata");
    verify_cargo_toml_metadata()?;
    println!("release-check: metadata check passed");

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

    let required_fields = ["license", "description", "repository"];
    let mut missing = Vec::new();

    for field in &required_fields {
        let value = package.get(field);
        match value {
            // Field present and is a plain string
            Some(toml::Value::String(s)) if !s.is_empty() => {}
            // Field present as a table (e.g. { workspace = true }) -- acceptable
            Some(toml::Value::Table(_)) => {}
            _ => missing.push(*field),
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(
            "Cargo.toml missing required fields for publishing: {}",
            missing.join(", ")
        )
    }
}

fn run_clean() -> anyhow::Result<()> {
    println!("Cleaning build artifacts...");

    let cmd = duct::cmd!("cargo", "clean")
        .stdout_capture()
        .stderr_capture();

    let output = cmd.run()?;

    if output.status.success() {
        println!("Clean completed successfully");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Clean failed: {stderr}");
    }

    Ok(())
}

fn run_docs() -> anyhow::Result<()> {
    println!("Generating documentation...");

    let cmd = duct::cmd!("cargo", "doc", "--no-deps", "--document-private-items")
        .stdout_capture()
        .stderr_capture();

    let output = cmd.run()?;

    if output.status.success() {
        println!("Documentation generated successfully");
        println!("Open docs with: cargo doc --open");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Documentation generation failed: {stderr}");
    }

    Ok(())
}

fn run_check() -> anyhow::Result<()> {
    println!("Running comprehensive checks...");

    // Check development environment setup (non-blocking)
    check_development_setup()?;

    // Run format check
    quality::run_format_check()?;
    // Run linting
    quality::run_lint()?;
    // Run tests
    test::run(&test::TestArgs::default())?;

    println!("All checks passed!");
    Ok(())
}

fn run_setup() -> anyhow::Result<()> {
    println!("Setting up development environment...");

    // Set up git hooks (generic)
    setup_git_hooks()?;

    // Install xtask-required tools
    install_xtask_tools()?;

    // Check and install pre-commit (if configured)
    setup_pre_commit()?;

    // Check Rust toolchain
    check_rust_setup()?;

    println!("Development environment ready!");
    println!("Run 'cargo xtask check' to verify everything works");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn check_development_setup() -> anyhow::Result<()> {
    // Check git hooks
    check_git_hooks_setup();

    // Check pre-commit setup
    check_pre_commit_setup();

    Ok(())
}

fn check_git_hooks_setup() {
    let git_hooks_dir = std::path::Path::new(".git/hooks");
    let project_hooks_dir = std::path::Path::new(".git-hooks");

    // Check if project has custom hooks
    if project_hooks_dir.exists()
        && let Ok(entries) = std::fs::read_dir(project_hooks_dir)
    {
        let mut hook_count = 0;
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                let hook_path = git_hooks_dir.join(file_name);
                if !hook_path.exists() {
                    println!("Git hook '{file_name}' not installed");
                    hook_count += 1;
                }
            }
        }
        if hook_count > 0 {
            println!("Consider running: cargo xtask setup");
        } else {
            println!("Project git hooks are installed");
        }
    }
}

fn check_pre_commit_setup() {
    // Check if .pre-commit-config.yaml exists
    if !std::path::Path::new(".pre-commit-config.yaml").exists() {
        return; // Pre-commit is optional
    }

    // Check if pre-commit hooks are installed
    let git_hooks_dir = std::path::Path::new(".git/hooks");
    let pre_commit_hook = git_hooks_dir.join("pre-commit");

    if !pre_commit_hook.exists() {
        println!("Pre-commit config found but hooks not installed");
        println!("Consider running: cargo xtask setup");
        return;
    }

    println!("Pre-commit hooks are installed");
}

#[allow(clippy::unnecessary_wraps)]
fn setup_git_hooks() -> anyhow::Result<()> {
    let project_hooks_dir = std::path::Path::new(".git-hooks");
    let git_hooks_dir = std::path::Path::new(".git/hooks");

    // Check if project has custom hooks
    if !project_hooks_dir.exists() {
        return Ok(()); // No custom hooks to install
    }

    println!("Installing custom git hooks...");

    if let Ok(entries) = std::fs::read_dir(project_hooks_dir) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                let source_path = entry.path();
                let target_path = git_hooks_dir.join(file_name);

                // Copy the hook file
                if let Err(e) = std::fs::copy(&source_path, &target_path) {
                    println!("Failed to install hook '{file_name}': {e}");
                    continue;
                }

                // Make it executable
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = std::fs::metadata(&target_path) {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        let _ = std::fs::set_permissions(&target_path, perms);
                    }
                }

                println!("Installed hook: {file_name}");
            }
        }
    }

    println!("Custom git hooks installed");
    Ok(())
}

#[allow(clippy::unnecessary_wraps, clippy::if_not_else)]
fn install_xtask_tools() -> anyhow::Result<()> {
    println!("Installing xtask-required cargo tools...");

    // List of tools that xtask commands depend on
    let tools = vec![
        "cargo-llvm-cov", // Coverage reporting
        "cargo-nextest",  // Fast testing
        "cargo-watch",    // File watching
        "cargo-edit",     // Dependency management
        "cargo-audit",    // Security auditing
        "cargo-deny",     // License checking
        "cargo-outdated", // Dependency updates
    ];

    let mut installed_count = 0;

    for tool in tools {
        // Check if tool is already installed
        let check_result = duct::cmd!("cargo", "install", "--list")
            .stdout_capture()
            .stderr_capture()
            .run();

        let is_installed = if let Ok(output) = check_result {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains(tool)
            } else {
                false
            }
        } else {
            false
        };

        if !is_installed {
            println!("Installing {tool}...");
            let install_result = duct::cmd!("cargo", "install", tool)
                .stdout_capture()
                .stderr_capture()
                .run();

            if install_result.is_ok() && install_result.unwrap().status.success() {
                println!("Installed {tool}");
                installed_count += 1;
            } else {
                println!("Failed to install {tool}");
            }
        } else {
            println!("{tool} already installed");
        }
    }

    if installed_count > 0 {
        println!("Installed {installed_count} new tools");
    } else {
        println!("All xtask tools already installed");
    }

    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn setup_pre_commit() -> anyhow::Result<()> {
    // Check if .pre-commit-config.yaml exists
    if !std::path::Path::new(".pre-commit-config.yaml").exists() {
        return Ok(()); // Pre-commit is optional
    }

    println!("Setting up pre-commit hooks...");

    // Check if pre-commit is installed
    let pre_commit_check = duct::cmd!("which", "pre-commit")
        .stdout_capture()
        .stderr_capture()
        .run();

    if pre_commit_check.is_err() || !pre_commit_check.unwrap().status.success() {
        println!("Installing pre-commit...");

        // Try different installation methods
        let install_result = duct::cmd!("pip", "install", "pre-commit")
            .stdout_capture()
            .stderr_capture()
            .run();

        if install_result.is_err() || !install_result.unwrap().status.success() {
            let install_result = duct::cmd!("pip3", "install", "pre-commit")
                .stdout_capture()
                .stderr_capture()
                .run();

            if install_result.is_err() || !install_result.unwrap().status.success() {
                println!("Failed to install pre-commit automatically");
                println!("Install manually with: pip install pre-commit");
                return Ok(());
            }
        }

        println!("Pre-commit installed");
    }

    // Install hooks
    let install_result = duct::cmd!("pre-commit", "install")
        .stdout_capture()
        .stderr_capture()
        .run();

    if install_result.is_err() || !install_result.unwrap().status.success() {
        println!("Failed to install pre-commit hooks");
        return Ok(());
    }

    // Install commit-msg hooks if available
    let _ = duct::cmd!("pre-commit", "install", "--hook-type", "commit-msg")
        .stdout_capture()
        .stderr_capture()
        .run();

    println!("Pre-commit hooks installed");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn check_rust_setup() -> anyhow::Result<()> {
    // Check if rustfmt and clippy are available
    let components = vec!["rustfmt", "clippy"];

    for component in components {
        let check_result = duct::cmd!("cargo", component, "--version")
            .stdout_capture()
            .stderr_capture()
            .run();

        if check_result.is_err() || !check_result.unwrap().status.success() {
            println!("{component} not available, installing...");
            let _ = duct::cmd!("rustup", "component", "add", component)
                .stdout_capture()
                .run();
        }
    }

    println!("Rust toolchain ready");
    Ok(())
}
