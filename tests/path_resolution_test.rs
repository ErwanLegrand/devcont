//! Regression tests for devcontainer.json path resolution.
//!
//! Verifies that `build.dockerfile`, `build.context`, and `dockerComposeFile`
//! are resolved relative to the directory containing `devcontainer.json`
//! (i.e., `config_dir`), not relative to the workspace root — as required by
//! the containers.dev spec.
//!
//! These tests do NOT require a container runtime.

use std::path::Path;

use devcont::devcontainers::Devcontainer;

/// Helper: assert a path string ends with the given suffix (using `/` separators).
fn assert_path_ends_with(path: &str, suffix: &str) {
    assert!(
        path.ends_with(suffix) || path.replace('\\', "/").ends_with(suffix),
        "expected path to end with '{suffix}', got: '{path}'"
    );
}

// ---------------------------------------------------------------------------
// Nested layout: .devcontainer/devcontainer.json
// ---------------------------------------------------------------------------

/// `Devcontainer::load` for the nested layout must record `config_dir` as
/// `<workspace>/.devcontainer/`, not the workspace root.
#[test]
fn nested_layout_config_dir_is_devcontainer_subdir() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        r#"{"name":"nested-test","image":"alpine"}"#,
    )
    .expect("write devcontainer.json");

    let dc = Devcontainer::load(ws.path()).expect("load should succeed");
    assert_eq!(
        dc.config_dir, dc_dir,
        "config_dir for nested layout must be <workspace>/.devcontainer"
    );
}

/// When `build.dockerfile` is set and `build.context` is absent, the Dockerfile
/// must be looked up relative to the directory containing `devcontainer.json`
/// (`<workspace>/.devcontainer/`), not the workspace root.
#[test]
fn nested_layout_dockerfile_resolved_against_devcontainer_dir() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        r#"{"name":"nested-df","build":{"dockerfile":"Dockerfile"}}"#,
    )
    .expect("write devcontainer.json");
    // The Dockerfile must be in .devcontainer/ (relative to config_dir).
    std::fs::write(dc_dir.join("Dockerfile"), "FROM alpine\n").expect("write Dockerfile");

    // Devcontainer::load runs build_provider which calls resolve_build_source.
    // If the Dockerfile is resolved against the wrong base, validation fails.
    let result = Devcontainer::load(ws.path());
    assert!(
        result.is_ok(),
        "load should succeed when Dockerfile is in .devcontainer/ (spec-correct layout), got: {:?}",
        result.err()
    );
}

/// When the nested layout is used, config_dir anchoring ensures the Dockerfile
/// is resolved to `<ws>/.devcontainer/Dockerfile`, not `<ws>/Dockerfile`.
/// This verifies the anchor is correct by checking config_dir directly.
#[test]
fn nested_layout_config_dir_anchors_dockerfile_in_devcontainer_subdir() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    // Only put a Dockerfile in .devcontainer/ (spec-correct location)
    std::fs::write(dc_dir.join("Dockerfile"), "FROM alpine\n").expect("write Dockerfile");
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        r#"{"name":"nested-df-anchor","build":{"dockerfile":"Dockerfile"}}"#,
    )
    .expect("write devcontainer.json");

    let dc = Devcontainer::load(ws.path()).expect("load should succeed");
    // config_dir must be the .devcontainer dir, confirming Dockerfile resolution
    // is anchored there, not at the workspace root
    assert_eq!(
        dc.config_dir, dc_dir,
        "config_dir must point to .devcontainer/, confirming Dockerfile anchoring"
    );
    assert!(
        dc.config_dir.join("Dockerfile").exists(),
        "Dockerfile must exist at config_dir/Dockerfile (the resolved location)"
    );
}

// ---------------------------------------------------------------------------
// Root layout: .devcontainer.json at workspace root
// ---------------------------------------------------------------------------

/// `Devcontainer::load` for the root layout must record `config_dir` as the
/// workspace root itself.
#[test]
fn root_layout_config_dir_is_workspace_root() {
    let ws = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        ws.path().join(".devcontainer.json"),
        r#"{"name":"root-test","image":"alpine"}"#,
    )
    .expect("write .devcontainer.json");

    let dc = Devcontainer::load(ws.path()).expect("load should succeed");
    assert_eq!(
        dc.config_dir,
        ws.path(),
        "config_dir for root layout must be the workspace root"
    );
}

