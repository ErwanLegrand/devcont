# Spec — Add an Inspect-Shaped Subcommand

## Severity
**High** — without it, downstream callers (CI scripts, editors that shell out to devcont, wrapper scripts) cannot retrieve the container name without parsing `start` log output, which is fragile, locale-sensitive, and side-effectful (it actually launches the container).

## Problem

`src/main.rs:15` defines `CliCommand` with only two variants: `Rebuild` and `Start`. There is no read-only command that reports facts about the container devcont *would* operate on for a given directory.

Common things callers want to know:
1. The deterministic container name (so they can run `docker exec` / `podman exec` directly).
2. Whether the container exists.
3. Whether it is running.
4. The image it was built from (if applicable).
5. The workspace folder mounted into it.

## Decision

Provide **two** read-only subcommands:

1. **`devcont container-name [DIR]`** — prints the container name (and nothing else) to stdout. Single-purpose, scriptable, zero ambiguity.
2. **`devcont info [DIR]`** — prints a JSON document with structured metadata. Forward-compatible: new fields can be added without breaking parsers.

Rationale: `container-name` solves the immediate "give me the name" need with a one-line shell-friendly output. `info` is the place we hang future inspect-shaped functionality (probe, doctor, diagnose) without polluting the container-name interface.

### `container-name` — exact contract

- Stdout: a single line containing the container name, terminated by `\n`.
- Stderr: empty on success.
- Exit code:
    - `0` — name was determined and printed.
    - `2` — `devcontainer.json` could not be loaded (with diagnostic on stderr).
    - `3` — config loaded but the container name could not be derived (e.g., `safe_name()` failed).
- No side effects on the container; never invokes `docker`/`podman`.

### `info` — exact contract

Stdout: a JSON object (one line, no trailing newline beyond the JSON's own) with at least these fields:

```json
{
  "container": "devcont-myproject-abc123",
  "image": "myproject:latest" or null,
  "workspace": "/abs/path/to/workspace",
  "config_dir": "/abs/path/to/.devcontainer",
  "exists": true,
  "running": false,
  "engine": "docker"
}
```

- `exists` and `running` MAY require an engine probe (one or two `docker inspect` invocations). They MUST be computed via the existing `Provider::exists` and `Provider::running` methods.
- A `--no-probe` flag suppresses the probe and omits `exists`/`running` from the output (useful in tight-loop scripts).
- Field order is fixed so the JSON diffs cleanly; we do not promise stable ordering inside `serde_json`'s default behavior — use `serde_json::to_string_pretty` with a fixed struct.
- Exit code `0` on success; `2` on config load failure; non-zero on engine probe failure (with stderr diagnostic) unless `--no-probe` was passed.

## Acceptance Criteria

1. `devcont --help` lists `rebuild`, `start`, `container-name`, `info`, `help`.
2. `devcont container-name` (no arg) prints the container name for the current directory and exits 0.
3. `devcont container-name <dir>` prints the container name for `<dir>` and exits 0.
4. `devcont info` returns valid JSON parsable by `jq -e .container`.
5. `devcont info --no-probe` returns JSON without `exists`/`running` and never invokes the engine.
6. Both subcommands work for all engines (docker, docker-compose, podman, podman-compose, nerdctl, apple).
7. Neither subcommand has side effects on the container.
8. Both subcommands respect `--trust`, `--no-audit-log` semantics where applicable. `container-name` does NOT require `--trust` because it executes no host hooks.
9. `--help` text for both subcommands documents the exit-code contract.
10. Integration tests cover stdout/stderr/exit-code contracts for both subcommands.

## Non-Goals

- A full `devcont exec` / `devcont shell` subcommand. (Consider as a follow-up.)
- Live JSON streaming or watch mode.
- Cross-engine inspection of arbitrary metadata (port mappings, mounts, env). Add only what we already compute internally.

## References

- `src/main.rs:15` (`CliCommand` enum)
- `src/devcontainers/mod.rs` — `safe_name()`, `Provider::exists`, `Provider::running`
- `src/commands/start.rs`, `src/commands/rebuild.rs` — pattern for new `commands/<name>.rs` modules
