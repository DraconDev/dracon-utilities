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

## Code Quality Fixes (Second Pass)

### 13. Incorrect indent calculation in version bumping
- **Location:** `bump_in_section` function
- **Problem:** `line.splitn(2, 'v').next()` was incorrect logic for extracting indentation.
- **Impact:** Broken indentation preservation in Cargo.toml version bumps
- **Fix:** Use `line.chars().take_while(|c| c.is_whitespace()).collect()`
- **Priority:** High
- **Status:** [x]

### 14. Silent value clamping in policy validation
- **Location:** `SyncPolicy::normalize()`
- **Problem:** Timeout values were silently clamped without user notification.
- **Impact:** Users unaware their config was being modified
- **Fix:** Added warning messages when values are adjusted
- **Priority:** Medium
- **Status:** [x]

### 15. Non-existent watch roots silently skipped
- **Location:** `watch_root_paths()`
- **Problem:** Invalid paths in config were silently ignored.
- **Impact:** Configuration errors hidden from users
- **Fix:** Added warning message for non-existent paths
- **Priority:** Medium
- **Status:** [x]

### 16. Redundant proto conversion functions
- **Location:** `to_proto_status`, `to_proto_entries`
- **Problem:** Identity transformations after removing dracon-protocols dependency.
- **Impact:** Dead code, confusion
- **Fix:** Simplified to `s.clone()` and `entries.to_vec()`
- **Priority:** Low
- **Status:** [x]

### 17. Limited lockfile detection
- **Location:** `is_lockfile_path()`
- **Problem:** Only detected `Cargo.lock`, not other common lockfiles.
- **Impact:** Lockfile noise from other ecosystems would be committed
- **Fix:** Expanded to detect package-lock.json, yarn.lock, pnpm-lock.yaml, poetry.lock, composer.lock, Gemfile.lock, go.sum
- **Priority:** Medium
- **Status:** [x]

### 18. TOML parse errors silently ignored
- **Location:** `load_repo_override()`
- **Problem:** Parse errors returned default without logging.
- **Impact:** Debugging difficulty for misconfigured repos
- **Fix:** Added warning on parse failure
- **Priority:** Medium
- **Status:** [x]

### 19. Cargo.lock update errors silently ignored
- **Location:** `bump_patch_version_in_repo()`
- **Problem:** `unwrap_or(false)` swallowed errors.
- **Impact:** Silent failures in lockfile updates
- **Fix:** Log error on failure
- **Priority:** Medium
- **Status:** [x]

### 20. Confusing nested if structure
- **Location:** `run_repair_concerns()` push handling
- **Problem:** Double-nested `if` without proper braces was hard to read.
- **Impact:** Maintainability, potential bugs
- **Fix:** Fixed indentation and braces
- **Priority:** Low
- **Status:** [x]

---

## Summary

**Total Completed:** 20 issues

**Remaining (Low Priority - 3):**
- #5 - Policy reload race
- #6 - Repo discovery race
- #11 - Status inconsistency in CLI fallback
