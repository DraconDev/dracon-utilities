# Platform PUSH_STUCK Resolution — 2026-06-26

> **Status**: design-only analysis. The operator has not approved any
> recovery action yet. This doc is the third deliverable in goal
> `mqu95usq-hbt73a` (cut daemon alert spam, then analyze cause). The
> first two deliverables (raising the daemon's `alert_unpushed_threshold`
> to 50 and tightening `~/.dracon/sync-notify/sync-notify.sh` with a 30-min
> throttle + signature-based dedup) are already deployed. This doc is the
> "think about addressing the cause" part.

## Summary

`dracon-platform` has been PUSH_STUCK since 2026-06-23 13:36 BST (goal
`mqqsyzyd-qkvna5`, see `docs/design/gitlab-storage-and-divergence-2026-06-23.md`
for the original incident). The push is stuck because local and codeberg
have **diverged**: local has 905+ commits codeberg doesn't have (growing
~1 commit every 30 seconds), and codeberg has 20 commits local doesn't have
(older codeberg-side agent pushes). The daemon's `git pull --rebase`
strategy fails because the branches are no longer fast-forward-compatible.

This doc enumerates four recovery options, each with cost, risk, and
operator-effort estimates. The recommendation is at the end; the operator
decides.

## Problem statement (2026-06-26 02:40 BST)

```
$ cd /home/dracon/Dev/dracon-platform && git rev-list --count codeberg/main..HEAD
905        # local is 905 commits ahead of codeberg
$ git rev-list --count HEAD..codeberg/main
20         # codeberg is 20 commits ahead of local
$ dracon-sync repos --json | python3 -c "..." | grep platform
  dracon-platform: ahead=905 behind=20 push_status=PUSH_STUCK
                  state=committing concern=true
```

**Divergence history** (from `journalctl --user -u dracon-sync.service`):
- 2026-06-23 13:36 BST: first PUSH_STUCK (504 ahead, 20 behind)
- 2026-06-25 19:36 BST: previous goal snapshot (704 ahead, 20 behind)
- 2026-06-26 02:40 BST: this snapshot (905 ahead, 20 behind)

