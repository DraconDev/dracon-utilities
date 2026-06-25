# Big-Repo Storage Strategy

**Status**: Investigation / decision-ready design doc
**Date**: 2026-06-25
**Author**: DraconDev (pi-authored under operator review)
**Goal**: `mqtjb7sl-sbbbvz`
**Prior context**: `gitlab-storage-and-divergence-2026-06-23.md` (the immediate precursor — solved the gitlab and github push failures for the platform via `exclude_remotes`).

## 1. Problem statement

The operator's multi-mirror sync system (dracon-sync daemon) is well-architected:
**one local repo, three independent remotes** (github + gitlab + codeberg) for
resilience. As of 2026-06-25 this works for 17 of 18 watched repos.

The strategy breaks at the storage boundary. github's free personal tier
recommends **≤ 5 GB per repo** (soft cap; pushing over it returns HTTP 500).
gitlab's free tier enforces **≤ 10 GiB per project** (hard pre-receive hook
rejection). codeberg's free tier has no documented per-repo limit. When a
repo's `.git/` grows past one of those limits, that mirror fails.

This doc investigates **3 approaches** for handling the size growth on
all 9 repos currently over 1 GiB, and ends with a per-repo recommendation
and a recommended next-step POC.

## 2. Size survey (2026-06-25, raw data at `/tmp/big-repo-survey.txt`)

### 2.1 Per-repo size breakdown

| repo                          | .git (MiB) | worktree (MiB) | total (MiB) | top 3 worktree prefixes (MiB)              |
|-------------------------------|-----------:|---------------:|------------:|--------------------------------------------|
| dracon-platform               |  19 231    |  97 420        | 116 651     | target/83 763; web/13 649; apis/8          |
| dracon-code                   |     221    |  26 848        |  27 069     | target/26 813; examples/30; docs/3         |
| rust-ai-web-auto              |      15    |  23 571        |  23 586     | target/23 567; tests/1; src/1              |
| avid                          |      52    |  15 992        |  16 044     | target/14 861; fuzz-uploader/937; output/175|
| pully-fully-pull-based-fleet-reconciler | 39 |  7 833    |   7 872     | target/2 656; pully/2 535; fully/2 360     |
| ai-auto-writer                |     124    |   6 347        |   6 471     | target/6 079; authors/140; test-books/65   |
| quick-draw-screenshot-clipboard|       4   |   4 067        |   4 071     | target/4 067; src/1; contrib/1             |
| dracon-utilities              |      30    |   3 506        |   3 536     | dracon-sync/3 502; scripts/1; evidence/1   |
| browser-extensions-shared     |     528    |   1 795        |   2 323     | node_modules/1 551; extensions/237; docs/4 |
| search-daemon                 |       1    |     750        |     751     | (well-distributed)                         |
| dracon-libs                   |      85    |     118        |     203     |                                            |
| dracon-strategy               |       3    |       3        |       6     |                                            |
| pi-plugins                    |       1    |       1        |       2     |                                            |

**Threshold buckets**:
- **Over 1 GiB total** (9 repos): need per-repo decision.
- **Over 5 GiB total** (8 repos): same set, plus audit of `.git` size separately.
- **Over 10 GiB total** (3 repos): platform, dracon-code, rust-ai-web-auto.

### 2.2 The two distinct size problems

This is the key insight: there are **two different problems** that look the
same in `du -sh` output.

**Problem A — bloated worktree, small `.git/`** (the safe kind):
The huge `target/`, `node_modules/`, `output/`, `fuzz-uploader/` directories
are untracked build artifacts. `.git/` is tiny (15-221 MiB) and the mirror
push works fine. The repo just looks big on disk.

Repos in this category: **dracon-code, rust-ai-web-auto, avid, pully,
ai-auto-writer, quick-draw-screenshot-clipboard, search-daemon**.

**Problem B — large `.git/` from tracked binaries** (the dangerous kind):
Large binary or text assets are committed and live in git history. `.git/`
grows because every change to a large file inflates the pack. Mirror push
fails when `.git/` exceeds the remote's storage cap.

Repos in this category: **dracon-platform** (.git = 19 GB, mostly
~17 MB MP3 audio assets committed across game history) and
**browser-extensions-shared** (.git = 528 MB, mostly 28 MB JSON in
test fixtures).

### 2.3 What's bloating the platform's `.git/`

