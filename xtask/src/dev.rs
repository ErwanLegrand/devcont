// Copyright (c) 2025 Erwan Patrick Legrand

//! Development utilities -- file-watching rebuild via cargo-watch.

use clap::Args;
use std::process::Command;

#[derive(Args)]
pub struct DevArgs {
    /// Extra arguments forwarded to cargo watch (e.g. -- --bin myapp)
    #[arg(last = true)]
    extra: Vec<String>,
}

/// Launch `cargo watch` for iterative development.
///
/// Runs `cargo watch -x check -x test -x run` by default, forwarding any
/// trailing arguments supplied by the caller.
///
/// # Errors
///
/// Returns an error if `cargo-watch` is not installed or exits with failure.
pub fn run(args: DevArgs) -> anyhow::Result<()> {
    println!("Starting dev watch (cargo watch)...");

    let mut cmd = Command::new("cargo");
    cmd.args(["watch", "-x", "check", "-x", "test"]);

    for arg in &args.extra {
        cmd.arg(arg);
    }

    let status = cmd
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch cargo-watch: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("cargo watch exited with {status}")
    }
}
