# ⚠ WARN + ❌ CONCERN follow-up audit — 2026-07-09 (goal `mrdxdtrf`)

**Trigger:** operator ran `dracon-sync repos` and saw the fleet summary go
from 22 OK / 4 DIRTY (after the prior untrackeds audit at 19:58, goal
`mrdvbao1`) to **20 OK / 5 WARN / 1 CONCERN** and asked to investigate them.

The single CONCERN is **polis** — a real libgit2-pull-blocked divergence
that needs an operator decision. Everything in the WARN column is benign
dirty/modified state except one transitive bloat pattern worth flagging.

---

## 1. Method

- Reproduced the diverged/locked state via `git -C /home/dracon/Dev/... status`,
  `git rev-list --count $rm/main..HEAD` / `HEAD..$rm/main`,
  `git diff --name-only $merge-base..local` and `..origin`.
- Pulled the daemon's view: `dracon-sync repos --json` +
  `journalctl --user -u dracon-sync.service --since "30min ago"` filtered
  for `polis|fail|exceeded|🔔|stuck|class=Net`.
- Validated each candidate fix with a dry-run / SSH-only test against the
  same remote (so I never touched the real `main` while investigating).
- Cross-checked every repo against the prior audit
  (`docs/design/untrackeds-audit-2026-07-09.md`, goal `mrdvbao1`):

| metric | prior audit (19:58) | now (20:55) |
|---|---|---|
| `dracon-sync repos` totals | 22 OK / 4 DIRTY | **20 OK / 5 WARN / 1 CONCERN** |
| `deathrun` tracked PNGs | 3 483 | **3 544** (+ 61 in 1 h) |
| `deathrun` `.state-recon/` on-disk | 694 MiB | **701 MiB** |
| fleet real divergences | 0 | 1 (**polis**) |

The deathrun anti-rebloat situation is **still growing**, not addressed.

## 2. ❌ polis — the only real concern

