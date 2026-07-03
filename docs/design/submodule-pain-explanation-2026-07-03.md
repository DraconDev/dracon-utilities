# Why submodules are painful — and where we actually are

A frank explanation of the situation, written 2026-07-03 at the user's
request. The user is frustrated (justifiably) that the daemon/submodule
combination is harder than expected. This document explains why, and what
the actual current state is.

## TL;DR — what we actually have right now

The daemon **is** committing on 9 of the 10 nested paths (proven by the
"dirty=N" column showing files that will be auto-committed on the next
daemon cycle). What is broken is:

1. **hegemon** is the worst case — `/Dev/hegemon` is back, nested is
   detached HEAD, github push is stuck on the 2GB limit
2. **dirty files everywhere** — capture-anime-girls, endless-td,
   deathrun, junk-runner all have uncommitted changes that the daemon
   hasn't flushed yet (this is the "we are still not seeing that we are
   fully committing in all places" feeling)

So the user's perception is correct: **commits are happening on the
nested submodule paths, but they are not landing on all 4 remotes
reliably**. The daemon has been working through a backlog.

## Why submodules are painful — the actual reasons

### 1. Git submodules are two repos in a trench coat

When you have `dracon-platform/web/games/wip/polis/`, you have:
- The parent repo (`dracon-platform`) which stores a 40-byte gitlink
- The submodule repo (polis itself) which lives at
  `/home/dracon/Dev/dracon-platform/.git/modules/web-games-polis/`
- A "worktree" of the submodule repo at the nested path

So a commit to a file in polis touches **three places**:
1. The nested worktree's `index` and `objects/`
2. The shared gitdir's `refs/heads/main` (when on a branch)
3. The parent repo's gitlink (when the daemon stages the submodule update)

If any of these three go out of sync, the next `git pull` or
`git push` will fail or do something unexpected.

### 2. The "fake worktree" trap (what bit us)

When `git submodule update --init` runs, it creates a worktree at the
nested path. But the nested path's `.git` is **a file**, not a
directory. That file points at `<shared_gitdir>/worktrees/<X>`, where
`<X>` is the worktree's name.

**However**: when the daemon does `git -C $nested commit ...` while the
shared gitdir's HEAD is already `ref: refs/heads/main`, the commit goes
into the shared gitdir as a regular branch advance. That works fine.

The bug appears when:
- Someone (a human) runs `git checkout <sha>` on the nested path
- This detaches HEAD, and the shared gitdir's HEAD file gets
  **rewritten from `ref: refs/heads/main` to the SHA itself**
- After that, every commit goes onto a detached HEAD instead of
  advancing `refs/heads/main`
- When the daemon later does `git push origin HEAD:refs/heads/main`,
  the detached HEAD is the same SHA as `refs/heads/main` (because
  the daemon has been fast-forwarding main to match). But the
  next `git checkout main` (or similar) won't restore HEAD to the
  branch pointer — because HEAD is now permanently a SHA

This is **not fixable from inside the daemon** in any obvious way. The
"correct" fix is: never let HEAD get detached in the first place. But
humans `git checkout` all the time to inspect old commits, so this is
inherently fragile.

### 3. The 2GB github limit is structural

hegemon's pack is **2.3 GiB** because `static/` (430MB of MP3s and
PNG sprites) is committed to git. This will NEVER fit on github. There
are three options:

| Option | Effort | Outcome |
|--------|--------|---------|
| A. `git rm --cached static/`, expand `.gitignore`, push | Medium | Pack drops to ~1.9 GiB, fits github |
| B. Drop the github remote for hegemon | Low | 3/4 forever, no github visibility |
| C. Accept 3/4 as steady state | None | What we have now |

Option A is the right answer, but it requires updating the asset
hosting story (move static/ to a CDN or bucket, see
`web/CANONICAL-ASSET-HOSTING.md`).

### 4. Submodules are worktrees-of-worktrees-of-worktrees

