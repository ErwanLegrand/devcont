# Spec — `build.dockerfile` Resolution Must Be Independent of `build.context`

## Source
Filed as **Bug 6** by the user on 2026-05-04, while verifying the Bug 4 fix (`fix_build_context_relative_resolution_20260503`). Quoting the report:

> Residual (call it Bug 6) — when context IS explicitly set, the dockerfile path resolution is wrong: `-f /tmp/devcont-bug4/Dockerfile` instead of `/tmp/devcont-bug4/.devcontainer/Dockerfile`. Per spec, dockerfile is always relative to devcontainer.json regardless of context.

## Severity
**Blocker (spec compliance)** — every devcontainer.json that sets both `build.dockerfile` (relative) and `build.context` (relative) resolves the dockerfile against the wrong base. With `dockerfile: "Dockerfile"` and `context: ".."` from `<workspace>/.devcontainer/devcontainer.json`, devcont passes `-f <workspace>/Dockerfile` to docker, but the spec-correct path is `<workspace>/.devcontainer/Dockerfile`. Build either fails ("no such file") or — worse — silently picks up an unrelated Dockerfile at the workspace root.

## Spec Citation

Per [containers.dev JSON Reference](https://containers.dev/implementors/json_reference/):

- **`build.dockerfile`**: *"The location of the Dockerfile that defines the contents of the container. The path is **relative to the devcontainer.json file**."*
- **`build.context`**: *"Path that the Docker build should be run from **relative to devcontainer.json**. For example, a value of `".."` would allow you to reference content in sibling directories. The default is `"."`."*

Both fields are independently anchored at the directory containing `devcontainer.json`. They are NOT chained. The Dockerfile path is never relative to the context.

## Problem

`src/provider/utils.rs:338` (`resolve_dockerfile_path`):

```rust
pub(crate) fn resolve_dockerfile_path(
    config_dir: &std::path::Path,
    dockerfile: &str,
    context: Option<&str>,
) -> std::path::PathBuf {
    let dockerfile_path = std::path::Path::new(dockerfile);
    if dockerfile_path.is_absolute() {
        return dockerfile_path.to_path_buf();
    }
    let base = if let Some(ctx) = context {
        let ctx_path = std::path::Path::new(ctx);
        if ctx_path.is_absolute() {
            ctx_path.to_path_buf()
        } else {
            config_dir.join(ctx_path)         // ← context chained
        }
    } else {
        config_dir.to_path_buf()
    };
    base.join(dockerfile_path)                // ← dockerfile resolved against `base`, which may be context-relative
}
```

When `context = Some("..")` and `dockerfile = "Dockerfile"`:

- `base = config_dir.join("..")` = workspace root.
- Result: `<workspace>/Dockerfile`.
- Spec expects: `<config_dir>/Dockerfile` = `<workspace>/.devcontainer/Dockerfile`.

This mimics `docker build -f`'s relative-to-context behaviour, which is the wrong reference for `devcontainer.json`.

## Why It Wasn't Caught Earlier

`fix_dockerfile_path_resolution_20260503`'s acceptance criterion #3 (authored 2026-05-03) read:

> 3. **`build.dockerfile`** with a relative `build.context`: resolves to `<config_dir>/<context>/<dockerfile>`.

That criterion is **wrong** with respect to the containers.dev spec. The implementing agent followed it faithfully, including a passing TDD test (`resolve_dockerfile_with_relative_context`) that locks in the bug. TDD verified implementation-vs-spec; the spec itself wasn't traced back to the canonical source per criterion. Process improvement noted in the "Lessons" section below.

## Acceptance Criteria

1. **`build.dockerfile` (relative) always resolves against `config_dir`**, never against `context`. Function signature should reflect this — `context` should not influence dockerfile resolution.
2. **`build.dockerfile` (absolute) is returned unchanged** (matches existing behaviour and the spec's "relative to devcontainer.json" only applies when relative).
3. **`build.context` resolution is unchanged** by this track. `resolve_build_context` (mod.rs:708) is correct as of `fix_build_context_relative_resolution_20260503` and stays.
4. **The `context` parameter** is removed from `resolve_dockerfile_path`'s signature (or kept and explicitly documented as unused). Either is acceptable; recommend removal so the API matches the semantics.
5. **Existing test** `resolve_dockerfile_with_relative_context` (utils.rs around line 367–391) is **flipped** — under the spec-correct behaviour, that test's expected output changes from `/ws/subdir/Dockerfile` to `/ws/Dockerfile`. Rename to `resolve_dockerfile_ignores_relative_context` or similar.
6. **Existing test** `resolve_dockerfile_absolute_path_returned_unchanged` continues to pass.
7. **New test** `resolve_dockerfile_dotdot_context_does_not_affect_path`: with `dockerfile = "Dockerfile"` and `context = Some("..")`, result is `<config_dir>/Dockerfile`.
8. **New test** `resolve_dockerfile_absolute_context_does_not_affect_path`: with `dockerfile = "Dockerfile"` and `context = Some("/abs/ctx")`, result is `<config_dir>/Dockerfile` (NOT `/abs/ctx/Dockerfile`).
9. **Integration coverage** at `tests/path_resolution_test.rs` adds an end-to-end test with a devcontainer.json containing both `dockerfile: "Dockerfile"` and `context: ".."`, asserting the resolved Dockerfile path is under `.devcontainer/`.
10. **Smoke test** against a real Docker daemon: reproduce the user's Bug 6 fixture (`/tmp/devcont-bug4/.devcontainer/{devcontainer.json, Dockerfile}` with `context: ".."`) using `tmp/devcont-bug6-smoke/` per repo convention; confirm the printed `docker build -f ...` line points at `<fixture>/.devcontainer/Dockerfile` and the build proceeds.

## Non-Goals

- **`build.context` semantics.** Already correct after Track #4.
- **Mount source resolution.** Mount sources are spec'd as relative-to-workspace-folder, not relative to `config_dir`; out of scope.
- **`docker compose` path resolution.** Compose-file resolution is correct after Track #2; only `build.dockerfile` is broken here.

## Lessons (process improvement)

The wrong acceptance criterion shipped because the spec author (me, the planning conversation) wrote criterion #3 from intuition rather than from the canonical source. Going forward, every spec criterion that asserts a path-resolution rule should:

1. **Quote the canonical source verbatim** in a "Spec Citation" block (as this spec does).
2. **List worked examples** in the criterion itself, including pathological combinations (`dockerfile + context = ".."`, `dockerfile + absolute context`, etc.).
3. Be **diff-reviewed** against the canonical source, not just for internal consistency.

This is informational; no infrastructure change required. The convention can be carried forward in future specs.

## Risks

- **Backward compatibility.** Anyone whose Dockerfile happened to be at `<context>/Dockerfile` AND not at `<config_dir>/Dockerfile` will now fail to find their Dockerfile. This was a spec violation; we accept the break.
- **Cross-checking other path-resolution functions.** `resolve_build_context` correctly uses `config_dir`. `compose_path_and_service` correctly uses `config_dir`. No other path-bearing field should change.

## References

- [containers.dev JSON Reference](https://containers.dev/implementors/json_reference/) — the authoritative source.
- `src/provider/utils.rs:338` (`resolve_dockerfile_path`) — implementation.
- `src/provider/utils.rs:367–391` — existing tests (one to flip).
- `src/devcontainers/mod.rs:466` (`resolve_build_source`) — caller; passes `context.as_deref()`.
- `projector/tracks/fix_dockerfile_path_resolution_20260503/spec.md` — predecessor track whose acceptance criterion #3 was wrong.
- `tests/path_resolution_test.rs` — existing regression coverage to extend.
