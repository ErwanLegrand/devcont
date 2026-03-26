// Copyright (c) 2025 Erwan Patrick Legrand

//! Testing operations

use clap::Args;

#[derive(Args, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct TestArgs {
    /// Run tests with coverage
    #[arg(long)]
    coverage: bool,

    /// Show coverage summary
    #[arg(long)]
    summary: bool,

    /// Generate HTML coverage report
    #[arg(long)]
    html: bool,

    /// Run examples along with tests
    #[arg(long)]
    examples: bool,

    /// Run doc tests
    #[arg(long)]
    doc: bool,

    /// Additional cargo arguments
    #[arg(last = true)]
    cargo_args: Vec<String>,
}

pub fn run(args: &TestArgs) -> anyhow::Result<()> {
    println!("Running tests with nextest...");

    let mut cmd_args = vec!["nextest", "run"];

    // Add any additional cargo arguments
    cmd_args.extend(args.cargo_args.iter().map(String::as_str));

    let output = duct::cmd("cargo", cmd_args)
        .stdout_capture()
        .stderr_capture()
        .run()?;

    if output.status.success() {
        println!("Tests passed");

        if args.doc {
            println!("Running doc tests...");
            let doc_output = duct::cmd!("cargo", "test", "--doc")
                .stdout_capture()
                .stderr_capture()
                .run()?;
            if doc_output.status.success() {
                println!("Doc tests passed");
            } else {
                let stderr = String::from_utf8_lossy(&doc_output.stderr);
                anyhow::bail!("Doc tests failed: {stderr}");
            }
        }

        if args.examples {
            println!("Running examples...");
            let examples_output = duct::cmd!("cargo", "test", "--examples")
                .stdout_capture()
                .stderr_capture()
                .run()?;
            if examples_output.status.success() {
                println!("Examples passed");
            } else {
                let stderr = String::from_utf8_lossy(&examples_output.stderr);
                anyhow::bail!("Examples failed: {stderr}");
            }
        }

        if args.coverage {
            run_coverage(args)?;
        }

        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Tests failed: {stderr}");
    }
}

fn run_coverage(args: &TestArgs) -> anyhow::Result<()> {
    println!("Generating test coverage with llvm-cov...");

    let mut cmd_args = vec!["llvm-cov", "--workspace"];

    if args.html {
        cmd_args.push("--html");
    }

    let output = duct::cmd("cargo", cmd_args)
        .stdout_capture()
        .stderr_capture()
        .run()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Coverage Report:");
        println!("{stdout}");

        if args.html {
            println!("HTML coverage report generated");
            println!("Open target/llvm-cov/html/index.html in your browser");
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Coverage generation failed: {stderr}");
        println!("Install llvm-cov with: cargo install cargo-llvm-cov");
        println!(
            "   Also ensure you have the llvm-tools component: rustup component add llvm-tools-preview"
        );
    }

    Ok(())
}
