/// Typed options passed to [`super::Provider::create`].
///
/// Carries the per-container configuration that cannot be encoded on the provider
/// struct itself (environment variables originating from `remoteEnv`).
/// All other container settings (workspace mount, `run_args`, mounts, ports, …)
/// are stored directly on the provider and applied there.
#[derive(Debug, Clone)]
pub struct ContainerOptions {
    /// Environment variables to inject into the container, sorted by key.
    ///
    /// Derived from the `remoteEnv` map in `devcontainer.json`.
    pub remote_env: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_options_stores_env_vars() {
        let opts = ContainerOptions {
            remote_env: vec![
                ("FOO".to_string(), "bar".to_string()),
                ("KEY".to_string(), "value".to_string()),
            ],
        };
        assert_eq!(opts.remote_env.len(), 2);
        assert_eq!(opts.remote_env[0], ("FOO".to_string(), "bar".to_string()));
        assert_eq!(opts.remote_env[1], ("KEY".to_string(), "value".to_string()));
    }

    #[test]
    fn container_options_empty_env_vars() {
        let opts = ContainerOptions { remote_env: vec![] };
        assert!(opts.remote_env.is_empty());
    }

    #[test]
    fn container_options_clone() {
        let opts = ContainerOptions {
            remote_env: vec![("A".to_string(), "1".to_string())],
        };
        let cloned = opts.clone();
        assert_eq!(cloned.remote_env, opts.remote_env);
    }
}
