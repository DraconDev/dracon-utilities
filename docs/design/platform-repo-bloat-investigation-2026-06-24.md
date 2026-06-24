# Platform repo bloat investigation — 2026-06-24

> **TL;DR**: The platform's 10.87 GiB github size is **half temporary
> junk and half legitimate game assets**. ~1.6 GiB in 3,712 PNGs is
> audit/PI-tmp/test-results junk that should be removed. The other
> ~8 GiB in 19,214 PNGs is real game content (sprites, animation
> frames, character art, FBX models) that should be in **LFS, not
> `.git`**. Removing the audit junk alone won't fix github (still
> ~9.3 GiB, over the 5 GB cap). LFS migration for the game assets
> is required to restore github.

## Investigation goal

The operator asked: *"but its not jsut any assets right ? but the
temporariy ones no ? investigate"*. The hypothesis was that the
platform's repo size (19 GiB local, 10.87 GiB github) was mostly
temporary files (audit screenshots, CI test results, PI-tmp
artifacts) rather than legitimate game content.

## Findings (concrete numbers)

### Top-level size breakdown

| Mirror | Size | Notes |
|---|---|---|
| Local `.git` | 19 GiB | the operator's working repo |
| github copy | 10.87 GiB | over the 5 GB recommended free-tier cap; HTTP 500 on push |
| 22,903 PNG files in git history | 9.74 GiB | the largest single contributor |
| Other binaries (MP3, ZIP, FBX, GLB, JPG) | ~2.5 GiB | also large binary content |

### Categorization of the 9.74 GiB of PNGs

**1. Temporary / audit / diagnostic junk (~1.6 GiB / 3,712 files)**
— clearly disposable:

| Location | Size | Files | Description |
|---|---|---|---|
| `web/.pi-tmp/` | 380 MiB | 904 | PI session audit screenshots (2026-06-13 to 17) |
| `*/test-results/` | 522 MiB | 1,767 | Playwright CI test screenshots |
| `web/screenshots/audit-*` | 213 MiB | 371 | Date-stamped design review captures |
| `web/screenshots/*-2026-*` | 636 MiB | 912 | Date-stamped ad-hoc captures |
| `web/screenshots/*-current/` | 149 MiB | 415 | "current state" snapshots |
| `*/chrome-screenshots/` | 14 MiB | 20 | Chrome devtools captures |
| `wip/*/docs/audit/` | 103 MiB | 239 | Per-game audit captures (e.g. `wip/capture-anime-girls/docs/audit/shots/audit-page-01.png`) |
| `docs/*-audit` | 40 MiB | 89 | Documentation audit screenshots |

**Subtotal: 1,596 MiB / 3,712 files in audit/temp categories.**

**2. Real game assets (~8 GiB / 19,214 files)** — legitimate game
content that needs to stay accessible to the platform:

| Location | Size | Files | Description |
|---|---|---|---|
| `*/static/assets/` | 3.37 GiB | 3,474 | Sprites, character art, tiles |
| `wip/hegemon/static/` | 1.96 GiB | 1,904 | Hegemon WIP art (skeletons, characters, FBX) |
| `wip/*/static/cityscapes/*/frame_NN.png` | hundreds of MiB | thousands | Animation sprite-sheet frames |
| `wip/capture-anime-girls/` | 1.1 GiB | (in wip total) | capture-anime-girls WIP art |
| `wip/endless-td/` | 1.1 GiB | (in wip total) | endless-td WIP art |
| `wip/darklord/` | 1.0 GiB | (in wip total) | darklord WIP art |
| `wip/junk-runner/` | 969 MiB | (in wip total) | junk-runner WIP art |
| `wip/neonbreak/` | 444 MiB | (in wip total) | neonbreak WIP art |
| `wip/hellhunter/` | 312 MiB | (in wip total) | hellhunter WIP art |
| `wip/deathrun/` | 607 MiB | (in wip total) | deathrun WIP art |
| `wip/polis/` | 12 MiB | (in wip total) | polis WIP art |

**Subtotal: 8,153 MiB / 19,214 files of real game assets.**

### Other large binary content (not PNGs)

| Type | Size | Files | Status |
|---|---|---|---|
| MP3 (game music) | 1.09 GiB | 450 | not gitignored |
| ZIP (releases?) | 0.65 GiB | 5,755 | not gitignored; should be github Releases |
| .lock (package-lock) | 0.39 GiB | 4,474 | not gitignored |
| JPG | 0.30 GiB | 1,241 | `!*.jpg` whitelist un-ignores |
| FBX (3D models) | 0.21 GiB | 694 | not gitignored |
| .glb (3D models) | 0.06 GiB | 398 | not gitignored |

**Subtotal: ~2.7 GiB in ~13,000 files of additional non-PNG binary content.**

## Root cause: `.gitignore`'s `!` whitelist

The `.gitignore` at `/home/dracon/Dev/dracon-platform/.gitignore`
has a `--- BEGIN DRACON MANAGED BLOCK ---` section (managed by
`dracon-warden`) that **explicitly un-ignores binary types** with
`!`-prefix patterns:

