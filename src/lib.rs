//! `devcont` — a CLI for launching and managing Dev Containers.
//!
//! The library surface is intentionally small. External callers use:
//!
//! - [`devcontainers::Devcontainer`] — load, run, and rebuild a dev container from a workspace.
//! - [`error::Error`] / [`error::Result`] — all library errors.
//! - [`settings::Settings`] — user settings loaded from `~/.config/devcont/config.toml`.
//!
//! The [`provider`] module is public for integration tests but its types are not stable API.
#![warn(clippy::pedantic)]

pub(crate) mod audit;
pub mod devcontainers;
pub mod error;
pub mod provider;
pub mod settings;
