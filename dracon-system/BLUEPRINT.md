# Dracon-System Blueprint

## Issues Fixed

### 1. Missing daemon lock for Guard daemon
- **Problem:** No exclusive lock to prevent multiple daemon instances running simultaneously
- **Fix:** Added `acquire_daemon_lock()` using fs2::FileExt at line 788-801
- **Priority:** High
- **Status:** [x]

### 2. Unbounded memory growth in GuardRuntimeState
- **Problem:** `notify_cooldowns` HashMap grows indefinitely without cleanup
- **Fix:** Added cleanup after each guard pass - entries older than 2x cooldown period are removed (lines 569-571)
- **Priority:** Medium
- **Status:** [x]

### 3. Silent failures with `let _ =`
- **Problem:** Notification/renice command failures silently discarded
- **Location:** Lines 450-454 (notification), 476-479 (renice)
- **Priority:** Low
- **Status:** [ ] (intentional - these are best-effort operations)

### 4. Config parsing silently ignores errors
- **Problem:** Invalid TOML silently returns defaults
- **Fix:** Added warning on parse failure at line 775-777
- **Priority:** Medium
- **Status:** [x]

---

## Code Quality Notes

### Guard Policy Normalization
- `normalize_guard_policy()` at line 837 ensures all config values are within safe bounds
- Prevents misconfiguration from causing issues

### Link Management
- `evaluate_link()` properly handles symlinks, missing targets, and non-symlink paths
- `apply_link_policy()` supports force-replace with automatic backup

### Storage Analysis
- Delegates to `dracon-system-lib::analyze_workspace_storage`
- Cleanup respects `--allow-tracked` flag to avoid deleting git-tracked directories

---

## Remaining Low Priority

- No graceful shutdown handling (Guard daemon uses infinite loop)
- No signal handling for cleanup on termination
- Storage cleanup could benefit from progress indication for large operations
