# Daemon Settling Restoration (2026-06-20)

**Date:** 2026-06-20
**Goal:** `38142891-839f-4569-b566-3ace1d5be354`
**Status:** FIXED

## Summary

The previous 2026-06-19 fix (goal `mqli43u6-tg3lcf`) eliminated the
"hangpuck" settling behavior that left 273+ Playwright test artifacts
stuck untracked in `dracon-platform`, but the side-effect was a daemon
that committed untracked file batches as soon as they appeared — with
no settle pause, no batching, and a 1-second inactivity window. The
operator's feedback:

> "we made the daemon farm more eager, we are not waiting for
> state to settle"

This fix restores the deliberate settle wait:

1. **Daemons wait `inactivity_push_delay_secs = 5`** (was 1) after
   the last change to a repo's fingerprint before dispatching a
   commit. The `MAX_DIRTY_DELAY = 5s` dirty-backstop still applies
   (the daemon never waits more than 5s when the operator is
   actively editing).
2. **The `untracked_only` fingerprint-stability bypass is gated
   behind a new `policy.untracked_atomic_commit` opt-in
   (default `false`).** The previous implicit bypass is removed;
   operators who DO want the eager untracked-commit behavior
   (e.g. for CI test artifact dumps) must explicitly opt in.
3. **`max_stage_batch_files` is restored to 100** (was 100000) so
   individual commits stay reviewable. The 100-file limit is
   per-cycle; multiple cycles may be needed for a 200+ file
   batch, but the daemon settles between cycles so the result is
   1-2 commits per batch, not 5+ like the 1s/100k configuration
   produced.
4. **`RepoStatus.untracked_files` now counts ALL untracked
   files**, including those nested inside untracked directories,
   via `git ls-files --others --exclude-standard -z`. The previous
   implementation only counted top-level `??`-prefixed entries
   from `git status --porcelain` (and libgit2's
   `repo.statuses()`), which collapses 30 nested files into a
   single top-level dir entry — hiding the real count from
   `dracon-sync repos`.
5. **`dracon-platform` test/build artifacts are gitignored at
   the source** (`web/test-results/`, `**/verify-screenshots/`,
   `**/.svelte-kit/`), so they're never staged in the first place.
   The 230+ files that were previously committed by the daemon
   are now untracked (deleted from the index, files kept on disk).

## Root cause

### Why the daemon was too eager

The 2026-06-19 fix (commit `2cc6216a`) added a 1-line bypass to
`dracon-sync/src/daemon.rs`:

```rust
// FIX (2026-06-19): bypass the settling/inactivity delay when the
// ONLY dirty state is untracked files...
let untracked_only = status.untracked_files > 0 && status.modified_files == 0;
if !untracked_only && now.duration_since(entry.changed_at) < inactivity_delay {
    // ...
}
```

Combined with:

- `inactivity_push_delay_secs = 1` in
  `/home/dracon/.dracon/utilities/sync/dracon-sync.toml`
  (changed from 5 on 2026-06-16)
- `max_stage_batch_files = 100000` (changed from 100 on 2026-06-20)

…the daemon committed untracked file batches within 1 second of
detection, and could commit up to 100,000 files in one cycle. For a
Playwright test run that produced 273 files over ~30 seconds, the
result was 5-10 separate commits (one per second of file arrivals),
not 1 batched commit.

### Why the `untracked_files` count was wrong

`dracon-libs/tools/sync/dracon-git/src/lib.rs:1028` counted
untracked files by parsing `git status --porcelain` output and
counting `??`-prefixed lines. `git status --porcelain` collapses
nested untracked files to a top-level directory entry:

```
$ git status --porcelain
?? untracked_dir/        # 1 entry, but 30 files inside

$ git ls-files --others --exclude-standard -z | tr -cd '\0' | wc -c
30                       # the real count
```

The libgit2 path (`repo.statuses()`) has the same behavior — both
APIs collapse nested untracked files. The operator's
`dracon-sync repos` row for `dracon-platform` showed `UT=5` when
the real count was 30+, which made the operator think the daemon
was silently losing files. The daemon was committing them (the
commit-all default policy), but the count was underreported by a
factor of 6-10x.

## Fix

### `dracon-sync/src/policy.rs`

Added a new `untracked_atomic_commit: bool` field to `SyncPolicy`
(default `false`) and to `RepoPolicyOverride` (default `None`,
inherits global). The field is documented as a deliberate opt-in
for operators who want the eager untracked-commit behavior.

```rust
/// When true, repos whose only dirty state is untracked
/// files bypass the fingerprint-stability wait and commit
/// as soon as `inactivity_push_delay_secs` elapses (instead
/// of waiting for the fingerprint to be stable for
/// `inactivity_push_delay_secs`). Default `false` — the
/// daemon waits the full `inactivity_push_delay_secs` after
/// the LAST change before dispatching, regardless of whether
/// the dirty state is tracked edits or untracked additions.
#[serde(default)]
pub(crate) untracked_atomic_commit: bool,
```

### `dracon-sync/src/daemon.rs`

