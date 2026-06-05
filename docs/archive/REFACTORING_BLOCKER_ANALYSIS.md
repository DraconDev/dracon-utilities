# Refactoring Blocker Analysis — 2026-06-01

## Executive Summary

Four refactoring tasks were identified during a comprehensive codebase review of dracon-utilities. Each was deferred due to specific blockers — ranging from incomplete prerequisite work to high risk of destabilizing working code. This document analyzes why each task is stuck and provides options for future work.

## The 4 Deferred Tasks

### 1. H-SEC-LIB: Complete Security Lib Split (dracon-warden)

**Status**: Partially started, then reverted  
**Estimated effort**: 4-6 hours  
**Priority**: High (code organization)

#### What was attempted
A full security lib split was attempted — extracting `lib.rs` (currently 1534+ lines) into separate modules:
- `modules/crypto.rs` — encryption/decryption logic
- `modules/filter.rs` — git clean/smudge filter
- `modules/keygen.rs` — key generation
- `modules/team.rs` — team management
- `modules/backup.rs` — backup operations

#### Why it got stuck
1. **Circular dependencies**: The extracted modules depended on types defined in `lib.rs`, which also depended on module functions. Breaking the cycle required restructuring the public API.
2. **Test breakage**: Moving code between modules broke 20+ tests that referenced `crate::` paths. Each module needed its own `#[cfg(test)]` block with adjusted imports.
3. **Filter complexity**: The clean/smudge filter (`modules/filter.rs`) has tight coupling with the git integration — it reads/writes stdin/stdout and must handle binary data. Extracting it cleanly requires abstracting the I/O layer.
4. **Revert happened**: The split was reverted to restore a working state, losing the partial progress.

#### Options for future work
- **Option A (Incremental)**: Extract one module at a time, running tests after each. Start with the least coupled module (keygen) and work toward the most coupled (filter). Estimated: 6-8 hours total.
- **Option B (Big bang)**: Do the full split in one commit with all test fixes. Faster if it works, but higher risk of prolonged breakage. Estimated: 4-6 hours.
- **Option C (Defer indefinitely)**: `lib.rs` at 1534 lines is large but functional. The warden is stable and rarely modified. Leave it as-is until a major feature addition forces the split.

#### Recommendation
Option A (incremental). The warden is stable code — incremental extraction with test verification after each step is the safest path.

---

### 2. H-DAEMON: Extract Cooldown Manager (dracon-sync)

**Status**: Started, then reverted  
**Estimated effort**: 2-4 hours  
**Priority**: High (daemon reliability)

#### What was attempted
Extracting the cooldown logic from `daemon.rs` into a separate `cooldown.rs` module. A `CooldownManager` struct was created that tracks:
- Per-repo cooldown state (filter-only cooldown after staging filter changes)
- Cooldown expiry timing
- Cooldown reset on state changes

#### Why it got stuck
1. **Tight coupling with daemon state**: The cooldown logic reads and writes daemon state (fingerprint, staging area status, last sync time). Extracting it required passing `&mut` references to daemon state, which created borrow checker issues.
2. **Async context**: The daemon runs in a tokio runtime. The cooldown manager needs to interact with async git operations, which means it can't be a simple synchronous struct.
3. **Test isolation**: Testing the cooldown manager in isolation requires mocking the daemon state, which is complex given the number of fields involved.
4. **Revert happened**: The extraction was reverted to restore a working state.

#### Options for future work
- **Option A (Trait-based)**: Define a `CooldownPolicy` trait that the daemon implements. The cooldown manager uses the trait to read/write state without direct struct references. Estimated: 3-4 hours.
- **Option B (Event-driven)**: Instead of the cooldown manager reading daemon state, have the daemon emit events (e.g., `FilterOnlyDetected`, `SyncCompleted`) and the cooldown manager react to them. Estimated: 4-5 hours.
- **Option C (Leave in daemon.rs)**: The cooldown logic is ~100 lines and closely tied to the sync loop. It's not worth extracting unless daemon.rs grows significantly larger.

#### Recommendation
Option C (leave in daemon.rs). The cooldown logic is small and tightly coupled. Extracting it adds complexity without meaningful benefit. Only extract if daemon.rs crosses 1500+ lines.

---

### 3. L-HEALTH-ENDPOINT: Add Daemon Health Check Socket

**Status**: Not started  
**Estimated effort**: 3-5 hours  
**Priority**: Low (operational visibility)

#### What was planned
A Unix domain socket (UDS) endpoint that provides JSON health status for the sync daemon:
- Socket created at `~/.local/state/dracon/sync.sock` on daemon start
- `curl --unix-socket ~/.local/state/dracon/sync.sock http://localhost/health` returns JSON
- Reports: daemon uptime, last sync time, number of repos synced, error count