The platform's local `.git` keeps growing because the daemon's commit-all
policy continues to commit working-tree changes locally (~62 commits/h
in the last 24h, per the daemon's stats), but the push fails on every
attempt because codeberg has diverged.

## Root cause analysis

The daemon's push strategy is `git push --force-with-lease` to codeberg
(per the daemon source at `dracon-sync/src/git/multi_remote.rs` and the
prior goal's `force_push_when_behind = true` for codeberg). This SHOULD
work for the platform case (operator is sole author per
`/home/dracon/Dev/dracon-platform/.dracon/dracon-sync.toml:owned = true`).

But the daemon's push **also requires** a successful `git pull --rebase`
before the push (auto_pull = true in the global config). The pull fails
because:

1. Local has 905 commits codeberg doesn't → codeberg rejects the push
   (it's not a fast-forward)
2. Local tries `git pull --rebase codeberg` → codeberg's HEAD is not a
   direct ancestor of local's HEAD (it's the diverged line)
3. Rebase fails with "fatal: refusing to merge unrelated histories" or
   similar
4. Daemon logs the failure, schedules a retry, and the cycle repeats

The 20 codeberg-only commits are old (from 2026-06-23 ish, when a
separate codeberg-side agent process was active). They've been sitting
on codeberg and never made it to local.

**Why doesn't `--force-with-lease` save us?**
The `--force-with-lease` flag requires the remote's current ref to match
the expected ref. The daemon computes the expected ref as `codeberg/main`
from BEFORE the push attempt. But:
- If `auto_pull` runs first and fails, the push is never attempted
- If `auto_pull` succeeds with a merge, the local HEAD diverges from what
  `--force-with-lease` expects → push aborts

The full chain is: pull → fail → push never attempted → daemon retries →
pull fails again → loop. This is why the platform's ahead count keeps
growing but no commit ever reaches codeberg.

## Option A: LFS migration first (recommended for safety)

**What**: Migrate the 19 GiB of MP3 audio files in
`web/games/wip/*/static/audio/` to Git LFS. This shrinks the local `.git`
from 19 GiB to ~1 GiB. Then re-attempt the 3-mirror sync (currently
platform only pushes to codeberg via `exclude_remotes = ["github", "gitlab"]`).

**Cost**: 1-2 weeks of work (was §10's "Recommended path forward" in the
prior design doc at `docs/design/big-repo-storage-strategy.md`).

**Risk**: Low. LFS migration is reversible. The MP3 files don't change
often, so the LFS pointers are stable.

**Operator effort**: High (manual migration of audio files, daemon LFS
support verification).

**Why recommended**: The 19 GiB `.git` is the underlying reason the
platform can't push to github (5 GB soft cap) or gitlab (10 GiB cap). Even
after resolving the divergence, the platform stays codeberg-only until
LFS migrates the bloat. This option resolves both problems at once.

**Trade-off**: Does NOT immediately resolve the PUSH_STUCK. After LFS
migration, the operator still needs to do a recovery action (B or C) to
unblock the daemon.

## Option B: `git push --force-with-lease` to overwrite codeberg

**What**: Run `git push --force-with-lease origin main` from
`/home/dracon/Dev/dracon-platform` to overwrite codeberg's diverged 20
commits with local's 905 commits. The operator is sole author per the
per-repo override (`owned = true`), so no risk of clobbering someone
else's work.

**Cost**: 1 minute. Single command, single push.

**Risk**:
- **Loses the 20 codeberg-only commits.** They were pushed by a separate
  codeberg-side process; they're old (2026-06-23) and likely obsolete, but
  they're gone after this. Need to verify nothing currently depends on
  them before running.
- If the local commits haven't all been tested, this could push
  untested work. (For the platform, all commits are by DraconDev, so this
  is acceptable.)
- The 905-commit push will hit the daemon's `push_op_timeout_secs = 900s`
  (15 min). The current PUSH_STUCK loop suggests a single push IS slow
  on this repo. May need temporary timeout bump.

**Operator effort**: Low (one command + verify post-state).

**Why viable**: The daemon ALREADY has `force_push_when_behind = true`
for codeberg (per `utilities/sync/dracon-sync.toml:267`). This option
just runs that code path manually.

**Why NOT recommended as the FIRST option**: The 19 GiB `.git` is still
there. After this option, platform stays codeberg-only, and the next
commit cycle will push more data through codeberg (slow but works). The
fundamental bloat problem isn't fixed.

## Option C: `git pull --rebase` to merge codeberg's 20 commits first

**What**: Run `git pull --rebase codeberg main` from
`/home/dracon/Dev/dracon-platform` to bring codeberg's 20 diverged
commits into local. Then push normally (no force-push needed).

**Cost**: 5-15 minutes (rebase complexity depends on conflict count).

**Risk**:
- **Rebase conflicts**: codeberg's 20 commits modify files that local
  may also have modified (since 2026-06-23). Each conflict needs manual
  resolution. Worst case: 20 separate conflict-resolution sessions.
- **History rewrite**: the rebase rewrites local's commit SHAs. AGENTS.md
  says "NEVER rewrite history" but rebasing local unpushed commits is
  safe (no remote has them yet).
- The result is a complex merge commit history; debugging gets harder.

**Operator effort**: Medium-High (must resolve conflicts).

**Why NOT recommended**: The rebase is risky and time-consuming for
what is essentially "resolve a divergence caused by another process".
The 20 codeberg-only commits are from a defunct agent process; they
should probably just be discarded (Option B), not merged.

## Option D: Drop local diverged state and re-clone from codeberg

**What**: `rm -rf /home/dracon/Dev/dracon-platform && git clone codeberg`
to start fresh from codeberg's state.

**Cost**: 5 minutes (clone the 19 GiB repo).

**Risk**:
- **LOSES THE 905 LOCAL COMMITS.** Codeberg doesn't have them. They
  would be deleted permanently (no reflog survives a `rm -rf`).
- AGENTS.md forbids history rewrites; this is even worse (it deletes
  history).

**Operator effort**: Low (rm + clone).

**Why this is a NON-OPTION**: The 905 local commits are real work
(game assets, code changes, screenshots). Deleting them is unacceptable.

If the operator disagrees ("these 905 commits are throwaway anyway"),
this becomes viable but should be confirmed explicitly.

## Recommendation

**Primary recommendation: Option B** (`git push --force-with-lease`),
preceded by a brief verification that the 20 codeberg-only commits are
safe to discard.

**Reasoning**:
- The platform is single-author (`owned = true`), so `--force-with-lease`
  is safe (no one else's work gets clobbered).
- The 20 codeberg-only commits are from a defunct agent process that's
  no longer running; they should be considered stale.
- Option B is 1 minute vs Option C's 5-15 minutes of conflict resolution.
- After Option B, the daemon can resume normal 3-mirror syncs (still
  codeberg-only due to `.git` size, but at least it'll work).

**Secondary recommendation: Option A** (LFS migration) as a follow-up
goal. This addresses the underlying bloat and unlocks github + gitlab
mirrors for the platform. Not urgent — Option B unblocks the daemon.

**To verify the 20 codeberg-only commits are safe to discard**:
```bash
git log HEAD..codeberg/main --oneline
# Review the 20 commits. If they're all from a known-defunct agent
# process and contain no work you still want, Option B is safe.
```

If the 20 commits contain anything you want to keep, fall back to
Option C (rebase).

## Open questions and risks

1. **What's the actual state of codeberg's 20 commits?** I've described
   them as "from a defunct agent process" but haven't verified. Operator
   should run `git log HEAD..codeberg/main` in `/home/dracon/Dev/dracon-platform`
   before deciding.

2. **Will the 905-commit push fit in `push_op_timeout_secs = 900s`?**
   Unknown. The current PUSH_STUCK loop suggests pushes are slow on
   this repo. May need temporary bump to 1800s for the recovery push.

3. **What if Option B fails partway?** If the push times out after
   pushing 500 commits, codeberg has 500 of local's commits but not
   the other 405. Local still has all 905. The daemon's next push
   attempt would push the remaining 405 (a much smaller push). Likely
   self-healing but worth monitoring.

4. **Should I also fix the daemon's pull-fails-on-divergence logic?**
   That's a daemon Rust code change, which is out of scope for this
   goal. Could be a follow-up goal: "make daemon's auto_pull handle
   diverged branches with a fallback to --force-with-lease".

## Files of interest

- `/home/dracon/.dracon/utilities/sync/dracon-sync.toml` — global
  daemon config; raised `alert_unpushed_threshold` from 10 to 50
  (goal `mqu95usq-hbt73a`)
- `/home/dracon/.dracon/sync-notify/sync-notify.sh` — desktop notification
  path; raised throttle from 5min to 30min and added dedup-by-signature
  (goal `mqu95usq-hbt73a`)
- `/home/dracon/Dev/dracon-platform/.dracon/dracon-sync.toml` —
  per-repo override: `owned = true`, `exclude_remotes = ["github", "gitlab"]`
  (pre-existing from goal `mqqsyzyd-qkvna5`)
- `/home/dracon/Dev/dracon-utilities/docs/design/gitlab-storage-and-divergence-2026-06-23.md`
  — prior design doc that identified the divergence initially
- `/home/dracon/Dev/dracon-utilities/docs/design/big-repo-storage-strategy.md`
  — prior design doc with §10's LFS recommendation (Option A reference)

## Non-blocking observations

- The platform's `.git` is 19 GiB. Even after Option B, every future
  push will be slow (~900s timeout). This is a fundamental scaling
  problem; Option A (LFS) addresses it.
- The daemon's `auto_pull = true` strategy is incompatible with
  diverged branches. This is a daemon design issue, not a config issue.
  Fixing it requires a daemon Rust code change.
- The 3-mirror strategy is preserved (platform pushes to codeberg only,
  other 14 repos push to all 3 mirrors). No structural changes proposed
  by this doc.

## Decision

The operator must approve one of Options A/B/C/D (or an alternative).
This goal does NOT execute any of them. A follow-up goal will be
created once the operator decides.

If the operator does not respond within a week, the daemon will
continue to emit ALERT entries (now throttled to 30-min intervals by
the notification path) and the divergence will continue to grow.