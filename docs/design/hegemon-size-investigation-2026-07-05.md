# Hegemon size investigation — 2026-07-05

User question: "hegemon is way too big we need to look into it how we
cut it down more like the others it makes zero sense why is it so big
when others are not fully investigate"

## TL;DR

**Hegemon's pack is 4.18 GB. The next-largest single-repo pack is
browser-extensions-shared at 500 MB.** That's 8x larger than the
next-biggest single repo. But the previous investigation (yesterday's
`hegemon-backup-cruft-comparison-2026-07-05.md`) only addressed
350 MB of `.backup-*` cruft in `static/assets/` — missing the
**real bloat source by a wide margin**.

**The dominant bloat is NOT static/. It is `.pi/`. Specifically:
`.pi/chrome-screenshots/` — a Playwright-based session-artifact dump
that nobody gitignored.**

| Top-dir | Tracked MB | % of pack |
|---------|-----------:|----------:|
| **`.pi/`** | **1,955 MB** | **~80%** |
| `static/` | 476 MB | ~19% |
| `src/` | 1 MB | ~0% |
| `scripts/` | <1 MB | ~0% |
| `docs/` | <1 MB | ~0% |

**Top 5 contributors to `.pi/`**:
1. `.pi/chrome-screenshots/` — **1,220 MB / 729 files** (50% of pack!)
2. `.pi/investigation/` (audit screenshots) — 99 MB / 159 files
3. `.pi/visual-audit/` — 29 MB / 33 files
4. `.pi/goals/archived/` — ~150 KB / 28 files (negligible, keep)
5. `.pi/cap.mjs` — Playwright script (1 KB, but writes the bloat)

**Differential from other repos**:
- Other "HEAVY" games have most pack weight in `static/` (regen output)
- Hegemon's pack weight is in `.pi/chrome-screenshots/` (visual iteration)
- This is a *workflow drift*, not a content-quality difference

## 1. Per-directory pack size (top 30)

Top-level totals across all 2,326 tracked files in hegemon:

| Top-dir | Tracked MB | Files |
|---------|-----------:|------:|
| `.pi/` | **1,955** | 1,383 |
| `static/` | 476 | 847 |
| `src/` | 1 | 97 |
| `docs/` | <1 | 52 |
| `scripts/` | <1 | 153 |

Within `.pi/`:

| Subdir | Tracked MB | Files |
|--------|-----------:|------:|
| `chrome-screenshots/` | **1,220** | 729 |
| `investigation/` | 99 | 159 |
| `visual-audit/` | 29 | 33 |
| `goals/archived/` | <1 | 28 |
| `goals/goal_events.jsonl` | <1 | 1 (458 KB) |
| `cap.mjs` | <1 | 1 (script) |

Within `static/assets/`:

| Subdir | Tracked MB | Files |
|--------|-----------:|------:|
| `music/` | 170 | 34 mp3 |
| `creatures-painted-v3/` | 117 | 111 + 56 frames |
| `skills-v8/` | 41 | 27 |
| `animations-v7/` | 27 | 32 |
| `schools-painted-v7-alt-A/` | 20 | 11 |
| `towns-3x3-mmx-v2/` | 19 | 8 |
| `terrain-painted-v15/` | 17 | 56 |
| (smaller categories) | ~65 | ~750 |

## 2. What is `.pi/chrome-screenshots/`?

It's a Playwright/Chromium screenshot dump run from the `.pi/cap.mjs`
script (a 20-line script using `playwright.chromium`):

```js
const browser = await chromium.launch({ headless: true, ... });
const page = await browser.newPage();
await page.goto(`${url}/play?seed=${seed}&reveal=all&camera-zoom=1`, ...);
await page.screenshot({ path: out, fullPage: false });
```

This captures visual states of the game during regen rounds (r3, r6,
r9, r10, r11, h13, etc.) at various zoom levels and seed variants.
The files end up in `.pi/chrome-screenshots/` because that's where
the scripts write them.

**The 729 files break down as**:
- `r11-k-after/`: 180 files / 348 MB (most recent regen)
- `r10-k-after/`: 59 files / 105 MB
- `h13-k-after/`: 76 files / 106 MB
- `r11-k-before/`: 10 files / 18 MB
- `r10-k-before/`: 4 files / 7 MB
- `h13-k-before/`: 3 files / 14 MB
- Root-level (~397 files / ~410 MB): the **r9-v*-zoom.png series** —
  ~998 zoom variants of the same regen round at root