The 19 GB `.git/objects/pack/` is dominated by **3-17 MB MP3 audio files**
committed as game assets. The top 20 tracked files in the platform are all
`.mp3` audio tracks for games like `junk-runner`, `endless-td`, `hegemon`,
`capture-anime-girls`:

| size (B)    | path                                                              |
|------------:|-------------------------------------------------------------------|
| 17 106 117  | web/games/wip/junk-runner/src/assets/audio/music/cockpit/cockpit_theme_3.mp3 |
| 16 490 266  | web/games/wip/junk-runner/src/assets/audio/music/planet/planet_theme_2.mp3  |
| 11 684 877  | web/games/wip/endless-td/static/assets/audio/music/music_wave_6to15_v9.mp3  |
| 10 356 364  | web/games/wip/endless-td/static/assets/audio/music/music_boss.mp3           |
| 10 356 364  | web/games/wip/endless-td/static/assets/audio/music/concat/music_boss.mp3   |
|  9 923 110  | web/games/wip/endless-td/static/assets/audio/music/music_wave_1to5_v9.mp3   |
|  9 536 001  | web/games/wip/junk-runner/src/assets/audio/music/station/station_theme_1.mp3|
|  9 182 035  | web/games/wip/hegemon/static/assets/music/theme-map.mp3                     |
|  8 706 777  | web/games/wip/junk-runner/src/assets/audio/music/event/event_theme_3.mp3   |
|  8 098 549  | web/games/wip/endless-td/static/assets/audio/music/music_waveClear.mp3     |

**File-count by size bucket** in the platform's tracked tree:

| bucket        | count |
|---------------|------:|
| < 1 KB        |   800 |
| < 10 KB       | 2 594 |
| < 100 KB      | 1 548 |
| < 1 MB        | 2 366 |
| < 10 MB       | 1 102 |
| < 100 MB      |     3 |
| ≥ 100 MB      |     0 |

So: 1 105 files over 1 MB, dominated by audio assets.

## 3. Mirror-availability survey

### 3.1 GitHub (authenticated via `gh api`, user `DraconDev`, free personal)

| repo                          | size (KB) | size (GB) | private |
|-------------------------------|----------:|----------:|---------|
| dracon-platform               | 11 398 169 | **10.87** | true    |
| rust-ai-web-auto              |     3 247 | 0.003     | true    |
| browser-extensions-shared     |   509 867 | 0.48      | true    |
| avid                          |     4 502 | 0.004     | true    |
| ai-auto-writer                |   111 003 | 0.10      | true    |
| pully-fully-pull-based-fleet-reconciler | 7 006 | 0.007 | true    |
| dracon-code                   |    99 064 | 0.09      | true    |
| quick-draw-screenshot-clipboard|      450 | 0.0004    | true    |
| dracon-utilities              |    16 057 | 0.015     | false   |

**Github's free personal account policy**:
- 5 GB recommended repo size (soft cap — pushes return HTTP 500 when over)
- 100 MB per-file size limit
- Already on file (see `gitlab-storage-and-divergence-2026-06-23.md`):
  `error: RPC failed; HTTP 500 curl 22 ... fatal: the remote end hung up`

### 3.2 GitLab (anonymous API — 404 on all private repos)

| repo                          | size (public API) |
|-------------------------------|------------------:|
| (all 9 large repos)           | 404 — private     |

**GitLab's free tier policy** (per
`gitlab-storage-and-divergence-2026-06-23.md`):
- 10 GiB per-project free-tier quota
- Pre-receive hook rejection: "Your push would exceed the allocated
  storage for your project"
- The platform already had 9.5 GiB / 10 GiB (95% full) on gitlab as of
  2026-06-23

Anonymous API cannot return sizes for private repos. Without the
operator's `GITLAB_TOKEN`, gitlab-side sizes are not visible to the
agent. The platform's gitlab copy is irrelevant now (the platform is
codeberg-only) but for the other 8 repos that DO still use gitlab, the
operator's gitlab-side sizes are unknown to this analysis.

### 3.3 Codeberg (anonymous API — 404 on all private repos; SSH probe OK)

