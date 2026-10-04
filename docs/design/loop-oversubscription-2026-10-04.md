# Loop oversubscription + pressure blindness (2026-10-04)

## Symptom

Interactive performance bad despite the disk work: load 27–31 on 16
cores, swap thrashing at 55k–250k pages/s in AND out, 1.6G available of
30G at the worst sample. Waves, not a steady state.

## Cause: 20 loops > 16 cores + 30G RAM

Twenty live `pi` agent loops (fifteen game loops started within minutes
of each other, ~4.5h old at discovery) running bun tests, rustc builds,
and svelte-check concurrently: 29 heavy procs burning 706% CPU in one
sample. An orphaned `bun test` (parent dead, reparented to systemd) held
8G alone before dying on its own. No single leak or runaway — the fleet
of loops is simply bigger than the box. Per-loop attribution (subtree
CPU/RSS per pi root) is the pause-decision input; nothing here is the
guard's to kill while the owners are live.

## Why the guard stayed silent

Three stacked gaps, all verified in code against live readings:

1. Swap-in velocity was measured ONLY as a PSI-absent fallback. With
   PSI present the guard never looked at swap-in rate — so si at 100k+
   pages/s with PSI full at 3.9 classified "ok".
2. `mem_psi_full_warn = 10.0` is far above the pain threshold; the box
   is unusable well before 10% full-stall. Left unchanged (operator
   config); the velocity signal covers the gap.
3. renice mitigation is disarmed (service lacks `CAP_SYS_NICE`), so even
   a fired verdict could only OOM-bias — and nothing fired.

## Fix (this date)

Swap-in velocity is a first-class thrash signal: measured every pass,
verdict at or above new knob `mem_swapin_warn_pages_per_sec` (default
1000, the legacy fallback constant — PSI-absent hosts unchanged).
Swap-out stays a non-signal (reclaim working, not pressure);
unmeasured fails closed. The existing 120s sustain still smooths spikes.

## Follow-up: pressure-gated orphan reap (same date)

The 24h auto-reap floor cannot catch a runaway whose owner just died,
so a new opt-in (`reap_orphans_on_pressure`, default OFF) reaps under
warn/critical pressure with NO age/CPU/state gates when ALL of these
hold: allowlisted signature, parent reparented to init/systemd (owner
provably dead), no controlling terminal (disowned shell jobs keep
theirs), not exempt, not reserved/self. Kill-time re-verification
(starttime + still-orphaned + still-detached + still-allowlisted)
replaces the idle proof. Proven live in-test against a real setsid
orphan (scan → verify → SIGTERM → recorded, 0.15s).

## Residuals

- Concurrency itself is unmanaged: the guard can now SEE thrash and
  bias OOM, but nothing caps how many loops run heavy jobs at once.
  CPUQuota offender caps are ARMED at 50% (2026-10-04) and engage on
  critical verdicts; first engagement still to be observed.
- dracon-sync burned ~40% CPU through the incident in a commit storm
  fed by loop file writes — secondary, settles with the loops. (The
  7.5h wedged tasks were a separate filter-hang issue; see
  `sync-wedge-filter-hang-2026-10-04.md`.)
- CAP_SYS_NICE: already granted by the unit AND effective in the live
  daemon (verified CapEff + renice lines in journal). The "renice
  mitigation disabled" message seen during diagnosis came from a
  shell-context `guard once` smoke test, not the daemon. No action.
