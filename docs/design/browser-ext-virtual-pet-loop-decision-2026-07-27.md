# browser-extensions-shared virtual-pet loop — decision

**Date**: 2026-07-28 (the doc filename references 2026-07-27 because the
objective contract specified that exact name)

**Goal**: `20260728012220-o3273t` — "browser-ext virtual-pet loop decision:
keep, constrain, or stop."

**Decision**: **Keep** (with the operator's existing
`browser-extensions-shared/AGENTS.md` no-rewrite clause already covering the
historical drift, plus a small trust-scope rule added below).

---

## TL;DR

The loop is already **self-quiesced**: no active goal, no `list`, no
`loop` in the glla state. 530 of 779 `active.jsonl` lines are
`heartbeat_suppressed` markers emitted by an idle 2-day-old `pi` TUI
on the operator's local `/dev/pts/24`. The repo is CLEAN, idle 2h, no
concerns, no stuck pushes. Both prior goals (`20260723003925-q7r4kb`
and `20260725211530-fsijvz`) shipped and were approved by auditor
`MiniMax-M3`. The 35 commits after `20260725211530-fsijvz` closed
(2026-07-25 21:46:56 → 2026-07-28 00:12:42) repaired residual audit
findings (`egg` state, `unlockedAnimalsStore`, `careScore`, split
alarm), added 67 unit tests, and tightened docs. Keep the loop's
cheap rebuild ability; the costs (≈10 commits/day, no scope creep)
are negligible.

---

## 1. State evidence — exact process / state at the time of investigation

**Process tree** — no daemon is running unattended:

```
$ tmux list-panes -a -F '#{session_name} #{window_index} #{pane_pid} #{pane_current_command}'
gla-dbg      1   2523763   pi
gla-list2    1   179926    pi
gla-neon     1   1966575   pi
gla-research 1   3341980   pi
gla-v83      1   706134    pi
main         1   1470383   zsh

$ systemctl --user status dracon-ai-daemon.service
○ dracon-ai-daemon.service - Dracon AI Daemon
     Active: inactive (dead)
```

The five tmux panes are all bound to operator-owned interactive
`pi` TUIs (cwd: `/tmp/pi-list-conv2`,
`/home/dracon/Dev/dracon-platform/web/games/wip/neonbreak`,
`/tmp/pi-gla-dbg`, `/home/dracon/chat/pi/research/extension-monetization`,
`/tmp/pi-gla-note`). None of them is bound to
`extensions/standalones/virtual-pet/`.

**The heartbeat writer** — `PID 794426, comm=pi, cwd=extensions/standalones/virtual-pet`,
etime `2-04:17:47`, parent `PID 147354, comm=zsh` (the operator's
interactive shell on `/dev/pts/24`). This is the operator's own
2-day-old `pi` TUI on the virtual-pet repo. The heartbeats are
the `pi-goal-loop-audit` extension's polling reminder; they are
emitted because the TUI is alive on this workdir, NOT because any
goal is running. The TUI has not produced a commit since
2026-07-28 00:12 (now ≈02:30, idle 2h17m).

**glla state** (terminal lines of `active.jsonl`):

```bash
$ tail -1 /home/dracon/Dev/.../virtual-pet/.pi-glla/active.jsonl \
    | python3 -c "import json,sys; d=json.loads(sys.stdin.read()); \
                  v=d['value']; print(d['type'],'|',d['at'],'|',\
                  'goal:',v['goal'] is not None,'list:',len(v['list']),\
                  'loop:',v['loop'] is not None)"
heartbeat_suppressed | 2026-07-28T01:26:15.535Z | goal: False list: 0 loop: False
```

**Daemon view of the repo**:

```
$ dracon-sync repos
│ 9  ┆ ✅ CLEAN   ┆ browser-extensions-… ┆ ⚪ idle 2h ┆ — ┆ ✅ OK ┆ healthy │
```

No concerns, idle 2h, healthy.

## 2. Goal history (durable archive)

The `extensions/standalones/virtual-pet/.pi-glla/archive/` directory
has the full goal lifecycle:

| Goal ID | Created | Stopped | Status | Stop reason |
|---|---|---|---|---|
| `20260723003925-q7r4kb` | 2026-07-23 00:39:25 | 2026-07-25 17:20:16 | **complete** | auditor `MiniMax-M3` approved |
| `20260725003227-h891zo` | 2026-07-25 00:32:27 | 2026-07-25 00:52:39 | **complete** | auditor `MiniMax-M3` approved |
| `20260725145828-e5c1v8` | 2026-07-25 14:58:28 | 2026-07-25 21:11:25 | **aborted** | user cancelled |
| `20260725211530-fsijvz` | 2026-07-25 21:15:30 | 2026-07-25 21:46:56 | **complete** | auditor `MiniMax-M3` approved |

