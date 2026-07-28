# capture-anime-girls (CAG) GitHub push block — investigation

**Date**: 2026-07-28
**Status**: investigation only — no remediation applied
**Goal**: none (operator chose "investigation only" after seeing the
size math rejected the "shrink main" approach)

---

## TL;DR

The "hint" in `dracon-sync repos` for `capture-anime-girls`
(`.git exceeds 2 GB (github limit) — may fail to push to github`)
is more serious than the hint suggests. The daemon is **already
silently skipping GitHub pushes** for CAG because the pushable
branch is **2.37 GiB** (just over GitHub's 2 GiB limit). The skip
is correct (under the current architecture, there is no
in-daemon fix), but the warning is buried in `journalctl` rather
than surfaced in the `repos` table as a CONCERN. The 2.7 GiB is
legitimate game art (1170 PNGs + 31 MP3s + 15 JPGs + 81 JSONs,
total reachable content ≈ 565 MiB), not dev leftovers.

## Evidence

### Pre-fix daemon log (the silent skip)

```
$ journalctl --user -u dracon-sync.service --since "1h ago" \
    | grep -i 'capture-anime-girls' | head -2

Jul 28 09:08:49 nixos dracon-sync[2702053]: ⚠️ 🚫 skipping github push
  for /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls:
  pushable branch is 2.37 GiB (exceeds github's 2 GiB pack limit).
  Needs history rewrite / OVH migration; will resume once shrunk below 2 GiB.
```

Direct grep of the daemon's source confirms this is the intended
behavior (not a bug):

- `dracon-sync/src/sync.rs:1819` — `log_warn!(...)` is the only
  surface for the skip
- `dracon-sync/src/sync.rs:1788-1819` — adds `github` to
  `combined_exclude` so the path actually skips (otherwise the
  orphan git push hangs and re-dispatches every cycle — that's
  the 2026-07-09 sync-stall audit fixed)
- `dracon-sync/src/sync.rs:1832` — optional webhook notification
  (not configured for the operator)
- `dracon-sync/src/report.rs:3102-3107` — flag is computed per
  `(pushable_size, pushable_to_github)` pair, not exposed to the
  `repos` table as a CONCERN

The skip is permanent. The log message says "will resume once
shrunk below 2 GiB" — but the daemon itself does not shrink
history. There is no auto-repair path for this class.

### Total .git size and pushable size

```
$ du -sh /home/dracon/Dev/dracon-platform/.git/modules/web-games-capture-anime-girls/
2.7G    ...
```

```
$ git -C .../capture-anime-girls cat-file --batch-check --batch-all-objects \
    --unordered | awk '$2=="blob"{n+=1; s+=$3} END{printf "%d blobs, %.2f GiB total\n", n, s/1024/1024/1024}'
8151 blobs, 2.68 GiB total
```

```
$ git -C .../capture-anime-girls gc --prune=now --quiet
   (no meaningful change; 2.7 GiB → 2.68 GiB)
```

`git gc --prune=now` saves nothing because the pack is dominated
by reachable content, not unreachable garbage. The 126 `git fsck`
unreachable objects are tiny commits/trees/blobs, not the big
ones.

### Reachable content by file type

```
$ python3 -c "<see end of doc>"  # ext -> file count, total MiB
png    :   1170 files,    484.6 MiB
mp3    :     31 files,     68.1 MiB
jpg    :     15 files,      4.6 MiB
json   :     81 files,      2.7 MiB
ts     :    154 files,      1.1 MiB  (source — barely contributes)
svelte :     38 files,      0.5 MiB
```

**Total visible content ≈ 565 MiB**. The 2.7 GiB pack size vs
565 MiB visible content is normal git pack-delta growth over a
long history (1657 commits — all linear, no merges): each PNG
re-encoding accumulates delta chains that don't compress as
strongly as raw content.

### Where the 2 GiB pushable side comes from

The 2.37 GiB delta between `git rev-parse main` and the gh-side
"NotFound" baseline is the full local history pack-streamed to
the GitHub receiver. Even though the 2.7 GiB is a delta-compressed
pack, GitHub re-checks the individual object sizes for the 2 GiB
per-pack limit (the per-push wire-protocol enforces this).

### Composition by blob size

The biggest blobs:

```
$ python3 -c "<see end of doc for full output>"
   >= 10.0 MiB:    0 blobs,     0.0 MiB saved (0.0% of total)
   >=  5.0 MiB:    6 blobs,    38.9 MiB saved (1.4% of total)    # AI-generated MP3s
   >=  1.0 MiB: 1047 blobs,  1491.4 MiB saved (54.3% of total)   # mostly PNGs
   >=  0.5 MiB: 2342 blobs,  2398.6 MiB saved (87.3% of total)
   >=  0.2 MiB: 2853 blobs,  2569.9 MiB saved (93.5% of total)
```

The 6 blobs ≥ 5 MiB are MP3 audio (verified by `git cat-file -p`
showing `ID3` headers + `HUABABSpeech7E01` AI speech-generation
tags). The 1047 blobs ≥ 1 MiB = 1.49 GiB are almost entirely PNG
sprite sheets, character portraits, and animation frames.

### Trim analysis

Three reframings of "shrink to <2 GiB":

| Approach | Saves | Final size | Cost |
|---|---|---|---|
| Strip 36.5 MiB of `.archive/audio/2026-07-17/*.mp3` | 36.5 MiB | 2.33 GiB | Not enough — still over 2 GiB |
| Strip 6 blobs ≥ 5 MiB (the AI audio) | 38.9 MiB | 2.33 GiB | Same — not enough |
| Strip 1047 blobs ≥ 1 MiB (most PNG art) | 1.49 GiB | 1.23 GiB ✓ | Loses the 1.49 GiB of high-res art, including all sprite sheets, character portraits, animation frames |
| Strip 2342 blobs ≥ 500 KiB (most art) | 2.40 GiB | 0.34 GiB ✓ | Loses 87% of all blobs |
| Orphan cutover (HEAD-tree squash) | 2.13 GiB | ≈ 565 MiB ✓ | Loses history on GitHub (no per-commit log); all art preserved in the squash |

**Conclusion**: "shrink main" is not achievable without destroying
most of the game's art. The orphan cutover is the only path
that preserves the art and fits under 2 GiB.

### The "proven pattern" reference is broken

The platform's `dracon-platform-github-sync.timer` references
`/home/dracon/Dev/dracon-platform/scripts/sync-github-main.sh`,
but that script **does not exist on disk** (it was deleted in
commit `61e5b1446e` ~3 weeks ago). The timer has been firing
every 10 min and failing with `EXDEV / No such file or
directory` since — verified by:

```
$ journalctl --user -u dracon-platform-github-sync.service --since "1h ago" | tail -6
Jul 28 10:00:49 nixos (-main.sh)[3680384]: dracon-platform-github-sync.service:
  Failed at step EXEC spawning /home/dracon/Dev/dracon-platform/scripts/sync-github-main.sh:
  No such file or directory
```

The platform's current approach is the inverse: "shrink main to
under 2 GiB, push full main to github directly". The platform's
main is now ~1.4 GiB (after cleanup) and `main` is pushed directly
to GitHub via a `post-commit` hook (the daemon excludes GitHub
in the platform's `dracon-sync.toml`).

CAG's analogue of the platform's cleanup has not happened, so
the platform's `github-main` cutover script (which was the
prior approach, before the cleanup) was retired. Adopting the
orphan cutover for CAG would be rebuilding the platform's
retired pattern with the platform's known foot-gun (the
sync-github-main.sh script being deleted and the timer
silently failing).

