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

## Phase 2: New Pattern — Missing Dockerfile

- [ ] Task: Red Phase — add `format_exec_error_missing_dockerfile`:
    - stderr: `"#1 [internal] load build definition from Dockerfile\n#1 transferring dockerfile: 2B done\n#1 DONE 0.0s\nERROR: failed to build: failed to solve: failed to read dockerfile: open Dockerfile: no such file or directory"`
    - exit code: `1`
    - assert message contains `"Dockerfile"` and `"not found"` (or equivalent), and the first matching line from stderr.
    - Run `cargo test format_exec_error_missing_dockerfile` — confirm fail.
- [ ] Task: Green Phase — add a new arm to `format_exec_error` matching `"failed to read dockerfile"` OR (`"no such file or directory"` AND NOT `".sock"` AND `"dockerfile"`-mention). Format: `"Dockerfile not found: <first matching line of stderr>"`.
    - Order arms carefully — the daemon-not-running arm already special-cases `".sock"` exclusion for `"no such file or directory"`. The new arm must come AFTER the daemon arm so the `.sock` case is still caught first.
    - Run `cargo test format_exec_error` — confirm new test passes and existing tests still pass.
- [ ] Task: Refactor — keep the function readable; extract `first_nonempty_line(stderr: &str) -> &str` if it's used in more than two arms.
- [ ] Task: Verify Coverage on `format_exec_error`. Coverage of the new arm should be 100%.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`feat(provider): recognise missing-Dockerfile errors in format_exec_error`).
- [ ] Task: Projector — User Manual Verification 'Phase 2: New Pattern — Missing Dockerfile' (Protocol in workflow.md)

---

## Phase 3: Fallback — Surface stderr Verbatim

- [ ] Task: Red Phase — add `format_exec_error_fallback_includes_stderr`:
    - stderr: `"some completely unfamiliar error from the engine"`, exit `137`.
    - assert message contains `"some completely unfamiliar error"` and `"137"`.
    - Run and confirm fail.
- [ ] Task: Red Phase — add `format_exec_error_fallback_collapses_multiline`:
    - stderr: `"\n\nfirst real line\nstack frame 2\nstack frame 3"`, exit `2`.
    - assert message contains `"first real line"` and `"2"`, and does NOT contain `"stack frame 2"`.
- [ ] Task: Red Phase — add `format_exec_error_fallback_empty_stderr`:
    - stderr: `""`, exit `99`.
    - assert message is exactly `"exec failed with exit code 99"` (no `:` and no trailing whitespace).
- [ ] Task: Red Phase — add `format_exec_error_fallback_truncates_long`:
    - stderr: a 1000-char single line.
    - assert resulting message length ≤ 280 and ends with `"…"`.
- [ ] Task: Green Phase — replace the fallback `format!("Exec failed with exit code {exit_code}")` with a helper `fallback_message(exit_code: i32, stderr: &str) -> String` that:
    1. Trims `stderr` to first non-empty line.
    2. Truncates to 240 chars with `"…"` suffix.
    3. If line is empty: returns `"exec failed with exit code {N}"`.
    4. Otherwise: returns `"{trimmed_line} (exit code {N})"`.
    - Run all `format_exec_error*` tests; all should pass.
- [ ] Task: Refactor — if Phase 2 already extracted `first_nonempty_line`, reuse it here.
- [ ] Task: Verify Coverage on the fallback paths.
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`feat(provider): include captured stderr in format_exec_error fallback`).
- [ ] Task: Projector — User Manual Verification 'Phase 3: Fallback — Surface stderr Verbatim' (Protocol in workflow.md)

---

## Phase 4: Re-run the Cascade Track's Smoke Test

- [ ] Task: Rebuild the release binary (`cargo build --release`).
- [ ] Task: Run the existing fixture at `tmp/devcont-smoke-test/.devcontainer/devcontainer.json` (recreate if absent — see fix_start_orchestration_cascade plan's Verification Log).
- [ ] Task: Confirm `./target/release/devcont start tmp/devcont-smoke-test` now produces an error message containing the words `Dockerfile` and `not found` (or, if the missing-file pattern doesn't trigger for this exact docker buildx output, at least the docker stderr's first informative line).
- [ ] Task: Update `projector/tracks/fix_start_orchestration_cascade_20260503/plan.md`:
    - Mark the "Names the missing Dockerfile" row in its Verification Log as ✓ instead of ✗.
    - Strike the "Follow-up (open)" paragraph and replace it with: `**Follow-up (closed).** Resolved by track \`improve_format_exec_error_stderr_20260503\` (commit <SHA>).`
- [ ] Task: Pre-commit checks.
- [ ] Task: Commit (`projector(plan): close stderr-passthrough follow-up on cascade track`).
- [ ] Task: Projector — User Manual Verification 'Phase 4: Re-run the Cascade Track's Smoke Test' (Protocol in workflow.md)

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
