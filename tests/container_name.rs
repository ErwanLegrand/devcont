//! Integration tests for `devcont container-name`.
//!
//! Verifies:
//! - exit code 0, correct name on stdout, empty stderr (happy path)
//! - exit code 2, non-empty stderr, empty stdout (missing devcontainer.json)

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Path to the `inspect_project` fixture that has a proper `.devcontainer/` layout.
fn inspect_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/inspect_project")
}

/// Create a unique empty temporary directory for tests that need a dir without config.
fn unique_empty_dir() -> PathBuf {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock error")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("devcont_test_{ns}"));
    std::fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

/// `devcont container-name <dir>` prints the container name and exits 0.
#[test]
fn container_name_prints_name_and_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_devcont"))
        .args(["container-name", inspect_fixture().to_str().unwrap()])
        .output()
        .expect("failed to run devcont container-name");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(0),
        "exit code should be 0, got {:?}; stderr: {stderr}",
        output.status.code()
    );
    assert_eq!(
        stdout.trim(),
        "devcont-inspect-project",
        "stdout should be the container name, got: {stdout:?}"
    );
    assert!(
        stderr.is_empty(),
        "stderr should be empty on success, got: {stderr:?}"
    );
}

/// `devcont container-name <missing-dir>` exits 2 with a diagnostic on stderr.
#[test]
fn container_name_missing_config_exits_two() {
    let tmpdir = unique_empty_dir();

    let output = Command::new(env!("CARGO_BIN_EXE_devcont"))
        .args(["container-name", tmpdir.to_str().unwrap()])
        .output()
        .expect("failed to run devcont container-name on missing config");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Clean up temp dir.
    std::fs::remove_dir_all(&tmpdir).ok();

    assert_eq!(
        output.status.code(),
        Some(2),
        "exit code should be 2 when devcontainer.json is missing, got {:?}; stderr: {stderr}",
        output.status.code()
    );
    assert!(
        stdout.is_empty(),
        "stdout should be empty on error, got: {stdout:?}"
    );
    assert!(
        !stderr.is_empty(),
        "stderr should contain a diagnostic message, got empty"
    );
}
