#![warn(clippy::pedantic)]
use clap::{Parser, Subcommand};
pub(crate) mod commands; // sub-command handlers
/// Top-level CLI definition.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,
}
/// Sub-commands exposed by the binary.
#[derive(Subcommand)]
#[allow(clippy::doc_markdown)]
enum CliCommand {
    /// Tear down and recreate the dev container from scratch.
    Rebuild {
        // -- rebuild variant --
        /// Optional project directory path.
        dir: Option<String>, // project dir
        /// Skip the Docker layer cache during image build.
        #[arg(short, long)]
        no_cache: bool, // cache flag
        /// Trust the devcontainer and skip the initializeCommand confirmation prompt.
        #[arg(long)]
        trust: bool,
        /// Suppress the warning when the container will run as root with no remoteUser configured.
        #[arg(long)]
        no_root_check: bool,
        /// Disable the structured audit log for this invocation.
        #[arg(long)]
        no_audit_log: bool,
        /// Override the per-hook timeout in seconds (overrides hookTimeoutSeconds in config).
        #[arg(long)]
        hook_timeout: Option<u32>,
    }, // end Rebuild
    /// Launch (or re-attach to) the dev container.
    Start {
        // -- start variant --
        /// Optional project directory path.
        dir: Option<String>, // project dir
        /// Trust the devcontainer and skip the initializeCommand confirmation prompt.
        #[arg(long)]
        trust: bool,
        /// Suppress the warning when the container will run as root with no remoteUser configured.
        #[arg(long)]
        no_root_check: bool,
        /// Disable the structured audit log for this invocation.
        #[arg(long)]
        no_audit_log: bool,
        /// Override the per-hook timeout in seconds (overrides hookTimeoutSeconds in config).
        #[arg(long)]
        hook_timeout: Option<u32>,
    }, // end Start
    /// Print the deterministic container name for the dev container and exit.
    ///
    /// Reads devcontainer.json without starting the container or running any hooks.
    ///
    /// Exit codes:
    ///   0 — name determined and printed.
    ///   2 — devcontainer.json could not be loaded (diagnostic on stderr).
    ///   3 — config loaded but name could not be derived.
    ContainerName {
        // -- container-name variant --
        /// Optional project directory path (defaults to current directory).
        dir: Option<String>,
    }, // end ContainerName
} // end CliCommand
/// Route parsed CLI to the appropriate handler.
fn dispatch(parsed: &Cli) -> std::io::Result<()> {
    match &parsed.command {
        Some(CliCommand::Start {
            dir,
            trust,
            no_root_check,
            no_audit_log,
            hook_timeout,
        }) => {
            commands::start::run(
                dir.as_deref(),
                *trust,
                *no_root_check,
                *no_audit_log,
                *hook_timeout,
            )?;
        } // handled start
        Some(CliCommand::Rebuild {
            dir,
            no_cache,
            trust,
            no_root_check,
            no_audit_log,
            hook_timeout,
        }) => {
            commands::rebuild::run(
                dir.as_deref(),
                !no_cache,
                *trust,
                *no_root_check,
                *no_audit_log,
                *hook_timeout,
            )?;
        } // handled rebuild
        Some(CliCommand::ContainerName { dir }) => {
            commands::container_name::run(dir.as_deref());
        } // handled container-name
        None => commands::start::run(None, false, false, false, None)?,
    } // match dispatch
    Ok(()) // dispatch result
} // end dispatch
fn main() -> anyhow::Result<()> {
    // Initialise tracing — respects RUST_LOG env var (default: warn).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .without_time()
        .with_target(false)
        .init();

    let parsed = Cli::parse();
    dispatch(&parsed)?;
    Ok(()) // entry point result
} // end main
