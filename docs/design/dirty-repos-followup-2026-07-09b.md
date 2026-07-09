# ⚠ DIRTY follow-up audit — 2026-07-09 21:12 (goal `mre05oga-blv70j`)

**Trigger:** operator ran `dracon-sync repos` and pasted the snapshot as the
goal objective for `mre05oga-blv70j`. The snapshot showed 5 WARN + 0 CONCERN
at that moment (junk-runner, deathrun, dracon-platform, plus
captured earlier). After the polis CONCERN + neonbreak WARN fixes from goal
`mrdxdtrf-uj8rqx`, the current fleet view (21:20) shows **22 OK / 4 DIRTY /
0 CONCERN**.

The 4 DIRTY repos are all benign **transient daemon catch-up** states — except
**deathrun**, whose DIRTY state is caused by the SAME growing-budget defect
(`deathrun/.state-recon/`) that was deferred from goals `mrdvbao1` and
`mrdxdtrf-uj8rqx`.

---

## 1. Method

- `dracon-sync repos --json` for a fleet-wide flag census.
- Per-repo `git status --short` + `git rev-list --left-only --count
  $rm/main..HEAD` / `HEAD..$rm/main` for each flagged repo.
- For deathrun: `git ls-files | grep -c '\.png$'`, `du -sh .state-recon`,
  `git log --since "2 hours ago" --oneline | wc -l`, `du -sh
  .git/modules/web-games-deathrun/objects/pack`, and a per-commit PNG-count
  sample to measure the growth trend.
- Verified gitignore coverage for `.state-recon/` (submodule context, same
  lesson as the neonbreak `playwright-report/` fix in goal `mrdxdtrf`).

## 2. The 4 DIRTY repos — all benign transient (will clear on next cycle)

| repo | modified | untracked | real reason | action |
|---|---|---|---|---|
| **dracon-platform** | 6→1 | 0 | `web/.pi/goals/*.md` (goal lifecycle file) + `web/games/docs/LIBRARY-RESEARCH-PHASER-STACK.md` (already committed between checks) + submodule gitlinks (deathrun/endless-td/junk-runner) that the daemon stages | daemon will commit |
| **dracon-utilities** | 0 | 1 | this new goal file `active_goal_2026070922120461_mre05oga-blv70j.md` + 3 nested standalone dirs (`dracon-sync/`, `dracon-system/`, `dracon-warden/`) — all normal | daemon will commit |
| **endless-td** | 2 | 0 | legit game code (`src/lib/phaser/GameScene.ts`, `MenuScene.ts`) — part of the phaser stack work | daemon will commit |
| **deathrun** | 3 | 1 | `.pi/goals/archived/goal_*.md` (closed goal) + 1 PNG in `.state-recon/` modified in last 10 min + still writing audit screenshots | daemon will commit — but the **budget is the real issue** (see §3) |

All 4 have `a=0/b=0` on all 4 remotes — no divergence, pure local dirty
state. Push status `OK`. These will clear on the daemon's next commit pass.

## 3. 🔴 deathrun `.state-recon/` — STILL GROWING, now 3677 PNGs / 723 MiB

### 3.1 Measured trend (3 audits today)

| time | tracked PNGs | on-disk `.state-recon/` | git pack | commits in window |
|---|---|---|---|---|
| 19:58 (goal `mrdvbao1`) | 3 483 | 694 MiB | — | — |
| 21:00 (goal `mrdxdtrf`) | 3 544 | 701 MiB | — | — |
| 21:20 (this goal) | **3 677** | **723 MiB** | **1.2 GiB** | 53 in last 2 h |

Δ in ~2 h since the last audit: **+133 PNGs / +22 MiB**. The git pack is
now **1.2 GiB** and climbing. This is the same defect class as the hegemon
2 GiB incident (`f228b540`) — audit-screenshot dumps being committed into
history because no gitignore rule covers them.

### 3.2 Why it keeps growing

- `deathrun/.gitignore` has **no `.state-recon/` rule** (verified).
- The parent `dracon-platform/.gitignore` has my `playwright-report/` rule
  (added in goal `mrdxdtrf`) but **not** `.state-recon/` — and submodules
  don't inherit parent .gitignore (same lesson as neonbreak's
  `playwright-report/` fix). So the rule must go in deathrun's own
  `.gitignore`.
- The daemon's `git add -A -- <explicit-paths>` commits every new
  `.state-recon/**` PNG because the directory is not ignored.
