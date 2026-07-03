# Binary asset strategy — the missing piece

A follow-up to `submodule-pain-explanation-2026-07-03.md`. The user
correctly pointed out that the "move static/ to a bucket" approach is
awkward for development. This document explains why, and what actually
works.

## The bucket strategy is wrong for development

The reasons:

1. **No diff UI** — `git diff static/sprites/hero.png` is meaningless
   for binary. With a bucket, you have no diff at all, just timestamps.

2. **No attribution** — `git log static/sprites/hero.png` tells you who
   committed it. With a bucket, you have a separate access log that
   doesn't tie to commit history.

3. **No offline dev** — bucket-required assets mean every dev machine
   needs network. Single-player game development breaks on planes.

4. **No version pinning** — "what was the asset hash on commit X?"
   becomes a separate problem. With git, the answer is implicit in
   the SHA.

5. **Two systems to keep in sync** — git history AND bucket lifecycle.
   When you revert a commit, the bucket assets don't revert.

6. **No rollback** — corrupted asset in bucket? Roll back git is one
   command. Roll back bucket is a separate process.

Buckets/CDNs are for end-user distribution (the *running game* fetches
sprites from `cdn.hegemon.games/sprite.png`), not for development.

## What actually works for binary assets in git

### Option A: git-lfs (Large File Storage)

Replace the binary file with a small text "pointer file" in the repo,
and store the real binary in a separate server.

```
$ cat static/sprites/hero.png
version https://git-lfs.github.com/spec/v1
oid sha256:abc123...
size 124000

$ git log static/sprites/hero.png
commit 4a3b... by dracon <dracsharp@gmail.com>
Date: Mon Jul 3 01:00:00 2026 +0100
   New hero sprite variant

$ git lfs ls-files
abc123... * static/sprites/hero.png
```

Pros:
- Diff is "pointer changed", log is normal git log
- `git lfs pull` on clone fetches real assets
- github supports it natively
- Existing clone → LFS fetch is opt-in (`GIT_LFS_SKIP_SMUDGE=1`)
- Codeberg and gitlab both support it

Cons:
- github: 1 GiB free, then $5/month for 50 GB. For hegemon's 430 MB
  static/, this is well within the free tier for a single dev.
- LFS server is the single point of failure (though major providers
  guarantee reliability)
- Slightly more setup: `git lfs install`, `git lfs track "*.png"`

### Option B: git partial clone (`--filter=blob:none`)

Modern git feature that lets you clone WITHOUT blobs, then fetch them
on-demand. Works on github, codeberg, gitlab without any setup.

```
$ git clone --filter=blob:none https://github.com/DraconDev/hegemon.git
$ cd hegemon
$ ls static/sprites/hero.png
# File exists, but contents are "blob placeholder" until fetched
$ git ls-files --stage | grep hero.png
$ git cat-file -p <hash>
# Real contents fetched transparently
```

Pros:
- No setup required
- Free on all major hosts (github, codeberg, gitlab)
- `git clone` is fast even for huge repos

Cons:
- The pack is still bigger than 2 GiB server-side — github will still
  reject the PUSH (this fixes the local dev experience but not the
  push-to-github problem)
- So this is useful but not a github-push solution

### Option C: git-lfs + partial clone (best of both)

Use LFS for active binary assets (sprites you edit, audio you tune).
This makes the pack under 2 GiB so github accepts pushes.

Use partial clone in CI / deploy contexts where you only need to build
once and serve the artifacts.

Result:
- `git push` to github works (pack < 2 GB)
- `git clone` works for normal devs
- `git clone --filter=blob:none` works for build servers
- Asset history is preserved in git-lfs

## Recommendation

For hegemon specifically:

1. Install git-lfs
2. `git lfs track "static/**/*.png"` and `static/**/*.mp3`
3. `git lfs migrate import --include="static/**/*"`
4. Expand `.gitignore` to not track new binaries (only allow LFS)
5. Verify pack size drops under 2 GiB
6. Push to github

For the bigger picture (dracon-platform at 12 GiB):

- dracon-platform's pack includes all the submodule gitlinks + docs +
  scripts. The actual repo state is small. The 12 GiB pack is because
  git counts the **shared gitdirs' objects** if they're inline. Need
  to confirm by running `git count-objects -vH` from inside
  `dracon-platform` (separate measurement, not the parent counting
  submodule content). If dracon-platform's "own" content is under 2
  GB, github would accept it. If not, apply LFS to its docs/ assets.

For ai-auto-writer (109 MB but generates images):

- Decide: do generated images go in git?
- If yes: use LFS proactively before it grows past 2 GB
- If no: regenerate on each dev environment (config in CI)

For browser-extensions-shared (499 MB):

- Likely screenshots, screenshots dirs
- Audit and decide which need to be in git
- LFS for the rest

## Git-lfs free tier math

github LFS free: 1 GiB storage, 1 GiB/month bandwidth.

hegemon static/ today: 430 MB.
After LFS migration: 430 MB in LFS storage (free).
Bandwidth: each clone downloads 430 MB. Per-month bandwidth is 1 GiB.
At ~2 clones per month, you'd exceed the free tier.

But: codeberg and gitlab have their own LFS pricing. codeberg: free
public repos, generous private. gitlab.com: 10 GiB free LFS.

Practical recommendation:
- github: keep public, accept LFS cost when needed (or accept 1 GiB
  free tier if usage is light)
- codeberg: primary push target (5 GB pack limit, LFS free for public)
- gitlab: 10 GB pack limit, free LFS

## Summary

- Bucket strategy: WRONG for development. Use LFS instead.
- LFS fixes the github 2 GB limit for any single repo.
- Partial clone speeds up local dev without changing the push story.
- The "drop github remote" or "accept 3/4" options are workarounds,
  not solutions. LFS is the actual fix.

The auditor's question "if any repo ever goes over 2 gigs we are still
stuck" is correct — github's 2 GB pack limit is structural. The answer
is NOT to avoid large content, but to put large content in LFS so the
**pack** stays small while the **repo's effective size** can be large.