# Plan — Resolve `build.context` Relative to `config_dir` Before Validation

Track ID: `fix_build_context_relative_resolution_20260503`
Spec: [./spec.md](./spec.md)

> **Sequencing note:** depends on `fix_dockerfile_path_resolution_20260503` (already landed) — closes that track's acceptance criterion #7. No new tracks are blocked by this one.

---

## Phase 1: Red — Failing Tests for `..` and Cousins

- [ ] Task: Audit `tests/path_resolution_test.rs` and `src/devcontainers/mod.rs::tests` for any existing `validate_build_context` tests. List which inputs are already covered.
- [ ] Task: Red Phase — add new tests in `tests/path_resolution_test.rs`:
    - `validate_build_context_dotdot_resolves_to_workspace_root`: `.devcontainer/devcontainer.json` layout, `context = ".."` → must NOT error.
    - `validate_build_context_subdir_relative_to_config_dir`: `context = "subdir"` → must NOT error; resolves to `<workspace>/.devcontainer/subdir`.
    - `validate_build_context_root_layout_dot`: `.devcontainer.json` layout, `context = "."` → must NOT error.
    - `validate_build_context_dotdot_dotdot_escapes_root`: `.devcontainer/devcontainer.json` layout, `context = "../../escape"` → must error.
    - `validate_build_context_sibling_directory_escapes_root`: `.devcontainer/devcontainer.json` layout, `context = "../sibling-project"` → must error.
    - `validate_build_context_absolute_inside_root`: absolute path inside workspace → must NOT error.
    - `validate_build_context_absolute_outside_root_warns_only`: absolute path outside workspace → must NOT error (warning only). Verify no error returned; tracing warning emission is best-effort.
    - Run `cargo test validate_build_context` — confirm the first three new tests fail (the `..` rejection bug). Existing tests stay green.
- [ ] Task: Pre-commit checks (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, `cargo test`) — accept that the new tests fail; commit them anyway in the next step or roll into the green commit.
- [ ] Task: Commit (`test(devcontainers): cover build.context relative resolution before validation`) — commit the failing tests so the green commit's diff is clear. Use `--allow-empty` is NOT needed; the tests compile but fail.
- [ ] Task: Projector — User Manual Verification 'Phase 1: Red — Failing Tests for `..` and Cousins' (Protocol in workflow.md)

---

## Phase 2: Green — Resolve-Then-Validate

- [ ] Task: Green Phase — change `validate_build_context`'s signature to `(root: &Path, config_dir: &Path, context: &str)`.
    - For relative `context_path`: compute `resolved = lexical_normalize(config_dir.join(context_path))`, then call `validate_within_root(root, &resolved)`.
    - For absolute `context_path`: behaviour unchanged (warning-only check against root).
    - Update the doc-comment to reflect the new contract.
- [ ] Task: Green Phase — update the call site at `validate_devcontainer_paths` (mod.rs:694) to pass `config_dir`. The function `validate_devcontainer_paths` itself receives `directory` (workspace root) today; thread `config_dir` through, OR call it with both. Inspect the caller chain and pick the smaller refactor.
- [ ] Task: Green Phase — implement `lexical_normalize(path: &Path) -> PathBuf` (or use an existing helper) that:
    - Iterates path components.
    - On `Component::ParentDir` (`..`), pops the last accumulated component if any (and that component is not itself `..`); if the stack is empty, retains the `..` literal so out-of-root detection still works.
    - On `Component::CurDir` (`.`), skips.
    - On `Component::Normal(s)`, pushes.
    - Preserves the path's prefix and root.
    - Add unit tests for `lexical_normalize` covering: `.`, `..`, `a/b/../c` → `a/c`, `a/b/c/../..` → `a`, `../etc` → `../etc` (escapes), `/abs/../path` → `/path`.
- [ ] Task: Run `cargo test` — confirm all new Phase 1 tests pass and existing tests still pass.
- [ ] Task: Refactor — if `lexical_normalize` is useful elsewhere (e.g., the `paths` module), promote it to `src/devcontainers/paths.rs` for reuse.
- [ ] Task: Verify Coverage on `validate_build_context` and `lexical_normalize` ≥ 80%.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`fix(devcontainers): resolve build.context against config_dir before validating root containment`).
- [ ] Task: Projector — User Manual Verification 'Phase 2: Green — Resolve-Then-Validate' (Protocol in workflow.md)

---

## Phase 3: End-to-End Smoke Test with a Real Engine

- [ ] Task: Recreate (or create new) the smoke fixture at `tmp/build-context-smoke/`:
    - `tmp/build-context-smoke/Dockerfile` with `FROM alpine:3` and a `COPY ./README.md /README.md` (or similar — anything that requires the workspace-root context to succeed).
    - `tmp/build-context-smoke/README.md` with one line.
    - `tmp/build-context-smoke/.devcontainer/devcontainer.json`:
      ```json
      {
        "name": "build-context-smoke",
        "build": { "dockerfile": "../Dockerfile", "context": ".." }
      }
      ```
    - Note: this uses `..` for both `dockerfile` (resolves to `<workspace>/Dockerfile`) and `context` (resolves to `<workspace>`).
- [ ] Task: Build the release binary (`cargo build --release`).
- [ ] Task: Run `./target/release/devcont rebuild --no-cache tmp/build-context-smoke` against Docker 29.3.0 (running per Bug-fix track #1 smoke test).
- [ ] Task: Confirm:
    - Build succeeds (`docker build` returns 0, image is created).
    - The file `/README.md` is in the resulting image (`docker run --rm <image> cat /README.md` returns the line).
    - No "escapes workspace root" error appears in stderr.
    - Document captured stdout/stderr in this plan's "Verification Log" section.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`projector(plan): record Phase 3 smoke test for build.context resolution`).
- [ ] Task: Projector — User Manual Verification 'Phase 3: End-to-End Smoke Test with a Real Engine' (Protocol in workflow.md)

### Verification Log (filled in during Phase 3)

- _Pending — fill with the actual `docker build` output on completion._

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- The `tools-lib` consumer's original `devcontainer.json` (with `"context": ".."`) builds successfully via `devcont rebuild` against a real engine, with no need to drop or rewrite the `context` field.
- `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- Quality gates green at every commit.
- Manual verification recorded in a git note on the final checkpoint commit:
    - reproduce a build with `"context": ".."`,
    - confirm the build succeeds,
    - confirm exit status is zero.
