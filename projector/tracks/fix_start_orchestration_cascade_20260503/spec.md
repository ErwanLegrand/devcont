# Spec — Fix `start` Orchestration Error Cascade

## Severity
**Polish (UX)** — but it actively masks Bug 1 (`fix_dockerfile_path_resolution_20260503`) and any other build failure, because the user sees five misleading errors after the first real failure. Fix this **before** Bug 1 ships, so users diagnosing path-resolution problems see one clear error instead of six.

## Problem

`Devcontainer::run` (devcontainers/mod.rs:230) orchestrates the lifecycle as:

```text
ensure_created → start → postStartCommand → post_create
              → restart → attach → postAttachCommand
              → stop (if shutdownAction)
```

Every step uses `?` on its `Result`. **However**, the `Provider` trait (provider/mod.rs:20) returns `io::Result<bool>` from `build`, `create`, `start`, `stop`, `restart`, `attach`, where the **inner `bool`** indicates "the subprocess exited with status zero". The implementation in `provider/utils.rs:165` is:

```rust
pub(crate) fn run_and_check(command: &mut Command) -> Result<bool> {
    print_command(command);
    Ok(command.status()?.success())
}
```

`?` on `runtime.start()` only short-circuits when the *spawn itself* fails (the outer `io::Error`). When the subprocess exits non-zero, the call returns `Ok(false)` and execution silently continues. The orchestration discards the bool. Result: a single failed `docker build` cascades into five further docker invocations (`docker create`, `docker start`, `docker restart`, `docker attach`, `docker stop`), each of which also fails for the same root cause, each emitting its own confusing message.

## Acceptance Criteria

1. **A failed subprocess invocation halts orchestration immediately** with a clear, single-error message that names the failed step and includes captured stderr (mapped via `format_exec_error`).
2. **`Devcontainer::run` and `Devcontainer::rebuild` return `Err` on the first failed lifecycle step.** No subsequent provider call is made after a failure.
3. **The `Provider` trait API is updated** so non-zero exit becomes an `Err`, not `Ok(false)`. Either:
    - **Option A (preferred):** change return type from `io::Result<bool>` to `io::Result<()>` and have `run_and_check` map non-zero exit into `Err(io::Error::other(format_exec_error(...)))`.
    - **Option B:** keep `io::Result<bool>` but require every caller to check the bool. Reject — too easy to regress; the compiler can't enforce it.
4. **`Provider::exists`** and **`Provider::running`** keep their `io::Result<bool>` signatures. Their bool is a genuine boolean fact ("does the container exist", "is it running"), not a success/failure proxy.
5. **`run_and_check` is renamed or reshaped** to reflect the new contract: spawn failure or non-zero exit both produce `Err`. Callers no longer have to inspect a bool.
6. **Captured stderr is plumbed through** so the user sees the actual engine-reported reason (image not found, build failed, daemon not running, …). Reuse `format_exec_error` (utils.rs:335) for the message.
7. **All Provider implementations** (docker, docker_compose, podman, podman_compose, nerdctl, apple) and the test stub at `mod.rs:804+` are updated.
8. **No silent failures.** `cargo clippy -W clippy::needless_question_mark -W clippy::let_underscore_must_use` (or equivalent) should be clean; we should not be discarding any success value.
9. **Existing hook-failure tests** (`run_aborts_on_post_create_hook_failure` and friends, mod.rs:1005+) continue to pass and a new test class is added: `run_aborts_on_<step>_failure` for each provider step (`build`, `create`, `start`, `restart`, `attach`, `stop`).

## Non-Goals

- Adding retry logic for transient engine failures (separate concern; consider as a follow-up if the field demands it).
- Refactoring the orchestration into a state machine. The control flow is already linear; we just need correct error propagation.
- Changing `format_exec_error` heuristics — fix the propagation, not the message taxonomy.

## Migration Risk

The `Provider` trait's signature changes are visible to anyone who implemented the trait. Today, all impls live in this crate. The trait is `pub` in `provider/mod.rs` but the providers are `pub(crate)`. We accept the breaking change and document it in `CHANGELOG.md`.

## References

- `src/devcontainers/mod.rs:230` (`Devcontainer::run`)
- `src/devcontainers/mod.rs:271–287` (the cascading lifecycle calls)
- `src/devcontainers/mod.rs:804+` (test stub `Provider` impl)
- `src/provider/mod.rs:20` (trait `Provider`)
- `src/provider/utils.rs:165` (`run_and_check`)
- `src/provider/utils.rs:335` (`format_exec_error`)
