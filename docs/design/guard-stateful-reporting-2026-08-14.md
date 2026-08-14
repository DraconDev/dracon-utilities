# Guard stateful reporting, pressure classification & cleanup cadence — 2026-08-14 (v0.112.37)

**Scope**: `dracon-system` guard daemon (`guard daemon` / `guard once`).
**Release**: v0.112.37.
**Behavior change**: the guard went from "instant, repeating" reporting to a
per-condition state machine, and from "swap occupancy = incident" to
sustained multi-signal pressure classification.

## Why

The v0.112.35/36 guard added many detectors (disk trends + rapid-fill,
zombies, memory/swap pressure, heavy processes, cleanup candidates). On a
busy machine each detector could fire repeatedly:

1. A persistent heavy process was re-notified every `notify_cooldown_secs`.
2. Swap occupancy alone was classified as memory pressure even with plenty
   of free RAM and zero PSI stall — likely false alarms and, worse, a
   trigger for mitigation (renice/OOM bias) after the sustain window.
3. A disk sitting in the ordinary `warn` band re-notified on every service
   restart (fresh process state = "changed").
4. While disk stayed above action level, `run_auto_cleanup` re-walked the
   whole `~/Dev` tree (`du` + `find`) every guard cycle (30 s) in
   report-only mode — constant I/O, no new information.

Constraints from the operator: quiet/coalesced/stateful alerts; explicit
approval for destructive actions; no automatic sync freezing; swap
occupancy is telemetry, not an incident.

## The state machine

`report_state_transition(state, key, value, repeat_secs)` in
`src/main.rs`:

- `value` differs from the last recorded value → **emit now** (entry,
  escalation, recovery) and update `last_emitted`.
- `value` unchanged and non-`"ok"` → emit only when `repeat_secs` elapsed
  since the last emission (heartbeat, default 1800 s).
- `value` unchanged and `"ok"` → never emit.

Every detector uses it: memory pressure, zombies, heavy processes, rapid
disk fill, early disk warning, disk state changes, auto/proactive cleanup
outcomes. `GuardRuntimeState.report_states` holds the per-key
`(value, last_emitted)`; stale keys are dropped when the underlying
condition clears (e.g. heavy-process keys are removed once the process
incarnation is no longer heavy).

## Memory pressure classification (hysteresis)

`classify_memory_pressure(mem_low, swap_high, psi_or_swapin_active)`:

- `mem_low && (swap_high || psi_or_swapin_active)` → `critical`
- `mem_low || psi_or_swapin_active` → `warn`
- else → `ok`

Swap occupancy is never sufficient by itself. `stabilize_memory_pressure_at`
then requires the observed state to persist `memory_pressure_sustain_secs`
(default 120) before the stabilized `pressure` changes; transients only
update the candidate. Mitigation (`auto_renice_on_memory`,
`bias_oom_on_pressure`, `cap_offenders_cpu_percent`) and notifications key
off the **stabilized** state. `guard once --json` exposes both
`observed_pressure` and `pressure`.

## Heavy-process alerts (per-incarnation keys)

Alert state keys are `heavy-process-{pid}-{starttime}` where starttime is
`/proc/<pid>/stat` field 22. Consequences:

- One notification on sustained entry, one on stuck-candidate escalation,
  none in between.
- PID reuse cannot inherit silence (new starttime = new key).
- Keys are pruned when the incarnation leaves the heavy set, so a later
  recurrence gets a fresh notification.

## Disk notifications

`check_disk_state_change` notifies only when the state changes within the
guard process lifetime (`last_disk_state`), or when the very first reading
is `critical`. Restarting the service at 82% (warn) is silent.
`check_disk_early_warning` is likewise transition-gated.

## Bounded cleanup scans

- `run_auto_cleanup` (action/critical disk states) now runs at most every
  `auto_cleanup_interval_secs` (default 1800, min 60), in apply **and**
  report-only modes. The timestamp is recorded *before* the scan so a
  persistent FS error cannot cause a 30-second retry loop.
  `auto_cleanup_due_at(state, interval, now)` drives the gate.
- Proactive stale-`target/` cleanup: example + live policy raised to
  `proactive_cleanup_percent = 80` (was 50) with the existing 120-cycle
  cadence — healthy disks are not scanned on a schedule.

## Defaults (v0.112.37, unchanged safety posture)

```toml
auto_renice = false              # CPU-heavy → report only
auto_renice_on_memory = true     # renice ONLY under sustained multi-signal pressure
bias_oom_on_pressure = true      # steers last-resort OOM victim; never kills
cap_offenders_cpu_percent = 0    # hard caps opt-in
freeze_sync_at_action = false    # disk pressure never pauses dracon-sync
auto_cleanup_apply = false       # cleanup is report-first
auto_cleanup_interval_secs = 1800
proactive_cleanup_percent = 80
memory_pressure_sustain_secs = 120
report_repeat_secs = 1800
notify_cooldown_secs = 120       # live policy; code default 300
```

## Verification

- `cargo test --locked`: 1363 passed, 9 ignored (added classification,
  hysteresis, cadence, and state-machine tests).
- `cargo clippy --workspace --locked -- -D warnings`: clean.
- `cargo deny check`: clean (bans/advisories/licenses/sources).
- Live `guard once --json` (2026-08-14): disk 82% `warn`, sync unfrozen,
  0 alerts, memory `ok`/`ok`, no limiters active.
- Release v0.112.37 published to crates.io, tagged `v0.112.37`, GitHub
  release + GitLab tag pushed; local service restarted on the released
  binary (fixture check passed).

## Related

- `dracon-system/CHANGELOG.md` [0.112.37]
- `dracon-system/release-notes-v0.112.37.md`
- `dracon-system/BLUEPRINT.md` (Guard Reporting & Pressure Classification)
- `dracon-system/dracon-system.example.toml` (knobs)
- Prior incidents: 2026-08-09/10 swap-thrash and disk-full events
  (`docs/design/disk-full-credentials-2026-08-10.md`) that motivated the
  v0.112.35/36 detector/limiter layers.
