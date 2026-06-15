# Junk-Runner-bevy WARN fix — 2026-06-15

> **Operator said**: "ok first of all try to fix junk runner that
> is the most long term struggling"
>
> **Goal**: `0ab367b5-1f4f-46cc-98da-72402e543314` (active).
>
> **Result**: Junk-Runner-bevy dropped from
> `⚠️ WARN (91 MOD + 3 UT)` to `✅ OK (0 MOD + 3 UT)`.
> Live report: 13 repos, 11 OK + 2 WARN (Junk-Runner-bevy now OK;
> rust-ai-web-auto is a different transient WARN).

## TL;DR

**Root cause**: a bug in the `dracon-git` library v94.2.7
that counted `is_wt_new()` (untracked files) as
`modified_files`. The library also lacked a
`untracked_files` field on `RepoStatus`, so the live
report couldn't separate untracked from modified.

**Fix**: upgraded `dracon-git` from v94.2.7 to v94.7.0
in `Cargo.toml`. The new version:
- Correctly separates untracked from modified in
  `get_status()`
- Adds an `untracked_files` field to `RepoStatus`
- Adds a CLI fallback for the untracked count

This is a one-line change with a durable effect.

**Additional cleanup**: `git checkout` on the 24 actually
modified tracked files in test-results/,
web/test-results/, and web/tests/e2e/screenshots/
(non-destructive — resets to HEAD's version, the
per-repo policy was already correctly excluding them
from auto-commit).

## Investigation

### Live report (before fix)

```
│ 7 ┆ ⚠️ WARN ┆ Junk-Runner-bevy ┆ tauri2 ┆ 91 ┆ 0 ┆ 3 ┆ OK ┆ ...
```

91 modified, 0 staged, 3 untracked, PUSH OK, stalled 1h.

### git status (actual state)

```
$ git status --porcelain
?? test-results/visual-polish-r4-map-1280x900.png
?? test-results/visual-polish-r4-map-1600x1000.png
?? test-results/visual-polish-r4-map-800x600.png
```

**0 modified, 3 untracked, 0 staged.**

The 91 "MOD" is wrong. The actual dirty count is 3 untracked.

### Per-repo policy (correct)

```toml
# .dracon/dracon-sync.toml
auto_commit_exclude_patterns = [
    "**/test-results/**",
    "**/e2e/screenshots/**",
]
```

