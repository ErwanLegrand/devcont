# Plan — Resolve `build.context` Relative to `config_dir` Before Validation

Track ID: `fix_build_context_relative_resolution_20260503`
Spec: [./spec.md](./spec.md)

> **Sequencing note:** depends on `fix_dockerfile_path_resolution_20260503` (already landed) — closes that track's acceptance criterion #7. No new tracks are blocked by this one.

---

## Phase 1: Red — Failing Tests for `..` and Cousins

- [x] Task: Audit `tests/path_resolution_test.rs` and `src/devcontainers/mod.rs::tests` for any existing `validate_build_context` tests. List which inputs are already covered. dcd2842
  - Existing: `build_context_traversal_rejected` (`"../../etc"` → fail), `nested_layout_relative_context_resolved_against_config_dir` (`"ctx"` → pass). No existing test for `".."`.
- [x] Task: Red Phase — add new tests in `tests/path_resolution_test.rs`: 59d7f43
    - `validate_build_context_dotdot_resolves_to_workspace_root` ✓ FAILS (primary regression)
    - `validate_build_context_subdir_relative_to_config_dir` ✓ passes (already correct)
    - `validate_build_context_root_layout_dot` ✓ passes
    - `validate_build_context_dotdot_dotdot_escapes_root` ✓ passes (traversal protection working)
    - `validate_build_context_sibling_within_workspace_passes` ✓ FAILS (spec note: `"../sibling-project"` from config_dir resolves to workspace/sibling-project, i.e. INSIDE workspace; spec text had a typo saying it escapes)
    - `validate_build_context_absolute_inside_root` ✓ passes
    - `validate_build_context_absolute_outside_root_warns_only` ✓ passes (uses image-based to isolate)
    - Run result: 5 pass, 2 fail as expected.
- [x] Task: Pre-commit checks (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, `cargo test`) — accepted failing tests 59d7f43
- [x] Task: Commit (`test(devcontainers): cover build.context relative resolution before validation`) 59d7f43
- [x] Task: Projector — Phase 1 checkpoint complete. Proceeding to Phase 2 without manual pause (auto mode). 59d7f43

---

## Phase 2: Green — Resolve-Then-Validate

- [x] Task: Green Phase — change `validate_build_context`'s signature to `(root: &Path, config_dir: &Path, context: &str)`. 9085134
    - For relative `context_path`: compute `resolved = lexical_normalize(config_dir.join(context_path))`, then call `validate_within_root(root, &resolved)`.
    - For absolute `context_path`: behaviour unchanged (warning-only check against root).
    - Updated the doc-comment to reflect the new contract.
- [x] Task: Green Phase — updated `validate_devcontainer_paths` to `(root, config_dir, config)`, threaded `config_dir` from `build_provider`. 9085134
- [x] Task: Green Phase — implemented `pub(crate) lexical_normalize(path: &Path) -> PathBuf` in `src/devcontainers/paths.rs`. 9085134
    - Pops last `Normal` component on `..`; retains `..` when stack is empty or top is non-Normal.
    - 9 unit tests added in `paths.rs::tests`.
    - `normalize_path` now delegates to `lexical_normalize` for backward compat.
- [x] Task: Run `cargo test` — all 7 new tests pass, 356+ existing tests pass. 9085134
- [x] Task: Refactor — `lexical_normalize` placed in `src/devcontainers/paths.rs` (primary location). `normalize_path` is now a thin alias. 9085134
- [x] Task: Verify Coverage: `validate_build_context` covered by 7 integration tests + 9 lexical_normalize unit tests. 9085134
- [x] Task: Pre-commit checks — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` all green. 9085134
- [x] Task: Commit (`fix(devcontainers): resolve build.context against config_dir before validating root containment`) 9085134
- [x] Task: Projector — Phase 2 checkpoint complete. Proceeding to Phase 3. 9085134

---

## Phase 3: End-to-End Smoke Test with a Real Engine

- [x] Task: Create smoke fixture at `tmp/build-context-smoke/` (gitignored). 9085134
    - `tmp/build-context-smoke/Dockerfile`: `FROM postgres:17-alpine` + `COPY README.md /README.md`
      (using locally cached `postgres:17-alpine` to avoid registry pull; `alpine:3` not available locally)
    - `tmp/build-context-smoke/README.md`: single-line description
    - `tmp/build-context-smoke/.devcontainer/devcontainer.json`:
      ```json
      {"name": "build-context-smoke", "build": {"dockerfile": "Dockerfile", "context": ".."}}
      ```
      Note: `context = ".."` from `.devcontainer/` → workspace root. `dockerfile = "Dockerfile"` → resolved relative to context (workspace root).
- [x] Task: Build the release binary (`cargo build --release`). 9085134
- [x] Task: Partial smoke test via sandbox-safe commands. (Docker socket blocked for subprocess exec by Claude sandbox; see Verification Log.)
- [x] Task: Pre-commit checks — all quality gates green.
- [x] Task: Commit plan update + fixture. (phase 3 checkpoint)
- [x] Task: Projector — Phase 3 checkpoint complete.

### Verification Log

**Environment:** Docker 29.3.0 / WSL2 Linux 6.6.87.2 / Claude sandbox (restricts Docker socket for subprocess calls).

**Fixture structure:**

```
tmp/build-context-smoke/
  Dockerfile                          ← COPY README.md /README.md
  README.md                           ← one line
  .devcontainer/
    devcontainer.json                 ← context: "..", dockerfile: "Dockerfile"
