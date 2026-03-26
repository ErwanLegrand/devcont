use serde::Deserialize;
use std::{collections::HashMap, path::Path}; // JSON5 deserialization
//
use crate::{
    devcontainers::one_or_many::OneOrMany,
    error::{Error, Result},
};
//
/// Behaviour when the dev-container session terminates.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
enum TerminationBehaviour {
    None, // keep container running
    #[default]
    StopContainer, // stop single container
    StopCompose, // stop compose project
} // end TerminationBehaviour
//
/// Parsed representation of a `devcontainer.json` configuration file.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")] // match JSON field names
pub struct Config {
    // mirrors the devcontainer.json schema
    /// Project name — used to derive the container name via [`Config::safe_name`].
    name: String, // required field
    /// Pre-built image to pull instead of building from a Dockerfile.
    pub image: Option<String>,
    /// Build configuration (`dockerfile`, `context`, `args`).
    pub build: Option<Build>, // mutually exclusive with `image` in practice
    /// Host ports to forward into the container.
    #[serde(default)] // empty vec when absent
    pub forward_ports: Vec<u16>, // e.g. [3000, 8080]
    /// Host-side hook run before the container is created.
    pub initialize_command: Option<OneOrMany>,
    /// In-container hook run once when the container is first created.
    pub on_create_command: Option<OneOrMany>,
    /// In-container hook run when content changes (e.g., re-clone).
    pub update_content_command: Option<OneOrMany>,
    /// In-container hook run after the container is created.
    pub post_create_command: Option<OneOrMany>,
    /// In-container hook run every time the container starts.
    pub post_start_command: Option<OneOrMany>,
    /// In-container hook run after attaching to the container.
    pub post_attach_command: Option<OneOrMany>,
    /// User to run commands as inside the container. Defaults to `"root"`.
    #[serde(default = "fallback_remote_user")]
    pub remote_user: String, // defaults to "root"
    /// Extra arguments forwarded to `docker run` / `podman run`.
    #[serde(default)] // empty vec when absent
    pub run_args: Vec<String>, // e.g. ["--network", "host"]
    /// When `true`, overrides the container's default command with the shell.
    #[serde(default)] // false when absent
    pub override_command: bool,
    /// Volume mount definitions for the container.
    pub mounts: Option<Vec<HashMap<String, String>>>,
    /// Environment variables injected into the container.
    #[serde(default)] // empty map when absent
    pub remote_env: HashMap<String, String>, // e.g. {"FOO": "bar"}
    /// Path to the `docker-compose.yml` file (enables Compose mode).
    pub docker_compose_file: Option<String>, // triggers compose workflow
    /// Compose service name (required when `dockerComposeFile` is set).
    pub service: Option<String>, // only meaningful in compose mode
    /// Override `SELinux` auto-detection for SSH socket relabelling (`:z`).
    /// When absent, `SELinux` enforcing mode is detected at runtime.
    pub selinux_relabel: Option<bool>,
    /// Working directory inside the container. Defaults to `"/workspace"`.
    #[serde(default = "fallback_workspace_folder")]
    pub workspace_folder: String, // defaults to "/workspace"
    /// Maximum number of seconds any single lifecycle hook may run.
    ///
    /// When absent (the default), hooks run without a timeout.
    /// Overridable at runtime with `--hook-timeout`.
    pub hook_timeout_seconds: Option<u32>,
    /// Container shutdown behaviour when the session ends.
    #[serde(default)] // defaults to StopContainer
    shutdown_action: TerminationBehaviour,
} // end Config
//
/// Build configuration block within `devcontainer.json`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")] // match JSON field names
pub struct Build {
    // nested object inside devcontainer.json
    /// Path to the Dockerfile relative to `context` (or the workspace root if `context` is absent).
    pub dockerfile: Option<String>, // e.g. "Dockerfile.dev"
    /// Directory sent to the daemon as the build context. Defaults to the workspace root.
    pub context: Option<String>,
    /// Build-time arguments passed as `--build-arg KEY=VALUE`.
    #[serde(default)] // empty map when absent
    pub args: HashMap<String, String>, // e.g. {"VER": "1.2"}
} // end Build
//
impl Config {
    // public API for devcontainer configuration
    /// Parse a `devcontainer.json` (or JSON5) file from `file`.
    ///
    /// # Errors
    /// Returns [`Error::Io`] if the file cannot be read, or [`Error::ConfigParse`]
    /// if the content is not valid JSON5 or does not match the expected schema.
    pub fn parse(file: &Path) -> Result<Config> {
        let raw = std::fs::read_to_string(file).map_err(Error::Io)?;
        Self::parse_str(&raw)
    }
    //
    /// Parse a `devcontainer.json` configuration from a raw JSON5 string.
    ///
    /// # Errors
    /// Returns [`Error::ConfigParse`] if the content is not valid JSON5 or does
    /// not match the expected schema.
    ///
    /// # Examples
    /// ```
    /// use devcont::devcontainers::config::Config;
    /// let c = Config::parse_str(r#"{"name":"my-app","image":"ubuntu:24.04"}"#).unwrap();
    /// ```
    pub fn parse_str(input: &str) -> Result<Config> {
        let config: Config =
            json_five::from_str(input).map_err(|err| Error::ConfigParse(err.to_string()))?;
        config.validate()?;
        Ok(config)
    } // end parse_str
    //
    /// Validate cross-field constraints that cannot be expressed via serde alone.
    fn validate(&self) -> Result<()> {
        if self.docker_compose_file.is_some() && self.service.is_none() {
            return Err(Error::InvalidConfig(
                "dockerComposeFile requires a 'service' field".to_string(),
            ));
        }
        if self.hook_timeout_seconds == Some(0) {
            return Err(Error::InvalidConfig(
                "hookTimeoutSeconds must be at least 1 (got 0)".to_string(),
            ));
        }
        Ok(())
    }
    //
    /// Return the Dockerfile path from the `build` block, if present.
    #[must_use]
    pub fn dockerfile(&self) -> Option<String> {
        // delegates to Build
        self.build.as_ref().and_then(|cfg| cfg.dockerfile.clone())
    } // end dockerfile
    //
    /// Return the build arguments from the `build` block, or an empty map.
    #[must_use]
    pub fn build_args(&self) -> HashMap<String, String> {
        // delegates to Build
        match self.build.as_ref() {
            Some(cfg) => cfg.args.clone(),
            Option::None => HashMap::new(),
        }
    } // end build_args
    //
    /// Return a container-safe name with the `devcont-` prefix.
    ///
    /// Lowercases the project name, replaces spaces with dashes, and strips
    /// characters that are not alphanumeric, `-`, `_`, or `.`.
    ///
    /// # Errors
    /// Returns [`Error::InvalidConfig`] when the project name produces an empty
    /// string after stripping — this happens for all-Unicode names that have no
    /// ASCII representation.
    pub fn safe_name(&self) -> Result<String> {
        let lowered = self.name.to_ascii_lowercase();
        let with_dashes = lowered.replace(' ', "-");
        let sanitized: String = with_dashes
            .chars()
            .filter(|ch| matches!(ch, 'a'..='z' | '0'..='9' | '-' | '_' | '.'))
            .collect();
        //
        if sanitized.is_empty() {
            return Err(Error::InvalidConfig(format!(
                "Cannot derive a container name from project name '{}'. \
                 Rename the project to use ASCII characters.",
                self.name
            )));
        } // end empty check
        //
        // Docker requires names to start with a letter or digit.
        let label = if sanitized.starts_with(|ch: char| ch.is_ascii_alphanumeric()) {
            sanitized
        } else {
            tracing::info!(
                "container name derived from '{}' starts with a non-alphanumeric \
                 character; prepending 'dev-'",
                self.name
            );
            format!("dev-{sanitized}")
        };
        //
        Ok(format!("devcont-{label}"))
    } // end safe_name
    //
    /// Return `true` if the container should be stopped after the session ends.
    ///
    /// Note: `StopCompose` and `StopContainer` are currently treated identically —
    /// both stop the container/project on exit. The distinction (stop only the
    /// service vs. full `compose down`) is not yet implemented; tracked in
    /// `conductor/tracks/code_inventory_20260228/findings.md` (Item 4, Retain).
    #[must_use]
    pub fn should_shutdown(&self) -> bool {
        // checks TerminationBehaviour
        match self.shutdown_action {
            TerminationBehaviour::None => false,
            TerminationBehaviour::StopContainer | TerminationBehaviour::StopCompose => true,
        }
    } // end should_shutdown
    //
    /// Return `true` if this config uses Docker Compose (i.e., `dockerComposeFile` is set).
    #[must_use]
    pub fn is_compose(&self) -> bool {
        // checks docker_compose_file presence
        self.docker_compose_file.is_some() // None means single-container mode
    } // end is_compose
} // end impl Config
//
fn fallback_remote_user() -> String {
    // serde default for remote_user
    "root".to_owned()
} // end fallback_remote_user
//
fn fallback_workspace_folder() -> String {
    // serde default for workspace_folder
    "/workspace".to_owned()
} // end fallback_workspace_folder

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn safe_name_uses_devcont_prefix() {
        let fixture = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/devcontainer.json"
        ));
        let config = Config::parse(fixture).expect("fixture should parse");
        let name = config.safe_name().expect("safe_name should succeed");
        assert!(
            name.starts_with("devcont-"),
            "safe_name() should start with 'devcont-', got '{name}'"
        );
        assert_eq!(name, "devcont-test-project");
    }

    #[test]
    fn parse_missing_file_returns_err_not_panic() {
        let missing = Path::new("/tmp/nonexistent_devcontainer_fixture.json");
        let result = Config::parse(missing);
        assert!(
            result.is_err(),
            "Config::parse() on a missing file must return Err"
        );
    }

    #[test]
    fn parse_invalid_json5_returns_err_not_panic() {
        let invalid = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/devcontainer_invalid.json"
        ));
        let result = Config::parse(invalid);
        assert!(
            result.is_err(),
            "Config::parse() on invalid JSON5 must return Err"
        );
        // Ensure it's a ConfigParse variant, not an Io error
        assert!(matches!(result.unwrap_err(), Error::ConfigParse(_)));
    }

    #[test]
    fn parse_image_field() {
        let fixture = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/devcontainer_image.json"
        ));
        let config = Config::parse(fixture).expect("image fixture should parse");
        assert_eq!(
            config.image,
            Some("alpine".to_string()),
            "image field should be 'alpine'"
        );
        assert!(config.build.is_none(), "build should be absent");
    }

    #[test]
    fn safe_name_replaces_spaces_with_dashes() {
        let minimal = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/devcontainer_minimal.json"
        ));
        let config = Config::parse(minimal).expect("minimal fixture should parse");
        let name = config.safe_name().expect("safe_name should succeed");
        assert_eq!(name, "devcont-minimal");
        assert!(!name.contains(' '), "safe_name must not contain spaces");
    }

    #[test]
    fn safe_name_strips_unicode_and_returns_error_when_empty() {
        let config = Config {
            name: "\u{4e2d}\u{6587}".to_string(), // "中文" (Chinese)
            image: None,
            build: None,
            forward_ports: vec![],
            initialize_command: None,
            on_create_command: None,
            update_content_command: None,
            post_create_command: None,
            post_start_command: None,
            post_attach_command: None,
            remote_user: "root".to_string(),
            run_args: vec![],
            override_command: false,
            mounts: None,
            remote_env: std::collections::HashMap::new(),
            docker_compose_file: None,
            service: None,
            selinux_relabel: None,
            workspace_folder: "/workspace".to_string(),
            hook_timeout_seconds: None,
            shutdown_action: TerminationBehaviour::default(),
        };
        let result = config.safe_name();
        assert!(result.is_err(), "all-unicode name should produce an error");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("ASCII"), "error message should mention ASCII");
    }

    // --- Field deserialization tests ---

    fn parse_inline(json: &str) -> Config {
        json_five::from_str(json).expect("inline fixture should parse")
    }

    #[test]
    fn forward_ports_parsed_as_vec() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine", "forwardPorts": [3000, 8080] }"#);
        assert_eq!(c.forward_ports, vec![3000u16, 8080]);
    }

    #[test]
    fn forward_ports_absent_defaults_to_empty() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine" }"#);
        assert!(c.forward_ports.is_empty());
    }

    #[test]
    fn remote_user_parsed_explicitly() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine", "remoteUser": "vscode" }"#);
        assert_eq!(c.remote_user, "vscode");
    }

    #[test]
    fn remote_user_defaults_to_root() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine" }"#);
        assert_eq!(c.remote_user, "root");
    }

    #[test]
    fn run_args_parsed_as_vec() {
        let c =
            parse_inline(r#"{ "name": "t", "image": "alpine", "runArgs": ["--network", "host"] }"#);
        assert_eq!(c.run_args, vec!["--network", "host"]);
    }

    #[test]
    fn run_args_absent_defaults_to_empty() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine" }"#);
        assert!(c.run_args.is_empty());
    }

    #[test]
    fn override_command_parsed_true() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine", "overrideCommand": true }"#);
        assert!(c.override_command);
    }

    #[test]
    fn override_command_defaults_to_false() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine" }"#);
        assert!(!c.override_command);
    }

    #[test]
    fn remote_env_parsed_as_map() {
        let c = parse_inline(
            r#"{ "name": "t", "image": "alpine", "remoteEnv": { "FOO": "bar", "BAZ": "1" } }"#,
        );
        assert_eq!(c.remote_env.get("FOO").map(String::as_str), Some("bar"));
        assert_eq!(c.remote_env.get("BAZ").map(String::as_str), Some("1"));
    }

    #[test]
    fn remote_env_absent_defaults_to_empty() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine" }"#);
        assert!(c.remote_env.is_empty());
    }

    #[test]
    fn selinux_relabel_parsed_true() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine", "selinuxRelabel": true }"#);
        assert_eq!(c.selinux_relabel, Some(true));
    }

    #[test]
    fn selinux_relabel_parsed_false() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine", "selinuxRelabel": false }"#);
        assert_eq!(c.selinux_relabel, Some(false));
    }

    #[test]
    fn selinux_relabel_absent_is_none() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine" }"#);
        assert!(c.selinux_relabel.is_none());
    }

    #[test]
    fn workspace_folder_parsed() {
        let c = parse_inline(
            r#"{ "name": "t", "image": "alpine", "workspaceFolder": "/home/user/app" }"#,
        );
        assert_eq!(c.workspace_folder, "/home/user/app");
    }

    #[test]
    fn workspace_folder_defaults_to_workspace() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine" }"#);
        assert_eq!(c.workspace_folder, "/workspace");
    }

    #[test]
    fn shutdown_action_none_means_no_shutdown() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine", "shutdownAction": "none" }"#);
        assert!(!c.should_shutdown());
    }

    #[test]
    fn shutdown_action_stop_container_means_shutdown() {
        let c = parse_inline(
            r#"{ "name": "t", "image": "alpine", "shutdownAction": "stopContainer" }"#,
        );
        assert!(c.should_shutdown());
    }

    #[test]
    fn shutdown_action_absent_defaults_to_stop_container() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine" }"#);
        assert!(c.should_shutdown());
    }

    #[test]
    fn initialize_command_as_string() {
        let c =
            parse_inline(r#"{ "name": "t", "image": "alpine", "initializeCommand": "init.sh" }"#);
        assert!(c.initialize_command.is_some());
    }

    #[test]
    fn initialize_command_as_array() {
        let c = parse_inline(
            r#"{ "name": "t", "image": "alpine", "initializeCommand": ["bash", "init.sh"] }"#,
        );
        assert!(matches!(
            c.initialize_command,
            Some(crate::devcontainers::one_or_many::OneOrMany::Many(_))
        ));
    }

    #[test]
    fn on_create_command_as_string() {
        let c =
            parse_inline(r#"{ "name": "t", "image": "alpine", "onCreateCommand": "create.sh" }"#);
        assert!(c.on_create_command.is_some());
    }

    #[test]
    fn update_content_command_as_array() {
        let c = parse_inline(
            r#"{ "name": "t", "image": "alpine", "updateContentCommand": ["npm", "install"] }"#,
        );
        assert!(c.update_content_command.is_some());
    }

    #[test]
    fn post_create_command_as_string() {
        let c =
            parse_inline(r#"{ "name": "t", "image": "alpine", "postCreateCommand": "post.sh" }"#);
        assert!(c.post_create_command.is_some());
    }

    #[test]
    fn post_start_command_parsed() {
        let c =
            parse_inline(r#"{ "name": "t", "image": "alpine", "postStartCommand": "start.sh" }"#);
        assert!(c.post_start_command.is_some());
    }

    #[test]
    fn post_attach_command_parsed() {
        let c =
            parse_inline(r#"{ "name": "t", "image": "alpine", "postAttachCommand": "attach.sh" }"#);
        assert!(c.post_attach_command.is_some());
    }

    #[test]
    fn build_args_parsed_from_build_block() {
        let c = parse_inline(
            r#"{ "name": "t", "build": { "dockerfile": "Dockerfile", "args": { "VER": "1.2" } } }"#,
        );
        assert_eq!(c.build_args().get("VER").map(String::as_str), Some("1.2"));
    }

    #[test]
    fn build_args_absent_returns_empty_map() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine" }"#);
        assert!(c.build_args().is_empty());
    }

    #[test]
    fn is_compose_true_when_docker_compose_file_set() {
        let c = parse_inline(
            r#"{ "name": "t", "dockerComposeFile": "docker-compose.yml", "service": "app" }"#,
        );
        assert!(c.is_compose());
    }

    #[test]
    fn is_compose_false_without_docker_compose_file() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine" }"#);
        assert!(!c.is_compose());
    }

    #[test]
    fn dockerfile_method_returns_none_for_image_config() {
        let c = parse_inline(r#"{ "name": "t", "image": "alpine" }"#);
        assert!(c.dockerfile().is_none());
    }

    #[test]
    fn dockerfile_method_returns_path_from_build_block() {
        let c = parse_inline(r#"{ "name": "t", "build": { "dockerfile": "Dockerfile" } }"#);
        assert_eq!(c.dockerfile(), Some("Dockerfile".to_string()));
    }

    #[test]
    fn safe_name_digit_start_is_valid_without_dev_prefix() {
        let config = Config {
            name: "1myproject".to_string(),
            image: None,
            build: None,
            forward_ports: vec![],
            initialize_command: None,
            on_create_command: None,
            update_content_command: None,
            post_create_command: None,
            post_start_command: None,
            post_attach_command: None,
            remote_user: "root".to_string(),
            run_args: vec![],
            override_command: false,
            mounts: None,
            remote_env: std::collections::HashMap::new(),
            docker_compose_file: None,
            service: None,
            selinux_relabel: None,
            workspace_folder: "/workspace".to_string(),
            hook_timeout_seconds: None,
            shutdown_action: TerminationBehaviour::default(),
        };
        let name = config.safe_name().expect("digit-start should succeed");
        assert!(
            name.starts_with("devcont-1"),
            "digit-start name should not have 'dev-' inserted, got: {name}"
        );
    }

    // --- Config::parse error paths ---

    #[test]
    fn config_parse_nonexistent_file_returns_io_error() {
        use crate::error::Error;
        let result = Config::parse(Path::new("/nonexistent/devcont/devcontainer.json"));
        assert!(result.is_err(), "nonexistent file should return Err");
        assert!(
            matches!(result.unwrap_err(), Error::Io(_)),
            "nonexistent file error should be Error::Io"
        );
    }

    #[test]
    fn config_parse_invalid_json5_returns_config_parse_error() {
        use crate::error::Error;
        let dir = std::env::temp_dir();
        let path = dir.join("devcont_test_invalid.json");
        std::fs::write(&path, "{ not : valid : json }").expect("write test file");
        let result = Config::parse(&path);
        let _ = std::fs::remove_file(&path);
        assert!(result.is_err(), "invalid JSON5 should return Err");
        assert!(
            matches!(result.unwrap_err(), Error::ConfigParse(_)),
            "invalid JSON5 error should be Error::ConfigParse"
        );
    }

    #[test]
    fn config_parse_missing_required_name_returns_error() {
        let dir = std::env::temp_dir();
        let path = dir.join("devcont_test_noname.json");
        std::fs::write(&path, r#"{ "image": "alpine" }"#).expect("write test file");
        let result = Config::parse(&path);
        let _ = std::fs::remove_file(&path);
        assert!(
            result.is_err(),
            "config without 'name' field should return Err"
        );
    }

    #[test]
    fn config_parse_unknown_fields_ignored() {
        let dir = std::env::temp_dir();
        let path = dir.join("devcont_test_unknown.json");
        std::fs::write(
            &path,
            r#"{ "name": "myproject", "image": "alpine", "unknownField": true }"#,
        )
        .expect("write test file");
        let result = Config::parse(&path);
        let _ = std::fs::remove_file(&path);
        assert!(result.is_ok(), "unknown fields should be silently ignored");
    }

    #[test]
    fn safe_name_prepends_dev_when_starts_with_dash() {
        let config = Config {
            name: "-myproject".to_string(),
            image: None,
            build: None,
            forward_ports: vec![],
            initialize_command: None,
            on_create_command: None,
            update_content_command: None,
            post_create_command: None,
            post_start_command: None,
            post_attach_command: None,
            remote_user: "root".to_string(),
            run_args: vec![],
            override_command: false,
            mounts: None,
            remote_env: std::collections::HashMap::new(),
            docker_compose_file: None,
            service: None,
            selinux_relabel: None,
            workspace_folder: "/workspace".to_string(),
            hook_timeout_seconds: None,
            shutdown_action: TerminationBehaviour::default(),
        };
        let name = config.safe_name().expect("should succeed");
        assert!(
            name.starts_with("devcont-dev-"),
            "name starting with '-' should get 'dev-' prefix, got: {name}"
        );
    }

    // --- cross-field validation ---

    #[test]
    fn compose_without_service_rejected_at_parse_time() {
        let result =
            Config::parse_str(r#"{ "name": "t", "dockerComposeFile": "docker-compose.yml" }"#);
        assert!(
            result.is_err(),
            "compose without service should be rejected"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("service"),
            "error should mention 'service', got: {msg}"
        );
    }

    #[test]
    fn compose_with_service_accepted() {
        let result = Config::parse_str(
            r#"{ "name": "t", "dockerComposeFile": "docker-compose.yml", "service": "app" }"#,
        );
        assert!(result.is_ok(), "compose with service should be accepted");
    }

    #[test]
    fn hook_timeout_zero_rejected_at_parse_time() {
        let result =
            Config::parse_str(r#"{ "name": "t", "image": "alpine", "hookTimeoutSeconds": 0 }"#);
        assert!(result.is_err(), "timeout=0 should be rejected");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("at least 1"),
            "error should mention 'at least 1', got: {msg}"
        );
    }

    #[test]
    fn hook_timeout_positive_accepted() {
        let result =
            Config::parse_str(r#"{ "name": "t", "image": "alpine", "hookTimeoutSeconds": 30 }"#);
        assert!(result.is_ok(), "timeout=30 should be accepted");
    }
}
