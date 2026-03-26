// Copyright (c) 2025 Erwan Patrick Legrand

//! Build and dependency management operations

use clap::Args;

#[derive(Args)]
pub struct BuildArgs {
    /// Build in release mode
    #[arg(long)]
    release: bool,

    /// Build all targets
    #[arg(long)]
    all_targets: bool,

    /// Build all feature combinations
    #[arg(long)]
    all_features: bool,

    /// Additional cargo arguments
    #[arg(last = true)]
    cargo_args: Vec<String>,
}

pub fn run(args: &BuildArgs) -> anyhow::Result<()> {
    println!("Building project...");

    let mut cmd_args = vec!["build"];

    if args.release {
        cmd_args.push("--release");
    }

    if args.all_targets {
        cmd_args.push("--all-targets");
    }

    if args.all_features {
        cmd_args.push("--all-features");
    }

    // Add any additional cargo arguments
    cmd_args.extend(args.cargo_args.iter().map(String::as_str));

    let output = duct::cmd("cargo", cmd_args)
        .stdout_capture()
        .stderr_capture()
        .run()?;

    if output.status.success() {
        if args.release {
            println!("Release build completed successfully");
        } else {
            println!("Debug build completed successfully");
        }
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Build failed: {stderr}");
    }
}

pub fn run_update() -> anyhow::Result<()> {
    println!("Updating dependencies...");

    let output = duct::cmd!("cargo", "update")
        .stdout_capture()
        .stderr_capture()
        .run()?;

    if output.status.success() {
        println!("Dependencies updated successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Update failed: {stderr}");
    }
}

pub fn run_outdated() -> anyhow::Result<()> {
    println!("Checking for outdated dependencies...");

    let output = duct::cmd!("cargo", "outdated")
        .stdout_capture()
        .stderr_capture()
        .run()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            println!("All dependencies are up to date");
        } else {
            println!(" Outdated dependencies:");
            println!("{stdout}");
        }
    } else {
        // cargo outdated might not be installed, try alternative approach
        println!("cargo-outdated not available, checking with cargo tree...");

        let output = duct::cmd!("cargo", "tree")
            .stdout_capture()
            .stderr_capture()
            .run()?;

        if output.status.success() {
            println!(
                "Dependency tree checked (consider installing cargo-outdated for better reports)"
            );
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Tree check failed: {stderr}");
        }
    }

    Ok(())
}
