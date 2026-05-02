# Spec — Fix `build.dockerfile` Path Resolution

## Severity
**Blocker** — every spec-compliant `devcontainer.json` that lives at `.devcontainer/devcontainer.json` and references a relative `build.dockerfile` (with no `build.context`) currently fails because devcont resolves the Dockerfile relative to the workspace root instead of relative to the directory containing `devcontainer.json`.

## Problem

Per the [containers.dev spec](https://containers.dev/implementors/json_reference/), paths in `devcontainer.json` (`build.dockerfile`, `build.context`, `dockerComposeFile`, …) are interpreted **relative to the directory containing `devcontainer.json`**, not relative to the workspace root.

Today, `src/provider/utils.rs:307` (`resolve_dockerfile_path`) does the wrong thing when `context` is `None`:

```rust
let base = if let Some(ctx) = context {
    /* … context-relative branch … */
} else {
    workspace.to_path_buf()      // <- wrong: should be the dir holding devcontainer.json
};
base.join(dockerfile_path)
```

`devcontainers/mod.rs:466` (`resolve_build_source`) calls it with `directory` = workspace root, so a `devcontainer.json` at `.devcontainer/devcontainer.json` with `"build.dockerfile": "Dockerfile"` resolves to `<workspace>/Dockerfile` instead of `<workspace>/.devcontainer/Dockerfile`. Validation then either fails or — worse — succeeds against an unrelated file at the workspace root.

The compose path resolver at `devcontainers/mod.rs:447` (`compose_path_and_service`) hard-codes `.devcontainer` for `dockerComposeFile`:

```rust
let compose_path = directory.join(".devcontainer").join(compose_file);
```

That works for the common case (`devcontainer.json` at `.devcontainer/devcontainer.json`) but is **also incorrect** for the alternative location (`.devcontainer.json` at workspace root). Both should resolve relative to the directory containing the `devcontainer.json` we actually loaded.

## Acceptance Criteria

1. **Path resolution is anchored to the `devcontainer.json` directory.** `Devcontainer` (or its `Config` loader) tracks the absolute directory of the `devcontainer.json` file actually loaded (`<workspace>/.devcontainer/` or `<workspace>/`) and exposes it. Call this `config_dir`.
2. **`build.dockerfile`** without a `build.context`: resolves to `<config_dir>/<dockerfile>`.
3. **`build.dockerfile`** with a relative `build.context`: resolves to `<config_dir>/<context>/<dockerfile>`.
4. **`build.dockerfile`** with an absolute `build.context`: resolves to `<context>/<dockerfile>` (unchanged behavior).
5. **Absolute `build.dockerfile`**: returned unchanged (unchanged behavior).
6. **`dockerComposeFile`** is resolved against `config_dir` (not against `<workspace>/.devcontainer/` literally), keeping current behavior for `.devcontainer/devcontainer.json` and fixing it for `.devcontainer.json` at the workspace root.
7. **`build.context`** validation (`validate_build_context`, mod.rs:487): relative contexts are validated as residing within the workspace **root**, but resolved (for `docker build` invocation) against `config_dir`. The two responsibilities must be kept distinct.
8. **Tests at `provider/utils.rs:367–391`** are updated to encode the new (correct) behavior.
9. **Regression tests** are added covering both `.devcontainer/devcontainer.json` and `.devcontainer.json` (workspace-root) layouts for each of: `build.dockerfile` only, `build.context` only, both, neither (image-based), and `dockerComposeFile`.
10. **No silent fallback** to the workspace root when the resolved path does not exist — surface a clear error naming the file we expected to find.

## Non-Goals

- Supporting `devcontainer.json` files outside the standard locations (`.devcontainer/devcontainer.json` and `.devcontainer.json`).
- Rewriting `validate_within_root`/`validate_build_context` semantics. The validation root remains the workspace root; only the resolution base changes.
- Changing the public API of `Devcontainer` beyond the minimum needed (we may add a getter for `config_dir`).

## Out of Scope (Adjacent but Separate)

- Bug 3 (start-orchestration error cascade) — tracked separately.
- Adding an `inspect`/`container-name` subcommand — tracked separately.

## Risks

- **Mounts and other path fields** may share the same incorrect anchoring. We will audit `validate_mounts`, the `dockerComposeFile` resolver, and `templates/docker-compose.yml` rendering for the same bug as part of Phase 1.
- **Backwards compatibility**: anyone who unintentionally depended on the workspace-root anchoring will see a behavior change. This is acceptable — the previous behavior was a spec violation.

## References

- containers.dev spec — JSON Reference: <https://containers.dev/implementors/json_reference/>
- `src/provider/utils.rs:307` (`resolve_dockerfile_path`)
- `src/provider/utils.rs:367–391` (existing tests encoding the wrong behavior)
- `src/devcontainers/mod.rs:447` (`compose_path_and_service`)
- `src/devcontainers/mod.rs:466` (`resolve_build_source`)
- `src/devcontainers/mod.rs:487` (`validate_build_context`)
