# Claude Code Instructions

## Branch Model

- **`trunk` is the only canonical branch.** All work targets `trunk`. PRs merge into `trunk`.
- Before starting any non-trivial work, confirm `git rev-parse --abbrev-ref HEAD` is `trunk`. If not, `git checkout trunk` first.
- If you encounter stale `main` or `dev` refs locally or in old session memory, they have been deleted (2026-05-04) and should be ignored.

## Spawning Agents in Worktrees

The `Agent` tool's `isolation: "worktree"` creates a worktree forked from the **parent's current HEAD**, not from `trunk`. Stale worktrees under `.claude/worktrees/agent-*` may also be on branches that diverged from `trunk` long ago. Both lead to agent commits that conflict on integration even when no real code conflict exists.

**Required pre-flight before any `isolation: "worktree"` agent spawn from this repo:**

1. **Commit anything you want agents to fork from to local `trunk` first.** Uncommitted edits, sub-worktree-branch commits, and stash entries are *not* visible to agents.
2. Confirm the parent's local `trunk` is the intended fork base: `git rev-parse trunk` from the trunk worktree shows the SHA you expect.
3. **Instruct each spawned agent to align its worktree with the parent's local `trunk` before reading specs:**

   > Before reading any spec, run `git fetch origin && git reset --hard trunk`. This pins your branch's base to the parent's *local* `trunk` tip, including any commits the parent has made but not yet pushed. Local `trunk` is shared across all worktrees of the same repo, so this resolves to whatever the parent's trunk worktree most recently committed.

   ⚠ **Use `trunk` (local), not `origin/trunk`.** `origin/trunk` only reflects the last push; the parent may have committed unpushed work on top of it. Resetting to `origin/trunk` would silently drop those commits from the agent's view.

4. Before integrating an agent's branch, verify `git merge-base trunk <agent-branch>` is recent (i.e., a commit on `trunk`). If it's deep history (e.g., the initial commit), the agent forked wrong — prefer cherry-pick over merge to avoid pulling in divergent history.

Skip `git push --force` to recover from a misaligned base; cherry-pick the deliverable commits onto `trunk` instead. Treat misaligned bases as a recoverable diagnostic, not a blocker.

## Git Commits

- **Always use** `git commit -m 'message'` directly — no command substitution, no heredocs, no temp files.
- For multi-line messages, use multiple `-m` flags (each becomes a paragraph):
  ```bash
  git commit -m 'subject line' -m 'body paragraph'
  ```
- Single-line: `git commit -m 'subject line'`

## File Reading and Editing

- **Never use `sed` to read files** — use the Read tool instead (supports `offset`/`limit` for specific line ranges).
- **`sed` for in-place string substitution is fine** (e.g., `sed -i 's/foo/bar/g' file`), unless the Edit tool suits the case better.
- Use the Edit tool for targeted, reviewable changes to existing files.
- Reserve Bash for commands that have no dedicated tool equivalent.

## Temp Files

- **Never write to `/tmp`** — that path requires user approval for every write.
- If temp files are needed, create a `tmp/` directory in the workspace root and add it to `.gitignore`.