| repo                          | SSH ls-remote (HEAD)                   |
|-------------------------------|-----------------------------------------|
| dracon-platform               | `2a36874756cb80761db631542f2891efa20f1780` (main) |
| rust-ai-web-auto              | `5af2589af2ed0219ebf899b5ee7266b9acf9a040` (main) |
| browser-extensions-shared     | `db8e752c9279dcdb4fce62b2a07671f1cd908d4e` (main) |
| avid                          | `10e37561ce8aee30f3692dff9ffca45dc5fee73a` (main) |
| ai-auto-writer                | `36db4d4bcd9a20d175f755f02b5f5a2dfca03676` (main) |
| pully-fully-pull-based-fleet-reconciler | `a664931dc62e92b249c7462ab9c58203bb7637c3` (main) |
| dracon-code                   | `8a4b8abed3976694f259e890f8572b4b434ff88b` (main) |
| quick-draw-screenshot-clipboard | `81ff13db144701258e246e2304a908ae6c5f2a42` (main) |

**Codeberg's free tier policy**: no documented per-repo size limit found
in public docs. Codeberg's instance-wide limit is multi-TB; per-user
quotas appear to be in the 10-100 GB range. Need operator's
`CODEBERG_TOKEN` to query exact quota for any of these repos.

### 3.4 Summary of the storage problem

| mirror  | platform size | other 8 large repos          | failure mode                                |
|---------|--------------:|------------------------------|---------------------------------------------|
| github  | 10.87 GB      | 0.0004 - 0.48 GB             | HTTP 500 on push (already disabled for platform) |
| gitlab  | unknown (was 9.5 GiB before) | unknown (private API)     | pre-receive hook (already disabled for platform) |
| codeberg| pushed fine   | all SSH-OK                   | none known                                  |

**Only 1 of 9 large repos has a real storage problem right now**:
dracon-platform. The other 8 have small `.git/` and push fine. But
several are in the "about to be a problem" zone (browser-extensions-shared
.git = 528 MB, growing) and several have untracked build artifacts that
**would** become a problem if the daemon's `.gitignore` coverage ever
regressed (see §6.1 below).

## 4. Approach A: submodules / repo split

### 4.1 What it is

Split each large repo along its natural seams into multiple smaller
repos, wired together with `git submodule` (or `git subtree` or
`git-subrepo`). Each new sub-repo is mirror-pushable independently.

### 4.2 Per-repo natural seams

**dracon-platform**: per-game directories
(`web/games/wip/junk-runner/`, `web/games/wip/endless-td/`,
`web/games/wip/hegemon/`, `web/games/demos/*`, `web/games/wip/polis/`,
etc.) are independent. The big audio files are per-game. A natural
split would be 1 platform-docs repo + 1 platform-shared repo +
~15 per-game repos. But: the **.git/ bloat is in the audio assets
within the per-game repos**, not the platform-as-a-whole. Splitting
doesn't fix the problem unless audio is moved to LFS.

**browser-extensions-shared**: per-extension directories
(`extensions/auto-form-filler/`, `extensions/youtube-dislike/`, etc.)
are independent. ~30+ extensions. The 28 MB JSON in test fixtures is
in `extensions/youtube-dislike/tests/`. Splitting into 1 per-extension
repo would reduce the platform's biggest tracked file to 1-2 MB.

**dracon-utilities**: per-tool directories
(`dracon-sync/`, `dracon-warden/`, etc.) are already separate repos in
this case. No split needed; the .git is only 30 MB anyway.

**avid, ai-auto-writer, pully, rust-ai-web-auto, dracon-code,
quick-draw, search-daemon**: their `.git/` is small (15-221 MB); the
large worktree is from `target/` (untracked build artifacts). Splitting
won't reduce `.git/` size. Don't split.

### 4.3 Trade-offs

- **git-submodule**: well-supported, but has known performance issues at
  scale — `git submodule update` in a tree with 100+ submodules becomes
  slow, and recursive submodules add complexity. The 30+ extensions in
  browser-extensions-shared would be right at the cliff.
- **git-subrepo**: cleaner UX (one working tree, no submodule init),
  but historically unmaintained and not in git-core.
- **git subtree**: lowest ceremony, but cross-subtree refactoring is
  painful (each subtree is just a directory in the parent repo).
- **New repos with no parent** (the cleanest option): per-game repos
  with no umbrella. The "platform" becomes a Docker image or a script
  that clones all the sub-repos. Loses the single-checkout developer
  experience.

### 4.4 Estimated cost

- **dracon-platform split** (per-game): 2-3 weeks of careful history
  rewriting + per-game-repo mirror setup + daemon per-game-repo
  config. **But: this doesn't fix the audio-bloat problem**. Would also
  need LFS migration.
- **browser-extensions-shared split** (per-extension): 1-2 weeks for
  30+ extension moves. **Does fix the bloat** (28 MB JSON moves to
  per-extension repo).
