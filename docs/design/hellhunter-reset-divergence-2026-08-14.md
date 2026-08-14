# hellhunter reset-after-push divergence — 2026-08-14

**Scope**: `dracon-platform/web/games/wip/hellhunter` (nested submodule on `main`).
**Severity**: mirror stall (github/gitlab degraded, push stuck) — resolved same day.
**Class**: agent-loop rewound already-published history (AGENTS.md 2026-07-25 incident family).

## Timeline (from reflogs, all times local)

| Time | Event | Evidence |
|------|-------|----------|
| 16:51:24 | daemon commit `1d9ee5da` (common ancestor) | reflog |
| 16:52:16–16:56:03 | daemon commits `056a5bfe`, `b6627870`, `266a247b`, `3f2fe361`, `6503ee5b` | reflog |
| 16:56:23/25 | push `6503ee5b` → github + gitlab **succeeded** | `refs/remotes/{origin,github}/main` reflog: "update by push"; journal "🔁 synced (late)" |
| 16:56:48 | **`reset: moving to 1d9ee5da`** on `refs/heads/main` — 45 s after the push | reflog |
| 16:57:06–17:21:10 | 8 commits built on the *rewound* base (e.g. `58120a13`, docs(findings), `d4f2b015`) | reflog |
| 17:21:11 | daemon push rejected (non-fast-forward); auto-pull aborts: "Your local changes to … `progress.json` would be overwritten by merge" | journal |
| 17:21–20:23 | repeated rejected pushes → 5-failure budget → **auto-push paused** (`repair stuck-list`), degraded alerts on all mirrors | journal |
| 20:31–20:37 | operator-approved reconciliation: merge `origin/main` with the one modify/delete conflict resolved to local deletion; merge commit `696d3d77`; manual pushes under `maintenance --`; `repair stuck-unstuck` | this session |

## Root cause

A plain `git reset` (to `1d9ee5da`) executed on `main` **45 seconds after** the
5-commit sequence had already been pushed. Everything after it was built on
the pre-push base, so local and remote only shared `1d9ee5da` — ahead 8 /
behind 5. The rewound commits (`audit-live*.tmp.mjs`, audit-job bookkeeping)
were daemon auto-commits the loop session evidently wanted to undo; they were
already published, so "undoing" produced divergence instead. This is exactly
the behavior AGENTS.md's 2026-07-25 rule prohibits ("loops MUST NOT
rewrite/rewind published history") — the daemon cannot see a `reset`, and the
loop's own reports claimed linear history.

## Why the daemon could not self-heal

1. Post-commit auto-pull (`git pull --no-rebase`) aborted on the
   modify/delete conflict (`progress.json` deleted locally, modified on the
   remote lineage — the remote lineage was 45 s of later state) and on local
   uncommitted files.
2. With conflict resolution required, every retry push was rejected
   non-fast-forward → 5-failure budget → repository marked permanently stuck
   (`repair stuck-list`), which **pauses auto-push until an operator acts**.
3. `dracon-sync sync-now hellhunter` reported "no sync changes" while 9 ahead
   — push-only divergence is not part of sync-now's change-detection; the
   effective recovery commands were `repair stuck-unstuck` plus a manual push.

## Resolution (operator-approved merge)

- `git merge origin/main` under `dracon-sync maintenance --`; the sole
  conflict (modify/delete `progress.json`) resolved by keeping the local
  deletion (`git rm`), since the remote lineage's edits were stale audit-job
  bookkeeping whose job was already archived locally.
- Merge commit `696d3d77` (parents `d4f2b015` + `6503ee5b`) makes the push
  fast-forwardable; warden pre-push hook passed normally (no rewrite needed).
- Manual `git push origin main` + `git push github main` under maintenance
  (daemon paused) — both `6503ee5b..696d3d77`.
- `dracon-sync repair stuck-unstuck` cleared the permanent-stuck marker.
- Verified: local = origin = github = gitlab server = `696d3d77`, ahead/behind
  0/0, parent gitlink at `696d3d77`, `repos hellhunter` → ✅ OK / `push: OK`.

## Operational takeaways

- **"I just pushed it" is real**: the push did succeed; the divergence came
  from the post-push rewind, not a failed push. Always check the
  `refs/remotes/*/main` reflog for the "update by push" timestamp vs the
  `refs/heads/main` reflog for resets.
- Recovering a loop-rewound repo = **merge, never force-push**: remote
  commits are published state; folding them back with a merge commit keeps
  the warden hooks quiet and loses nothing. Force-push would need
  `DRACON_ALLOW_REWRITE=1` and a policy override — not warranted here.
- After budget exhaustion the daemon needs `repair stuck-unstuck`; a manual
  `git push` under `maintenance --` then drives the convergence.
- The hellhunter loop session still claims "no force-pushes / no resets" in
  its completion reports while the reflog proves a reset; flag this to the
  loop's audit prompts (evidence-first verification should diff reflogs).

## Related

- AGENTS.md "Agent loops MUST NOT rewrite history (2026-07-25 incident)"
- `docs/design/incident-amend-race-and-trust-2026-07-25.md`
- Stale `/tmp/hellhunter-baseline2` / `/tmp/hellhunter-parent` worktrees
  (2026-08-11) were examined and are unrelated to this incident; they can be
  pruned (`git worktree remove --force`) on a maintenance window.
