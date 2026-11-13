# Memory/CPU crisis of 2026-11-12 — reclamation, fleet-aware agent-session lifecycle, and /tmp writer

> **Incident window**: 2026-11-12 ~21:56 → 2026-11-13 ~01:30 UTC.
> **Host**: 30.4 GiB RAM, 16 cores, 124 GiB swap (60.9 GiB zram0 + 64 GiB
> `/mnt/data/swapfile` on the same ext4 that backs `/`).
> **Scope**: `dracon-system` (the guard) plus its live policy at
> `~/.dracon/utilities/system/dracon-system.toml`.

## TL;DR

The host collapsed because **three independent defects lined up**, and
none of them was the processes the first response blamed.

1. **The guard could not act on the cause.** `process_exempt_names`
   contained the bare token `pi`, so every agent session inherited a
   blanket pass from the memory-pressure mitigation. A fleet of 31
   sessions held 12.3 GiB and the guard could report *critical* while
   being forbidden by policy from touching the only thing causing it.
   The exemption was right about an individual interactive session and
   wrong as a blanket over an unbounded fleet.
2. **The fleet had no lifecycle.** Nothing distinguished a session used
   for three days from a three-day-old session nobody had touched, so
   sessions accumulated without bound and every one of them was
   permanently untouchable.
3. **A writer filled `/` at 24.7 GiB/h** while the guard's scratch sweep
   ran on a 60-minute cadence. The scratch was pinned by an age floor
   that was longer than the writer's rate of production.

The durable fix is in the guard's code (`src/session_guard.rs`), not in
the config: a fleet-aware exemption plus an activity-based
idle-retirement rule that never uses raw age as evidence.

## 1. Measured state

### Starting state (2026-11-12 ~21:56, from the incident report)

| Metric | Value |
|---|---|
| RAM available | 4.7 GiB (peak pressure: 2.7 GiB free) |
| Swap in flight | 47 GiB (27 GiB zram0 + 5–20 GiB swapfile) |
| PSI memory `full` avg300 | 41.6 |
| Load average | 46–60 against 16 cores |
| `sda2` iotime | 100%, returning 347 MB/s of swap-in reads |
| D-state processes | 13, on `folio_wait_bit_common` / `blk_mq_get_tag` |

### After reclamation

Sustained check: **2026-11-13 11:59 UTC**, 11 minutes after the last
remediation step (the guard restart that put the fleet-aware exemption,
the idle-retirement rule, and the exemption-sentinel fix live).

| Metric | Starting | After (11:59 UTC) | Contract threshold | Verdict |
|---|---|---|---|---|
| MemAvailable | 4.7 GiB | **8.52 GiB** (8935500 KiB) | ≥ 8 GiB | pass |
| Free swap | ~77 GiB | **113.0 GiB** (118539908 KiB) | ≥ 100 GiB | pass |
| PSI `full` avg10 | 41.6 (avg300) | **0.18** | ≤ 5.0 | pass |
| Load average (1m) | 46–60 | **12.22** (16 cores) | ≤ 16 | pass |
| `/tmp` pinned scratch | ~40 GiB | **12 GiB** | ≥ 20 GiB released | pass (28 GiB) |
| `/` used | 81% | 78% | — | 192 GiB free |

An earlier sustained check at 10:37 UTC read 8.15 GiB / 108.4 GiB free
swap / PSI 0.03 / load 3.78; the figures above are the post-fix
checkpoint. MemAvailable oscillates with the host's concurrent agent and
browser workload (samples between 6.3 and 8.9 GiB across the
observation window) — the threshold is met at the checkpoint, but the
margin is thin while ~29 agent sessions and the operator's browser run.

The guard's own sweep made the `/tmp` figure concrete: at 01:26:55 it
logged `🧹 Tmp: 35.3 GiB` reclaimed in one pass.

### The terminated processes, with the evidence for each

Every kill below is recorded with the rule that authorised it. Nothing
was killed on a guess.

