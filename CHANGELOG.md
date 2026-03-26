# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `exec_capture` method on `Provider` trait for programmatic command output capture
- `ExecOutput` struct with `stdout_lossy()` / `stderr_lossy()` convenience methods
- `ExecCaptureFailed` error variant with truncated stderr display
- xtask development task runner (`cargo xtask quality`, `check`, `lint`, `test`, etc.)
- Workspace structure with `devcont` + `xtask` members

### Changed

- Converted project to Cargo workspace
