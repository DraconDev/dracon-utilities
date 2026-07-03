# Correction: the actual history + a real choice

The user's reply ("before we were a monorepo that was super clean until
we couldn't push, but still this lfs seems like a ripoff compared to
just multi repos") forced me to actually look at the git history. What I
found:

1. **Until 4 days ago, dracon-platform WAS a monorepo.** All 10 games
   lived in `web/games/wip/` and `web/games/released/` as plain
   directories. The pack was 3.6 GiB — 95.2% from the games.

2. **The monorepo broke github push on 2026-06-25** at the 2.00 GiB
   pack limit. This is documented in
   `web/docs/SYNC-AUDIT-2026-06-28.md`.

3. **The submodule migration was executed 2026-06-29 to 2026-06-30**
   ("light", "medium", "heavy" phases), with the express goal of
   "Restore 3-remote push workflow by extracting games to submodules".

4. **hegemon's empty github remote was THE explicit goal of this
   migration** — the migration commit `953d7f8759` says:
   > NOTE: hegemon's game repo on github is empty (2.39 GiB pack
   > exceeds github's 2.00 GiB wall). hegemon's full history lives in
   > codeberg and gitlab game repos.

5. **The submodule migration was a workaround for the github 2 GB
   limit.** It was supposed to shrink the main repo's pack to 65 MiB
   (97% reduction). What actually happened: hegemon alone is 2.27 GiB,
   so github STILL rejects it after the migration.

## Why submodules didn't fix the github problem

The submodule migration idea assumed: if hegemon is its own repo, its
pack lives in its own `.git`, and the main repo's pack stays small.

But git's pack-size limit applies **per-repo, not per-repo-and-its-
submodules-from-the-parent's-view**. After migration:
- main repo (.git): 65 MiB pack ✓
- hegemon repo (.git): 2.27 GiB pack ✗ (still rejected by github)

The migration didn't shrink anything that mattered. The github limit was
on hegemon-as-its-own-repo, not hegemon-as-subdirectory-of-main.

## So the user's intuition is correct

The user said:
> "before we were a monorepo that was super clean until we couldn't
> push, but still this lfs seems like a ripoff compared to just multi
> repos"

Both observations are correct:
1. **Monorepo was clean** — until the binary content grew past 2 GiB
2. **git-lfs is expensive AND complex** compared to multi-repo

## What multi-repo would actually look like

We have a top-level `/home/dracon/Dev/dracon-utilities` repo and 25
sibling repos. The 10 games are inside `dracon-platform` as
submodules. Removing submodules means:

- Game at `dracon-platform/web/games/wip/polis` becomes
  `/home/dracon/Dev/polis` (sibling repo, like the others)
- Same for all 10 games
- `dracon-platform` becomes a tree with `web/games/` gitlinked to nothing
- The "monorepo" boundary moves: platform no longer "owns" games

```
/Dev/
  polis/                  ← NEW: own repo, github-pushable
  darklord/               ← NEW: own repo
  ...
  hegemon/                ← NEW: own repo, BUT 2.3 GiB pack, STILL rejected by github
  ...
  dracon-platform/        ← SHRUNK: no more web/games subtree
  dracon-utilities/
  ...
```

The github-2GB problem is per-repo, not per-tree. So multi-repo doesn't
fundamentally fix hegemon's case either. **For hegemon specifically,
LFS is still needed.**

But for the OTHER 9 games (all < 350 MB tracked), multi-repo makes
sense because:
- Each game is github-pushable as-is
- No submodule complexity
- No daemon-side detached-HEAD trap
- Git operations are normal

So the answer is: **multi-repo for 9 games + LFS for hegemon**. LFS
isn't a "ripoff compared to multi repos" — it's an additional
mechanism needed for ONE specific repo (hegemon) that genuinely has too
much binary content.

## What about the OTHER 9 games: are they really cheap on github?

Per the migration audit (2026-06-29):

| Game | Tracked | Github-pushable today? |
|------|---------|-------------------------|
| polis | 33 MB | Yes |
| darklord | 82 MB | Yes |
| endless-td | 301 MB | Yes |
| capture-anime-girls | 214 MB | Yes |
| neonbreak | 231 MB | Yes |
| deathrun | 230 MB | Yes |
| hellhunter | 1 MB | Yes |
| junk-runner | 21 MB | Yes |
| one-mil-girls | 53 MB | Yes |
| hegemon | 2,346 MB | **No — github rejects** |

So **9 of 10 games would be free github pushes if we move them to
multi-repo**. Only hegemon needs LFS.

## What about the parent platform repo?

