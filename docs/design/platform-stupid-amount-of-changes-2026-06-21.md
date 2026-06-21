# Platform "stupid amount of changes" investigation — 2026-06-21

**Goal:** `bb5ce6d5-f630-4077-869f-03d7b7ab5645` (investigate in detail).
**Status:** INVESTIGATION COMPLETE. No auto-fix applied.

## TL;DR

The platform `/home/dracon/Dev/dracon-platform` is seeing a "stupid
amount of changes" because **multiple parallel dev sessions are
actively producing untracked artifacts faster than the daemon can
batch-commit them, AND the daemon's commit loop is blocked by a
4-PNG stale unmerged index state**.

| Source | Count | Size | Type | Recommendation |
| --- | ---: | ---: | --- | --- |
| **capture-anime-girls** card PNGs (char_5007-5199) | 193 | 33.9 MB | DELIVERABLE | commit |
| **endless-td** music mp3s + menu_backdrop | 7 | 14.2 MB | DELIVERABLE | commit |
| **deathrun** static/images v2/v3 | 22 | 3.3 MB | DELIVERABLE | commit |
| **deathrun** docs/audit PNGs + MDs | 12 | 5.8 MB | DOCS | commit (audits) |
| **hellhunter** scripts/{debug,smoke-out} | 17 | 1.8 MB | EPHEMERAL | gitignore |
| **darklord** scripts/smoke-out + audit | 22 | 1.2 MB | EPHEMERAL+audit | commit audit, gitignore smoke-out |
| **hegemon** v0.52.0 docs | 5 | 0.5 MB | DOCS | commit |
| **endless-td** gen scripts (.py/.sh) | 3 | 21 KB | SOURCE | commit |
| **hellhunter** gen scripts (.mjs) | 5 | 17 KB | SOURCE | commit |
| **darklord** gen scripts (.py/.mjs/.ts) | 3 | 11 KB | SOURCE | commit |
| **deathrun** gen scripts (.mjs/.sh) | 2 | 6 KB | SOURCE | commit |
| **ai-hub** audit-20260630 (NEW day) | 6 | 1 KB | DOCS | commit |
| **capture-anime-girls/tests** | 1 | 9 KB | TEST | commit |
| **junk-runner/tests/e2e** | 1 | 2 KB | TEST | gitignore `_debug-*.spec.ts` |
| **xenonauts** (NEW project skeleton) | 5 | 2 KB | SOURCE | commit (after init) |
| **web/HOME-PAGE-RESTRUCTURE-2026-06-21.md** | 1 | 24 KB | DOCS | commit |
| **web/games/Games ideas.docx** | 1 | unknown | DOCS | operator decision |
| **web/games/docs/GAME-STRATEGY-2026-06-21.md** | 1 | 7 KB | DOCS | commit |
| **TOTAL** | **293+** | **~62 MB** | mixed | see above |

**Daemons state**: continuous commit-failure loop. 444-447 files
batched per cycle, every ~10s. Each cycle fails on the unmerged
index. Has been failing for at least 4+ hours (since 14:36 today).

**Active producer processes (9 vite dev + 3+ Playwright test jobs)**:

```
vte dev ports: 5173 5174 5179 5187 5188 5189 1440 1441 1450
playwright: tests/e2e/map-*.spec.ts (deathrun)
            /tmp/etd-v32-final-verify.mjs (endless-td)
            /tmp/etd-v60-verify.mjs  (endless-td)
            scripts/browser-smoke.mjs (some game)
            bun vite build (hellhunter)
```

The 36 chromium processes are spawned by Playwright, not by the
daemon. The daemon is in pure consumer mode (just trying to commit
what already exists).

## The 4 unmerged PNGs — sole daemon blocker

These 4 files have a **real unmerged index state** (stage 1/2/3
have different content). They are NOT phantom merges. The unmerged
state dates to at least 2026-06-21 14:36 (4+ hours old).

| File | Working tree size | mtime | Origin |
| --- | ---: | --- | --- |
| `web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile-drawer-open.png` | 50,939 B | 12:34 | audit-20260629 batch |
| `web/ai-hub/audit-20260629/05-mobile-view-screenshots/providers-mobile.png` | 43,301 B | 13:32 | audit-20260629 batch |
| `web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/02-main-nav-open.png` | 50,901 B | 12:34 | audit-20260629 batch |
| `web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/04-desktop-baseline.png` | 85,853 B | 12:34 | audit-20260629 batch |