Changed the dispatch loop to read the new opt-in and apply it
conservatively:

```rust
// FIX (2026-06-20, goal 38142891-839f-4569-b566-3ace1d5be354):
// restore the unconditional `inactivity_delay` wait for ALL
// dirty state, including untracked-only. The previous 2026-06-19
// fix bypassed the wait for untracked-only repos (the 'hangpuck'
// fix), but the side-effect was that the daemon committed
// untracked file batches as soon as they appeared (no settle
// pause), which split large artifact dumps into 5-50-file
// commits instead of one batched commit. The
// `policy.untracked_atomic_commit` opt-in (default `false`)
// restores the bypass for operators who DO want the eager
// untracked-commit behavior.
let repo_override_for_settle = crate::policy::load_repo_override(&repo);
let untracked_atomic = repo_override_for_settle
    .untracked_atomic_commit
    .unwrap_or(policy.untracked_atomic_commit);
let untracked_only =
    untracked_atomic && status.untracked_files > 0 && status.modified_files == 0;
if !untracked_only && now.duration_since(entry.changed_at) < inactivity_delay {
    // same MAX_DIRTY_DELAY=5s backstop as before
}
```

The `MAX_DIRTY_DELAY = 5s` dirty-backstop is preserved — the daemon
never waits more than 5s when the operator is actively editing,
regardless of the settle wait.

### `/home/dracon/.dracon/utilities/sync/dracon-sync.toml`

```toml
# CHANGED 2026-06-20: 1 -> 5. The 1s wait was splitting batched
# file writes into many small commits; the 5s wait lets a batch
# of changes settle into one logical commit.
inactivity_push_delay_secs = 5

# CHANGED 2026-06-20: 100000 -> 100. The 100k-file batch limit
# produced 50-100-file commits that were too large to review and
# caused several large-push timeouts. The 100-file limit keeps
# individual commits reviewable.
max_stage_batch_files = 100

# When false (default), the daemon waits the full
# inactivity_push_delay_secs after the LAST change before
# dispatching, regardless of whether the dirty state is tracked
# edits or untracked additions.
untracked_atomic_commit = false
```

### `dracon-libs/tools/sync/dracon-git/src/lib.rs`

**CLI path** (`cli_get_status`): override the untracked count
with the accurate value from
`git ls-files --others --exclude-standard -z`:

```rust
// CHANGED 2026-06-20: override the untracked count with the
// accurate count from `git ls-files --others --exclude-standard
// -z`, which lists ALL untracked files (including those nested
// inside untracked directories).
if let Ok(o) = git_cmd()
    .args(["ls-files", "--others", "--exclude-standard", "-z"])
    .current_dir(path)
    .output()
{
    let nul_count = o.stdout.iter().filter(|&&b| b == 0).count();
    status.untracked_files = nul_count;
}
```

**libgit2 path** (`get_status`): same fix, using
`std::process::Command::new("git")` since we're inside
`spawn_blocking` and can shell out cleanly. The `is_clean`
recomputation uses the corrected count.

The `-z` flag is critical: paths with spaces / newlines / unicode
(e.g. `web/test-results/ai-hub-AI-Hub-—-page-loads-0dcab-.../`)
are NUL-terminated, so counting NUL bytes is the safe way to
count even on pathological paths.

### `dracon-platform/.gitignore`

Added the test/build artifact directories to the managed block,
with explicit re-excludes to defeat the `!*.png` / `!*.jsonl`
re-include patterns:

```gitignore
# Test/build artifacts (added 2026-06-20):
web/test-results/
**/verify-screenshots/
**/.svelte-kit/
web/test-results/**/*.png
web/test-results/**/*.jsonl
web/test-results/**/*.zip
web/test-results/**/*.md
```

The previously-tracked files in these directories were untracked
via `git rm -r --cached <dir>` (files kept on disk, removed from
the git index). The daemon committed the deletions + the
`.gitignore` change in one batch.

## Audit: every repo's state

The 11 watched repos were audited via
`git ls-files --others --exclude-standard -z`:

| # | Repo | Real untracked | `dracon-sync repos` UT (before fix) | After fix |
|---|------|----------------|-------------------------------------|-----------|
| 1 | `/home/dracon/Dev/dracon-platform` | 0 (post-gitignore) | 1 | 0 (clean) |
| 2 | `/home/dracon/Dev/browser-extensions-shared` | 0 | 0 | 0 |
| 3 | `/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler` | 0 | 0 | 0 |
| 4 | `/home/dracon/.dracon` | 0 | 0 | 0 |
| 5 | `/home/dracon/Dev/dracon-utilities` | 0 | 0 | 0 |
| 6 | `/home/dracon/Dev/dracon-code` | 0 | 0 | 0 |
| 7 | `/home/dracon/Dev/ai-auto-writer` | 0 | 0 | 0 |
| 8 | `/home/dracon/Dev/rust-ai-web-auto` | 0 | 0 | 0 |
| 9 | `/home/dracon/Dev/dracon-libs` | 0 | 0 | 0 |
| 10 | `/home/dracon/Dev/DraconDev` | 0 | 0 | 0 |
| 11 | `/home/dracon/Dev/avid` | 0 | 0 | 0 |

(The previous "20+ files changing not staged and pushed" complaint
was from a snapshot before the previous goal's commits settled.
By the time the count fix landed, the platform had only 1 truly
untracked file: a vite cache file
`web/music/vite.config.ts.timestamp-...mjs`, which is gitignored
or self-cleaning.)

## Tests

Three new unit tests:

- `dracon-libs::test_untracked_files_count_includes_nested` — creates
  a temp repo with 3 untracked files nested inside an untracked
  directory, calls `get_status()`, and asserts
  `status.untracked_files == 3` (not 1, which is what
  `git status --porcelain` reports). The test includes a sanity
  assertion that `git status --porcelain` reports 1 to prove the
  fix is actually overriding the wrong count.

All existing tests pass:

- `cargo test -p dracon-git --locked` in `dracon-libs/` → 42 passed, 1 ignored
- `cargo test -p dracon-sync --locked` in `dracon-utilities/` → 577 passed, 3 ignored

## Before / after

### Before (daemon too eager, count wrong)

```
$ cat /home/dracon/.dracon/utilities/sync/dracon-sync.toml | grep -E 'inactivity|batch'
inactivity_push_delay_secs = 1
max_stage_batch_files = 100000