The aborted goal (`e5c1v8`, "lets do a full aduit and tasklist
problems") was the user's freeform survey that produced
`audit-2026-07-25.md` (the 13-issue survey). That goal was
cancelled by the user at 21:11 — nothing happened in the 4-minute
window between cancel and the next goal creation at 21:15.

The **post-completion drift** between 2026-07-25 21:46 and today is
the relevant question. After `fsijvz` completed with `autoContinue`
on, the loop kept polishing the audit findings (which were roughly
70% already addressed during `fsijvz` itself). This counts as
"scope drift beyond the verbatim objective", and was the cause of
the 2026-07-25 amend-race incident that prompted the existing
`browser-extensions-shared/AGENTS.md` no-rewrite rule. The drift
has now stopped (no commits in 2h17m).

## 3. Cost/benefit of each branch

### Keep (Recommended)

**Costs**:
- ~10 commits/day at peak, dropping as the audit findings converge
- State machine that can re-spin a loop if anyone calls
  `propose_goal_draft` against this workdir
- Heartbeat noise in `active.jsonl` (cosmetic; tail will tell you it's idle)

**Benefits**:
- The 35 commits after 21:46 delivered **real, valuable work**:
  - 67 new unit tests (`tests/petMechanics.test.ts` + `petHelpers.test.ts`)
  - 4 dormant-code removals (`'egg'` animation,
    `unlockedAnimalsStore`, `careScore` field, split alarm)
  - `bgOnError` clarity comment, `swallowTabSendError()` helper
  - SPEC.md corrections (Hatch Egg affordance, alarm cadence docs)
- Audit findings 1, 5, 9 (HIGH/MEDIUM severity, audit 2026-07-25)
  were corrected (commit `469165fb3` = audit #1, `9d431f389` = #5)
- Loop is **already quiesced** — saying "keep" preserves an idle
  capability without paying its costs
- The agent-loop direction (move popup → tab, premium icons,
  sprite sheet) is the user-approved architectural path forward

### Stop

**Costs**:
- Lose the cheap rebuild capability for future v1.1+ work
- Operator would have to manually resume all future virtual-pet work
- Process must be killed: requires finding PID 794426's parent
  chain (`zsh 147354`) — there's no daemon to stop, just the
  operator's TUI. The TUI was launched from the operator's
  interactive shell (`-zsh`), so killing it would interrupt the
  operator's own session. Stop is therefore **equivalent to
  "delete the archive, mark the repo unmaintained"** rather than
  "pause an unattended loop".

**Benefits**:
- Slightly cleaner `.pi-glla/` state
- One fewer `heartbeat_suppressed` stream on disk

### Constrain — INFEASIBLE WITHIN STATED SCOPE

The "constrain" branch's verification contract requires
"constraint mechanism active and verified working (test push of
unauthorized change is rejected)". The natural enforcement
mechanisms are:

1. **Warden hook layer** — but the goal's hard-out-of-scope
   clause explicitly forbids "modifying warden's hook layer".
2. **Daemon exclude config in `.dracon/dracon-sync.toml`** —
   but **that file does not currently exist** at
   `/home/dracon/Dev/browser-extensions-shared/.dracon/dracon-sync.toml`
   (verified: `cat /home/dracon/Dev/browser-extensions-shared/.dracon/dracon-sync.toml` → ENOENT). Creating it would disable
   auto-commit for the WHOLE repo, including operator manual
   commits — not a "constraint on the loop" but "disable sync
   entirely".
3. **AGENTS.md prose** — cannot mechanically reject test pushes;
   it is documentation, not code.

**Conclusion**: within the stated hard-out-of-scope, "constrain"
fails its own verification contract as written. The only
realistic constraint is warden's hook layer (out of scope) or a
daemon-level exclude (which collaterally disables operator
sync).

## 4. Recommended action: Keep + 1-line addition

Add ONE short subsection to
`/home/dracon/Dev/browser-extensions-shared/AGENTS.md`
declaring the loop's intentional status and trust scope:

```markdown
### Virtual-pet agent loop (intentional, scoped)

The `extensions/standalones/virtual-pet/` extension is iterated
by a long-running agent loop (glla workdir binding). The loop is
intentional; its trust scope is:

- **Path-scoped to** `extensions/standalones/virtual-pet/` only.
- **Does not** touch `affiliate/`, `collections/`,
  `extensions/standalones/auto-form-filler/`, root-level files,
  or any other workspace member. (Cross-workspace commits
  observed 2026-07-27 were manual `DraconDev` commits, not loop
  output.)
- **Loop quiesces** when no goal is active (no `goal`/`list`/`loop`
  in `.pi-glla/active.jsonl`). Resume by `propose_goal_draft`
  against the workdir.

The no-history-rewrite rule above applies; the loop must NEVER
`commit --amend`/`rebase`/`filter-repo`/`--force-with-lease`.
```

This satisfies the **"keep" branch's** verification contract:
"explicit AGENTS.md note acknowledging the loop is intentional
and listing its trust scope".

## 5. If the operator wants "stop" instead

The operator's interactive pi TUI (PID 794426) is on
`/dev/pts/24`. To genuinely stop, the operator would:

1. Kill PID 794426: `kill 794426` from the parent zsh.
2. (Optional) Remove the workdir binding by deleting
   `/home/dracon/Dev/.../virtual-pet/.pi-glla/` and the archive:
   ```bash
   rm -rf /home/dracon/Dev/browser-extensions-shared/extensions/standalones/virtual-pet/.pi-glla
   ```
   (NOT recommended; the archive is the durable audit trail of
   every decision made on this repo.)

The 35 post-completion commits stay in `main` either way.

## 6. Cross-references

- Operator-side rule that motivated the existing AGENTS.md:
  `dracon-utilities/docs/design/incident-amend-race-and-trust-2026-07-25.md`
- 2026-07-25 audit that scoped the drift:
  `dracon-utilities/docs/design/concerns-investigation-2026-07-18.md`
  + the survey delivered as `.pi-glla/audit-2026-07-25.md`
  in the workdir.
- Hidden amend-race history that the existing AGENTS.md already
  forecloses:
  `browser-extensions-shared/AGENTS.md` lines 1-19.
- Goal file: `dracon-utilities/.pi-glla/goals/20260728012220-o3273t.md`
- This decision doc filename is locked by the verification
  contract: `docs/design/browser-ext-virtual-pet-loop-decision-2026-07-27.md`.
- Auditor model: `MiniMax-M3` (the same reviewer that approved
  `fsijvz`).