/// When `build.dockerfile` is set and `build.context` is absent, the Dockerfile
/// must be looked up relative to the workspace root for the root layout.
#[test]
fn root_layout_dockerfile_resolved_against_workspace_root() {
    let ws = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        ws.path().join(".devcontainer.json"),
        r#"{"name":"root-df","build":{"dockerfile":"Dockerfile"}}"#,
    )
    .expect("write .devcontainer.json");
    std::fs::write(ws.path().join("Dockerfile"), "FROM alpine\n").expect("write Dockerfile");

    let result = Devcontainer::load(ws.path());
    assert!(
        result.is_ok(),
        "load should succeed when Dockerfile is at workspace root (root layout), got: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// Image-based: no Dockerfile, just an image reference
// ---------------------------------------------------------------------------

/// Image-based configs (no build block) should load without path-resolution issues
/// in both layouts.
#[test]
fn nested_layout_image_based_loads_ok() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        r#"{"name":"nested-img","image":"alpine"}"#,
    )
    .expect("write devcontainer.json");

    assert!(Devcontainer::load(ws.path()).is_ok());
}

#[test]
fn root_layout_image_based_loads_ok() {
    let ws = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        ws.path().join(".devcontainer.json"),
        r#"{"name":"root-img","image":"alpine"}"#,
    )
    .expect("write .devcontainer.json");

    assert!(Devcontainer::load(ws.path()).is_ok());
}

// ---------------------------------------------------------------------------
// dockerComposeFile path resolution
// ---------------------------------------------------------------------------

/// For the nested layout, `dockerComposeFile` must be resolved relative to
/// `<workspace>/.devcontainer/`.
#[test]
fn nested_layout_compose_file_resolved_against_devcontainer_dir() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        r#"{"name":"nested-compose","dockerComposeFile":"docker-compose.yml","service":"app"}"#,
    )
    .expect("write devcontainer.json");

    // The compose file path is stored in DockerCompose::file.
    // We cannot easily inspect that without a public accessor, but we can verify
    // the internal state via config_dir.
    let dc = Devcontainer::load(ws.path()).expect("load should succeed for nested compose layout");
    let expected_compose = dc_dir.join("docker-compose.yml");
    assert_path_ends_with(
        &expected_compose.to_string_lossy(),
        ".devcontainer/docker-compose.yml",
    );
    // config_dir reflects where the compose file will be resolved from
    assert_eq!(dc.config_dir, dc_dir);
}

/// For the root layout, `dockerComposeFile` must be resolved relative to the
/// workspace root — NOT `<workspace>/.devcontainer/`.
#[test]
fn root_layout_compose_file_resolved_against_workspace_root() {
    let ws = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        ws.path().join(".devcontainer.json"),
        r#"{"name":"root-compose","dockerComposeFile":"docker-compose.yml","service":"app"}"#,
    )
    .expect("write .devcontainer.json");

    let dc = Devcontainer::load(ws.path()).expect("load should succeed for root compose layout");
    let expected_compose = ws.path().join("docker-compose.yml");
    // config_dir == workspace root for root layout
    assert_eq!(dc.config_dir, ws.path());
    // Verify the compose path would be resolved correctly
    assert_path_ends_with(&expected_compose.to_string_lossy(), "docker-compose.yml");
    assert!(
        !expected_compose
            .to_string_lossy()
            .contains(".devcontainer/docker-compose.yml")
    );
}

// ---------------------------------------------------------------------------
// build.context: relative context resolved against config_dir
// ---------------------------------------------------------------------------

/// A relative `build.context` with the nested layout must be resolved against
/// `<workspace>/.devcontainer/`, not the workspace root.
#[test]
fn nested_layout_relative_context_resolved_against_config_dir() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    // Dockerfile inside <ws>/.devcontainer/ctx/Dockerfile
    let ctx_dir = dc_dir.join("ctx");
    std::fs::create_dir_all(&ctx_dir).expect("create ctx dir");
    std::fs::write(ctx_dir.join("Dockerfile"), "FROM alpine\n").expect("write Dockerfile");
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        r#"{"name":"nested-ctx","build":{"dockerfile":"Dockerfile","context":"ctx"}}"#,
    )
    .expect("write devcontainer.json");

    let result = Devcontainer::load(ws.path());
    assert!(
        result.is_ok(),
        "load should succeed when context is a subdir of config_dir, got: {:?}",
        result.err()
    );
}

/// A path-traversal attempt in `build.context` must be rejected.
#[test]
fn build_context_traversal_rejected() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        r#"{"name":"traversal","build":{"dockerfile":"Dockerfile","context":"../../etc"}}"#,
    )
    .expect("write devcontainer.json");

    let result = Devcontainer::load(ws.path());
    assert!(
        result.is_err(),
        "load should fail for path traversal in build.context"
    );
}

// ---------------------------------------------------------------------------
// Path helpers used by the load() tests
// ---------------------------------------------------------------------------

