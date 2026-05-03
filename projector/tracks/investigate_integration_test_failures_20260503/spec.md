# Spec — Investigate `tests/integration.rs` Failures

## Severity
**Medium** — 23 of 32 tests in `tests/integration.rs` fail consistently across all fresh branches and on `trunk` after the bugs-batch landed. Reproduced on Track #1, Track #2, Track #3 worktrees, and on `trunk` post-cherry-pick — failure mode is invariant to source changes. They make `cargo test --workspace` unreliable: the noise masks real regressions, and contributors can't easily distinguish "I broke something" from "this was already broken".

## Problem

```
$ cargo test --workspace --tests
...
test result: FAILED. 9 passed; 23 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.92s
```

(All failures are in `tests/integration.rs`; every other test file is green.)

The 23 failing tests have names like:
- `test_docker_build_and_create`, `test_docker_start_and_running`, `test_docker_stop_and_rm`, `test_docker_cp`, `test_docker_exec`, `test_docker_restart`, `test_docker_running_returns_false_when_stopped`
- `test_podman_build_and_create`, `test_podman_start_and_running`, `test_podman_stop_and_rm`, `test_podman_cp`, `test_podman_exec`, `test_podman_restart`, `test_podman_running_returns_false_when_stopped`
- `test_post_create_command`, `test_exec_in_container`
- *(and ~9 more — full list to be enumerated in Phase 1)*

Track #3's agent characterised them as "Docker/Podman live-engine tests are pre-existing failures". A live `docker` daemon **is** present (Docker 29.3.0 confirmed in the Phase 3 smoke test of `fix_start_orchestration_cascade_20260503`), so the failures are not simply "no daemon". The actual failure mode for each test is unknown and must be empirically determined.

## Investigation Goals

For each failing test:

1. **Capture the failure output** verbatim (panic / assertion / I/O error / engine-reported error).
2. **Classify** by failure mode:
    - **Needs a specific image** that isn't pulled (e.g., `alpine:latest`).
    - **Needs network access** to pull images.
    - **Needs `podman`** which isn't installed in the dev environment.
    - **Misconfigured fixture path** or environment variable.
    - **Genuinely broken** (the test catches a real regression we should fix).
    - **Brittle to ambient state** (depends on a container name not already existing, on no leftover images, etc.).
    - **Architecture- or platform-specific** (e.g., podman rootless setup that's not configured).
3. **Decide remediation** per class:
    - Live-engine-required → `#[ignore]` by default; surface via a CI job that runs `cargo test -- --ignored`.
    - Network-required → same as above; document the prerequisite.
    - Misconfigured → fix.
    - Genuinely broken → red-green-refactor cycle.
    - Brittle → make robust (random suffix on container names, cleanup hooks) or `#[ignore]`.
    - Platform-specific → `#[cfg]`-gate or `#[ignore]` with explanation.

## Decision Criteria for Default-Run vs `#[ignore]`

A test runs by default only if it satisfies **all** of the following:

- Does not require pulling external images (no internet at test time).
- Does not require an internet connection.
- Works in a container-runtime-less environment OR uses a runtime already known to be available.
- Cleans up after itself (no leaked containers/images that could affect later tests).
- Completes in under 10 seconds on a developer laptop.

Otherwise it is `#[ignore]`-gated. CI runs `--ignored` in a dedicated job with a prepared engine.

## Acceptance Criteria

1. **`cargo test --workspace` passes on a clean dev machine** — including in this sandbox where 23 tests currently fail. "Passes" means exit code 0 with the integration tests either passing or `#[ignore]`-gated.
2. **Each `tests/integration.rs` test is documented** in this track's `findings.md` (under the track directory): purpose, requirements, decided remediation, current `#[ignore]` status.
3. **Tests gated by `#[ignore]`** carry a doc-comment explaining the prerequisite and how to run them locally (`cargo test --test integration -- --ignored`).
4. **CI** has a separate job in `.github/workflows/` that runs `cargo test --workspace -- --ignored` (or equivalent) with `docker` available. Job is allowed to be slower / less reliable than the default suite.
5. **No `tests/integration.rs` test silently fails** anymore — every failure is either fixed or `#[ignore]`-gated explicitly.
6. **At least one default-run integration test** exercises a real engine path *if* such a test is achievable without pulling images and within 10 s. If not, document why and rely on unit tests + the `#[ignore]`-gated CI job.

## Non-Goals

- Adding new test infrastructure (docker-in-docker, custom test harnesses, `nextest` migration) — defer if needed.
- Rewriting all integration tests as unit tests with mocks — only do so where trivially obvious.
- Setting up a podman environment in this sandbox — `podman` failures are accepted as `#[ignore]`-gated.
- Fixing flakiness in tests that are already passing.

## Risks

- **Genuinely broken tests** could surface real regressions in the source. We don't pre-judge — Phase 1 enumerates failures without prejudice; Phase 3 acts on them.
- **CI complexity.** Adding a `--ignored` job adds CI surface that must stay green; sketch the job in Phase 4 before committing it so the user can sign off.
- **`tests/integration.rs` may be load-bearing for confidence.** Disabling 23 tests shouldn't be a green-washing exercise — document each decision so a reviewer can audit it.

## References

- `tests/integration.rs` — the failing tests.
- `.github/workflows/*.yml` — CI configuration; need to add an `--ignored` job.
- Memory of Track #3 agent's characterisation ("pre-existing failures").
- `projector/tracks/fix_start_orchestration_cascade_20260503/plan.md` Verification Log — confirms a live docker daemon is available.
