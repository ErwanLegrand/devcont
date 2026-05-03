# Plan — Investigate `tests/integration.rs` Failures

Track ID: `investigate_integration_test_failures_20260503`
Spec: [./spec.md](./spec.md)

---

## Phase 1: Enumerate and Capture Failures [checkpoint: cddfe67]

- [x] Task: Run `cargo test --workspace --test integration --no-fail-fast 2>&1 | tee tmp/integration_test_output.txt` (or stash output in this track directory's `findings.md` working notes — do not commit `tmp/`). cddfe67
- [x] Task: List every failing test name and its panic/assertion message in `findings.md` under section `## Failure Inventory`. One subsection per test, format:
    ```
    ### test_<name>
    - **File:line:** path/to/file.rs:NNN
    - **Panic/error:** <message>
    - **Stderr excerpt:** <relevant subprocess stderr if any>
    - **Hypothesis:** <one-line guess>
    ```
- [x] Task: List the 9 currently-passing tests for completeness, with a note explaining why they pass (no engine call, mocked, etc.).
- [x] Task: Pre-commit checks (no source changes expected; just doc additions).
- [x] Task: Commit (`docs(projector): enumerate tests/integration.rs failure inventory`).
- [x] Task: Projector — User Manual Verification 'Phase 1: Enumerate and Capture Failures' (Protocol in workflow.md)

---

## Phase 2: Classify and Decide Remediation [checkpoint: cddfe67]

- [x] Task: For each failing test in `findings.md`, classify by:
    - **Class:** `live-engine-required` / `network-required` / `podman-not-installed` / `misconfigured` / `genuinely-broken` / `brittle` / `platform-specific`.
    - **Decision:** `#[ignore]` / `fix-test` / `fix-source` / `cfg-gate`.
    - **Justification:** one sentence per decision.
- [x] Task: Add a `## Classification Summary` section to `findings.md`:
    - Table of test → class → decision.
    - Aggregate counts per class.
- [x] Task: Validate decisions against the spec's "Decision Criteria for Default-Run vs `#[ignore]`". Document any outliers.
- [x] Task: Pre-commit checks.
- [x] Task: Commit (`docs(projector): classify integration test failures and decide remediation`).
- [x] Task: Projector — User Manual Verification 'Phase 2: Classify and Decide Remediation' (Protocol in workflow.md)

---

## Phase 3: Apply `#[ignore]` and Fix Misconfigured Tests [checkpoint: 6750dc3]

- [x] Task: For each test classified `#[ignore]`:
    - Red Phase — add a doc-comment above the test explaining the prerequisite and how to run it (`cargo test --test integration -- --ignored test_<name>`). Then add `#[ignore = "requires <prerequisite>"]`.
    - Green Phase — re-run `cargo test --workspace --test integration` and confirm the test no longer fails (it should be in the "ignored" count, not the "failed" count).
  6750dc3
- [x] Task: For each test classified `fix-test`: No tests classified fix-test in this track. 6750dc3
- [x] Task: For each test classified `fix-source`: No tests classified fix-source in this track. 6750dc3
- [x] Task: For each test classified `cfg-gate`: No tests classified cfg-gate in this track. 6750dc3
- [x] Task: Run `cargo test --workspace` and confirm exit code 0 with no `FAILED` lines. Result: 0 failed, 32 ignored. 6750dc3
- [x] Task: Verify Coverage — unit tests in src/provider/* are unchanged; coverage not affected. 6750dc3
- [x] Task: Pre-commit checks (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, `cargo test`). 6750dc3
- [x] Task: Commit (`test(integration): gate live-engine tests with #[ignore]; fix misconfigured ones`). 6750dc3
- [x] Task: Projector — User Manual Verification 'Phase 3: Apply #[ignore] and Fix Misconfigured Tests' (Protocol in workflow.md) 6750dc3

---

## Phase 4: CI Job for `--ignored` Tests [YAML drafted; awaiting user review]

- [x] Task: Audit `.github/workflows/*.yml` for the existing test job. Identified: `test.yml` already installs podman but runs integration tests without `--ignored` (making the step a no-op). `ci.yml` runs `cargo test --all-targets` which also skips ignored tests. Both workflows need updating.
- [x] Task: Sketch a new job `tests-ignored` — drafted in `findings.md` under "Phase 4 — Proposed CI YAML". The job installs podman/podman-compose, pre-pulls alpine:latest for both Docker and Podman, and runs `cargo test --test integration -- --ignored --test-threads=1`.
- [x] Task: Open a draft of the YAML in `findings.md` and request user review BEFORE editing the actual workflow file. Draft written in findings.md.
- [ ] Task: Once approved, commit the YAML change with `ci: add --ignored integration tests job`. BLOCKED — awaiting user review per spec guardrail.
- [ ] Task: Pre-commit checks. BLOCKED — pending YAML approval.
- [ ] Task: Projector — User Manual Verification 'Phase 4: CI Job for --ignored Tests' (Protocol in workflow.md). BLOCKED — pending approval.

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