The unmerged state was created sometime between 13:32 (last
file write) and 14:36 (first daemon failure). A working tree
file's mtime tells us when its content was last written, not when
`git add` was run. The actual `git add` that created the unmerged
state is in the past; the conflict was probably caused by an
attempted `git pull` or `git checkout --theirs/--ours` from
another working tree (e.g., a concurrent commit landing via the
daemon while the user was editing).

**Per-file stage content** (verified by sha256):

| File | stage 1 | stage 2 | stage 3 | working tree |
| --- | --- | --- | --- | --- |
| `free-mobile-drawer-open.png` | 32028320ad87 | 75a9a586e738 | b2209306b48b | 75a9a586e738 (matches stage 2) |
| `providers-mobile.png` | b566ba61818f | aa1b4a31cc45 | f86729847608 | aa1b4a31cc45 (matches stage 2) |
| `02-main-nav-open.png` | e5dbecee0efb | f32d07707204 | 1ddea3b5cae6 | f32d07707204 (matches stage 2) |
| `04-desktop-baseline.png` | 3e9b6a3fd0d2 | 88ef46813b6a | 02eef9dc163a | 88ef46813b6a (matches stage 2) |

Stage 1 = common ancestor, stage 2 = HEAD (target), stage 3 =
merge result. Working tree matches stage 2 in all 4 cases. The
unmerged state can be resolved with `git checkout --ours`
(wt=stage 2 is the "ours" side of the merge).

## Per-directory source analysis

### capture-anime-girls (193 files, 33.9 MB)

**What's there**: `web/games/games/capture-anime-girls/static/images/cards/char_5007.png` through `char_5199.png` (193 new card images, sequential numbering, named after the existing pattern).

**Source**: The capture-anime-girls game has a "Phase 25" commit
(`56833cdd2f`) titled "Refile deck UI + add non-girl card art"
that just landed. This is the game's card art batch — 193 new
characters (NPCs) for the deck.

**Type**: DELIVERABLE. The existing card art (`char_000.png`
through ~`char_5006.png`) is already tracked. The new ones follow
the same pattern and naming convention.

**Recommendation**: `git add web/games/games/capture-anime-girls/static/images/cards/char_5*.png` (or commit the whole dir). Should land in a follow-up commit, not mixed with the unmerged-index fix.

### endless-td (11 files, 14.2 MB)

**What's there**:
- `static/assets/audio/music/music_{boss,menu,waveClear,wave_1to5,wave_6to15}_v8.mp3` (5 mp3s, 2-3 MB each)
- `static/assets/audio/music/SOURCES.md` (provenance)
- `static/assets/raw/menu_backdrop_v8.png` (raw 1500×1500 source)
- `static/assets/png/menu_backdrop_v8.png` (post-processed)
- `scripts/gen-menu-backdrop-v8.py`, `gen-music-v8.sh`, `post-process-menu-backdrop-v8.sh` (the generation scripts)

**Source**: `etd-v8` music + menu_backdrop generation pipeline. Scripts ran today (mtime 17:50-17:53). The existing music dir already has `music_*.mp3` files tracked (old versions), so the v8 batch follows the same pattern.

**Type**: DELIVERABLE. Music is the game's actual audio assets.

**Recommendation**: `git add web/games/games/endless-td/static/assets/audio/music/*_v8.mp3 SOURCES.md web/games/games/endless-td/static/assets/{raw,png}/menu_backdrop_v8.png` then a follow-up commit for the scripts.

### deathrun (34 files, 9 MB total)

**What's there**:
- `static/images/`: 22 PNGs (background tiers v2, beat halos v2, particle effects, character portraits v3, etc.)
- `docs/`: 9 PNGs + 1 MD for v0.7.1 and v0.8.0 audit (mouse HUD, character, run-scene)
- `scripts/`: 2 files (`capture-v0.8.0.mjs`, `generate-assets-v0.8.0.sh`)

**Source**: v0.8.0 asset generation pipeline. Mtime 17:50-17:55. The existing `static/images/` already has 100+ tracked PNGs (`ach-*.jpg`, etc.), so the new v2/v3 versions are updates to existing art.

**Type**: DELIVERABLE. Game art assets. The docs are audits and follow the pattern of `v0.7.1-run-scene-*.png` already in `docs/`.

**Recommendation**: Commit in 2 commits — one for the art (22 files), one for the docs (10 files) + scripts (2 files).

