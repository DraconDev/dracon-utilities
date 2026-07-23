# AUDIT-3-UTILITIES-FILTER-REPO-2026-07-15

## Objective
Strip the 9 forward-only ephemeral DIR-level patterns from the dracon-platform
nested game submodules via `git filter-repo --path-regex ... --invert-paths --refs main`,
force-push the rewritten history to all remotes, preserve legit uncommitted
edits, retain pre-rewrite backup refs, keep GitLab `main` protected
(`allow_force_push=false`), and clear the `PACK_SIZE_WARNING`.

## Submodules processed (8 actual; `endless-td` does not exist under /Dev)
deathrun, hegemon, junk-runner, capture-anime-girls, darklord, hellhunter,
neonbreak, polis.
polis was already clean (eph=0) — no rewrite needed; gitlink already matched.

## 9 patterns stripped
`.pi/`, `test-results/`, `verify-screenshots/`, `__screenshots__/`,
`.state-recon/`, `chrome-screenshots/`, `chrome-*/`, `sign-in-flash-audit/`, `~/`

Regex used: `(^|/)(\.pi|test-results|verify-screenshots|__screenshots__|\.state-recon|chrome-screenshots|chrome-[^/]+|sign-in-flash-audit|~)(/|$)`

## Results
- All filtered submodules: eph=0 (9 patterns stripped from history).
- All 8 synced (local == github == gitlab == codeberg == origin) at filtered SHAs.
- Pre-rewrite backup refs retained (local `refs/heads/backup/pre-sync-largeblob-fix-*`):
  - junk-runner `1784112643`, capture-anime-girls `1784113072`, darklord `1784113291`,
    hellhunter `1784113416`, neonbreak `1784113582` (local, pre-rewrite).
  - deathrun `1784111463` + hegemon `1784112055` (local, fetched from the
    corresponding gitlab pre-rewrite backup branches).
  - deathrun + hegemon ALSO retain remote gitlab backup branches
    `backup/pre-sync-largeblob-fix-*` (extra safety net).
- GitLab `main` protected with `allow_force_push=false` for all 8 submodules
  (PIDs verified via API: deathrun 83906612, junk-runner 83905868,
  capture-anime-girls 83906614, darklord 83906616, hellhunter 83905869,
  polis 83905863, neonbreak 83906718, hegemon 83934017).
- `PACK_SIZE_WARNING` cleared: `dracon-sync repos` shows no PACK warnings.
- Final daemon state: **📦 29 repos ✅ OK 29 · ⚠️ WARN 0 · ❌ CONCERN 0**.

## Legit edits preserved
Each submodule's uncommitted work was `git stash`ed before `filter-repo` and
`git stash pop`ed after. The daemon subsequently auto-committed the operator's
edits (verified synced). Pre-rewrite history remains recoverable via the backup
refs above. Operator's unrelated in-progress work in `dracon-platform`
(ai-api, auth-api, billing-api, etc.) was intentionally left uncommitted; only
the submodule gitlink updates were committed (parent commit `4524041628`,
pushed to codeberg + gitlab).

## Notes / lessons
- `git filter-repo --refs main` keeps the local backup ref PRISTINE (pre-rewrite),
  because only `main` is rewritten.
- Multiple `filter-repo` passes on the same repo cause SHA churn + can leave a
  post-filter ephemeral file behind (e.g. darklord's `.pi/TEST-BASELINES.md`);
  a fresh `rm -rf .git/filter-repo` + single re-run resolves it.
- GitLab force-push requires: DELETE `protected_branches/main`, force-push,
  then POST `protected_branches` with `allow_force_push=false`.
- A stale local `origin/main` tracking ref (pre-force-push) makes the daemon
  report a false AHEAD/BEHIND CONCERN; `git fetch origin main` refreshes it.