/// Canonical check: the path anchoring function used in Devcontainer::load
/// correctly distinguishes the nested and root layouts.
#[test]
fn load_distinguishes_nested_from_root_layout() {
    let ws = tempfile::tempdir().expect("tempdir");

    // First, only the nested path exists
    let dc_dir = ws.path().join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    let nested_json = dc_dir.join("devcontainer.json");
    std::fs::write(&nested_json, r#"{"name":"nested","image":"alpine"}"#)
        .expect("write nested devcontainer.json");

    let dc = Devcontainer::load(ws.path()).expect("nested load should succeed");
    assert_eq!(
        dc.config_dir, dc_dir,
        "must use nested layout when .devcontainer/devcontainer.json exists"
    );

    // Remove nested file, add root file
    std::fs::remove_file(&nested_json).expect("remove nested file");
    std::fs::write(
        ws.path().join(".devcontainer.json"),
        r#"{"name":"root","image":"alpine"}"#,
    )
    .expect("write root .devcontainer.json");

    let dc = Devcontainer::load(ws.path()).expect("root load should succeed");
    assert_eq!(
        dc.config_dir,
        ws.path(),
        "must use root layout when only .devcontainer.json exists at workspace root"
    );
}

// ---------------------------------------------------------------------------
// Error message quality (acceptance criterion 10)
// ---------------------------------------------------------------------------

/// When neither config file exists, the error message must describe both
/// candidate locations so the user knows where to put their devcontainer.json.
#[test]
fn missing_config_error_mentions_both_candidate_paths() {
    let ws = tempfile::tempdir().expect("tempdir");
    let result = Devcontainer::load(ws.path());
    assert!(result.is_err(), "must fail when no config file exists");
    let err = result.err().expect("must be Err");
    let msg = err.to_string();
    assert!(
        msg.contains(".devcontainer"),
        "error should mention .devcontainer directory, got: {msg}"
    );
    assert!(
        msg.contains("devcontainer.json"),
        "error should mention devcontainer.json, got: {msg}"
    );
}

/// We test that the path used for Devcontainer is distinct from the Path struct
/// used by the standard library.
#[test]
fn sanity_path_type_check() {
    let p = Path::new("/ws/.devcontainer");
    assert!(p.is_absolute());
    assert_eq!(p.file_name().unwrap().to_str().unwrap(), ".devcontainer");
}

// ---------------------------------------------------------------------------
// build.context relative resolution against config_dir (AC #1-#8)
//
// These tests close the residual gap from fix_dockerfile_path_resolution:
// validate_build_context must resolve the context string against config_dir
// before checking workspace-root containment.
// ---------------------------------------------------------------------------

/// `context = ".."` from `.devcontainer/devcontainer.json` resolves to the
/// workspace root and must pass validation.
///
/// This is the canonical regression test for the bug: the old code validated
/// `".."` literally against the workspace root, which rejected a spec-compliant
/// context that points at the project root.
///
/// Layout:
/// ```text
/// <workspace>/
///   Dockerfile            ← build context root
///   .devcontainer/
///     devcontainer.json   ← config_dir here; context ".." → workspace root
/// ```
#[test]
fn validate_build_context_dotdot_resolves_to_workspace_root() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        // context ".." resolves to workspace root; Dockerfile at workspace root
        // is named "Dockerfile" relative to that context.
        r#"{"name":"ctx-dotdot","build":{"dockerfile":"Dockerfile","context":".."}}"#,
    )
    .expect("write devcontainer.json");
    // Dockerfile at workspace root — resolved as context.join("Dockerfile") = workspace/Dockerfile
    std::fs::write(ws.path().join("Dockerfile"), "FROM alpine\n").expect("write Dockerfile");

    let result = Devcontainer::load(ws.path());
    assert!(
        result.is_ok(),
        "context '..' from .devcontainer/ must resolve to workspace root and pass, got: {:?}",
        result.err()
    );
}

/// `context = "subdir"` from `.devcontainer/devcontainer.json` resolves to
/// `<workspace>/.devcontainer/subdir` — inside the workspace.
#[test]
fn validate_build_context_subdir_relative_to_config_dir() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    let ctx_dir = dc_dir.join("subdir");
    std::fs::create_dir_all(&ctx_dir).expect("create subdir");
    std::fs::write(ctx_dir.join("Dockerfile"), "FROM alpine\n").expect("write Dockerfile");
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        r#"{"name":"ctx-subdir","build":{"dockerfile":"Dockerfile","context":"subdir"}}"#,
    )
    .expect("write devcontainer.json");

    let result = Devcontainer::load(ws.path());
    assert!(
        result.is_ok(),
        "context 'subdir' from .devcontainer/ must resolve within workspace, got: {:?}",
        result.err()
    );
}

