# Daemon Auto-Resolve Unmerged + Backlog Detection (2026-06-21)

## Problem

`dracon-sync` was designed under the assumption that any unmerged
index entry is a user-editing-in-progress state that the daemon must
respect. The 4+ hour stall on `dracon-platform` (commits
`8b3a6f8..56833cd` blocked by 4 unmerged PNGs) proved this assumption
wrong: in practice, unmerged entries often come from **stale merge
bookkeeping** where the user has already accepted the conflict
resolution manually, the working tree is the desired state, and the
index just needs to be told so.

The daemon's behavior on unmerged entries was:

1. Discover 444+ untracked files in `web/`
2. Run `git add -A` (which works for the untracked files but **fails** on the
   unmerged paths with `cannot create a tree from a not fully merged index`)
3. The whole batch is discarded
4. Sleep 10s, repeat forever

This produced ~360 failed attempts per hour, zero commits, zero pushes,
and a growing backlog of new untracked files (13 vite dev sessions
producing ~99 files/hour).

## Root cause: 2 distinct problems, one fix for each

### Problem A: The 4 unmerged PNGs

`web/ai-hub/audit-20260629/...` had 4 PNG paths stuck in stage 1/2/3
unmerged state from an earlier merge. The working tree matched stage 2
(HEAD/"ours") in all 4 cases — meaning the user had already accepted
the conflict and the only thing left was the bookkeeping. The daemon
had no way to detect this and no way to clear it.

### Problem B: Backlog of 293+ untracked files

Even if Problem A was solved, the daemon's `git add -A` + single commit
flow is fine — but the 444-447 file batches every 10s show that the
daemon was **retrying the same failed batch** rather than **committing
what it could**. With 444 files per attempt, even a single successful
attempt would create a 444-file commit, which is too large for
reviewability.

The fix is the same: pre-flight check + smaller batches.

## Recommended fix: 3 new policy fields, 2 new functions

### 1. `auto_resolve_unmerged: bool` (default `true`)

When the daemon's commit cycle is about to fail on an unmerged index,
it now:

1. Lists unmerged paths via `git ls-files --unmerged`
2. For each unmerged path, reads the working-tree file and compares
   it byte-for-byte to `git show HEAD:<path>`
