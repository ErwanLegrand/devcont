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

## [ ] Track: Surface captured stderr in `format_exec_error` fallback
*Link: [./tracks/improve_format_exec_error_stderr_20260503/](./tracks/improve_format_exec_error_stderr_20260503/)*
Type: feature · Severity: medium (UX) · Sequencing: depends on `fix_start_orchestration_cascade_20260503` (already landed); closes the open follow-up in that track's Verification Log

## [ ] Track: Investigate `tests/integration.rs` failures
*Link: [./tracks/investigate_integration_test_failures_20260503/](./tracks/investigate_integration_test_failures_20260503/)*
Type: bug · Severity: medium · Sequencing: independent — can run in parallel with the format_exec_error track

## [ ] Track: Resolve `build.context` relative to `config_dir` before validation
*Link: [./tracks/fix_build_context_relative_resolution_20260503/](./tracks/fix_build_context_relative_resolution_20260503/)*
Type: bug · Severity: blocker (spec-compliant `"context": ".."` rejected) · Sequencing: closes acceptance criterion #7 of `fix_dockerfile_path_resolution_20260503`; independent of the up-command track

## [ ] Track: Add `devcont up` subcommand for non-attached start
*Link: [./tracks/add_devcont_up_command_20260503/](./tracks/add_devcont_up_command_20260503/)*
Type: feature · Severity: high (interop — blocks programmatic callers) · Sequencing: independent — can land in parallel with the build.context fix

## [ ] Track: `build.dockerfile` resolution must be independent of `build.context`
*Link: [./tracks/fix_dockerfile_path_independent_of_context_20260504/](./tracks/fix_dockerfile_path_independent_of_context_20260504/)*
Type: bug · Severity: blocker (spec violation when both fields are set) · Sequencing: closes a wrong acceptance criterion (#3) from `fix_dockerfile_path_resolution_20260503`; independent of all other open tracks
