use crate::error::{Error, Result};
use serde::Deserialize;
//
/// Container engine provider selection.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")] // match TOML values
pub enum Provider {
    // Exhaustive: Docker | Podman | Apple | Nerdctl
    /// Use Docker (default).
    #[default]
    Docker, // most common runtime
    /// Use Podman.
    Podman, // rootless alternative
    /// Use Apple Container (macOS only).
    Apple,
    /// Use nerdctl (containerd).
    Nerdctl,
} // end Provider
//
/// User settings loaded from `~/.config/devcont/config.toml`.
#[derive(Default, Debug, Deserialize)] // TOML deserialization
pub struct Settings {
    // top-level TOML structure
    /// Dotfiles to copy into the container (relative paths from `~`).
    #[serde(default)]
    pub dotfiles: Vec<String>, // e.g. [".bashrc", ".vimrc"]
    /// Container engine to use.
    #[serde(default)]
    pub provider: Provider, // defaults to Docker
} // end Settings
//
impl Settings {
    // single public constructor: `load()`
    /// Load settings from the user config file.
    ///
    /// Returns `Ok(default)` when the settings file does not exist.
    /// Returns `Err` when the file exists but cannot be read or parsed.
    ///
    /// # Errors
    /// Returns [`Error::Io`] if the settings file exists but cannot be read.
    /// Returns [`Error::SettingsLoad`] if the settings file exists but cannot be parsed.
    pub fn load() -> Result<Self> {
        let Some(dirs) = directories::ProjectDirs::from("com", "Big Refactor", "devcont") else {
            return Ok(Self::default());
        };
        //
        let config_path = dirs.config_dir().join("config.toml");
        //
        if !config_path.is_file() {
            return Ok(Self::default());
        }
        //
        let raw = std::fs::read_to_string(&config_path).map_err(|io_err| {
            Error::Io(std::io::Error::new(
                io_err.kind(),
                format!(
                    "could not read settings file {}: {io_err}",
                    config_path.display()
                ),
            ))
        })?;
        //
        let parsed: Self = toml::from_str(&raw).map_err(|parse_err| {
            Error::SettingsLoad(format!(
                "could not parse settings file {}: {parse_err}",
                config_path.display()
            ))
        })?;
        //
        Ok(parsed)
    }
} // end impl Settings

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_returns_default_when_no_config_file() {
        // This test relies on the fact that in CI/test environments there is
        // no devcont config file present.  Settings::load() must not panic.
        let settings = Settings::load().expect("load should succeed with no config file");
        // Default provider is Docker
        assert!(matches!(settings.provider, Provider::Docker));
        assert!(settings.dotfiles.is_empty());
    }

    #[test]
    fn invalid_toml_fails_to_parse() {
        // Verify that malformed TOML yields a parse error (unit-tests the parsing layer
        // without needing a real settings file on disk).
        let contents = "not valid toml {{{";
        let result: std::result::Result<Settings, _> = toml::from_str(contents);
        assert!(result.is_err(), "invalid TOML should fail to parse");
    }

    #[test]
    fn provider_docker_is_default() {
        let s: Settings = toml::from_str("").expect("empty TOML should parse");
        assert!(matches!(s.provider, Provider::Docker));
    }

    #[test]
    fn provider_podman_parses() {
        let s: Settings =
            toml::from_str("provider = \"podman\"").expect("podman provider should parse");
        assert!(matches!(s.provider, Provider::Podman));
    }

    #[test]
    fn provider_apple_parses() {
        let s: Settings =
            toml::from_str("provider = \"apple\"").expect("apple provider should parse");
        assert!(matches!(s.provider, Provider::Apple));
    }

    #[test]
    fn provider_nerdctl_parses() {
        let s: Settings =
            toml::from_str("provider = \"nerdctl\"").expect("nerdctl provider should parse");
        assert!(matches!(s.provider, Provider::Nerdctl));
    }

    #[test]
    fn unknown_provider_value_fails_to_parse() {
        let result: std::result::Result<Settings, _> = toml::from_str("provider = \"nspawn\"");
        assert!(
            result.is_err(),
            "unknown provider value should fail to parse"
        );
    }

    #[test]
    fn dotfiles_parsed_as_vec() {
        let s: Settings =
            toml::from_str("dotfiles = [\".bashrc\", \".vimrc\"]").expect("dotfiles should parse");
        assert_eq!(s.dotfiles, vec![".bashrc", ".vimrc"]);
    }

    #[test]
    fn dotfiles_absent_defaults_to_empty() {
        let s: Settings = toml::from_str("").expect("empty TOML should parse");
        assert!(s.dotfiles.is_empty());
    }
} // end tests