#### Why it's stuck
1. **No immediate need**: The daemon already logs health status and provides `dracon-sync health` CLI command. The UDS endpoint is a convenience for monitoring tools, not a core feature.
2. **Socket lifecycle management**: Creating a Unix socket requires handling:
   - Socket creation on daemon start
   - Socket removal on daemon stop/crash
   - Stale socket cleanup on startup (if previous daemon crashed)
   - Permissions (who can connect to the socket)
3. **Tokio integration**: Adding a UDS listener to the existing tokio runtime requires careful integration with the sync loop to avoid blocking.
4. **Security considerations**: The socket must be restricted to the user's processes. Other users on the system should not be able to connect.

#### Options for future work
- **Option A (Minimal)**: Just the health endpoint — no process management, no monitoring integration. Estimated: 2-3 hours.
- **Option B (Full monitoring)**: Health endpoint + Prometheus metrics export + alerting hooks. Estimated: 6-8 hours.
- **Option C (Systemd integration)**: Use systemd's `Type=notify` and `WatchdogSec` instead of a custom socket. The daemon sends `READY=1` and periodic `WATCHDOG=1` to systemd. Estimated: 1-2 hours.

#### Recommendation
Option C (systemd integration). It's simpler, uses existing infrastructure, and provides the same operational visibility without custom socket management.

---

### 4. L-ASYNC-UNIFY: Unify Sync Git Calls to Async

**Status**: Not started  
**Estimated effort**: 4-6 hours  
**Priority**: Medium (code quality)

#### What was planned
Converting 30+ synchronous `std::process::Command` git calls in `sync.rs` to async `tokio::process::Command`. This would:
- Prevent blocking the tokio runtime during long git operations
- Allow parallel git operations (e.g., push to multiple remotes simultaneously)
- Improve daemon responsiveness under load

#### Why it's stuck
1. **Blocking calls are intentional**: Many git operations are short-lived (< 100ms). Converting them to async adds overhead without benefit. The daemon already uses `spawn_blocking` for truly long operations (push, clone).
2. **Test infrastructure**: All 406+ tests use synchronous `std::process::Command`. Converting to async requires either:
   - Converting all tests to async (massive effort)
   - Using `tokio::runtime::Runtime::block_on()` in tests (adds complexity)
3. **Interaction with libgit2**: Some git operations use libgit2 (via `git2` crate), which is synchronous. Converting the `Command` calls to async but leaving libgit2 calls synchronous creates an inconsistent API.
4. **No measurable benefit**: The daemon's performance is limited by network I/O (push/pull), not CPU. Async git calls wouldn't improve throughput.

#### Options for future work
- **Option A (Selective)**: Only convert the long-running operations (push, pull, clone) to async. Leave short operations (status, diff, log) synchronous. Estimated: 2-3 hours.
- **Option B (Full conversion)**: Convert all git calls to async, including test infrastructure. Estimated: 6-8 hours.
- **Option C (Abandon)**: The current mix of sync/async is functional and performant. No conversion needed.

#### Recommendation
Option C (abandon). The daemon is already performant. The effort-to-benefit ratio is poor. Only reconsider if profiling shows blocking git calls as a bottleneck.

---

## Common Blocker Patterns

### Pattern 1: Revert on Failure
Three of four tasks (H-SEC-LIB, H-DAEMON, L-ASYNC-UNIFY) were attempted, encountered issues, and reverted. This suggests:
- **The codebase is stable** — maintainers prefer working code over risky refactors
- **Test coverage is high** — broken tests are caught quickly
- **The refactor scope was too ambitious** — trying to do everything at once

### Pattern 2: Tight Coupling
The daemon and warden modules have high internal coupling. Functions depend on shared state, making extraction difficult. This is a design choice — the code prioritizes simplicity over modularity.

### Pattern 3: Diminishing Returns
The refactoring tasks target working, stable code. The benefit of cleaner code structure is real but small compared to the risk of breaking working functionality.

## Recommendations for Future Work

1. **Incremental over big-bang**: Extract one function at a time, not entire modules
2. **Test after every change**: Run the full test suite after each extraction
3. **Defer low-priority items**: L-HEALTH-ENDPOINT and L-ASYNC-UNIFY have low value — only pursue if needed
4. **Focus on high-impact items**: H-SEC-LIB is the most valuable refactor (code organization) but should be done incrementally
5. **Consider the "leave it alone" option**: Stable, working code doesn't always need refactoring

## Files Referenced

- `dracon-warden/src/security/src/lib.rs` — 1534+ lines, target for H-SEC-LIB
- `dracon-sync/src/daemon.rs` — contains cooldown logic for H-DAEMON
- `dracon-sync/src/sync.rs` — contains 30+ sync git calls for L-ASYNC-UNIFY
- `dracon-sync/src/report.rs` — health status reporting for L-HEALTH-ENDPOINT
- Git history: commits `dafddd12`, `4adcf16d`, `2c669a1a`, `44e43137` show the attempted/reverted refactors

---

*Analysis completed 2026-06-01. All 4 tasks are deferred with documented rationale.*
