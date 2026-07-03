# LFS vs Bucket vs "just don't grow": the real tradeoff

The user pushed back on LFS with two important points:

1. "LFS seemingly might eventually get pricey if my repo kept growing"
2. "If we just keep growing one repo then we are destined to keep
   paying LFS and make the triple sync invalid cause others will be
   too small to follow"

Both points are correct, and reveal a third issue I missed:

3. "If I ever got a 2 gig repo that would be a problem" — i.e., the
   problem isn't "how do I fit hegemon under 2 GB on github?" but
   "how do I keep growing without hitting a wall?"

This document re-examines the LFS recommendation in light of the
bucket strategy that's actually in place.

## What the LFS doc missed

I previously proposed: "apply LFS to hegemon's 430 MB of static/,
github free tier fits 1 GB, all done". The user is right that this
misses the bigger picture:

**Hegemon's `static/assets/` is regenerable.** I counted 47
`gen-*.py` scripts in `static/assets/`'s parent. They all do the same
pattern: read prompts from `scripts/style-pipeline/<thing>.txt`, call
mmx to generate, write PNG/MP3 to `static/assets/<thing>/`.

So the `static/assets/` content is the **output of running the gen
scripts**. It's regenerable from prompts in git.

This means LFS isn't necessary — instead, the bucket strategy ALREADY
in place (`web/docs/asset-pipeline.md`) is the right answer:
- Gen scripts in git
- Output in the OVH bucket (or just kept locally + gitignored)

## The bucket is already configured for this

Looking at `web/docs/asset-pipeline.md`:
- `web/games/libs/platform/ovh-bucket.ts` is the runtime loader
- `web/scripts/ovh-bucket-publish.mjs <slug>` publishes games
- For source assets: "Layer 2 — TODO after OVH secret recovery"
  publish source assets under `games/<slug>/assets/<sha256>.<ext>`

The bucket is **already wired up**. The only missing piece is:
"publish source assets under `games/<slug>/assets/`". Once that's
done:
- `static/assets/` is gitignored
- Gen scripts in git produce the assets on each dev environment
- Bucket hosts them for the running game

This is the cheaper, simpler answer than LFS.

## Why the user is right about growth

LFS has a **bandwidth** problem github enforces. Free tier:
1 GiB/month bandwidth. A single `git clone` downloads whatever LFS
content is referenced. If hegemon grows to 4 GiB static/, every clone
costs 4 GiB of bandwidth. 4 clones/month = 16 GiB/month. github
charges for overage.

The bucket has **zero egress** (per OVH pricing in asset-pipeline.md:
"zero ingress, zero egress, zero API call fees"). So a bucket-based
strategy scales cheaply as content grows.

LFS scales expensive as content grows.

## So the actual answer for the binary content problem

For hegemon's `static/assets/`:
- Add `static/assets/` to `.gitignore` (already excluded for
  `static/assets-legacy/`)
- Move existing tracked content out via `git rm --cached`
- Gen scripts stay in git
- Output goes to bucket (when dev wants to play) OR is regenerated
  on the fly
- The `0` byte pack for the binary content becomes `0` → trivially
  fits github

For other games' similar `static/`: same fix.

For ai-auto-writer generated images: same fix.

For browser-extensions screenshots: same fix.

## What this means for the triple-sync workflow

User's worry: "If I keep growing one repo I'm destined to keep
hitting 2 GB and have to keep paying LFS."

Reality:
- If you don't put large binary content in git, **the repo doesn't
  grow past 2 GB regardless of how much content you produce**.
- Content (sprites, audio, generated images) lives in the bucket.
- The repo stays at code/config/small-data size.
- github's 2 GB wall becomes irrelevant because the repo never
  approaches it.

This is the same pattern as: "the daemon doesn't auto-stage files
> 100 MiB" (per AGENTS.md). The pattern is "don't put large content
in git, put it elsewhere".

## Re-examination of the git submodule question

If we accept "binary content goes to bucket, not git":

