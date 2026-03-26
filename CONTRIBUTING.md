# Contributing

Thank you for your interest in contributing to this project.

## Getting Started

1. Fork the repository and clone your fork locally.
2. Create a new branch from `main` for your changes.
3. Make your changes following the guidelines below.
4. Push your branch and open a pull request.

## Development Setup

### Prerequisites

- Rust stable toolchain (see `rust-toolchain.toml` for the exact version)
- Cargo

### Building

```bash
cargo build
```

### Running Quality Checks

This project uses an xtask-based workflow for quality checks:

```bash
# Run all quality checks (format, lint, test, docs)
cargo xtask quality

# Individual checks
cargo xtask check    # Type checking
cargo xtask lint     # Clippy lints
cargo xtask test     # Run tests
```

## Pull Request Process

1. Ensure all quality checks pass (`cargo xtask quality`).
2. Update documentation if your change affects public APIs.
3. Add or update tests to cover your changes.
4. Write a clear commit message describing the change and its motivation.
5. Keep pull requests focused -- one logical change per PR.

## Code Style

- Follow standard Rust formatting (`rustfmt`). The project includes a `rustfmt.toml` configuration.
- Address all Clippy warnings. The project includes a `clippy.toml` configuration.
- Prefer immutable data structures and explicit error handling.
- Keep functions small and files focused.

## Testing

- All new functionality must include tests.
- Run the full test suite before submitting a PR: `cargo xtask test`.
- Aim for meaningful coverage of edge cases, not just the happy path.

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (Apache-2.0 OR MIT).
