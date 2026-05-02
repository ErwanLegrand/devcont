# devcont — Product Guide

## Vision

A standalone CLI that runs Dev Containers (per the [containers.dev](https://containers.dev/) spec) outside any IDE — usable from a terminal or CI — across multiple container engines.

## Primary Goal

Provide a fast, scriptable way to start, rebuild, and manage Dev Containers locally and in CI without depending on VS Code, IntelliJ, or any GUI.

## Target Users

- Developers who prefer terminals and editors other than VS Code or JetBrains IDEs (e.g., Vim, Emacs, Helix, Kakoune, Zed, Sublime).
- CI/CD pipelines that want to spin up the same Dev Container the maintainer uses locally for build, test, and release jobs.
- Maintainers who want a portable, IDE-agnostic source of truth for their dev environment.

## Goals

- Spec-faithful execution of `devcontainer.json` files (single-container, compose-mode, Dockerfile-builds).
- First-class multi-engine support: docker, docker-compose, podman, podman-compose, nerdctl, and Apple's `container` runtime on macOS.
- Zero-friction SSH agent forwarding into the container.
- Optional copy of host dotfiles (`.gitconfig` always; user-configurable list otherwise).
- Reasonable defaults; light, well-documented config in `~/.config/devcont/config.toml`.

## Non-Goals

- **Not an IDE plugin or GUI.** Devcont stays terminal- and CI-only; VS Code Dev Containers and the JetBrains plugin already cover IDE flows.
- **Not a re-implementation of Docker or Podman.** Devcont orchestrates existing engines; container runtime work is out of scope.
- **Not a full devcontainer Features/Templates registry.** Resolving and composing `features` from the spec is out of scope (at least for now).

## Stakeholders

- **Maintainer:** Erwan Patrick Legrand
- **Contributors:** anyone opening PRs against `main` per `CONTRIBUTING.md`
- **Users:** developers running `cargo install devcont` or building from source, plus CI jobs invoking the binary

## Initial Concept

> Run [devcontainers](https://containers.dev/) from CLI or CI.

(See `README.md` for canonical user-facing documentation.)
