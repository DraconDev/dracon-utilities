# Dracon Sync Improvement Blueprint

## Status Legend
- [ ] Not started
- [~] In progress
- [x] Completed

---

## Critical Bugs

### 1. Stale `status.ahead` check after commit
- **Location:** `src/main.rs:2041`
- **Problem:** `status` is fetched at line 1863, but the "ahead > 0" check at line 2041 uses stale data. After a commit, the ahead count increases, but we use the old value.
- **Impact:** Push decision based on incorrect ahead count
- **Fix:** Re-fetch status after commit before the push check, or track ahead differently
- **Priority:** High
- **Status:** [ ]

### 2. Activity entry removed after every sync attempt
- **Location:** `src/main.rs:2281` (daemon mode)
- **Problem:** `activity.remove(&repo)` runs regardless of sync success/failure, resetting the inactivity delay timer and causing constant sync attempts on persistent failures.
- **Impact:** CPU waste, log spam, potential rate limiting
- **Fix:** Only remove activity entry on successful sync, or keep track of failure count
- **Priority:** Medium
- **Status:** [ ]

### 3. No maximum retry limit for persistent failures
- **Location:** `run_daemon()` function
- **Problem:** No "give up after N attempts" threshold. A repo with an unresolvable issue could spin forever.
- **Impact:** Resource exhaustion, log bloat
- **Fix:** Add exponential backoff with maximum attempt count, then mark repo as "failed" and skip until manual intervention
- **Priority:** Medium
- **Status:** [ ]

---

## Race Conditions

### 4. No inter-process locking for daemon instances
- **Location:** `run_daemon()`
- **Problem:** Multiple `dracon-sync daemon` processes could run simultaneously, causing conflicting sync operations on the same repos.
- **Impact:** Corrupted git state, duplicate commits, race conditions
- **Fix:** Add PID file or file locking (e.g., `flock`) for daemon singleton enforcement
- **Priority:** High
- **Status:** [ ]

### 5. Policy reload mid-sync could cause inconsistency
- **Location:** `src/main.rs:2143`
- **Problem:** Policy is reloaded every loop iteration. A policy change mid-sync could cause behavior mismatch between pre-sync checks and actual operations.
- **Impact:** Unexpected behavior, potential data loss
- **Fix:** Clone policy at start of each repo iteration, or use RwLock for thread-safe access
- **Priority:** Low
- **Status:** [ ]

### 6. Repo discovery vs sync race
- **Location:** `src/main.rs:2155-2166`
- **Problem:** Repos are discovered, then iterated. If a repo is deleted between discovery and processing, sync operations will fail.
- **Impact:** Error logs, potential panic
- **Fix:** Check repo existence before processing, handle ENOENT gracefully
- **Priority:** Low
- **Status:** [ ]

---

## Deprecation/Migration

### 7. `git filter-branch` is deprecated
- **Location:** `src/main.rs:1773-1784` (`rewrite_ahead_paths`)
- **Problem:** `git filter-branch` is deprecated and may fail on newer git versions, leaving repo in stuck state with backup branch.
- **Impact:** Failed large blob rewrites, stuck repos
- **Fix:** Migrate to `git filter-repo` or BFG Repo-Cleaner
- **Priority:** Medium
- **Status:** [ ]

---

## Error Handling Improvements

### 8. Large blob detection silently ignores failures
- **Location:** `src/main.rs:1991, 2044`
- **Problem:** `detect_large_blobs_ahead(...).unwrap_or_default()` returns empty vec on failure, potentially allowing large blobs to be pushed.
- **Impact:** Push failures due to large files, host rejection
- **Fix:** Log warning on detection failure, consider failing safe (skip push on error)
- **Priority:** Medium
- **Status:** [ ]

### 9. Pull/rebase failure leaves repo in undefined state
- **Location:** `src/main.rs:1819-1833`
- **Problem:** `pull_rebase()` failure only logs a warning and continues. If rebase fails mid-way, the repo could be left with merge conflicts that are not detected/handled.
- **Impact:** Stuck repos, manual intervention required
- **Fix:** Detect rebase conflict state and mark repo as needing repair, don't continue with sync
- **Priority:** High
- **Status:** [ ]

### 10. Incident ledger has no file locking
- **Location:** `src/main.rs:1656-1673`
- **Problem:** Multiple concurrent daemon instances could corrupt the JSONL file. `append(true)` without locking is not atomic.
- **Impact:** Corrupted incident log, lost audit trail
- **Fix:** Use `fs2::FileExt::file_lock` or similar for atomic appends
- **Priority:** Low (mitigated by fixing #4)
- **Status:** [ ]

---

## Edge Cases

### 11. Status inconsistency between libgit2 and CLI fallback
- **Location:** `src/main.rs:1875-1889`
- **Problem:** When fallback CLI entries are used, `status.staged_files` is not recalculated. Only `is_clean`, `modified_files`, and `entries` are updated, leaving inconsistent state.
- **Impact:** Incorrect status reporting, potential commit issues
- **Fix:** Recalculate all status fields in fallback path, or fetch fresh status after fallback
- **Priority:** Low
- **Status:** [ ]

### 12. Cargo.lock-only guardrail may lose previously staged content
- **Location:** `src/main.rs:1915-1922`
- **Problem:** If `stage_paths` contains only `Cargo.lock` after filtering, the code restores all paths and returns early. However, if there were previously staged files (before `add_paths`), they could be incorrectly reverted.
- **Impact:** Lost staged changes
- **Fix:** Check for pre-existing staged content before the restore
- **Priority:** Medium
- **Status:** [ ]

---

## Completed Fixes

### [x] Stalling on excluded-only changes
- **Location:** `src/main.rs:1891-2040`
- **Problem:** When a repo had only excluded changes, `sync_repo()` would skip commit but not clean the dirty state, causing infinite re-processing in daemon mode.
- **Fix:** Partition entries into `to_stage` and `to_restore`. Restore excluded paths after commit or when all changes are filtered.

### [x] Missing dracon-protocols dependency
- **Location:** `Cargo.toml`, imports
- **Problem:** `dracon-protocols` crate doesn't exist in the workspace.
- **Fix:** Use `dracon-git::types` directly instead of protocol types.

### [x] Inconsistent return value on large blob skip
- **Location:** `src/main.rs:1997`
- **Problem:** Returned `Ok(true)` after skipping push due to large blobs, but the non-commit path returns `Ok(false)`.
- **Fix:** Changed to `Ok(false)` for consistency.

---

## Implementation Order (Suggested)

1. **#4 - Daemon singleton lock** - Prevents most concurrency issues
2. **#9 - Handle rebase conflicts** - Critical for reliability
3. **#1 - Stale status.ahead** - Correctness issue
4. **#2 - Activity removal on failure** - Resource efficiency
5. **#7 - Migrate from filter-branch** - Future compatibility
6. **#3 - Max retry limit** - Resilience
7. **#8 - Large blob detection errors** - Safety
8. **#12 - Cargo.lock guardrail** - Edge case
9. **#5, #6, #10, #11** - Lower priority refinements
