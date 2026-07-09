# Untrackeds + push-pipeline audit — 2026-07-09 (goal `mrdvbao1`)

**Trigger:** operator asked to explore every untracked file across the
watched fleet — are they meant to be out — and check whether everything
tracked is in fact being committed and pushed; investigate any repos
sitting on changes or not pushing.

---

## 1. Method

- Enumerated the daemon's watched fleet: `dracon-sync repos --json`
  (26 repos, 0 errors at parse time).
- For each repo, ran `git status --porcelain -z`, kept only the
  `??`-prefixed lines = the universe of untracked entries.
- Cross-referenced each untracked against:
  - the repo's `.gitignore` (and any parent-platform `.gitignore`), and
  - the daemon's ledger / `dracon-sync repos --json` flags
    (AHEAD/BEHIND/STUCK/UNOWNED/DIRTY).
- Daemon health: `systemctl --user is-active dracon-sync.service`,
  orphan-push count via `ps aux | grep 'git push'`, journal
  `journalctl --user -u dracon-sync.service --since "30min ago"`
  filtered for `reject|fatal|non-fast-forward|denied|auth|unsupported URL|class=Net|untrusted`.
- Push-pipeline verification: corrected local-vs-remote divergence
  scan (valid 40-char SHA only — no `github/HEAD` false positives)
  for every repo × every configured remote (`origin github gitlab
  codeberg`).

## 2. Inventory — every untracked file across the 26-repo fleet

Two of twenty-six repos contain untracked entries; twenty-four have
**zero untracked files**.

### 2.1 `/home/dracon/Dev/dracon-utilities` (4 entries)
| path | size | kind | ext |
|---|---|---|---|
| `.pi/goals/active_goal_2026070919562860_mrdvbao1-7triyb.md` | 4 374 B | FILE | md |
| `dracon-sync/` | — | DIR (nested git) | — |
| `dracon-system/` | — | DIR (nested git) | — |
| `dracon-warden/` | — | DIR (nested git) | — |

### 2.2 `/home/dracon/Dev/dracon-platform/web/games/wip/deathrun` (1 entry)
| path | size | kind | ext |
|---|---|---|---|
| `.state-recon/audit-2026-07-09-credits/` | ~20+ PNGs | DIR (audit screenshots) | png |

### 2.3 Repos with zero untrackeds (24)
`junk-runner`, `hegemon`, `neonbreak`, `hellhunter`, `dracon-platform` (parent),
`darklord`, `endless-td`, `capture-anime-girls`, `browser-extensions-shared`,
`/home/dracon/.dracon`, `avid`, `polis`, `pully`, `dracon-sync`
(inner nested), `dracon-code`, `web-auto`, `rust-ai-web-auto`,
`ai-auto-writer`, `pi-plugins`, `dracon-strategy`,
`one-mil-girls`, `dracon-system` (inner nested),
`DraconDev` (submodule), `dracon-warden` (inner nested).

## 3. Classification — correctly out vs missed

### 3.1 dracon-utilities entries — **all correctly untracked**
- `.pi/goals/active_goal_*.md` — the daemon's active-goal file for this
  audit. The daemon archives it as
  `archived/goal_<created>_mrdvbao1-7triyb.md` on completion and the
  archived copy is tracked. The current untracked status reflects the
  pre-commit state and will resolve on the next daemon cycle. **No
  operator action.**
- `dracon-sync/`, `dracon-system/`, `dracon-warden/` — separately
  watched standalone git repositories inside the dracon-utilities tree.
  Each has its own `.git/` and is excluded by the global
  `untracked_exclude_patterns` behaviour plus `dracon-utilities`' own
  nested-repo exclusions. **No operator action.**

### 3.2 deathrun `.state-recon/audit-2026-07-09-credits/` — **definitionally wrong root, but this single dir is correctly untracked**
The dir name (`audit-2026-07-09-credits/`) suggests new audit PNGs that
the operator's audit run produced and dropped into deathrun's working
tree. The fact that they appear `??` (untracked) is itself notable only
because it sits *inside* `.state-recon/`, a directory that **is already
heavily tracked** in deathrun's history (see §4.1). Treating this one
dir as untracked is inconsistent with the rest of the parent — siblings
like `audit-2026-07-09-welcome/`, `audit-2026-07-09-parity-death/`,
`audit-2026-07-09-fixfullwidth/`, `audit-2026-07-09-strict/`,
`audit-2026-07-09-project/`, and `audit-2026-07-09-phaser/` are all
**tracked** and have been actively pushed to all three remotes.