- A sample of the last 10 commits shows audit dumps at
  `.state-recon/audit-2026-07-09-phaser/`, `.state-recon/audit-2026-07-09-final5/`,
  `.state-recon/audit-2026-07-09-final4/` — each adding 24-34 PNGs.

### 3.3 The fix (operator decision required)

This is the **same recipe as the hegemon 2 GiB rewrite** (`f228b540`), which
the operator explicitly authorized. For deathrun:

1. **gitignore** — add to the USER section of
   `web/games/wip/deathrun/.gitignore` (after `# --- END DRACON MANAGED
   BLOCK ---`):
   ```gitignore
   # --- USER (warden preserves) ---
   # Anti-rebloat: audit / recon screenshot dumps. Same pattern as hegemon's
   # .state-recon/ (goal f228b540). See goal mre05oga-blv70j.
   **/.state-recon/**
   ```
2. **rewrite** — drop the existing trees from history (daemon's own
   `rewrite_ahead_paths` / `auto_repair_concerns` path, or manually):
   ```bash
   cd /home/dracon/Dev/dracon-platform/web/games/wip/deathrun
   git filter-repo --invert-paths --force --path-glob '**/.state-recon/**'
   ```
3. **force-push** to all 3 remotes:
   ```bash
   git push --force origin main
   git push --force github main
   git push --force gitlab main
   git push --force codeberg main
   ```
4. **parent gitlink** — update the dracon-platform submodule gitlink for
   deathrun to the rewritten SHA (daemon `stage_gitlink_updates` will do
   this automatically on next cycle; verify it lands).

⚠ **This rewrites history + force-pushes.** Per the operator's commit
policy (AGENTS.md), force-push / filter-repo requires explicit operator
approval. The hegemon version was approved via a prior `/goal-tweak`;
deathrun has been **deferred** from two prior goals waiting for the same
explicit authorization.

### 3.4 Why not just gitignore and move on?

Because the 3677 PNGs / 723 MiB are **already in history** (1.2 GiB pack).
A gitignore alone stops NEW commits but does NOT reclaim the existing
pack size. Only `filter-repo --invert-paths` shrinks the history. If the
operator prefers to NOT rewrite, the alternative is to leave the bloat
(young repo, 1.2 GiB pack is annoying but not yet GitHub's 2 GiB
server-side per-pack hard limit) and just add the gitignore so it stops
growing.

## 4. Fleet-wide sanity (21:20)

| check | result |
|---|---|
| Daemon status | `active` |
| Orphan `git push` processes | 0 |
| Repos with `AHEAD > 0` on any remote | 0 |
| Repos with `BEHIND > 0` on any remote | 0 |
| Repos with `STUCK_*` | 0 |
| Repos with `CONCERN` | 0 |
| Repos `DIRTY` (transient) | 4 (dracon-platform, dracon-utilities, endless-td, deathrun) |
| Non-cosmetic journal errors | none (deathrun is normal daemon commit traffic) |

## 5. Action checklist

1. **deathrun `.state-recon/`** — EXECUTE the §3.3 recipe (gitignore +
   filter-repo + force-push to all 3 remotes + parent gitlink update),
   **pending operator approval** for the history rewrite + force-push.
   Same authorization as hegemon `f228b540`.
2. **The 4 DIRTY transient repos** — no action; daemon will commit on
   next cycle.
3. **Daemon libgit2-pull bug** — still affects polis-style SCP-SSH pull
   failures (was worked around via `git reset --hard` in goal
   `mrdxdtrf`). Deferred; no new occurrences since the polis fix.

## 6. Verification evidence index

- Fleet DIRTY count: `dracon-sync repos --json` → 22 OK / 4 DIRTY / 0 CONCERN.
- deathrun PNG count: `git ls-files | grep -c '\.png$'` = **3 677** (was
  3 544 at 21:00).
- deathrun on-disk: `du -sh .state-recon` = **723M** (was 701M).
- deathrun git pack: `du -sh
  .git/modules/web-games-deathrun/objects/pack` = **1.2G**.
- deathrun commit rate: `git log --since "2 hours ago" | wc -l` = **53**.
- deathrun still writing: `find .state-recon -name '*.png' -mmin -10 | wc -l`
  = 1 (active at 21:20).
- deathrun gitignore: `grep state-recon deathrun/.gitignore` → no match
  (confirmed absent).
- parent gitignore: has `playwright-report/` (from `mrdxdtrf`) but NOT
  `.state-recon/` (confirmed absent).
- The 4 DIRTY repos: all `a=0/b=0` on origin/github/gitlab/codeberg (no
  divergence, pure local dirty state).