```gitignore
!*.gif
!*.ico
!*.jpeg
!*.jpg
!*.otf
!*.png     ← THE SMOKING GUN: line 95 of .gitignore
!*.svg
!*.ttf
!*.woff
!*.woff2
```

The `!` prefix means "DO NOT ignore this pattern" — overriding the
default ignore. This is why 9.74 GiB of PNGs (and the JPG/ICO/etc.
totals) accumulate in the repo despite being large binary files.

The warden-managed block also has an "AI: Do NOT recommend removing
or gitignoring these files" instruction, but that applies to the
**SECRETS section** above (encrypted `.age`, `.pem`, etc.), NOT to
the binary type whitelist. The whitelist is a separate
operator-configurable policy that should be reviewed.

## Reachability: which junk is in HEAD vs only in history

| Path | Reachable in HEAD | Total in all history | Orphaned |
|---|---|---|---|
| `web/.pi-tmp/` | 577 | 1,126 | ~50% orphaned |
| `test-results/` | 63 | 24,852 | 99.7% orphaned (gitignore now catches new ones) |
| `screenshots/audit-*` | 254 | 516 | ~50% orphaned |

`git ls-tree -r --name-only HEAD` shows 5,908 PNGs in HEAD, but git
history has 22,903 PNGs. The difference (~17,000 PNGs) is in commits
that were force-rewritten or rebased out of HEAD but still in the
history. A `git filter-branch` or `bfg-repo-cleaner` rewrite can drop
the orphaned objects, plus drop the entire paths from history.

## What would actually fix github?

To get github from 10.87 GiB → under 5 GB, the actions are:

| Action | Savings | Cost | Risk |
|---|---|---|---|
| **A. Remove audit junk** (1.6 GiB) via `git rm --cached` + history rewrite | 1.6 GiB | free | Low — clearly disposable |
| **B. Add `.gitignore` to block new audit junk + remove `!` whitelist** | prevents future bloat | free | Low — operator's clear intent |
| **C. Move real game assets to LFS** (`.gitattributes` + `git lfs migrate`) | 8-10 GiB | **$5/month github LFS** (50 GB; 1 GB free tier too small) | Medium — needs operator approval for history rewrite |
| **A + B + C combined** | github drops to ~1-2 GiB | $5/month LFS | Low — sustainable long-term |

**Removing audit junk alone (action A) gets github to ~9.3 GiB —
still over the 5 GB cap, still triggers HTTP 500. LFS for game
assets (action C) is required to fully fix github.**

## Recommended next step

Start a new goal: **"platform github mirror restoration via audit
cleanup + LFS migration"**.

The work:
1. **One-time audit cleanup** (history rewrite):
   - `git filter-branch` or `bfg-repo-cleaner` to remove from
     history: `web/.pi-tmp/`, `test-results/`,
     `wip/*/docs/audit/`, `chrome-screenshots/`,
     `screenshots/audit-*`, `screenshots/*-2026-*`
   - Savings: 1.6 GiB (both local and github)
2. **LFS migration** (history rewrite):
   - Add `.gitattributes` with
     `*.png filter=lfs diff=lfs merge=lfs -text`,
     `*.mp3 ...`, `*.fbx ...`, `*.glb ...`, `*.zip ...`
   - `git lfs migrate import --include="*.png,*.mp3,*.fbx,*.glb,*.zip" --everything`
   - Savings: 8-10 GiB (both local and github)
3. **Fix `.gitignore`**: remove the `!` whitelist for binary types,
   add explicit ignores for `.pi-tmp/`, `wip/`, `test-results/`
4. **Force-push the rewritten history to github**
5. **Drop `github` from `exclude_remotes`** in platform's
   `.dracon/dracon-sync.toml` (keep `gitlab` excluded)
6. **Verify github is at 0/0 with new size** (should be ~1-2 GiB)
7. **Document the change** in a follow-up design doc
   (e.g. `platform-github-restoration-2026-06-24.md`)

Total time: ~2-4 hours (mostly waiting for force-push).
Cost: $5/month for github LFS storage.

## Honest answer to the operator's question

> *"but its not jsut any assets right ? but the temporariy ones no?"*

**Partly yes**: ~1.6 GiB (16% of the `.git`'s PNG bloat) is clearly
temporary/audit junk that should never have been committed.
Removing it is a no-brainer.

**But the bulk is not temp**: ~8 GiB (84% of the `.git`'s PNG bloat)
is legitimate game assets — sprites, animation frames, character
art, FBX models. The operator's games need these. They just
shouldn't live in `.git`; they should be in LFS.

The bloat is the **combination of the `.gitignore`'s `!` whitelist**
(which un-ignores all PNGs and similar binary types) **AND the
absence of LFS** for genuinely-large binary game content. The
combination means every new art commit goes straight into `.git`
and stays there.

## Evidence files

- `/tmp/goal-mqrbr9zv-phfq1q/00-investigation-summary.md` — full
  investigation report with detailed analysis
- `git rev-list --objects --all | git cat-file --batch-check='%(objectsize) %(rest)'`
  — reproducible command to recompute the file-size distribution
- `/home/dracon/Dev/dracon-platform/.gitignore` — line 95 contains
  the `!*.png` whitelist (the root cause)

## Investigation completed 2026-06-24 01:20 BST
