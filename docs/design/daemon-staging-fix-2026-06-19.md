# Daemon Staging Fix Design (2026-06-19)

## Problem

The daemon's fingerprint stability check treats untracked file
additions the same as tracked file edits. This causes the daemon to
wait for stability when new files are being added rapidly (e.g.,
Playwright test artifacts), resulting in a trailing-drain loop that
commits 1-55 files per cycle and never catches up.

## Recommended Fix: Separate Fingerprint for Untracked Files

### Why this is the right systemic fix

1. **Untracked file additions are atomic** - new files appear all
   at once, so the fingerprint stabilizes immediately after a
   batch of new files is added. No need to wait for "stability".

2. **Tracked file edits are not atomic** - a half-written file
   could be committed if the daemon doesn't wait for stability.
   The current 5s wait is correct for tracked file edits.

3. **The fix is minimal** - one additional component in the
   fingerprint format string, one additional field in the status
   struct, one additional command in the status check.

### Changes required

#### 1. daemon.rs:1795-1807 - Add untracked_files to fingerprint

```rust
let fingerprint = format!(
    "{}:{}:{}:{}:{}:{}",
    status.branch,
    effective_dirty as u8,
    status.staged_files,
    status.ahead,
    status.behind,
    status.untracked_files  // NEW
);
```

#### 2. daemon.rs status struct - Add untracked_files field

Add `untracked_files: u64` to the `RepoStatus` struct.

#### 3. sync.rs status check - Populate untracked_files

Add `git ls-files --others --exclude-standard | wc -l` to the
status check and store the count in `status.untracked_files`.

#### 4. Tests - Add fingerprint test

Add a test that verifies:
- Adding a new untracked file changes the fingerprint
- The fingerprint includes untracked_files in its format

### Expected behavior after fix

**Before fix** (current behavior):
- 273 untracked files → daemon commits 1-55 per cycle
- Trailing-drain loop, never catches up
- "settling" state persists for hours

**After fix**:
- 273 untracked files → fingerprint changes once (273 ≠ 0)
- Daemon waits 5s for stability
- Fingerprint stable (no new files added in 5s)
- Daemon commits all 273 files in one batch
- No trailing-drain loop
- No "settling" state

## Alternative Fixes Considered

### Option B: Reduce settling_max_delay_secs to 5s

**Rejected**: The 5s fingerprint stability wait is applied to ALL
dirty state, not just untracked files. Reducing it to 5s would
also reduce the wait for tracked file edits, which could cause
half-written files to be committed.

### Option C: Bypass settling for untracked files

**Considered but not chosen**: This would require adding a check
in the dispatch logic to skip the settling wait when the only
dirty state is untracked files. This is more complex than Option A
and harder to test.

### Option D: Config-only fix (no daemon release)

**Rejected**: The fingerprint format is hardcoded in daemon.rs.
There's no config option to add a fingerprint component. A
config-only fix would require a daemon release anyway.

## Tradeoffs

### Pros of Option A

- **Minimal change**: One line in the fingerprint format, one
  field in the status struct, one command in the status check
- **Testable**: Easy to add a test for the new fingerprint
  behavior
- **Backwards compatible**: Old daemon versions ignore the new
  fingerprint component
- **Systemic**: Fixes the root cause, not just the symptom

### Cons of Option A

- **Requires daemon release**: Need to build and release a new
  version of dracon-sync
- **Slight performance overhead**: Status check now runs
  `git ls-files --others --exclude-standard | wc -l` on every
  cycle (negligible, <10ms)

## Implementation Plan

1. **Make the daemon code changes** (daemon.rs, sync.rs)
2. **Add tests** (daemon.rs test module)
3. **Build and release** dracon-sync v0.113.0 (or next version)
4. **Update the systemd service** to use the new version
5. **Verify** the daemon commits all 273 files in one batch
6. **Commit and push** the design doc and code changes

## Verification

- `git ls-files --others --exclude-standard | wc -l` = 0 in all
  13 repos within 10 minutes of files being created
- Daemon log shows single large commits (not 1-55 file batches)
- No "trailing-drain: clearing stuck in_flight entries" messages
  for untracked file commits
- All 4 remotes at ahead=0, behind=0
- New daemon version installed and running
