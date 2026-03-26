/// Safe flag prefixes that are allowed in `runArgs`.
pub(crate) const ALLOWED_FLAGS: &[&str] = &[
    "--env",
    "--env-file",
    "--label",
    "--name",
    "--hostname",
    "--network",
    "--expose",
    "--publish",
    "--dns",
    "--add-host",
    "--workdir",
    "--user",
    "--memory",
    "--cpus",
    "--memory-swap",
    "--cpu-shares",
    "--shm-size",
    "--ulimit",
    "--tmpfs",
    "--read-only",
    "--init",
    "-e",
    "-p",
    "-u",
    "-w",
    "-h",
];

/// Privilege-escalating flags that are denied in `runArgs`.
pub(crate) const DENIED_FLAGS: &[&str] = &[
    "--privileged",
    "--cap-add",
    "--cap-drop",
    "--security-opt",
    "--device",
    "--pid",
    "--ipc",
    "--userns",
    "--cgroupns",
    "--sysctl",
    "--apparmor",
    "--volume",
    "--mount",
    "-v",
];

/// Validate a list of `runArgs` against the allowlist and denylist.
///
/// - Denied flags → returns `Err` with the offending flag name.
/// - Unknown flags (not in either list) → prints a warning to stderr, does not abort.
/// - Allowed flags → pass silently.
/// - Value arguments (non-flag tokens after a flag) → pass silently.
///
/// # Errors
/// Returns `Err(String)` naming the denied flag if any denied flag is found.
pub(crate) fn validate_run_args(args: &[String]) -> Result<(), String> {
    for arg in args {
        // Only validate tokens that look like flags (start with `-`)
        if !arg.starts_with('-') {
            continue;
        }

        // Extract flag name: the part before `=` (or the whole arg if no `=`)
        let flag = match arg.find('=') {
            Some(pos) => &arg[..pos],
            None => arg.as_str(),
        };

        if DENIED_FLAGS.contains(&flag) {
            return Err(format!(
                "runArgs contains denied flag '{flag}': privilege-escalating flags are not allowed"
            ));
        }

        if !ALLOWED_FLAGS.contains(&flag) {
            tracing::warn!("runArgs contains unknown flag '{flag}'; allowing but not validated");
        }
    }

    Ok(())
}

/// Validate that a container name produced by `safe_name()` is safe for use in
/// Docker/Podman filter expressions and container names.
///
/// Valid pattern: `^[a-zA-Z0-9][a-zA-Z0-9_.-]*$`
///
/// # Errors
/// Returns `Err(String)` with a description if the name fails validation.
pub(crate) fn validate_container_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("container name is empty".to_string());
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err("container name is empty".to_string());
    };
    if !first.is_ascii_alphanumeric() {
        return Err(format!(
            "container name '{name}' must start with an alphanumeric character, got '{first}'"
        ));
    }
    for ch in chars {
        if !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '.' | '-') {
            return Err(format!(
                "container name '{name}' contains invalid character '{ch}'"
            ));
        }
    }
    Ok(())
}