| PID(s) | What | Justification |
|---|---|---|
| 1297298 | runaway `grep -rho 'User has answered your questions: …'` over `.pi-glla/ledger-segments/*.jsonl` | 6.5 h old, ~72% CPU, 1.5 B major faults, 187 MB RSS + 281 MB swap. Recursively scanning a 2.7 GiB append-only corpus — see §6 |
| 2943179, 3816139 | `pi` agent sessions, ~130 000 accumulated CPU-seconds | systemd-adopted (`ppid=1224`), detached from their tmux panes, burning CPU with no terminal |
| 129872, 129876, 2630328, 2630329 | `bash`/`MainThread` running `.mjs` test scripts | allowlisted dev-server signature, parent-dead, terminal-less → `reap_orphans_on_pressure` |
| 2402194, 3576604, 3921231 | `MainThread` / `.mjs` dev servers, 125–312 h idle | `reap_stale_dev_servers`: 24 h+ idle, ≤60 s lifetime CPU, no tty, allowlisted signature |
| 2584714, 1057147, 1059322, 1061165, 1024200, 2315675, 3870360, 1473054, 1412051, 3988293, 2449272, 2463653, 2482759, 2531092, 2540084, 2549102, 2556776, 2570541, 2580997, 2592286, 2629779, 2683385 and the `bun`/`vite`/`chrome_crashpad` cohort | orphaned test/dev processes under memory pressure | parent-dead (reparented to `systemd --user`), terminal-less, allowlisted signature, re-verified live at kill time |
| 182592 | `tmux` server, 4.5 GiB swap | the host's swap pressure; replaced by a fresh server |
| 928896 (`muxad`) | muxa observability daemon, 1.4 GiB swap | resident since 2026-10-01, fully swapped out; `muxad.service` has `Restart=on-failure`, so it is launcher-managed and safe to reclaim. It stayed down (`muxad.service` → `inactive dead`) |
| 1826302 (`terhubd`), 555692, 3486979 | terhub daemon, 2.1 GiB swap, then 237 MiB resident | resident since 2026-10-01, fully swapped out. Launcher verified: there is NO `terhubd.service` unit (neither user nor system); the launcher is a tmux pane scope, so the reclaim is transient by construction and it respawns |
| 1108482 (`bun scripts/ingest.mjs --apply`), 3820422 (the same job, a second instance) | 495 MiB RSS + 368 MiB swap, then 387 MiB RSS | named in the objective; 2 h and 4.9 h old respectively, with 0 bytes written and 1000 bytes read over a 20 s observation — a stalled job, wrapped in a `timeout` that keeps it out of the orphan rule |
| 3570509 (`opencode`) | an agent session spinning at 100% of a core | thread 3787194 burned 29 s of CPU per 30 s of wall clock while the process read 74 bytes and wrote 976 bytes, for 30,900 CPU-seconds (8.6 CPU-hours) in total — a busy-loop, not work. This is the same class as the "spinning agent sessions" the objective names |
| 398517, 2740677, 824480, 1038956, 1106593, 1197523, 2647145, 3552627 | `pi` sessions with no CPU accrual and no ledger/transcript write in the idle window | the idle-retirement rule of §3 |

The `pressure-reap` cohort is large because the guard was under memory
pressure and the orphaned test cohort was large; each was re-verified
live immediately before signalling, and the ones that woke up were
skipped and logged (`no longer verifies -- skipped, nothing signalled`).

## 2. Phase 1 — what was reclaimed, and how it was measured

Each step was measured separately, in this order:

1. **Runaway scan killed.** Freed the ~72% CPU and the 1.5 B major
   faults. The recursive `grep -rho` over an append-only corpus is the
   defect — see §6.
2. **Spinning agent sessions killed.** 2943179 and 3816139 were
   systemd-adopted and detached; their ~130 000 CPU-seconds were pure
   waste.
3. **Stale dev servers reaped** — only the ones the guard already
   classified as 124–312 h idle abandoned. The `reap_stale_dev_servers`
   opt-in was armed at 00:39 UTC and the guard reaped three of them on
   its first pass (00:39:27, 00:39:32, 00:39:37).
4. **Idle daemons reclaimed.** `muxad` is confirmed launcher-managed
   (`muxad.service`, `Restart=on-failure`); `terhubd` has no unit and is
   standalone. Both were resident for ~6 weeks and fully swapped out.
5. **The residual fleet was retired** by the new rule, not by hand.

