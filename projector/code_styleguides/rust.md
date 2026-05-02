# Rust Code Style — devcont

This guide layers on top of `general.md` and the standard Rust style:

- `cargo fmt` (config in `rustfmt.toml`).
- `cargo clippy` (config in `clippy.toml`); CI runs with `-D warnings`.
- The [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).

## Project-Specific Conventions

### Errors
- **Binaries / top-level entry points** use `anyhow::Result` with `.context(...)` for human-readable errors.
- **Library code** (anything `pub` in `src/lib.rs` and below) uses `thiserror`-derived error enums. Never expose `anyhow::Error` from a library API.
- Prefer `?` over `.unwrap()` outside tests. Use `expect("...")` only when an invariant truly cannot fail at runtime — and the message must explain *why* it can't fail.

### Logging
- Use `tracing` macros (`error!`, `warn!`, `info!`, `debug!`, `trace!`).
- Public functions that perform I/O or shell out should be `#[tracing::instrument(level = "debug", skip(...))]` where reasonable.
- Never `println!`/`eprintln!` for diagnostic output. Reserve those for user-facing CLI output.

### CLI Surface
- One clap struct per subcommand under `src/cli/` (or close to it).
- Doc-comments on clap structs and fields are the source of truth for `--help`. Keep them tight, action-oriented, in imperative mood.
- Long-form help (`#[command(long_about = "...")]`) is for behavior that doesn't fit a one-liner — not for marketing.

### Subprocess Invocation
- Wrap all `std::process::Command` calls behind a small trait so unit tests can substitute fakes.
- Never pass user-supplied strings into `sh -c`. Always use the exec form (`Command::new(...).arg(...)`).
- Always check exit status and capture stderr for error context.

### Config & Paths
- All config paths go through `directories::ProjectDirs` (or equivalent). Never hardcode `~/.config/...`.
- All user-supplied path strings pass through `shellexpand::tilde(...)` before use.
- Validate that paths exist (or are creatable); surface a clear error if not.

### Modules & File Organization
- Prefer many small files (200–400 lines typical, 800 max) over one large module.
- Group by feature/domain (e.g., `engine/podman.rs`), not by type (e.g., `traits.rs`).
- Public re-exports live in `lib.rs`; keep them deliberate.

### Async
- Devcont is sync today. Do not introduce a runtime (`tokio`, `async-std`) without first updating `tech-stack.md` with the rationale.

### Tests
- Co-locate unit tests via `#[cfg(test)] mod tests { ... }`.
- Integration tests live under `tests/` (one file per scenario).
- Use `temp-env` to scope environment-variable changes; never call `std::env::set_var` in a test.
- Avoid filesystem flakiness: use `tempfile` (added when needed) for scratch directories.

### Documentation
- Every public item has a doc-comment.
- Run `cargo doc --workspace --no-deps --open` locally to spot missing or malformed doc-comments.
- Examples in doc-comments use `///` and run via `cargo test --doc` — keep them compilable.

### Unsafe & FFI
- No `unsafe` in this crate without a written justification in the surrounding doc-comment.
- No FFI dependencies without a prior tech-stack update.

## When in Doubt

- Defer to The Rust API Guidelines.
- Defer to existing patterns in the codebase.
- If neither answers it, open a discussion in the relevant track's `plan.md`.