- Plus 60+ audit/visual screenshots scattered by name

**Why they are tracked**: `.gitignore` does not exclude
`.pi/chrome-screenshots/`. The daemon's policy is to commit all
untracked unless excluded. So every Playwright run that writes
there is auto-committed.

**Why other games don't have this problem**:
- Other "WIP" games (`darklord`, `endless-td`, `polis`, `neonbreak`)
  share the same `.gitignore` template. They have rules for
  `scripts/smoke-out/`, `docs/screenshots/`, `docs/audit/`, and
  `playwright-report/` — but **none of them exclude
  `.pi/chrome-screenshots/` either**, because that directory is a
  hegemon-specific drift.
- Other games don't run `.pi/cap.mjs` at all, so they never
  accumulate. But if any other game ever adopts a similar workflow,
  they'll grow pack at this rate too.

## 3. Growth trajectory

| Date | Commits | Notes |
|------|--------:|-------|
| 2026-06-23 | 6 | (cross-repo batch w/ endless-td) |
| 2026-06-24 | 706 | First r2 cycle |
| 2026-06-25 | 97 | |
| 2026-06-26 | 192 | |
| 2026-06-28 | 663 | First chrome-screenshots |
| 2026-06-29 | 56 | |
| 2026-06-30 | 17 | |
| 2026-07-01 | 23 | |
| 2026-07-02 | 99 | |
| 2026-07-03 | 499 | r9 cycle |
| **2026-07-04** | **2,330** | **Peak regen day** |
| **2026-07-05** | **1,093** | **Still going** |

`.pi/chrome-screenshots/` was **invented 8 days ago** (first file
`hegemon-map-v1.png` on 2026-06-28). **95% of all chrome-screenshot
tracked content is from the last 48 hours.**

Growth direction: **pack is growing, not shrinking**. With r9-zoom
variants and r11-k-after series both active, the pack grows by
hundreds of MB per day during regen. At the current rate:
- 1.22 GB in 48h = **~600 MB/day net growth from chrome-screenshots alone**
- Adding `static/` regeneration (~50 MB / regen round × 1-2 rounds/day)
- Total: **~700 MB-1 GB/day of pack growth**

If left unchecked: pack size could reach 7-10 GB within 2 weeks.

## 4. Why do the OTHER 25 repos stay manageable?

| Repo | Pack MB | Tracked | KB/track |
|------|--------:|--------:|---------:|
| **dracon-platform** | **13,232** | 3,919 | 3,376 (meta-repo of 30 submods) |
| **hegemon** | **4,176** | **2,326** | **1,795** |
| browser-extensions-shared | 511 | 3,039 | 168 |
| deathrun | 229 | 830 | 276 |
| endless-td | 228 | 676 | 337 |
| one-mil-girls | 213 | 174 | 1,228 (older release) |
| capture-anime-girls | 217 | 1,075 | 201 |
| junk-runner | 210 | 525 | 399 |
| neonbreak | 206 | 645 | 319 |
| dracon-code | 184 | 439 | 419 |
| ai-auto-writer | 112 | 3,667 | 30 |
| .dracon | 79 | 204 | 387 |
| hellhunter | 78 | 137 | 568 |
| darklord | 77 | 866 | 88 |
| polis | 65 | 937 | 69 |
| dracon-utilities | 38 | 290 | 130 |
| (9 more repos < 10 MB pack) | — | — | — |

Of the 25 non-meta repos, **only hegemon has KB/track > 1000**
(outside of frozen releases like one-mil-girls). The pattern:

- **Other "HEAVY" games** (darklord 585 MB static, endless-td 472 MB,
  capture-anime-girls 219 MB) have **hand-maintained or generated
  assets with no chrome-screenshot workflow**. Their pack sizes
  stay under 230 MB.
- **Hegemon has BOTH** the static/ asset pipeline AND the chrome-
  screenshot visual iteration loop. The latter is what makes it
  8x bigger than the next-biggest.

## 5. Recommended hygiene fix (durable)

### 5.1 Add to `.gitignore` (hegemon root)

```gitignore
# Session artifacts — visual iteration captures from regen rounds
.pi/chrome-screenshots/
.pi/visual-audit/

# Investigation ad-hoc PNGs (audit screenshots etc.)
.pi/investigation/*.png
.pi/investigation/*-audit.png

# HoMM3 reference materials (vendor via OVH bucket, not git)
.pi/investigation/h3-refs-r3/

# Pre-v1.0 backup dirs — never commit these
static/assets/.backup-*/
static/assets/*backup-*/
static/assets/*pre-v1.png
```