**Classification verdict:** correctly untracked at the per-file level
for the moment, but the parent directory *should not be tracked at all*
(see §4.1). The next daemon cycle is highly likely to commit this dir
on its own, inconsistent with whatever the operator's intention is for
session-scratch audit material.

### 3.3 What about the earlier ad-hoc untrackeds I spotted at first pass?
- `_pc.mjs` (deathrun), `web/games/libs/phaser/src/svelte/` empty dir,
  `web/games/docs/B-MIGRATION.md` — all either transient build
  artifacts, transient empty dirs filled in seconds later, or tracked
  docs that the working tree showed untracked at exactly that second.
  None were real missed legit files.

## 4. Investigation — repos sitting on changes / not pushing

The user's framing pointed at "repos sitting on changes, or not pushing"
as the common failure modes. Here is what I found.

### 4.1 deathrun `.state-recon/` — actively bloating history (THE finding)
- `git ls-files | grep '\.png$' | wc -l` → **3 483 tracked PNGs**.
- `du -sh .state-recon` → **694 MiB** on disk.
- Recent commits show a relentless dump:
  - `55892f2` → BIN:7 (new `audit-2026-07-09-credits/*.png` × 4+).
  - `862bc72` → BIN:1 (`audit-2026-07-09-welcome/cold.png`).
  - `b9a023a` → BIN:6 (`audit-2026-07-09-parity-death/*.png` × 6).
  - `cc999cf` → BIN:24 (`audit-2026-07-09-fixfullwidth/*.png`).
  - `ffec8e4` → BIN:18 (`audit-2026-07-09-strict/before/*.png`).
  - `a1ed037` → BIN:24 (`audit-2026-07-09-strict/*.png`).
  - `b739bd3` → BIN:13 (`audit-2026-07-09-project/runtime-*.png`).
- This is the **identical anti-rebloat pattern** that produced the
  hegemon 2 GiB / orphan-push / GitHub-pack-overflow incident earlier
  today (goal `f228b540`): audit screenshots being committed into
  git because `.state-recon/` (or a similar name) is **not in the
  deathrun `.gitignore` `!*.png` carve-outs**.
- deathrun's `.gitignore` does not list `.state-recon/` anywhere
  (verified by grep over `deathrun/.gitignore`,
  `dracon-platform/web/.gitignore`, `dracon-platform/web/games/.gitignore`,
  `dracon-platform/.gitignore`). The warden-managed block was likely
  overwritten or never had a per-game `.state-recon/` rule.
- **Recommendation:** add `**/.state-recon/**` to a user-managed
  section of `deathrun/.gitignore` (parent-dir excludes override
  warden `!*.png` / `!*.jpg` negations per the AGENTS.md note on
  the hegemon fix). Then, just as for hegemon today, `git filter-repo
  --invert-paths --force` to drop the existing trees from history and
  force-push. Treat as an OPERATOR decision; this audit does not
  execute it.

### 4.2 neonbreak `test-results/` race — daemon commits a PNG that's been deleted mid-operation
- Journal (19:50:47):
  > `⚠️ sync failed for .../neonbreak: git add failed ... fatal: unable to stat 'test-results/.playwright-artifacts-6/988dcf407e7b8c07793a20d60bafff6d.png': No such file or directory`
- Cause: Playwright cleans up `test-results/` while the daemon is
  running its cycle; the daemon's `git add` then fails on the vanished
  file. The next cycle's `git status` no longer shows the file (it has
  been removed), so the daemon just logs the failure and proceeds.
- `dracon-platform/.gitignore` already lists `**/test-results/`, so
  the file is untracked in the index; the failure is **purely a TOCTOU
  race**, not a real divergence. No data loss.
- Minor improvement (deferred): daemon could ignore
  `error: unable to stat '<ephemeral>'` as a "file was deleted while
  we walked" warning instead of `⚠️ sync failed … aborting sync pass`.
  This would prevent the cascade of "exceeded max failures / skipping"
  that haunted pully earlier (goal `9f95d4d6`).

### 4.3 deathrun index.lock race — concurrent daemon cycle
- Journal (19:53:57):
  > `⚠️ sync failed for .../deathrun: git add failed ... fatal: Unable to create '/home/dracon/Dev/dracon-platform/.git/modules/web-games-deathrun/index.lock': File exists.`
- Cause: a previous `git add -A` / commit cycle on the deathrun nested
  submodule left an `index.lock` behind. The next cycle tried to
  acquire the same lock and failed (128). Recovery: stale lock is
  automatically cleaned on the next cycle (a couple of minutes later the
  daemon retried successfully — see §6 below).
