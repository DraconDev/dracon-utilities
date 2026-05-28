# Dracon Utilities Audit — 2026-05-28

## Status: IN PROGRESS

---

## Recently Completed

### ✅ F1 + G2: New Branch Auto-Push + filter_only_cleared Fix (sync.rs)

**Three changes in `dracon-sync/src/sync.rs`:**

1. **`push_with_blob_check`** — pushes even when `ahead == 0` if the branch has no upstream tracking (`sync.rs:994`). Condition: `(ahead == 0 && branch_has_upstream)` is the only skip case; new branches with no upstream proceed to push.

2. **`handle_ahead_push`** — same logic applied after auto_pull_merge syncrs:1481), so new branches get pushed after every sync cycle, not just when commits exist.

3. **`filter_only_cleared` handling** — when `compute_diff_entries` returns `filter_only_cleared=true`, `sync_repo` now immediately returns `NothingToDo` (sync.rs:1423) instead of falling through to staging logic. The daemon's re-check of dirty fingers after sync success applies a cooldown for filter-only dirty repos.

**Files changed:** `dracon-sync/src/sync.rs`
**Build:** ✅ `cargo build --release` passes
**Installed:** ✅ daemon restarted with new binary (sha256: d8e3f0a6...)

---

## Audit Tasklist

### Category: Core Functionality

---

#### ✅ F1 (COMPLETED — see above)

---

#### F2: `auto_pull_merge` and new branch tracking

**Finding: NOT A BUG — works as designed.**

Location: `sync.rs:279`
```rust
if policy.auto_pull && ctx.has_origin && ctx.has_upstream && initial_status.behind > 0 ...
```
A brand-new branch has `has_upstream = false` (no `@{upstream}` tracking ref), so `auto_pull_merge` correctly skips — no remote ref exists to pull from. This is the correct behavior.

**When `has_upstream` becomes `true`:** After `handle_ahead_push` pushes `HEAD:refs/heads/<branch>` to origin, git automatically creates `refs/remotes/origin/<branch>` tracking ref. On the next sync cycle, `has_tracking_upstream()` returns `true`, `behind > 0` can trigger, and normal pull/push cycles begin.

**No action needed.**

---

#### F3: Mirror push failure handling

**Finding: Correctly independent per-mirror. One acceptable limitation.**

Location: `sync.rs:985` (`push_with_blob_check`)

`push_with_blob_check` performs two pushes in sequence:
1. **origin** via `push_with_retries` (retry loop + SSH→HTTPS transport fallback)
2. **mirrors** via `push_mirror_remotes` → `push_to_all_remotes` (iterates each mirror, tries each once)

Key behaviors:
- Each mirror is tried independently — if mirror N fails, mirror N+1 still runs
- `push_to_named_remote` has its own retry loop (retries × attempts) per mirror
- `force_when_behind` is per-mirror, configured in `RemoteConfig`
- When a mirror fails, `ctx.remote_failures` map is incremented (line 1085)
- `push_mirror_remotes` returns `Vec<(String, Result)>` — all results checked; any non-`Ok` means `push_with_blob_check` returns `false`
- Webhook notification fires per failed mirror

**Acceptable limitation:** `handle_ahead_push` (line 1473) calls `push_with_blob_check` but treats any `false` return as informational only (`[]` println). If mirror pushes fail in `handle_ahead_push`, `sync_repo` still returns `SyncOutcome::Synced`. The mirror failures ARE tracked in `ctx.remote_failures` → `mirror_consecutive_fails` (daemon.rs:1236) and generate sustained-notification at MIRROR_DEGRADED_THRESHOLD=3 (daemon.rs:1263), but do NOT make `sync_success == false`.

If gitlab consistently fails while origin succeeds, the daemon never marks the repo stuck on push (stuck marking requires origin failure, tracked by `failure_count`). Mirror failures produce degraded notifications but not stuck-repo blocking. **This is an acceptable design trade-off** — origin is the source of truth for stuck marking, mirrors are secondary.

**Status: No action needed.**

---

#### F4: `IndexLock` coordination mechanism

**Finding: Correctly implemented. No issues.**

Location: `git/status.rs:20-64`

```rust
// acquire
std::fs::OpenOptions::new().write(true).create_new(true)  // O_EXCL
// drop
let _ = std::fs::remove_file(&self.path);
```