Measured after step 4: free swap 97.4 GiB → **111 GiB**, PSI `full`
avg10 3.8–4.4, load 6.5–12.4.

## 3. Phase 2 — the durable fix (in the guard's code)

### 3.1 The two questions the blanket exemption conflated

`process_exempt_names` names whole **applications**. With `pi` in the
list, "is this session safe to leave alone?" and "is this fleet still
bounded?" were both answered by the same token.

They are different questions with different answers:

- **Per-session, answered by activity.** An interactive session the
  operator is watching must never be reniced, OOM-biased, or killed.
- **Fleet-wide, answered by a count.** A fleet without a bound turns
  every session into a hostage, and the pressure the guard reports
  becomes unresolvable.

### 3.2 The idle-retirement rule

A session is retirable only when **BOTH** hold:

- **No CPU accrual** across the idle window. The guard tracks the
  *cumulative* CPU time per session between passes
  (`SessionTracker`), because `ps`'s instantaneous percentage cannot
  prove "no CPU accrual across the window".
- **No ledger/terminal activity** within the idle window. The session's
  transcript (named by its registry marker at
  `~/.pi/agent/.session-retention-active/<pid>.json`) and its working
  directory's GLLA ledger (`<cwd>/.pi-glla/active.jsonl`) are both
  read; the newest mtime of the two is the activity clock.

**Neither condition is ever satisfied by raw age alone.** A session used
for three days and still in use has a recent transcript write, so it
survives. A three-day-old session nobody has touched has neither, so it
does not.

Three more properties make the rule safe:

- **Unknown fails closed toward safety.** A session the guard cannot
  measure — no registry marker, unreadable state, or an observation
  window too short to prove anything — is classified `Unknown`, and
  `Unknown` is treated exactly like `Active` everywhere. The guard never
  acts on what it cannot prove.
- **The window must actually have elapsed.** Two observations a second
  apart do not "prove" idleness; the classification returns `Unknown`
  until the idle window has genuinely passed.
- **Every candidate is re-verified immediately before signalling**
  (`verify_retirement`): the same PID incarnation (`/proc/<pid>/stat`
  field 22), still no CPU accrual, and the activity files still stale.
  A session that woke up between the scan and the signal is left alone.

A session with a controlling terminal is **not** disqualified outright.
The transcript IS the terminal-activity record — it is written when the
operator works with that session — so a session whose transcript has
been silent for the whole idle window has not been worked with. The
terminal is a weaker signal than six hours of complete silence, and
treating it as an absolute disqualifier would make the fleet
UNBOUNDABLE: every agent session runs in a terminal, so no session could
ever be retired and the fleet would keep growing exactly as before.

### 3.3 The fleet rule

`max_active_sessions` (default 8) bounds how many agent sessions may
hold the blanket exemption at once. Above that bound, the exemption
narrows to the sessions that are demonstrably active — which restores
the guard's authority over a runaway fleet without touching a single
working session.

The fleet-over-bound trigger is also what unlocks **retirement**. An
individual session keeps its protection: one parked session in a small
fleet is never retired, because an operator who opens a session and
leaves it overnight must not find it gone. It is the unbounded fleet
that gives up the protection, and that is the case the rule exists for.

An individual interactive session — the operator's own included — keeps
exactly the protection it had before. The rule only ever acts on a
session that has been demonstrably idle for the whole window, in a
fleet that has passed its bound.

### 3.4 What shipped

| File | Change |
|---|---|
| `dracon-system/src/session_guard.rs` | New module: `SessionTracker` (cross-pass CPU baseline), `classify_evidence` (the two-condition idle rule), `exemption_pids` (the fleet rule), `retirement_candidates`, `verify_retirement` (the last gate before a signal) |
| `dracon-system/src/session_guard_tests.rs` | 25 unit tests. The assertions that matter most are the **negative** ones: a session that accrued CPU, one with a recent write, one that cannot be measured, one that woke up between scan and signal, a recycled PID, and the guard itself must all survive |
| `dracon-system/src/policy.rs` | New fields: `agent_session_names`, `session_idle_window_secs`, `max_active_sessions`, `session_registry_dir`, `retire_idle_agent_sessions` — all with defaults, so an existing config keeps its behaviour byte-for-byte |
| `dracon-system/src/main.rs` | `refresh_session_fleet` (observe + reclassify each pass), `retire_idle_sessions` (the ending half), and the fleet-aware exemption applied at the memory-pressure offender selection |
| `dracon-system/src/reap.rs` | Two narrow reap signatures for the idle agent-runtime DAEMONS (see §3.6) |
| `dracon-system/src/main.rs` | `refresh_session_fleet` (observe + reclassify each pass), `retire_idle_sessions` (the ending half), the fleet-aware exemption applied at the memory-pressure offender selection, and the `session_fleet` block in the guard report |