/// Validate `remoteEnv` keys and values.
///
/// Keys must match `^[A-Za-z_][A-Za-z0-9_]*$`.
/// Values must not contain null bytes.
///
/// # Errors
/// Returns `Err(String)` naming the offending key or describing the problem.
pub(crate) fn validate_remote_env(
    env: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    for (key, value) in env {
        // Validate key format
        if key.is_empty() {
            return Err("remoteEnv contains an empty key".to_string());
        }
        let mut key_chars = key.chars();
        let Some(first) = key_chars.next() else {
            return Err("remoteEnv contains an empty key".to_string());
        };
        if !matches!(first, 'A'..='Z' | 'a'..='z' | '_') {
            return Err(format!(
                "remoteEnv key '{key}' must start with a letter or underscore, got '{first}'"
            ));
        }
        for ch in key_chars {
            if !matches!(ch, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_') {
                return Err(format!(
                    "remoteEnv key '{key}' contains invalid character '{ch}'"
                ));
            }
        }
        // Validate value has no null bytes
        if value.contains('\0') {
            return Err(format!(
                "remoteEnv value for key '{key}' contains a null byte"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> String {
        v.to_string()
    }

    #[test]
    fn empty_args_ok() {
        assert!(validate_run_args(&[]).is_ok());
    }

    #[test]
    fn allowed_env_flag_passes() {
        let args = vec![s("--env"), s("FOO=bar")];
        assert!(validate_run_args(&args).is_ok());
    }

    #[test]
    fn allowed_env_flag_with_equals_passes() {
        let args = vec![s("--env=FOO=bar")];
        assert!(validate_run_args(&args).is_ok());
    }

    #[test]
    fn allowed_network_flag_passes() {
        let args = vec![s("--network"), s("bridge")];
        assert!(validate_run_args(&args).is_ok());
    }

    #[test]
    fn allowed_short_flags_pass() {
        let args = vec![s("-e"), s("FOO=bar"), s("-p"), s("8080:8080")];
        assert!(validate_run_args(&args).is_ok());
    }

    #[test]
    fn denied_privileged_aborts() {
        let args = vec![s("--privileged")];
        let result = validate_run_args(&args);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("privileged"),
            "error message should contain 'privileged', got: {msg}"
        );
    }

    #[test]
    fn denied_cap_add_aborts() {
        let args = vec![s("--cap-add"), s("SYS_ADMIN")];
        let result = validate_run_args(&args);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("cap-add"),
            "error message should contain 'cap-add', got: {msg}"
        );
    }

    #[test]
    fn denied_cap_add_with_equals_aborts() {
        let args = vec![s("--cap-add=SYS_ADMIN")];
        let result = validate_run_args(&args);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("cap-add"),
            "error message should contain 'cap-add', got: {msg}"
        );
    }

    #[test]
    fn denied_security_opt_aborts() {
        let args = vec![s("--security-opt"), s("apparmor=unconfined")];
        let result = validate_run_args(&args);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("security-opt"));
    }

    #[test]
    fn denied_sysctl_aborts() {
        let args = vec![s("--sysctl"), s("net.ipv4.ip_forward=1")];
        let result = validate_run_args(&args);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("sysctl"),
            "error message should contain 'sysctl', got: {msg}"
        );
    }

    #[test]
    fn unknown_flag_warns_but_passes() {
        // --unknown-flag is not in either list → should warn but return Ok
        let args = vec![s("--unknown-flag")];
        let result = validate_run_args(&args);
        assert!(
            result.is_ok(),
            "unknown flags should warn but not abort, got: {result:?}"
        );
    }

    #[test]
    fn mixed_allowed_and_value_args_pass() {
        // Value args (non-flag tokens) should be skipped
        let args = vec![
            s("--env"),
            s("MY_VAR=hello"),
            s("--network"),
            s("host"),
            s("--label"),
            s("project=myapp"),
        ];
        assert!(validate_run_args(&args).is_ok());
    }

    #[test]
    fn denied_flag_among_allowed_aborts() {
        let args = vec![
            s("--env"),
            s("FOO=bar"),
            s("--privileged"),
            s("--network"),
            s("bridge"),
        ];
        let result = validate_run_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("privileged"));
    }

    #[test]
    fn denied_volume_aborts() {
        let args = vec![s("--volume"), s("/etc:/host-etc")];
        let result = validate_run_args(&args);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("volume"),
            "error message should contain 'volume', got: {msg}"
        );
    }

    #[test]
    fn denied_volume_equals_aborts() {
        let args = vec![s("--volume=/etc:/host-etc")];
        let result = validate_run_args(&args);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("volume"),
            "error message should contain 'volume', got: {msg}"
        );
    }

    #[test]
    fn denied_mount_aborts() {
        let args = vec![s("--mount"), s("type=bind,source=/etc,target=/host-etc")];
        let result = validate_run_args(&args);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("mount"),
            "error message should contain 'mount', got: {msg}"
        );
    }

    #[test]
    fn denied_short_v_aborts() {
        let args = vec![s("-v"), s("/etc:/host-etc")];
        let result = validate_run_args(&args);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("-v"),
            "error message should contain '-v', got: {msg}"
        );
    }

    // container name validation
    #[test]
    fn valid_container_name_ok() {
        assert!(validate_container_name("devcont-my-project").is_ok());
    }

    #[test]
    fn container_name_empty_is_err() {
        assert!(validate_container_name("").is_err());
    }

    #[test]
    fn container_name_starts_with_dash_is_err() {
        assert!(validate_container_name("-foo").is_err());
    }

    #[test]
    fn container_name_with_dollar_is_err() {
        assert!(validate_container_name("foo$bar").is_err());
    }

    #[test]
    fn container_name_alphanumeric_dot_dash_ok() {
        assert!(validate_container_name("abc.1-2_ok").is_ok());
    }

    #[test]
    fn env_key_empty_is_err() {
        let mut env = std::collections::HashMap::new();
        env.insert(String::new(), "value".to_string());
        assert!(validate_remote_env(&env).is_err());
    }

    // remoteEnv validation
    #[test]
    fn valid_env_key_ok() {
        let mut env = std::collections::HashMap::new();
        env.insert("MY_VAR".to_string(), "value".to_string());
        assert!(validate_remote_env(&env).is_ok());
    }

    #[test]
    fn env_key_starting_with_digit_is_err() {
        let mut env = std::collections::HashMap::new();
        env.insert("1VAR".to_string(), "value".to_string());
        assert!(validate_remote_env(&env).is_err());
    }

    #[test]
    fn env_key_with_hyphen_is_err() {
        let mut env = std::collections::HashMap::new();
        env.insert("MY-VAR".to_string(), "value".to_string());
        assert!(validate_remote_env(&env).is_err());
    }

    #[test]
    fn env_value_with_null_byte_is_err() {
        let mut env = std::collections::HashMap::new();
        env.insert("MY_VAR".to_string(), "val\0ue".to_string());
        assert!(validate_remote_env(&env).is_err());
    }

    #[test]
    fn env_value_normal_ok() {
        let mut env = std::collections::HashMap::new();
        env.insert("MY_VAR".to_string(), "hello world!".to_string());
        assert!(validate_remote_env(&env).is_ok());
    }
}
