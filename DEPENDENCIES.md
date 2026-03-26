# Dependencies

Documents all runtime and development dependencies and the rationale for each.

## Runtime

| Crate | Version | Purpose |
|-------|---------|---------|
| [clap](https://crates.io/crates/clap) | 4.x | CLI argument parsing — derive API, subcommand routing |
| [serde](https://crates.io/crates/serde) | 1.x | Serialization framework used by serde_json and toml |
| [serde_json](https://crates.io/crates/serde_json) | 1.x | JSON deserialization (used internally by json5) |
| [json-five](https://crates.io/crates/json-five) | 0.3.x | Parse `devcontainer.json` — supports comments and trailing commas per the spec |
| [toml](https://crates.io/crates/toml) | 1.0.x | Parse user config at `~/.config/devcont/config.toml` |
| [directories](https://crates.io/crates/directories) | 6.x | OS-appropriate config/data directory resolution (XDG on Linux, etc.) |
| [shellexpand](https://crates.io/crates/shellexpand) | 3.x | `~` and `$VAR` expansion in user-provided directory paths |
| [tinytemplate](https://crates.io/crates/tinytemplate) | 1.x | `docker-compose.yml` template rendering for compose providers |
| [colored](https://crates.io/crates/colored) | 3.x | Colored terminal output for the command echo feature |
| [anyhow](https://crates.io/crates/anyhow) | 1.x | Application-level error propagation with human-readable context chains |
| [thiserror](https://crates.io/crates/thiserror) | 2.x | Derive macro for typed domain error enums (`src/error.rs`) |
| [tracing](https://crates.io/crates/tracing) | 0.1.x | Structured diagnostic logging — replaces `eprintln!` in library code |
| [tracing-subscriber](https://crates.io/crates/tracing-subscriber) | 0.3.x | Subscriber with env-filter for `RUST_LOG`-based log level control |

## xtask

| Crate | Version | Purpose |
|-------|---------|---------|
| [clap](https://crates.io/crates/clap) | 4.x | CLI argument parsing for xtask subcommands |
| [anyhow](https://crates.io/crates/anyhow) | 1.x | Error propagation in task runner |
| [duct](https://crates.io/crates/duct) | 1.1.x | Shell command execution with piping support |
| [toml](https://crates.io/crates/toml) | 1.0.x | Parse workspace Cargo.toml for metadata |

## Dev Tools (installed in CI and dev container)

| Tool | Purpose |
|------|---------|
| [cargo-llvm-cov](https://crates.io/crates/cargo-llvm-cov) | LLVM-based code coverage — enforces >80% threshold in CI |
| [cargo-deny](https://crates.io/crates/cargo-deny) | Dependency audit: license compliance + security advisories |

## Intentionally Excluded

| Crate | Reason |
|-------|--------|
| proptest / criterion | Property-based testing and benchmarks deferred to dedicated testing track |
| cargo-audit | Superseded by `cargo-deny` which covers both licenses and advisories |
