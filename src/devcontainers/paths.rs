use std::io;
use std::path::{Component, Path, PathBuf};

/// Normalize a path lexically by collapsing `.` and `..` components.
///
/// Unlike [`std::fs::canonicalize`], this function never accesses the filesystem —
/// the path does not need to exist. This is important for build contexts and
/// Dockerfile paths that may not yet be present when `devcontainer.json` is loaded.
///
/// ## Rules
///
/// - `Component::CurDir` (`.`) is skipped.
/// - `Component::ParentDir` (`..`) pops the last accumulated *non-`..`* component.
///   If the accumulated stack is empty or the top is already `..`, the `..` is
///   retained, preserving leading `..` segments on relative paths so that
///   out-of-root detection downstream still works.
/// - `Component::RootDir` and `Component::Prefix` are pushed as-is (absolute paths
///   cannot pop past their root via this function).
/// - `Component::Normal(s)` is pushed.
///
/// ## Examples
///
/// ```text
/// /a/b/../c          →  /a/c
/// /a/b/c/../..       →  /a
/// /workspace/../etc  →  /etc      (escape detected by starts_with check in caller)
/// ../foo             →  ../foo    (leading .. preserved for relative paths)
/// a/b/../c           →  a/c
/// ```
pub(crate) fn lexical_normalize(path: &Path) -> PathBuf {
    let mut parts: Vec<Component<'_>> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {
                // Skip `.` — it contributes nothing.
            }
            Component::ParentDir => {
                // Pop the last normal component, if any.
                // Never pop a RootDir, Prefix, or a leading `..` (to preserve
                // the ability to detect escapes on relative paths).
                let should_pop = matches!(parts.last(), Some(Component::Normal(_)));
                if should_pop {
                    parts.pop();
                } else {
                    // Stack is empty, topped by root, prefix, or `..` — retain `..`.
                    parts.push(component);
                }
            }
            other => {
                parts.push(other);
            }
        }
    }
    parts.iter().collect()
}

/// Normalize a path by resolving `..` and `.` components without requiring
/// the path to exist on the filesystem (unlike `canonicalize`).
///
/// Private alias kept for backward compatibility within this module.
/// New callers should use [`lexical_normalize`] directly.
fn normalize_path(path: &Path) -> PathBuf {
    lexical_normalize(path)
}

/// Resolve `candidate` relative to `root` (if relative) and verify the result
/// stays within `root`.
///
/// - Relative candidates are resolved as `root.join(candidate)`.
/// - Absolute candidates are used directly.
/// - Both are normalized (no filesystem access).
///
/// Returns the normalized absolute path on success, or a
/// `PermissionDenied` error if the resolved path escapes `root`.
pub(crate) fn validate_within_root(root: &Path, candidate: &Path) -> io::Result<PathBuf> {
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };

    let normalized = normalize_path(&joined);
    let normalized_root = normalize_path(root);

    if normalized.starts_with(&normalized_root) {
        Ok(normalized)
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "path '{}' escapes workspace root '{}'",
                candidate.display(),
                root.display()
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn normal_relative_path_is_ok() {
        let result = validate_within_root(Path::new("/workspace"), Path::new("subdir/file"));
        assert!(result.is_ok(), "normal relative path should be allowed");
        assert_eq!(result.unwrap(), PathBuf::from("/workspace/subdir/file"));
    }

    #[test]
    fn parent_traversal_sibling_is_err() {
        let result = validate_within_root(Path::new("/workspace"), Path::new("../sibling"));
        assert!(
            result.is_err(),
            "../sibling should be rejected as it escapes workspace"
        );
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn deep_traversal_etc_passwd_is_err() {
        let result = validate_within_root(Path::new("/workspace"), Path::new("../../etc/passwd"));
        assert!(
            result.is_err(),
            "../../etc/passwd should be rejected as it escapes workspace"
        );
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn absolute_path_within_root_is_ok() {
        let result = validate_within_root(Path::new("/workspace"), Path::new("/workspace/sub"));
        assert!(
            result.is_ok(),
            "absolute path inside workspace should be allowed"
        );
        assert_eq!(result.unwrap(), PathBuf::from("/workspace/sub"));
    }

    #[test]
    fn absolute_path_outside_root_is_err() {
        let result = validate_within_root(Path::new("/workspace"), Path::new("/etc/passwd"));
        assert!(
            result.is_err(),
            "/etc/passwd should be rejected as it is outside workspace"
        );
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn dot_in_path_is_ok() {
        let result = validate_within_root(Path::new("/workspace"), Path::new("./src/main.rs"));
        assert!(result.is_ok(), "./src/main.rs should be allowed");
        assert_eq!(result.unwrap(), PathBuf::from("/workspace/src/main.rs"));
    }

    #[test]
    fn traversal_then_back_in_is_ok() {
        // subdir/../file stays within root
        let result = validate_within_root(Path::new("/workspace"), Path::new("subdir/../file"));
        assert!(
            result.is_ok(),
            "subdir/../file resolves to /workspace/file which is within root"
        );
        assert_eq!(result.unwrap(), PathBuf::from("/workspace/file"));
    }

    #[test]
    fn root_itself_is_ok() {
        let result = validate_within_root(Path::new("/workspace"), Path::new("."));
        assert!(result.is_ok(), ". (root itself) should be allowed");
    }

    // ----- lexical_normalize unit tests -----

    #[test]
    fn lexical_normalize_removes_dot() {
        assert_eq!(lexical_normalize(Path::new("./foo")), PathBuf::from("foo"));
    }

    #[test]
    fn lexical_normalize_collapses_dotdot() {
        assert_eq!(
            lexical_normalize(Path::new("a/b/../c")),
            PathBuf::from("a/c")
        );
    }

    #[test]
    fn lexical_normalize_collapses_multiple_dotdot() {
        assert_eq!(
            lexical_normalize(Path::new("a/b/c/../..")),
            PathBuf::from("a")
        );
    }

    #[test]
    fn lexical_normalize_preserves_leading_dotdot_relative() {
        // Leading `..` on a relative path must be retained so callers can detect
        // out-of-root traversals.
        assert_eq!(
            lexical_normalize(Path::new("../etc")),
            PathBuf::from("../etc")
        );
    }

    #[test]
    fn lexical_normalize_abs_path_dotdot_collapses() {
        assert_eq!(
            lexical_normalize(Path::new("/abs/../path")),
            PathBuf::from("/path")
        );
    }

    #[test]
    fn lexical_normalize_workspace_devcontainer_dotdot() {
        // The primary use-case: config_dir = /ws/.devcontainer, context = ".."
        // → /ws/.devcontainer/.. → /ws
        let p = Path::new("/ws/.devcontainer/..");
        assert_eq!(lexical_normalize(p), PathBuf::from("/ws"));
    }

    #[test]
    fn lexical_normalize_workspace_devcontainer_dotdot_sibling() {
        // context = "../sibling" from /ws/.devcontainer → /ws/sibling (inside workspace)
        let p = Path::new("/ws/.devcontainer/../sibling");
        assert_eq!(lexical_normalize(p), PathBuf::from("/ws/sibling"));
    }

    #[test]
    fn lexical_normalize_double_escape() {
        // context = "../../escape" from /ws/.devcontainer → /escape (outside workspace)
        let p = Path::new("/ws/.devcontainer/../../escape");
        assert_eq!(lexical_normalize(p), PathBuf::from("/escape"));
    }
}
