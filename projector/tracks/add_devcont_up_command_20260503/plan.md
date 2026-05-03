# Plan — Add `devcont up` Subcommand for Non-Attached Start

Track ID: `add_devcont_up_command_20260503`
Spec: [./spec.md](./spec.md)

> **Sequencing note:** independent of all current open tracks. Can land in parallel with `fix_build_context_relative_resolution_20260503`.

---

## Phase 1: Refactor — Split `Devcontainer::run` into `ensure_up` + `attach_and_finalize`

- [ ] Task: Audit `Devcontainer::run` (src/devcontainers/mod.rs:360) and identify the natural split point. The split is between `post_create(&audit)?;` (the last hook before attach) and `runtime.restart()?; runtime.attach()?;`. Document the split in this plan as a comment block before starting the refactor.
- [ ] Task: Red Phase — confirm existing snapshot/integration tests for `Devcontainer::run` are exhaustive enough to detect a behaviour regression in the refactor. Run `cargo test devcontainers::tests::run_` and list every test that exercises `run`. If coverage is thin (e.g., the post-attach hook isn't tested), add a missing test BEFORE the refactor.
    - Specifically verify: `run_succeeds_with_all_hooks`, `run_aborts_on_*_failure`, `run_succeeds_with_no_hooks`, `run_succeeds_when_container_already_exists`, the `shutdownAction = "stopContainer"` path. Add any missing test.
- [ ] Task: Green Phase — extract `Devcontainer::ensure_up` (signature: `(&self, use_cache, trust, no_root_check, no_audit_log) -> Result<()>`):
    - `validate_run_args` / `safe_name` / `validate_remote_env` checks (currently at the top of `run`)
    - `AuditLogger::new` + `ContainerStart` event log
    - `initializeCommand` host hook with confirmation
    - `ensure_created`
    - root warning
    - `start()` if not running
    - `postStartCommand`
    - `post_create` (onCreate, updateContent, postCreate, dotfiles)
    - **Final assertion:** call `runtime.running()?`; if `false`, return `Err(Error::ContainerNotRunning(name))` (add the variant to `error.rs` if absent). Documents the "container exited between start and return" failure mode from the spec's risks section.
- [ ] Task: Green Phase — extract `Devcontainer::attach_and_finalize` (signature: `(&self, audit: &AuditLogger, no_audit_log: bool) -> Result<()>`):
    - `restart`, `attach`
    - `postAttachCommand`
    - `shutdownAction` handling.
- [ ] Task: Green Phase — rewrite `Devcontainer::run` as `self.ensure_up(...)?; self.attach_and_finalize(...)`. Confirm all existing tests pass.
- [ ] Task: Refactor — if the auditor needs to be shared between the two, plumb it through (probably via taking `&AuditLogger` as parameter to `attach_and_finalize`).
- [ ] Task: Verify Coverage on `ensure_up` and `attach_and_finalize` ≥ 80%.
- [ ] Task: Pre-commit checks (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, `cargo test`).
- [ ] Task: Commit (`refactor(devcontainers): split Devcontainer::run into ensure_up and attach_and_finalize`).
- [ ] Task: Projector — User Manual Verification 'Phase 1: Refactor — Split Devcontainer::run' (Protocol in workflow.md)

---

## Phase 2: Wire `devcont up` into the CLI

- [ ] Task: Red Phase — add `tests/up.rs` (integration test) with a fixture that uses `MockProvider` via the existing `make_devcontainer_with_provider` helper. Cases:
    - `up_calls_build_create_start_when_container_absent`: assert `MockProvider` recorded `build`, `create`, `start` calls; did NOT record `attach`, `restart`, `stop`.
    - `up_skips_build_create_when_container_exists`: with `MockProvider::with_existing()`, assert `start` was called but not `build`, `create`, `attach`, `restart`, `stop`.
    - `up_does_not_call_stop_even_with_shutdownAction`: config has `shutdownAction = "stopContainer"`; assert `stop` was NOT called.
    - `up_runs_post_create_hooks`: config has `postCreateCommand`; assert it was dispatched.
    - `up_does_not_run_post_attach_hook`: config has `postAttachCommand`; assert it was NOT dispatched.
    - `up_returns_err_if_container_not_running_after_start`: configure `MockProvider` so `running()` returns `false` after `start()`; assert `up` returns `Err(Error::ContainerNotRunning)`.
    - Run and confirm all tests fail (the `up` method doesn't exist yet).
- [ ] Task: Green Phase — add `Devcontainer::up`. It's `self.ensure_up(...)`.
- [ ] Task: Green Phase — add `src/commands/up.rs` mirroring `src/commands/start.rs`:
    ```rust
    pub fn run(
        dir: Option<&str>,
        trust: bool,
        no_root_check: bool,
        no_audit_log: bool,
        hook_timeout: Option<u32>,
    ) -> std::io::Result<()> {
        let directory = super::get_project_directory(dir)?;
        let mut dc = Devcontainer::load(&directory)?;
        if let Some(secs) = hook_timeout {
            dc = dc.with_hook_timeout(secs);
        }
        dc.up(true, trust, no_root_check, no_audit_log)?;
        Ok(())
    }
    ```
- [ ] Task: Green Phase — add `Up { … }` variant to `CliCommand` in `src/main.rs`. Same field set as `Start { dir, trust, no_root_check, no_audit_log, hook_timeout }` (no `--cache` since we always use cache for `up`; rebuilds go through `rebuild`).
- [ ] Task: Green Phase — wire `Up` through `dispatch` in `src/main.rs`, calling `commands::up::run(...)`.
- [ ] Task: Run all tests; confirm Phase 2 tests pass and existing tests still pass.
- [ ] Task: Refactor — share helper modules between `commands/up.rs` and `commands/start.rs` if duplication grows; otherwise keep them parallel.
- [ ] Task: Verify Coverage.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`feat(cli): add up subcommand for non-attached start`).
- [ ] Task: Projector — User Manual Verification 'Phase 2: Wire devcont up into the CLI' (Protocol in workflow.md)

---

## Phase 3: Documentation, `--help`, and Subcommand Ordering

- [ ] Task: Red Phase — extend the existing `devcont --help` snapshot test to assert the subcommand order is: `rebuild | start | up | container-name | info | help`.
    - Run and confirm fail.
- [ ] Task: Green Phase — set the order via clap attributes (declaration order in `CliCommand` defines `--help` order).
- [ ] Task: Update doc-comments on `CliCommand::Up` and `CliCommand::Start`:
    - `Up`: "Bring the dev container up and return without attaching. Suitable for scripts."
    - `Start` long-about: "Launch (or re-attach to) the dev container with an interactive shell. Use `up` for non-interactive callers."
- [ ] Task: Update `README.md`:
    - Add a "Subcommands" comparison section (markdown table) with rows for `rebuild`, `start`, `up`, `container-name`, `info`, columns: `purpose`, `attaches?`, `runs hooks?`, `honours shutdownAction?`.
    - Add a "Programmatic usage" subsection showing the `devcont up && devcont info | jq .container` pattern.
- [ ] Task: Run all tests including snapshot tests.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`docs: document devcont up vs start; update --help and README`).
- [ ] Task: Projector — User Manual Verification 'Phase 3: Documentation, --help, and Subcommand Ordering' (Protocol in workflow.md)

