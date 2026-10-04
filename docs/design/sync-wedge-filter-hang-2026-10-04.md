# Sync wedge: filter-process hang pins tasks 7.5h (2026-10-04)

## Observed

Two `git status --porcelain -z` + two `dracon-warden filter-process`
ran 7.5h as daemon children (freeport, pi-goal-list-loop-audit).
Filter stuck in `futex_do_wait` (main thread; one tokio thread in
epoll); git stuck reading its filter pipe; sync task never completed;
repos pinned in-flight; trailing-drain message every cycle; sync at
30–41% CPU; box load hit 58. Killing the two filters cascaded cleanly
(git parents exited, no orphans); load fell to 12 within minutes.

## Confirmed mechanism (filter → git → task)

`filter.dracon.process` + `* filter=dracon` routes every file through
warden. A wedged filter wedges git's status, which wedges the owning
sync task. Did NOT reproduce on a clean `git status` (exit 0, instant)
— the hang needed the transient state of 7.5h earlier (fd showed git
mid-read of a worktree file). Warden-side root cause (which futex, what
it waits on) is still open — needs a live repro under strace or a
core dump, not more code reading.

## Wedge-abort gap (sync side)

The 15-min wedge abort (`request_worker_cancellation` → `AbortHandle`)
fired ONCE per repo (4 warnings, 09:12–09:28) then went silent for 7h —
no repeat warnings despite the clock resetting after each warning. Two
candidate explanations, undetermined: (a) the abort dropped the future
but the children survived (kill gap), the cancelled handle was consumed,
and the repo was never re-warned; (b) the tasks finished from the
daemon's view while children lived on. The existing
`classification_cancellation_terminates_git_process_group` fixture
proves the kill machinery works when the future drops — so the live
tasks' futures were likely never dropped, i.e. never under a timeout.

## Untimed-spawn hunt (negative result)

The stuck argv (`git -c core.hooksPath=/dev/null status --porcelain
-z`) matches NO production spawn site in the tree OR in the running
0.113.92 tag: the only prod status spawn (report.rs) carries the 8s
PORCELAIN_BUDGET + kill_on_drop + process-group guard, and no prod code
injects `-c core.hooksPath`. Searched: all `status`/`porcelain` spawns,
`GIT_CONFIG_*`, daemon environ for a wrapper, `-c` literals,
spawn_blocking git waits, storage journal. Either an untimed path
assembles that argv indirectly, or the flag was injected outside the
codebase. This is the lead to pull if it recurs — with `debug_enabled`
scheduler logs or an argv audit, not another grep sweep.

## Follow-ups

1. Warden: reproduce the filter-process futex hang (soak a filtered
   status under memory pressure?) and fix/timeout at that layer.
2. Sync: audit why wedge warnings don't repeat; consider a hard cap on
   detached-registry task age (kill + reap, release repo) so no wedge
   can pin a repo longer than N minutes regardless of path.
3. Sync: argv audit — log full argv at every git spawn (debug level) so
   the next wedge's provenance is one journal line, not a hunt.
