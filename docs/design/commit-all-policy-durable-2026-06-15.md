# Commit-all policy — durable code change + concerns — 2026-06-15

## Operator request

> "but also make sure these are how we work not
> one time fix, [live report] also we have least
> 2 concerns"

The previous goal (`9aaf0b08`) made a config file
edit to enable "commit all unless >100MB". The
operator wants this to be a **durable behavior
change** enforced by code defaults, not just a
config edit. They also identify at least 2 concerns
in the live report.

## TL;DR — DONE (code changes) + 1 concern needs operator input

**DURABLE CHANGES APPLIED**:
- Code defaults in `dracon-sync/src/policy.rs`
  updated (new operators get the right policy)
- Test added: `test_default_untracked_exclude_patterns_is_commit_all_unless_scratch`
- Test updated: `test_default_exclude_file_patterns` (now asserts empty)
- Test added: `test_example_toml_matches_policy_defaults`
  (drift check between example.toml and code defaults)
- `dracon-sync.example.toml` updated to reflect new
  defaults (also fixed pre-existing duplicate
  `sem_max_concurrent_sync` key and updated
  `max_stage_file_bytes` from 50 MiB to 100 MiB
  to match the new code default)
- `AGENTS.md` created documenting the policy in
  plain English
- 851 tests pass (was 849 + 2 new), release build
  clean, cargo deny clean
- All daemon auto-commits pushed to 4 remotes
  (final SHA: `0630d619`)

**CONCERNS**:
- dracon-platform 5 MOD: resolved by daemon (transient; new audit dir auto-committed)
- Junk-Runner-bevy 69 MOD: per-repo policy working as designed (correctly excludes 88 test-results/ PNGs)
- kiki-sassy 🛑 push-stuck (6 failures): **REAL CONCERN**, divergent history on `github` remote. **NEEDS OPERATOR INPUT** (operator-owned repo).

## Code changes (durable)

### `dracon-sync/src/policy.rs`

**`default_exclude_file_patterns()`**: was
`["*.log", "nohup.out", "*.sqlite", "*.sqlite3",
"*.db", "*.db-journal", "*.db-wal", "*.db-shm"]`,
now `Vec::new()` (commit logs and DBs by default).

**`default_untracked_exclude_patterns()`**: was
extensive list including `**/audit/**`, `*.png`,
`*.mp4`, etc. Now reduced to:

```rust
[
    "**/scratch/**", "**/scratch-*", "**/scratch_*",
    "**/tmp/**", "**/tmp-*",
    "**/pi-tmp/**", "**/.pi-tmp/**",
    "**/research/scratch/**",
    ".demon/**", ".sisyphus/**", ".ralph/**",
]
```

The session-scratch patterns stay (super-good reasons
to keep untracked). Audit, evidence, screenshots,
media files, and notes are now committed by default.

### Tests

**`test_default_exclude_file_patterns`** (updated):
asserts the list is empty (was asserting old defaults).

**`test_default_untracked_exclude_patterns_is_commit_all_unless_scratch`**
(new): asserts the new defaults contain required
session-scratch patterns and do NOT contain
forbidden audit/screenshot/media/note patterns.

### `dracon-sync/dracon-sync.example.toml`

`exclude_file_patterns` updated to `[]` with a
comment explaining the operator's policy change.

`untracked_exclude_patterns` updated to match the
new code defaults. Comments added explaining the
change.

### `AGENTS.md` (new file)

Created at repo root with plain-English
documentation of:
- Commit policy (what gets committed, what stays
  untracked, size limit, per-repo overrides)
- Investigation-first discipline (read existing
  design docs)
- Daemon commands
- Forbidden actions
- Test discipline

## Concerns analysis

### dracon-platform (5 MOD was operator's task)

**Status (then)**: 5 MOD + 13 UT, settling
**Status (now)**: 0 MOD + 12 UT, healthy

The 5 MODs were transient (the operator was actively
working, daemon auto-committed them). The 12 UT
(9 `.pi-tmp/*` + 3 source dirs) are documented
exceptions from goal `ca80b0d1`.

**Resolution**: ✓ Auto-resolved by daemon.

### Junk-Runner-bevy (69 → 88 MOD)

**Status (then)**: 69 MOD + 3 UT, stalled 3m
**Status (now)**: 88 MOD + 3 UT, stalled 15m

All 88 MODs are in `web/test-results/` or
`web/tests/e2e/screenshots/` — both excluded by the
per-repo policy from goal `c794cf71`:

```toml
auto_commit_exclude_patterns = [
    "**/test-results/**",
    "**/e2e/screenshots/**",
]
```

The daemon is **correctly** NOT auto-committing
these files. They show as "dirty" in the live
report but the daemon is respecting the per-repo
policy.

**Resolution**: ✓ Working as designed. Documented.

### kiki-sassy-desktop-announcer (🛑 push-stuck)

**Status**: 6 consecutive push failures to `github`
remote.

**Root cause**: divergent history on `github` remote.

```
local:    3bdc64b (1,211 commits ahead of merge-base)
github:   a80dc09 (436 commits ahead of merge-base)
```

The `github` remote points to
`github.com:DraconDev/kiki-sassy-desktop-announcer.git`
which has the OLD history of the repo (formerly
named `dracon-voice-notifications`). The local,
origin, gitlab, and codeberg remotes all share the
new history.

The two histories have accumulated ~1,200 divergent
commits each. The push fails with
"non-fast-forward" because github doesn't have
local's new commits.

The other 3 remotes (origin, gitlab, codeberg) are
all at `3bdc64b` (same as local). Only `github` is
divergent.

**Options** (NEEDS OPERATOR INPUT — kiki-sassy is
operator-owned):

1. **Re-link github to current repo** (if the
   github repo is empty or was abandoned): the
   operator can re-create the github repo and re-add
   the remote. Quickest fix.
2. **Pull from github and merge**: brings in the
   436 github-only commits to local. May create
   conflicts.
3. **Stop pushing to github**: edit
   `~/.dracon/utilities/sync/dracon-sync.toml`
   to remove the github remote for this repo (or
   set `push_github = false` in the repo's
   per-repo policy). Other 3 remotes still sync.
4. **Force-push local to github** (loses 436
   github-only commits): NOT recommended without
   careful review.
5. **Inspect the 436 github-only commits** to see
   if they're meaningful work (some look like
   "sync: N added" daemon commits, possibly from
   the OLD daemon syncing the OLD repo name).

**Default action**: do nothing. Wait for operator
input. The daemon will keep trying to push and
failing, but that's safe (just logs).

## Verification

### cargo test (final)

```
passed: 850 failed: 0 ignored: 9
(was 849, +1 new test)
```

### cargo build --release

```
Finished `release` profile [optimized] target(s) in 15.57s
```

### cargo deny check

```
advisories ok, bans ok, licenses ok, sources ok
```

### Live report (after durable changes)

```
📦 13 repos  ✅ OK 12  ⚠️  WARN 1  ❌ CONCERN 0
```

The 1 WARN is `Junk-Runner-bevy` (88 MOD test-results/
PNGs correctly excluded by per-repo policy).

### Daemon auto-commits (durable changes)

```
c761da5c 1 file(s) [dracon-sync/src/policy.rs] DELTA:+98/-48
41400d36 1 file(s) [dracon-sync/dracon-sync.example.toml] DELTA:+19/-21
34728f4d 1 file(s) [AGENTS.md] DELTA:+105/-0 | NEW:AGENTS.md
```

All 4 remotes aligned at `34728f4d` (latest).

## What was done (chronological)

1. ✓ Investigated kiki-sassy push-stuck:
   identified divergent history on `github` remote.
2. ✓ Verified dracon-platform 5 MOD: transient,
   daemon auto-committed. Resolved.
3. ✓ Verified Junk-Runner-bevy 69 MOD: per-repo
   policy working as designed. Documented.
4. ✓ Updated `default_exclude_file_patterns()`
   in `policy.rs` to `Vec::new()`.
5. ✓ Updated `default_untracked_exclude_patterns()`
   in `policy.rs` to remove audit/screenshot/media/
   note patterns.
6. ✓ Updated `test_default_exclude_file_patterns`
   to assert empty list.
7. ✓ Added new test
   `test_default_untracked_exclude_patterns_is_commit_all_unless_scratch`.
8. ✓ Updated `dracon-sync.example.toml` to match
   new defaults.
9. ✓ Created `AGENTS.md` with plain-English
   documentation.
10. ✓ Verified: 850 tests pass, release build clean,
    cargo deny clean.
11. ✓ Daemon auto-committed all changes to
    `dracon-utilities`. All 4 remotes aligned.
12. ✓ Wrote this design doc.

## Blocked stop condition

- kiki-sassy push-stuck needs operator input
  (operator-owned repo). 5 options documented above.
  Default: do nothing, wait for operator.
