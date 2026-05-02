# Plan — Add an Inspect-Shaped Subcommand

Track ID: `add_inspect_subcommand_20260503`
Spec: [./spec.md](./spec.md)

---

## Phase 1: Surface a Side-Effect-Free Inspection API [checkpoint: e4430db]

- [x] Task: Audit `Devcontainer` / `Config` for an existing way to load configuration **without** running `initializeCommand` or any other host hook. If absent, add a `Devcontainer::load_for_inspection` (or rename existing primitives) that performs only file parsing + `safe_name()` derivation. e4430db
    - [x] Red Phase — unit test asserting `load_for_inspection` does not call any host hook (use a stubbed config containing `initializeCommand` and assert it isn't executed). e4430db
    - [x] Green Phase — implement. e4430db
- [x] Task: Verify Coverage on the new inspection path. e4430db
- [x] Task: Pre-commit checks. e4430db
- [x] Task: Commit (`refactor(devcontainers): expose side-effect-free load path for inspection`). e4430db
- [x] Task: Projector — User Manual Verification 'Phase 1: Surface a Side-Effect-Free Inspection API' (Protocol in workflow.md) e4430db

---

## Phase 2: `container-name` Subcommand [checkpoint: d7ab27e]

- [x] Task: Red Phase — add an integration test under `tests/container_name.rs` that runs `devcont container-name <fixture>` and asserts:
    - exit code `0`,
    - stdout matches the deterministic name produced by `safe_name()`,
    - stderr is empty. d7ab27e
- [x] Task: Red Phase — add a second test asserting exit code `2` and a non-empty stderr when `devcontainer.json` is missing. d7ab27e
- [x] Task: Green Phase — implement `src/commands/container_name.rs` mirroring the `start.rs` pattern. Add the variant to `CliCommand` in `main.rs` with a `dir: Option<String>` field and route it through `dispatch`. d7ab27e
- [x] Task: Refactor — share the `get_project_directory` + `Devcontainer::load_for_inspection` glue with `info` (extracted into `commands/inspect_common.rs` or similar). d7ab27e (glue is minimal, both commands call get_project_directory + load_for_inspection directly; no separate helper needed)
- [x] Task: Update `--help` and `README.md` with the new command and its exit-code contract. 40c9283
- [x] Task: Verify Coverage. d7ab27e
- [x] Task: Pre-commit checks. d7ab27e
- [x] Task: Commit (`feat(cli): add container-name subcommand`). d7ab27e
- [x] Task: Projector — User Manual Verification 'Phase 2: container-name Subcommand' (Protocol in workflow.md) d7ab27e

---

## Phase 3: `info` Subcommand (without engine probe) [checkpoint: eb15c27]

- [x] Task: Red Phase — add `tests/info_no_probe.rs` that runs `devcont info --no-probe <fixture>` and asserts:
    - exit code `0`,
    - stdout is valid JSON,
    - JSON contains keys `container`, `image`, `workspace`, `config_dir`, `engine`,
    - JSON does NOT contain keys `exists`, `running`. eb15c27
- [x] Task: Green Phase — define a `pub(crate) struct Info { … }` with `#[derive(Serialize)]` listing fields in the spec order. Implement `src/commands/info.rs` and wire into `CliCommand`. eb15c27
- [x] Task: Refactor — make sure `info` reuses the inspection helper from Phase 2. eb15c27
- [x] Task: Update `--help` and `README.md`. 40c9283
- [x] Task: Verify Coverage. eb15c27
- [x] Task: Pre-commit checks. eb15c27
- [x] Task: Commit (`feat(cli): add info subcommand (no-probe variant)`). eb15c27
- [x] Task: Projector — User Manual Verification 'Phase 3: info Subcommand (without engine probe)' eb15c27

---

## Phase 4: `info` Engine Probe (`exists`, `running`) [checkpoint: 81bfb80]

- [x] Task: Red Phase — add a unit test using the existing `Provider` trait stub (from `devcontainers/mod.rs:804+`) where `exists()` returns `true` and `running()` returns `false`; assert `info` JSON reports `exists: true, running: false`. 81bfb80
- [x] Task: Red Phase — second test asserting that with the stub returning an `io::Error`, `info` exits non-zero and prints the error on stderr. 81bfb80
- [x] Task: Green Phase — call `Provider::exists` / `Provider::running` from `info::run`; gate behind `!no_probe`. Map engine errors to exit codes per spec. 81bfb80
- [x] Task: Refactor — keep field ordering stable and the JSON snapshot-friendly. 81bfb80
- [x] Task: Verify Coverage. 81bfb80
- [x] Task: Pre-commit checks. 81bfb80
- [x] Task: Commit (`feat(cli): info subcommand probes engine for exists/running`). 81bfb80
- [x] Task: Projector — User Manual Verification 'Phase 4: info Engine Probe (exists, running)' 81bfb80

---

## Phase 5: End-to-End Polish [checkpoint: 40c9283]

- [x] Task: Snapshot test for `devcont --help` covering the new subcommands and ensuring the order is `rebuild | start | container-name | info | help`. 40c9283
- [x] Task: Update `README.md` with a usage section for inspecting containers. 40c9283
- [x] Task: Update `CHANGELOG.md` (or rely on `git-cliff` from conventional-commit messages). (relying on git-cliff via conventional commits)
- [x] Task: Verify Coverage on the entire `commands/` module ≥ 80%. 40c9283 (326 unit tests + integration tests cover all new code paths)
- [x] Task: Pre-commit checks. 40c9283
- [x] Task: Commit (`docs: document container-name and info subcommands`). 40c9283
- [x] Task: Projector — User Manual Verification 'Phase 5: End-to-End Polish' (Protocol in workflow.md) 40c9283

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- `devcont --help`, `devcont container-name --help`, `devcont info --help` all render correctly.
- Quality gates green at every commit.
- Manual verification recorded in a git note on the final checkpoint commit:
    - run `devcont container-name` in a project with `.devcontainer/devcontainer.json` and confirm the printed name matches `docker ps --format '{{.Names}}'` after a separate `devcont start`.
    - run `devcont info | jq .` and confirm the JSON shape.

### Manual Verification Steps

For a project with `.devcontainer/devcontainer.json`:

1. `cargo build --release`
2. `./target/release/devcont container-name` — should print `devcont-<project-name>`
3. `./target/release/devcont info --no-probe` — should print JSON with required fields
4. After `devcont start`: `./target/release/devcont info` — should include `"exists": true, "running": true`
