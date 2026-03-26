# Claude Code Instructions

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
