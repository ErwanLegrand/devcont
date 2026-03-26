# Canonical Dependency Versions

This file is the **authoritative version list** for all Rust template dependencies.
When updating a dependency version, update it here first, then propagate to all templates.

## Version Table

| Crate | Version | Notes |
|-------|---------|-------|
| `criterion` | `0.5` | Benchmarking framework; 0.5 adds async support and improved output |
| `thiserror` | `2.0` | Derive macro for error types; 2.0 uses `#[from]` proc-macro v2 syntax |
| `tokio` | `1.x` (latest stable) | Async runtime; require at minimum `1.36` for `tokio::task::spawn_blocking` improvements |
| `axum` | `0.8.x` | HTTP framework built on hyper 1.x; 0.8 is the first hyper-1.x-compatible release |
| `serde` | `1.x` | Serialization framework; `1.0` has been stable for years  stay on `1.0` |
| `anyhow` | `1.0` | Error propagation helper; complements `thiserror` for application-level errors |
| `clap` | `4.x` | CLI argument parsing; `4.0` introduces derive API improvements |
| `tracing` | `0.1` | Structured logging and diagnostics |
| `tracing-subscriber` | `0.3` | Subscriber implementations for `tracing` |

## Rationale

### criterion 0.5
Version 0.5 adds native async benchmark support, removes the dependency on `rayon`
for parallelism, and produces cleaner HTML reports. All templates should use 0.5
rather than the older 0.4 series.

### thiserror 2.0
Version 2.0 switches to the proc-macro v2 API, reducing compile times and adding
support for `#[error(transparent)]` improvements. The API is backwards-compatible
with 1.x for most use cases. All templates standardise on 2.0.

### tokio 1.x
The `1.x` series is stable and long-supported. Specify the lowest compatible minor
version in each template to maximise consumer compatibility, but test against the
latest stable minor release.

### axum 0.8.x
The 0.8 series targets hyper 1.x (which is a major rewrite). Templates must not mix
axum 0.7.x (hyper 0.x) with axum 0.8.x dependencies. Use `0.8` and pin minor as
needed.

### serde 1.x
The serde `1.x` release train has been stable since 2017. Always use `1.0` and let
Cargo resolve the latest compatible patch version. Avoid pre-release or `2.x` when
it becomes available until ecosystem adoption is proven.

## Update Process

1. Decide on new canonical version here in `DEPENDENCIES.md`.
2. Update `[workspace.dependencies]` in `rust-dev-template/Cargo.toml`.
3. Run `cargo update -p <crate>` in `rust-dev-template/` to lock the new version.
4. Propagate to:
   - `rust-xtask-template/Cargo.toml`
   - `rust-axum-server-template/Cargo.toml`
   - `rust-pre-commit-template/Cargo.toml` (if applicable)
   - `rust-base-template/Cargo.toml` (if applicable)
5. Run `cargo update -p <crate>` in each updated template directory.
6. Commit with `chore(deps): bump <crate> to <version> across all templates`.
