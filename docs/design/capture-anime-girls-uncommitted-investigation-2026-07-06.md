# capture-anime-girls: 2 Uncommitted Files Investigation — 2026-07-06

## Summary

capture-anime-girls shows 2 uncommitted files in `dracon-sync repos`:
`AUDIT.md` and `scripts/verify-release.sh`.

**Root cause: NOT a daemon bug in the settle heuristic.** These are legitimate
leftover edits from a prior agent/pi session (the Phase 80k audit-fix sweep,
tied to the paused Sisyphus goal `mr7p34aa-52ioat`). The daemon DID eventually
commit them (commit `cc09222` at **22:42:46**, "2 file(s) in scripts
[AUDIT.md, scripts/verify-release.sh] DELTA:+196/-3"), but only after a
**27-minute stall** (22:15:31 → 22:42:14 with zero commits across ALL repos).

The "settle" heuristic itself is fine and simple (see below). The delay was
caused by the daemon's `in_flight` gating combined with a stalled push on the
4 GB hegemon repo, NOT by the inactivity timer. See "Why the 27-minute stall".

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

## What "settle" actually means (the heuristic)

The daemon's commit decision in `src/daemon.rs:~2778` is:

```rust
const MAX_DIRTY_DELAY: Duration = Duration::from_secs(5);
let enough_time = entry.dirty_since.is_some_and(|since| {
    now.duration_since(since) >= MAX_DIRTY_DELAY
}) || now.duration_since(entry.changed_at) >= inactivity_delay;
```

where `inactivity_delay = Duration::from_secs(
policy.inactivity_push_delay_secs.max(1))` and
`changed_at` is bumped only when the repo's fingerprint *changes* between scans.

So "settle" = **2 seconds of fingerprint inactivity** (`inactivity_push_delay_secs = 2`
in the global config) OR a **5-second hard cap** for continuously-editing repos
(`MAX_DIRTY_DELAY`). It is exactly the simple "X seconds of inactivity" the
operator expected. `pulse_interval_secs = 1`, so the loop re-scans every 1s.

**The inactivity condition for these 2 files was satisfied by 22:17:08**
(2s after the last write at 22:17:06). The operator's intuition was correct:
if it were purely the settle timer, they would have been committed within
seconds.

## Why the 27-minute stall (the real cause)

The `enough_time` check only runs for repos that are NOT already `in_flight`.
A repo gets marked `in_flight` the moment the daemon dispatches its sync task
(commit + pull + push), and is skipped in subsequent cycles until the apply
phase removes it.

Timeline:

```
22:15:05  capture-anime-girls: scaling push timeout 900s -> 600s (1 commits ahead)
          ^-- a push was DISPATCHED and capture-anime-girls entered in_flight
22:15:07  committed dracon-platform (4 files)
22:15:31  dracon-platform scaling push
22:17:06  AUDIT.md modified (2 new uncommitted files appear in capture-anime-girls)
          ^-- but capture-anime-girls is STILL in_flight from the 22:15 push,
             so the daemon SKIPS re-evaluating it; the new files wait
... 27-minute gap: zero commits for ANY repo ...
22:29-30  hegemon's 4 GB github push observed running pack-objects (blocks throughput)
22:42:46  committed capture-anime-girls (2 files)   <-- the 2 files land
22:42:55  trailing-drain: clearing 4 stuck in_flight entries
          (capture-anime-girls, dracon-platform, junk-runner, polis)
```

The `in_flight` push dispatched at 22:15:05 stayed unresolved for ~27 minutes
because it was contended with / serialized behind hegemon's 4 GB github push
(which times out against github's 2 GB limit and gets retried with a scaling
timeout: 900s -> 600s). The daemon uses BOUNDED PARALLEL SYNC
(`tokio::spawn` + `FuturesUnordered`, bounded by `sem_max_concurrent_sync = 4`),
and the apply phase has a 2-second deadline
(`apply_deadline = pulse_interval_secs * 2`) after which slow pushes are left
in `in_flight` rather than blocking the loop. So hegemon's giant push did NOT
freeze the whole loop — but it kept capture-anime-girls' earlier push slot busy
(or its own slot saturated the semaphore), so capture-anime-girls remained
`in_flight` and was skipped every cycle until that push resolved at ~22:42:46.

**Net: a repo with a stuck/slow in-flight push will NOT pick up new local
edits (no matter how long they've been idle) until that push resolves.** That
gating — not the settle timer — is what delayed these 2 files by 25 minutes.

## Verification that this is NOT a settle-timer bug

- `git check-ignore AUDIT.md scripts/verify-release.sh` → **empty** (neither is
  ignored, so the daemon would normally auto-commit both).
- `git diff --name-only` → exactly the 2 files (no other untracked/staged state).
- `lsof` → **no process has either file open** (not being actively edited).
- Both diffs are complete, finished edits (not partial/interrupted writes).
- The daemon's own hint explicitly says it will handle them after changes settle.
- CONFIRMED: the files were committed at 22:42:46 (`cc09222`) once the
  `in_flight` push resolved. Working tree is now clean.

## Recommendation

No action required. These were legitimate audit-fix deliverables and are now
committed. But note the underlying daemon behavior: a repo with a stuck
`in_flight` push will not re-commit new local edits until that push resolves,
even if those edits have been idle far longer than `inactivity_push_delay_secs`.
If the operator wants to avoid 25-minute delays on small repos when a giant
repo (hegemon, 4 GB) is pushing, options to consider (separate task):
- Decouple the commit decision from `in_flight` (commit new local edits even
  while a prior push is still in flight, then queue another push).
- Lower `sem_max_concurrent_sync` contention impact, or give large repos their
  own push budget.
- The hegemon/github exclusion (already a known follow-up) would remove the
  4 GB push that saturates the semaphore in the first place.