- **Other 7 repos**: 0 — don't need it.

### 4.5 Verdict

- **Submodule / split is the right tool for browser-extensions-shared**
  (small binary bloat + natural per-extension seams + low coupling).
- **Submodule / split is the wrong tool for platform** (audio bloat
  won't be solved by splitting unless paired with LFS migration; the
  audio is per-game so it would split along with the games, but the
  per-game `.git/` would still be > 1 GB and the same problem
  reappears).
- **Submodule / split is unnecessary for the other 7 repos**.

## 5. Approach B: daemon bucketing

### 5.1 What it is

Modify the dracon-sync daemon to partition a single worktree's commits
into **N sibling "bucket" repos** by path-prefix, push each bucket to
its own 3-mirror set, and synthesize a unified checkout on demand via
`git worktree add` of each bucket. The user sees one worktree; the
daemon sees N repos.

### 5.2 Existing tools surveyed

- **git-annex**: manages large files **outside** git. Tracks file
  metadata in git; file contents in special remotes (S3, rsync, etc.).
  Solves a different problem: "I want my 50 GB of audio backed up but
  not in git". Not "I want my git history to be smaller".
- **bup**: backup tool with git-style deduplication. Backup-only, not a
  development workflow.
- **git-subrepo**: extracts a single subtree to its own repo on demand.
  N=1, not N=bucket-count.
- **git partial clone** (`--filter=blob:none`): the closest existing
  thing — clones metadata only, fetches blobs on demand. Doesn't reduce
  the **push** size, only the **clone** size. Doesn't help us.
- **git sparse-checkout**: client-side view filter. Same problem as
  partial clone.

**No existing tool solves "partition a single worktree into N
mirror-pushable repos"**. Approach B is novel daemon engineering.

### 5.3 Design sketch

```
[ worktree at /home/dracon/Dev/dracon-platform ]
              |
              | on each commit, daemon splits by path-prefix:
              |
              ├── bucket-games/junk-runner/    → bucket repo "games-junk-runner"
              ├── bucket-games/endless-td/      → bucket repo "games-endless-td"
              ├── bucket-games/hegemon/         → bucket repo "games-hegemon"
              ├── bucket-shared/target/*        → NOT IN ANY BUCKET (untracked)
              ├── bucket-docs/                  → bucket repo "platform-docs"
              └── ... etc
```

The daemon:
1. On commit detection, classify each changed path by its bucket rule
   (e.g. `web/games/wip/<game>/` → bucket `<game>`).
2. For each bucket, create a "synthetic" commit on the bucket's
   branch (e.g. `games/junk-runner` branch on `bucket-games-junk-runner`
   repo), with the same author/date but only the files in that bucket.
3. Push each bucket to its own 3-mirror set (github + gitlab + codeberg
   under the bucket's repo name).
4. The user sees the original worktree; the bucket repos are stored
   under `~/.dracon/buckets/<repo>/<bucket>/` and the daemon uses `git
   worktree` to manage them.

### 5.4 Trade-offs

- **Pros**: single worktree for the user; each bucket is small enough
  to mirror freely; the daemon's commit-all policy still works.
- **Cons**: 
  - Cross-bucket atomicity is lost (one logical commit becomes N
    synthetic commits). Bad for any change that spans buckets.
  - File moves across buckets are ambiguous (the file moves from one
    synthetic commit to another; SHA changes).
  - The bucket repos are bare — `git log` of the worktree no longer
    shows the bucket history. Need a synthetic history view.
  - The daemon gets substantially more complex (a new bucketing
    module, a new config schema, N pushes per commit instead of 1).
  - 4-8 weeks of careful engineering + testing.

### 5.5 Verdict

**Approach B is too speculative to commit to without a 1-week spike**.
The novel engineering is real: the daemon would need a new bucketing
module, a new synthetic-history viewer, a new cross-bucket move
resolver, and new testing. There is no prior art to crib from.

For the operator's actual problem (1 repo with a real storage issue,
several with latent risk), Approach B's complexity is not justified.

## 6. Approach C: pay for git storage

### 6.1 What it is

Pay for more storage on the existing mirrors. No structural change.

### 6.2 Pricing captured (2026-06-25, from public pricing pages)

**github**:
- **Pro**: $4 / month, 100 GiB LFS included
- **Team**: $4 / user / month (irrelevant — operator is solo)
- **LFS data pack**: $0.10 / GiB / month (after the included quota)
- **LFS file size limit**: 10 GiB per file on Pro
- **Per-file size limit**: 100 MiB (same as free — actually, 100 MiB
  on free, 2 GiB on Pro for regular files)

