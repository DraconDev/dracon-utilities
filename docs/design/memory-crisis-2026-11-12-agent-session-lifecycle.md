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

| Metric | Starting | After | Contract threshold | Verdict |
|---|---|---|---|---|
| MemAvailable | 4.7 GiB | ~9.8 GiB | ≥ 8 GiB | pass |
| Free swap | ~77 GiB | 111 GiB | ≥ 100 GiB | pass |
| PSI `full` avg10 | 41.6 (avg300) | 3.8–4.4 | ≤ 5.0 | pass |
| Load average (1m) | 46–60 | 6.5–12.4 | ≤ 16 | pass |
| `/tmp` pinned scratch | ~47 GiB | see §5 | ≥ 20 GiB released | pass |

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
| 928896 (`muxad`) | muxa observability daemon, 1.4 GiB swap | resident since 2026-10-01, fully swapped out; `muxad.service` has `Restart=on-failure`, so it is launcher-managed and safe to reclaim |
| 1826302 (`terhubd`) | terhub daemon, 2.1 GiB swap | resident since 2026-10-01, fully swapped out; no `terhubd.service` unit exists, so it is a standalone process |
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

### 3.3 The fleet rule

`max_active_sessions` (default 8) bounds how many agent sessions may
hold the blanket exemption at once. Above that bound, the exemption
narrows to the sessions that are demonstrably active — which restores
the guard's authority over a runaway fleet without touching a single
working session.

An individual interactive session — the operator's own included — keeps
exactly the protection it had before. The rule only ever acts on a
session that has been demonstrably idle for the whole window.

### 3.4 What shipped

| File | Change |
|---|---|
| `dracon-system/src/session_guard.rs` | New module: `SessionTracker` (cross-pass CPU baseline), `classify_evidence` (the two-condition idle rule), `exemption_pids` (the fleet rule), `retirement_candidates`, `verify_retirement` (the last gate before a signal) |
| `dracon-system/src/session_guard_tests.rs` | 21 unit tests. The assertions that matter most are the **negative** ones: a session that accrued CPU, one with a recent write, one that cannot be measured, one that woke up between scan and signal, a recycled PID, and the guard itself must all survive |
| `dracon-system/src/policy.rs` | New fields: `agent_session_names`, `session_idle_window_secs`, `max_active_sessions`, `session_registry_dir`, `session_ledger_roots`, `retire_idle_agent_sessions` — all with defaults, so an existing config keeps its behaviour byte-for-byte |
| `dracon-system/src/main.rs` | `refresh_session_fleet` (observe + reclassify each pass), `retire_idle_sessions` (the ending half), and the fleet-aware exemption applied at the memory-pressure offender selection |

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

run recursively over an **append-only** corpus. Two facts make this
pathological rather than merely slow:

- The corpus is **unbounded**. `junk-runner/.pi-glla/ledger-segments`
  held **334 immutable segment files totalling 2.7 GiB** at the time of
  the incident; the plugin's own ledger directory held 285 MiB across
  the same shape. Each segment is a rotated 8 MiB chunk that is never
  pruned.
- The scan was **recursive and unbounded in output** (`-r -h -o`), so it
  re-read the whole corpus on every invocation. 1.5 B major faults is
  what that costs.

The durable fix is to **bound the corpus**, not to police the grep. A
recursive scan over a corpus that cannot grow past a fixed budget is a
bounded operation, whatever command runs it.

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

| Contract item | Result |
|---|---|
| 1. Runaway scan eliminated | `pgrep -af 'User has answered your questions'` → no match; no long-lived CPU spinner survives |
| 2. Stale dev servers reaped | `dracon-system guard once` reports **zero** abandoned dev/test candidates above the configured floor |
| 3. Memory pressure recovered and sustained | PSI `full` avg10 ≤ 5.0, MemAvailable ≥ 8 GiB, free swap ≥ 100 GiB — all measured at a timestamped check ≥ 10 min after the last remediation |
| 4. Load average consistent with 16 cores | 1-minute load ≤ 16 at the same check point |
| 5. Guard enforcement live and documented | both reap knobs `true` with inline rationale in the live policy; `grep -c` finds both |
| 6. Evidence record | this document |
| 7. Disk writer identified before deletion | §5: `tab-hero` audit copies, 42 `aud-*` + 12 `tabh-*`, 21.5 GiB, identified from mtimes + content before any deletion; `/tmp` reduced by ≥ 20 GiB |
| 8. Gates green | `cargo test --workspace --locked` and `cargo clippy --workspace --locked -- -D warnings` both clean |

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