| Repo | Tracked size without binaries | Github-pushable? |
|------|-------------------------------|------------------|
| hegemon | ~50 MB (code, scripts, prompts) | YES trivially |
| polis | 33 MB | YES |
| darklord | 82 MB | YES |
| endless-td | 301 MB (some binary in code) | YES, but worth checking |
| capture-anime-girls | 214 MB (likely some binary) | YES, but worth checking |
| neonbreak | 231 MB | YES, worth checking |
| deathrun | 230 MB | YES, worth checking |
| hellhunter | 1 MB | YES |
| junk-runner | 21 MB | YES |
| one-mil-girls | 53 MB | YES |

The submodule migration was unnecessary IF the binary content had
been kept out of git in the first place. The migration was a fix
for "we put 430 MB of MP3/PNG in git and now github rejects us".

## The actual recommended path

**Not multi-repo. Not LFS. Just: don't put large binary content in
git.**

Specifically:

1. **Add `static/assets/` (or whatever the binary dir is called) to
   each game's `.gitignore`**

2. **Run `git rm --cached static/assets/`** on each game that has
   tracked binary content (this removes from index, keeps on disk)

3. **Confirm the bucket strategy is wired up for source assets** —
   the layer-2 todo in asset-pipeline.md

4. **The repo sizes drop by 95%**: hegemon's 2.27 GiB → ~50 MiB,
   other games similarly shrink. github accepts all of them.

5. **Optional cleanup**: revert the 6 daemon fixes for submodule
   quirks by un-doing the submodule migration entirely
   (dracon-platform → monorepo like before, with binary content
   gitignored).

6. **Dev workflow**:
   - Run `bun run scripts/regenerate-assets.mjs` after clone
   - OR pull from bucket via `bun run scripts/ovh-bucket-asset-pull.mjs`
   - Either approach is offline-friendly once assets are local

7. **The bucket is the right home** for:
   - Built game bundles (`games/<slug>/<version>/`)
   - Source assets (`games/<slug>/assets/<sha256>.<ext>`)
   - Generated images, audio, screenshots
   - OVH costs $22.75/TiB/month for storage; egress is free

## What I got wrong in the earlier docs

In `binary-asset-strategy-2026-07-03.md` I said:
> "LFS is the right answer. The 'drop github remote' or 'accept 3/4'
> options are workarounds, not solutions."

This is wrong for our specific situation because:
1. The content is **regenerable** (47 gen scripts prove it)
2. The bucket is **already configured** for it
3. The user is right that LFS scales expensive as content grows
4. github's 1 GiB/month bandwidth cap makes LFS non-viable for any
   active public repo with growing binary content

The bucket is the actual right answer for games-style content.

## What I got wrong in `submodule-pain-explanation-2026-07-03.md`

In that doc I said LFS was preferred over bucket because:
- bucket has "no diff UI"
- "no attribution"
- "no offline dev"

These are true for arbitrary binary content but **NOT for regenerable
content**:
- Diff: gen scripts in git, output in bucket. The scripts' diff
  shows what changed.
- Attribution: gen scripts have author. Generated output has the
  commit hash that produced it.
- Offline dev: `bun run scripts/regenerate-assets.mjs` works offline.

The "bucket is awkward" concern applies to things like raw
screenshots or hand-drawn art. For AI-generated art with prompts in
git, the bucket is the right answer.

## TL;DR

- The user is right that LFS is the wrong tool for our content
- The user is right that "growing one repo" + LFS is a death spiral
- The actual right answer is: don't put binary content in git at
  all — keep it in the bucket
- This makes submodules, multi-repo, and monorepo all viable
  because repo sizes stay small (under 100 MB regardless of content
  growth)
- hegemon github-empty problem solves itself once `static/` is
  out of git
- The submodule migration was an unnecessary complication; we can
  keep submodules for separation of concerns (parent doesn't need
  to know game internals) but the github issue is solved by
  gitignoring binary content, not by LFS

This document supersedes the LFS recommendations in
`binary-asset-strategy-2026-07-03.md`.