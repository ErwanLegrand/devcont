use std::io;
use std::path::{Component, Path, PathBuf};

/// Normalize a path by resolving `..` and `.` components without requiring
/// the path to exist on the filesystem (unlike `canonicalize`).
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                components.pop();
            }
            Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
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
}
