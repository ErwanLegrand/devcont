# Spec — Surface Captured stderr in `format_exec_error` Fallback

## Severity
**Medium (UX)** — the cascade-halt behaviour from `fix_start_orchestration_cascade_20260503` is correct: orchestration stops on the first provider failure with one error. But for unrecognised stderr, that error reads `Error: build: Exec failed with exit code 1`, hiding the engine's actual diagnostic ("failed to read dockerfile: open Dockerfile: no such file or directory") from the user. This regresses spec acceptance criterion #6 of the cascade track ("captured stderr is plumbed through so the user sees the actual engine-reported reason").

## Problem

`src/provider/utils.rs:335` (`format_exec_error`) classifies stderr by pattern matching:

```rust
pub(crate) fn format_exec_error(exit_code: i32, stderr: &str) -> String {
    let lower = stderr.to_lowercase();
    if lower.contains("no such image") || lower.contains("image not known") {
        format!("Image not found: {}", stderr.lines().next().unwrap_or("unknown"))
    } else if lower.contains("permission denied") || lower.contains("not enough permissions") {
        format!("Permission denied: {}", stderr.lines().next().unwrap_or("unknown"))
    } else if lower.contains("cannot connect")
        || lower.contains("connection refused")
        || lower.contains("daemon is not running")
        || (lower.contains("no such file or directory") && lower.contains(".sock"))
    {
        "Container daemon is not running — check that your container runtime is started".to_string()
    } else if lower.contains("container already exists") {
        format!("Container already exists: {}", stderr.lines().next().unwrap_or("unknown"))
    } else {
        format!("Exec failed with exit code {exit_code}")
    }
}
```

The fallback (last branch) discards `stderr` entirely. Anything not in the four known patterns surfaces as `Exec failed with exit code N` — useless for diagnosis.

The Phase 3 smoke test of `fix_start_orchestration_cascade_20260503` (recorded in that track's `plan.md` Verification Log) confirmed this: docker's `failed to read dockerfile: open Dockerfile: no such file or directory` was suppressed.

## Decision

1. **Always include captured stderr in the fallback.** The new fallback message is `"<step>: <first non-empty line of stderr> (exit code N)"` — or, when stderr is empty, `"<step>: exec failed with exit code N"`.
2. **Add a pattern arm for missing-file errors** so they get a friendly title plus the underlying line. Triggers: `"failed to read dockerfile"`, `"open .* no such file or directory"` (when not a `.sock`), `"dockerfile not found"`.
3. **Preserve the four existing pattern arms** unchanged. Their friendly messages stay; only the fallback changes.
4. **Trim and bound the surfaced stderr.** First non-empty line, capped at 240 characters, with `…` if truncated. Multi-line stack-trace stderr is never dumped wholesale.

The function signature remains `(exit_code: i32, stderr: &str) -> String`. Callers (`run_and_check`, `run_step`) need no changes.

## Acceptance Criteria

1. **Existing tests of `format_exec_error` continue to pass** without modification:
    - Image not found → `"Image not found: ..."`
    - Permission denied → `"Permission denied: ..."`
    - Daemon not running → `"Container daemon is not running ..."`
    - Container already exists → `"Container already exists: ..."`
    *(If existing tests don't cover these branches, add them in the Red phase.)*
2. **New pattern: missing Dockerfile.** Stderr containing `"failed to read dockerfile: open Dockerfile: no such file or directory"` produces a message that contains both the word `Dockerfile` and the phrase `not found` (or equivalent), surfacing the file name.
3. **New fallback.** Stderr like `"some unknown error\nstack trace line 1\nstack trace line 2"` and exit code 137 produces a message containing `"some unknown error"` and `"137"`. Multi-line stderr is collapsed to first non-empty line.
4. **Empty stderr fallback.** When `stderr` is empty/whitespace-only, the message is `"exec failed with exit code N"` (no broken `:` punctuation).
5. **Length cap.** Stderr longer than 240 characters is truncated with `…`. Test asserts on a 1000-char stderr that the message length is ≤ 280.
6. **Phase 3 smoke test of `fix_start_orchestration_cascade_20260503` is re-run** with the new behaviour and the Verification Log in that track's `plan.md` is updated to reflect the now-passing "names the missing Dockerfile" criterion. The followup-open caveat in that log is closed out.
7. **No allocations on hot paths beyond what the existing implementation does.** This is a CLI error path, not perf-critical, but avoid quadratic string operations on huge stderr buffers.

## Non-Goals

- **Not a redesign of the error taxonomy.** The function still returns `String` and is consumed by `run_and_check` via `io::Error::other(...)`.
- **Not adding structured error variants.** No new `thiserror` enum cases; stay with the string-formatted message.
- **Not localising messages.** English-only.
- **Not parsing JSON-formatted docker output** (newer docker buildx may emit JSON on `--progress=plain`); we operate on raw stderr text.
- **Not propagating stderr line breaks.** The CLI shows a single-line error; multi-line dumps belong in `RUST_LOG=trace`.

## Risks

- **Pattern matching is brittle.** Docker, podman, nerdctl, and the Apple `container` CLI may phrase the same condition differently across versions. The fallback ensures we never lose information regardless of phrasing.
- **Sensitive data in stderr.** If a build step echoes a secret to stderr, surfacing the first line could leak it. **Mitigation:** the existing `redact_env_args` helper handles `--env` redaction at command-print time; stderr from inside the build is the user's own code, and we mirror what they'd see running the same docker command directly. Document this in the doc-comment.

## References

- `src/provider/utils.rs:335` (`format_exec_error`) — implementation.
- `src/provider/utils.rs:165` (`run_and_check`), `src/provider/utils.rs:?` (`run_step`) — callers.
- `projector/tracks/fix_start_orchestration_cascade_20260503/plan.md` — Verification Log captures the regression this track addresses.
- containers.dev spec — n/a (this is a CLI UX change, not a spec-compliance issue).