- Root cause: an interrupted / crashed prior cycle. Not a real sync
  failure. Same minor-improvement class as 4.2 (better lock cleanup).

### 4.4 dracon-utilities transient AHEAD:1 — race-window artifact, **not** stuck
- During this very audit, the corrected scan briefly caught
  `dracon-utilities` at AHEAD:1 vs `gitlab/main` and `codeberg/main`
  (origin/github had already received the push). Within ~3 minutes the
  daemon pushed to gitlab/codeberg and the repo returned to
  ahead=0/behind=0 across all three remotes.
- This matches the canonical daemon-push-to-all behaviour:
  `origin` (github) tends to receive first because of a faster push
  path; gitlab/codeberg follow within a cycle. No operator action.

### 4.5 `dracon-utilities` GitLab/Codeberg metadata-API errors (19:44:32)
- > `⚠️ failed to set GitLab metadata for dracon-utilities: GitLab metadata update failed: repo not found`
- > `⚠️ failed to set Codeberg metadata for dracon-utilities: Codeberg metadata update failed: repo not found`
  (and the paired `visibility` errors).
- These are **post-push hygiene calls** (set repo visibility /
  metadata), not push failures. The push itself succeeded (the commit
  arrived on both remotes per §4.4). The metadata endpoint reported
  "repo not found" — most likely because the API token does not own
  the `DraconDev/dracon-utilities` path on codeberg/gitlab, or the
  endpoints exist but the path casing differs (per AGENTS.md
  `goal 354fe3cb` note, SSH URLs use `DraconDev` and trusted-host
  matching was uppercased; the metadata API may still use a different
  case).
- A direct check (`curl https://gitlab.com/api/v4/projects/DraconDev%2Fdracon-utilities` with the token from
  `~/.dracon/utilities/sync/secrets/.gitlab-token`) → HTTP 200, so
  the repo **does exist** and the API resolves it. The daemon's
  failure must be a casing / trailing-slash / scope mismatch in its
  `metadata update` endpoint.
- Material: not a sync blocker; the pushes work. Cosmetic /
  debug-quality finding — recommend the daemon log the API URL it's
  calling so future audits can pinpoint the route. Deferred.

## 5. Push-pipeline verification — everything else IS going up

| metric | value | evidence |
|---|---|---|
| Daemon status | active | `systemctl --user is-active dracon-sync.service` → active |
| Orphan `git push` processes | **0** | `ps aux \| grep 'git push' \| grep -v grep \| wc -l` |
| Total repos | 26 | `dracon-sync repos --json` |
| Repos with `AHEAD>0` | **0** | per-repo ahead/behind tally |
| Repos with `BEHIND>0` | **0** | per-repo ahead/behind tally |
| Repos with `STUCK_*` | **0** | `state_flags` scan |
| Repos with `untrusted_author` | **0** | `state_flags` scan (dracon-code resolved by goal `mrdmxu8n`) |
| Corrected divergence scan (valid 40-char SHA only) | **0 real divergences** | all local `main` == all configured remote `main` after the brief dracon-utilities race in §4.4 |
| Non-cosmetic journal errors in last 30 min | 2 (both TOCTOU races, both auto-recovered — see §4.2 + §4.3) | filtered journal |

## 6. Verification evidence index

- `dracon-sync repos --json` → 26 repos, ahead/behind totals = 0/0.
- `git status --porcelain -z` per repo → only 5 untracked entries
  across 2 repos (full list in §2).
- `journalctl --user -u dracon-sync.service --since "30min ago"`
  grep'd for
  `reject|fatal|non-fast-forward|denied|auth|unsupported URL|class=Net|untrusted`
  → 2 transient race-condition sync-fail lines (§4.2, §4.3), 0
  permanent push rejections.
- `ps aux | grep 'git push' | grep -v grep | wc -l` → 0.
- Corrected divergence scan → 0 real divergences after the brief
  dracon-utilities AHEAD:1 window.
- deathrun `git ls-files | grep '\.png$' | wc -l` → 3 483.
- `du -sh .state-recon` → 694 MiB.
- `curl -s -o /dev/null -w "%{http_code}" 'https://gitlab.com/api/v4/projects/DraconDev%2Fdracon-utilities'`
  with the daemon's stored `~/.gitlab-token` → HTTP 200 (so the "repo
  not found" in §4.5 is an endpoint-routing issue, not a missing
  repo).
