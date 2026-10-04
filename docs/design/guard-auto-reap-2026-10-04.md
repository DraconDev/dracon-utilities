# Guard auto-reap of abandoned dev servers (2026-10-04)

## Context

Test runs and agent sessions launch dev servers (`vite`, `eve web`,
Playwright/Chromium, `bun run <script>`, ad-hoc `http.server`) as
children. When the run is killed rather than completing -- a `timeout`
deadline, a SIGKILL, a crashed agent -- those children are reparented and
keep listening forever. A 2026-10-04 survey found **97** server-ish
processes, including 7-day-old e2e harnesses and five `stub-companion`
zombies whose `/tmp` scripts were long gone yet still running.

`reap.rs` already *reported* these (sleeping, no tty, near-zero lifetime
CPU, old, allowlisted signature). The report was explicitly the whole
story: the module's contract was that the guard never ends anything
itself. The operator has now chosen guard auto-reap, which **reverses
that contract under an explicit opt-in**.

## Decision

- New policy knob `reap_stale_dev_servers` (default **false**). Destructive
  behavior requires an explicit opt-in, matching the repo's precedent
  (`revert_excluded_to_head`, `auto_cleanup_apply`). The report stays a
  human worklist when the knob is off.
- Same certainty bar as the report, re-checked live at kill time: state
  `S`, `tty_nr == 0`, allowlist signature, exemptions, CPU ceiling --
  plus a `starttime` equality check against PID reuse. Age is not
  re-checked (a process only gets older). Any doubt fails closed: the
  candidate is recorded as unverified and no signal is sent.
- Reserved PIDs (<= 1) and the guard's own PID are refused outright, in
  both the verifier and the terminator (defence in depth).
- SIGTERM first, 5s grace, then SIGKILL, 2s grace. Zombies count as dead
  (a zombie still answers `kill(pid, 0)` but holds no port, memory, or
  CPU). EPERM is a refusal, surviving SIGKILL is a recorded failure.
- The pass runs on a `spawn_blocking` thread: grace sleeps must never
  stall a tokio worker (cf. the R4-SC-04 inline-`gc` lesson).
- Everything is recorded: the report gains a `reaped` array (kills AND
  skips), and every action is echoed to stderr for the journal.

## Verification

- 15 new tests in `reap_tests.rs` (fixture proc tree + real own-children
  for the signal paths) and 1 policy default test. Pre-change proof: the
  new tests failed to compile before the implementation landed.
- The tests caught two real bugs during development: zombies answering
  the liveness probe (fixed by treating `Z`/`X` as dead), and a
  TERM-during-interpreter-startup race in the escalation test (fixed with
  a readiness handshake).
- Live smoke through the real binary: with the opt-in off the marker was
  listed and left alive; with it on the marker was re-verified,
  SIGTERMed, and recorded in both JSON and stderr.

## Residuals

- The verify-then-kill window (microseconds) cannot be closed further
  from userspace; kill-by-PID is inherently racy and the starttime check
  is the mitigation, not a cure.
- Only the matched process is signalled, not its children: a killed
  harness wrapper can orphan a grandchild sleeper (observed in the smoke
  test, cleaned by hand). Process-group kill is deliberately NOT used --
  the guard must never signal a PID it did not verify.
