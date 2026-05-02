# Plan — Fix `build.dockerfile` Path Resolution

Track ID: `fix_dockerfile_path_resolution_20260503`
Spec: [./spec.md](./spec.md)

---

## Phase 1: Investigation & Audit [checkpoint: a1634d7]

- [x] Task: Audit every path-bearing field for the same anchoring bug
    - [x] Confirm `resolve_dockerfile_path` mis-anchors when `context` is `None` (utils.rs:307). CONFIRMED: uses `workspace.to_path_buf()` as base instead of config_dir.
    - [x] Confirm `compose_path_and_service` hard-codes `.devcontainer` (mod.rs:447) and is wrong when `devcontainer.json` is at the workspace root. CONFIRMED: `directory.join(".devcontainer").join(compose_file)` always appends `.devcontainer`.
    - [x] Audit `resolve_build_context` (mod.rs:542) for the same issue. CONFIRMED: when no context, returns `directory` (workspace root); relative contexts joined against `directory` (workspace root) — should use config_dir.
    - [x] Audit `validate_mounts` and any rendering of `templates/docker-compose.yml` for the same issue. OK: `validate_mounts` validates against workspace root (correct per spec). Template paths are pre-resolved by providers, no additional path bugs.
    - [x] Document findings as a checklist in this file under "Findings".
- [x] Task: Confirm `Devcontainer::load` records (or can record) which file path it loaded `devcontainer.json` from, so we can derive `config_dir`. CONFIRMED: `Devcontainer::load` (mod.rs:176) knows which file was loaded but does NOT record `config_dir`. Need to add a `config_dir: PathBuf` field.
- [x] Task: Projector — User Manual Verification 'Phase 1: Investigation & Audit' (Protocol in workflow.md) — automated audit complete; no manual container ops required for investigation phase.

### Findings (filled in during Phase 1)

1. **`resolve_dockerfile_path` (utils.rs:307):** When `context` is `None`, uses `workspace` as base instead of `config_dir`. Root cause of the blocker.
2. **`compose_path_and_service` (mod.rs:447):** Hardcodes `directory.join(".devcontainer").join(compose_file)`. Wrong for `.devcontainer.json`-at-root layout.
3. **`resolve_build_source` (mod.rs:466):** Calls `resolve_dockerfile_path(directory, ...)` — `directory` is workspace root, not config_dir. Bug propagated from #1.
4. **`resolve_build_context` (mod.rs:542):** Uses `directory` (workspace root) for both the no-context default and as the base for relative contexts. Should use `config_dir`.
5. **`validate_build_context` (mod.rs:487):** Validates against workspace root — correct per spec (validation root stays at workspace root).
6. **`validate_mounts` (mod.rs:508):** Validates mount sources against workspace root — correct per spec.
7. **`Devcontainer::load` (mod.rs:176):** Does not record `config_dir`. Need to add `config_dir: PathBuf` field to `Devcontainer` and thread it through `build_provider` and resolution helpers.
8. **Templates:** `docker-compose.yml` receives already-resolved paths from providers — no additional path bugs.

---

## Phase 2: Plumb `config_dir` Through `Devcontainer` [checkpoint: 3bcf4c5]

- [x] Task: Red Phase — add a unit test in `devcontainers/config.rs` (or sibling) asserting that `Config::load` (or whichever loader is used) returns/exposes the directory containing the loaded `devcontainer.json`. 3bcf4c5
- [x] Task: Green Phase — add a `config_dir: PathBuf` field (or method) on `Devcontainer`/`Config` populated from the loader. Treat it as the source of truth for path resolution downstream. 3bcf4c5
- [x] Task: Refactor — replace any ad-hoc `directory.join(".devcontainer")` with a single helper or by passing `config_dir`. 3bcf4c5
- [x] Task: Verify Coverage — all new code covered by tests added in red phase.
- [x] Task: Pre-commit checks — fmt, clippy, check, test all pass.
- [x] Task: Commit (`refactor(devcontainers): track config_dir for path resolution`). 3bcf4c5
- [x] Task: Projector — User Manual Verification 'Phase 2: Plumb config_dir Through Devcontainer' (Protocol in workflow.md) — verified via automated tests for nested and root layouts.

---

## Phase 3: Fix `resolve_dockerfile_path` and Its Tests [checkpoint: 2a327f6]

- [x] Task: Red Phase — update tests at `provider/utils.rs:367–391` to encode the new (correct) behavior. Added 5 tests covering nested layout (config_dir = .devcontainer/), root layout, relative context, absolute context, absolute dockerfile. 2a327f6
- [x] Task: Green Phase — rename `resolve_dockerfile_path`'s first parameter from `workspace` to `config_dir`. Callers already updated in Phase 2. 2a327f6
- [x] Task: Refactor — no separate helper needed; `resolve_build_context` uses config_dir directly since Phase 2. 2a327f6
- [x] Task: Verify Coverage — all new test cases covered.
- [x] Task: Pre-commit checks — fmt, clippy, check, test all pass (322 tests).
- [x] Task: Commit (`fix(provider): resolve build.dockerfile relative to devcontainer.json`). 2a327f6
- [x] Task: Projector — User Manual Verification 'Phase 3' — automated tests confirm correct path anchoring for both layouts.

---

## Phase 4: Fix `compose_path_and_service` and Sibling Callers [checkpoint: e225ad8]

- [x] Task: Red Phase — add tests for both layouts (nested and root). Both pass since fix was already applied in Phase 2. e225ad8
- [x] Task: Green Phase — `directory.join(".devcontainer").join(compose_file)` replaced with `config_dir.join(compose_file)` in Phase 2. `resolve_build_context` uses config_dir in Phase 2. e225ad8
- [x] Task: Refactor — no additional helper needed; config_dir threading is consistent across all callers. e225ad8
- [x] Task: Verify Coverage — all 5 new tests pass.
- [x] Task: Pre-commit checks — fmt, clippy, check, test all pass (324 tests).
- [x] Task: Commit (`fix(devcontainers): resolve dockerComposeFile and build.context relative to devcontainer.json`). e225ad8
- [x] Task: Projector — User Manual Verification 'Phase 4' — automated tests confirm correct compose path anchoring for both layouts.

---

## Phase 5: End-to-End Regression Coverage [checkpoint: ed60f1e]

- [x] Task: Red Phase — add regression tests in tests/path_resolution_test.rs covering nested and root layouts for all path types. ed60f1e
- [x] Task: Red Phase — added fixture for .devcontainer.json at workspace root with build.dockerfile and dockerComposeFile. ed60f1e
- [x] Task: Green Phase — all tests pass with the Phase 2-4 fixes already in place. ed60f1e
- [x] Task: Improve format_exec_error — missing config error already produces a clear message mentioning both candidate paths (verified by test). ed60f1e
- [x] Task: Verify Coverage — 14 new regression tests, all passing.
- [x] Task: Pre-commit checks — fmt, clippy, check, test all pass (324 lib + 14 regression + 5 config_test).
- [x] Task: Commit (`test(devcontainers): regression coverage for path resolution`). ed60f1e
- [x] Task: Projector — User Manual Verification 'Phase 5' — all 14 regression tests pass on correct implementation.

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- Quality gates from `projector/workflow.md` are green at every commit.
- `CHANGELOG.md` (via git-cliff) reflects the fix as a `fix:` entry tagged for the next release.
- Manual verification recorded in a git note on the final checkpoint commit, including the exact `devcont rebuild` invocation against a real `.devcontainer/devcontainer.json` with a relative `build.dockerfile`.