- `O_EXCL` atomic create — no TOCTOU race
- Lock is released on both success and error paths via `Drop`
- `bypass()` constructor exists for `once`/`repair` commands that don't need coordination
- Used at `sync.rs:1383` (standard files) and in warden `harden_repo`

**Status: No issues found.**

---

### Category: Safety Guards

---

#### G1: Mass deletion guard

**Finding: The mass deletion guard has been REMOVED. AGENTS.md is stale.**

The metrics endpoint at `main.rs:847` confirms:
```rust
// Mass deletion guard removed — IndexLock fixes the clone race
println!("dracon_sync_mass_deletion_guard_blocked_total 0");
```

The guard thresholds exist only as a property test (`sync.rs:3045` — `test_safety_guard_property`), verifying the logic but not enforcing it. The comment at `daemon.rs:845` confirms removal.

**Actual safety:** `IndexLock` prevents clone races; `git rm --ignore-unmatch` handles missing files gracefully. Deletions commit normally without special guard logic in production.

**Action needed:** AGENTS.md Section "Safety Behaviors / Mass deletion prevention" describes the three thresholds as active, but they are not. This documentation should be updated to reflect the guard's removal.

---

#### ✅ G2 (COMPLETED in F1)

---

#### G3: Stuck repo mechanism

**Finding: Well-designed. One minor timing gap.**

Location: `daemon.rs:744, 1107, 1210, 1226`

Stuck marking condition (line 1210):
```rust
if is_diverged || (!effective_dirty && status.ahead > 0 && entry.failure_count >= 3)
```

