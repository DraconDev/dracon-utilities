# What folders should be auto-ignored — 2026-07-05

User: "we should investigate what folders we should be auto ignoring"

## Answer

`.pi/chrome-screenshots/` and `.pi/visual-audit/` are the problem.
They are session artifacts — not code, not assets, not config. They
should never be committed.

**Add to `.gitignore`** (hegemon + standard WIP-game template):

```gitignore
# Session artifacts — Playwright screenshot dumps from regen rounds
.pi/chrome-screenshots/
.pi/visual-audit/
```

**Additionally** (hegemon-specific):

```gitignore
# Investigation ad-hoc PNGs (audit screenshots)
.pi/investigation/*.png

# HoMM3 reference images
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

## What's tracked now

| Directory | Tracked | MB | Verdict |
|-----------|--------:|----:|---------|
| `.pi/chrome-screenshots/` | 1,108 files | 1,821 MB | **IGNORE** — Playwright dumps |
| `.pi/visual-audit/` | 33 files | 29 MB | **IGNORE** — session screenshots |
| `.pi/investigation/` | 258 files | 105 MB | Mixed: .md = keep, .png = ignore |
| `.pi/goals/` | 30 files | <1 MB | **KEEP** — pi tool history |
| `.pi/tasks/` | 0 files | 0 | N/A |

## Why other games don't accumulate

All 8 WIP games share the same `.gitignore` template. None exclude
`.pi/chrome-screenshots/`. But only hegemon actively runs
`.pi/cap.mjs` for visual iteration — the other games don't
accumulate this bloat.

| Game | .pi/chrome-screenshots tracked | Pack |
|------|-------------------------------:|-----:|
| **hegemon** | **1,108 / 1,821 MB** | **4,096 MB** |
| deathrun | 290 / 147 MB | 224 MB |
| darklord | 156 / 125 MB | 75 MB |
| endless-td | 8 / 2 MB | 224 MB |
| polis | 2 / <1 MB | 64 MB |

Hegemon is 8-12x bigger than the next biggest peer. The fix stops
the bleed for all current and future games.

## Files

- `docs/design/hegemon-autoignore-2026-07-05.md` — investigation
- `docs/design/hegemon-size-investigation-2026-07-05.md` — earlier investigation
