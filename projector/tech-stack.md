# devcont — Tech Stack

## Language & Toolchain

- **Rust 2024 edition** (`Cargo.toml`: `edition = "2024"`).
- **MSRV:** 1.85 (`rust-version = "1.85"`).
- **Workspace layout:** root crate (`devcont`) plus an `xtask` helper crate (cargo-xtask pattern for project-local automation).
- **Toolchain pin:** `rust-toolchain.toml`.

## Runtime Crates

| Concern | Crate | Why |
|---|---|---|
| CLI parsing | `clap` (derive) | de facto standard; auto-generates `--help` from doc-comments |
| Config files | `serde` + `toml` | TOML for user config; serde for ergonomic structs |
| `devcontainer.json` parsing | `serde_json` + `json-five` | the spec allows JSONC (comments, trailing commas); `json-five` handles it |
| Path expansion | `shellexpand` | `~` and `$HOME` in user-supplied paths |
| Platform paths | `directories` | XDG / Apple / Windows conventions for config and cache dirs |
| Templating | `tinytemplate` | small, dependency-light substitution for variables in command strings |
| Error handling | `anyhow` (binary) + `thiserror` (library) | application errors with context vs. typed library errors |
| Logging | `tracing` + `tracing-subscriber` | structured, leveled logs; `env-filter` reads `RUST_LOG` |
| Terminal color | `colored` | NO_COLOR-aware; small footprint |

## Dev Dependencies

- `temp-env` — scoped environment-variable mutation in tests without leaking into other tests.

## External Engines (runtime requirements)

Devcont **invokes** these; they are not Rust dependencies:

- `docker` and `docker-compose`
- `podman` and `podman-compose`
- `nerdctl` (containerd)
- `container` (Apple, macOS 14+)

## Tooling

- **Build & lint loop:** `cargo fmt`, `cargo check`, `cargo clippy`, `cargo audit` — run before every commit; mirrored in CI.
- **Coverage:** to be wired up via `cargo llvm-cov` (target ≥ 80% per `workflow.md`).
- **Fuzzing:** `cargo-fuzz` (`fuzz/` workspace member).
- **Supply-chain audit:** `cargo-deny` with `deny.toml`.
- **Changelog generation:** `git-cliff` with `cliff.toml`.
- **Pre-commit:** `pre-commit` framework with `.pre-commit-config.yaml` (fmt, clippy, audit hooks).
- **CI:** GitHub Actions in `.github/workflows/` (uses `actions/checkout@v6`, `cargo-deny-action@v2`).
- **Self-hosted dev environment:** dogfooded via `.devcontainer/`.

## Distribution

- **From source:** `cargo install --path .` (current README) and `cargo install devcont` once published to crates.io.
- **License:** dual MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
- **Branches:** `main` is canonical; `dev` and `trunk` are intentionally disjoint orphan branches.

## Deliberate Non-Choices

- **No async runtime.** Devcont shells out to subprocesses; sync `std::process::Command` is sufficient and keeps the dependency tree small. Revisit only if we need concurrent multi-engine probing.
- **No HTTP client.** Devcont does not fetch features or templates over the network.
- **No persistent state of our own.** Container state lives with the engine.