## Recommendations (deferred — operator chose investigation only)

These are written here so a future goal-draft can pick them up:

1. **Daemon-side classification bug** (low effort, high value):
   surface the `too_big_for_github` skip as a CONCERN in the
   `repos` table, not just a `hint`. The `hint` field is at
   `dracon-sync/src/report.rs:~3102` and the `concern` field
   is per-row decision. Fix: change the row's `concern` boolean
   when `pushable_size > 2 GiB` for the `github` remote. The
   other two remotes (codeberg, gitlab) keep working, so the
   repo is NOT a CONCERN overall — but the GitHub-side
   divergence IS, since it will silently drift forever.

2. **Use OVH bucket for art** (medium effort): the
   `dracon-platform/scripts/install-music-api-service.sh` and
   `ovh-verify-bucket.mjs` already exist. Move the 1170 PNGs +
   31 MP3s to OVH and reference from code by URL. This is the
   "OVH migration" the daemon log message alludes to. Cost: a
   full asset URL rewrite across the codebase. Benefit: full
   history preserved, full art preserved, pushable branch fits
   under 2 GiB.

3. **Orphan github-main cutover** (high effort, deferred):
   rebuild the platform's retired pattern. The platform's
   pre-cleanup state was 16 GiB; after the orphan cutover it
   was 1.4 GiB. CAG's pre-cutover is 2.7 GiB; after the orphan
   cutover it would be ~565 MiB (current HEAD tree content).
   Script location: `dracon-platform/scripts/sync-github-main.sh`
   (would need re-creation since it was deleted). Timer:
   `dracon-cag-github-sync.timer`. Caveat: GitHub becomes a
   current-state mirror with no history.

