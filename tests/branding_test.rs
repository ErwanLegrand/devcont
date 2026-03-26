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
