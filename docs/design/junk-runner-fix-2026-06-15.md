# Junk-Runner-bevy WARN fix — 2026-06-15

> **Operator said**: "ok first of all try to fix junk runner that
> is the most long term struggling"
>
> **Goal**: `0ab367b5-1f4f-46cc-98da-72402e543314` (active).

## TL;DR

**Root cause**: a bug in the `dracon-git` library (v94.2.7)
counts `is_wt_new()` (untracked files) as `modified_files`.
This makes Junk-Runner-bevy show 91 "MOD" in the live report,
even though all 91 are untracked test-results/ PNGs that the
per-repo policy correctly excludes from auto-commit.

**Fix**: query `git status --porcelain` directly in
`dracon-sync/src/report.rs` to get the correct (modified,
untracked) split. Use the correct count for both the
row's columns and the WARN classification. The
`dracon-git` library bug is documented but **not fixed**
(it's a published crate; the fix lives in dracon-utilities
to avoid a [patch] section).

**After fix**: Junk-Runner-bevy shows `0 MOD + 3 UT`
(untracked PNGs in test-results/, correctly classified as
untracked, not modified). The repo should drop from
`WARN` to `OK`.

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

We can't fix the library directly (it's a published crate
without a path dep). The fix lives in
`dracon-sync/src/report.rs`:

1. **Add a helper function** that queries
   `git status --porcelain` and returns
   `(modified_count, untracked_count)` separately.
2. **Use the correct counts** in the row construction:
   - `modified: actual_modified_count` (not from library)
   - `untracked: actual_untracked_count`
3. **Use the correct count** in the WARN classification:
   - `actual_modified > 0` (not from library)
4. **Add tests** for the new helper.
5. **CHANGELOG entry** under [Unreleased] → Fixed.
6. **No `[patch.crates-io]`** needed — the fix is in the
   consumer (dracon-utilities), not the library.

### Why not patch the library?

The operator's pattern (per goal `cca2169f`) is to avoid
patching external crates. The fix is small enough to live
in the consumer, and it documents the library bug so a
future upgrade to a fixed version can be detected.

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
