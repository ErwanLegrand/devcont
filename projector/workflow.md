# devcont — Project Workflow

## Guiding Principles

1. **The Plan is the Source of Truth.** All work is tracked in the relevant track's `plan.md`.
2. **The Tech Stack is Deliberate.** Changes to the tech stack must be documented in `tech-stack.md` *before* implementation.
3. **Test-Driven Development.** Write failing tests first, then implement.
4. **High Code Coverage.** Aim for ≥80% line coverage on changed code (`cargo llvm-cov`).
5. **CI-Aware.** Every command runs non-interactively. Use `CI=true` where appropriate.
6. **Conventional Commits.** Commit messages follow the conventional-commits format so `git-cliff` produces an accurate `CHANGELOG.md`.

## Task Workflow

All tasks follow a strict lifecycle.

### Standard Task Workflow

1. **Select Task** — pick the next available `[ ]` task from the track's `plan.md`.
2. **Mark In Progress** — change `[ ]` to `[~]` in `plan.md`.
3. **Red Phase (failing tests).**
   - Add test(s) in a `#[cfg(test)] mod tests` block, a sibling `tests.rs`, or under `tests/` for integration tests.
   - Run `cargo test --workspace -- <test_name>` and confirm failure.
4. **Green Phase (minimal implementation).**
   - Write the minimum code to make the failing tests pass.
   - Run `cargo test --workspace` and confirm all tests pass.
5. **Refactor (optional).** Clean up with tests as a safety net; rerun `cargo test`.
6. **Verify Coverage.** Run `cargo llvm-cov --workspace --html` and confirm changed code is ≥80% covered.
7. **Document Deviations.** If implementation diverges from `tech-stack.md`, **stop**, update `tech-stack.md` with a dated note, then resume.
8. **Pre-Commit Checks:**
   ```bash
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo check --workspace --all-targets
   cargo test --workspace
   cargo audit
   ```
9. **Commit Code Changes.** Stage code, commit with a conventional-commits message (e.g., `feat(podman): support userns_mode = host`).
10. **Attach Task Summary with Git Notes.**
    - Get the just-committed hash: `git log -1 --format='%H'`.
    - Draft a note: task name, summary of changes, files touched, and the "why".
    - Attach: `git notes add -m '<note>' <hash>`.
11. **Update Plan.**
    - In the track's `plan.md`, change the task line from `[~]` to `[x]` and append the 7-character short SHA.
    - Commit the plan update: `projector(plan): mark task '<name>' as complete`.

### Phase Completion Verification and Checkpointing Protocol

**Trigger:** executed immediately after a task that concludes a phase in `plan.md`.

1. **Announce Protocol Start.**
2. **Ensure Test Coverage for Phase Changes.**
   - Determine phase scope: find the previous phase's checkpoint SHA in `plan.md`.
   - List changed files: `git diff --name-only <previous_checkpoint_sha> HEAD`.
   - For each `.rs` file in scope, verify a corresponding test exists (a `tests::*` mod, `tests/<name>.rs`, or runnable example). Create one if missing, matching repo conventions.
3. **Run Automated Tests with Proactive Debugging.**
   - Announce the exact command: `CI=true cargo test --workspace --all-targets`.
   - Execute it. On failure, attempt up to two fixes; if it still fails, stop and ask for guidance.
4. **Propose a Manual Verification Plan.**
   - Tie steps to the phase's user-facing goals (from `product.md`, `product-guidelines.md`, `plan.md`).
   - Format example for a CLI feature:
     ```
     The automated tests have passed. For manual verification:
     1. Build:   cargo build --release
     2. Run:     ./target/release/devcont rebuild --no-cache  # in a project with .devcontainer/devcontainer.json
     3. Confirm: container is destroyed and rebuilt without using cached layers.
     ```