---

## Phase 4: End-to-End Smoke Test with a Real Engine

- [ ] Task: Reuse the fixture at `tmp/devcont-smoke-test/` from the cascade-track Verification Log, but add an actual `Dockerfile`:
    ```dockerfile
    FROM alpine:3
    CMD ["sleep", "infinity"]
    ```
    so the build succeeds and the container stays running.
- [ ] Task: Build the release binary (`cargo build --release`).
- [ ] Task: Smoke test 1 — `devcont up`:
    - Run `time ./target/release/devcont up tmp/devcont-smoke-test` and capture stdout/stderr/exit.
    - Confirm: exits 0, returns within seconds (not blocking on attach), final stderr line confirms hooks completed if any.
    - Confirm container is running: `docker ps --filter name=devcont/devcont-smoke-test --format '{{.Status}}'` should show `Up`.
    - Run `docker exec $(./target/release/devcont container-name tmp/devcont-smoke-test) echo hello` — confirm it prints `hello`.
- [ ] Task: Smoke test 2 — verify `devcont start` still attaches (briefly):
    - Run `./target/release/devcont start tmp/devcont-smoke-test` in a subshell with a timeout (e.g., `timeout 5 ./target/release/devcont start ...`).
    - Confirm: exits non-zero (timeout) because attach was blocking. This is expected — it proves `start` still attaches.
    - Tear down: `docker stop $(./target/release/devcont container-name tmp/devcont-smoke-test) && docker rm $_`.
- [ ] Task: Document captured output in this plan's "Verification Log" section.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`projector(plan): record devcont up smoke test against real engine`).
- [ ] Task: Projector — User Manual Verification 'Phase 4: End-to-End Smoke Test with a Real Engine' (Protocol in workflow.md)

### Verification Log (filled in during Phase 4)

- _Pending — fill with `docker ps`, `time` output, and `docker exec` output on completion._

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- `devcont up` returns within seconds against a running container; `devcont start` still attaches.
- `devcont --help` lists `up` between `start` and `container-name`, in that order.
- `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- Quality gates green at every commit.
- Manual verification recorded in a git note on the final checkpoint commit:
    - `devcont up` returns,
    - `docker ps` shows the container running,
    - `docker exec` works,
    - `devcont start` still attaches.
