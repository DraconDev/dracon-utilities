# pi-goal-list-loop-audit handoff (2026-08-09 11:00 BST)

Started from the user's pasted `dracon-sync repos` snapshot — fleet looked
healthy at a glance, but three "dirty" rows + 1 WARN invited scrutiny.

## Resolved (stale snapshots, not real concerns)

- **Row 11 🟡 WARN dracon-utilities "dirty 10h"**: stale. Daemon committed
  `.pi-glla/goals/20260809095202-mxysjl.md` at 10:53:17. Current: clean,
  0/0/0. The "dirty 10h" was `last_commit_minutes` against the prior daemon
  commit (5a7d66e09 at 10:51:53).
- **Row 10 dirty 52m capture-anime-girls**: stale. Daemon committed + pushed
  5b4606dd at 10:55:31. Normal 40–60s daemon cadence.
- **Row 5 pully-fully-pull-based-fleet-reconciler "pushing 0m"**: stale.
  Daemon committed + pushed 5b4606dd at 10:55:33. Has a stale
  `pre-rewrite-head-b6795b8b` backup branch (5 days old, janitor should reap
  per v0.113.10) — minor, no functional impact.

## Real concern worth a follow-up

**pi-goal-loop-audit stuck push for ~50 min, recovered on its own.**

Timeline (v0.113.47 daemon pid 2865494):
- 10:05:01 committed 197 files (submodule churn + repo hygiene).
- 10:05:05 origin push rejected non-fast-forward — auto-pull started.
- 10:05:11 auto-pull failed.
- 10:05:13–18 push retries 2/3 + 3/3.
- **10:05:42 gitlab push failed: `error: The destination you provided is not a
  full refname (i.e., starting with "refs/")`** — this is the detached-HEAD
  refspec bug. `src/git/push.rs:97–105` picks `ssh_refspec = "HEAD"` when
  `current_branch(repo) = Some(branch)`, but at the moment of failure the
  worktree was detached (HEAD was a commit, not a ref). git interprets bare
  `HEAD` as the commit SHA → rejection.
- 10:05:42 Push Failed alert + 9 retries over 47 min (10:11–10:52).
- ~10:55 push succeeded (repo currently shows pushed 2m, 0/0, push OK).

**Root cause**: `src/git/push.rs:97–106` — `current_branch()` can return
`Some(branch)` while HEAD is still a commit (worktree state inconsistency
between `resolve_head_path` HEAD-file read and the actual index/HEAD
pointer). The SSH refspec should be defensively `HEAD:refs/heads/<branch>`
whenever the caller can't prove HEAD is a named ref. The HTTPS fallback at
push.rs:139 already uses the fully-qualified form — making SSH match would
fix this.

**Proposed follow-up** (next session, after quota refresh):
1. Change `src/git/push.rs:97–106` to always use `HEAD:refs/heads/<branch>`
   as the ssh refspec (current_branch → just get the branch name, then
   format the refspec explicitly). Eliminates the detached-HEAD ambiguity.
2. Add a regression test: detached HEAD repo + ahead commit + push → push
   succeeds (currently would fail with the bare-HEAD refspec).
3. Audit the other call sites that use bare `HEAD` as src refspec.

Severity: MEDIUM-LOW. The repo recovers on its own within the retry
window, and the daemon's HTTPS fallback at push.rs:139 already works.
Not v0.113.48-worthy on its own — bundle with the next batch of fixes
(or open a follow-up goal).

## Currently

- Daemon pid 2865494 (v0.113.47) running, fleet 37 repos · 34 clean · 3
  active · 🟡 0 · ❌ 0 · ⛔ 0.
- Hit the main model quota wall at 11:00 (hourly refresh). Pausing until
  resume.