**gitlab**:
- **Premium**: $29 / user / month
- **Ultimate**: $99 / user / month
- **Storage add-on**: $5 / 10 GiB / year (per project; on top of plan)
- **Per-file size limit**: not explicitly listed (assume similar to
  github)

**codeberg**:
- Free; donations-supported. No paid tier published.

### 6.3 Cost math for the platform (the only repo that needs help)

Platform `.git/` is 19 GiB. The MP3s that cause the bloat could move to
**git LFS** instead of being stored in pack files:

- github Pro ($4/mo) includes 100 GiB LFS — covers the platform fully
  for $48/year.
- github LFS only ($0.10/GiB/mo after the included quota): 19 GiB ×
  $0.10 × 12 = $22.80/year.

If we also want the platform to be pushable to gitlab too, that's
gitlab's $5/10 GiB/yr × 2 (to go from 10 GiB → 30 GiB quota) = $10/year.
gitlab's $5/10 GiB/yr is much cheaper than github LFS but gitlab is
less critical now that the platform is already codeberg-only.

### 6.4 Cost-benefit

| option                                | annual cost | effort       | result |
|---------------------------------------|------------:|--------------|--------|
| github Pro (covers LFS)               | $48         | 30 min setup | platform mirror works on github |
| github LFS only                       | $22.80      | 30 min setup | platform mirror works on github |
| gitlab storage add-on (10→30 GiB)     | $10         | 30 min setup | platform mirror works on gitlab (if we want it) |
| codeberg (already free)               | $0          | 0            | already works |
| Approach A (split platform by game)   | $0          | 2-3 weeks    | .git/ per-game would still be big; need LFS anyway |
| Approach A (split browser-extensions) | $0          | 1-2 weeks    | .git/ per-extension small enough to mirror |
| Approach B (daemon bucketing)         | $0          | 4-8 weeks    | unproven; high risk |

**Verdict**: pay-for-storage is **dramatically cheaper** than the
structural alternatives. The whole problem is one $22-48/year
subscription.

### 6.5 Risks of paying

- **Recurring cost**: $22-48/year forever (vs. one-time $0 cost of a
  fix). Acceptable at this scale.
- **Vendor lock-in**: github could change pricing. Mitigation: stay
  on the free tier for gitlab + codeberg; if github raises prices,
  drop github for the platform (already done — the platform is
  codeberg-only).
- **LFS bundles add complexity to the daemon**: the daemon's
  auto-commit + auto-push pipeline doesn't currently understand LFS
  pointers. Adding LFS support is 1-2 weeks of daemon work.
  **However**: this is far less work than a full repo split.

### 6.6 Verdict

**Approach C is the right answer for the platform** (the only repo
that needs extra storage). It costs $22-48/year and the daemon's LFS
support is a known 1-2 weeks of work. Versus 2-3 weeks for a split
that doesn't actually fix the problem (audio still bloaty).

## 7. Cross-reference with existing policies

### 7.1 Daemon's commit-all rule

`AGENTS.md` (read 2026-06-25): the daemon's `untracked_exclude_patterns = []`
means **every untracked file gets auto-committed**. This is a **latent
risk** for the 7 repos with untracked build artifacts. If any repo's
`.gitignore` is ever lost or overridden, the daemon will auto-commit
`target/` and `.git/` will balloon overnight.

**Recommended pre-requisite** (do this BEFORE any structural fix):
add `target/`, `node_modules/`, `build/`, `dist/`, `output/` to
`.gitignore` in the 7 repos that are missing them. This is a 30-minute
task and prevents the next storage crisis.

### 7.2 Existing related design docs

- `gitlab-storage-and-divergence-2026-06-23.md`: solved the immediate
  platform github+gitlab push failures via the `exclude_remotes` daemon
  code change. This doc extends that work by considering the
  **structural** alternatives.
- `commit-all-policy-2026-06-15.md`: defines the daemon's commit-all
  default. The 7-repo `.gitignore` gap (§7.1 above) is a side effect of
  this policy.

## 8. Per-repo recommendation table