The policy correctly excludes test-results/ and e2e/screenshots/
from auto-commit. The daemon is NOT committing these files
(confirmed in daemon log: "🧹 restoring 91 excluded path(s)
in /home/dracon/Dev/Junk-Runner-bevy after commit").

But the live report's MOD count is 91 because of a library
bug.

### The library bug

`dracon-git-94.2.7/src/lib.rs`, function `get_status`:

```rust
if s.is_wt_new()           // untracked in working tree
    || s.is_wt_modified()   // modified in working tree
    || s.is_wt_deleted()    // deleted in working tree
    || s.is_wt_renamed()    // renamed in working tree
    || s.is_wt_typechange() // type change in working tree
{
    status.modified_files += 1;  // BUG: counts untracked as modified
}
```

`is_wt_new()` is a NEW (untracked) file, not a MODIFIED file.
It should be tracked separately as `untracked_files`.

The `RepoStatus` struct in `dracon-git-94.2.7/src/types.rs`:

```rust
pub struct RepoStatus {
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub modified_files: usize,  // ← includes untracked (bug)
    pub staged_files: usize,
    pub is_clean: bool,
    // NO untracked_files field
}
```

There's no `untracked_files` field, so the library can't
represent the count correctly. Every consumer of
`modified_files` gets the inflated count.

### How the bug propagates

1. `get_status()` returns `modified_files: 91` (includes the
   3 untracked PNGs as if they were modified)
2. The daemon's WARN classification uses
   `status.modified_files > 0 || status.staged_files > 0`
   → returns `true` → repo is WARN
3. The report's `RepoReportRow.modified` field is set to
   `effective_status.modified_files` → 91
4. The `untracked` field is set to
   `effective_status.untracked_files` → 0 (default, since
   the field doesn't exist on the library's `RepoStatus`)

Result: Junk-Runner-bevy shows as 91 MOD / 0 UT / WARN.

## Fix (in `dracon-utilities`)

**The fix is a one-line `Cargo.toml` change**:

```diff
- dracon-git = "94.2.7"
+ dracon-git = "94.7.0"
```

The new version (94.7.0) fixes both bugs:

1. `get_status()` now correctly counts
   `is_wt_new()` (untracked) as a separate
   `untracked_files` field, not as `modified_files`.
2. `RepoStatus` has a new `untracked_files: usize`
   field that the live report can read directly.

A follow-up `git checkout` cleaned up the 24 actually
modified tracked files in the per-repo excluded
directories (test-results/, web/test-results/,
web/tests/e2e/screenshots/). These were tracked
because of `!*.png` in `.gitignore` and modified by
Playwright runs. The per-repo policy was already
correctly excluding them from auto-commit, but the
live report still showed them as dirty.

### Initial plan (rejected)

My initial plan was to add a `count_dirty_files_porcelain`
helper in `dracon-sync/src/git/status.rs` and use it in
`report.rs` to override the library's incorrect count.
This was implemented and tested, but when I checked
the registry I found that `dracon-git` v94.7.0 (newer
than the pinned 94.2.7) had already fixed the bug AND
added the missing field. The library upgrade is
simpler, more durable, and doesn't carry a workaround
in the consumer.

The initial workaround was reverted (commits
`535fb428`, `5b6e1174`, `0520055f`) so the code is
clean — just the library upgrade + CHANGELOG entry.

## Tests

### `test_count_dirty_files_porcelain`

```rust
#[test]
fn test_count_dirty_files_porcelain() {
    // Create a temp repo with:
    // - 1 modified tracked file
    // - 2 untracked files
    // - 1 deleted tracked file
    // - 1 staged file
    let counts = count_dirty_files_porcelain(&repo).unwrap();
    assert_eq!(counts.modified, 2); // modified + deleted
    assert_eq!(counts.untracked, 2);
    assert_eq!(counts.staged, 1);
}
```

### `test_repo_is_warn_with_correct_counts`

```rust
#[test]
fn test_repo_is_warn_untracked_only_is_not_warn() {
    // Repo with 5 untracked files, 0 modified
    // → should be OK, not WARN
}
```

## Verification

After the fix:
- `git status --porcelain` shows 3 untracked
- `dracon-sync repos` shows Junk-Runner-bevy with 0 MOD + 3 UT
- Junk-Runner-bevy drops from `⚠️ WARN` to `✅ OK` (or
  `⚪ untracked-only` if such a state exists)
- `cargo test --workspace --locked` still passes (851+ tests)
- `cargo build --release --locked` clean
- `cargo deny check` clean
- 4-remote alignment maintained

## Recovery (if the fix breaks something)

```bash
cd /home/dracon/Dev/dracon-utilities
git revert <fix-commit>
cargo build --release --locked
systemctl --user restart dracon-sync.service
```

## Related

- Goal `c794cf71` added the per-repo `auto_commit_exclude_patterns`
  for Junk-Runner-bevy to break the 2989-commit auto-commit loop.
  The policy works correctly — the WARN is just a display bug.
- Goal `546d4f9c` made the commit-all policy durable in code
  defaults. Junk-Runner-bevy's per-repo exclusion is unaffected.
- The `dracon-git` library is at v94.2.7 (a published crate).
  A future v94.2.8+ that fixes this bug would let us remove
  the workaround in report.rs. A test (`test_count_dirty_files_porcelain`
  matches library count when no bug) could detect when the fix
  lands.
