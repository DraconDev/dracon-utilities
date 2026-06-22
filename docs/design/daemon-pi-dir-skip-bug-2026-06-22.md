# Daemon `.pi/` Skip Bug (2026-06-22)

## Problem

The `dracon-sync` daemon's `stage_existing_files` recursion
**silently drops any untracked file inside a directory whose
basename starts with `.`**. This was meant to skip ephemeral dirs
like `.cache/`, `.venv/`, and `.direnv/`, but it ALSO skips `.pi/`
— meaning the agent goal-tracking files inside
`*/.pi/goals/archived/*.md` are never auto-committed, even when
they are clearly operator docs that should go up.

This directly violates the operator's commit-all principle
(see `commit-all-principle-2026-06-16.md`):

> "git sync just has to make sure that nothing left out unless we
> have a very good reason to leave it out. 100MB sqlite or build
> file sure, but docs are DEFINITELY going up."

The `.pi/goals/archived/*.md` files are exactly the "docs" the
operator is talking about — they are the durable record of each
pi-goal session (objective, task list, completion evidence,
verification summary). They are not 100MB, they are not build
artifacts, they are not temp files. They are operator docs.

## Root cause

In `dracon-sync/src/sync.rs::stage_existing_files`, the
recursion that walks into untracked directories has this guard:

```rust
// Skip dotfile dirs (.git, .cache, .venv, etc.)
if let Some(name) = cp.file_name().and_then(|n| n.to_str()) {
    if name.starts_with('.') {
        continue;
    }
    ...
}
```

The intent was to skip `.git/`, `.cache/`, `.venv/`, `.direnv/`,
etc. But `.pi/` starts with `.` and is NOT in any of the
gitignored or build-ephemeral categories. The broad
`starts_with('.')` check treats it the same as `.git/`.

The exclusion list `excluded_dir_names` (default: `target`,
`node_modules`, `.cache`, `.direnv`, `.venv`, `dist`, `build`,
`archives`, `.tmp-*`) does NOT contain `.pi/`, and the
operator's policy is `untracked_exclude_patterns = []` (commit
everything untracked by default). So the user's intent and the
excluded list both say "commit `.pi/` files" — but the recursion
guard contradicts them.

## Evidence

### Reproduction (browser-extensions-shared, 2026-06-22)

```bash
$ cd /home/dracon/Dev/browser-extensions-shared
$ git ls-files --others --exclude-standard
all-coupons-one-button/.pi/goals/archived/goal_2026062122562979_mqob5p3n-t0jn73.md
all-coupons-one-button/.pi/goals/archived/goal_2026062201504686_mqoemaz7-e89cg6.md

$ git check-ignore -v all-coupons-one-button/.pi/goals/archived/goal_2026062122562979_mqob5p3n-t0jn73.md
(empty — file is NOT ignored)

$ git add -A all-coupons-one-button/.pi/
$ git diff --cached --name-status
A	all-coupons-one-button/.pi/goals/archived/goal_2026062122562979_mqob5p3n-t0jn73.md
A	all-coupons-one-button/.pi/goals/archived/goal_2026062201504686_mqoemaz7-e89cg6.md
```

Git's view: the files are untracked, not ignored, and `git add`
stages them correctly. So git itself is willing to commit them.

### Daemon's view (with `DRACON_SYNC_DEBUG=1`)

```
$ DRACON_SYNC_DEBUG=1 dracon-sync sync-now /home/dracon/Dev/browser-extensions-shared
ℹ️ skip pull/merge for /home/dracon/Dev/browser-extensions-shared (no origin remote)
🐛 /home/dracon/Dev/browser-extensions-shared status: clean=false modified=0 staged=0 entries(libgit2)=1
🐛 /home/dracon/Dev/browser-extensions-shared to_stage=1 to_restore=0
🐛 /home/dracon/Dev/browser-extensions-shared skipped commit: all changes were filter-only (smudge/clean)
✅ no sync changes /home/dracon/Dev/browser-extensions-shared
```

The daemon sees `entries(libgit2)=1` (the dir
`all-coupons-one-button/.pi/` collapsed to a single entry by
`git status --porcelain`), the recursion in
`stage_existing_files` walks into the dir, hits the
`name.starts_with('.')` skip, bails out, the stage list ends up
empty, `git add` does nothing, `git diff --cached` is empty, the
daemon concludes "all changes were filter-only (smudge/clean)"
and aborts the commit.

