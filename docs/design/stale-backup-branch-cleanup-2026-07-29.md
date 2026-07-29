# Stale backup-branch cleanup — 2026-07-29

> **Trigger**: operator review of the new v0.113.8/v0.113.9
> rich-table SIZE column: "we are seeing quite a concern with
> sizes here but deathrun is shown as clean". Investigation
> confirmed deathrun's ✅ CLEAN was *correct* (main pushable
> 0.25 GiB) and its 4.08 GiB SIZE was three dead backup
> branches from the July rewrite era. Operator authorized
> fleet-wide **bundle + delete + gc** (2026-07-29).

## What the SIZE column vs STATUS distinction means

| Metric | Source | Meaning |
|---|---|---|
| **SIZE** | `git count-objects -v` (on-disk gitdir) | Local disk footprint incl. garbage, dead branches, old pack residue |
| **STATUS** (CONCERN/CLEAN) | pushable uncompressed blob sum on the push branch | Whether github will accept the next push (2 GiB pack limit) |

A repo can be big-on-disk and healthy-to-push (deathrun:
4.08 GiB disk / 0.25 GiB pushable → ✅ CLEAN + Yellow SIZE).
A repo can be small-on-disk and broken-to-push (junk-runner
post-cleanup: 1018 MiB disk / 3.79 GiB pushable → ❌ CONCERN
+ Red SIZE via `pack_too_large`). The v0.113.9 SIZE-color
semantics (Red iff `pack_too_large`) encode exactly this.

## Cleanup executed (operator-approved)

Sequence per repo: `git bundle create` stale branches →
`git bundle verify` → `git branch -D` →
`git reflog expire --expire=now --all` → `git gc --prune=now`.
Daemon stopped during the operation, restarted after.
Bundles (recovery archives) live **outside** any gitdir at:

```
/home/dracon/dracon/backups/stale-branch-bundles-20260729/
├── avid.bundle                 736K
├── dracon-platform.bundle      68M
├── capture-anime-girls.bundle  1019M
├── darklord.bundle             649M
├── deathrun.bundle             2.8G
├── hellhunter.bundle           310M
├── junk-runner.bundle          1.2G
└── neonbreak.bundle            481M
```

Restore procedure (if ever needed):
`git -C <repo> fetch <bundle> 'refs/heads/*:refs/heads/restored-*'`.

### Branches deleted (all local-only except where noted)

| Repo | Branches | Era |
|---|---|---|
| avid | `backup/pre-sync-largeblob-fix-1780417168` | Jun 2 (pre-AGENTS.md) |
| dracon-platform | `backup/pre-sync-largeblob-fix-1784077563` | Jul 14 |
| capture-anime-girls | `backup/pre-sync-largeblob-fix-1784113072` | Jul 15 |
| darklord | `backup/pre-sync-largeblob-fix-1784113291` | Jul 15 |
| deathrun | `backup/pre-deathrun-rewrite-1784804212`, `backup/pre-sync-largeblob-fix-1784110476`, `backup/pre-sync-largeblob-fix-1784111463` (**also on github+gitlab — see below**), `daemon-standalone`, `rebirth-20260723` | Jul 15/23 |
| hellhunter | `backup/pre-sync-largeblob-fix-1784113416` | Jul 15 |
| junk-runner | `backup/pre-sync-largeblob-fix-1784112643` (**`preserve/*` stash-rescues deliberately kept**) | Jul 15 |
| neonbreak | `backup/pre-sync-largeblob-fix-1784113582` | Jul 15 |

Also removed: deathrun's orphaned `refs/remotes/restore/*`
tracking refs (the `restore` remote was deleted during the
2026-07-23 cutover recovery but its refs lingered) and a
stale `tmp_obj_*` file in deathrun's object store.

## Results (fresh measurement after clearing `repos-size-cache.json`)

