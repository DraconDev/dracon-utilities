# Dracon Utilities — Audit Task List

**Audit date:** 2026-05-29
**Workspace:** 0.112.3 | **Tests:** 656 pass, 0 fail, 6 ignored | **Unsafe:** 0

---

## CRITICAL — Must Fix

### C1: Hardcoded `/home/dracon/` paths in dracon-ai

**File:** `dracon-ai/src/main.rs`
**Lines:** 1454, 1458, 1484, 1739, 1743

These paths hardcode `/home/dracon/dracon` and `/home/dracon/dracon/nixos` as fallback values. Will **break on any other user's system**.

**Current code:**
```rust
.unwrap_or_else(|| Path::new("/home/dracon/dracon"))       // line 1454
.unwrap_or_else(|| Path::new("/home/dracon/dracon/nixos")) // line 1458
.unwrap_or_else(|| PathBuf::from("/home/dracon/dracon/nixos")); // line 1484
.unwrap_or_else(|| Path::new("/home/dracon/dracon"))       // line 1739
.unwrap_or_else(|| Path::new("/home/dracon/dracon/nixos")) // line 1743
```

**Fix:** Replace with `dirs::home_dir()` (already a workspace dependency):
```rust
.unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("dracon"))
.unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("dracon/nixos"))
```

Or if these are NixOS-specific paths, make them configurable via policy.

**Effort:** Small | **Risk:** High if not fixed

---

### C2: Mutex poison crash in dracon-system guard

**File:** `dracon-system/src/main.rs:1301`

```rust
if let Some(cached) = cache.lock().unwrap().get(name) {
```

If any thread panics while holding this lock, the mutex poisons and the guard daemon crashes on every subsequent cache access. The daemon has `Restart=always` so it recovers, but it enters a crash loop until the poisoned state is cleared.

**Fix:** Use poison-recovery:
```rust
if let Some(cached) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(name) {
```

**Effort:** Trivial | **Risk:** Medium (daemon crash loop)

---

## HIGH — Should Fix

### H1: Dead code cleanup (17 suppressions)

Remove or justify all `#[allow(dead_code)]` in production code.

| File:Line | Item | Action |
|-----------|------|--------|
| `dracon-sync/src/log.rs:5` | `enum Level` | Remove if unused, or remove `#[allow]` if used |
| `dracon-sync/src/log.rs:34` | `fn log_repo()` | Same |
| `dracon-sync/src/log.rs:43` | `fn log_module()` | Same |
| `dracon-sync/src/policy.rs:73` | `struct RemoteConfig` | Used by dracon-libs? Verify before removing |
| `dracon-sync/src/policy.rs:102` | `impl RemoteConfig` | Same as above |
| `dracon-sync/src/report.rs:421` | `fn repo_is_warn()` | Remove if replaced by another function |
| `dracon-sync/src/main.rs:102` | `SyncNow.force` | Planned feature? If not, remove field |
| `dracon-sync/src/git/status.rs:51` | `IndexLock::bypass()` | Used in repair commands? Verify |
| `dracon-sync/src/git/config.rs:20` | `fn real_git_path()` | Remove if unused |
| `dracon-system/src/events.rs:18` | `EventSeverity::Debug` | Remove variant if unused |

**Approach:** For each item, grep for usage. If zero callers outside the declaring module, remove. If used by `dracon-libs`, keep with a comment explaining why.

**Effort:** Medium | **Risk:** Low

---

### H2: Compiler warning — `todo_commit_messages` field

**File:** `dracon-sync/src/policy.rs:340`

```rust
pub(crate) todo_commit_messages: bool,
```

Compiler warns this field is never read. Either wire it into the commit message logic or remove it.

**Effort:** Small | **Risk:** Low

---

### H3: Clippy warnings (3 fixable issues)

**File:** `dracon-sync/src/sync.rs`

1. **Line 994-995** — `manual_strip`: Use `content.strip_prefix("- [~]")` instead of `starts_with` + index slice
2. **Line 1006** — `collapsible_str_replace`: Use `replace(['[', ']'], "")` instead of chained `.replace('[', "").replace(']', "")`

**Effort:** Trivial | **Risk:** Low

---

### H4: Duplicated cache cleanup in dracon-system

**File:** `dracon-system/src/main.rs:1111-1209`

Four identical `match`/`remove_dir_all` blocks for cargo, npm, pip, go caches. Also causes 8-level deep nesting.

**Current pattern (repeated 4x):**
```rust
Ok(ref safe_path) => {
    if let Err(e) = tokio::fs::remove_dir_all(safe_path).await {
        eprintln!("failed to remove {name} cache: {}", e);
        succeeded = false;
    }
}
Err(e) => {
    eprintln!("skipping {name} cache: {}", e);
    succeeded = false;
}
```

**Fix:** Extract helper:
```rust
async fn clean_cache_dir(path: &Path, name: &str, succeeded: &mut bool) {
    match validate_safe_path(path) {
        Ok(ref safe_path) => {
            if let Err(e) = tokio::fs::remove_dir_all(safe_path).await {
                eprintln!("failed to remove {name} cache: {e}");
                *succeeded = false;
            }
        }
        Err(e) => {
            eprintln!("skipping {name} cache: {e}");
            *succeeded = false;
        }
    }
}
```