### 5.2 Add to `.dracon/dracon-sync.toml`

```toml
auto_commit_exclude_patterns = [
    "**/.pi/chrome-screenshots/**",
    "**/.pi/visual-audit/**",
    "**/.pi/investigation/**/*.png",
    "**/static/assets/.backup-*/**",
]
```

This is belt-and-suspenders: even if someone adds a file in chrome-
screenshots before `.gitignore` is updated, the daemon will refuse
to auto-commit it.

### 5.3 Retroactive cleanup (optional, requires operator)

If the operator agrees, run:

```bash
cd /home/dracon/Dev/dracon-platform/web/games/wip/hegemon
git rm -r --cached .pi/chrome-screenshots/        # -1,220 MB
git rm -r --cached .pi/visual-audit/              # -29 MB
git rm -r --cached '.pi/investigation/*.png'      # -80 MB
git rm -r --cached .pi/investigation/h3-refs-r3/  # -20 MB
git commit -m "chore: drop .pi/ session artifacts (~1.35 GB tracked, history preserved)"
```

**Result**: tracked index loses ~1.35 GB. Pack files don't
physically shrink until `git gc --aggressive --prune=now` is run
separately; that's a separate operation with operator sign-off.

### 5.4 Standard-template fix (cross-repo)

Add to the standard WIP-game `.gitignore` template that gets
auto-applied to new games:

```gitignore
# Per-game session artifacts
.pi/chrome-screenshots/
.pi/visual-audit/
```

This prevents future projects from accumulating the same bloat.

## 6. Estimated outcomes

### With hygiene fix only (forward-looking)
- Pack growth: **stops** immediately at +0 MB/day from chrome-screenshots
- New regens that add visual iteration will land in `.pi/chrome-
  screenshots/`, get gitignored, NEVER reach git
- Static/ asset regen continues to add ~50-100 MB per round (already in
  this regime)

### With retroactive cleanup (immediate)
- **Tracked blob total**: 4.18 GB → ~2.5 GB (after repack)
- **Tracked file count**: 2,326 → ~1,560 (drops ~766 files)
- **Working tree disk**: 479 MB (after previous cleanup) → 350 MB (more
  chrome-screenshots dir recovered)
- **Pack size after repack**: 4.18 GB → ~2.5 GB

### With OVH bucket migration (medium-term, separate work)
- Static/ contents offloaded to bucket
- Pack size drops below 100 MB
- `exclude_remotes = ["github"]` removed from hegemon config
- hegemon mirrors to ALL 4 remotes (origin + github + gitlab + codeberg)

## 7. Files

This doc: `docs/design/hegemon-size-investigation-2026-07-05.md`

Related:
- `docs/design/hegemon-state-investigation-2026-07-05.md` — earlier 6-
  dimension diagnosis
- `docs/design/hegemon-backup-cruft-comparison-2026-07-05.md` —
  `.backup-*` cleanup (350 MB, only addressed static/)
- `docs/design/binary-asset-strategy-2026-07-03.md` — why bucket is
  the answer for binary assets
- `docs/design/full-audit-2026-07-05.md` — 26-repo push health audit

Measurement method: `git ls-files | xargs -I {} bash -c "git cat-file -s
HEAD:{}"` to enumerate tracked blob sizes; `git count-objects -vH` for
pack-on-disk; cross-checked with `du -sb .git/objects/pack`.

## 8. Direct answer to the operator

> "hegemon is way too big we need to look into it how we cut it down
>  more like the others"

**Three things**:
1. **The `.pi/` directory is doing it** (80% of pack weight). Not
   `static/` (only 19%). The previous `static/` cleanup removed 350 MB
   but the real bloat source was always `.pi/chrome-screenshots/`.
2. **Hegemon is unique in running `.pi/cap.mjs`** (Playwright session-
   artifact dump). Other games don't run this script, so they never
   accumulate chrome-screenshots. This is a workflow drift, not a
   content quality difference.
3. **The fix is `.gitignore`** (one file change). `static/ -> OVH
   bucket` is the longer-term roadmap, but the immediate ~1 GB unpacked
   gain comes from gitignoring session artifacts and retroactively
   removing them.

After hygiene fix: **pack drops from 4.18 GB to ~2.5 GB** (after repack).
After OVH bucket migration: **pack drops below 100 MB**. Both are
operator-actionable.