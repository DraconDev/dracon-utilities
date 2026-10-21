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
