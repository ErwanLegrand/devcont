# Plan — Fix `start` Orchestration Error Cascade

Track ID: `fix_start_orchestration_cascade_20260503`
Spec: [./spec.md](./spec.md)

> **Sequencing note:** this track should land **before** `fix_dockerfile_path_resolution_20260503`, so users hitting the dockerfile-path bug see one clear error instead of six.

---

## Phase 1: Reshape `run_and_check` and the `Provider` Trait

- [ ] Task: Red Phase — add a unit test in `provider/utils.rs::tests` that runs a command which exits with status 2 and asserts `run_and_check` returns `Err` whose message includes "exit code 2" (or the relevant `format_exec_error` mapping).
- [ ] Task: Red Phase — adjust the existing `run_and_check_returns_false_on_failure` test (utils.rs:738+) to assert `Err` instead of `Ok(false)`. Confirm tests fail.
- [ ] Task: Green Phase — change `run_and_check`'s signature to `-> io::Result<()>`. Capture stdout/stderr (via `Command::output` instead of `Command::status`); on non-zero status, return `Err(io::Error::other(format_exec_error(code, &stderr)))`.
- [ ] Task: Green Phase — change every `Provider::{build,create,start,stop,restart,attach,rm,cp}` signature in `provider/mod.rs` from `io::Result<bool>` to `io::Result<()>`. Update each impl (`docker.rs`, `docker_compose.rs`, `podman.rs`, `podman_compose.rs`, `nerdctl.rs`, `apple.rs`) to drop the bool. Keep `exists` and `running` as `io::Result<bool>` (genuine boolean facts).
- [ ] Task: Green Phase — update the test stub `Provider` impl at `devcontainers/mod.rs:804+` to match. Make the stub support per-method "fail next" toggles (one of `build`, `create`, `start`, `restart`, `attach`, `stop`) so we can write per-step abort tests.
- [ ] Task: Refactor — fold any `let _ = runtime.foo();` discards into proper `?` propagation. Run `cargo clippy -- -D clippy::let_underscore_must_use -D clippy::let_underscore_drop` and clean up findings.
- [ ] Task: Verify Coverage on `provider/utils.rs` and the trait impls.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`refactor(provider): make non-zero exit propagate as Err`).
- [ ] Task: Projector — User Manual Verification 'Phase 1: Reshape run_and_check and the Provider Trait' (Protocol in workflow.md)

---

## Phase 2: Update Orchestration to Stop on First Failure

- [ ] Task: Red Phase — add new tests under `devcontainers/mod.rs::tests` mirroring the existing hook-failure tests:
    - `run_aborts_on_build_failure`
    - `run_aborts_on_create_failure`
    - `run_aborts_on_start_failure`
    - `run_aborts_on_restart_failure`
    - `run_aborts_on_attach_failure`
    - `run_aborts_on_stop_failure`
    - Each test configures the stub `Provider` to fail at the named step and asserts:
        - `Devcontainer::run` returns `Err`,
        - the failure message names the failed step and includes the stub's stderr,
        - **no subsequent step is invoked** (the stub records call counts; assert later steps' counters are 0).
    - Run `cargo test run_aborts_on` and confirm failures.
- [ ] Task: Green Phase — review `Devcontainer::run` (mod.rs:230). With the new trait signatures, `?` already propagates correctly. Confirm tests pass without further code changes; if any hand-written `if !runtime.foo()? { ... }` patterns remain (none expected), simplify to `runtime.foo()?;`.
- [ ] Task: Green Phase — apply the same review to `Devcontainer::rebuild` (mod.rs:301), `ensure_created` (mod.rs:325), and `post_create` (mod.rs:338). Confirm no early returns are missed.
- [ ] Task: Refactor — if multiple call sites duplicate "wrap a step's error with the step name", introduce a small helper (e.g., `step("start", || runtime.start())`).
- [ ] Task: Verify Coverage on the orchestration paths ≥ 80%.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`fix(devcontainers): stop lifecycle on first provider failure`).
- [ ] Task: Projector — User Manual Verification 'Phase 2: Update Orchestration to Stop on First Failure' (Protocol in workflow.md)

---

## Phase 3: Error Message Quality and Audit

- [ ] Task: Red Phase — add a test asserting that when `build` fails, the error message includes the step name (`build`), the engine binary (`docker`), and a non-empty captured stderr excerpt.
- [ ] Task: Green Phase — wire `format_exec_error` into `run_and_check` (already done in Phase 1) and thread the step name through (e.g., via a thin wrapper `run_step(name, &mut cmd)`).
- [ ] Task: Manual smoke test — trigger Bug 1 (a `.devcontainer/devcontainer.json` with a relative `build.dockerfile`) on a real engine and confirm:
    - Exactly one error is printed.
    - The error names the missing Dockerfile.
    - No subsequent docker invocation occurs.
    - Document the captured stderr/stdout in this plan's "Verification Log" section before closing the phase.
- [ ] Task: Verify Coverage.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`feat(provider): include step name and stderr in lifecycle error`).
- [ ] Task: Projector — User Manual Verification 'Phase 3: Error Message Quality and Audit' (Protocol in workflow.md)

### Verification Log (filled in during Phase 3)

- _TBD_

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- `cargo clippy --workspace --all-targets -- -D warnings -D clippy::let_underscore_must_use -D clippy::let_underscore_drop` is clean.
- Quality gates green at every commit.
- Manual verification recorded in a git note on the final checkpoint commit:
    - reproduce a failing build (e.g., reference a non-existent Dockerfile in `devcontainer.json`),
    - confirm one — and only one — error message is printed,
    - confirm exit status is non-zero.
