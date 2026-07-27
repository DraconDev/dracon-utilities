# Repo Audit — 2026-07-10

Goal: `mrf8u33y-vyjr5n` — "audit the repos but we still have a problem with polis".

Tool: `dracon-sync repos` (27 watched repos) + direct `git` inspection of
`/home/dracon/Dev/dracon-platform` and its submodules.

## 1. Health snapshot (27 repos)

| State | Initial (this session) | Final (live, end of session) |
|---|---|---|
| ✅ OK | 21 | 20 |
| ⚠️ WARN | 5 | 7 |
| ❌ CONCERN | **1 (polis)** | **0** |
| ⛔ init/status failed | 0 | 0 |

The only CONCERN (`polis`) is resolved (see §2). The 7 WARNs at end-of-session
are **transient dirty/debounce states** — every flagged submodule was verified on
`heads/main` with its gitlink tracked in the parent, and the daemon converges
parent gitlinks to each submodule's `refs/heads/main` in batches. No WARN
represents a real failure (0 init/status failures).

## 2. The `polis` problem — root cause, recurrence, and fix

### 2a. First occurrence (CONCERN)
- `polis` was ❌ CONCERN, **1 ahead / 1 behind**, stuck *"pushing 83m"*.
- Cause: the **same SA-8b edit** to `src/routes/play/+page.svelte`
  (`rgba(0,0,0,0.85)→rgba(26,17,51,0.72)`) was committed twice ~84 min
  apart — locally as `cc4a792` and on the remote as `465db02` (identical
  content). Non-fast-forward → daemon (`force_push_when_behind = false`)
  was stuck.
- Resolution: the daemon's own repair merged `origin/main` into local →
  merge commit `ae15b72` ("Merge gitlab.com:DraconDev/web-games-polis"),
  committed the 2 in-progress files (`c424e25`), and pushed. 0/0, synced.
  CONCERN dropped 1 → 0.

### 2b. Recurrence (the real, persistent problem)
- The operator (DraconDev) then committed a **new** `polis` commit
  `9640736` ("SA-8a — Phaser canvas transparent + CSS scene bg") on local
  `main` **without first pulling** the daemon's merge `ae15b72`.
- Topology: common ancestor `c424e25`. Local `9640736` descends from
  `c424e25`; remote `ae15b72` is a merge of `c424e25` + `465db02`.
  So local and remote diverged from the same base → **1 ahead / 1 behind again**.
- This is the **recurring root cause**: the operator commits directly to
  `polis` local `main` and does not pull the remote's ahead commits (which
  the daemon had pushed as merges). Both sides advance `main` →
  non-fast-forward → daemon (won't force-push when behind) stuck.

### 2c. Fix applied this session (verified live)
- `git merge --no-ff origin/main` in `polis` → **clean merge, no conflicts**
  (local SA-8a touches `IsoCanvas.ts` + `.canvas-wrap` in `+page.svelte`;
  remote SA-8b touches `.end-screen` in `+page.svelte` — different regions).
  Created merge commit **`778f0d2`**.
- `git push origin main` → fast-forward `ae15b72..778f0d2 main -> main`.
- **Live verification:**
  - `local HEAD = origin/main = 778f0d2` (confirmed via `git rev-parse` + `git ls-remote`)
  - `ahead=0 behind=0`, working tree clean
  - pushed to github / gitlab / codeberg
- Parent `dracon-platform` gitlink for `polis` advances `ae15b72 → 778f0d2`
  via daemon convergence (shared `refs/heads/main` = `778f0d2`); verified
  converged.

## 3. Recommendation (prevent recurrence)
The `polis` divergence will recur as long as the operator commits to
`polis` local `main` without first pulling the remote. Fix the workflow:
- **Operator:** before committing to `polis` `main`, run
  `git fetch && git merge origin/main` (or `git pull --rebase origin main`).
  This keeps local `main` containing the remote before adding new work, so
  the daemon can fast-forward-push.
- **Alternative (only if force-push is acceptable for this operator-owned
  repo):** set `force_push_when_behind = true` for `polis` in its
  `.dracon/dracon-sync.toml`. AGENTS.md permits force-push for ≤5 ahead;
  a merge (as applied here) is cleaner and avoids history rewrite.

## 4. Other submodules (all verified healthy)
darklord, neonbreak, capture-anime-girls, hegemon, hellhunter, endless-td,
deathrun, junk-runner, one-mil-girls — all on `heads/main`, gitlinks
tracked in the parent, no CONCERN/init failures. (darklord was previously
fixed this session for a detached-HEAD / 36-commits-behind-`main` anomaly;
now on `main` and converged.)