The `to_stage=1` is a lie — the daemon thought it was staging
the dir, but the recursion never actually added the files inside.

## Scope (2026-06-22)

Direct untracked `.pi/` files across watched repos:

| Repo | Untracked `.pi/` files |
| --- | ---: |
| `browser-extensions-shared` | 2 |
| `dracon-platform` | 4 |
| **Total** | **6** |

These 6 files are operator docs (goal tracking records) that
should be committed but are silently dropped by the daemon.

Other 12 watched repos have NO untracked `.pi/` files (their
pi-goal records were already committed or never existed).

## Why the dotfile-skip exists

The original author of `stage_existing_files` was worried about
walking into `.git/` (a directory inside the working tree that
points to the actual git dir via `gitdir:` file) and other
ephemeral dot-dirs that should never be auto-staged. The
broad `starts_with('.')` skip is a blunt instrument that
overshoots — it should have been a specific allowlist:

| Skip reason | Directory |
| --- | --- |
| git internals | `.git` |
| build artifacts | `.cache`, `.direnv`, `.venv`, `target`, `build`, `dist`, `archives` |
| JS/TS dependencies | `node_modules` |
| agent temp | `.tmp-*` |
| **.pi/ is NOT any of these** | — |

## Recommended fix

Replace the broad `name.starts_with('.')` skip with the existing
`excluded_dir_names` BTreeSet check. The set already contains all
the dot-dirs we want to skip (`.cache`, `.direnv`, `.venv`) plus
`node_modules`, `target`, `dist`, `build`, `archives`, `.tmp-*`.
We just need to make sure the recursion consults the set instead
of blanket-skipping dot-dirs.

```rust
// Before (buggy):
if name.starts_with('.') {
    continue;
}

// After (fixed):
if excluded.contains(name) {
    continue;
}
```

This is a one-line change. The `excluded.contains(name)` check
already does the right thing: it skips the dots we want to skip
(`.cache`, `.direnv`, `.venv`, etc.) and lets `.pi/` through to
be recursed.

### Why this is safe

- `.git/` is handled by a separate guard earlier in the
  function (`if full_dot_git.is_file() { continue; }`) so it is
  still skipped.
- `.cache/`, `.direnv/`, `.venv/` are in the default
  `excluded_dir_names` set, so they are still skipped.
- `.tmp-*/` is in the set, so it is still skipped.
- Other dot-dirs that the operator might want to ignore
  (e.g. `.vscode/`, `.idea/`) can be added to
  `excluded_dir_names` in the policy file if needed.
- Files that are actually ephemeral (build artifacts, etc.)
  remain gitignored via `.gitignore` and are filtered by
  `git ls-files --others --exclude-standard` before they ever
  reach the recursion.

### New test

Add a unit test in `dracon-sync/src/sync.rs::tests` that:
1. Creates a temp git repo
2. Creates `.pi/goals/archived/goal_test.md` with markdown content
3. Runs the daemon's commit path (or a focused
   `stage_existing_files` wrapper)
4. Asserts that the file ends up in the git index

This is a regression test that catches the bug if the broad
dotfile-skip is ever re-introduced.

## Release plan

This is a daemon bug fix. The fix lands in `dracon-sync`
submodule and is cut as **v0.112.14**:

1. Apply the one-line fix in `src/sync.rs`
2. Add the regression test
3. `cargo test --locked` (597+ tests, all pass)
4. `cargo build --release --locked`
5. `cargo deny check`
6. `scripts/release.sh 0.112.14 --yes` — bump Cargo.toml, publish
   to crates.io, push git tag, create GitHub release
7. Install the new binary at `~/.local/bin/dracon-sync`
8. Restart `dracon-sync.service`

## Related

- `commit-all-policy-2026-06-15.md` — operator's commit-all
  policy
- `commit-all-principle-2026-06-16.md` — operator's stated
  principle: "git sync just has to make sure that nothing left
  out unless we have a very good reason"
- `daemon-auto-resolve-unmerged-2026-06-21.md` — the previous
  daemon fix (v0.112.13), which addressed the unmerged-index
  stall but did NOT address the .pi/ skip
- `concern-repo-investigation-2026-06-21.md` — earlier
  investigation that mentioned `.pi/goals/archived/` files in
  passing
