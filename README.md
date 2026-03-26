![CI](https://github.com/USERNAME/REPO/actions/workflows/ci.yml/badge.svg)
[![Crates.io](https://img.shields.io/crates/v/CRATE_NAME.svg)](https://crates.io/crates/CRATE_NAME)
[![Docs.rs](https://docs.rs/CRATE_NAME/badge.svg)](https://docs.rs/CRATE_NAME)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE-MIT)

# Rust Dev Template

Complete Rust development environment with all essential tools integrated.

## Purpose

This template provides a fully configured local Rust development environment that combines a base project structure, a dev-container for hermetic builds, pre-commit hooks for automated quality gates, and XTask-driven workflow commands. It is intended to be the starting point for any new Rust crate in the repository, eliminating the per-project setup of formatting, linting, testing, and editor tooling.

## Features

- **Base Project Structure** - Essential Rust patterns and error handling.
- **Development Container** - Isolated, consistent development environment with pre-installed tools.
- **Code Quality Tools** - Pre-commit hooks for automated formatting, linting, and testing.
- **Development Tasks** - XTask commands for a unified development workflow (`cargo xtask check`, `cargo xtask lint`, etc.).
- **VS Code Integration** - Optimized editor configuration, extensions, and debug support.
