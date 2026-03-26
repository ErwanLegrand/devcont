# Architecture Overview

## Module Structure

```
src/
  main.rs              CLI entry point (clap)
  lib.rs               Library root — re-exports public API
  error.rs             Error types (thiserror)
  settings.rs          User config (~/.config/devcont/config.toml)
  audit.rs             Structured audit log (NDJSON)
  commands/
    mod.rs             Command module
    start.rs           `devcont start` — start or resume a container
    rebuild.rs         `devcont rebuild` — destroy and rebuild
  devcontainers/
    mod.rs             Devcontainer lifecycle (run, rebuild, hooks)
    config.rs          devcontainer.json parsing (JSON5)
    one_or_many.rs     OneOrMany type for hook values
    paths.rs           Path validation (traversal prevention)
    run_args.rs        runArgs validation (privilege denylist)
  provider/
    mod.rs             Provider trait + ExecOutput + shared utilities
    docker.rs          Docker engine
    podman.rs          Podman engine (rootless, SELinux, error parsing)
    nerdctl.rs         nerdctl (containerd) engine
    apple.rs           Apple Container Runtime (macOS)
    docker_compose.rs  Docker Compose engine
    podman_compose.rs  Podman Compose engine
    options.rs         ContainerOptions struct
    utils.rs           Shared helpers (compose override, Dockerfile resolution)
```

## Data Flow

```
CLI (main.rs)
  -> Commands (start/rebuild)
    -> Devcontainer::run() / rebuild()
      -> Provider trait (build, create, start, attach, exec, ...)
        -> Container engine (docker/podman/nerdctl/apple CLI)
```

1. **CLI** parses args via clap, delegates to `commands::start::run()` or `commands::rebuild::run()`.
2. **Commands** load `Settings`, parse `devcontainer.json` into `Config`, select a `Provider`.
3. **Devcontainer** orchestrates the lifecycle: build image, create container, copy dotfiles, run hooks, attach shell.
4. **Provider** translates operations into container engine CLI commands.

## Provider Trait

All container engines implement `Provider`:

- `build()`, `create()`, `start()`, `stop()`, `restart()`, `attach()`, `rm()`
- `exists()`, `running()` — container state queries
- `cp()` — file copy into container
- `exec()`, `exec_raw()` — run commands inside (terminal-attached)
- `exec_capture()` — run commands inside (output captured as bytes)

## Error Handling

- `thiserror` for typed errors (`Error` enum in `error.rs`)
- `anyhow` for application-level propagation in CLI commands
- Provider methods return `std::io::Result` (legacy) except `exec_capture` which returns `crate::error::Result`

## Security Model

- `runArgs` validated against a privilege-escalation denylist
- `initializeCommand` requires user confirmation (or `--trust` flag)
- Root user warning when no `remoteUser` configured
- Path traversal prevention for workspace and Dockerfile paths
- Environment variable values redacted in terminal output
- Structured audit log with restricted file permissions
