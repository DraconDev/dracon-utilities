# Release Notes — v0.112.34 (2026-07-22) — F1.16 + config cleanup

**Headline**: Two operator-approved fixes closing the audit's
remaining decision items. **820 daemon tests** (+2), clippy + deny
clean.

---

## 1. Excluded-path semantics: preserve edits by default (F1.16)

`auto_commit_exclude_patterns` now means **"don't auto-commit these
files"** — nothing more. After each commit, the daemon **unstages**
excluded files (so its own `git add -A` doesn't sweep them into the
operator's next manual commit) but **preserves their worktree
content**. Operator edits to excluded files stay on disk, visible in
`git status` as modified-unstaged.

Before v0.112.34, `restore_excluded_paths` ran
`git restore --staged --worktree` on excluded files after every
commit — **silently deleting the operator's uncommitted edits**
(audit F1.16: "data loss as the silent default of a knob named
'exclude from auto-commit'").

Operators who WANT hygiene enforcement ("these files must always
equal HEAD") opt in per-repo:

```toml
# .dracon/dracon-sync.toml
revert_excluded_to_head = true
```

Destructive behavior requires an explicit opt-in; it is never the
silent default. Two regression tests: default preserves edits;
opt-in reverts to HEAD. Documented in AGENTS.md ("Excluded-path
semantics" section).

## 2. Live config cleanup (audit M20/F3.2 follow-up)

The operator's `~/.dracon/utilities/sync/dracon-sync.toml`:

- `standard_files_auto = true` moved ABOVE the `[[standard_files]]`
  blocks — TOML silently absorbs trailing bare keys into the last
  table entry, so the field was being ignored (harmless while it
  matched the default, a trap the day it changed).
- `[extra_remotes]` section deleted — it mapped to no `SyncPolicy`
  field and was silently dropped by the parser. If extra remotes are
  ever needed, that's a real feature (serde field + tests), not a
  zombie section.

Verified: `tomllib` parse shows the field is now top-level;
`dracon-sync config validate` passes with zero warnings (the M20
absorbed-field warning is gone).

---

## Test discipline

- `cargo test --workspace --locked` ✅ **820 daemon** (+2), warden 83,
  security ~111, system 86 — 0 failed
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean
