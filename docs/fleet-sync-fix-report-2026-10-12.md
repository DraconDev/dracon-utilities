# Fleet sync fix report — 2026-10-12

Goal: all non-clean rows (table rows 1-13 plus polis row 23) to clean-and-pushed,
safe sync only (no history rewrite, no GC, no mass delete).

## Daemon health — HEALTHY, running, actively working

- `dracon-sync health`: healthy, daemon running, freeze off, policy valid
  (exit 0, observed 2026-10-12 ~02:05 BST).
- `systemctl --user status dracon-sync.service`: active (running) since
  2026-10-10 07:43 BST. No restart was needed or performed.
- Daemon actively pushing during the whole observation window:
  browser-extensions-shared, football-forever, junk-runner in 🟣 pushing;
  deathrun drained to CLEAN/synced (commit 4e46ab8d).
- Known limitation (not fixed, out of scope): status inspection wedges
  (>60s, aborted, re-probed next cycle) on huge repos — come-get-me,
  deathrun, db-gateway, terhub observed in journal. This is why many repos
  show 🔴 STALLED despite PUSH ✅ OK.

## Fully cleared

- deathrun: STALLED M4/U2 → ✅ CLEAN, 0/0/0, PUSH OK, synced (4e46ab8d).

## Push-STUCK trio — root-caused, needs operator content decision

All three are refused by the pre-push hook
`BUCKET_STRATEGY_GUARD_FORWARD_ONLY` (verified via
`git push --dry-run --porcelain`, read-only). The daemon correctly retries
and fails; bypassing the hook or rewriting history is out of scope.

- dracon-platform: 28→30 ahead, STUCK (11 failures). Blocking commit
  e49d8e4a adds 32 files under web/books (chapters 01/02 …).
- hellhunter: 9 ahead, STUCK (11 failures). Blocking commit bca24ce1
  renames 100 files static/art/generated/ → static/art/shipped/.
- polis: 9 ahead, STUCK (8 failures), otherwise CLEAN. Blocking commit
  dda6a2a0 renames static/images/polis/screens/olympus-bg-v3.png → …-v2.png.

Fix options for the operator (not taken): revert with forward commits, or
authorize a guard exception. No `--force`, rebase, or filter-branch was used.

## Draining / moving target (daemon pushing, PUSH PENDING)

- browser-extensions-shared: MOD 50→63→66→71, UT 30→35→39→43, ahead 1→2,
  🟣 pushing. A live dev loop is writing faster than the daemon drains.
  Needs loop pause or acceptance; not a sync defect.
- football-forever: 0/0/0, ahead 1, 🟣 pushing.
- junk-runner: M1/U4, ahead 3, 🟣 pushing.
- pi-goal-list-loop-audit: MOD 6→1 (daemon committing goal files), PUSH OK.

## STALLED with PUSH ✅ OK (inspection wedge, tiny dirty, will drain)

.dracon (M2), pi-plugins (M1), freeport (M1/U7), dracon-strategy (M1),
darklord (M1/S1/U1), neonbreak (M4/S1/U1), one-mil-girls (M1), come-get-me
(M1), capture-anime-girls (M1/U2), doomtap (M1), terhub (M1). All PUSH OK,
no ahead. Contents are ordinary docs/tests/data edits (spot-checked via
`git status --short`; no secrets seen). These flip CLEAN↔STALLED as the
daemon's wedged inspections abort on multi-GiB repos and loops write.

## Commands run (all read-only except daemon's own commits)

- `dracon-sync health`, `systemctl --user status dracon-sync.service`
- `dracon-sync repos` (4× full runs over ~20 min), `dracon-sync repos <name>`
- `dracon-sync repair concerns` (dry-run; only unrelated vanished paths)
- `git remote -v`, `git push --dry-run --porcelain` (platform/hellhunter/polis)
- `git status --short` spot checks (.dracon path missing at
  /home/dracon/Dev/.dracon — daemon label unresolved, PUSH OK regardless)

No `push --force`, rebase, filter-branch/filter-repo, `rm -rf`, or GC was
run. No branches created. No identities touched.
