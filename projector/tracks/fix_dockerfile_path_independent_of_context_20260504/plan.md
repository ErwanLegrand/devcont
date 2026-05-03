# Plan — `build.dockerfile` Resolution Must Be Independent of `build.context`

Track ID: `fix_dockerfile_path_independent_of_context_20260504`
Spec: [./spec.md](./spec.md)

> **Sequencing note:** depends on `fix_build_context_relative_resolution_20260503` (already landed). Closes a residual spec-compliance gap from `fix_dockerfile_path_resolution_20260503`'s acceptance criterion #3. No new tracks blocked by this one.

---

## Phase 1: Red — Failing Tests for Spec-Correct Behaviour

- [ ] Task: Audit `src/provider/utils.rs::tests` for `resolve_dockerfile_path` coverage. Identify which tests encode the current (wrong) behaviour and need flipping vs. which encode behaviour that survives the fix.
    - `resolve_dockerfile_no_context_relative_to_devcontainer_dir`: keep (correct already).
    - `resolve_dockerfile_with_relative_context`: **flip** — expected output changes from `/ws/subdir/Dockerfile` to `/ws/Dockerfile`. Rename to `resolve_dockerfile_ignores_relative_context`.
    - `resolve_dockerfile_with_absolute_context`: **flip** — expected output changes from `/other/ctx/Dockerfile` to `<config_dir>/Dockerfile`. Rename to `resolve_dockerfile_ignores_absolute_context`.
    - `resolve_dockerfile_absolute_path_returned_unchanged`: keep.
- [ ] Task: Red Phase — adjust the two existing tests above and add the following new ones:
    - `resolve_dockerfile_dotdot_context_does_not_affect_path`: `config_dir = "/ws/.devcontainer"`, `dockerfile = "Dockerfile"`, `context = Some("..")` → `/ws/.devcontainer/Dockerfile`.
    - `resolve_dockerfile_absolute_context_does_not_affect_path`: `dockerfile = "Dockerfile"`, `context = Some("/abs/ctx")` → `<config_dir>/Dockerfile` (NOT `/abs/ctx/Dockerfile`).
    - `resolve_dockerfile_subdir_context_does_not_affect_path`: `dockerfile = "Dockerfile"`, `context = Some("subdir")` → `<config_dir>/Dockerfile`.
    - Run `cargo test resolve_dockerfile` — confirm the flipped + new tests fail; the absolute-path test still passes.
- [ ] Task: Pre-commit checks (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, `cargo test`) — accept the failing `resolve_dockerfile_*` tests; everything else stays green.
- [ ] Task: Commit (`test(provider): cover spec-correct dockerfile resolution independent of context`).
- [ ] Task: Projector — User Manual Verification 'Phase 1: Red — Failing Tests for Spec-Correct Behaviour' (Protocol in workflow.md)

---

## Phase 2: Green — Always Resolve Against `config_dir`

- [ ] Task: Green Phase — change `resolve_dockerfile_path` so the dockerfile is always resolved against `config_dir` for relative paths, and unchanged for absolute paths. The `context` parameter no longer influences the result.
    - **Recommended:** drop the `context` parameter entirely; update the doc-comment to state the spec-correct contract; update the single caller in `src/devcontainers/mod.rs:466` (`resolve_build_source`) to pass only `config_dir` and `dockerfile`.
    - **Acceptable alternative:** keep the parameter for now but ignore it; doc-comment explicitly notes "deprecated; ignored — kept for ABI continuity"; mark with `#[allow(unused)]` or `_context` binding.
- [ ] Task: Run `cargo test` — confirm all `resolve_dockerfile_*` tests pass and existing tests still pass.
- [ ] Task: Refactor — if the simplified function is now trivial (`config_dir.join(dockerfile)` or absolute pass-through), inline at the call site only if it doesn't reduce readability. Otherwise keep the named function.
- [ ] Task: Verify Coverage — `resolve_dockerfile_path` should remain at 100% with the new test set.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`fix(provider): resolve build.dockerfile against config_dir independent of build.context`).
- [ ] Task: Projector — User Manual Verification 'Phase 2: Green — Always Resolve Against config_dir' (Protocol in workflow.md)

---

## Phase 3: Integration Test + Real-Engine Smoke Test

- [ ] Task: Red Phase — add an integration test in `tests/path_resolution_test.rs`:
    - `dockerfile_path_independent_of_context_with_dotdot`: build a temp `.devcontainer/devcontainer.json` with `"dockerfile": "Dockerfile", "context": ".."`. Place the actual `Dockerfile` at `.devcontainer/Dockerfile`. Load with `Devcontainer::load`, call `resolve_build_source`, assert the resolved path is `<workspace>/.devcontainer/Dockerfile`.
- [ ] Task: Run the new integration test; confirm it passes after Phase 2's source change.
- [ ] Task: Smoke fixture — create `tmp/devcont-bug6-smoke/`:
    - `.devcontainer/devcontainer.json`:
      ```json
      { "name": "bug6", "build": { "dockerfile": "Dockerfile", "context": ".." } }
      ```
    - `.devcontainer/Dockerfile`:
      ```
      FROM alpine:3
      CMD ["sleep", "infinity"]
      ```
    - (No top-level `Dockerfile` in the fixture root — the bug-flavoured behaviour would look there and fail; the fixed behaviour finds the `.devcontainer/Dockerfile`.)
- [ ] Task: Build release binary (`cargo build --release`).
- [ ] Task: Run `./target/release/devcont up --trust tmp/devcont-bug6-smoke`. Confirm:
    - Printed `docker build -f` path ends in `tmp/devcont-bug6-smoke/.devcontainer/Dockerfile` (NOT `tmp/devcont-bug6-smoke/Dockerfile`).
    - Build completes (alpine is small; a CACHED build is fine if the image is already locally available).
    - `docker ps` shows the container `Up`.
    - Tear down: `docker stop $(./target/release/devcont container-name tmp/devcont-bug6-smoke) && docker rm $_`.
- [ ] Task: Document captured stdout/stderr in this plan's "Verification Log" section.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`projector(plan): record Phase 3 smoke test for dockerfile path independence`).
- [ ] Task: Projector — User Manual Verification 'Phase 3: Integration Test + Real-Engine Smoke Test' (Protocol in workflow.md)

### Verification Log (filled in during Phase 3)

- _Pending — fill with the actual `docker build -f` line and `docker ps` status on completion._

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- The user's Bug 6 reproducer (`devcontainer.json` with `dockerfile: "Dockerfile", context: ".."` and Dockerfile at `.devcontainer/Dockerfile`) builds successfully via `devcont up --trust`.
- `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- `tests/path_resolution_test.rs` test count grew by ≥ 1.
- `src/provider/utils.rs::tests` test count grew by ≥ 3 (two flipped, three added — net new ≥ 3 if rename is in-place).
- Quality gates green at every commit.
- Manual verification recorded in a git note on the final checkpoint commit:
    - reproduce the build with `context: ".."` and Dockerfile at `.devcontainer/Dockerfile`,
    - confirm the `-f` path printed by devcont points inside `.devcontainer/`,
    - confirm exit status is zero.
