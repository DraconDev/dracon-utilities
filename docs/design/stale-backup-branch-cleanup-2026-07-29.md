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

## UNEXPECTED OUTCOME: junk-runner ❌ CONCERN cleared — the guard was over-conservative

After the gc, junk-runner's gitdir dropped under 2 GiB, which
flipped `github_pack_too_large`'s fast path
(`dracon-sync/src/git/mod.rs:48-53`: `size < 2 GiB → false`)
and cleared the PACK_SIZE_WARNING concern. Ground-truth
verification:

| Metric | Value | Under github's 2 GiB limit? |
|---|---:|:---:| |
| Whole-branch uncompressed blob sum (the guard's metric) | 3.79 GiB | ❌ (as measured) |
| Delta to github (96 commits behind) — compressed pack | **14.77 MiB** | ✅ trivially |
| Full-history compressed pack (fresh-remote scenario) | **736 MiB** | ✅ comfortably |

**The guard's uncompressed-blob-sum metric is a false proxy
for compressible content.** junk-runner's bloat is JSONL
text (`active.jsonl` versions) which delta-compresses
~5:1; the 3.79 GiB of blobs are *already on github* from
incremental pushes before the guard existed (v0.112.38 era).
Github's actual limit applies to the **compressed incoming
pack**, which for junk-runner has never exceeded ~1 GiB.
Contrast deathrun-in-July: PNG screenshots are
incompressible, so its 2.85 GiB uncompressed genuinely
shipped ~2.85 GiB and github really did reject it.

**Implications:**

1. **junk-runner's github sync is restored** — the next
   natural commit will push the 14.77 MiB delta (daemon
   only attempts github pushes when new commits exist; the
   last skip fired 03:11:46, matching the last commit).
2. **The Scenario B bulk filter-repo** (`docs/design/
   junk-runner-history-rewrite-2026-07-28.md`) is demoted
   from "required to unblock github" to **optional hygiene**
   (every fresh clone still carries ~736 MiB of dead scratch
   JSONL; the rewrite shrinks that to ~250 MiB). Still
   recommended, no longer urgent.
3. **Daemon guard improvement candidate (v0.113.10?)**:
   `github_pack_too_large` should measure the
   **delta-vs-remote compressed pack** (`git pack-objects
   --revs --stdout main --not --remotes=<target> | wc -c`)
   rather than the whole-branch uncompressed blob sum. This
   handles both cases correctly: fresh remote → delta =
   whole branch (deathrun case caught); incremental → only
   the delta (junk-runner case cleared). Alternatively the
   cheap fix: estimate compressed size as the gitdir pack
   size attributable to the branch. Filed for operator
   decision — the current guard is safe-but-noisy
   (false-positive direction only).
4. **capture-anime-girls' ❌ CONCERN stands** — its bloat
   is PNGs (incompressible), 2.34 GiB gitdir is a faithful
   proxy; Option A filter-repo + OVH migration unchanged.

## Remote cleanup — EXECUTED 2026-07-29 (operator-authorized, `DRACON_ALLOW_REWRITE=1`)

Actual findings differed from the initial runbook:

- **deathrun `backup/pre-sync-largeblob-fix-1784111463`**: was
  on **gitlab only** (github never had it). Deleted on
  gitlab, `fetch --prune`, final gc → **deathrun 4.07 GiB →
  237 MiB** (94% reclaimed; final state is the 0.25 GiB
  pushable main + normal overhead).
- **`daemon-standalone` on the 5 games**: already deleted on
  gitlab long ago (2026-07-08 materialization removal era) —
  only the local tracking refs were stale. `fetch --prune`
  dropped them; no remote deletion needed.
- No other stale remote branches found on the audited repos.

Remaining fleet state after full cleanup: 30 CLEAN /
3 ACTIVE / 2 WARN / 1 CONCERN (capture-anime-girls —
genuine, PNG bloat; Option A filter-repo still pending
operator authorization).

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
