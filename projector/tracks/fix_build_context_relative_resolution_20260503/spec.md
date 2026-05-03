# Spec — Resolve `build.context` Relative to `config_dir` Before Validation

## Severity
**Blocker (spec compliance)** — every spec-compliant `devcontainer.json` that uses a relative `build.context` of `".."` (or any path that resolves UP from `.devcontainer/` to the workspace root) is rejected with an "escapes workspace root" error. Per the [containers.dev spec](https://containers.dev/implementors/json_reference/), paths in `devcontainer.json` are interpreted relative to the directory containing `devcontainer.json` — so `"context": ".."` from `.devcontainer/devcontainer.json` IS the workspace root, and is valid.

The `tools-lib` consumer hit this and worked around it by dropping the `context` declaration entirely and adjusting `COPY` paths in their Dockerfile — a workaround that pushes the problem onto every downstream user.

This track closes the residual gap from `fix_dockerfile_path_resolution_20260503`. That track's acceptance criterion #7 read:

> `build.context` validation (`validate_build_context`, mod.rs:487): relative contexts are validated as residing within the workspace **root**, but resolved (for `docker build` invocation) against `config_dir`. The two responsibilities must be kept distinct.

The "resolved against config_dir" half landed (`resolve_build_context` now uses `config_dir`). The "validated *after* resolution" half didn't — `validate_build_context` still validates the literal context string against workspace root.

## Problem

`src/devcontainers/mod.rs:652` (`validate_build_context`) currently:

```rust
fn validate_build_context(root: &Path, context: &str) -> Result<()> {
    let context_path = Path::new(context);
    if context_path.is_absolute() {
        // ... warning-only, allowed through ...
        Ok(())
    } else {
        validate_within_root(root, context_path)?;  // ← rejects ".." literally
        Ok(())
    }
}
```

For `context = ".."`, `validate_within_root(workspace_root, "..")` fails: lexically, `..` resolves up from the *workspace root*, which is outside the root.

The spec-correct interpretation: from `config_dir = <workspace>/.devcontainer/`, `..` resolves to `<workspace>/`. That IS within the workspace root.

The call site is `src/devcontainers/mod.rs:694` (`validate_devcontainer_paths`), which already has access to the loaded `Devcontainer` and therefore to `config_dir`.

## Acceptance Criteria

1. **Resolve before validate.** `validate_build_context` accepts `config_dir` (the directory of the loaded `devcontainer.json`) in addition to `root`. Relative `context` paths are resolved as `config_dir.join(context)`, then lexically normalised (collapsing `..` segments without touching the filesystem). The resolved absolute path is checked to lie within `root`.
2. **`context = ".."`** from a `.devcontainer/devcontainer.json` layout passes. Resolves to workspace root.
3. **`context = "../sibling-project"`** from a `.devcontainer/devcontainer.json` layout fails (resolves outside workspace root).
4. **`context = "subdir"`** from `.devcontainer/devcontainer.json` layout passes (resolves to `<workspace>/.devcontainer/subdir`).
5. **`context = "."`** from `.devcontainer.json` (root layout) passes (resolves to workspace root).
6. **Absolute contexts** behave unchanged (warning if outside root, allowed through).
7. **Path-traversal protection preserved.** Use `paths::validate_within_root` *after* resolution. Lexical normalisation must collapse `..` segments correctly so that `<workspace>/.devcontainer/../../etc` is detected as out-of-root.
8. **Tests** at `tests/path_resolution_test.rs` extend to cover all five layouts:
    - `.devcontainer.json` (root layout) with `context = "."`
    - `.devcontainer/devcontainer.json` with `context = ".."`
    - `.devcontainer/devcontainer.json` with `context = "subdir"`
    - `.devcontainer/devcontainer.json` with `context = "../sibling-project"` (must fail)
    - any layout with absolute `context` inside root
    - any layout with absolute `context` outside root (warning, no error)
9. **Existing tests** at `tests/path_resolution_test.rs` continue to pass.
10. **The `tools-lib` consumer's original `devcontainer.json`** (with `"context": ".."`) builds successfully via `devcont rebuild` against a real engine, with no need to drop or rewrite the `context` field.

## Non-Goals

- Reshaping `validate_within_root` itself.
- Changing the warning-vs-error policy for absolute contexts outside root.
- Reshaping `validate_mounts`. Mount sources are spec'd as relative-to-workspace-folder (NOT relative to `config_dir`); leave alone.
- Filesystem canonicalisation (`Path::canonicalize`). The build hasn't run yet — the resolved path may not exist on disk. Use lexical normalisation only.

## Risks

- **Symlink TOCTOU.** Lexical normalisation cannot detect symlinks pointing outside the workspace root. We accept this trade-off; the workspace-root check is a soft barrier, not a security boundary (the user owns the host filesystem already).
- **Backward compatibility.** Anyone whose existing `devcontainer.json` had `"context": ".."` was already failing. Nobody can be relying on that failure as a feature; this fix is straight-additive.

## References

- containers.dev spec — JSON Reference (`build.context` semantics).
- `src/devcontainers/mod.rs:652` (`validate_build_context`).
- `src/devcontainers/mod.rs:694` (`validate_devcontainer_paths`) — call site.
- `src/devcontainers/mod.rs:708` (`resolve_build_context`) — already uses `config_dir`; mirror the resolution.
- `projector/tracks/fix_dockerfile_path_resolution_20260503/spec.md` acceptance criterion #7 — the gap this closes.
- `tests/path_resolution_test.rs` — existing path-resolution coverage to extend.
