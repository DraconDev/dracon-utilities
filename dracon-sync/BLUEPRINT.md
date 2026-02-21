# Dracon Sync Improvement Blueprint

## Status Legend
- [ ] Not started
- [~] In progress
- [x] Completed

---

## Critical Bugs (All Fixed)

### 1. Stale `status.ahead` check after commit
- **Status:** [x] Re-fetch status after commit for accurate push decision

### 2. Activity entry removed after every sync attempt
- **Status:** [x] Only remove activity entry on successful sync, track failure count

### 3. No maximum retry limit for persistent failures
- **Status:** [x] Added MAX_FAILURES constant (5), repo is skipped after exceeding

### 4. No inter-process locking for daemon instances
- **Status:** [x] Added fs2 file locking via `acquire_daemon_lock()`

### 5. Pull/rebase failure leaves repo in undefined state
- **Status:** [x] Added conflict state detection (rebase/merge/cherry-pick), skip sync and return early

### 6. Large blob detection silently ignores failures
- **Status:** [x] Properly propagate errors, skip push on detection failure

---

## Code Quality Fixes

### 7. Incorrect indent calculation in version bumping
- **Status:** [x] Fixed to use proper whitespace extraction

### 8. Silent value clamping in policy validation
- **Status:** [x] Added warning messages when values are adjusted

### 9. Non-existent watch roots silently skipped
- **Status:** [x] Added warning message for non-existent paths

### 10. Redundant proto conversion functions
- **Status:** [x] Simplified to `s.clone()` and `entries.to_vec()`

### 11. Limited lockfile detection
- **Status:** [x] Expanded to detect 8 common lockfile types

### 12. TOML/Cargo.lock parse errors silently ignored
- **Status:** [x] Added warning on parse failure

### 13. Confusing nested if structure
- **Status:** [x] Fixed indentation and braces

---

## Deprecation/Migration

### 14. git filter-branch deprecated
- **Status:** [x] Added git-filter-repo detection with fallback to filter-branch

---

## Edge Cases

### 15. Cargo.lock-only guardrail may lose previously staged content
- **Status:** [x] Check for pre-existing staged content before restoring

### 16. Untracked excluded files can't be restored
- **Status:** [x] Added `can_restore_entry()` check - only Modified/Renamed/TypeChange can be restored. Untracked files now show helpful message suggesting .gitignore

---

## Remaining (Low Priority - 3)

- [ ] #5 - Policy reload race (clone policy at start of each repo iteration)
- [ ] #6 - Repo discovery race (check repo existence before processing)
- [ ] #11 - Status inconsistency in CLI fallback (recalculate all status fields)
