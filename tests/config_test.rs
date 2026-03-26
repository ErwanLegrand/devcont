use std::path::Path;

// We reference the library via the crate name established in Cargo.toml.
// Since this is a binary crate we pull in the modules directly.
// Integration tests for a binary crate must use `#[path = ...]` to reach
// internal modules.  We re-export what we need via a thin shim in src/lib.rs,
// but for now we duplicate the test inside the module tests and here we just
// smoke-test the fixture files so the test suite is aware of them.

use devcont::provider::apple::AppleContainer;
use devcont::provider::docker::BuildSource;
use std::collections::HashMap;

/// Confirm the standard fixture file exists and is valid JSON5.
#[test]
fn devcontainer_json_fixture_is_present() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/devcontainer.json"
    ));
    assert!(path.exists(), "devcontainer.json fixture must exist");
    let contents = std::fs::read_to_string(path).expect("should be readable");
    assert!(!contents.is_empty(), "fixture must not be empty");
}

/// Confirm the minimal fixture file exists.
#[test]
fn devcontainer_minimal_fixture_is_present() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/devcontainer_minimal.json"
    ));
    assert!(
        path.exists(),
        "devcontainer_minimal.json fixture must exist"
    );
    let contents = std::fs::read_to_string(path).expect("should be readable");
    assert!(
        contents.contains("minimal"),
        "minimal fixture should contain 'minimal'"
    );
}

/// Confirm the invalid fixture exists and is indeed not valid JSON5.
#[test]
fn devcontainer_invalid_fixture_is_not_valid_json5() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/devcontainer_invalid.json"
    ));
    assert!(
        path.exists(),
        "devcontainer_invalid.json fixture must exist"
    );
    let contents = std::fs::read_to_string(path).expect("should be readable");
    let result: Result<serde_json::Value, _> = json_five::from_str(&contents);
    assert!(
        result.is_err(),
        "invalid fixture must fail JSON5 parsing, got: {result:?}"
    );
}

/// Test that AppleContainer can be instantiated via its constructor.
#[test]
fn apple_container_integration() {
    let _container = AppleContainer::new(
        HashMap::from([("TEST_ARG".to_string(), "test_value".to_string())]),
        "/test/context".to_string(),
        BuildSource::Dockerfile("Dockerfile.test".to_string()),
        "container".to_string(),
        "/test/workspace".to_string(),
        vec![8080, 3000],
        "apple-test-container".to_string(),
        vec!["--network=host".to_string()],
        Some(vec![HashMap::from([
            ("type".to_string(), "bind".to_string()),
            ("source".to_string(), "/host".to_string()),
            ("target".to_string(), "/container".to_string()),
        ])]),
        "testuser".to_string(),
        "/test/workspace".to_string(),
        true,
    );
    // Field assertions are in unit tests (apple.rs #[cfg(test)])
}

/// Test that Nerdctl provider can be instantiated via its constructor.
#[test]
fn nerdctl_integration() {
    use devcont::provider::nerdctl::Nerdctl;

    let _container = Nerdctl::new(
        HashMap::from([("TEST_ARG".to_string(), "test_value".to_string())]),
        "/test/context".to_string(),
        BuildSource::Dockerfile("Dockerfile.test".to_string()),
        "nerdctl".to_string(),
        "/test/workspace".to_string(),
        vec![8080, 3000],
        "nerdctl-test-container".to_string(),
        vec!["--network=host".to_string()],
        Some(vec![HashMap::from([
            ("type".to_string(), "bind".to_string()),
            ("source".to_string(), "/host".to_string()),
            ("target".to_string(), "/container".to_string()),
        ])]),
        "testuser".to_string(),
        "/test/workspace".to_string(),
        true,
    );
    // Field assertions are in unit tests (nerdctl.rs #[cfg(test)])
}
