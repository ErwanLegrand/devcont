/// Integration tests verifying the devcont branding contract:
/// - Binary is named `devcont`
/// - Container name prefix is `devcont-`
use std::process::Command;

/// The binary name must be `devcont`.
#[test]
fn binary_name_is_devcont() {
    let output = Command::new(env!("CARGO_BIN_EXE_devcont"))
        .arg("--version")
        .output()
        .expect("failed to run devcont --version");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("devcont"),
        "--version output should contain 'devcont', got: {stdout}"
    );
}

/// `devcont --help` should mention the `devcont` binary name in usage.
#[test]
fn help_output_references_devcont() {
    let output = Command::new(env!("CARGO_BIN_EXE_devcont"))
        .arg("--help")
        .output()
        .expect("failed to run devcont --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("devcont"),
        "--help output should reference 'devcont', got: {stdout}"
    );
}

/// `devcont --help` lists subcommands in the correct order:
/// `rebuild`, `start`, `up`, `container-name`, `info`, `help`.
#[test]
fn help_lists_subcommands_in_correct_order() {
    let output = Command::new(env!("CARGO_BIN_EXE_devcont"))
        .arg("--help")
        .output()
        .expect("failed to run devcont --help");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify each subcommand is present.
    for subcmd in &["rebuild", "start", "up", "container-name", "info", "help"] {
        assert!(
            stdout.contains(subcmd),
            "--help should list '{subcmd}', got: {stdout}"
        );
    }

    // Verify order: rebuild < start < up < container-name < info < help.
    // Use leading two-space prefix to match the indented subcommand listing in clap output,
    // avoiding accidental matches against words inside doc-comments.
    let pos_rebuild = stdout.find("  rebuild").expect("rebuild not found");
    let pos_start = stdout.find("  start").expect("start not found");
    let pos_up = stdout.find("  up").expect("up not found");
    let pos_container_name = stdout
        .find("  container-name")
        .expect("container-name not found");
    let pos_info = stdout.find("  info").expect("info not found");
    let pos_help = stdout.rfind("  help").expect("help not found");

    assert!(
        pos_rebuild < pos_start,
        "rebuild should appear before start in --help"
    );
    assert!(
        pos_start < pos_up,
        "start should appear before up in --help"
    );
    assert!(
        pos_up < pos_container_name,
        "up should appear before container-name in --help"
    );
    assert!(
        pos_container_name < pos_info,
        "container-name should appear before info in --help"
    );
    assert!(
        pos_info < pos_help,
        "info should appear before help in --help"
    );
}

/// `devcont up --help` should document the non-attach semantics.
#[test]
fn up_help_documents_non_attach_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_devcont"))
        .args(["up", "--help"])
        .output()
        .expect("failed to run devcont up --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Must mention: does not attach
    assert!(
        stdout.contains("attach"),
        "devcont up --help should mention 'attach', got: {stdout}"
    );
    // Must mention: suitable for scripts
    assert!(
        stdout.contains("script") || stdout.contains("Suitable"),
        "devcont up --help should mention scripting use, got: {stdout}"
    );
}
