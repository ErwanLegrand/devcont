# Rust Base Template Documentation

This directory contains additional documentation for the Rust Base Template.

## Table of Contents

- [Architecture](architecture.md) - Template architecture and design decisions
- [Customization](customization.md) - How to customize generated projects
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
- [FAQ](faq.md) - Frequently asked questions

## Quick Reference

### Template Variables

The template uses these variables for customization:

| Variable | Description | Example |
|----------|-------------|---------|
| `{{project_name}}` | Project name | `my-awesome-project` |
| `{{project_name_snake}}` | Snake_case version | `my_awesome_project` |
| `{{project_description}}` | Project description | `A cool Rust project` |
| `{{repository_url}}` | Git repository URL | `https://github.com/user/project` |
| `{{current_year}}` | Current year | `2025` |

### Generated Structure

```
project/
 src/
    lib.rs           # Main library file
    error.rs         # Error types
    prelude.rs       # Common imports
 tests/
    integration_test.rs
 Cargo.toml           # Package configuration
 README.md            # Project documentation
 LICENSE-MIT          # MIT license
 LICENSE-APACHE       # Apache 2.0 license
 rust-toolchain.toml  # Rust toolchain
```

## Support

For questions or issues:

- [GitHub Issues](https://github.com/erwanlegrand/rust-base-template/issues)
- [GitHub Discussions](https://github.com/erwanlegrand/rust-base-template/discussions)
- [Documentation](https://erwanlegrand.github.io/rust-base-template/)