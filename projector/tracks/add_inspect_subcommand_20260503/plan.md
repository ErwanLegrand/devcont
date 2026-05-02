# Plan — Add an Inspect-Shaped Subcommand

Track ID: `add_inspect_subcommand_20260503`
Spec: [./spec.md](./spec.md)

---

## Phase 1: Surface a Side-Effect-Free Inspection API

- [ ] Task: Audit `Devcontainer` / `Config` for an existing way to load configuration **without** running `initializeCommand` or any other host hook. If absent, add a `Devcontainer::load_for_inspection` (or rename existing primitives) that performs only file parsing + `safe_name()` derivation.
    - [ ] Red Phase — unit test asserting `load_for_inspection` does not call any host hook (use a stubbed config containing `initializeCommand` and assert it isn't executed).
    - [ ] Green Phase — implement.
- [ ] Task: Verify Coverage on the new inspection path.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`refactor(devcontainers): expose side-effect-free load path for inspection`).
- [ ] Task: Projector — User Manual Verification 'Phase 1: Surface a Side-Effect-Free Inspection API' (Protocol in workflow.md)

---

## Phase 2: `container-name` Subcommand

- [ ] Task: Red Phase — add an integration test under `tests/container_name.rs` that runs `devcont container-name <fixture>` and asserts:
    - exit code `0`,
    - stdout matches the deterministic name produced by `safe_name()`,
    - stderr is empty.
- [ ] Task: Red Phase — add a second test asserting exit code `2` and a non-empty stderr when `devcontainer.json` is missing.
- [ ] Task: Green Phase — implement `src/commands/container_name.rs` mirroring the `start.rs` pattern. Add the variant to `CliCommand` in `main.rs` with a `dir: Option<String>` field and route it through `dispatch`.
- [ ] Task: Refactor — share the `get_project_directory` + `Devcontainer::load_for_inspection` glue with `info` (extracted into `commands/inspect_common.rs` or similar).
- [ ] Task: Update `--help` and `README.md` with the new command and its exit-code contract.
- [ ] Task: Verify Coverage.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`feat(cli): add container-name subcommand`).
- [ ] Task: Projector — User Manual Verification 'Phase 2: container-name Subcommand' (Protocol in workflow.md)

---

## Phase 3: `info` Subcommand (without engine probe)

- [ ] Task: Red Phase — add `tests/info_no_probe.rs` that runs `devcont info --no-probe <fixture>` and asserts:
    - exit code `0`,
    - stdout is valid JSON,
    - JSON contains keys `container`, `image`, `workspace`, `config_dir`, `engine`,
    - JSON does NOT contain keys `exists`, `running`.
- [ ] Task: Green Phase — define a `pub(crate) struct Info { … }` with `#[derive(Serialize)]` listing fields in the spec order. Implement `src/commands/info.rs` and wire into `CliCommand`.
- [ ] Task: Refactor — make sure `info` reuses the inspection helper from Phase 2.
- [ ] Task: Update `--help` and `README.md`.
- [ ] Task: Verify Coverage.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`feat(cli): add info subcommand (no-probe variant)`).
- [ ] Task: Projector — User Manual Verification 'Phase 3: info Subcommand (without engine probe)' (Protocol in workflow.md)

---

## Phase 4: `info` Engine Probe (`exists`, `running`)

- [ ] Task: Red Phase — add a unit test using the existing `Provider` trait stub (from `devcontainers/mod.rs:804+`) where `exists()` returns `true` and `running()` returns `false`; assert `info` JSON reports `exists: true, running: false`.
- [ ] Task: Red Phase — second test asserting that with the stub returning an `io::Error`, `info` exits non-zero and prints the error on stderr.
- [ ] Task: Green Phase — call `Provider::exists` / `Provider::running` from `info::run`; gate behind `!no_probe`. Map engine errors to exit codes per spec.
- [ ] Task: Refactor — keep field ordering stable and the JSON snapshot-friendly.
- [ ] Task: Verify Coverage.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`feat(cli): info subcommand probes engine for exists/running`).
- [ ] Task: Projector — User Manual Verification 'Phase 4: info Engine Probe (exists, running)' (Protocol in workflow.md)

---

## Phase 5: End-to-End Polish

- [ ] Task: Snapshot test for `devcont --help` covering the new subcommands and ensuring the order is `rebuild | start | container-name | info | help`.
- [ ] Task: Update `README.md` with a usage section for inspecting containers.
- [ ] Task: Update `CHANGELOG.md` (or rely on `git-cliff` from conventional-commit messages).
- [ ] Task: Verify Coverage on the entire `commands/` module ≥ 80%.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`docs: document container-name and info subcommands`).
- [ ] Task: Projector — User Manual Verification 'Phase 5: End-to-End Polish' (Protocol in workflow.md)

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- `devcont --help`, `devcont container-name --help`, `devcont info --help` all render correctly.
- Quality gates green at every commit.
- Manual verification recorded in a git note on the final checkpoint commit:
    - run `devcont container-name` in a project with `.devcontainer/devcontainer.json` and confirm the printed name matches `docker ps --format '{{.Names}}'` after a separate `devcont start`.
    - run `devcont info | jq .` and confirm the JSON shape.