| repo                          | current .git | risk          | recommended approach | estimated post-fix .git | cost of fix |
|-------------------------------|-------------:|---------------|----------------------|------------------------:|------------|
| **dracon-platform**           | **19 GiB**   | **CRITICAL**  | **C: pay (github LFS) + drop LFS audio to LFS pointers** | **< 100 MiB** (LFS in pack, not in objects) | **$22-48/yr + 1-2 wk daemon LFS support** |
| browser-extensions-shared     | 528 MiB      | medium (growing) | **A: split per-extension** (28 MB JSON goes to one per-extension repo) | < 50 MiB per repo | 1-2 wk |
| dracode                       | 221 MiB      | low           | leave alone; add `target/` to .gitignore (precaution) | 221 MiB | 30 min |
| ai-auto-writer                | 124 MiB      | low           | leave alone; add `target/` to .gitignore (precaution) | 124 MiB | 30 min |
| avid                          | 52 MiB       | low           | leave alone; add `target/`, `output/` to .gitignore (precaution) | 52 MiB | 30 min |
| pully-fully-pull-based-fleet-reconciler | 39 MiB | low | leave alone; add `target/`, `dist/` to .gitignore (precaution) | 39 MiB | 30 min |
| dracon-utilities              | 30 MiB       | low (transient gitlab divergence already fixed) | leave alone | 30 MiB | 0 |
| quick-draw-screenshot-clipboard| 4 MiB      | low           | leave alone (already in .gitignore) | 4 MiB | 0 |
| rust-ai-web-auto              | 15 MiB       | low           | leave alone; add `target/` to .gitignore (precaution) | 15 MiB | 30 min |

**Below threshold (no action needed)**: search-daemon, dracon-libs,
dracon-strategy, pi-plugins.

## 9. Recommended next step: one specific POC

### 9.1 Pick the lowest-risk, highest-leverage experiment

The **`.gitignore` hygiene pass** is the universal pre-requisite and the
lowest-cost first step. It costs 30 minutes and prevents the next
storage crisis across 7 repos. This should be the FIRST deliverable of
any follow-up goal, regardless of which structural approach is chosen.

After `.gitignore` hygiene:

**The platform is the only repo with a real storage problem now.** The
recommended POC is:

> **Migrate the platform's MP3 audio assets to git LFS** and add LFS
> support to the dracon-sync daemon. This brings the platform's `.git/`
> from 19 GiB to < 100 MiB. The platform can then be re-pushed to
> github (LFS) and gitlab (LFS) without further intervention.

### 9.2 Why this POC, not another

- **Lowest cost**: $22-48/year (pay) + 1-2 weeks (daemon LFS) vs.
  2-3 weeks (split) or 4-8 weeks (bucketing).
- **Real validation**: it directly solves the problem the operator
  flagged (platform can't mirror to github/gitlab because of size).
- **Doesn't preclude later work**: if LFS proves unsatisfactory, the
  structural alternatives are still available. The migration is
  reversible (`git lfs migrate export` rewrites history back).
- **Doesn't touch the 8 healthy repos**: minimal blast radius.

### 9.3 POC success criteria

- `du -sm .git` for the platform < 200 MiB after LFS migration.
- Platform pushes successfully to github (via LFS data pack).
- Platform pushes successfully to gitlab (if LFS is enabled there too;
  optional for the POC).
- The daemon commits and pushes LFS pointer files correctly without
  operator intervention.
- No regression in the other 8 large repos.
- 1-2 weeks of daemon work; 30 minutes for the operator to enable
  github Pro.

### 9.4 Out of scope for this POC

- Splitting browser-extensions-shared per-extension. Can be a
  follow-up goal if the platform LFS POC succeeds.
- Daemon bucketing (Approach B). Too speculative; revisit only if LFS
  and splits both fail to scale.
- Repo's own LFS server. github's hosted LFS is good enough.

## 10. Conclusion

The 3-mirror sync system is sound. The storage problem is narrow:
**1 of 9 large repos** has a real issue, and it's a $22-48/year problem
with a known 1-2 week fix (LFS + daemon LFS support). The other 8
repos are healthy and need only a 30-minute `.gitignore` hygiene pass
as insurance against future `.git/` bloat.

Submodule/split (Approach A) is the right tool for
browser-extensions-shared, but not for the platform. Daemon bucketing
(Approach B) is novel engineering with no prior art and is too
speculative to commit to without a 1-week spike.

**Recommended path forward**: `.gitignore` hygiene → platform LFS
migration + daemon LFS support → split browser-extensions-shared per
extension (optional, separate goal). Total cost: 30 min + 1-2 weeks +
$22-48/year. Implementation deferred to a follow-up goal.