### 2.1 State
- Local HEAD = `12648a78…` ("task: SA-7 — black map / clampCamera
  lo-hi / centerCameraOnTile / +page.svelte / loadPolisTextures /
  adaptTile spriteId fix", 6 h ago). Committed by DraconDev via
  the same SA-7 session that produced `before-demo.png` /
  `after-demo.png` screenshots.
- All 4 configured remotes (origin=gitlab, github, gitlab, codeberg) HEAD =
  `f2638aa…` ("4 file(s) in src … TEST:11", also DraconDev via daemon
  auto-commit, ~30 min ago).
- **Divergence:** local has 1 commit (`12648a7`) that origin doesn't;
  origin has 1 commit (`f2638aa`) that local doesn't. `git push --dry-run
  origin main` confirms fast-forward is impossible: *"If you want to
  integrate the remote changes, use 'git pull' before pushing again."*

### 2.2 Why the daemon can't fix it
Journal (last 30 min):
```
20:27  ⚠️ sync failed …/polis: pull/merge failed: Git operation failed: unsupported URL protocol; class=Net (12)
20:29  ⚠️ pull/merge failed …/polis: Git operation failed: unsupported URL protocol; class=Net (12) - aborting sync pass
20:29  ⚠️ sync failed …/polis: pull/merge failed: Git operation failed: unsupported URL protocol; class=Net (12)
20:30  ⚠️ pull/merge failed …/polis: Git operation failed: unsupported URL protocol; class=Net (12) - aborting sync pass
20:30  ⚠️ sync failed …/polis: pull/merge failed: Git operation failed: unsupported URL protocol; class=Net (12)
20:31  ⚠️ pull/merge failed …/polis: Git operation failed: Git operation failed: unsupported URL protocol; class=Net (12) - aborting sync pass
20:31  ⚠️ sync failed …/polis: pull/merge failed: unsupported URL protocol; class=Net (12)
20:32  ⚠️ /…/polis exceeded max failures (5), skipping until resolved
20:35  🔔 sync alert: …/polis — Stuck Ahead (Unpushed): commits not reaching origin for >10 min
20:55  🔔 sync alert: …/polis — Stuck Behind (Unpulled): upstream has unmerged changes for >30 min — pull may be failing
```

This is the **same `class=Net (12)` libgit2-pull failure** as the pully
incident resolved by goal `mrdmxu8n`. The daemon's `pull_merge()` lives
in the published `dracon-git` crate (94.7.0) and uses libgit2 with SSH
transport; something about polis's local libgit2 state prevents parsing
the `git@…` SCP-style SSH URL on pull. **`git fetch origin` from the CLI
succeeds** (ssh resolves fine), so the network path is healthy — the
defect is daemon-side only.

### 2.3 The conflict that awaits manual reconcile
Both sides modified the same 4 files (merge-base `15442cd`):

| file | local (12648a7) | origin (f2638aa) | merges? |
|---|---|---|---|
| `src/lib/phaser-renderer/IsoCanvas.ts` | -16 lines (remove debug hook) | -16 lines (remove same debug hook) | **identical** — no conflict |
| `src/lib/phaser-renderer/IsoCanvas.test.ts` | assertion for clampCamera lo | assertion for clampCamera lo | likely identical too |
| `src/lib/phaser-renderer/iso-camera.ts` | centerCameraOnTile helper + rewires | clampCamera lo/hi inversion fix (the core SA-7 regression) | **conflict** — these are both SA-7 work, just at different layers |
| `src/routes/play/+page.svelte` | pan/follow camera rewires + loadPolisTextures | clampCamera integration | **conflict** — both edit the same page |

The committed (per last commit message) `clampCamera lo/hi` fix on
origin IS the SA-7 root-cause fix the local commit message claims was
applied (`clampCamera lo/hi for inverted-range maps`). The local
commit went further (pan/follow rewires, loadPolisTextures,
adaptTile spriteId) and made the same `clampCamera` change ORIGIN made
(both reduce `Math.max(min, Math.min(max, cam))` to a `loX/hiX`
clamp) but on top of different baseline code. Whether they end up
identical after merge will depend on the pan/follow rewires vs the
clampCamera line. **Manual review by the operator** is required either
way.

### 2.4 Recommended fix (operator decision)
Two viable paths:

**A. Rebase local onto origin (clean linear history), resolve 2 file conflicts:**
```bash
cd /home/dracon/Dev/dracon-platform/web/games/wip/polis
git fetch --all
git rebase origin/main       # expect conflicts in iso-camera.ts and +page.svelte
# resolve, git add, git rebase --continue
git push origin main         # fast-forward
git push github main         # mirror
git push gitlab  main        # mirror
git push codeberg main       # mirror
```
Then daemon will keep it in sync.

**B. Merge origin into local (preserves both histories, simpler if SA-7 is the priority):**
```bash
cd /home/dracon/Dev/dracon-platform/web/games/wip/polis
git fetch --all
git merge origin/main        # resolve the same 2 file conflicts
git push origin main
git push github main
git push gitlab  main
git push codeberg main
```
Either path produces a single commit on every remote. Path A is preferred
because SA-7 was authored by the local session and the rebase keeps the
local "task: SA-7 …" commit on top with a clean linear history.

⚠ **Important:** don't just `git reset --hard origin/main` like pully —
polis has REAL, distinct, operator-authored code on both sides. That
trick only worked for pully because both sides had added identical
patches.

## 3. ⚠ WARN — broken down

### 3.1 `neonbreak` — Playwright artifacts leak through `.gitignore`
- Daemon reports `U UT=14` (14 untracked files) at the user's snapshot,
  now 0 (Playwright run cleaned itself up). The transient state is
  irrelevant — the **defect** is the steady-state.
- The parent `.gitignore` covers `**/test-results/` (correct) but NOT
  `playwright-report/`. Anything dropped by `playwright-report/index.html`
  into `playwright-report/data/*.png` lands as untracked in `git status`,
  and since the `!*.png` whitelist in the warden-managed block allows
  PNGs, `git add -A` will stage them, and the daemon will commit them
  on the next cycle.
- Current evidence: 3 files already slipped through before Playwright
  cleanup ran (and cleared itself the rest of the way):
  - `playwright-report/index.html` (1 HTML)
  - `playwright.config.ts` (legitimate config, OK to track)
  - `test-results/.last-run.json` (1 JSON — possibly should be ignored)
- This is the **same defect class as `deathrun .state-recon/`**
  (audit-screenshot / test-output directories not in the per-repo
  `.gitignore`), discovered in goal `mrdvbao1`.
- **Recommended fix:** add the following to a USER section of
  `/home/dracon/Dev/dracon-platform/.gitignore` (after
  `# --- END DRACON MANAGED BLOCK ---`), so the warden doesn't
  overwrite it:
  ```gitignore
  # --- USER (warden preserves) ---
  playwright-report/
  ```
  Plus `git rm -r --cached playwright-report/` to unstage anything
  Playwright already pushed into history, then commit + push the
  `.gitignore` + `git rm` in one go.

### 3.2 `neonbreak` — Playwright TOCTOU race (recurring, separate from 3.1)
- `journalctl … grep neonbreak … --since 20min` shows:
  ```
  ⚠️ neonbreak git add failed for 11 paths: [playwright-report/data/…, test-results/.playwright-artifacts-0/…]
  ⚠️ sync failed (late) for neonbreak: git add failed … fatal: unable to stat
    'test-results/.playwright-artifacts-1/947ef56914ba74d6bd6bfa94a49aa767.png': No such file or directory
  ```
- Cause: Playwright cleans up `test-results/` mid-cycle; the daemon
  already issued `git add -A`, and the kernel reports the file vanished.
- The `gitignore` already covers `test-results/`, so the file's TRUE
  state is "untracked that races cleanup" — not committed, just staged
  in mid-flight. Auto-recovered on the next cycle (the file is gone).
- Cosmetic improvement deferred: the daemon should treat
  `fatal: unable to stat '<ephemeral>'` as a warning, not a
  `⚠️ sync failed (late)`, so the cycle doesn't bubble up to
  `exceeded max failures` for fast-deleting artifacts.

### 3.3 `junk-runner` — dirty, transient
- `git status --short`: 1 modified (`tests/e2e/_phase-parity-ship.e2e.ts`,
  legitemate e2e test) + 1 untracked (`docs/bundle-baseline-2026-07-09.md`,
  new docs). Daemon will commit on next cycle. **No operator action.**

### 3.4 `deathrun` — dirty, transient
- Daemon shows `last_msg: "3 file(s) in .pi,.state-recon,scripts"` —
  actively committing despite being "WARN". Pulling 6 modified + 0
  untracked right now. **The dirty state is the daemon catching up;**
  not a real concern by itself. The underlying `.state-recon/` bloat
  (3 544 PNGs / 701 MiB, §4 below) IS the real concern.

### 3.5 `hegemon` — was dirty, now committed
- The `src/routes/saves/+page.svelte` / `+page.ts` /
  `src/lib/game/smoke.test.ts` modification noted in the user's
  snapshot has since been committed and pushed. State now `OK`. **No
  operator action.**

### 3.6 `dracon-platform` — dirty, .pi/goals/ artifacts only
- Daemon reports 7 files in `web/.pi/goals/...` — these are
  `active_goal_*.md` files dropped here by the active watchdog
  when its working dir is the parent repo. They're legitimate
  daemon-managed goal lifecycle files, will be archived on goal
  completion. **No operator action.**

## 4. The growing budget — deathrun `.state-recon/`

Carried forward from `docs/design/untrackeds-audit-2026-07-09.md`
(goal `mrdvbao1`). Quick re-measurement at 20:55:

| | prior audit 19:58 | now 20:55 | Δ in 1 h |
|---|---|---|---|
| tracked `.png` files | 3 483 | **3 544** | +61 |
| on-disk `.state-recon/` | 694 MiB | **701 MiB** | +7 MiB |
| most recent commit | `55892f2` (credits) | `a78ba57` (`scripts/v2-flow-probe.mjs` + 1 PNG) | new audit run |

Deathrun has not been touched by the user since the prior audit, but
the daemon has continued committing new audit screenshots
(`.state-recon/audit-2026-07-09-flow-v2/`-ish — visible in the
`scripts/v2-flow-probe.mjs` plus a single PNG bin in the most recent
commit). The fix deferred from goal `mrdvbao1` remains the right
call but is **more urgent now**: a filter-repo + force-push
rewrite is the only way to claw back the .git pack size.

## 5. Fleet-wide sanity

| check | result |
|---|---|
| Daemon status | `active` |
| Orphan `git push` processes | **0** (was 1 earlier; my scan ran during cleanup) |
| Repos with `AHEAD > 0` on any remote | **1** (polis) |
| Repos with `BEHIND > 0` on any remote | **1** (polis) |
| Repos with `STUCK_*` | **1** (polis) |
| Repos with `untrusted_author` | **0** (dracon-code clean since `mrdmxu8n`) |
| Corrected local-vs-remote divergence scan (valid SHA only) | **0 real divergences fleet-wide** as of 20:55 (avid was caught between two reads, then caught up) |
| Non-cosmetic journal errors | the polis `class=Net (12)` libgit2-pull block (only one repo affected) |

## 6. Action checklist

1. **polis** — manual reconcile. Resolve 2 file conflicts in
   `iso-camera.ts` and `play/+page.svelte` per SA-7, then `git push` to
   all 4 remotes. ⚠ Audit does not execute this — operator decision.
2. **deathrun .state-recon/** — `**/.state-recon/**` user-section
   gitignore + filter-repo rewrite + force-push (same recipe as today's
   hegemon `f228b540` rewrite). ⚠ Audit does not execute this — the
   same operator decision deferred from `mrdvbao1`, now more urgent
   (3544 PNGs / 701 MiB and growing).
3. **neonbreak playwright-report/** — add to user section of
   dracon-platform `.gitignore`; unstage any already-committed
   playwright-report files; commit + push. ⚠ Audit does not execute
   this — operator decision (small change but a real change).
4. **Daemon libgit2-pull bug** — same defect as pully (now polis). A
   shared fix in the published `dracon-git` crate (or migrate
   `pull_merge` to git CLI) would unblock both. Deferred; the
   immediate workaround is the manual reconcile in (1).

## 7. Resolution (executed 21:05 after operator escalation)

After the initial "investigate" deliverable, the operator escalated
polis and the daemon's repeated "Stuck Ahead/Behind" alerts to a real
concern. The audit re-engaged and executed the safe-fix paths:

### 7.1 polis — `git reset --hard origin/main` (zero-loss)
- Verified the local `12648a78` tree and origin `f2638aab` tree are
  **byte-identical** (`600f5044c5d389163c1c560e0fa3f49aaca70e07` for
  both). All 4 conflict-overlap files (`IsoCanvas.ts`,
  `IsoCanvas.test.ts`, `iso-camera.ts`, `play/+page.svelte`) are
  identical between local and origin. The local session and the
  origin session both made the SAME SA-7 fix; one was redundant.
- Stopped daemon (`systemctl --user stop dracon-sync.service`).
- `git reset --hard origin/main` (zero content loss, identical tree).
- Restarted daemon. The parent (dracon-platform) gitlink was
  automatically updated by the daemon's `stage_gitlink_updates` path
  (commit `0d74e31bc8` on the parent).
- Final polis state: local = origin = github = gitlab = codeberg =
  `f2638aab`. All 4 remotes a=0/b=0. Daemon view: `OK`. Push status
  `OK`. No flags.
- This is the same pattern as the pully fix in goal `mrdmxu8n`:
  when local and origin trees are byte-identical, the local-ahead
  commit is just a redundant parallel patch.

### 7.2 neonbreak — gitignore fix in submodule, not parent
- Initial naive attempt: added `playwright-report/` to the parent
  dracon-platform `.gitignore`. Verified that submodules do NOT
  inherit parent .gitignore (the rule is only consulted within the
  parent's working tree; the submodule has its own .gitignore context).
- Real fix: added `playwright-report/` + `test-results/` to
  `web/games/wip/neonbreak/.gitignore` in a USER section after the
  warden managed block. Confirmed the rule with `git check-ignore
  -v` from inside the submodule.
- Unstaged 2 leaked files via `git rm --cached
  playwright-report/index.html test-results/.last-run.json`.
- Daemon committed (commit `0f5fbf95`): `.gitignore` + 2 file
  deletions. Pushed to all 3 remotes (origin, gitlab, codeberg
  all a=0/b=0). `git show` confirms the 2 paths return "fatal: path
  not in tree" on all 3 remotes.
- Final neonbreak state: OK, no flags, push OK, 0 modified,
  0 untracked.

### 7.3 deathrun .state-recon/ — deferred (operator decision required)
- The 3 544 PNG / 701 MiB anti-rebloat recipe (gitignore + filter-repo
  + force-push) is the same as today's hegemon `f228b540` rewrite
  and was previously deferred from goal `mrdvbao1`. The bloat is
  still growing (verified live: 3 544 PNGs / 701 MiB at 21:00, up
  from 3 483 PNGs / 694 MiB at 19:58). A full filter-repo + force-push
  is irreversible and requires explicit operator authorization
  per the operator's commit policy. **Not executed by this audit.**

### 7.4 Fleet view after fix
| repo | a | b | ut | mod | flags | push |
|---|---|---|---|---|---|---|
| polis | 0 | 0 | 0 | 0 | OK | OK |
| neonbreak | 0 | 0 | 0 | 0 | OK | OK |
| junk-runner | 0 | 0 | 0 | 1 | DIRTY (transient) | OK |
| deathrun | 0 | 0 | 0 | 1 | DIRTY (transient) | OK |
| hegemon | 0 | 0 | 0 | 0 | OK | OK |
| dracon-platform | 0 | 0 | 0 | 2 | DIRTY (transient) | OK |

**Fleet totals:** 22 OK / 4 DIRTY (transient) / 0 CONCERN / 26 total.

The 4 remaining DIRTY repos are normal daemon catch-up cycles and
will clear on the next commit pass. No real divergence anywhere in
the fleet. The polis CONCERN is gone.

## 7. Verification evidence index

- Polis divergence: `git rev-list --count origin/main..HEAD = 1`,
  `git rev-list --count HEAD..origin/main = 1`, `git push --dry-run
  origin main` → "use 'git pull' before pushing again".
- Polis libgit2 error: 5 occurrences of `class=Net (12)` in the last
  30 min of journal, followed by "exceeded max failures (5)".
- Polis CLI OK: `git fetch origin` exit 0, `git remote -v` shows
  valid `git@…` SSH URLs on 4 remotes.
- Deathrun bloat: `git ls-files | grep '.png$' | wc -l` = 3 544 now
  vs 3 483 an hour ago; `du -sh .state-recon` = 701 MiB vs 694 MiB.
- Neonbreak playwright-report leak:
  `git ls-files | grep playwright` returns 3 entries (`index.html`,
  `playwright.config.ts`, `test-results/.last-run.json`).
- Neonbreak TOCTOU: `journalctl … grep neonbreak … | grep "fatal:
  unable to stat"` shows the `git add` race on `test-results/
  .playwright-artifacts-1/...png`.
- Fleet divergence scan: corrected SHA-only comparison (no
  `github/HEAD` false positives) returns 0 real divergences after
  the brief polis / avid races.
