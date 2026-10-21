# Performance investigation — 2026-10-21

## Measurements

At 01:43 BST the 16-thread Ryzen 7700 host had a load average of 201,
30 GiB RAM used, about 180 MiB available, and 63 GiB swap used.
Vmstat measured 160–290 MiB/s swap-in, 200–300 MiB/s swap-out and
46–58% I/O wait. Memory PSI full was 65% over ten seconds. Root had
138 GiB available; this slowdown was active memory overcommit and paging.

Two large agent scopes had thousands of tasks: 3,446 and 4,534. Systemd
reported approximately 14.3 GiB and 6.2 GiB swap respectively. Both mixed
agents and browser/test children, so whole-scope termination would affect
active work. RSS summed over browser children can double-count shared pages;
the scope counters are preferable for aggregate accounting.

An inventory of orphaned Chrome roots with temporary profiles older than
one hour is saved at `/tmp/dracon-performance-orphan-browsers-20261021.json`.
It records PID/starttime, profile, age and descendant accounting. Fourteen
use disposable `cw-browser-test-*` profiles; a separate `yff-live-profile`
instance is outside the proposed closure. Operator approval was requested
for graceful termination of only the fourteen disposable test instances.
No browser was terminated or profile deleted during this investigation.

## Immediate actions and bounded fix

The guard had waited almost ten minutes on its own `nix-collect-garbage`
child before reaching memory checks. Its PID, exact command and parent were
verified, then it received SIGTERM through a pidfd. It reported an ordinary
interruption at 01:50. The service remained running. This does not fix the
underlying memory overcommit; later memory/IO PSI improved but stayed severe.

Clean Web's browser harness had an actual escalation defect: after SIGTERM,
`killGroup` returned when the process group still existed, so SIGKILL was
never attempted. Changed that return to continue. A regression launches only
its own detached Node child, waits until its SIGTERM-ignore handler is ready,
and verifies cleanup reaches SIGKILL. The regression and workspace typecheck
pass. This explains a leak path, not proof that every inventoried browser
used that exact path.

## Further guard work

- Bound external cleanup commands and keep memory monitoring independent of
  slow storage maintenance. Merely moving memory checks earlier still leaves
  long gaps while a later command blocks the same loop.
- Admit browser/build/test jobs with explicit per-job ownership and bounded
  concurrency. CPU throttling alone cannot free their resident working sets.
- Keep browser cleanup scoped to verified disposable jobs. Generic Chrome
  exemptions protect normal desktop sessions and must not be removed globally.
- Record swapped footprint/cgroup pressure alongside RSS. Top RSS alone misses
  the largest processes after the kernel has evicted most of their pages.

No memory caps, cache drops, swapoff, blanket process kills, credential
cleanup, or changes to the operator's desktop browsers were applied.

## RAM attribution follow-up, 02:00–02:05 BST

The largest identified browser was not one of the fourteen Clean Web roots.
It was PID 400399, a separate headless YouTube Filter Chrome using
`/tmp/yff-fresh-profile`, CDP port 9555, running for about fourteen hours.
Read-only `/json/list` inspection found 175 page targets, including 174
YouTube pages. A later sample found 178 pages: accumulation was continuing.
Its tree contained 184 processes. A proportional measurement of 182 surviving
processes found 883 MiB PSS and 16,082 MiB SwapPss, about 16.6 GiB combined;
this avoids the inflated 22.5 GiB sum of per-process RSS and VmSwap.
Closing this specific test browser was separately proposed for approval.

The broader process snapshot had 414 Chrome/Chromium processes, 38 Vite/WXT
processes, 43 test/typecheck processes and 29 actual Pi agent processes.
Approximate summed swapped footprints were Chrome 24.2 GiB, Vite/WXT
11.5 GiB, agent-category processes 9.9 GiB and tests/typechecks 5.0 GiB.
These are process accounting estimates, not proportional physical RAM totals.
Deathrun alone had eighteen Vite-related processes at the snapshot; music
had six. Several dev servers were 18–52 hours old, including orphaned ones.

Zram explains much of the occupied physical RAM: `zramctl` measured 58.6 GiB
of logical swapped data compressed into 16.7 GiB actual RAM. The 64 GiB disk
swapfile held only about 1.4 GiB at that sample. `/etc/nixos/configuration.nix`
intentionally configures zstd zram with memoryPercent=200 and priority=5.
That is a logical swap capacity, not a reservation of twice the machine's
RAM. Nevertheless the compressed pool currently consumes over half its RAM,
leaving little for active working sets and causing repeated reclaim/faults.
No live swap configuration was changed; swapoff during this pressure would
be unsafe. The immediate causes to remove are leaked targets and excessive
concurrent workloads, rather than blindly retuning the existing swap policy.

The YouTube Filter CDP driver opened targets but its `close()` only closed
the WebSocket. It now tracks targets created by that connection, awaits their
closure, and leaves pre-existing targets alone. Receipt scripts call cleanup
in finally blocks, including failure paths. Completed CDP request timers are
also cleared. Two targeted ownership regressions, workspace typecheck and
Node syntax checks pass. No live target was closed during verification.
The entire browser suite was not rerun while the host was thrashing.
