# Pre-Migration State — dracon-platform — 2026-06-28

## OVH Bucket `dracon-master` (audited via live API call, not doc claim)

| Prefix | Objects | Size |
|---|---|---|
| music/audio/ | 601 | 3,405 MB |
| music/covers/ | 61 | 18 MB |
| music/ (root) | 1 | ~0 |
| **music total** | **663** | **3,424 MB** |
| games/hegemon/assets/ | 2440 | 2,105 MB |
| games/junk-runner/ | 155 | 311 MB |
| games/one-mil-girls/ | 101 | 54 MB |
| games/polis/ | 217 | 20 MB |
| **games total** | **2,914** | **2,489 MB** |
| dracon/ (db/wal segments) | 61059 | 9 MB |
| Various test prefixes | 313 | 13 MB |
| **TOTAL BUCKET** | **65,052** | **5.84 GB** |

Note: prior audit doc claimed "1.3 GB in bucket" — actual measured is **5.84 GB**. Doc was substantially wrong.

## Tracked binary content in git (276 audio + 5273 image files = 5,549 binary files)

| Type | Tracked files | Tracked size |
|---|---|---|
| Audio (mp3, wav, ogg, flac) | 276 | 648 MB |
| Images (png, jpg, jpeg, gif, webp) | 5,273 | 2,689 MB |
| **Total tracked binary** | **5,549** | **3,337 MB (~3.3 GB)** |

## Bucketing compliance violations

`bun run web/scripts/check-bucketing-compliance.mjs --max-size 1` (current default) reports **1,119 grandfathered MIGRATION_TODO entries** plus a handful of new violations in chrome-screenshots/ and docs/.visual-refs/ directories.

The MIGRATION_TODO list (in compliance script) covers:
- polis (~15 MB), one-mil-girls (~52 MB), deathrun (~64 MB), neonbreak (~100 MB)
- junk-runner (~189 MB audio), capture-anime-girls (~214 MB), endless-td (~299 MB), hegemon (~2.2 GB)
- darklord, hellhunter (marker entries, 0 MB verified clean)
- Layer-3 visual artifacts (screenshots)
- web/screenshots/ (PR-review screenshots)

## PUSH_STUCK current state

- ahead: ~3,038 (and growing every minute as daemon auto-commits)
- behind: 1
- codeberg tip: 6a7cf69324074e35cff9e64f4aa3ef15d6c3b4e5 (unchanged since 2026-06-26)
- 86 push failures logged