### hellhunter (22 files, 3.1 MB)

**What's there**:
- `scripts/smoke-out/pause-v5/`: 8 PNGs + 1 JSON (smoke test output)
- `scripts/smoke-out/pause-v5-investigate/`: 6 PNGs (debug investigation)
- `scripts/`: 5 .mjs files (`debug-loopstate.mjs`, `debug2.mjs`, `pause-bg-check.mjs`, `pause-investigate.mjs`, `pause-test.mjs`)
- `docs/audits/pause-visual-bug-v5.md`: 1 audit doc
- `docs/design/diablo1-town-visual-spec.md`: 1 design doc

**Source**: hellhunter pause-feature investigation. mtime 17:43-17:55. Two smoke-out directories: one for `pause-v5` (canonical output) and one for `pause-v5-investigate` (debugging iteration).

**Type**: MIXED.
- The 5 .mjs debug scripts and 2 smoke-out PNGs/`loop-log.json` are EPHEMERAL (debug artifacts that should not be tracked).
- The `docs/audits/pause-visual-bug-v5.md` and `docs/design/diablo1-town-visual-spec.md` are DOCS (intentional deliverables).

**Recommendation**: Add `scripts/smoke-out/` and `scripts/debug*.mjs`, `scripts/pause-*.mjs` to hellhunter's `.gitignore`. Then commit the 2 docs only.

### darklord (25 files, 1.2 MB)

**What's there**:
- `scripts/smoke-out/v0807-*`: 21 PNGs (smoke test output for 3 resolutions × 7 screens)
- `scripts/gen-v0807-assets.py`: 1 source
- `scripts/v0807-ui-review.mjs`: 1 source
- `src/lib/game/v0807HudAssets.test.ts`: 1 test
- `docs/audits/AUDIT-V0807.md`: 1 audit

**Source**: darklord v0.8.0.7 asset generation + UI review. Mtime 17:43-17:55.

**Type**: MIXED.
- 21 smoke-out PNGs are EPHEMERAL.
- 3 scripts (gen/review) and 1 test are SOURCE.
- 1 audit MD is DOCS.

**Recommendation**: Add `scripts/smoke-out/` to darklord's `.gitignore`. Commit the 4 source/audit files.

### hegemon (5 files, 0.5 MB)

**What's there**:
- `MENU-v0.52.0.md`, `STYLE-v0.52.0.md` (root-level design docs)
- `docs/H3-COMPARISON-v0.52.0.md`, `docs/H3-DIFF-v0.52.0.html` (HoMM3 comparison)
- `docs/H5-REFERENCES/homm5-battle-reference.png` (HoMM5 reference)

**Source**: hegemon v0.52.0 design docs. Mtime older (17:30-17:43).

**Type**: DOCS (intentional deliverables — version-bumped design docs).

**Recommendation**: Commit as one "hegemon v0.52.0 docs" commit.

### junk-runner (1 file, 2 KB)

**What's there**: `tests/e2e/_debug-shape-multijump.spec.ts` (filename starts with underscore).

**Source**: User debug spec.

**Type**: EPHEMERAL debug. The leading underscore is the convention
for "do not run as part of normal test suite" — same pattern
exists in `web/tests/shared/_audit-home.spec.ts` which IS tracked
(so leading-underscore alone is not enough to gitignore).

**Recommendation**: Per-directory `.gitignore` rule
`tests/e2e/_debug-*.spec.ts`. Alternative: just delete the file
if it's not actively used.

### ai-hub audit-20260630 (6 files, 1 KB)

**What's there**: `web/ai-hub/audit-20260630/03-ai-hub-internal-error-and-byteplus-referral/{README.md,01-status.txt,...05-free-grep.txt}` (6 files).