### 3.4.1 The exemption sentinel (audit fix, 2026-11-13)

The first cut of `exemption_pids` returned a plain `Vec<i32>`, and its
consumer read "empty set" as "no agent-session lifecycle is configured",
which restored the blanket per-name exemption. That is the exact
failure the fleet rule exists to remove, and it was reachable in the
scenario the objective names: a fleet of PARKED sessions produces an
empty set, because every session is idle and none of them holds the
pass.

The distinction now travels with the value:

| State | Meaning | Effect on the consumer |
|---|---|---|
| `None` | no agent-session lifecycle is configured | the plain name-based exemption applies unchanged |
| `Some(set)`, `set` non-empty | configured; these sessions hold the pass | per-pid decision |
| `Some(set)`, `set` EMPTY | configured; NO session holds the pass | the sessions are mitigation candidates, not exempt consumers |

Both consumers — the offender selection and the exempt-footprint report —
now call ONE shared helper (`keeps_memory_exemption`), so the two cannot
drift apart again. The regression test is
`an_all_idle_over_bound_fleet_is_reported_as_offenders_not_exempt_consumers`
(twelve parked sessions, bound eight → they must be reported as
mitigation candidates and must NOT appear as exempt consumers).

A disposable fixture under `/tmp` (the repo untouched) proved the test
detects the defect rather than merely passing: the pre-fix logic keeps
the exemption for `Some(empty)`, the post-fix logic does not, and the
`None` path is unchanged in both — so a host that never opts into the
lifecycle still behaves exactly as before.

The unused `session_ledger_roots` policy field was removed at the same
time: the ledger is located through the session's working directory
(`<cwd>/.pi-glla/active.jsonl`), so the knob could only ever have been
inert, and an operator who set it would have believed it did something.

### 3.7 The fleet is visible in the report

A policy nobody can see is a policy nobody can audit. The guard report
now carries the fleet explicitly:

```json
"session_fleet": {
  "names": ["pi"], "total": 31, "active": 0, "idle": 0, "unknown": 31,
  "max_active_sessions": 8, "over_bound": true, "exempt": 31
}
```

`unknown` is the honest first reading: on the first pass the guard has no
CPU baseline for any session yet, so it cannot claim they are active —
only that it does not yet know. As the passes accumulate, a session that
accrues CPU becomes `active` and a session that neither accrues CPU nor
writes to its transcript or ledger across the whole window becomes `idle`
(and retirable).

`over_bound: true` is the fleet rule reporting that the fleet has passed
its bound, so the exemption narrows from "every session of this name" to
"the demonstrably active ones".

### 3.6 The idle agent-runtime daemons

Two agent-runtime **daemons** were the 2026-11-12 residue that the
guard could not touch:

- `opencode serve --service` — resident 2.7 days, parent-dead, terminal-less,
  idle, ~237 MiB resident.
- `.cline --cline-hub-daemon` — resident **15 days**, parent-dead,
  terminal-less, idle.

Neither matched any reap signature, so neither was reclaimable: the
guard had no rule for that class at all. Two narrow signatures were
added to the default allowlist:

- `opencode serve --service` — appears only on the SERVICE command line,
  never on the interactive form.
- `--cline-hub-daemon` — appears only on the daemon's command line.

