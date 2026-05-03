# Spec — Add `devcont up` Subcommand for Non-Attached Start

## Severity
**High (interop)** — `devcont start` always ends with `docker attach`, blocking the calling process on the container's stdin/stdout. Programmatic callers (CI scripts, the agent's `entrypoint.sh` in `good_vibe_sandbox_*`) want "ensure running, return, then `docker exec` later" semantics. Reported as the only remaining blocker for re-checkpointing `good_vibe_sandbox_20260321` clean.

## Problem

`src/devcontainers/mod.rs:360` (`Devcontainer::run`) unconditionally calls:

```rust
runtime.start()?;
// ... post-start hooks ...
self.post_create(&audit)?;
runtime.restart()?;
runtime.attach()?;          // ← blocks until interactive session ends
if /* shutdownAction */ {
    runtime.stop()?;
}
```

Programmatic callers cannot get past `attach()` without sending a TTY break or killing the process. They want a verb that means "ensure the container is up and running with hooks completed, then return success".

## Decision

Add a new subcommand: **`devcont up [DIR]`** — ensures the container exists, is running, and lifecycle hooks (`onCreate`, `updateContent`, `postCreate`, `postStart`) have run. Does NOT call `attach`. Does NOT honour `shutdownAction` (the caller manages teardown explicitly).

### Why a new subcommand, not a `--no-attach` flag

- **Discoverability** — `devcont --help` lists `up` next to `start`, mirrors `docker compose up`, signals intent clearly.
- **Verb integrity** — flags that change a verb's meaning are a known antipattern. `start` keeps meaning "start and attach"; `up` means "bring up and return".
- **Zero deprecation friction** — `start` is unchanged; existing scripts that depend on attach behaviour keep working.

### Internal refactor

`Devcontainer::run` is split into:

- **`Devcontainer::ensure_up(use_cache, trust, no_root_check, no_audit_log) -> Result<()>`** — performs `initializeCommand` confirmation, `ensure_created`, root-warning, `start` (if not running), `postStartCommand`, `post_create` (which runs `onCreate` / `updateContent` / `postCreate` / dotfiles copy). Returns when the container is running and hooks have completed.
- **`Devcontainer::attach_and_finalize(audit, no_audit_log) -> Result<()>`** — performs `restart`, `attach`, `postAttachCommand`, and the `shutdownAction` honour.

`Devcontainer::run` becomes `ensure_up` + `attach_and_finalize` (preserves current behaviour).
`Devcontainer::up` is just `ensure_up`.

## Acceptance Criteria

1. **`devcont up [DIR]`** is a new variant in `CliCommand` (src/main.rs) and a new module under `src/commands/up.rs`.
2. **Behaviour:** ensures the container exists (build + create if needed), is running (start if stopped), and runs all post-creation/post-start hooks. Returns 0 on success.
3. **No `attach`** is called. The calling shell is unblocked once the container is running and hooks have completed.
4. **No `shutdownAction`** is honoured. The container stays running on success regardless of `shutdownAction` in `devcontainer.json`.
5. **Same flags as `start`** (`--trust`, `--no-root-check`, `--no-audit-log`, `--hook-timeout`).
6. **`devcont start` is unchanged.** Output, exit code, and side effects remain bit-for-bit identical to the pre-track behaviour. Existing snapshot tests for `start` must continue to pass.
7. **Implementation symmetry:** `Devcontainer::run` is refactored into `ensure_up` + `attach_and_finalize`, called in sequence. `Devcontainer::up` is just `ensure_up`. The refactor is a behaviour-preserving change for `start`.
8. **Tests:** unit tests using the existing `MockProvider` assert that `up`:
    - calls `build`, `create`, `start` (or fewer if container already up — via `with_existing()` mock variant);
    - does NOT call `attach`;
    - does NOT call `restart`;
    - does NOT call `stop`, even when the config has `shutdownAction = "stopContainer"`.
9. **`devcont up --help`** documents the contract: "ensures the dev container exists and is running; runs lifecycle hooks; returns when ready; does not attach to the container".
10. **`devcont start --help`** is updated to clarify "starts and attaches an interactive session".
11. **README** gets a new "Subcommands" comparison section showing the difference between `start`, `up`, and `rebuild`.
12. **`devcont info`** (existing) documents in its `--help` that `up` is the recommended scriptable companion: `devcont up <DIR> && devcont info <DIR> | jq .container | xargs -I{} docker exec -it {} sh`.

## Non-Goals

- Adding `devcont down` / `devcont stop` / `devcont rm`. Useful follow-ups but out of scope. The caller uses the engine directly today (`docker stop $(devcont container-name <DIR>)`).
- Adding a `--detach` / `--no-attach` flag to `start`. We commit to the new-subcommand approach.
- Background / daemonised execution. `up` is synchronous: returns when hooks complete; the container's processes continue running supervised by the engine, not by devcont.
- Changing `rebuild`'s behaviour. `rebuild` still attaches at the end (it's a `start`-shaped operation).

## Risks

- **Hooks blocking forever.** `postCreateCommand` etc. could be long-running. The existing `--hook-timeout` flag already addresses this; document the recommendation in `up --help` ("if your hooks may run long, pass `--hook-timeout 600`").
- **Container exits between `up` returning and a subsequent `exec`.** If the container's main process exits (e.g., a `Dockerfile` with `CMD ["/bin/false"]`), `up` may return success but a later `exec` fails. **Mitigation:** after `start` (or no-op if already running), verify `Provider::running()` returns `true`; surface a clear error otherwise. Add this as a final assertion at the end of `ensure_up`.
- **Audit log shape.** Today the `ContainerStart` event covers the whole `run`. With `up`, the same event fires but the container isn't attached. Decide: same event, or a new `ContainerUp` event. Recommendation: same event (`up` and `start` both "start"); add a `mode: "up" | "start" | "rebuild"` field to disambiguate.

## References

- `src/devcontainers/mod.rs:360` (`Devcontainer::run`) — the function to split.
- `src/devcontainers/mod.rs:?` (`ensure_created`, `post_create`) — already-extracted helpers we reuse.
- `src/main.rs` (`CliCommand` enum) — where the new variant goes.
- `src/commands/start.rs`, `src/commands/rebuild.rs` — patterns for the new `commands/up.rs` module.
- `src/commands/container_name.rs`, `src/commands/info.rs` — example of a side-effect-free command we recently added; `up` is closer to `start` than to `info`.
- `docker compose up` — naming inspiration.
- Caller waiting on this: `good_vibe_sandbox_20260321` (external repo).