**Effort:** Small | **Risk:** Low

---

### H5: Duplicated shutdown signal setup (3x)

**Files:**
- `dracon-system/src/main.rs:2967-2971`
- `dracon-sync/src/main.rs:558-561`
- `dracon-warden/src/main.rs:1336-1339`

Identical pattern:
```rust
let shutdown_sigterm = shutdown.clone();
let shutdown_sigint = shutdown.clone();
let reload_sighup = reload.clone();
```

**Fix:** Extract to a shared helper in `dracon-libs` or at minimum a local function.

**Effort:** Small | **Risk:** Low

---

## MEDIUM — Nice to Have

### M1: Large files needing modular split

| File | Lines | Suggested split |
|------|-------|-----------------|
| `dracon-system/src/main.rs` | 3,469 | Extract guard, storage, events, CLI into separate modules |
| `dracon-sync/src/sync.rs` | 3,075 | Extract commit, push, merge into sub-modules |
| `dracon-sync/src/report.rs` | 2,768 | Extract repair logic into `repair.rs` |
| `dracon-warden/src/main.rs` | 2,117 | Extract daemon, filter, harden into separate files |

**Effort:** Large | **Risk:** Low (refactoring only)

---

### M2: Long functions (>100 lines)

| File:Line | Function | Lines |
|-----------|----------|-------|
| `dracon-warden/src/main.rs:1302` | `run_daemon()` | 191 |
| `dracon-system/src/events.rs:66` | `emit_event()` | 180 |
| `dracon-sync/src/exclude.rs:703` | `is_build_output_dir_name()` | 127 |

Break into smaller functions with clear responsibilities.

**Effort:** Medium | **Risk:** Low

---

### M3: `once_cell` → `std::sync::OnceLock`

**File:** `dracon-warden/src/security/src/lib.rs`

`once_cell` (1.19) can be replaced with `std::sync::OnceLock` / `std::sync::LazyLock` since Rust 1.80+.

**Effort:** Small | **Risk:** Low (cosmetic, only if MSRV allows)

---

### M4: curl → reqwest migration

**Files:** `dracon-sync/src/visibility.rs`, `dracon-sync/src/release.rs`

The project already depends on `reqwest` but uses shell `curl` for API calls. Migrating to `reqwest` would improve error handling and eliminate shell argument escaping edge cases.

**Effort:** Medium | **Risk:** Low

---

## LOW — Backlog

### L1: Add `DRACON_SYNC_GIT_BIN` to clap help text

**File:** `dracon-sync/src/main.rs`

The env var is documented in AGENTS.md but not in `--help` output.

**Effort:** Trivial

---

### L2: Add `sha256sum` to install.sh output

**File:** `install.sh`

Print hash of installed binaries after install for verification.

**Effort:** Trivial

---

### L3: Add TOML field ordering warning to example config

**File:** `dracon-sync/dracon-sync.example.toml`

Add comment: `# NOTE: standard_files must appear before any section headers`

**Effort:** Trivial

---

### L4: Add size guard to incident ledger startup

**File:** `dracon-sync/src/daemon.rs` (or wherever `enforce_retention_at_startup` lives)

Check file size > 100MB before `read_to_string` to prevent OOM on corrupted ledger.

**Effort:** Small

---

### L5: Add test for new branch auto-push (F1 coverage)

No test exists for the new branch auto-push feature added in the previous audit cycle.

**Effort:** Medium

---

### L6: Add test for `filter_only_cleared` cooldown path

No test exists for the `NothingToDo` return when clean/smudge filter produces no-diff entries.

**Effort:** Medium

---

## INFO — No Action Required

| Item | Status |
|------|--------|
| Zero unsafe code | Excellent |
| Zero production panics | Excellent |
| All dependencies actively maintained | Good |
| cargo-deny enforcing advisories | Good |
| IndexLock TOCTOU mitigation | Solid |
| Atomic file writes everywhere | Good |
| Secret directory permission checks | Good |
| All 656 tests passing | Good |

---

## Priority Execution Order

1. **C1** — Fix hardcoded paths in dracon-ai (portability blocker)
2. **C2** — Fix mutex unwrap in dracon-system (crash loop risk)
3. **H3** — Fix clippy warnings (trivial, clean build)
4. **H2** — Remove dead `todo_commit_messages` field
5. **H4** — Extract cache cleanup helper (dedup + reduce nesting)
6. **H1** — Dead code audit and removal pass
7. **H5** — Extract shutdown signal helper
8. **M3** — once_cell → OnceLock (if MSRV allows)
9. **M1** — Begin splitting large files (start with `dracon-system/src/main.rs`)
10. **L1-L4** — Quick wins in a single pass

---

## Tracking

| Status | Count |
|--------|-------|
| CRITICAL | 2 |
| HIGH | 5 |
| MEDIUM | 4 |
| LOW | 6 |
| INFO | 9 |
| **Total** | **26** |
