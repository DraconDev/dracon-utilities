# System thrash: recursive Polis release gate — 2026-10-17

## Observed incident

At 15:52 local, 30 GiB RAM had only 183 MiB available, swap usage was
62 GiB and load average was 104.28. Memory PSI full avg10 was 66.39%;
I/O PSI full avg10 was 49.81%. This was active swap thrashing, not disk
space exhaustion (79 GiB free on root).

Process accounting found 71 bun processes with approximately 18 GiB
swapped out. Seventeen identical full-suite processes were still alive
when targeted cleanup ran. Their working directory was
`dracon-platform/web/games/wip/polis`. Nine orphaned ffmpeg encoders,
about 20 hours old, held another approximately 11–12 GiB swapped out.
The encoders and their output files were left intact.

## Root cause and bounded fix

`src/lib/regression-contract-evidence-pin.test.ts`, REQ-02 (h), runs
`scripts/verify-release.sh`. That script runs `bun run test`, whose full
suite includes REQ-02 (h) again. Per-command timeouts did not prevent
recursive fan-out or orphaned descendant typechecks.

Changes in the Polis checkout:

- The release script sets `POLIS_RELEASE_GATE_ACTIVE=1` for its unit-test
  subprocess only.
- The outer release-contract test explicitly skips only its self-invocation
  when that marker is present. All other unit tests and release checks
  remain enabled. The outer test also sets the marker for its release
  subprocess, preventing recursion regardless of entry path.
- Existing uncommitted timeout changes in the test file were preserved.

Cleanup sent SIGTERM using pidfds to 17 exact-command, exact-cwd full
suite processes, then to two verified parent-dead Polis svelte-check
processes. No agent/editor session, video encoder, source file or video
output was killed/deleted.

## Verification

- `bash -n scripts/verify-release.sh`: exit 0.
- `POLIS_RELEASE_GATE_ACTIVE=1 timeout 15 bun test src/lib/regression-contract-evidence-pin.test.ts --test-name-pattern 'REQ-02 \(h\)'`:
  expected explicit nested self-check skip, 0 failures, 40 ms. This is a
  recursion guard check, NOT a full release qualification.
- `git diff --check`: exit 0.
- At 15:54:52, recursive full-suite count was zero, available RAM was
  6.5 GiB, swap use 32 GiB and one-minute load was falling (34.47).
- Earlier post-cleanup vmstat interval showed 62–70% CPU idle and no
  swap-out, compared with near-zero idle and ~600–750 MiB/s swap-out
  during the incident. Memory PSI full avg10 fell to 0.20%.

## Separate remaining guard defect

`dracon-system-guard.service` was active throughout, but CPU-cap actions
failed with `Read-only file system` writing cgroup.procs. Live unit has
`ProtectControlGroups=yes` and `Delegate=no`. No sandbox change or guard
restart was attempted during emergency relief. This requires a separate
service-hardening review: do not simply disable sandbox protections.

Residual swap usage alone does not establish ongoing pressure; cold
pages can remain swapped out after recovery. Do not use swapoff to force
32 GiB back into a 30 GiB machine. Full release gates were deliberately
not launched during recovery.
