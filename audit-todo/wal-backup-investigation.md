# wal-backup Daemon Sync Loop — Investigation Report

**Date:** 2026-05-22  
**Author:** Ralph audit loop (iteration 3)  
**Status:** Complete

---

## Executive Summary

The wal-backup repo's rapid DIRTY cycling is **not a bug in dracon-sync**. The daemon is operating correctly. The root cause is **a ralph-loop agent actively modifying files** in the wal-backup repo, creating a continuous cycle of change → commit → change → commit.

---

## Evidence

### 1. Ralph-loop Agent Active in wal-backup

**File:** `/home/dracon/Dev/wal-backup/.ralph/audit-fixes.state.json`

```json
{
  "name": "audit-fixes",
  "taskFile": ".ralph/audit-fixes.md",
  "iteration": 1,
  "maxIterations": 50,
  "active": true,
  "status": "active",
  "startedAt": "2026-05-18T22:42:42.341Z"
}
```

There is also a full ralph-loop configuration at `.ralph/audit-loop/RALPH.md` that runs automated checks (cargo check, clippy, tests, audit) and iterates up to 20 times. This ralph loop makes changes to files and commits them.

### 2. Files Being Modified by Ralph Agent

Recent commits show the ralph agent modifying:

| File | Change | Commit |
|------|--------|--------|
| `AUDIT_2026-05-22.md` | Updated audit findings (status: FIXED) | `3c42fa9`, `656e542`, `4063e53` |
| `scripts/edge_case_test.sh` | `BUCKET="${E2E_BUCKET_NAME:-dracon-master}"` | `3c42fa9` |
| `scripts/extensive_test.sh` | `BUCKET="${E2E_BUCKET_NAME:-dracon-master}"` | `3c42fa9` |
| `scripts/lifecycle_test.sh` | `BUCKET="${E2E_BUCKET_NAME:-dracon-master}"` | `3c42fa9` |
| `crates/wal-backup-daemon/demo.sh` | Fix lock file naming, hardcoded path | `48fbe29` |
| `todo.md` | Updated task progress | `fe0729c` |

### 3. Rapid Cycling Timeline

From the incident ledger (timestamps in Unix epoch):

| Timestamp (UTC) | Interval | Event |
|-----------------|----------|-------|
| 10:23:15 | - | Commit: update AUDIT |
| 10:26:40 | 3m25s | Commit: update AUDIT |
| 10:26:58 | 18s | Commit: update demo |
| 10:29:15 | 2m17s | Commit: update AUDIT + scripts |

The intervals (3min → 18s → 2min) show the ralph agent making rapid successive changes.

### 4. index.lock Contention

Two incident ledger entries show `fatal: Unable to create '/home/dracon/Dev/wal-backup/.git/index.lock': File exists` — the ralph agent and sync daemon are contending for git's lock file.

```json
{"ts_unix":1779144610, "result":"fail", "details":"git add failed ... index.lock: File exists"}
{"ts_unix":1779426451, "result":"fail", "details":"git add failed ... index.lock: File exists"}
```

### 5. Stuck `find` Process

```
PID 3733929, May 21, D state (uninterruptible disk sleep)
find / -name *.until-done* -path *wal-backup*
```

This is looking for ralph-loop completion marker files (`*.until-done*`) across `/`. Running as root-level `find /` with DFS traversal — could be stuck on a deep or blocked directory. Related to a ralph-runner invocation that may have been aborted or timed out.

---

## Analysis

### The Cycle

1. Ralph-loop agent (or pi agent) modifies files in wal-backup
2. Sync daemon detects DIRTY state in next scan cycle
3. Sync daemon waits `inactivity_push_delay_secs` (default 5s) for fingerprint to stabilize
4. If fingerprint keeps changing (agent is actively editing), it waits up to 5s (`MAX_DIRTY_DELAY`) then syncs anyway
5. Sync daemon commits changes → repo becomes clean
6. Ralph agent continues its work → files change again
7. Goto step 2

### Why This Is Not a Bug

- The sync daemon's **purpose** is to auto-commit all changes. It's doing its job.
- The `inactivity_push_delay_secs` / `MAX_DIRTY_DELAY` mechanism correctly handles actively-edited repos.
- The filter-only cooldown correctly handles cases where clean/smudge filters cause phantom changes.
- The `index.lock` guard correctly skips repos mid-checkout.

### The Real Problem

**When an AI agent (ralph, pi, or other) is actively working on a repo watched by sync, every agent action triggers a sync cycle.** This is by design — sync is supposed to commit everything. But it means:

1. **Ledger noise**: Every agent action generates an incident entry
2. **index.lock contention**: If both agent and sync run git concurrently, one will fail
3. **CPU waste**: Sync does full triage (diff, fingerprint, etc.) on every cycle for what are transient agent changes

### Solutions (Multiple Approaches)

#### Option A: Pause Sync During Agent Work (Recommended)

Before starting an AI agent on a repo watched by sync, pause sync:

```bash
dracon-sync pause
# ... do work ...
dracon-sync resume
```

Or add the repo to `exclude_repos` in dracon-sync.toml during agent sessions.

#### Option B: Exclude Agent Output Files

Add ralph/pi working files to the sync policy's `exclude_file_patterns`:
- `**/.ralph/**`
- `**/.pi/**`
- `**/.ralph-runner/**`

This prevents agent state files from triggering sync commits, but agent code changes would still trigger them (which is correct).

#### Option C: Improve Daemon-to-Agent Coordination

Add a mechanism where ralph/pi agents can signal "I'm working here, don't sync" — e.g., by setting `**/.freeze` marker or a specific file that the sync daemon recognizes.

#### Option D: Stuck Process Cleanup

The stuck `find` process should be investigated and killed if it's truly stuck:

```bash
kill 3733929  # if confirmed stuck (D state since May 21)
```

---

## Recommendations

1. **Short-term**: When running an AI agent on a watched repo, use `dracon-sync pause` first
2. **Medium-term**: Add ralph/pi working directories to `exclude_file_patterns` in the sync policy
3. **Cleanup**: Kill the stuck `find` process (PID 3733929)
4. **Monitoring**: The daemon's behavior is correct — no code changes needed

---

## Conclusion

**HYPOTHESIS confirmed:** The wal-backup rapid sync loop is caused by a ralph-loop AI agent actively modifying files in the repo. The sync daemon is operating correctly. No dracon-sync code changes are required. The fix is to add agent artifact exclusion patterns and document the pause-before-agent-work workflow.
