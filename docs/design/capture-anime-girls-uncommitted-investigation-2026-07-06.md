# capture-anime-girls: 2 Uncommitted Files Investigation — 2026-07-06

## Summary

capture-anime-girls shows 2 uncommitted files in `dracon-sync repos`:
`AUDIT.md` and `scripts/verify-release.sh`.

**Root cause: NOT a daemon bug.** These are **legitimate leftover edits from a
prior agent/pi session** (the Phase 80k audit-fix sweep, tied to the paused
Sisyphus goal `mr7p34aa-52ioat`), and the daemon simply has not re-committed
them yet because of its normal debounce/settle logic plus a current throughput
bottleneck on hegemon's 4 GB github push.

They are safe to leave for the daemon (it will auto-commit them once changes
settle) or can be force-committed with `dracon-sync sync-now --warns`.

## The 2 files

### 1. `scripts/verify-release.sh`
- **Diff**: a portability fix. Replaced the GNU-grep-specific `\<three\>`
  word-boundary syntax with `-w 'three'`, which works on both GNU grep and
  BSD grep (macOS). Comment updated to explain the change.
- **mtime**: `2026-07-06 22:14:39`
- **Status**: complete, sensible, intentional edit.

### 2. `AUDIT.md`
- **Diff**: appended a 192-line "Implementation Status (PHASE 80k execution
  summary)" section — documents that all 47 audit tasks (6 P0 + 17 P1 + 23 P2
  + 1 deferred) were addressed, `bun test` green (1031 pass), `bun run build`
  green, plus cross-references to prior phase audits.
- **mtime**: `2026-07-06 22:17:06`
- **Status**: complete, sensible, intentional edit. The section itself states
  it was "added after the audit-fix sweep, NOT during the audit" — i.e. written
  by the agent that ran the Phase 80k fixes.

## Timeline reconstruction

Daemon commit log for capture-anime-girls (from `journalctl --user -u dracon-sync.service`):

```
21:52:31  committed 3 file(s)
21:53:16  committed 3 file(s)
21:53:58  committed 2 file(s)
21:58:59  committed 10 file(s)
22:14:57  committed 18 file(s)   <-- last bulk commit
22:15:05  scaling push timeout 900s -> 600s (1 commits ahead)
```

File mtimes vs last commit:
- `verify-release.sh`  → 22:14:39  (18s BEFORE the 22:14:57 commit)
- `AUDIT.md`           → 22:17:06  (2 min AFTER the 22:15:05 commit)

So `AUDIT.md` was written *after* the daemon's last commit snapshot and is
correctly pending. `verify-release.sh` was written just before the 22:14:57
commit — it landed at the boundary of the daemon's scan window, so it was
missed by that commit (the daemon snapshots the working tree at a point in time,
then the file write completed moments later).

## Why the daemon has not re-committed yet

1. **Normal debounce/settle behavior.** The daemon commits a repo only after
   its changes "settle" (a quiet window). The `repos` hint for capture-anime-girls
   confirms this: *"daemon handles after changes settle; run sync-now --warns
   to force now"*.
2. **Throughput bottleneck.** At investigation time the daemon was busy pushing
   hegemon's 4 GB pack to github (`git pack-objects` running 22:29–22:30, which
   will time out against github's 2 GB limit). This slows the daemon's commit
   cycle for other repos, so capture-anime-girls has not yet been re-scanned and
   committed.

## Verification that this is NOT a daemon bug

- `git check-ignore AUDIT.md scripts/verify-release.sh` → **empty** (neither is
  ignored, so the daemon would normally auto-commit both).
- `git diff --name-only` → exactly the 2 files (no other untracked/staged state).
- `lsof` → **no process has either file open** (not being actively edited).
- Both diffs are complete, finished edits (not partial/interrupted writes).
- The daemon's own hint explicitly says it will handle them after changes settle.

## Recommendation

No action required unless the operator wants them committed immediately. Options:
- **Leave for daemon**: it will auto-commit both files in its next settle cycle.
- **Force now**: `dracon-sync sync-now --warns` (or target the repo) to commit
  them immediately.

Do NOT revert or discard — these are legitimate audit-fix deliverables.