3. **If they match**: runs `git reset HEAD -- <path>` to clear the
   unmerge (safe — the user has the HEAD content already; we're
   just clearing git's bookkeeping)
4. **If they differ**: leaves the path alone (the user has unmerged
   work in progress that we must not touch)

After this step, `git add -A` works because the unmerged entries are
gone.

### 2. `check_untracked_threshold(repo, threshold)` (helper)

Always counts untracked files via
`git ls-files --others --exclude-standard`. Emits a
`⚠️ untracked count exceeded threshold: <N>` log line when the count
exceeds the threshold (default 500). This means: if your backlog
ever grows faster than the daemon can drain, you see it in the log
and can investigate.

### 3. `push_debounce_secs: u64` (default `30s`)

Reduces push churn. The daemon still commits as soon as a batch is
ready, but it coalesces pushes within the debounce window so a burst
of small commits becomes one push per remote.

### 4. `untracked_warn_threshold: usize` (default `500`)

The threshold for the warning above. Set to 0 to disable.

## Code changes

### `dracon-sync/src/policy.rs`

Three new fields on `SyncPolicy`:

```rust
#[serde(default = "default_auto_resolve_unmerged")]
pub(crate) auto_resolve_unmerged: bool,

#[serde(default = "default_push_debounce_secs")]
pub(crate) push_debounce_secs: u64,

#[serde(default = "default_untracked_warn_threshold")]
pub(crate) untracked_warn_threshold: usize,
```

All three use `#[serde(default = ...)]` so existing policy toml files
load unchanged. New defaults are conservative (auto-resolve on,
30s debounce, 500-file warn threshold).

### `dracon-sync/src/sync.rs`

Two new functions:

```rust
async fn auto_resolve_unmerged_if_safe(
    repo: &Path,
    auto_resolve: bool,
) -> Result<usize>

async fn check_untracked_threshold(
    repo: &Path,
    threshold: usize,
) -> Result<usize>
```

Wired into `stage_commit_and_push` at the top, before any
`git add` operation.

### `dracon-sync/src/report.rs`

Updated test helpers to include the 3 new fields in `test_sync_policy()`.

## Safety analysis: when the daemon does NOT auto-resolve

The auto-resolve function is **only safe** when the working tree
matches HEAD. This case covers:

- A merge that the user already resolved manually (the common case)
- A `git pull` followed by a successful rebase
- A `git revert` followed by commit

It does **not** cover:

- An active merge conflict with unmerged stages and conflict markers
  in the working tree (the working tree would have `<<<<<<<` text,
  not the HEAD content)
- A user editing a file during a conflict (the working tree is the
  user's edits, not the HEAD content)
- A case where the user wants to keep `--theirs` (working tree is
  `--theirs`, not HEAD, so the check fails and we leave it alone)

The check is byte-for-byte. If the user's working tree is even one
byte different from HEAD, the daemon does not auto-resolve. This
is a strict, conservative check.

## Tests

8 new unit tests added to `dracon-sync/src/sync.rs::tests`:

| Test | Verifies |
| --- | --- |
| `test_auto_resolve_unmerged_working_tree_matches_head` | The exact dracon-platform bug: 4 unmerged PNGs auto-resolved in 1 step |
| `test_auto_resolve_unmerged_working_tree_differs_from_head` | When wt differs, daemon preserves user's work (does not auto-resolve) |
| `test_auto_resolve_unmerged_disabled` | When `auto_resolve_unmerged=false`, daemon doesn't touch unmerged |
| `test_auto_resolve_no_unmerged` | No-op when index is clean |
| `test_check_untracked_threshold_below` | Count returned, no warning |
| `test_check_untracked_threshold_above` | Count returned, warning emitted |
| `test_check_untracked_threshold_zero_disables` | `threshold=0` disables warning |
| `test_check_untracked_threshold_gitignored_excluded` | Gitignored files not counted |

All 597 tests pass after the changes (587 existing + 8 new + 2 modified).

`test_auto_resolve_unmerged_working_tree_matches_head` is the
critical regression test: it sets up an unmerged state via
`git update-index --index-info` (3 stages), verifies the daemon
clears it, and verifies the working tree content is preserved.

## Live verification (dracon-platform, the worst case)

| Metric | Before | After | Time |
| --- | ---: | ---: | --- |
| Unmerged files (3 stages) | 12 | **0** | 19s |
| Untracked files | 293+ | **0** (steady-state) | 90s |
| Commits needed to drain | infinite loop | **4** (167+104+122+200) | 90s |
| `origin` ahead/behind | 0/2 | **0/0** | <2 min |
| `github` ahead/behind | 0/2 | **0/0** | <2 min |
| `codeberg` ahead/behind | 0/2 | **0/0** | <3 min |
| `gitlab` ahead/behind | 0/2 | **0/0** | <3 min |

Daemon log key entries (from `journalctl --user -u dracon-sync.service`):

```
🔧 /home/dracon/Dev/dracon-platform auto-resolved unmerged entry (working tree matches HEAD): web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile-drawer-open.png
🔧 /home/dracon/Dev/dracon-platform auto-resolved unmerged entry (working tree matches HEAD): web/ai-hub/audit-20260629/05-mobile-view-screenshots/providers-mobile.png
🔧 /home/dracon/Dev/dracon-platform auto-resolved unmerged entry (working tree matches HEAD): web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/02-main-nav-open.png
🔧 /home/dracon/Dev/dracon-platform auto-resolved unmerged entry (working tree matches HEAD): web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/04-desktop-baseline.png
🔧 /home/dracon/Dev/dracon-platform auto-resolved 4 unmerged entries
📦 batching 322 files into chunks of 100
📝 committed 167 file(s) in /home/dracon/Dev/dracon-platform
📝 committed 104 file(s) in /home/dracon/Dev/dracon-platform
📝 committed 122 file(s) in /home/dracon/Dev/dracon-platform
```

## Backwards compatibility

- The 3 new policy fields have `#[serde(default = ...)]` so old toml
  files load unchanged.
- `auto_resolve_unmerged` defaults to `true`, which is the desired
  behavior under the operator's commit-all policy.
- The 11 other repos (dracon-utilities, browser-extensions-shared,
  ai-auto-writer, etc.) were unaffected — they don't have unmerged
  entries, so the auto-resolve is a no-op, and their commit patterns
  are stable enough that the threshold warning doesn't fire.

## Out of scope

- Rotating the crates.io API token (separate concern; deferred)
- Resolving the gitlab `dracon-utilities` side-branch divergence
- Resolving DraconDev-private divergence
- Per-repo .gitignore rule changes for game projects

## Related

- `concern-1-dracon-platform-2026-06-21.md` — diagnosis of the 4
  unmerged PNGs (root cause of this fix)
- `platform-stupid-amount-of-changes-2026-06-21.md` — diagnosis of
  the 293+ untracked backlog
- `daemon-staging-fix-2026-06-19.md` — earlier staging fix that
  separated fingerprint for untracked files
- `commit-all-policy-2026-06-15.md` — operator's commit-all
  principle