dracon-platform would lose `web/games/` content but keep its docs,
infra, apis, scripts. The pack would drop from 12.61 GiB (currently
counting submodule content) to likely 200-400 MiB (its own
documentation + scripts).

Wait — let me re-check. The "12.61 GiB" we measured on
`/home/dracon/Dev/dracon-platform` is counting pack size INCLUDING
submodule content via the shared gitdirs at
`.git/modules/web-games-*/`. So the 12.61 is misleading.

What matters is: if dracon-platform had no submodules, just its own
content (docs/, apis/, infra/, scripts/, web/music/, web/games/
ghosts?), the pack would be the size of those tracked files.

Looking at the migration audit §1:
| Component | Tracked Size |
|-----------|---------|
| web/music/ | 4 MiB |
| Everything else (apis/, infra/, scripts/, etc.) | 73 MiB |
| web/games/ (submodules now) | (mostly gitlinks now) |
| **Total** | **~77 MiB tracked** |

So dracon-platform's "own content" is ~77 MiB, well under github.
Going multi-repo (no submodules) means dracon-platform is a 77 MiB
repo. github-pushable.

## The real decision matrix

### Status quo (submodules)
- `dracon-platform` pack: 65 MiB (just docs/scripts) — github-OK
- 9 game submodule packs: small — github-OK
- `hegemon` submodule pack: 2.27 GiB — github-REJECTED
- Daemon code: 6 fixes for submodule quirks (worktree-style HEAD,
  detached HEAD refspec, materialize_pending_submodules, etc.)
- One regression this week (hegemon detached HEAD broke daemon)

### Option A: Multi-repo (remove submodules for 9 games)
- `dracon-platform` pack: 77 MiB — github-OK
- 9 games each: their own repo, github-OK
- `hegemon` repo: 2.27 GiB — github-REJECTED (same problem)
- Daemon code: standard (no submodule specials)
- Parent stops trying to gitlink games

### Option B: Multi-repo + git-lfs for hegemon
- hegemon uses LFS for `static/` content
- hegemon pack: ~50 MiB (pointers only) — github-OK
- All 10 games + parent: github-OK
- Daemon code: standard
- LFS adds: `.gitattributes`, `git lfs install`, `git lfs track` patterns
- github free tier: 1 GB LFS storage, hegemon needs 430 MB — fits

### Option C: Back to pure monorepo + LFS for everything
- All 10 games + parent: 1 single repo
- All binary content via LFS
- Local pack shrinks from 12 GiB to ~100 MiB
- github-OK
- Daemon: simple (no submodules at all)
- "Super clean" as the user remembers
- LFS bandwidth: each clone downloads 4+ GiB of binaries. Free
  tier 1 GiB/month → expensive at scale.
- Single history makes atomic cross-game commits possible
- Cons: if LFS server dies, repo's heavy content is gone
- Pros: matches the "monorepo was super clean" memory

## My honest recommendation

**Option B** (multi-repo + LFS for hegemon only) is the smallest
change with the biggest benefit. It:

1. Eliminates 6 daemon fixes for submodule quirks (rollback 6 commits
   in daemon code)
2. Removes the detached-HEAD failure mode (you just see a normal
   detached HEAD, daemon handles it correctly)
3. Fixes 9 of 10 games' github pushes automatically (they're normal
   small repos)
4. LFS for hegemon's static/ addresses the github-2GB problem with
   well-understood tooling (free tier fits)
5. Restores the "clean per-repo" mental model that the user prefers

The migration cost is ~1 day of work: move 9 games out of submodules
into sibling repos. hegemon stays as a special case requiring the LFS
migration.

Option C (pure monorepo + LFS) would also work but the user's
experience with LFS bandwidth suggests it's not worth the cost.

## What I should have noticed earlier

I framed the choice as "submodules vs nested-repos vs monorepo"
without noticing that **we WERE a monorepo until 4 days ago**, and the
submodule migration was a workaround for the github 2GB limit that
didn't work for hegemon anyway. The history shows:
- 2026-06-29: monorepo, super clean, broken github push
- 2026-06-29 to 30: extract games to submodules to fix github push
- 2026-06-30: subgraph-removal still needed; hegemon github still
  empty
- 2026-07-01 to 03: daemon code patches for submodule quirks

The submodule migration added complexity (daemon code, this week's
regression) to solve a problem that wasn't actually solved (hegemon
github still empty). **The right answer was multi-repo from the
start**, which was the user's prior intuition.

This document corrects the earlier framing in
`submodule-pain-explanation-2026-07-03.md` and
`binary-asset-strategy-2026-07-03.md`.