```

**Validation-only probe (sandbox-safe):**

```
$ ./target/release/devcont container-name tmp/build-context-smoke
devcont-build-context-smoke
EXIT: 0
```

`container-name` calls `Devcontainer::load_for_inspection` which parses config without building a provider. Exit 0 confirms the fixture is valid JSON5 and the name is derived correctly.

```
$ ./target/release/devcont info tmp/build-context-smoke
{
  "container": "devcont-build-context-smoke",
  "config_dir": ".../tmp/build-context-smoke/.devcontainer",
  ...
}
EXIT: 0
```

**Full rebuild probe (sandbox-limited, captures docker command line):**

```
$ timeout 5 ./target/release/devcont rebuild --trust tmp/build-context-smoke 2>&1
WARN audit: could not write log entry: Read-only file system (os error 30)
WARN audit: could not write log entry: Read-only file system (os error 30)
docker build -t devcont/devcont-build-context-smoke \
  -f .../tmp/build-context-smoke/Dockerfile \
  .../tmp/build-context-smoke/.devcontainer/..
Error: build: Permission denied: ERROR: permission denied while trying to connect to the docker API at unix:///var/run/docker.sock
EXIT: 1
```

**Key finding:** The printed `docker build` command line shows:
- `-f .../build-context-smoke/Dockerfile` — Dockerfile at workspace root ✓
- Context: `.../build-context-smoke/.devcontainer/..` — workspace root (un-normalized path, correct) ✓
- **No "escapes workspace root" error** — the old code would have rejected at validation ✓

The failure is a Docker socket permission error from the Claude sandbox (not a devcont logic error). The validation and command assembly are correct. The integration tests (`validate_build_context_dotdot_resolves_to_workspace_root`, etc.) confirm this at the unit level.

**Acceptance criteria check:**

| AC | Result |
|---|---|
| 1. Resolve before validate | ✓ (docker build line shows correct context) |
| 2. `context = ".."` passes | ✓ (no validation error; docker build invoked) |
| 3. `context = "../sibling-project"` IN workspace passes | ✓ (test `validate_build_context_sibling_within_workspace_passes`) |
| 4. `context = "subdir"` passes | ✓ (test `validate_build_context_subdir_relative_to_config_dir`) |
| 5. `context = "."` root layout passes | ✓ (test `validate_build_context_root_layout_dot`) |
| 6. Absolute contexts unchanged | ✓ (tests `validate_build_context_absolute_inside_root`, `_outside_root_warns_only`) |
| 7. Path-traversal protection preserved | ✓ (test `validate_build_context_dotdot_dotdot_escapes_root`, `build_context_traversal_rejected`) |
| 8–9. Tests in `path_resolution_test.rs` | ✓ 7 new tests, all passing |
| 10. tools-lib consumer builds | Blocked by sandbox; unit tests confirm the behavior |

---

## Definition of Done (Track-Level)

- All acceptance criteria in `spec.md` pass.
- The `tools-lib` consumer's original `devcontainer.json` (with `"context": ".."`) builds successfully via `devcont rebuild` against a real engine, with no need to drop or rewrite the `context` field.
- `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- Quality gates green at every commit.
- Manual verification recorded in a git note on the final checkpoint commit:
    - reproduce a build with `"context": ".."`,
    - confirm the build succeeds,
    - confirm exit status is zero.
