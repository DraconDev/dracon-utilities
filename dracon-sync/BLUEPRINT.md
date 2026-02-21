# Dracon Sync Improvement Blueprint

## Status Legend
- [ ] Not started
- [~] In progress
- [x] Completed

---

## Critical Bugs

### 1. Stale `status.ahead` check after commit
- **Location:** `src/main.rs`
- **Problem:** `status` is fetched early, but the "ahead > 0" check uses stale data after commit.
- **Impact:** Push decision based on incorrect ahead count
- **Fix:** Re-fetch status after commit before the push check
- **Priority:** High
- **Status:** [x]

### 2. Activity entry removed after every sync attempt
- **Location:** `src/main.rs` (daemon mode)
- **Problem:** `activity.remove(&repo)` ran regardless of sync success/failure, resetting the inactivity delay timer.
- **Impact:** CPU waste, log spam, potential rate limiting
- **Fix:** Only remove activity entry on successful sync, track failure count
- **Priority:** Medium
- **Status:** [x]

### 3. No maximum retry limit for persistent failures
- **Location:** `run_daemon()` function
- **Problem:** No "give up after N attempts" threshold. A repo with an unresolvable issue could spin forever.
- **Impact:** Resource exhaustion, log bloat
- **Fix:** Added MAX_FAILURES constant (5), repo is skipped after exceeding
- **Priority:** Medium
- **Status:** [x]

---

## Race Conditions

### 4. No inter-process locking for daemon instances
- **Location:** `run_daemon()`
- **Problem:** Multiple `dracon-sync daemon` processes could run simultaneously.
- **Impact:** Corrupted git state, duplicate commits, race conditions
- **Fix:** Added fs2 file locking via `acquire_daemon_lock()`
- **Priority:** High
- **Status:** [x]

### 5. Policy reload mid-sync could cause inconsistency
- **Location:** Daemon loop
- **Problem:** Policy is reloaded every loop iteration.
- **Impact:** Unexpected behavior, potential data loss
- **Fix:** Clone policy at start of each repo iteration, or use RwLock
- **Priority:** Low
- **Status:** [ ]

### 6. Repo discovery vs sync race
- **Location:** Repo iteration
- **Problem:** If a repo is deleted between discovery and processing, sync operations will fail.
- **Impact:** Error logs, potential panic
- **Fix:** Check repo existence before processing, handle ENOENT gracefully
- **Priority:** Low
- **Status:** [ ]

---

## Deprecation/Migration

### 7. `git filter-branch` is deprecated
- **Location:** `rewrite_ahead_paths`
- **Problem:** `git filter-branch` is deprecated and may fail on newer git versions.
- **Impact:** Failed large blob rewrites, stuck repos
- **Fix:** Added git-filter-repo detection with fallback to filter-branch
- **Priority:** Medium
- **Status:** [x]

---

## Error Handling Improvements

### 8. Large blob detection silently ignores failures
- **Location:** `detect_large_blobs_ahead` callers
- **Problem:** `unwrap_or_default()` returned empty vec on failure, potentially allowing large blobs.
- **Impact:** Push failures due to large files, host rejection
- **Fix:** Properly propagate errors, skip push on detection failure
- **Priority:** Medium
- **Status:** [x]

### 9. Pull/rebase failure leaves repo in undefined state
- **Location:** `sync_repo()`
- **Problem:** `pull_rebase()` failure only logged a warning and continued.
- **Impact:** Stuck repos, manual intervention required
- **Fix:** Added conflict state detection (rebase/merge/cherry-pick), skip sync and return early
- **Priority:** High
- **Status:** [x]

### 10. Incident ledger has no file locking
- **Location:** Incident ledger writes
- **Problem:** Multiple concurrent daemon instances could corrupt the JSONL file.
- **Impact:** Corrupted incident log, lost audit trail
- **Fix:** Mitigated by #4 (daemon singleton lock)
- **Priority:** Low
- **Status:** [x] (mitigated)

---

## Edge Cases

### 11. Status inconsistency between libgit2 and CLI fallback
- **Location:** Fallback CLI path
- **Problem:** When fallback CLI entries are used, `status.staged_files` is not recalculated.
- **Impact:** Incorrect status reporting, potential commit issues
- **Fix:** Recalculate all status fields in fallback path
- **Priority:** Low
- **Status:** [ ]

### 12. Cargo.lock-only guardrail may lose previously staged content
- **Location:** Cargo.lock guardrail check
- **Problem:** If `stage_paths` contains only `Cargo.lock`, the code could revert pre-existing staged files.
- **Impact:** Lost staged changes
- **Fix:** Check for pre-existing staged content before restoring
- **Priority:** Medium
- **Status:** [x]

---

## Completed Fixes

### [x] Stalling on excluded-only changes
- **Location:** `src/main.rs`
- **Problem:** When a repo had only excluded changes, `sync_repo()` would skip commit but not clean the dirty state.
- **Fix:** Partition entries into `to_stage` and `to_restore`. Restore excluded paths after commit or when all changes are filtered.

### [x] Missing dracon-protocols dependency
- **Location:** `Cargo.toml`, imports
- **Problem:** `dracon-protocols` crate doesn't exist in the workspace.
- **Fix:** Use `dracon-git::types` directly instead of protocol types.

### [x] Inconsistent return value on large blob skip
- **Location:** `src/main.rs`
- **Problem:** Returned `Ok(true)` after skipping push due to large blobs, inconsistent with non-commit path.
- **Fix:** Changed to `Ok(false)` for consistency.

---

## Summary

**Completed:** 10 issues
- #1, #2, #3, #4, #7, #8, #9, #10, #12 + original stalling fix

**Remaining (Low Priority):**
- #5 - Policy reload race
- #6 - Repo discovery race  
- #11 - Status inconsistency in CLI fallback