A repo becomes stuck when-diverged from origin (remote has commits local doesn't, AND 3 push failures) OR clean + unpushed + 3 failures. `failure_count` tracks origin-wide failures (incremented when `sync_success == false`).

Stuck repos are:
- Retried every 5 minutes (line 744: `stuck_age_secs < 300 → continue`)
- On retry: removed from stuck list, attempted once, re-marked if fails again
- Unstuck automatically: when `sync_success == true` and repo was stuck, removed from list (line 1107)

**Minor timing gap:** After the 5-minute auto-retry removes the stuck marker, the repo is NOT immediately re-added to the activity loop. It waits for the next scan cycle (≤ `scan_interval_secs`, default ~30s). So up to ~30s delay after a successful push before the repo is re-checked. **Low severity.**

**Status: Acceptable.**

---

#### G4: `repair-concerns` and `repair-warns`

**Finding: Repair runs inside sync timeout, with cooldown on unresolvable issues.**

Location: `daemon.rs:1056-1090`

- `repair_concerns` runs when `auto_repair_concerns == true` AND `sync_success == true`
- If `found > 0 && resolved_now == 0 && succeeded == 0` → `should_cooldown = true` (line 1072)
- Same pattern for `repair_warns`
- Cooldown prevents tight retry loops on unresolvable issues

**Concern:** Repair operations (branch rename, force-push, etc.) run inside the sync timeout window. A slow rename on a large repo could cause repeated timeouts without specific timeout-per-repair logic.

**Status: Mark for follow-up** — verify no repair operation can exceed the sync timeout and block the loop indefinitely.

---

### Category: Per-Remote Logic

---

#### R1: `repo_name_map` for dot-prefixed repos

**Finding: Not verified — requires live testing.**

Location: `policy.rs:93, 805-820`

The `repo_name_map` is validated for empty/invalid names but the actual mapping in `auto_create_all_remotes` (mirrors repo local name → remote name) hasn't been end-to-end tested. Would require creating a dot-prefixed repository and pushing to GitLab to verify the mapping works.

**Status: Mark for manual test** (R1a in follow-up).

---

#### R2: `auto_github_private` and repo reuse on name collision

**Finding: AGENTS.md requirement not verified in code.**

Location: `git/multi_remote.rs:329-370`

AGENTS.md: *"NEVER create suffixed repos. If the GitHub repo already exists, reuse it."*

The code calls `gh repo create --private <repo_name>`. If the name is taken, `gh` returns an error and the code falls back to the origin URL (if origin exists). The code does NOT pre-check existence with `gh repo view` before creating, and does NOT parse "name already exists" errors to reuse instead of fail/suffix.

**⚠️ Potential issue:** The AGENTS.md says to reuse, but the implementation may not do this. If `gh repo create` fails with "Name already exists", the fallback path returns `origin_url(repo)` which is the existing repo — but this falls through to `ensure_origin_remote` which might not return the correct github.com URL if origin is something else.

**Action needed:** Manual test: call with a repo name that already exists and check behavior.

---

#### R3: Visibility and metadata sync

**Finding: Correctly implemented. Cache is per-repo and TTL-gated.**

Location: `visibility.rs` (cache creation + fetch + TTL)

The visibility sync:
1. `get_visibility_and_metadata` queries GitHub via `gh api`
2. Per-repo `.last` timestamp file in `~/.local/state/dracon/visibility-sync/`
3. TTL enforced: 24 hours default (`sync_visibility_interval_hours`)
4. `prune_stale_visibility_cache` runs at startup (removes orphans for deleted repos)
5. `update_gitlab_visibility` / `update_codeberg_visibility` called per mirror if configured

**Note:** In-memory timestamp is lost on daemon restart, but persists in `.last` files. Next fetch will be TTL-checked against the file timestamp. Fine for a cache.

**Status: No issues.**

---

#### R4: Codeberg push-to-create disabled

**Finding: Correctly documented and implemented.**

Location: `git/multi_remote.rs:58, 148`

`push_mirror_remotes` iterates remotes from policy config. Codeberg is in the remote list configured in policy. The push-to-create limitation is a GitLab/Forgejo restriction — git push fails with a transport error for non-existent repos. No special error handling needed; the failure is clear.

**Status: No issues.**

---

### Category: Process Monitoring (dracon-system)

---

#### S1: Auto-renice graduated thresholds

**Finding: Correctly implemented. Well-tested.**

Location: `main.rs:578`, `guard_tests.rs:74-365`

```rust
let cpu_tiers: &[(f32, i32)] = &[(500.0, 15), (300.0, 10), (180.0, 5)];
let mem_tiers: &[(u64, i32)] = &[(8192, 10), (4096, 5)];
cpu_nice.max(mem_nice).clamp(0, 19)
```

Thorough test coverage in `guard_tests.rs`. Release after non-heavy for `release_after_secs` (default 120s) correctly restores nice to 0 via `renice_process(pid, 0)` at `main.rs:2035`.

**Status: No issues.**

---

#### S2: Proactive cleanup and active build protection

**Finding: Correctly protects active builds via PID exclusion + age heuristics.**

Location: `main.rs:604` (`detect_active_rust_builds`)

Active cargo/rustc PIDs are detected by scanning `ps` output and excluded from renicing and from `target/` cleanup. `target/` dirs older than 14 days (`rust_target_max_age_days`) are eligible for proactive cleanup. Active builds (running cargo/rustc) are protected regardless of dir age because the running process exits or is excluded.

The 14-day age is conservative — `ctime`/`mtime` gets updated by cargo when it writes to `target/`, so recently-built dirs won't be touched.

**Status: No issues.**

---

#### S3: Guard log rotation

**Finding: Rotation implementation not found in visible code.**

The `log_guard_event` function receives events but the rotation logic (checking `guard_log_max_mb` and rotating the log file) is not found in `main.rs` grep analysis. It may be in a separate file or the rotation may be delegated to logrotate/systemd. The `guard_events` channel is created but the consumer that handles rotation was not identified.

**Action needed:** Verify the log rotation implementation. If it doesn't exist, heavy guard event production could fill disk.

---

#### S4: Protected path handling

**Finding: System paths use exact/ancestor matching with canonicalization.**

Protected paths: `/`, `/home`, `/etc`, `/usr`, `/var`, `/boot`, `/nix`, `/run`, `/sys`, `/dev`, `/proc`

- `/` is exact match only
- All other system paths use ancestor matching (so `/home` protects `/home/dracon/Dev`)
- User-protected paths from config: policy says "canonicalized before comparison" — the exact canonicalization call wasn't verified in the grep scope of `main.rs`

**Action needed:** Verify canonicalization in the guard path-check function (e.g., `check_safe_to_delete_guard`).

---

#### S5: Process monitoring sustain time

**Finding: Potential minor gap — gap detection works, but gap must be shorter than sampling interval.**

Location: `main.rs:1916-1934`

The sustain time check: `heavy_since` tracks when a process first became heavy. It is NOT reset during brief dips below threshold — only when the process is removed from `current_heavy` set. If a process is heavy for 30s, drops for 5s, then heavy again for 30s: the 5s gap is invisible and the process appears sustained for 65s.

Gap tolerance = sampling interval (30s default). If gap < 30s, it won't reset `heavy_since`. This is a design choice, not a bug. Probability of a real sustained-hgih process being missed is very low.

**Status: Acceptable.**

---

### Category: Warden / Encryption

---

#### W1: DRACON_SECRET marker detection

**Finding: Correctly identifies all variants. Tests comprehensive.**

Location: `main.rs:1619-1625`, `tests.rs:619-668`

User-facing marker format: `[DRACON_SECRET:keyname]` (brackets required, colon required). Tests verify:
- `is_marker_string("[DRACON_SECRET:abc123]")` — basic
- `is_marker_string("[DRACON_SECRET:abc-123_456]")` — dashes/underscores
- `!is_marker_string("DRACON_SECRET not in brackets")` — unbracketed rejected
- `!is_marker_string("[DRACON_SECRET]")` — no colon rejected

**AGENTS.md note:** Mentions `DRACON_SECRET_x001_` variants — these are internal implementation, not user-facing. No action needed.

**Status: No issues.**

---

#### W2: Clean/smudge filter idempotency

**Finding: Filter is idempotent for both already-plaintext and already-encrypted content.**

`smart_smudge` checks for `[DRACON_SECRET:` before substituting — if absent, returns plaintext as-is. `smart_clean` first smudges, then re-encrypts, making it safe for re-runs. No false substitution of non-secret strings.

**Status: No issues.**

---

#### W3: `resmudge` command

**Finding: Correctly identifies ciphertext and fixes stuck working tree files.**

Location: `main.rs:1894` (`resmudge_repo`)

Calls `smart_smudge` on all files — correctly replaces `[DRACON_SECRET:key]` markers with real secret content. Files already plaintext are no-ops.

**Status: No issues.**

---

#### W4: IndexLock in harden_repo

**Finding: Correctly coordinates multiple working-tree writes.**

Location: `harden_repo` (warden), `git/status.rs:20-64`

`IndexLock` acquired before first write (`apply_overwrite_file`), held through subsequent writes (`publish_repo_pubkey`). `Drop::drop` unconditionally releases lock (`let _ = remove_file`) — covers success and error paths.

If second write fails, first write's side effects are not rolled back. Both operations are idempotent overwrite types — next cycle just reapplies. Acceptable.

**Status: No critical issues.**

---

### Category: Testing

---

#### T1: Serial test reliability

**Finding: Issue understood; documented workaround exists.**

AGENTS.md recommends `--test-threads=1` for reliability. The issues (PATH mutations from mock git binaries, fixed-port TCP listeners, `acquire_path_lock` for explicit serialization) are understood. Running with default parallelism produces ~10-20 flaky failures.

**Status: No action needed — documented.**

---

#### T2: Test for new branch auto-push

**Finding: Not yet implemented.**

The new branch push logic in `handle_ahead_push` and `push_with_blob_check` has no dedicated test. Would need: create temp repo, checkout new branch with no upstream, call push function, verify push attempt (requires a real remote or mock).

**Action needed: Add test.**

---

#### T3: Test for filter_only_cleared cooldown

**Finding: Not yet implemented.**

The `filter_only_cleared` path (now returning `NothingToDo`) has no dedicated test. Would need: set up clean/smudge filter producing modified-but-no-diff entries, call `sync_repo`, verify `NothingToDo` returned.

**Action needed: Add test.**

---

### Category: Operational State

---

#### O1: Incident ledger retention at startup

**Finding: Runs first, but could OOM on corrupted giant ledger.**

Location: `daemon.rs:386`, `report.rs:406-424`

`enforce_retention_at_startup` is called before daemon loop (line 386). It reads entire ledger into memory via `std::fs::read_to_string`, then filters. Default max 10,000 lines. A corrupted ledger (garbage text filling it to 100MB+) would be read entirely into memory.

**Recommendation:** Add a size guard — if ledger file > 100MB, skip reading and truncate to last 1000 lines, or just rename-and-restart.

**Status: Low severity, edge case.**

---

#### O2: Visibility cache concurrency

**Finding: Per-file cache makes concurrent access safe across repos.**

Location: `visibility.rs`, `daemon.rs:391`

Each repo has its own `.last` file. Pruning only removes files for deleted repos. Concurrent read of repo A and prune of repo A at same instant would need locking — not observed in code, but probability is low (prune runs once per startup). Concurrent read of repo A's cache during daemon loop and write of repo A's cache at same time — also not observed. This is a latent race.

**Status: Low severity.** Recommend reviewing `prune_stale_visibility_cache` for any global lock or rename-then-write atomic pattern.

---

#### O3: Stuck marking and per-remote tracking

**Finding: Stuck marking is repo-level (origin success/failure), not per-mirror.**

Location: `daemon.rs:744, 1210, 1226`

When a repo becomes stuck (is_diverged OR clean+ahead+3fails), the entire repo is marked in `stuck_push_repos.json`. The condition uses `failure_count` (origin failures only). Mirror failures are tracked separately in `mirror_consecutive_fails` (daemon.rs:1236), which only affects the sustained-notification system (MIRROR_DEGRADED_THRESHOLD), not stuck marking.

This is intentional — origin push success is the arbiter of repo health. Mirror degradation generates notifications but doesn't block the sync cycle.

**Status: Works as designed. No issues.**

---

#### O4: IndexLock stale lock cleanup

**Finding: Runs at startup and periodically. Both verified.**

Location: `daemon.rs:554` (`run_startup_cleanup`), `daemon.rs:645-649` (every 5 min via `cycle_count.is_multiple_of(300)`)

Startup cleanup calls `repair_broken_tracking`. In `repair_broken_tracking`, stale `index.lock` detection runs. The periodic check at cycle 300 (every ~5 min at 1s interval) also calls `repair_broken_tracking`.

The stale `index.lock` cleanup removes lock files whose holding processes no longer exist. This prevents a stale crash lock from blocking all git operations in that repo.

**Status: Correctly implemented.**

---

### Category: Configuration / Policy

---

#### P1: TOML field ordering risk

**Finding: Not validated at load time. Silent mis-parse risk.**

Location: `policy.rs:load`

AGENTS.md warns that `standard_files` must appear BEFORE section headers. The TOML parser will parse out-of-order fields silently into the wrong section if a section header precedes them. The `validate_config` command (policy.rs:705) validates values but not field ordering.

**Risk:** If a user puts `standard_files = [...]` after a `[[remotes]]` section header, the policy loader silently ignores it — standard files are not copied, and the user has no error. This could cause LICENSE to not be auto-copied to new repos.

**Action needed:** Either add field ordering validation to `validate_config`, or add a prominent comment in `dracon-sync.example.toml` warning about field ordering.

---

#### P2: `validate-config` coverage

**Finding: Validates values but not structure/ordering.**

Location: `policy.rs:705`

`validate_config` checks required fields, URL validity, remote configs, but not TOML field ordering (see P1). It also doesn't catch impossible combinations (e.g., `auto_publish = true` but `publish_targets = []` is an empty list — silently does nothing, which might be intended).

**Status: Acceptable for current usage.**

---

#### P3: Default values and AGENTS.md consistency

**Finding: Defaults mostly match. One discrepancy.**

| Setting | Code default | AGENTS.md |
|---------|-------------|-----------|
| `proactive_cleanup_percent` | 50% (policy.rs) | 50% ✅ |
| `rust_target_max_age_days` | 14 (policy.rs) | 14 ✅ |
| `proactive_cleanup_interval_cycles` | 120 (policy.rs) | 120 ✅ |
| `repair_cooldown_secs` | 60 (policy.rs) | 60 ✅ |
| `guard_log_max_mb` | 1 (dracon-system) | 1 ✅ |
| `release_after_secs` | 120 (dracon-system) | 120 ✅ |

All major defaults match. No action needed.

---

### Category: Secret Management

---

#### K1: Token resolution and test isolation

**Finding: EnvRestorer correctly restores. `load_secret` checks env first.**

Location: `secrets.rs:19-24`, `git/mod.rs:265-326` (tests)

`load_secret` priority: (1) env var if set and non-empty, (2) secrets dir .env files, (3) None.

Test isolation uses `EnvRestorer` which restores on drop. Tests verify precedence correctly (`test_load_secret_env_takes_precedence_over_file`).

**Status: No issues.**

---

#### K2: GH_TOKEN resolution order

**Finding: Env var takes precedence over `gh auth`.**

Location: `secrets.rs:19` → env var checked first.

The `gh auth` credential is only used if `load_secret("GH_TOKEN")` returns `None`. This means a user with `gh auth` can also set `GH_TOKEN` env var to override it for testing.

**Status: No issues.**

---

### Category: Release Pipeline

---

#### L1: `auto_tag` — annotated tags

**Finding: Correctly creates annotated tags (not lightweight).**

Location: `release.rs:61-68`

```rust
run_git_with_timeout(repo, &["tag", "-a", &tag, "-m", &format!("Release {tag}")], ...)
```

`-a` = annotated tag (with message via `-m`). This is correct for releases.

**Status: No issues.**

---

#### L2: `auto_release` — dry-run before publish

**Finding: Both dry-run and real publish are called unconditionally.**

Location: `release.rs:598-606`

When `repo_auto_release && bump_level == "major"`:
1. `create_github_release` is called (with `gh release create`) — this has no dry-run mode built-in
2. If it fails, a `Failed` step is recorded but the release step continues

There's no pre-publish dry-run for GitHub releases vs. other registries. For crates.io/npm, `publish_to_registry` calls `cargo publish --dry-run` first (`release.rs` would need verification — not read in detail).

**Status: GitHub Releases don't have a dry-run mechanism — this is acceptable given gh's own validation.**

---

#### L3: Nix flake auto-update PR

**Finding: Not verified. Requires integration test.**

The Nix flake PR creation logic (`nix_auto_update`) was not traced in detail. Would need a live GitHub token with repo permissions to verify the PR opens correctly.

**Status: Mark for integration test.**

---

#### L4: Registry pre-check skips already-published

**Finding: Correctly implemented. Fast (local metadata check).**

Location: `release.rs:619-626`

```rust
match version_exists_on_registry(target.registry, &pkg_name, new_version) {
    Ok(true) → Skipped("already on registry")
    Ok(false) → publish
    Err(e) → Failed
}
```

`version_exists_on_registry` makes a registry API call (crates.io API, npm registry API, PyPI JSON) — this is a network call, not purely local. It happens for each publish target on each bump cycle. If the network call times out or fails, the step fails rather than skipping.

**Potential improvement:** Cache registry metadata locally with TTL to avoid repeated network calls. Currently not cached.

**Status: Acceptable for low-frequency version bumps.**

---

## Quick Wins (Low Effort, High Value)

| ID | Action | Effort | Priority |
|----|--------|--------|----------|
| Q1 | Add `filter_only_cleared` handling | ✅ DONE in F1 | — |
| Q2 | Document `DRACON_SYNC_GIT_BIN` in sync `--help` | Add env var to clap docs | Low |
| Q3 | Add `sha256sum` to `install.sh` output | Edit install.sh | Low |
| Q4 | Add field ordering check to `validate_config` | Add to policy.rs validation | Low |
| Q5 | Add size guard to incident ledger prune | Add file size check in enforce_retention_at_startup | Low |

---

## Priority Ordered Follow-Up List

1. **G1**: Update AGENTS.md — mass deletion guard has been removed, replace with IndexLock explanation
2. **P1**: Add TOML field ordering validation to `validate_config` — silent mis-parse risk is high
3. **R2**: Manual test `auto_github_private` with existing repo name — AGENTS.md requirement not verified
4. **T2**: Add test for new branch auto-push
5. **T3**: Add test for `filter_only_cleared` cooldown path
6. **S3**: Verify guard log rotation implementation
7. **S4**: Verify canonicalization in protected path check
8. **O1**: Add size guard to incident ledger startup prune
9. **L3**: Integration test Nix flake PR creation
10. **G4**: Audit repair operations for timeout-safety

---

## Files In Scope For This Audit

```
dracon-sync/src/
├── sync.rs           — core sync logic, push mechanics, mass deletion (test only)
├── daemon.rs         — sync loop, stuck marking, cooldowns, startup cleanup
├── policy.rs         — policy loading and validation
├── git/
│   ├── push.rs       — push_with_retries
│   ├── multi_remote.rs — mirror push, repo creation, repo_name_map
│   ├── branch.rs      — branch tracking
│   └── status.rs     — IndexLock, has_tracking_upstream
├── report.rs         — incident ledger, repair-concerns/warns
├── release.rs        — tag, release, publish pipeline
├── secrets.rs        — token loading
├── visibility.rs     — cross-platform visibility/metadata sync
├── simple_ai.rs      — AI scribe
└── main.rs           — CLI, metrics (mass deletion guard stub)

dracon-system/src/main.rs  — guard loop, graduated renice, proactive cleanup
dracon-warden/src/main.rs  — warden, DRACON_SECRET markers, resmudge
```
