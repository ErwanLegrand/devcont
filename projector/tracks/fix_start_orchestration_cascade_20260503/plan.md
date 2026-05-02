# Plan — Fix `start` Orchestration Error Cascade

Track ID: `fix_start_orchestration_cascade_20260503`
Spec: [./spec.md](./spec.md)

> **Sequencing note:** this track should land **before** `fix_dockerfile_path_resolution_20260503`, so users hitting the dockerfile-path bug see one clear error instead of six.

---

## Phase 1: Reshape `run_and_check` and the `Provider` Trait [checkpoint: df097c2]

- [x] Task: Red Phase — add a unit test in `provider/utils.rs::tests` that runs a command which exits with status 2 and asserts `run_and_check` returns `Err` whose message includes "exit code 2" (or the relevant `format_exec_error` mapping). df097c2
- [x] Task: Red Phase — adjust the existing `run_and_check_returns_false_on_failure` test (utils.rs:738+) to assert `Err` instead of `Ok(false)`. Confirm tests fail. df097c2
- [x] Task: Green Phase — change `run_and_check`'s signature to `-> io::Result<()>`. Capture stdout/stderr (via `Command::output` instead of `Command::status`); on non-zero status, return `Err(io::Error::other(format_exec_error(code, &stderr)))`. df097c2
- [x] Task: Green Phase — change every `Provider::{build,create,start,stop,restart,attach,rm,cp}` signature in `provider/mod.rs` from `io::Result<bool>` to `io::Result<()>`. Update each impl (`docker.rs`, `docker_compose.rs`, `podman.rs`, `podman_compose.rs`, `nerdctl.rs`, `apple.rs`) to drop the bool. Keep `exists` and `running` as `io::Result<bool>` (genuine boolean facts). df097c2
- [x] Task: Green Phase — update the test stub `Provider` impl at `devcontainers/mod.rs:804+` to match. Make the stub support per-method "fail next" toggles (one of `build`, `create`, `start`, `restart`, `attach`, `stop`) so we can write per-step abort tests. df097c2
- [x] Task: Refactor — fold any `let _ = runtime.foo();` discards into proper `?` propagation. Run `cargo clippy -- -D clippy::let_underscore_must_use -D clippy::let_underscore_drop` and clean up findings. df097c2 (no discards existed; clippy clean)
- [x] Task: Verify Coverage on `provider/utils.rs` and the trait impls. df097c2 (326 tests pass)
- [x] Task: Pre-commit checks. df097c2
- [x] Task: Commit (`refactor(provider): make non-zero exit propagate as Err`). df097c2
- [x] Task: Projector — User Manual Verification 'Phase 1: Reshape run_and_check and the Provider Trait' (Protocol in workflow.md) df097c2

---

## Phase 2: Update Orchestration to Stop on First Failure [checkpoint: df097c2]

- [x] Task: Red Phase — add new tests under `devcontainers/mod.rs::tests` mirroring the existing hook-failure tests: run_aborts_on_{build,create,start,restart,attach,stop}_failure. df097c2
- [x] Task: Green Phase — review `Devcontainer::run` (mod.rs:230). With the new trait signatures, `?` already propagates correctly. No hand-written bool-check patterns remain. df097c2
- [x] Task: Green Phase — apply the same review to `Devcontainer::rebuild` (mod.rs:301), `ensure_created` (mod.rs:325), and `post_create` (mod.rs:338). Confirm no early returns are missed. df097c2
- [x] Task: Refactor — no duplication found; no helper needed. df097c2 (redundant after trait change)
- [x] Task: Verify Coverage on the orchestration paths ≥ 80%. df097c2 (326 tests pass, all lifecycle paths covered)
- [x] Task: Pre-commit checks. df097c2
- [x] Task: Commit (`fix(devcontainers): stop lifecycle on first provider failure`). df097c2 (merged with Phase 1 commit)
- [x] Task: Projector — User Manual Verification 'Phase 2: Update Orchestration to Stop on First Failure' (Protocol in workflow.md) df097c2

---

## Phase 3: Error Message Quality and Audit [checkpoint: 6a10a70]

- [x] Task: Red Phase — add a test asserting that when `build` fails, the error message includes the step name (`build`), the engine binary (`docker`), and a non-empty captured stderr excerpt. 6a10a70
- [x] Task: Green Phase — wire `format_exec_error` into `run_and_check` (already done in Phase 1) and thread the step name through (e.g., via a thin wrapper `run_step(name, &mut cmd)`). 6a10a70
- [ ] Task: Manual smoke test — trigger Bug 1 (a `.devcontainer/devcontainer.json` with a relative `build.dockerfile`) on a real engine and confirm:
    - Exactly one error is printed.
    - The error names the missing Dockerfile.
    - No subsequent docker invocation occurs.
    - Document the captured stderr/stdout in this plan's "Verification Log" section before closing the phase.
- [x] Task: Verify Coverage. 6a10a70 (329 tests pass)
- [x] Task: Pre-commit checks. 6a10a70
- [x] Task: Commit (`feat(provider): include step name and stderr in lifecycle error`). 6a10a70
- [ ] Task: Projector — User Manual Verification 'Phase 3: Error Message Quality and Audit' (Protocol in workflow.md)

### Verification Log (filled in during Phase 3)

- _Pending — manual smoke test requires a running container engine._

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::let_underscore_must_use -D clippy::let_underscore_drop` is clean.
- Quality gates green at every commit.
- Manual verification recorded in a git note on the final checkpoint commit:
    - reproduce a failing build (e.g., reference a non-existent Dockerfile in `devcontainer.json`),
    - confirm one — and only one — error message is printed,
    - confirm exit status is non-zero.
