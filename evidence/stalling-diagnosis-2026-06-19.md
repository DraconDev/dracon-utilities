# Stalling Diagnosis - 273 Untracked Playwright Files (2026-06-19)

## Problem

273 untracked Playwright test artifacts in `dracon-platform`
(`web/test-results/.playwright-artifacts-0/`) are stuck. The
daemon is committing 1-55 files at a time but never catches up,
resulting in a persistent "settling" state and trailing-drain loop.

## Evidence

### Journalctl pattern (last 10 minutes)

```
23:23:31 📝 committed 55 file(s) in /home/dracon/Dev/dracon-platform
23:23:31 🔄 trailing-drain: clearing 1 stuck in_flight entries
23:23:56 📝 committed 2 file(s)
23:23:58 🔄 trailing-drain: clearing 1 stuck in_flight entries
23:25:12 📝 committed 3 file(s)
23:25:14 🔄 trailing-drain: clearing 1 stuck in_flight entries
... (repeats every 25-30 seconds for 6+ minutes)
```

**Observation**: The daemon commits a small batch, then immediately
gets stuck in trailing-drain. The cycle never completes because new
files are being added faster than the daemon can commit them.

## Root Cause

### 1. Fingerprint stability check (daemon.rs:1839-1840)

The daemon computes a "fingerprint" for each repo:
```rust
let fingerprint = format!(
    "{}:{}:{}:{}:{}",
    status.branch,
    effective_dirty as u8,
    status.staged_files,
    status.ahead,
    status.behind
);
```

When the fingerprint changes, the daemon waits for stability
(`inactivity_delay`). With 273 files being added/removed rapidly by
Playwright, the fingerprint changes constantly, so the daemon never
sees stability.

### 2. MAX_DIRTY_DELAY = 5s (daemon.rs:1856)

If a repo has been dirty for >5s, the daemon commits regardless of
fingerprint. But the 5s window is too short when Playwright is
actively writing files - the fingerprint changes every 5s, resetting
the timer.

### 3. settling_max_delay_secs = 60s (daemon.rs:806, default)

After 60s of continuous dirty state, the daemon commits regardless
of fingerprint. But 60s is too long when there are 273 files to
commit.

### 4. Trailing-drain (daemon.rs:2213)

The daemon clears stuck in_flight entries after each commit, but
the entry immediately gets re-added because the repo is still
dirty (still has untracked files).

## Why the daemon commits in small batches (1-55 files)

The daemon processes repos one at a time. For dracon-platform:
1. Cycle 1: Fingerprint changes, wait 5s, commit 55 files
2. Trailing-drain clears stuck in_flight
3. Cycle 2: Fingerprint changes again (218 files left), wait 5s, commit 2-10 files
4. Trailing-drain clears stuck in_flight
5. Repeat

The daemon never gets to commit all 273 files at once because:
- The fingerprint changes between cycles
- The settling wait is applied per-cycle, not per-batch
- The trailing-drain is clearing entries that immediately get re-added

## Systemic Fix Design

### Option A: Separate untracked file fingerprint (RECOMMENDED)

Add a new fingerprint component for untracked file count:
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

**Why this works**: Untracked file additions are atomic (new files
appear all at once), so the fingerprint stabilizes immediately. No
need to wait for "stability" - if untracked_files > 0, commit
immediately.

### Option B: Reduce settling_max_delay_secs to 5s

Change the default from 60s to 5s so the daemon commits sooner.

**Why this doesn't work as well**: The 5s fingerprint stability
wait is applied to ALL dirty state, not just untracked files. This
would also reduce the wait for tracked file edits, which could
cause half-written files to be committed.

### Option C: Bypass settling for untracked files

Add a check: if the only dirty state is untracked files, skip the
fingerprint stability wait and commit immediately.

**Why this works**: Untracked files don't have the "half-written"
problem that tracked file edits have. New files are atomic.

## Recommended Fix: Option A (separate fingerprint)

### Changes required

1. **daemon.rs:1795-1807**: Add `status.untracked_files` to the
   fingerprint format string
2. **daemon.rs status struct**: Ensure `untracked_files` is
   populated by the status check
3. **sync.rs status check**: Add `git ls-files --others
   --exclude-standard | wc -l` to the status output
4. **Tests**: Add a test for the new fingerprint behavior

### Expected behavior after fix

- 273 untracked files → fingerprint changes once (273 ≠ 0)
- Daemon waits 5s for stability
- Fingerprint stable (no new files added)
- Daemon commits all 273 files in one batch
- No more trailing-drain loop
- No more "settling" state

## Verification

- `git ls-files --others --exclude-standard | wc -l` = 0 in all 13
  repos within 10 minutes of files being created
- Daemon log shows single large commits (not 1-55 file batches)
- No "trailing-drain: clearing stuck in_flight entries" messages
  for untracked file commits
- All 4 remotes at ahead=0, behind=0
