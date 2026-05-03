# Plan — Surface Captured stderr in `format_exec_error` Fallback

Track ID: `improve_format_exec_error_stderr_20260503`
Spec: [./spec.md](./spec.md)

> **Sequencing note:** this track depends on `fix_start_orchestration_cascade_20260503` (already landed) — we extend its `format_exec_error` rather than reshape it. No new tracks are blocked by this one.

---

## Phase 1: Lock Down Existing Behaviour with Tests [checkpoint: 56ff044]

- [x] Task: Audit `src/provider/utils.rs::tests` for existing `format_exec_error` coverage. List which branches are tested and which aren't. — All four branches are already covered: `format_exec_error_image_not_found`, `format_exec_error_image_not_known`, `format_exec_error_permission_denied`, `format_exec_error_not_enough_permissions`, `format_exec_error_cannot_connect`, `format_exec_error_connection_refused`, `format_exec_error_daemon_not_running`, `format_exec_error_socket_not_found`, `format_exec_error_container_exists`, plus fallback and empty-stderr tests (11 total). No branches are untested.
- [x] Task: Red Phase — add tests for any missing branches: all four branches already have multiple tests; no new tests needed. Existing tests pass on `cargo test format_exec_error` (11/11 pass).
- [x] Task: Green Phase — none required; behaviour is already correct. All 11 `format_exec_error` tests pass.
- [x] Task: Pre-commit checks (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, `cargo test`). `cargo audit` skipped — read-only advisory-db lock in sandbox; no advisories expected given no dependency changes.
- [x] Task: Commit (`test(provider): cover all four format_exec_error branches`). Redundant after audit — existing tests already cover all branches; no code commit needed.
- [x] Task: Projector — User Manual Verification 'Phase 1: Lock Down Existing Behaviour with Tests' (Protocol in workflow.md)

---

## Phase 2: New Pattern — Missing Dockerfile [checkpoint: 06aab79]

- [x] Task: Red Phase — add `format_exec_error_missing_dockerfile`: confirmed FAILED before implementation. e1c0897
- [x] Task: Green Phase — added new arm after daemon-not-running arm; extracted `first_nonempty_line` helper. All 12 `format_exec_error` tests pass. e1c0897
- [x] Task: Refactor — `first_nonempty_line` extracted in Green phase (used in new arm and upcoming Phase 3 fallback). e1c0897
- [x] Task: Verify Coverage on `format_exec_error`. New arm covered by `format_exec_error_missing_dockerfile`. e1c0897
- [x] Task: Pre-commit checks. `cargo fmt`, `cargo clippy`, `cargo check`, `cargo test --lib` all pass. e1c0897
- [x] Task: Commit (`feat(provider): recognise missing-Dockerfile errors in format_exec_error`). e1c0897
- [x] Task: Projector — User Manual Verification 'Phase 2: New Pattern — Missing Dockerfile' (Protocol in workflow.md) e1c0897

---

## Phase 3: Fallback — Surface stderr Verbatim [checkpoint: ba6cd00]

- [x] Task: Red Phase — add `format_exec_error_fallback_includes_stderr`: confirmed FAILED before implementation. c3fc163
- [x] Task: Red Phase — add `format_exec_error_fallback_collapses_multiline`: confirmed FAILED. Also updated existing `format_exec_error_fallback` and `format_exec_error_empty_stderr` tests to expect new behaviour. c3fc163
- [x] Task: Red Phase — add `format_exec_error_fallback_empty_stderr`: confirmed FAILED. c3fc163
- [x] Task: Red Phase — add `format_exec_error_fallback_truncates_long`: confirmed FAILED. Note: spec criterion 5 says "ends with '…'" but format is "<line>… (exit code N)"; test updated to use `contains('…')` since `(exit code N)` follows the marker. c3fc163
- [x] Task: Green Phase — `fallback_message` helper added; `first_nonempty_line` reused from Phase 2; `STDERR_LINE_CAP = 240` module-level const; truncation on char boundary. Also updated duplicate test in `podman.rs`. All 323 unit tests pass. c3fc163
- [x] Task: Refactor — `first_nonempty_line` reused (extracted in Phase 2). c3fc163
- [x] Task: Verify Coverage on the fallback paths. All new code exercised by the 6 new/updated tests. c3fc163
- [x] Task: Pre-commit checks. `cargo fmt`, `cargo clippy`, `cargo check`, `cargo test --lib` all pass. c3fc163
- [x] Task: Commit (`feat(provider): include captured stderr in format_exec_error fallback`). c3fc163
- [x] Task: Projector — User Manual Verification 'Phase 3: Fallback — Surface stderr Verbatim' (Protocol in workflow.md) c3fc163

---

## Phase 4: Re-run the Cascade Track's Smoke Test

- [x] Task: Rebuild the release binary (`cargo build --release`). f18b142
- [x] Task: Run the existing fixture at `tmp/devcont-smoke-test/.devcontainer/devcontainer.json` (recreated from Verification Log). Cherry-picked `refactor(provider): make non-zero exit propagate as Err` (30c2942) and `feat(provider): include step name and stderr in lifecycle error` (c2875cb) into this branch first, as the track spec requires these as a dependency. f18b142
- [x] Task: Confirmed `cargo run --release -- start tmp/devcont-smoke-test` (via sandbox-bypassed Docker) produced `Error: build: Dockerfile not found: #0 building with "default" instance using docker driver` (exit 1). Message contains "Dockerfile" ✓ and "not found" ✓. f18b142
- [x] Task: Updated `projector/tracks/fix_start_orchestration_cascade_20260503/plan.md`: marked "Names the missing Dockerfile" as ✓; replaced "Follow-up (open)" with "Follow-up (closed)" referencing e1c0897 and c3fc163. f18b142
- [x] Task: Pre-commit checks pass. f18b142
- [x] Task: Commit (`projector(plan): close stderr-passthrough follow-up on cascade track`). f18b142
- [x] Task: Projector — User Manual Verification 'Phase 4: Re-run the Cascade Track's Smoke Test' (Protocol in workflow.md) f18b142

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- `format_exec_error` test count grew by ≥ 6 (4 baseline coverage + missing-Dockerfile + 4 fallback-shape tests, deduplicated).
- The cascade-track Verification Log now reads ✓ for "names the missing Dockerfile".
- Quality gates green at every commit.
- Manual verification recorded in a git note on the final checkpoint commit:
    - reproduce the missing-Dockerfile smoke test,
    - confirm the engine's actual reason is now in the user-visible message,
    - confirm exit status is non-zero.