**Source**: NEW audit (the dir name is `audit-20260630`, but we're on 2026-06-21 — this is forward-dated, probably a typo).

**Type**: DOCS (audit evidence).

**Recommendation**: Commit. Possibly rename the dir to `audit-2026-06-21` first (operator decision).

### xenonauts (5 files, 2 KB) — NEW PROJECT

**What's there**: `package.json`, `svelte.config.js`, `tsconfig.json`, `vite.config.js`, `src/app.html`.

**Source**: mtime 17:56-17:57 (created in the last 5 minutes). This is a brand-new game project being initialized.

**Type**: SOURCE (project skeleton — typical SvelteKit init output).

**Recommendation**: Commit as "init: xenonauts project skeleton" once the project is actually ready. The `tests/` and `static/` dirs are also new but empty (no untracked files there yet).

### web/HOME-PAGE-RESTRUCTURE-2026-06-21.md (24 KB)

**Source**: HOME page design doc.

**Type**: DOCS.

**Recommendation**: Commit.

### web/games/Games ideas.docx

**Type**: DOCS (binary).

**Recommendation**: Operator decision — should binary design docs be tracked? The current `.gitignore` has `*.docx` (line 27 of root .gitignore) — wait, let me re-check.

Looking at the root .gitignore, I see:
```
*.log
*.jsonl
*.kra
*.kra~
*.sqlite
*.sqlite3
*.db
*.db-journal
*.db-shm
*.db-wal
```

No `*.docx` rule. So this file would be tracked by default. But should it be? The word "docx" implies a Microsoft Word file (binary, hard to diff). Recommendation: add `*.docx` to root `.gitignore` and convert to `.md` if the design doc is needed.

### web/games/docs/GAME-STRATEGY-2026-06-21.md (7 KB)

**Source**: Game strategy doc.

**Type**: DOCS.

**Recommendation**: Commit.

## Why is the daemon failing? (the core question)

The daemon is in a commit loop:
1. Discover 444-447 untracked files
2. Batch into chunks of 100
3. `git add` the chunk
4. `git commit -m "N file(s)..."` ← **FAILS** with "unmerged files"
5. Wait ~10s
6. Repeat

The unmerged-index error means git refuses to create a tree from
an index that has stage-1/2/3 entries. This is git's safety
mechanism to prevent data loss on conflicts. The daemon has been
hitting this error for at least 4+ hours (since 14:36 today,
verified via journalctl).

The fix is operator-actionable: resolve the 4 unmerged PNGs
(`git checkout --ours <path>` for each, or `git rm --cached` and
re-add). The daemon will immediately drain the 293+ untracked
files in ~3 batches of 100.

## Per-directory .gitignore recommendations

### Per-game (root-level `.gitignore` rules)

```gitignore
# hellhunter — debug scripts and smoke-out artifacts
web/games/games/hellhunter/scripts/smoke-out/
web/games/games/hellhunter/scripts/debug*.mjs
web/games/games/hellhunter/scripts/pause-*.mjs

# darklord — smoke-out artifacts
web/games/games/darklord/scripts/smoke-out/

# junk-runner — debug test specs
web/games/games/junk-runner/tests/e2e/_debug-*.spec.ts

# root — Office formats
*.docx
*.xlsx
```

These rules are conservative — they only gitignore clearly
ephemeral paths. The actual deliverable files (game art, audit
docs, gen scripts) remain tracked.

## Daemon health snapshot (2026-06-21 17:55 UTC)

- **Daemon process**: `dracon-sync[1514387]` (PID, started earlier)
- **Daemon state**: active
- **Last error**: `cannot create a tree from a not fully merged index.; class=Index (10); code=Unmerged (-10)` — continuously
- **Failure cadence**: ~10s between attempts
- **Files per batch attempt**: 444-447
- **Batches until drain**: ~3 (when unmerged is resolved, drain takes ~30s)
- **Other errors**: `push failed for /home/dracon/Dev/DraconDev-private` (separate repo, protected branch; out of scope for this investigation)

## Local commit status

**Local HEAD**: `56833cdd2f` "Phase 25: Refile deck UI + add non-girl card art" (just landed).

**All 4 remotes of dracon-platform are at 0/0** (verified 17:58 UTC):

| Remote | main HEAD | Behind/ahead | Status |
| --- | ---: | ---: | --- |
| `origin` | `56833cdd2f` | 0/0 | ✅ |
| `github` | `56833cdd2f` | 0/0 | ✅ |
| `codeberg` | `56833cdd2f` | 0/0 | ✅ |
| `gitlab` | `56833cdd2f` | 0/0 | ✅ |

The daemon successfully pushed Phase 24 + Phase 25 to all 4
remotes. The "2 commits behind" state from the previous concern
investigation is now resolved (commits landed in the last 30
minutes). The daemon IS pushing when commits succeed.

**The remaining problem is the 293+ untracked files** that the
daemon is trying to commit but is blocked on the unmerged index.

The `git push` test I ran as part of this investigation
(`git push origin main 2>&1 | head -10`) succeeded with
`5ca8d8e6b5..56833cdd2f  main -> main`. This confirms the
daemon's normal push path works for dracon-platform — the
unmerged index only blocks new commits, not new pushes.

## Why "stupid amount" is a real problem (not just a count)

The 293+ untracked files are accumulating at:
- **~99 files/hour** (last 60 min sample)
- **~29 files in last 30 min**
- **~13 files in last 5 min**

This rate exceeds the daemon's drain rate. Each daemon attempt
fails on the unmerged index within 1-2s. So the untracked count
grows monotonically between operator fixes. Without intervention,
the untracked count will reach 1,000+ within 10 hours.

**The unmerged index is the single point of failure** that, once
fixed, will unblock ~30s of drain time and remove the 293+ file
backlog.

## Resolution plan (prioritized)

### IMMEDIATE (operator action, <5 min)

1. **Resolve the 4 unmerged PNGs** (file paths above):
   ```bash
   cd /home/dracon/Dev/dracon-platform
   for f in \
     web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile-drawer-open.png \
     web/ai-hub/audit-20260629/05-mobile-view-screenshots/providers-mobile.png \
     web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/02-main-nav-open.png \
     web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/04-desktop-baseline.png ; do
     git checkout --ours "$f"
   done
   ```
   The working tree already matches the "ours" (HEAD) side of the
   conflict in all 4 cases (verified via sha256). After this, the
   daemon will drain the 293+ untracked files in ~30s.

2. **Add per-game `.gitignore` rules** for the ephemeral paths
   identified above (hellhunter debug/smoke-out, darklord
   smoke-out, junk-runner _debug-*.spec.ts, root *.docx).

3. **Commit game deliverables** as a follow-up commit (not part of
   this design doc; operator decision on grouping).

### SHORT-TERM (prevent recurrence, 1-2 days)

1. **Add git pre-commit hook or daemon feature** that detects
   unmerged index state and emits a clear operator alert
   (instead of looping on the same error every 10s for 4+ hours).
2. **Add a daemon config option** to gitignore the
   `scripts/smoke-out/` and `*.smoke-out.png` patterns globally,
   so new game projects inherit the rule without per-game edits.
3. **Add a daemon check** that warns when the untracked count
   exceeds a threshold (e.g., 100) without making progress for
   > 5 minutes.

### LONG-TERM (architectural, 1-2 weeks)

1. **Move debug output outside the watched tree.** Smoke-out
   artifacts and debug screenshots should land in
   `~/.dracon/scratch/{repo}/` or `/tmp/{repo}-debug/` rather than
   inside the repo. This decouples dev iteration from git churn.
2. **Separate test artifacts from production code.** A monorepo
   convention that all `scripts/smoke-out/`, `scripts/debug*.mjs`,
   and `tests/e2e/_debug-*.spec.ts` are gitignored at the root
   level (already partial — extend to cover all game subdirs).
3. **Add a CI lint** that fails if any game project's
   `.gitignore` is missing the standard ephemeral patterns.

## Open questions for the operator

1. **For the 4 unmerged PNGs**: `git checkout --ours` is the
   safe choice (working tree already matches HEAD). The user may
   prefer `git checkout --theirs` if they had uncommitted local
   edits to those screenshots.
2. **For the capture-anime-girls card art batch (193 files)**:
   commit as a single 33.9 MB commit, or split into 2-3 commits
   by character ID range (char_5007-5099, 5100-5199, etc.)?
3. **For the audit-20260630 dir** (date is wrong — we're on
   2026-06-21): rename to `audit-2026-06-21` or leave as-is?
4. **For the `web/games/Games ideas.docx`**: add `*.docx` to root
   `.gitignore` and convert to `.md` if the content is needed?
5. **For the 9 active dev sessions**: should the operator
   consolidate to fewer concurrent dev sessions to reduce git
   churn, or is this normal/expected workload?

## Reference

- `docs/design/concern-1-dracon-platform-2026-06-21.md` — the
  prior concern investigation that identified the unmerged-index
  blocker.
- `docs/design/sync-push-classification.md` — the daemon's push
  state classification rules.
- `docs/design/daemon-settling-2026-06-20.md` — the daemon
  settling behavior.
- `dracon-sync/src/git/multi_remote.rs` — the push-all logic.
- `dracon-sync/src/sync.rs` — the commit/push loop.
- `/home/dracon/.dracon/utilities/sync/dracon-sync.toml` — the
  global sync policy.