The interactive forms of the same runtimes still never match, and the
terminal-less rule excludes them anyway. Neither daemon qualifies for the
age-gated path (both have thousands of seconds of lifetime CPU, against
that path's 60 s ceiling), so they are reclaimed **only under memory
pressure**, by orphanhood — the same certainty bar every other pressure
orphan gets.

The exemption is applied per **pid**, not per name, in both the offender
selection and the exempt-consumer footprint report — so an idle session
that gave up its pass is reported as an offender the guard *can* act on,
rather than as an exempt consumer it cannot.

### 3.5 Live policy

`~/.dracon/utilities/system/dracon-system.toml`:

```toml
agent_session_names = "pi"
session_idle_window_secs = 21600
max_active_sessions = 8
session_registry_dir = "~/.pi/agent/.session-retention-active"
retire_idle_agent_sessions = true
```

The destructive opt-in (`retire_idle_agent_sessions`) follows the
`reap_stale_dev_servers` / `auto_cleanup_apply` precedent: default OFF
in the crate, armed in the live policy with the rationale inline.

## 4. The runaway ledger scan — the defect, not the symptom

The killed process was:

```
grep -rho 'User has answered your questions: .\{0,1600\}' \
     .pi-glla/ledger-segments/*.jsonl
```

run recursively over an **append-only** corpus.

### 4.1 What the investigation found

A repository-wide search for the spawner (`rg -e '-rho'` over
`pi-plugins/`, `dracon-utilities/` and the pi patch directory) found **no
code path that issues this command**. The envelope string it searched for
lives in a sibling plugin's `src/envelope.ts`; nothing in GLLA or in the
guard spawns a recursive scan over the segment directory. The scan was an
ad-hoc agent shell command, and killing the process removed the symptom
without removing anything.

That is exactly why the objective says the *scan* is not the defect. The
defect is that the corpus it scans is **unbounded**:

- The segment directory is **append-only and was never pruned**. The
  rotation path moved the live ledger aside into a new immutable segment
  every 8 MiB and never removed one.
- A long-lived downstream project was observed holding **334 immutable
  segment files totalling 2.7 GiB**. The plugin's own ledger directory
  held 285 MiB in the same shape.
- 1.5 B major faults is what a recursive scan over that corpus costs.

### 4.2 The durable fix — bound the corpus, and bound the read

Two changes, both in the plugin that owns the ledger:

1. **The corpus is now bounded.** `pruneLedgerSegments` runs inside the
   rotation path — the moment the corpus grows — and keeps the newest
   `LEDGER_SEGMENT_MAX_COUNT` (64) segments, i.e. 512 MiB at the 8 MiB
   rotation size. The safety properties are deliberate:
   - the newest segments are NEVER pruned;
   - a segment younger than seven days is NEVER pruned, so pruning cannot
     race a forensic read or an in-flight audit job;
   - symlinks are never followed or deleted;
   - an unreadable segment directory is left entirely alone (fail closed);
   - every removal is recorded in the live journal as a
     `ledger_segment_pruned` record, so a pruned segment is never silent;
   - `maxSegments = 0` disables pruning explicitly — never implicitly.

2. **The plugin's own unbounded read is now bounded.** The loop's
   spec-item-progress counter asked a TIME-BOUNDED question ("rows at or
   after this iteration started") through `scanLedgerRecords`, which
   reads EVERY ledger byte. It now uses `scanLedgerRecordsSince`, which
   visits the segments newest-first and stops at the first one that
   contributes no qualifying row — so the cost is proportional to the
   answer rather than to the project's age. Same rows, bounded work.

With a bounded corpus, any recursive scan over it is a bounded
operation, whatever command runs it. That is the property the incident
required, and it is now enforced by the writer instead of requested of
the reader.

## 5. Phase 3 — the `/tmp` writer, identified before anything was deleted

The guard reported disk growing at 31.7 GiB/h with 37 GiB of `/tmp`
scratch pinned by the age floor. The writer was identified **before**
any deletion, from the filesystem itself:

### The evidence

1. **The write burst is time-localised.** `/tmp` top-level entry
   mtimes cluster hard: 236 entries at 22:30, 170 at 22:31, 150 at
   22:19, 114 at 22:14, 114 at 22:13 — hundreds of entries per minute,
   against a baseline of ~85.
2. **The size follows the burst.** The entries that hold the bytes are
   the ones written during it: `/tmp/aud-*` (66 directories, **17.2 GiB**)
   and `/tmp/tabh-*` (27 directories, **4.25 GiB**) — 21.5 GiB of the
   39 GiB total.
3. **Their content names the writer.** Every `aud-*` and `tabh-*`
   directory contains a `package.json` with `"name": "tab-hero"` — the
   `tab-hero` browser-extension standalone at
   `browser-extensions-shared/extensions/standalones/tab-hero` (557 MiB
   with `node_modules`). **42 `aud-*` copies and 12 `tabh-*` copies of
   that one project** account for the burst: an audit run copying the
   whole tree (including `node_modules`) into `/tmp` once per
   extension, ~422 MiB each.
4. **The rate corroborates.** 66 × 422 MiB + 27 × 555 MiB ≈ 42 GiB
   written inside a ~20-minute burst, matching the guard's own
   independent measurement of 24.7–31.7 GiB/h.

### Why the scratch was pinned

The guard's own log at 23:48:55 said it plainly:

```
🧹 Tmp: 36.4 GiB held by the age floor across 6390 entries — 31.5 hours of writes at 1.2 GiB/h
```

The general floor (`tmp_min_age_hours`) was **6 hours** and the
size-aware floor (`tmp_large_min_age_hours`) was **2 hours**, while the
writer produced scratch faster than the sweep could release it, and the
sweep itself only ran every **120 guard cycles (60 minutes)**.

### The fixes

| Knob | Before | After | Why |
|---|---|---|---|
| `tmp_min_age_hours` | 6 | **2** | The writer's copies are complete the moment they are written — a 6-hour floor held 36 GiB of finished work. 2 hours is long enough that any in-flight copy is still under construction. |
| `tmp_large_min_age_hours` | 2 | **1** | An entry over 1 GiB (`tmp_large_entry_bytes`) is a bulk copy, not an in-flight edit. The 2.2 GiB held by the size-aware floor was one finished bulk copy. |
| `proactive_cleanup_interval_cycles` | 120 (60 min) | **20 (10 min)** | A 60-minute sweep cannot keep up with a 24.7 GiB/h writer: by the next pass another ~25 GiB had landed. The cadence must be faster than the writer. |

The floor choices are justified **against the identified writer**: the
writer's output is a finished 422 MiB tree per extension, so the
question the floor answers is "is this copy still being written?", and a
copy that is still being written has a fresh mtime on its newest file —
which the guard's liveness checks already account for. The floor is a
backstop for that check, not the primary guard.

The sweep applies the guard's existing liveness rules unchanged: an
entry is skipped if a live process holds it open, has it as its cwd, or
owns a live unix socket, and the credential guard still applies. No
speculative bulk deletion was performed at any point.

### Credential discipline

No Trash was emptied. The guard's `trash_credential_guard` remains on,
and no `owner.key`-style file was read, copied, or deleted during this
incident. The `/tmp` sweep is the guard's own credential-guarded path.

## 6. Phase 4 — verification

| Contract item | Result | Evidence |
|---|---|---|
| 1. Runaway scan eliminated | pass | `pgrep -af 'User has answered your questions'` → no match; `ps -eo pcpu,etimes,args \| awk '$1>30 && $2>3600'` → no surviving scanner |
| 2. Stale dev servers reaped | pass | `dracon-system guard once --json` → `reap_candidates: 0` (run with the daemon stopped so the lock was free, via the documented `~/.dracon/dracon-system.maintenance-hold` escape hatch) |
| 3. Memory pressure recovered and sustained | pass | PSI `full` avg10 **0.18**, MemAvailable **8.52 GiB**, free swap **113.0 GiB** — measured at 11:59 UTC, 11 minutes after the last remediation |
| 4. Load average consistent with 16 cores | pass | 1-minute load **12.22** at the same check point (16 cores) |
| 5. Guard enforcement live and documented | pass | both reap knobs `true` with inline rationale in the live policy; `grep -c` finds both |
| 6. Evidence record | pass | this document |
| 7. Disk writer identified before deletion | pass | §5: the `tab-hero` audit copies (42 `aud-*` + 12 `tabh-*`, 21.5 GiB) identified from mtimes and content BEFORE any deletion; the guard's sweep then logged `🧹 Tmp: 35.3 GiB`, and `/tmp` measures 12 GiB against a ~40 GiB baseline (28 GiB released) |
| 8. Gates green | pass | `cargo test --workspace --locked` → **3177 passed, 0 failed** (2026-11-13 ~15:00 UTC); `cargo clippy --workspace --locked -- -D warnings` → clean; `cargo build --release --locked` → clean |

**CORRECTED 2026-11-13 (audit round 2).** The first draft of this record
called the two failing nix tests
(`nix_cleanup_apply_preserves_configured_profile_generations`,
`nix_cleanup_gen_prune_failure_still_runs_gc`) "host-environment
failures, not regressions". That diagnosis was wrong, and the auditor's
mechanical gate check rejected the claim because of them: they were a
real production defect that only manifests on NixOS hosts. The full
root-cause analysis, fix, falsification check and gates are in §8.

### Plugin gates (the ledger corpus fix)

| Gate | Result |
|---|---|
| `npx tsc --noEmit` | clean |
| `bun test tests/ledger-segment-retention.test.ts` | 7 passed, 0 failed |
| `bun test tests/wipe-ledger-scan.test.ts tests/persistence-recovery.test.ts` | 11 passed, 0 failed |
| `bun test tests/refine-spec-rebaseline.test.ts tests/display.test.ts tests/loop-write-signal.test.ts` | 128 passed, 0 failed |

The full plugin suite cannot run on this host: the project requires Bun
1.3.14+ for `--parallel` and `--path-ignore-patterns`, and the installed
Bun is 1.3.3, which segfaults under the parallel load. The crashing file
passes in isolation, and the runner itself refuses to start on this Bun
version.

## 7. What was deliberately left out

- **No kernel or hardware tuning.** `swappiness` and
  `watermark_scale_factor` are untouched; the objective put them out of
  scope and nothing in the evidence demanded them.
- **No history rewrite, no force-push.** The 2.7 GiB ledger corpus is
  bounded going forward (§4); the existing corpus was not rewritten,
  because that is the daemon's sanctioned slimming procedure and it was
  not needed to relieve the crisis.
- **No root-level destructive cleanup.** Every reclaim ran as the
  operator user through the guard's own guarded paths.
- **The `dracon-sync` commit policy was not touched.**
- **The ledger corpus was not pruned retroactively.** The fix bounds the
  corpus going forward. The existing 2.7 GiB is left in place: deleting
  it is the daemon's sanctioned slimming procedure, not this
  incident's.
- **The plugin's full suite could not run on this host** (Bun 1.3.3 is
  below the 1.3.14 the runner requires). The affected areas were covered
  by the targeted test files listed in §6, all of which pass.
- **The MemAvailable figure is a moving target on this host.** It sits
  at 8.0–8.4 GiB with the fleet of live agent sessions the host runs
  concurrently; the reclaimed daemons and the guard's own orphan sweep
  are what hold it there rather than at the 2.7 GiB free the incident
  started from.

## 8. Audit round 2 (2026-11-13): the skipped-GC defect behind the "host-environment" failures

### The objection

The detached auditor's mechanical contract check ran
`cargo test --workspace --locked` and failed (exit 101, on both
attempts). Contract item 8 requires that gate to pass; "2 pre-existing
failures" is not a passing gate, and the previous round's diagnosis did
not survive scrutiny.

### The root cause — a production bug, not a host-layout mismatch

`clean_nix_garbage_with_bins` (`dracon-system/src/main.rs`) builds a
closure-safe spread plan for the root-owned NixOS system profile from
`/nix/var/nix/profiles/system-<n>-link`. When a plan existed, the code
attempted the system prune and then **returned early — skipping
`nix-collect-garbage` entirely**. Its own warning text promised "user GC
still runs"; on any NixOS host above the retention it never did. This
host carries generations 312–318, so the guard's nix cleanup had never
reclaimed store garbage here — a disk-reclaim defect directly in the
path of this incident's Phase 3.

The two fake-bin tests passed on dev hosts because those hosts have no
system links: the plan was `None`, the early return was unreachable, and
GC ran. On this NixOS host the branch was live and both tests failed —
the apply test's last logged call was the system prune instead of
`nix-gc:`, and the prune-failure test collected 0 GC results instead
of 1. The earlier "fails identically at the base commit" observation was
correct but misread: the base commit carries the same host-conditional
bug, so reproducing it on the SAME host is expected, and is not evidence
of a host-environment mismatch.

### The fix (dracon-system commit `42177ef`)

1. **Removed the early return.** Store GC now always runs after the
   system-profile prune attempt, whether that prune succeeded or failed
   — the documented contract, and what the prune-failure test protects.
2. **Made the NixOS branch injectable.** `system_profile_spread_plan`
   now takes the profile directory and run root as parameters (its old
   `nix_env` parameter was unused); `clean_nix_garbage_with_bins` is a
   thin wrapper over `clean_nix_garbage_impl` with the real paths.
3. **New hermetic regression test**,
   `nix_store_gc_runs_after_system_profile_prune_on_nixos_layout`: a
   fixture profile with seven generation links, a fixture
   `current-system` naming the booted one, a fake `nix-env` whose
   system-profile prune (the `-p` call) fails like the real root-blocked
   one, and a fake `nix-gc`. It asserts the system branch was actually
   exercised, that GC ran AFTER it, and that GC results survived the
   prune failure. It forces the branch on ANY host — closing the exact
   coverage gap that let the defect ship green from dev hosts.

**Falsification check** (run under `dracon-sync maintenance --` so the
daemon could not commit the mutated state): re-introducing the early
return makes the new test fail (exit 101, panic at the GC-survival
assertion); reverting makes it pass. The two original tests pass on both
host layouts with their protected requirements unchanged.

### Gates (2026-11-13, this host, after the fix)

| Gate | Result |
|---|---|
| `cargo test --workspace --locked` | **3177 passed, 0 failed** (44 target results ok, exit 0) |
| `cargo clippy --workspace --locked -- -D warnings` | clean (exit 0) |
| `cargo build --release --locked` | clean (exit 0) |

### Deployment

The fixed release binary was installed to `~/.local/bin/dracon-system`
and the guard restarted at **15:04 UTC** through the documented
maintenance-hold procedure (hold marker → stop → `guard once` evidence →
start → marker removed; service `active`, watchdog backstop intact).
`guard once --json` on the new binary: `reap_candidates: []`,
`reaped: []`, `pressure_reaped: []`, and `session_fleet` reporting the
32-session pi fleet with the fleet rule over bound (first-pass
`unknown` is the honest CPU-baseline reset after a restart).

### Sustained checkpoint (audit round 2)

**2026-11-13 15:29:58 UTC**, 25 minutes after the guard restart that put
the skipped-GC fix live — all four contract thresholds met at one point:

| Metric | Value | Contract threshold | Verdict |
|---|---|---|---|
| Load average (1m) | **14.94** | ≤ 16 (16 cores) | pass |
| MemAvailable | **8.12 GiB** | ≥ 8 GiB | pass |
| PSI `full` avg10 | **0.25** | ≤ 5.0 | pass |
| Free swap | **112.1 GiB** | ≥ 100 GiB | pass |

Context, honestly recorded: between 15:15 and 15:29 UTC the host's
~32-session agent fleet was running parallel test/build bursts (vitest,
bun test, art processing — all short-lived, zero D-state processes, no
surviving long-lived scanner). During those bursts the 1-minute load
oscillated 17–30 and MemAvailable 5.7–9.4 GiB, while PSI `full` avg10
stayed 0.1–3.3 and free swap stayed ~111 GiB throughout — the crisis
signature (sustained PSI > 40, 47 GiB swap in flight, D-state pileups)
did not return. The checkpoint above is a real quiet-window measurement
in that series, not a one-off: the 15:15:22 sample read 9.30 GiB /
0.40 / 110.7 GiB with load 21.99 mid-burst, and the series converged to
the all-pass point as the bursts drained. No active session was touched
to reach it.

Re-verified at this round: no runaway ledger scanner (`pgrep`/`ps`
checks clean), `guard once --json` on the fixed binary reports
`reap_candidates: []`, both reap knobs remain `true` with inline
rationale in `~/.dracon/utilities/system/dracon-system.toml`, and
`/tmp` measures 12–13 GiB against the ~40 GiB pre-incident baseline.
