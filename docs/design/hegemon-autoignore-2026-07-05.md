# Hegemon auto-ignore investigation — 2026-07-05

User question: "we should just gitinit the .pi no? if that is the
problem seemingly — if we did that then problem solved forever, chrome
screenshots that is why we are on this journey at all — we should
investigate what folders we should be auto ignoring"

## TL;DR — what folders should be auto-ignored

The user's instinct is correct: **`.pi/chrome-screenshots/` and
`.pi/visual-audit/` should never be committed to git.** These are
session artifacts (Playwright screenshot dumps), not source code or
assets.

**Recommended gitignore additions** (hegemon + standard WIP-game template):

```gitignore
# Session artifacts — Playwright screenshot dumps from regen rounds
.pi/chrome-screenshots/
.pi/visual-audit/
```

**Additionally** (hegemon-specific, not in standard template):

```gitignore
# Investigation ad-hoc PNGs (audit screenshots)
.pi/investigation/*.png

# HoMM3 reference images (static reference material)
.pi/investigation/h3-refs-r3/

# Pre-v1.0 backup dirs from asset regeneration
static/assets/.backup-*/
static/assets/*pre-v1.png
```

**Belt-and-suspenders** (`.dracon/dracon-sync.toml`):

```toml
auto_commit_exclude_patterns = [
    "**/.pi/chrome-screenshots/**",
    "**/.pi/visual-audit/**",
    "**/.pi/investigation/**/*.png",
]
```

**Estimated savings**: ~1.87 GB unpacked from tracked index.

## 1. What .pi/ directories currently exist in hegemon?

| Directory | Working tree | Tracked files | Tracked MB |
|-----------|-------------:|-------------:|----------:|
| `.pi/chrome-screenshots/` | **1.9 GB** | **1,061** | **1,821 MB** |
| `.pi/investigation/` | 106 MB | 258 | 105 MB |
| `.pi/visual-audit/` | 30 MB | 33 | 29 MB |
| `.pi/goals/` | 764 KB | 30 | <1 MB |
| `.pi/tasks/` | 4 KB | 0 | 0 |
| **Total** | **~2.0 GB** | **1,382** | **~1,956 MB** |

`.pi/chrome-screenshots/` is **50% of hegemon's entire pack size**.

## 2. Why .pi/chrome-screenshots/ exists

The `.pi/cap.mjs` script is a 20-line Playwright/Chromium screenshot
tool that captures game states during regen rounds:

```js
const browser = await chromium.launch({ headless: true, ... });
await page.goto(`${url}/play?seed=${seed}&reveal=all&camera-zoom=1`);
await page.screenshot({ path: out, fullPage: false });
```

It writes to `.pi/chrome-screenshots/` because that's where the
scripts dump their output. The daemon then auto-commits these files
because `.gitignore` does not exclude `.pi/chrome-screenshots/`.

The directory was **invented 8 days ago** (first commit
`hegemon-map-v1.png` on 2026-06-27). Since then, it has accumulated
1,061 files totaling 1,821 MB — **95% of that in the last 48 hours**.

## 3. Why this is a problem — but only for hegemon

All 8 WIP games share the same `.gitignore` template. None of them
exclude `.pi/chrome-screenshots/`. But only hegemon actively runs
`.pi/cap.mjs` for visual iteration — the other games don't accumulate
this bloat.

| Game | .pi/chrome-screenshots tracked | Pack size |
|------|-------------------------------:|----------:|
| **hegemon** | **1,061 files / 1,821 MB** | **4,096 MB** |
| deathrun | 290 files / 147 MB | 224 MB |
| darklord | 156 files / 125 MB | 75 MB |
| endless-td | 8 files / 2 MB | 224 MB |
| polis | 2 files / <1 MB | 64 MB |
| neonbreak | 2 files / <1 MB | 202 MB |
| capture-anime-girls | 1 file /