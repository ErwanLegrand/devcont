# Status — investigate_integration_test_failures_20260503

**Date:** 2026-05-03
**State:** Phase 4 YAML drafted; awaiting user review

---

## Summary

All five phases of the plan are substantially complete:

- **Phase 1 (Enumerate):** Completed. All 32 tests enumerated with failure modes.
- **Phase 2 (Classify):** Completed. 27 `network-required`, 5 `live-engine-required`.
  All 32 classified `#[ignore]`.
- **Phase 3 (Apply #[ignore]):** Completed. All 32 tests gated. `cargo test --workspace`
  exits 0, 32 ignored. `command_available("podman-compose")` guard removed.
- **Phase 4 (CI YAML):** Draft written in `findings.md`. **DO NOT commit changes to
  `.github/workflows/*.yml` until the user reviews and approves the draft.**
- **Phase 5 (Verification):** Completed. 27 docker+podman `--ignored` tests pass.
  5 podman-compose tests fail with "not found" (expected — podman-compose not installed
  in this dev environment; the CI YAML draft provisions it).

---

## Blocker

Phase 4 requires user review of the proposed CI YAML before the workflow file is
edited. The draft is in:

  `projector/tracks/investigate_integration_test_failures_20260503/findings.md`
  Section: "Phase 4 — Proposed CI YAML"

The user must:
1. Review the `tests-ignored` job YAML.
2. Decide whether to add a new `tests-ignored` job or update the existing
   `Run integration tests` step in `test.yml` (which currently does nothing
   since all tests are now `#[ignore]`-gated).
3. Approve or request changes.

Once approved, the CI YAML can be committed with `ci: add --ignored integration
tests job`.

---

## Key Acceptance Criteria Status

| Criterion | Status |
|---|---|
| `cargo test --workspace` exits 0 | ✅ 0 failed, 32 ignored |
| Each test documented in findings.md | ✅ All 32 documented |
| `#[ignore]`-gated tests carry doc-comment + run instructions | ✅ |
| CI job drafted | ✅ In findings.md; awaiting approval |
| No test silently fails | ✅ All failures now explicit |

---

## Next Steps (after user approval)

1. Commit the CI YAML change.
2. Verify the CI job passes on first run.
3. Update findings.md with the CI run link.
4. Final checkpoint commit.