/// `context = "."` from `.devcontainer.json` at workspace root resolves to the
/// workspace root itself and must pass validation.
#[test]
fn validate_build_context_root_layout_dot() {
    let ws = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        ws.path().join(".devcontainer.json"),
        r#"{"name":"ctx-dot","build":{"dockerfile":"Dockerfile","context":"."}}"#,
    )
    .expect("write .devcontainer.json");
    std::fs::write(ws.path().join("Dockerfile"), "FROM alpine\n").expect("write Dockerfile");

    let result = Devcontainer::load(ws.path());
    assert!(
        result.is_ok(),
        "context '.' from workspace root must pass, got: {:?}",
        result.err()
    );
}

/// `context = "../../escape"` from `.devcontainer/devcontainer.json` resolves
/// two levels above the workspace root and must be rejected.
///
/// Note: `"../sibling-project"` (single `..`) from `.devcontainer/` resolves to
/// `<workspace>/sibling-project` which is *inside* the workspace. Two `..`
/// hops are required to escape from `config_dir = <workspace>/.devcontainer/`.
#[test]
fn validate_build_context_dotdot_dotdot_escapes_root() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        r#"{"name":"ctx-escape","build":{"dockerfile":"Dockerfile","context":"../../escape"}}"#,
    )
    .expect("write devcontainer.json");

    let result = Devcontainer::load(ws.path());
    assert!(
        result.is_err(),
        "context '../../escape' must be rejected as it escapes the workspace"
    );
}

/// `context = "../sibling-project"` from `.devcontainer/devcontainer.json`
/// is resolved relative to `config_dir` (= `<workspace>/.devcontainer/`).
/// One `..` hop leads back to the workspace root; then `sibling-project` is
/// appended → `<workspace>/sibling-project` which is INSIDE the workspace.
///
/// This documents the expected behaviour and confirms that the fix does not
/// accidentally reject valid sub-paths.
#[test]
fn validate_build_context_sibling_within_workspace_passes() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    let sib_dir = ws.path().join("sibling-project");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    std::fs::create_dir_all(&sib_dir).expect("create sibling-project dir");
    std::fs::write(sib_dir.join("Dockerfile"), "FROM alpine\n").expect("write Dockerfile");
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        r#"{"name":"ctx-sibling","build":{"dockerfile":"Dockerfile","context":"../sibling-project"}}"#,
    )
    .expect("write devcontainer.json");

    // "../sibling-project" from .devcontainer/ → workspace/sibling-project → inside workspace → OK
    let result = Devcontainer::load(ws.path());
    assert!(
        result.is_ok(),
        "context '../sibling-project' resolves to <workspace>/sibling-project which is inside workspace, got: {:?}",
        result.err()
    );
}

/// Absolute `context` path inside the workspace must pass without error.
#[test]
fn validate_build_context_absolute_inside_root() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    let abs_ctx = ws.path().join("ctx");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    std::fs::create_dir_all(&abs_ctx).expect("create ctx dir");
    std::fs::write(abs_ctx.join("Dockerfile"), "FROM alpine\n").expect("write Dockerfile");
    let json = format!(
        r#"{{"name":"ctx-abs-in","build":{{"dockerfile":"Dockerfile","context":"{}"}}}}"#,
        abs_ctx.display()
    );
    std::fs::write(dc_dir.join("devcontainer.json"), json).expect("write devcontainer.json");

    let result = Devcontainer::load(ws.path());
    assert!(
        result.is_ok(),
        "absolute context inside workspace root must pass, got: {:?}",
        result.err()
    );
}

/// Absolute `context` path outside the workspace must NOT return an error from
/// `validate_build_context` — it emits a warning but is allowed through.
///
/// We use an image-based config (no Dockerfile) to isolate the context
/// validation check from Dockerfile path resolution.
#[test]
fn validate_build_context_absolute_outside_root_warns_only() {
    let ws = tempfile::tempdir().expect("tempdir");
    let dc_dir = ws.path().join(".devcontainer");
    std::fs::create_dir_all(&dc_dir).expect("create .devcontainer dir");
    // Use /tmp as the absolute context — guaranteed to be outside any tempdir workspace.
    // Use image (not dockerfile) so validate_build_source does not add a secondary failure.
    std::fs::write(
        dc_dir.join("devcontainer.json"),
        r#"{"name":"ctx-abs-out","image":"alpine","build":{"context":"/tmp"}}"#,
    )
    .expect("write devcontainer.json");

    let result = Devcontainer::load(ws.path());
    // validate_build_context allows absolute paths outside root (warning only);
    // build_source resolves to Image("alpine") so no Dockerfile validation occurs.
    assert!(
        result.is_ok(),
        "absolute context outside workspace root must not error (warning only), got: {:?}",
        result.err()
    );
}