The full nesting:
- dracon-platform (parent repo)
  - .git/modules/web-games-hegemon/ (shared gitdir for hegemon)
    - worktrees/hegemon/ (the standalone's worktree metadata)
    - HEAD (this is what `is_on_main_branch` reads!)
  - web/games/wip/hegemon/ (the nested worktree)
    - .git (file pointing to the shared gitdir root, NOT to worktrees/hegemon)

The nested's `.git` file points at the **shared gitdir root**, not at
a `<shared>/worktrees/<nested>` subdir. So the worktree's HEAD file
IS the shared gitdir's HEAD file. There is no per-worktree HEAD file
for the nested path — because it's the "main" worktree of the shared
gitdir (the only worktree that doesn't have a `worktrees/<X>` subdir).

This is the layout git creates by default for the **initial worktree**
of a bare repo. The standalone `/Dev/hegemon` is a SECOND worktree, so
it does have its own `<shared>/worktrees/hegemon/` subdir with its own
HEAD file.

When we removed the standalone, git removed the `<shared>/worktrees/hegemon/`
subdir. The nested is now the only worktree. Its HEAD is the shared HEAD.

**That's why `is_on_main_branch` looks at `<shared>/HEAD`** — because
for the only-worktree case, that's the only HEAD there is.

### 5. The daemon's auto-push retries don't compose with detached HEAD

When the daemon's push fails (e.g. github 2GB limit), it retries with
`pull --no-rebase origin HEAD: Fast-forward`. This fast-forwards the
local branch to match origin. **But on a detached HEAD, the local
HEAD file gets overwritten with the new SHA** (because that's how
detached HEAD updates — by rewriting the HEAD file).

So: push → fail → pull → HEAD becomes a different SHA → push fails
again (different reason this time).

The fix would be: when on detached HEAD, NEVER auto-pull. Just wait for
human intervention. But the daemon can't distinguish "detached because
of a deliberate git checkout" from "detached because something is wrong".

## What we should probably do

### Option 1: Stick with submodules, but fix the fragility

**Pros**: parent repo stays small (gitlinks only), daemon architecture
already mostly works, 9/10 games are fine
**Cons**: hegemon will keep breaking, every `git checkout <sha>` by a
human creates a regression risk, github 2GB limit on hegemon is permanent

**Fixes needed**:
1. Stop using `materialize_pending_submodules` — just trust the nested
   path always
2. Add a watchdog: if shared HEAD is detached for >5 min, rewind it
   to `ref: refs/heads/main`
3. Apply Option A for hegemon (git rm --cached static/)

### Option 2: Move static/ to a bucket, keep submodules

Same as Option 1 but explicitly separates code (in git) from content
(in bucket). This is what `web/CANONICAL-ASSET-HOSTING.md` already
suggests but never fully implemented for the games.

### Option 3: Convert games to nested repos (your preference)

Drop submodules entirely. Each game is its own repo at
`/Dev/<game>/`. Parent `dracon-platform` doesn't include them at all
(or includes them as non-tracked sibling directories).

**Pros**: no submodule complexity, each game is independent, github
push works (with the static/ → bucket migration), git operations are
normal
**Cons**: parent repo can't version games together with platform, no
parent gitlinks, the "dracon-platform is a coherent product" story
breaks

### Option 4: Monorepo (your other preference)

One single repo with all games at the top level. No submodules, no
nested repos.

**Pros**: simplest mental model, atomic commits across games, no
submodule pain
**Cons**: parent .git is now multi-GB (every clone downloads all
games' history), github 2GB limit hits the WHOLE platform, not just
hegemon. Could work with `static/` → bucket migration.

## Honest verdict

Submodules were chosen for size (so `dracon-platform/.git` stays
small). That benefit is real, but the cost is the complexity we are
now paying in daemon code, edge cases, and brittleness.

If we had to start over today, with the asset-hosting story in place
(static/ in a bucket, code-only in git), **Option 4 (monorepo) would
be the simplest**. The github 2GB limit applies to the pack size, but
if static/ is removed, the pack would be small enough.

The current state is "submodules mostly working, hegemon broken, daemon
working through a dirty-files backlog". It is fixable, but the fix
requires either:

1. An asset-hosting migration (move static/ to a bucket) — medium
   effort, addresses the github limit
2. A watchdog in the daemon to auto-rewind detached HEAD — small
   effort, addresses the brittleness
3. Both, plus the bulk migration re-done with proper `git worktree add`
   semantics — large effort, future-proofs against regression

The user is right that this is harder than expected. Submodules are
hard mode. Nested repos or monorepo would have been easier.