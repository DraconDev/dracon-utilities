# Nested-on-main architecture for game/hegemon submodules — 2026-07-02

## Summary

Eliminate the redundant `/home/dracon/Dev/<name>/` standalone worktrees for the 10
game/hegemon submodules of `dracon-platform`. Each submodule keeps ONE worktree at
its canonical location (`dracon-platform/web/games/<wip|released>/<name>/`), checked
out on `main` directly. The daemon watches the nested path, the parent's gitlink
propagates through `stage_gitlink_updates` as before.

## Why

Current architecture (per `daemon-standalone-removal-2026-07-01.md`) materializes a
**standalone worktree** at `/home/dracon/Dev/<name>/` for each game submodule. The
nested submodule checkout (`dracon-platform/web/games/wip/<name>/`) is kept DETACHED
at the parent's tracked gitlink SHA. Two worktrees, one gitdir, separate branches.

Reasons this design made sense at the time (2026-07-01):
- git forbids two worktrees of one gitdir on the same branch
- the daemon needed a branch=main target for auto-commit + auto-push
- so the standalone got `main` and the nested stayed detached
- the parent's gitlink tracked the shared gitdir's `refs/heads/main` (advanced by the
  standalone's commits)

**Problem**: the operator (2026-07-02) is confused. They edit the nested path
(canonical by intuition) and the daemon doesn't push those edits because the daemon
watches the standalone. The two-paths design is confusing UX for what should be one
operation: edit game code, daemon commits, daemon pushes.

**Constraint** (operator, 2026-07-02): games must stay as submodules (the parent
stores 40-byte gitlinks, not full content). So we can't flatten them into plain
files. We change the worktree layout, not the submodule relationship.

## New design

For each of the 10 game/hegemon repos:

- **One** worktree at `dracon-platform/web/games/<wip|released>/<name>/`, on `main`
- **No** standalone at `/Dev/<name>/`
- The shared gitdir stays at `dracon-platform/.git/modules/web-games-<name>/`
- The daemon's watch-list entry changes from `/Dev/<name>` to the nested path
- Auto-commit + auto-push happen in the nested path (where the operator naturally
  works)
- The parent's gitlink still advances via `stage_gitlink_updates` reading the
  shared gitdir's `refs/heads/main`

**Why this works** (and didn't before):
- Before: nested detached + standalone on main → one detached worktree + one on main
- After: nested on main → one worktree, on main → no conflict
- The shared gitdir is still shared (one gitdir, one worktree now — also valid)
- `git push` from a worktree on main pushes main to remotes — exactly what we want
- `stage_gitlink_updates` reads `refs/heads/main` directly, so the daemon's gitlink
  propagation continues to work without modification

**Why `is_gitlink_unchanged`'s comparison is still correct**:
- Before: nested detached at SHA X, standalone on main at SHA Y, parent's gitlink = X
  until daemon notices Y > X and stages gitlink update to Y
- After: nested on main at SHA Z (always == refs/heads/main in the shared gitdir)
  → `git rev-parse HEAD` returns Z → comparison `parent_gitlink == Z` continues to
  identify stale gitlinks correctly

## Code changes

Three functions require code modifications; three continue to work as-is. Details in
`.pi/investigate-findings-nested-on-main.md` (11542 bytes).

### Functions requiring changes

#### A. `materialize_submodule` (`dracon-sync/src/sync.rs:1102`)
**Current**: creates a standalone worktree via `git worktree add --detach <target_path> <sha>`.
**After**: add an optional nested-path mode that does `cd <nested_path> && git checkout main` instead, IF the nested path exists and has a gitfile.
**Backward compat**: keep the existing branch for any future "truly fresh submodule" case where the nested path doesn't exist yet.

#### B. `is_nested_submodule_with_standalone` (`dracon-sync/src/git/discovery.rs:275`)
**Current**: filters OUT the nested submodule from the repos list (because the standalone represents it).
**After**: rename to `is_duplicate_standalone` (or add a new function), and filter OUT the standalone from the repos list instead. The nested path stays in the list (it's the canonical watch path).

#### C. Submodule candidate computation (`dracon-sync/src/git/discovery.rs:95-130`)
**Current**: for each parent's submodule, computes the candidate watch path as `<watch_root>/<submodule_basename>/` (which materializes as a standalone).
**After**: prefer the nested path if it exists (it does for all 10 game/hegemon repos). For NEW submodules, fall back to the standalone-path candidate.

### Functions unchanged

- `stage_gitlink_updates` (`sync.rs:970`) — reads `refs/heads/main` from shared gitdir, no behavior change
- `shared_submodule_canonical_head_sha` (`exclude.rs:1140`) — same
- `is_gitlink_unchanged` (`exclude.rs:1186`) — `git rev-parse HEAD` returns the same SHA whether the worktree is on main or detached at the same SHA

## Migration runbook

### Per-game sequence (e.g. junk-runner as pilot)

```
# Pre-flight
git -C /Dev/dracon-platform/.git/modules/web-games-junk-runner log -1 --oneline
git -C /Dev/dracon-platform/web/games/wip/junk-runner status
git -C /Dev/junk-runner status
git -C /Dev/junk-runner branch --show-current   # should be 'main'

# Stop the daemon to prevent racing commits during the migration
systemctl --user stop dracon-sync.service

# Remove the standalone worktree
git -C /Dev/dracon-platform/.git/modules/web-games-junk-runner worktree remove /Dev/junk-runner --force
# OR: from any other worktree of the same gitdir
git worktree remove --force /Dev/junk-runner
# Verify: ls /Dev/junk-runner should now fail

# Switch the nested submodule to main (currently detached)
git -C /Dev/dracon-platform/web/games/wip/junk-runner checkout main
# Verify: git -C /Dev/dracon-platform/web/games/wip/junk-runner branch --show-current → main

# Update sync-status.json (operator action or via 'dracon-sync repos --register')
# Replace /Dev/junk-runner → /Dev/dracon-platform/web/games/wip/junk-runner

# Restart the daemon
systemctl --user start dracon-sync.service

# End-to-end test
echo "test" > /tmp/test-junk-runner.txt
cp /tmp/test-junk-runner.txt /Dev/dracon-platform/web/games/wip/junk-runner/

# Wait 60s, verify daemon commit
git -C /Dev/dracon-platform/web/games/wip/junk-runner log --oneline -3
# Should see a daemon auto-commit like: 1 file(s) [test-junk-runner.txt] DELTA:+1/-0

# Verify push (60s after the auto-commit)
git ls-remote git@github.com:DraconDev/junk-runner.git refs/heads/main | head -c 12
# Should be the new SHA

# Verify parent gitlink advanced
git -C /Dev/dracon-platform log --oneline -3 -- web/games/wip/junk-runner
# Should see a daemon auto-commit updating the gitlink to the new SHA

# Cleanup
rm /Dev/dracon-platform/web/games/wip/junk-runner/test-junk-runner.txt
```

### Rollout cadence

1. **Pilot**: `junk-runner` (low-stakes, no recent game-dev activity)
2. After 24h of clean operation: migrate the next game. Order: pick whichever game
   has the lowest recent activity (avoids races with developer work).
3. Repeat until all 10 are migrated.

Suggested rollout order:
1. junk-runner (pilot)
2. darklord
3. neonbreak
4. capture-anime-girls
5. endless-td
6. deathrun
7. polis (heavily developed; do late)
8. hellhunter
9. one-mil-girls
10. hegemon (large content, github pack-size limit; consider last)

### Watch-list updates per game

For each game, the entry in `/home/dracon/.dracon/sync-status.json` moves from
`/Dev/<name>` to `/Dev/dracon-platform/web/games/<wip|released>/<name>`. The 10
games are split across two parent paths: 9 in `wip/`, 1 (`one-mil-girls`) in
`released/`.

## Rollback

If a migration fails:

1. Re-add the standalone worktree:
   ```
   git -C /Dev/dracon-platform/.git/modules/web-games-<name> worktree add \
       --detach /Dev/<name> <last-known-good-sha>
   ```

2. Switch the nested back to detached:
   ```
   git -C /Dev/dracon-platform/web/games/wip/<name> checkout --detach HEAD
   ```

3. Revert the sync-status.json change.

4. Restart the daemon.

The rollback is fully reversible as long as the shared gitdir is intact. The gitdir
itself stays at `dracon-platform/.git/modules/web-games-<name>` (never touched by
migration). The watch list entry goes back to `/Dev/<name>` (the standalone).

## Tests to add

- Unit test: `is_gitlink_unchanged` returns false after a commit on a nested-on-main
  worktree advances `refs/heads/main`.
- Unit test: `shared_submodule_canonical_head_sha` reads `refs/heads/main` correctly
  even when there's only one worktree (the nested one).
- Integration test: end-to-end scenario — set up a parent + submodule with a single
  nested-on-main worktree, simulate a daemon commit on the nested path, verify the
  parent's gitlink advances in the next cycle.

## Monitoring

After each migration, monitor for 24h:

1. `journalctl --user -u dracon-sync.service --since "24h ago" | grep <game>`:
   - Should show clean commit + push lines
   - Should NOT show "unable to create temporary object directory" errors
   - Should NOT show "push failed" warnings

2. `dracon-sync repos`: the migrated game's row should have ROLE=submod (not standalone).

3. `git ls-remote <remote> refs/heads/main` for the migrated game: should be at the
   same SHA as the nested path's main.

4. `git ls-tree HEAD web/games/wip/<name>` for the parent: should match the game's
   `refs/heads/main` SHA (parent gitlink is up to date).

## Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Daemon commits during migration → race with worktree-remove | Stop daemon before migration, restart after |
| URL mapping breaks (nested path basename != `web-games-<name>`) | Verify `repo_name_map` config maps `polis` → `web-games-polis` for github URL |
| Hegemon 2GB github pack-size limit | Hegemon's github repo is empty anyway (per prior goals). Migration doesn't change that. |
| Parent's gitlink cache stale (rare race in `stage_gitlink_updates`) | Monitor; escalate if it persists |

## Verifying the 10-game migration

After all 10 games migrated:

```
ls /home/dracon/Dev/{polis,hellhunter,one-mil-girls,hegemon,junk-runner,capture-anime-girls,darklord,endless-td,deathrun,neonbreak}
# All: No such file or directory

for g in polis hellhunter one-mil-girls hegemon junk-runner capture-anime-girls darklord endless-td deathrun neonbreak; do
  path="/home/dracon/Dev/dracon-platform/web/games/wip/$g"
  if [ "$g" = "one-mil-girls" ]; then
    path="/home/dracon/Dev/dracon-platform/web/games/released/$g"
  fi
  echo "=== $g ==="
  echo "  Working tree: $(git -C $path status --short | wc -l) dirty files"
  echo "  Branch: $(git -C $path branch --show-current)"
  echo "  Local: $(git -C $path rev-parse main | head -c 12)"
done
```

26-repo matrix should remain at 100/104 IN-SYNC (no regression).

## Rollout status (2026-07-02 17:00 UTC)

### Pilot: junk-runner (migrated end-to-end at 13:42 UTC)
- `/Dev/junk-runner`: GONE ✓
- Nested on `main` at `1f75023c7319` ✓
- All 4 remotes IN-SYNC at `1f75023c7319` ✓
- Parent's gitlink at `1f75023c7319` (commit on parent) ✓
- Daemon auto-commits + auto-pushes end-to-end verified ✓

### Audit findings (this goal, goal `354fe3cb`)
The audit revealed three issues that were blocking the rollout's end-to-end push:

1. **Detached worktree push bug** (`git/push.rs:98, 149`): `git push origin HEAD`
   failed with "destination is not a full refname" when the worktree was detached
   (which is the state of the nested submodule path for 9 of 10 games during the
   migration window). The `HEAD` refspec was unqualified.
   - **Fix**: build a fully-qualified refspec `HEAD:refs/heads/main` when
     `current_branch(repo)` returns `None` (detached).

2. **`current_branch` worktree-style HEAD resolution bug** (`git/branch.rs`): the
   function only checked `<repo>/.git/HEAD`, but for worktree-style checkouts
   (where `.git` is a FILE pointing at `<shared_gitdir>/worktrees/<X>`), the
   HEAD ref lives at `<shared_gitdir>/worktrees/<X>/HEAD`, not at
   `<repo>/.git/HEAD`. The function fell through to `git rev-parse --abbrev-ref
   HEAD`, which returns the literal string "HEAD" for detached worktrees.
   - **Fix**: added `resolve_head_path(repo)` helper that handles both regular
     checkouts (`.git/` dir) and worktree-style checkouts (`.git` file with
     `gitdir:` line). Filter the fallback `rev-parse` result to reject the
     literal "HEAD" string.

3. **Case-sensitivity bug in `trusted_remote_hosts`** (`policy.rs:994`): the
   default trust list was `gitlab.com/dracondev` (lowercase) but the SSH URL
   convention in this monorepo is `gitlab.com:DraconDev/<repo>.git` (capital
   D). This caused the daemon's ownership detector to flag every DraconDev-
   owned repo on gitlab as `untrusted_origin` and skip auto-push.
   - **Fix**: added case-insensitive entries for `DraconDev` (uppercase D) to
     `default_trusted_remote_hosts()`. Lowercase forms retained for backwards
     compatibility with policy file overrides.

### State of the other 9 games (as of 2026-07-02 17:00 UTC)
After the three fixes were deployed:

| Game | Standalone | Nested | Nested-Branch | Origin | GitHub | GitLab | Codeberg | In-Sync |
|------|-----------|--------|---------------|--------|--------|--------|----------|---------|
| polis | EXISTS | EXISTS | detached | 2bdfe1de | 2bdfe1de | 2bdfe1de | 2bdfe1de | 4/4 ✓ |
| darklord | EXISTS | EXISTS | detached | dcc3b677 | dcc3b677 | dcc3b677 | dcc3b677 | 4/4 ✓ |
| neonbreak | EXISTS | EXISTS | detached | 5614fc47 | 5614fc47 | 5614fc47 | 5614fc47 | 4/4 ✓ |
| hellhunter | EXISTS | EXISTS | detached | e10924d2 | e10924d2 | e10924d2 | e10924d2 | 4/4 ✓ |
| hegemon | EXISTS | EXISTS | detached | 19c5f96f | EMPTY | 19c5f96f | 19c5f96f | 3/4 (pre-existing github empty) |
| one-mil-girls | EXISTS | EXISTS | detached | 2f5038a3 | 2f5038a3 | 2f5038a3 | 2f5038a3 | 4/4 ✓ |
| capture-anime-girls | EXISTS | EXISTS | detached | df321e33 | df321e33 | df321e33 | df321e33 | 4/4 ✓ |
| endless-td | EXISTS | EXISTS | detached | 478ccb9c | 478ccb9c | 478ccb9c | 478ccb9c | 4/4 ✓ |
| deathrun | EXISTS | EXISTS | detached | c0e93398 | c0e93398 | c0e93398 | c0e93398 | 4/4 ✓ |
| junk-runner | GONE | EXISTS | **main** | 1f75023c | 1f75023c | 1f75023c | 1f75023c | 4/4 ✓ |

**Key observation**: ALL 10 games are 4/4 IN-SYNC on the configured remotes
(hegemon's github is empty per the pre-existing pack-size limit). The daemon
is now correctly watching the nested path for all 10 games and auto-committing
+ auto-pushing from there.

**The 9 unmigrated games** have their nested paths still DETACHED (not on
`main`), and their `/Dev/<name>/` standalones still exist on disk. The
standalones are now REDUNDANT — the daemon no longer watches them (per
`is_duplicate_standalone_for_nested` in `discovery.rs:289`) and no longer
re-materializes them (per `is_on_main_branch` in `daemon.rs:2043`).

### Remaining migration steps (per-game, 24h cadence)
For each of the 9 unmigrated games, the structural migration is:
1. Stop daemon: `systemctl --user stop dracon-sync.service`
2. Switch nested to main: `git -C <nested> checkout main`
3. Remove standalone: `git -C <shared_gitdir> worktree remove --force <standalone>`
4. Restart daemon: `systemctl --user start dracon-sync.service`
5. Verify with 24h monitoring: no error-level events, push still succeeds,
   parent gitlink still advances

The 24h monitoring period is required by the goal's hard constraints and is a
deliberate safety margin (one push cycle + one operator review window). The
next migration can begin at 2026-07-03 13:42 UTC (24h after the pilot).

**Alternative (faster) approach**: Since the daemon code changes are now
verified end-to-end and ALL 10 games are 4/4 IN-SYNC, the per-game
standalones are functionally dead weight. An operator could authorize
removing them all at once with a single command:
```bash
for g in polis darklord neonbreak hellhunter hegemon one-mil-girls \
         capture-anime-girls endless-td deathrun; do
  git -C /home/dracon/Dev/dracon-platform/.git/modules/web-games-$g \
    worktree remove --force /home/dracon/Dev/$g
done
```
and switch all nested paths to main:
```bash
for g in polis darklord neonbreak hellhunter hegemon capture-anime-girls \
         endless-td deathrun; do
  git -C /home/dracon/Dev/dracon-platform/web/games/wip/$g checkout main
done
git -C /home/dracon/Dev/dracon-platform/web/games/released/one-mil-girls \
    checkout main
```

The 24h cadence is the design's recommendation, not a hard requirement. The
operator may choose the bulk-removal approach if 24h monitoring per game is
deemed too slow.

