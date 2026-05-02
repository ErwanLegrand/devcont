//! Integration tests for `devcont info --no-probe`.
//!
//! Verifies:
//! - exit code 0 on success
//! - stdout is valid JSON
//! - JSON contains keys: container, image, workspace, config_dir, engine
//! - JSON does NOT contain keys: exists, running

use std::path::PathBuf;
use std::process::Command;

/// Path to the `inspect_project` fixture that has a proper `.devcontainer/` layout.
fn inspect_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/inspect_project")
}

/// `devcont info --no-probe <dir>` exits 0 and prints valid JSON.
#[test]
fn info_no_probe_exits_zero_with_valid_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_devcont"))
        .args(["info", "--no-probe", inspect_fixture().to_str().unwrap()])
        .output()
        .expect("failed to run devcont info --no-probe");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(0),
        "exit code should be 0, got {:?}; stderr: {stderr}",
        output.status.code()
    );

    // Must be valid JSON.
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}\nstdout: {stdout:?}"));

    // Required fields.
    assert!(
        parsed.get("container").is_some(),
        "JSON must contain 'container', got: {parsed}"
    );
    assert!(
        parsed.get("image").is_some(),
        "JSON must contain 'image', got: {parsed}"
    );
    assert!(
        parsed.get("workspace").is_some(),
        "JSON must contain 'workspace', got: {parsed}"
    );
    assert!(
        parsed.get("config_dir").is_some(),
        "JSON must contain 'config_dir', got: {parsed}"
    );
    assert!(
        parsed.get("engine").is_some(),
        "JSON must contain 'engine', got: {parsed}"
    );

    // Probe fields must be absent with --no-probe.
    assert!(
        parsed.get("exists").is_none(),
        "JSON must NOT contain 'exists' with --no-probe, got: {parsed}"
    );
    assert!(
        parsed.get("running").is_none(),
        "JSON must NOT contain 'running' with --no-probe, got: {parsed}"
    );
}

/// The `container` field matches the expected name.
#[test]
fn info_no_probe_container_name_matches_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_devcont"))
        .args(["info", "--no-probe", inspect_fixture().to_str().unwrap()])
        .output()
        .expect("failed to run devcont info --no-probe");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}"));

    assert_eq!(
        parsed["container"].as_str(),
        Some("devcont-inspect-project"),
        "container field should match safe_name() output"
    );
}
