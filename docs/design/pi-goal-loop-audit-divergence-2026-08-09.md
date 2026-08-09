# pi-goal-loop-audit history divergence — 2026-08-09

> **Status**: LIVE. The repo is `🛑 STUCK` (auto-push paused) with a
> genuine 3-way history fork between github, gitlab, and codeberg.
> Reconciliation requires operator authorization — this document lays
> out the evidence, the root cause, and the options. The daemon is
> behaving CORRECTLY (it refuses to force-push divergent mirrors); no
> daemon code change is required to fix this.

## TL;DR

The pi-goal-loop-audit loop agent ran `git reset` at 10:07:47 (2026-08-09),
discarding the daemon's 197-file commit `d60ec3a2` from local `main` after
the pre-reset history had already been published to gitlab/codeberg. The
three forges now disagree about what `main` is:

| Remote | main tip | commits unique to it |
|---|---|---|
| github (origin) | `afc3bfd0` "Retroactive v0.34.116 ship marker" | 13 (v0.34.115–117 work) |
| gitlab | `a97d2255` | 59 (incl. v0.34.114 real work + `d60ec3a2`) |
| codeberg | `a97d2255` (same history) | 59 |

Merge-base: `801100b2` (v0.34.112). The daemon refuses to force-push
(`force_push_when_behind = false` default + warden pre-push hook) and has
paused auto-push after exhausting the stuck budget — correct behavior.

## Timeline (2026-08-09, all times BST)

| Time | Event |
|---|---|
| 10:05:01 | daemon commits 197 files → `d60ec3a2` (v0.34.114 work) |
| 10:05:05 | push to github rejected non-fast-forward; auto-pull attempted |
| 10:05:11 | auto-pull FAILED (exit 1); retries follow |
| 10:05:26 | origin (github) push fails — the local branch was already behind |
| 10:05:42 | gitlab push fails with the **bare-`HEAD` refspec bug** (v0.113.48 fixed this later); stuck record created `stuck_since=1786266342` |
| 10:07:47 | **loop agent runs `git reset` → local main rewound** |
| 10:08:03 / 10:08:13 | loop agent ships v0.34.115 on the rewound history |
| 10:08:40 | checkout back to `main` |
| 11:16 | v0.113.48 deployed (detached-HEAD refspec fix) |
| 12:36–12:48 | daemon commits v0.34.116 work locally; gitlab+codeberg pushes fail **non-fast-forward** (the mirrors hold the pre-reset history) |
| 12:48 | stuck budget exhausted (5 consecutive failures) → `🛑 push stuck` alert; auto-push paused |
| 13:46 | daemon successfully pushes the reset history to github (`afc3bfd0`) — github now == local |
| ~14:41 | user snapshot shows `pi-goal-loop-audit 🟡 WARN · 🛑 STUCK` — the state documented here |

## Root cause

The loop agent did not have a git-history policy file in this repo. The
AGENTS.md fleet policy ("Agent loops MUST NOT rewrite history", added
2026-07-25 to dracon-platform and browser-extensions-shared) was never
copied here, so the loop's "recovery" instinct (`git reset` to a tag when
the daemon's push wedged) was unconstrained. The reset raced the
already-published mirror history and created the fork.

Secondary factors:

- The 10:05 wedged push (pre-fix refspec bug) left local `main` visibly
  "wrong" (the 197-file commit couldn't land), inviting the loop to
  "fix" it by resetting. The v0.113.48 fix removes the wedge class, but
  the loop must still be told never to reset.
- The "Mirror Degraded: mirror may be unreachable" alert is misleading
  for this failure class — the mirror is reachable; its history
  diverged. (Observability follow-up, see below.)

## Daemon behavior assessment: CORRECT

- Refused to force-push both divergent mirrors (protecting 59 published
  commits from silent destruction). ✓
- Escalated: stuck-budget exhaustion → `🛑 push stuck` alert + paused
  auto-push (designed stop condition, H5/v0.112.31). ✓
- Continued committing the repo's local work and pushed it to github
  (the non-divergent remote). ✓
- The `repos` table renders the state accurately (`🛑 STUCK`, push-stuck
  activity, 121 MiB). ✓

## Reconciliation options (operator decision — NOT an agent action)

The daemon CANNOT resolve this; any resolution is a history rewrite of
published mirrors and requires operator authorization per
`junk-runner-history-rewrite-2026-07-28.md`.

**Option 1 (recommended) — github/local is canonical; force-update
gitlab + codeberg.** v0.34.115–117 are the newest shipped versions and
github already has them. The 59 mirror-only commits are superseded work
(preserved in git history + `.pi-glla/archive/`). Steps:

1. `dracon-sync maintenance -- git -C /home/dracon/Dev/pi-goal-loop-audit fetch --all`
2. Sanity-check the 59 mirror-only commits for anything not archived
   (`git log main..gitlab/main --oneline | grep -iE 'TAG:|release'`).
3. Unprotect `main` on gitlab (Settings → Repository → Protected
   branches → allow force push). Codeberg has no branch protection.
4. `cd /home/dracon/Dev/pi-goal-loop-audit && DRACON_ALLOW_REWRITE=1 git push --force-with-lease gitlab HEAD:refs/heads/main && DRACON_ALLOW_REWRITE=1 git push --force-with-lease codeberg HEAD:refs/heads/main`
5. Re-protect `main` on gitlab.
6. `dracon-sync unstuck pi-goal-loop-audit` (or `repair-concerns --apply`).

**Option 2 — leave STUCK.** The daemon keeps alerting (throttled to 30
min) and pauses auto-push. Commits continue to land locally + github.
Acceptable only if the loop is dormant (its queue is currently empty;
last goal completed 12:49).

**Option 3 — merge rather than fork.** Rebuild local `main` to include
both sides via `git merge` + resolve conflicts, then push to all. This
preserves every commit but is substantially more work and produces a
merge history the loop may not want.

## Immediate fixes applied (this incident)

1. **`AGENTS.md` added to pi-goal-loop-audit** (2026-08-09): copies the
   fleet "git-history rules for agent loops" section verbatim, adds the
   LIVE INCIDENT block documenting this fork, and forbids `git reset`.
   This closes the root-cause gap (missing policy file) so the loop is
   never again unconstrained.

## Observability follow-up (recommended, not shipped)

The `repos`/alert path reports mirror failures as "Mirror Degraded: N
consecutive push failures — mirror may be unreachable", which is wrong
for the divergence class (the mirror is reachable; the histories
forked). Recommended: thread the raw push error (or a classification:
divergence / server-policy / pack-too-large / transport) into the
stuck-ledger `last_error` and the alert text, so the operator sees
"history divergence — remote has N commits not on local" instead of
"may be unreachable". The raw message IS available at the push call
sites (e.g. `push_mirror_remotes` errors in `push_background`,
sync.rs:1858); only the per-remote count is currently persisted.

## Related

- `docs/design/detached-head-push-refspec-2026-08-09.md` — the 10:05
  wedge's root cause + v0.113.48 fix.
- `docs/design/junk-runner-history-rewrite-2026-07-28.md` — rewrite
  authorization procedure.
- `docs/design/incident-amend-race-and-trust-2026-07-25.md` — why agent
  loops must not rewrite history.
