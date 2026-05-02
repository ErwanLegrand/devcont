# Project Tracks

This file tracks all major tracks for the project. Each track has its own detailed plan in its respective folder under `projector/tracks/`.

A **track** is a high-level unit of work — typically a feature, refactor, or bug fix scoped to ship as a coherent change.

---

## [ ] Track: Fix `start` orchestration error cascade
*Link: [./tracks/fix_start_orchestration_cascade_20260503/](./tracks/fix_start_orchestration_cascade_20260503/)*
Type: bug · Severity: polish (masks blocker bugs) · Sequencing: land first

## [ ] Track: Fix `build.dockerfile` path resolution
*Link: [./tracks/fix_dockerfile_path_resolution_20260503/](./tracks/fix_dockerfile_path_resolution_20260503/)*
Type: bug · Severity: blocker (every spec-compliant `devcontainer.json` fails) · Sequencing: land after the cascade fix so users see a single clear error

## [ ] Track: Add an inspect-shaped subcommand (`container-name`, `info`)
*Link: [./tracks/add_inspect_subcommand_20260503/](./tracks/add_inspect_subcommand_20260503/)*
Type: feature · Severity: high · Sequencing: independent — can land in parallel
