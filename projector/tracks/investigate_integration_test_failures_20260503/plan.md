# Plan — Investigate `tests/integration.rs` Failures

Track ID: `investigate_integration_test_failures_20260503`
Spec: [./spec.md](./spec.md)

---

## Phase 1: Enumerate and Capture Failures

- [ ] Task: Run `cargo test --workspace --test integration --no-fail-fast 2>&1 | tee tmp/integration_test_output.txt` (or stash output in this track directory's `findings.md` working notes — do not commit `tmp/`).
- [ ] Task: List every failing test name and its panic/assertion message in `findings.md` under section `## Failure Inventory`. One subsection per test, format:
    ```
    ### test_<name>
    - **File:line:** path/to/file.rs:NNN
    - **Panic/error:** <message>
    - **Stderr excerpt:** <relevant subprocess stderr if any>
    - **Hypothesis:** <one-line guess>
    ```
- [ ] Task: List the 9 currently-passing tests for completeness, with a note explaining why they pass (no engine call, mocked, etc.).
- [ ] Task: Pre-commit checks (no source changes expected; just doc additions).
- [ ] Task: Commit (`docs(projector): enumerate tests/integration.rs failure inventory`).
- [ ] Task: Projector — User Manual Verification 'Phase 1: Enumerate and Capture Failures' (Protocol in workflow.md)

---

## Phase 2: Classify and Decide Remediation

- [ ] Task: For each failing test in `findings.md`, classify by:
    - **Class:** `live-engine-required` / `network-required` / `podman-not-installed` / `misconfigured` / `genuinely-broken` / `brittle` / `platform-specific`.
    - **Decision:** `#[ignore]` / `fix-test` / `fix-source` / `cfg-gate`.
    - **Justification:** one sentence per decision.
- [ ] Task: Add a `## Classification Summary` section to `findings.md`:
    - Table of test → class → decision.
    - Aggregate counts per class.
- [ ] Task: Validate decisions against the spec's "Decision Criteria for Default-Run vs `#[ignore]`". Document any outliers.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`docs(projector): classify integration test failures and decide remediation`).
- [ ] Task: Projector — User Manual Verification 'Phase 2: Classify and Decide Remediation' (Protocol in workflow.md)

---

## Phase 3: Apply `#[ignore]` and Fix Misconfigured Tests

- [ ] Task: For each test classified `#[ignore]`:
    - Red Phase — add a doc-comment above the test explaining the prerequisite and how to run it (`cargo test --test integration -- --ignored test_<name>`). Then add `#[ignore = "requires <prerequisite>"]`.
    - Green Phase — re-run `cargo test --workspace --test integration` and confirm the test no longer fails (it should be in the "ignored" count, not the "failed" count).
- [ ] Task: For each test classified `fix-test`:
    - Red Phase — confirm the test fails for the *expected* reason (not a different bug).
    - Green Phase — fix the test (path, env var, fixture, cleanup) and confirm it now passes.
    - Document the fix in `findings.md` under the test's subsection.
- [ ] Task: For each test classified `fix-source`:
    - Open a NEW track (`fix_<symptom>_<YYYYMMDD>`) and link it from `findings.md`. **Do not fix source bugs in this investigation track** — investigation and remediation should land separately so the diagnostic is auditable.
- [ ] Task: For each test classified `cfg-gate`:
    - Add the appropriate `#[cfg(...)]` attribute (typically `#[cfg(target_os = "linux")]` or similar).
- [ ] Task: Run `cargo test --workspace` and confirm exit code 0 with no `FAILED` lines.
- [ ] Task: Verify Coverage — coverage should not drop measurably for the source modules under `src/provider/*` since unit tests cover them.
- [ ] Task: Pre-commit checks (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, `cargo test`).
- [ ] Task: Commit (`test(integration): gate live-engine tests with #[ignore]; fix misconfigured ones`).
- [ ] Task: Projector — User Manual Verification 'Phase 3: Apply #[ignore] and Fix Misconfigured Tests' (Protocol in workflow.md)

---

## Phase 4: CI Job for `--ignored` Tests

- [ ] Task: Audit `.github/workflows/*.yml` for the existing test job. Identify where to plug in a new job (probably alongside the existing one, conditional on `docker` install).
- [ ] Task: Sketch a new job `tests-ignored` (or extend existing) that:
    - Installs `docker` (already common in `ubuntu-latest`).
    - Pulls one tiny image (e.g., `alpine:3`) before the run, to satisfy live-engine tests.
    - Runs `cargo test --workspace --test integration -- --ignored`.
    - Marks the job as `continue-on-error: false` for `docker` tests; consider `continue-on-error: true` for `podman` tests in environments where rootless podman isn't reliably set up.
    - Uploads test output as an artifact for diagnostic.
- [ ] Task: Open a draft of the YAML in `findings.md` and request user review BEFORE editing the actual workflow file. The user signs off, then we commit.
- [ ] Task: Once approved, commit the YAML change with `ci: add --ignored integration tests job`.
- [ ] Task: Pre-commit checks.
- [ ] Task: Projector — User Manual Verification 'Phase 4: CI Job for --ignored Tests' (Protocol in workflow.md)

---

## Phase 5: End-to-End Verification

- [ ] Task: From a clean checkout, run `cargo test --workspace`. Confirm: exit 0, no failures, with N `ignored` (where N matches the count in `findings.md`).
- [ ] Task: With docker running locally and an image preloaded (`docker pull alpine:3`), run `cargo test --workspace --test integration -- --ignored`. Confirm: all docker-based ignored tests pass.
- [ ] Task: Confirm `podman`-class tests still report `ignored` or `cfg`-skipped on the dev machine where podman is not configured.
- [ ] Task: Update `findings.md` with the final tallies and link the CI job's first successful run.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`docs(projector): close investigate_integration_test_failures track`).
- [ ] Task: Projector — User Manual Verification 'Phase 5: End-to-End Verification' (Protocol in workflow.md)

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- `cargo test --workspace` exits 0 on a developer machine without internet and without pre-pulled images.
- `findings.md` lives under this track's directory and documents every test's class, decision, and current state.
- Each `#[ignore]`-gated test carries a self-explanatory `#[ignore = "..."]` reason and a doc-comment with run instructions.
- CI has a separate `--ignored` job that runs green at least once.
- Any `fix-source`-classified failure has its own follow-up track with a link in `findings.md`.
- Quality gates green at every commit.
- Manual verification recorded in a git note on the final checkpoint commit.