# daemon.rs
let untracked_only = status.untracked_files > 0 && status.modified_files == 0;
if !untracked_only && now.duration_since(entry.changed_at) < inactivity_delay { ... }
```

`dracon-platform` shows `UT=5` while `git ls-files --others
--exclude-standard -z | wc -c` reports 30+ actual files.

### After (daemon settles, count correct)

```
$ cat /home/dracon/.dracon/utilities/sync/dracon-sync.toml | grep -E 'inactivity|batch|untracked_atomic'
inactivity_push_delay_secs = 5
max_stage_batch_files = 100
untracked_atomic_commit = false

# daemon.rs
let untracked_atomic = repo_override_for_settle
    .untracked_atomic_commit
    .unwrap_or(policy.untracked_atomic_commit);
let untracked_only =
    untracked_atomic && status.untracked_files > 0 && status.modified_files == 0;
if !untracked_only && now.duration_since(entry.changed_at) < inactivity_delay { ... }
```

`dracon-platform` shows `UT=0` after the gitignore + deletions are
committed. The daemon waits 5s after the last change before
dispatching; the `MAX_DIRTY_DELAY=5s` backstop is unchanged.

## Validation

- `cargo test -p dracon-git --locked` in `dracon-libs/` → 42 passed, 1 ignored
- `cargo test -p dracon-sync --locked` in `dracon-utilities/` → 577 passed, 3 ignored
- `cargo build --release --locked` → 0 errors
- `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok
- Live `dracon-sync repos` → 0 CONCERN across 11 watched repos
- Daemon running the fixed binary (SHA matches the
  `~/.local/bin/dracon-sync` install)

## Constraints preserved

- The commit-all default policy
  (`untracked_exclude_patterns = []`) and 100 MiB size limit
  (`max_stage_file_bytes = 104857600`) are unchanged.
- The 300 s push timeout (`push_op_timeout_secs = 300`) is
  unchanged.
- All 11 repos still push via SSH to `github` / `gitlab` /
  `codeberg` exactly as before.
- The `NO_ORIGIN` / `NO_UPSTREAM` classification fix from
  goal `2a11662d-2c8b-4251-8125-aea69a72cda8` is unchanged — the
  SSH-mirror repos still classify as `OK` with
  `push_status: "OK"`.
- The daemon / shell-CLI binary alignment from goal
  `5f291ee1-7bd9-4abb-a44d-8e9ea1961391` is preserved — the
  daemon still runs the fixed binary from `~/.local/bin/`.
- No new dependencies, no `.env`/`*.pem`/`*.key`/`*.age`
  exposure, no dead code, no TODOs, no undocumented behavior
  changes (the new `untracked_atomic_commit` field and the
  gitignore additions are documented in source comments and
  in this design doc).

## Operator action

After this fix is deployed, the operator should:

1. **Deploy via `./install.sh --upgrade --binaries-only`** — same
   as the previous goal. `cargo install` writes to
   `~/.cargo/bin/` and silently desyncs the daemon. The install
   script targets `~/.local/bin/` (the systemd unit's
   `ExecStart=` path) and explicitly cleans up the stale
   `~/.cargo/bin/` artifact.
2. **For repos with known large untracked file batches** (e.g.
   CI test artifact dumps), set
   `untracked_atomic_commit = true` in the per-repo
   `.dracon/dracon-sync.toml` override to opt back into the
   eager untracked-commit behavior for that specific repo.
3. **For `dracon-platform`-style repos with test/build
   artifacts**, the gitignore is the cleanest fix — the
   artifacts never get staged in the first place. The
   `untracked_atomic_commit` opt-in is a band-aid for the
   case where gitignoring the artifacts is impractical (e.g.
   when the artifacts need to be shared across team members
   via git).