5. **Await Explicit User Confirmation.** "Does this meet your expectations? Please confirm with yes or provide feedback on what needs to be changed."
6. **Create Checkpoint Commit.** Stage all changes (or empty commit if none). Commit with `projector(checkpoint): checkpoint end of Phase X`.
7. **Attach Verification Report via Git Notes** to the checkpoint commit (test command, manual steps, user's confirmation).
8. **Record Phase Checkpoint SHA in `plan.md`.** Append `[checkpoint: <sha7>]` to the phase heading.
9. **Commit Plan Update.** `projector(plan): mark phase '<NAME>' as complete`.
10. **Announce Completion.**

### Quality Gates

Before marking any task complete:

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo check --workspace --all-targets` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo audit` reports no advisories
- [ ] Coverage ≥80% on changed code (`cargo llvm-cov`)
- [ ] All public items have doc-comments
- [ ] Code follows `code_styleguides/rust.md` and `code_styleguides/general.md`
- [ ] Security-sensitive changes get a `security-reviewer` pass

## Development Commands

### Setup
```bash
cargo fetch                          # download deps
cargo install cargo-llvm-cov         # coverage tooling (one-time)
cargo install cargo-deny             # supply-chain audit (one-time)
cargo install cargo-fuzz             # fuzzing (one-time, optional)
pre-commit install                    # wire up pre-commit hooks (one-time)
```

### Daily Development
```bash
cargo check --workspace --all-targets   # fast typecheck
cargo test --workspace                  # run all tests
cargo fmt                               # format
cargo clippy --workspace --all-targets  # lint
cargo run -- <args>                     # run the CLI locally
cargo xtask <command>                   # project-specific automation
```

### Before Committing
```bash
cargo fmt --check && \
cargo clippy --workspace --all-targets -- -D warnings && \
cargo check --workspace --all-targets && \
cargo test --workspace && \
cargo audit
```

## Testing Requirements

### Unit Testing
- Each module has a `#[cfg(test)] mod tests` block or a sibling `tests.rs`.
- Use `temp-env` for env-var manipulation; never mutate `std::env` directly.
- Mock subprocess calls behind a thin trait abstraction; never exec real `docker`/`podman`/`nerdctl` in unit tests.
- Cover both success and failure paths.

### Integration Testing
- Lives under `tests/`.
- Spin up real container engines only behind a feature flag or as `#[ignore]`-by-default tests, runnable in dedicated CI jobs.
- Snapshot expected CLI output for `--help` and key commands.

### Fuzzing
- Targets live under `fuzz/`.
- Surface area: `devcontainer.json` parsing (JSONC), config TOML, and any string interpolation that flows into subprocess arguments.

## Code Review Process

### Self-Review Checklist
1. **Functionality** — feature works as specified; edge cases handled; error messages user-friendly.
2. **Code Quality** — follows `code_styleguides/`; clear names; no clippy warnings.
3. **Testing** — unit and (where applicable) integration coverage; ≥80% on changed code.
4. **Security** — no secrets; subprocess args correctly quoted/escaped; no shell-injection vectors; user-supplied paths validated.
5. **Performance** — no needless allocations on hot paths; subprocess invocations batched where reasonable.
6. **CI/CD** — all GitHub Actions jobs green.

## Commit Guidelines

### Message Format
```
<type>(<scope>): <description>

[optional body]
```

### Types
- `feat` — new feature
- `fix` — bug fix
- `docs` — documentation only
- `style` — formatting/whitespace (no logic change)
- `refactor` — refactor without behavior change
- `perf` — performance improvement
- `test` — tests only
- `chore` — maintenance / deps / tooling
- `ci` — CI configuration
- `projector` — projector workflow artifacts (plans, checkpoints)

### Examples
```bash
git commit -m 'feat(podman): add userns_mode = host option'
git commit -m 'fix(json): handle trailing commas in devcontainer.json'
git commit -m 'test(config): cover XDG_CONFIG_HOME override'
git commit -m 'projector(plan): mark task `add userns_mode` as complete'
```

## Definition of Done

A task is complete when:

1. Code is implemented to spec.
2. Unit (and integration where applicable) tests are written and passing.
3. Coverage ≥80% on changed code.
4. Documentation is updated (README, doc-comments, CHANGELOG via git-cliff).
5. `cargo fmt`, `cargo clippy`, `cargo check`, `cargo audit`, `cargo test` all green.
6. Plan updated with `[x]` and short SHA.
7. Conventional-commits message; git note attached with task summary.

## Emergency Procedures

### Critical Bug in a Released Version
1. Branch from `main` (e.g., `hotfix/<issue>`).
2. Write a failing test reproducing the bug.
3. Implement the minimal fix.
4. Run the full quality-gate suite.
5. Open a PR; tag and release once merged.
6. Document in the relevant track's `plan.md`, or open a new bug-fix track.

### Security Vulnerability
1. Triage privately; do not file a public issue (see `SECURITY.md`).
2. Rotate any exposed secrets immediately.
3. Patch on a private branch; coordinate disclosure with the reporter.
4. Cut a patched release; publish a `RUSTSEC` advisory if applicable.
5. Document remediation in `SECURITY.md` updates.

## Continuous Improvement

- Review this workflow whenever a process pain-point recurs.
- Open a `chore(workflow)` PR to update — workflow changes belong in version control like everything else.
- Keep things simple; resist process inflation.