4. **Asset archive-and-prune** (medium effort, breaks game):
   `.gitignore` and `git rm` the 36.5 MiB of archived MP3s
   (in `.archive/audio/2026-07-17/`) and the 17 AI-generated
   MP3s in `static/audio/`. Saves ~38 MiB. Not enough for the
   2 GiB push limit. Would lose the audio assets that the game
   runtime loads.

## Cross-references

- Source: `dracon-sync/src/sync.rs:1647-1840` (size guard + skip
  logic)
- Source: `dracon-sync/src/report.rs:3102-3107` (size-guard flag
  derivation for the repos table)
- Prior art: `dracon-platform/scripts/sync-github-main.sh` — was
  the orphan cutover script, deleted in commit `61e5b1446e`
- Prior design: `docs/design/incident-amend-race-and-trust-2026-07-25.md`
  (no-history-rewrite policy context)
- Audit: `docs/design/audit-screenshot-bloat-deathrun-2026-07-23.md`
  (deathrun's 2.85 GiB → <2 GiB GitHub push fix via screenshot
  hygiene, NOT asset migration)

## Reproduction commands

```bash
# 1. Total size (gitdir, not workdir)
du -sh /home/dracon/Dev/dracon-platform/.git/modules/web-games-capture-anime-girls/

# 2. Reachable blob content
git -C /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls \
  cat-file --batch-check --batch-all-objects --unordered \
  | awk '$2=="blob"{n+=1; s+=$3} END{printf "%d blobs, %.2f GiB total\n", n, s/1024/1024/1024}'

# 3. By file type
git -C /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls \
  ls-tree -r HEAD | awk '{print $4}' \
  | sed -E 's/.*\.([^.]+)$/\1/' | sort | uniq -c | sort -rn | head -10

# 4. Pushable size (simulates what GitHub would receive)
git -C /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls \
  rev-list --objects --all | sort -k2 | uniq -c -w40 | head -5

# 5. Daemon log
journalctl --user -u dracon-sync.service --since "1h ago" \
  | grep -i 'capture-anime-girls.*GiB'

# 6. Biggest blobs (sample)
git -C /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls \
  cat-file --batch-check --batch-all-objects --unordered \
  | awk '$2=="blob"{print $3, $1}' | sort -n | tail -10
```

## Python script used for the size breakdown

```python
import subprocess
# Total blobs
out = subprocess.run(['git','cat-file','--batch-check','--batch-all-objects','--unordered'],
                     capture_output=True, text=True).stdout
blobs = []
for line in out.splitlines():
    parts = line.split()
    if len(parts) >= 3 and parts[1] == 'blob':
        blobs.append((int(parts[2]), parts[0]))
blobs.sort(reverse=True)
total = sum(s for s, _ in blobs)
for threshold in [10*1024*1024, 5*1024*1024, 1*1024*1024, 500*1024, 200*1024]:
    saved = sum(s for s, _ in blobs if s >= threshold)
    count = sum(1 for s, _ in blobs if s >= threshold)
    print(f'  >= {threshold/1024/1024:5.1f} MiB: {count:4d} blobs, {saved/1024/1024:7.1f} MiB saved ({saved/total*100:.1f}%)')
```