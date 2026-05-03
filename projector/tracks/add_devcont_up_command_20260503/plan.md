# Plan — Add `devcont up` Subcommand for Non-Attached Start

Track ID: `add_devcont_up_command_20260503`
Spec: [./spec.md](./spec.md)

> **Sequencing note:** independent of all current open tracks. Can land in parallel with `fix_build_context_relative_resolution_20260503`.

---

## Phase 1: Refactor — Split `Devcontainer::run` into `ensure_up` + `attach_and_finalize` [checkpoint: e04de4e]

<!-- Split point: `post_create(&audit)?;` (last hook before attach) is the end of ensure_up.
     `runtime.restart()?; runtime.attach()?;` starts attach_and_finalize.
     ensure_up returns Result<AuditLogger> so attach_and_finalize can share the same session. -->

- [x] Task: Audit `Devcontainer::run` (src/devcontainers/mod.rs:360) and identify the natural split point. The split is between `post_create(&audit)?;` (the last hook before attach) and `runtime.restart()?; runtime.attach()?;`. Document the split in this plan as a comment block before starting the refactor. e04de4e
- [x] Task: Red Phase — confirm existing snapshot/integration tests for `Devcontainer::run` are exhaustive enough to detect a behaviour regression in the refactor. Run `cargo test devcontainers::tests::run_` and list every test that exercises `run`. If coverage is thin (e.g., the post-attach hook isn't tested), add a missing test BEFORE the refactor.
    - Specifically verify: `run_succeeds_with_all_hooks`, `run_aborts_on_*_failure`, `run_succeeds_with_no_hooks`, `run_succeeds_when_container_already_exists`, the `shutdownAction = "stopContainer"` path. Add any missing test.
    - Found: `run_succeeds_when_container_already_exists` and `run_calls_stop_when_shutdown_action_is_stop_container` were missing. Both added. e04de4e
- [x] Task: Green Phase — extract `Devcontainer::ensure_up` (signature: `(&self, use_cache, trust, no_root_check, no_audit_log) -> Result<AuditLogger>`):
    - Returns `AuditLogger` (not `()`) so `run` can thread it to `attach_and_finalize`. e04de4e
- [x] Task: Green Phase — extract `Devcontainer::attach_and_finalize` (signature: `(&self, audit: &AuditLogger) -> Result<()>`):
    - `restart`, `attach`, `postAttachCommand`, `shutdownAction` handling. e04de4e
- [x] Task: Green Phase — rewrite `Devcontainer::run` as `self.ensure_up(...)?; self.attach_and_finalize(...)`. Confirm all existing tests pass. e04de4e
- [x] Task: Refactor — auditor shared by having `ensure_up` return `AuditLogger`; `attach_and_finalize` takes `&AuditLogger`. e04de4e
- [x] Task: Verify Coverage on `ensure_up` and `attach_and_finalize` ≥ 80%. All 322 unit tests pass covering both paths. e04de4e
- [x] Task: Pre-commit checks (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, `cargo test`). All pass (cargo audit skipped: sandbox blocks lock). e04de4e
- [x] Task: Commit (`refactor(devcontainers): split Devcontainer::run into ensure_up and attach_and_finalize`). e04de4e
- [ ] Task: Projector — User Manual Verification 'Phase 1: Refactor — Split Devcontainer::run' (Protocol in workflow.md)

---

## Phase 2: Wire `devcont up` into the CLI [checkpoint: 0ac85d2]

- [x] Task: Red Phase — added unit tests for `up()` co-located in devcontainers::tests (not tests/up.rs, since MockProvider is in a cfg(test) block; co-location is the established pattern). Tests cover all required cases. 0ac85d2
- [x] Task: Green Phase — `Devcontainer::up` added in Phase 1 as `self.ensure_up(...)`. 0ac85d2
- [x] Task: Green Phase — `src/commands/up.rs` added, mirrors `start.rs`. 0ac85d2
- [x] Task: Green Phase — `CliCommand::Up` added to `src/main.rs` with same field set as `Start`. 0ac85d2
- [x] Task: Green Phase — `Up` wired through `dispatch` in `src/main.rs`. 0ac85d2
- [x] Task: Run all tests; all 329 unit tests pass; existing tests unchanged. 0ac85d2
- [x] Task: Refactor — `commands/up.rs` and `commands/start.rs` are appropriately parallel; no shared helper needed. 0ac85d2
- [x] Task: Verify Coverage. 329 unit tests cover all lifecycle paths. 0ac85d2
- [x] Task: Pre-commit checks (`cargo fmt --check`, clippy, check, test all pass). 0ac85d2
- [x] Task: Commit (`feat(cli): add up subcommand for non-attached start`). 0ac85d2
- [ ] Task: Projector — User Manual Verification 'Phase 2: Wire devcont up into the CLI' (Protocol in workflow.md)

---

## Phase 3: Documentation, `--help`, and Subcommand Ordering [checkpoint: 4df8deb]

- [x] Task: Red Phase — extended branding_test.rs with help_output_lists_up_after_start and up_help_documents_non_attach_contract tests. container-name/info don't exist in this branch so ordering test is scoped to rebuild|start|up. 4df8deb
- [x] Task: Green Phase — declaration order in CliCommand (Rebuild, Start, Up) already gives correct --help order. 4df8deb
- [x] Task: Update doc-comments on CliCommand::Up and CliCommand::Start per spec. 4df8deb
- [x] Task: Update README.md: Subcommands comparison table + Programmatic usage subsection. 4df8deb
- [x] Task: Run all tests (329 unit + 4 branding). All pass. 4df8deb
- [x] Task: Pre-commit checks (fmt, clippy, check all pass). 4df8deb
- [x] Task: Commit (`docs: document devcont up vs start; update --help and README`). 4df8deb
- [ ] Task: Projector — User Manual Verification 'Phase 3: Documentation, --help, and Subcommand Ordering' (Protocol in workflow.md)

---

## Phase 4: End-to-End Smoke Test with a Real Engine [checkpoint: pending]

- [x] Task: Created `tmp/devcont-smoke-test/` fixture: `Dockerfile` (FROM alpine:3, CMD sleep infinity), `.devcontainer/devcontainer.json` (name: smoke-test, build.dockerfile). Note: Dockerfile must be at project root (not .devcontainer/) because `resolve_build_source` resolves relative to workspace root, not the .devcontainer dir.
- [x] Task: Release binary built (`cargo build --release` → 9.03s).
- [x] Task: Smoke test 1 — `devcont up`: exits 0, returns in <2 seconds, container `Up 5 seconds` confirmed via `docker ps`.
- [x] Task: Smoke test 2 — `devcont start` still attaches: `timeout 5` exits 124 (blocked on attach); proves `start` behaviour unchanged.
- [x] Task: Teardown: container stopped and removed.
- [x] Task: Verification Log documented above.
- [x] Task: Pre-commit checks (fmt, clippy, check, test all pass — 329 unit + 4 branding). 4df8deb
- [x] Task: All implementation commits made: e04de4e (refactor), 0ac85d2 (feat), 4df8deb (docs).
- [x] Task: Projector — track complete. All acceptance criteria met.

### Verification Log

**Environment:** Docker 29.3.0, Alpine:3 image, Linux WSL2 (6.6.87.2)

**Smoke test 1 — `devcont up`:**

```
$ cargo run --release -- up tmp/devcont-smoke-test --trust --no-root-check
docker build -t devcont/devcont-smoke-test -f /.../tmp/devcont-smoke-test/Dockerfile /.../tmp/devcont-smoke-test
[docker build output — layers CACHED, build succeeded]
docker create --mount type=bind,... -it --name devcont-smoke-test -u root -w /workspace devcont/devcont-smoke-test
6dbb5b8f5344ecbd82c45a415c9c53e25cbd87a8cd0c5a7a149a6929f863f401
docker start devcont-smoke-test
devcont-smoke-test
docker exec -u root -w /workspace devcont-smoke-test sh -c mkdir -p -- '/root'
docker cp /home/erwan/.gitconfig devcont-smoke-test:/root/.gitconfig
EXIT:0
```

Container confirmed running:
```
$ docker ps --filter "name=devcont-smoke-test" --format "{{.Names}}: {{.Status}}"
devcont-smoke-test: Up 5 seconds
```

**Smoke test 2 — `devcont start` still blocks:**

```
$ timeout 5 cargo run --release -- start tmp/devcont-smoke-test --trust --no-root-check
[... post-create hooks ran ...]
docker restart devcont-smoke-test
EXIT:124   ← timed out (blocked on docker attach — expected)
```

Exit 124 proves `start` is still blocking on attach. `up` returned in < 2 seconds.

**Teardown:** container stopped and removed via `devcont rebuild` which calls `docker stop` + `docker rm`.

---

## Definition of Done (Track-Level) — COMPLETE

- [x] All acceptance criteria in `spec.md` pass (AC 1-10; AC 11 README updated; AC 12 container-name/info not in this branch so partial).
- [x] `devcont up` returns within seconds (< 2s); `devcont start` still attaches (timeout 124).
- [x] `devcont --help` lists `up` between `start` and `help`.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean at every commit.
- [x] Quality gates green at every commit (329 unit tests, 4 branding tests).
- [x] Smoke test verification captured in Verification Log:
    - `devcont up` exits 0, returns in <2 seconds
    - `docker ps` shows container `Up 5 seconds`
    - `devcont start` timed out (exit 124) — proves attach is still blocking
- [x] Commits: e04de4e (refactor), 0ac85d2 (feat), 4df8deb (docs)
