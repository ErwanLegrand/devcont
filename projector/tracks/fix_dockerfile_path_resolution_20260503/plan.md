# Plan — Fix `build.dockerfile` Path Resolution

Track ID: `fix_dockerfile_path_resolution_20260503`
Spec: [./spec.md](./spec.md)

---

## Phase 1: Investigation & Audit [checkpoint: pending]

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

## Phase 2: Plumb `config_dir` Through `Devcontainer`

- [x] Task: Red Phase — add a unit test in `devcontainers/config.rs` (or sibling) asserting that `Config::load` (or whichever loader is used) returns/exposes the directory containing the loaded `devcontainer.json`. 3bcf4c5
- [x] Task: Green Phase — add a `config_dir: PathBuf` field (or method) on `Devcontainer`/`Config` populated from the loader. Treat it as the source of truth for path resolution downstream. 3bcf4c5
- [x] Task: Refactor — replace any ad-hoc `directory.join(".devcontainer")` with a single helper or by passing `config_dir`. 3bcf4c5
- [x] Task: Verify Coverage — all new code covered by tests added in red phase.
- [x] Task: Pre-commit checks — fmt, clippy, check, test all pass.
- [x] Task: Commit (`refactor(devcontainers): track config_dir for path resolution`). 3bcf4c5
- [x] Task: Projector — User Manual Verification 'Phase 2: Plumb config_dir Through Devcontainer' (Protocol in workflow.md) — verified via automated tests for nested and root layouts.

---

## Phase 3: Fix `resolve_dockerfile_path` and Its Tests

- [ ] Task: Red Phase — flip the existing tests at `provider/utils.rs:367–391` to encode the new (correct) behavior:
    - `resolve_dockerfile_path(config_dir = "/ws/.devcontainer", "Dockerfile", None)` → `/ws/.devcontainer/Dockerfile`
    - `resolve_dockerfile_path(config_dir = "/ws/.devcontainer", "Dockerfile", Some("subdir"))` → `/ws/.devcontainer/subdir/Dockerfile`
    - `resolve_dockerfile_path(config_dir, "Dockerfile", Some("/other/ctx"))` → `/other/ctx/Dockerfile`
    - `resolve_dockerfile_path(config_dir, "/abs/Dockerfile", Some("ctx"))` → `/abs/Dockerfile`
    - Add a fifth test for the `.devcontainer.json`-at-root layout (`config_dir = "/ws"`, dockerfile = "Dockerfile") → `/ws/Dockerfile`.
    - Run `cargo test resolve_dockerfile` and confirm the new tests fail.
- [ ] Task: Green Phase — change `resolve_dockerfile_path`'s first parameter from `workspace` to `config_dir` (and rename it). Update all callers in `devcontainers/mod.rs`. Confirm tests pass.
- [ ] Task: Refactor — colocate any helper used by both `resolve_dockerfile_path` and `resolve_build_context`.
- [ ] Task: Verify Coverage.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`fix(provider): resolve build.dockerfile relative to devcontainer.json`).
- [ ] Task: Projector — User Manual Verification 'Phase 3: Fix resolve_dockerfile_path and Its Tests' (Protocol in workflow.md)

---

## Phase 4: Fix `compose_path_and_service` and Sibling Callers

- [ ] Task: Red Phase — add tests covering both layouts:
    - `.devcontainer/devcontainer.json` with `"dockerComposeFile": "compose.yml"` resolves to `<ws>/.devcontainer/compose.yml`.
    - `.devcontainer.json` (root) with `"dockerComposeFile": "compose.yml"` resolves to `<ws>/compose.yml`.
    - Run and confirm at least the second test fails today.
- [ ] Task: Green Phase — replace `directory.join(".devcontainer").join(compose_file)` with `config_dir.join(compose_file)`. Audit `resolve_build_context` (mod.rs:542) and any compose-template path interpolation for the same change.
- [ ] Task: Refactor — extract any duplicated `config_dir`-relative resolution into a single helper.
- [ ] Task: Verify Coverage.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`fix(devcontainers): resolve dockerComposeFile and build.context relative to devcontainer.json`).
- [ ] Task: Projector — User Manual Verification 'Phase 4: Fix compose_path_and_service and Sibling Callers' (Protocol in workflow.md)

---

## Phase 5: End-to-End Regression Coverage

- [ ] Task: Red Phase — add an integration test under `tests/` that loads a fixture devcontainer with `build.dockerfile = "Dockerfile"` at `.devcontainer/devcontainer.json` and asserts the resolved Dockerfile path.
- [ ] Task: Red Phase — add a fixture for `.devcontainer.json` at workspace root with both `build.dockerfile` and `dockerComposeFile`, asserting both resolved paths.
- [ ] Task: Green Phase — adjust source if any path still mis-resolves; otherwise the tests pass on existing code.
- [ ] Task: Improve `format_exec_error` (or upstream Error variant) so a "file not found" Dockerfile error names the path we attempted, mentions both candidate layouts, and points the user at the spec.
- [ ] Task: Verify Coverage.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`test(devcontainers): regression coverage for path resolution`).
- [ ] Task: Projector — User Manual Verification 'Phase 5: End-to-End Regression Coverage' (Protocol in workflow.md)

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- Quality gates from `projector/workflow.md` are green at every commit.
- `CHANGELOG.md` (via git-cliff) reflects the fix as a `fix:` entry tagged for the next release.
- Manual verification recorded in a git note on the final checkpoint commit, including the exact `devcont rebuild` invocation against a real `.devcontainer/devcontainer.json` with a relative `build.dockerfile`.
