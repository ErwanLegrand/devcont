//! Integration tests for `devcont info` (with engine probe).
//!
//! Verifies the probe behaviour of `devcont info` when `--no-probe` is NOT passed.
//! These tests do not require a running container engine: they verify that
//! `info` correctly attempts a probe and surfaces the result (or error).

use std::path::PathBuf;
use std::process::Command;

/// Path to the `inspect_project` fixture that has a proper `.devcontainer/` layout.
fn inspect_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/inspect_project")
}

/// Without `--no-probe`, the `info` command must attempt to probe the engine.
///
/// When the engine is available and the container does not exist, the output
/// should contain `"exists": false, "running": false`.
///
/// When the engine is NOT available (e.g. Docker not installed), the command
/// should exit non-zero with a diagnostic on stderr.
///
/// This test is marked `#[ignore]` because it requires a live engine. Run with:
///   cargo test --test info_probe -- --ignored
#[test]
#[ignore]
fn info_with_probe_includes_exists_and_running() {
    let output = Command::new(env!("CARGO_BIN_EXE_devcont"))
        .args(["info", inspect_fixture().to_str().unwrap()])
        .output()
        .expect("failed to run devcont info");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(0),
        "exit code should be 0 when engine is available, got {:?}; stderr: {stderr}",
        output.status.code()
    );

    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout is not valid JSON: {e}"));

    assert!(
        parsed.get("exists").is_some(),
        "JSON must contain 'exists' without --no-probe"
    );
    assert!(
        parsed.get("running").is_some(),
        "JSON must contain 'running' without --no-probe"
    );
}

/// When the engine is unavailable, `info` (without `--no-probe`) exits non-zero
/// with a diagnostic on stderr.
///
/// This test intentionally uses a nonexistent project directory to force the
/// config-load failure path (exit 2), which is more reliably testable than
/// simulating an engine failure.
#[test]
fn info_with_missing_config_exits_two_and_has_stderr() {
    let tmpdir_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock error")
        .as_nanos();
    let tmpdir = std::env::temp_dir().join(format!("devcont_probe_test_{tmpdir_ns}"));
    std::fs::create_dir_all(&tmpdir).expect("failed to create tmpdir");

    let output = Command::new(env!("CARGO_BIN_EXE_devcont"))
        .args(["info", tmpdir.to_str().unwrap()])
        .output()
        .expect("failed to run devcont info");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    std::fs::remove_dir_all(&tmpdir).ok();

    assert_eq!(
        output.status.code(),
        Some(2),
        "exit code should be 2 when devcontainer.json is missing, got {:?}; stderr: {stderr}",
        output.status.code()
    );
    assert!(
        stdout.is_empty(),
        "stdout should be empty on config load error, got: {stdout:?}"
    );
    assert!(
        !stderr.is_empty(),
        "stderr should contain a diagnostic, got empty"
    );
}
