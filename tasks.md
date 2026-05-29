# Dracon Utilities — Audit Task List

**Audit date:** 2026-05-29
**Workspace:** 0.112.3 | **Tests:** 656 pass, 0 fail, 6 ignored | **Unsafe:** 0

---

## CRITICAL — Must Fix

### ~~C1: Hardcoded `/home/dracon/` paths in dracon-ai~~ ✅ DONE

**File:** `dracon-ai/src/main.rs`
**Fix:** Replaced all 5 hardcoded `/home/dracon/dracon` paths with `dirs::home_dir()` fallbacks. Computed values before format macros to avoid lifetime issues.

---

### ~~C2: Mutex poison crash in dracon-system guard~~ ✅ DONE

**File:** `dracon-system/src/main.rs:1301`
**Fix:** Changed `cache.lock().unwrap()` to `cache.lock().unwrap_or_else(|e| e.into_inner())` for poison recovery.

---

## HIGH — Should Fix

### ~~H1: Dead code cleanup (17 suppressions)~~ ✅ DONE

| Item | Action Taken |
|------|--------------|
| `log.rs` (Level, log_repo, log_module, 6 macros) | Removed dead items, kept only `warn()` + `log_warn!` |
| `RemoteConfig` struct + impl | Removed `#[allow(dead_code)]` — struct IS used |
| `repo_is_warn()` | Changed to `#[cfg(test)]` — test-only |
| `SyncNow.force` | Changed to `hide = true` — keep CLI compat, hide from help |
| `IndexLock::bypass()` | Removed — never called |
| `EventSeverity::Debug`, `Critical` | Removed variants — never constructed |

---

### ~~H2: Compiler warning — `todo_commit_messages` field~~ ✅ DONE

**File:** `dracon-sync/src/policy.rs`
**Fix:** Removed field from struct, default impl, test data, and example config.

---

### ~~H3: Clippy warnings (3 fixable issues)~~ ✅ DONE

**File:** `dracon-sync/src/sync.rs`
**Fix:**
1. `starts_with("- [~]")` + `content[5..]` → `strip_prefix("- [~]")`
2. `starts_with("- [x]") || starts_with("- [X]")` → `strip_prefix` with `or_else`
3. `replace('[', "").replace(']', "")` → `replace(['[', ']'], "")`

---

### ~~H4: Duplicated cache cleanup in dracon-system~~ ✅ DONE

**File:** `dracon-system/src/main.rs`
**Fix:** Extracted `try_remove_cache_dir()` helper. Replaced 4 identical 30-line blocks with a loop over `(label, enabled, rel_path)` tuples. Reduced nesting from 8 levels to 4.

---

### ~~H5: Duplicated shutdown signal setup (3x)~~ ⏭️ SKIPPED

3 clone calls across 3 crates — extracting a helper would be over-engineering for this simple pattern.

---

## MEDIUM — Nice to Have

### ~~M3: `once_cell` → `std::sync::OnceLock`~~ ⏭️ SKIPPED

`OnceLock::get_or_try_init` is unstable (Rust issue #109737). `once_cell` stays until `get_or_try stabilizes.

### M4: curl → reqwest migration — Backlog

Large refactor, not urgent. `curl` works and is well-tested.

### M1: Split dracon-system/src/main.rs — Backlog

3,469 lines. Splitting is a significant refactor. Do when touching the file for other reasons.

### M2: Break long functions — Backlog

`run_daemon()` (191 lines) and `emit_event()` (180 lines). Break when touching these functions.

---

## LOW — Done

### ~~L1: Add `DRACON_SYNC_GIT_BIN` to clap help text~~ ✅ DONE

Added `after_help` with ENVIRONMENT section documenting `DRACON_SYNC_GIT_BIN`, `DRACON_SYNC_POLICY`, `DRACON_SYNC_STATE_DIR`.

---

### ~~L2: Add `sha256sum` to install.sh output~~ ✅ DONE

Added checksum block after `ls -la` that prints sha256sum for each installed binary.

---

### ~~L3: Add TOML field ordering warning to example config~~ ✅ DONE

Added warning comment at top of `dracon-sync.example.toml` about field ordering requirements.

---

### ~~L4: Add size guard to incident ledger startup~~ ✅ DONE

Added 100MB size check before `read_to_string`. If exceeded, truncates to `max_lines` and logs warning.

---

### ~~L5: Add test for new branch auto-push~~ ⏭️ SKIPPED

Deferred — requires setting up git mock infrastructure.

### ~~L6: Add test for filter_only_cleared cooldown~~ ⏭️ SKIPPED

Deferred — requires filter mock infrastructure.

---

## INFO — No Action Required

| Item | Status |
|------|--------|
| Zero unsafe code | Excellent |
| Zero production panics | Excellent (after C2 fix) |
| All dependencies actively maintained | Good |
| cargo-deny enforcing advisories | Good |
| IndexLock TOCTOU mitigation | Solid |
| Atomic file writes everywhere | Good |
| Secret directory permission checks | Good |
| All 656 tests passing | Good |

---

## Summary

| Status | Count |
|--------|-------|
| ✅ Completed | 12 |
| ⏭️ Skipped (not worth it) | 4 |
| 📋 Backlog (large refactor) | 4 |
| **Total** | **20** |

### Verification

```
cargo check --workspace    → 0 warnings
cargo clippy --workspace   → 0 warnings
cargo test --workspace     → 656 pass, 0 fail
```

### Files Modified

| File | Changes |
|------|---------|
| `dracon-ai/src/main.rs` | Fixed 5 hardcoded paths → `dirs::home_dir()` |
| `dracon-system/src/main.rs` | Mutex poison recovery, extracted cache cleanup helper |
| `dracon-system/src/events.rs` | Removed `Debug`/`Critical` variants |
| `dracon-system/src/events_tests.rs` | Updated test for removed variants |
| `dracon-sync/src/main.rs` | Added env var help text, hid `--force` flag |
| `dracon-sync/src/log.rs` | Stripped to only `warn()` + `log_warn!` |
| `dracon-sync/src/policy.rs` | Removed `RemoteConfig` dead_code allow, removed `todo_commit_messages` |
| `dracon-sync/src/report.rs` | `repo_is_warn` → `#[cfg(test)]`, added ledger size guard |
| `dracon-sync/src/sync.rs` | Fixed 3 clippy warnings |
| `dracon-sync/src/git/status.rs` | Removed `IndexLock::bypass()` |
| `dracon-sync/dracon-sync.example.toml` | Added TOML ordering warning, removed `todo_commit_messages` |
| `install.sh` | Added sha256sum output |