| Repo | Before | After | Reclaimed |
|---|---:|---:|---:|
| deathrun | 4.07 GiB | **1.83 GiB** | 2.25 GiB (1.5 GiB more pending remote deletion) |
| junk-runner | 1.40 GiB | **1018 MiB** | ~0.4 GiB |
| darklord | 1000 MiB | **784 MiB** | ~216 MiB |
| capture-anime-girls | 2.52 GiB | **2.34 GiB** | ~190 MiB |
| neonbreak | 658 MiB | **642 MiB** | ~16 MiB |
| hellhunter | 1.36 GiB | **1.35 GiB** | ~11 MiB |
| avid | 56.6 MiB | **57.8 MiB** | — (repack growth, branch was tiny) |
| dracon-platform (parent) | 11.44 GiB | **11.51 GiB** | — (see below) |

**Total immediate reclaim: ~3.1 GiB.** Note the daemon's
SIZE column lags up to 1h (`REPO_SIZE_CACHE_TTL_SECS = 3600`,
cache at `~/.dracon/utilities/sync/repos-size-cache.json`,
survives restarts) — deleting the cache file forces
re-measurement.

## Two lessons

### 1. Don't delete local remote-tracking refs before the remote branch is deleted

deathrun's `backup/pre-sync-largeblob-fix-1784111463` is
published on **both github and gitlab**. Deleting the local
`refs/remotes/*/backup/...` refs + gc would prune the objects
— and the daemon's next `git fetch` would **re-download ~2
GiB** and re-bloat the gitdir. Correct order:

1. Operator deletes the branch on the remotes (below)
2. `git fetch --prune` (drops local tracking refs)
3. `git gc --prune=now` → deathrun lands at ~0.3 GiB

### 2. dracon-platform's own object store is 11.5 GiB — separate problem

The parent's SIZE (12 GiB) is *not* mostly submodule
gitdirs (those live in `.git/modules/*` and aren't counted
by the parent's `count-objects`). The parent's own history —
the pre-submodule era when game files lived directly in it —
carries ~11.5 GiB. main pushable is under 2 GiB (✅ CLEAN),
so this is disk-capacity only. Reclaim = filter-repo on the
parent, a much bigger operation; **deferred** — the Yellow
SIZE cell is the intended capacity-planning signal.

## Operator runbook: remote branch deletions (pending)

These need `DRACON_ALLOW_REWRITE=1` (warden pre-push hook
blocks branch deletions) and are **operator-gated** per
AGENTS.md "For HUMAN operators". Gitlab branch protection
covers only main/master, so these deletes are allowed
remotely.

```bash
# 1. deathrun's published backup branch (github + gitlab) — ~1.5 GiB reclaim
cd /home/dracon/Dev/dracon-platform/web/games/wip/deathrun
DRACON_ALLOW_REWRITE=1 git push github --delete backup/pre-sync-largeblob-fix-1784111463
DRACON_ALLOW_REWRITE=1 git push gitlab --delete backup/pre-sync-largeblob-fix-1784111463
git fetch --prune github && git fetch --prune gitlab
git gc --prune=now

# 2. daemon-standalone on gitlab for 5 games (superseded 2026-07-02 design; tiny)
for g in capture-anime-girls darklord hellhunter neonbreak; do
  git -C /home/dracon/Dev/dracon-platform/web/games/wip/$g \
    push origin --delete daemon-standalone   # prefix with DRACON_ALLOW_REWRITE=1
  git -C /home/dracon/Dev/dracon-platform/web/games/wip/$g fetch --prune origin
done
# junk-runner's daemon-standalone rides along with its pending
# Scenario B filter-repo (docs/design/junk-runner-history-rewrite-2026-07-28.md)

# 3. Force the daemon to re-measure sizes immediately:
rm /home/dracon/.dracon/utilities/sync/repos-size-cache.json
dracon-sync repos
```

## Standing policy note

Per AGENTS.md, any *new* `backup/pre-sync-largeblob-fix-*`
branch is a fault requiring operator review. The 2026-07-15
branches cleaned here were all from the pre-SYNC-H6-fix era
(v0.113.3 fixed the auto-repair to use proper bundle backups
+ `--force-with-lease`). Post-v0.113.3, the daemon's
auto-repair creates bundle-file backups in `backup_dir`
(`/home/dracon/dracon/backups`) instead of in-repo branches
for the recovery path, so this class of disk-bloat should
not recur — but if a `backup/*` branch appears in `git
branch` output, treat it as a review signal, not noise.
