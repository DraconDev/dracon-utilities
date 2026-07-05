# Hegemon `.backup-*` cruft comparison — 2026-07-05

## Cleanup performed

Removed **43 tracked `.backup-*` dirs** (1,531 files) from `hegemon`'s
`static/assets/`, totaling **~350 MB** of pre-v1.0 cruft. Files
preserved in git history (no `git filter-repo` rewrite needed).

| Metric | Before | After | Δ |
|---|---:|---:|---:|
| Tracked file count | 3,741 | 2,210 | −1,531 |
| Working tree `static/` | 829 MB | 479 MB | −350 MB |
| Loose `.backup-*` files on disk | 1,531 | 0 | −1,531 |
| Pack file size on disk | 4.18 GB | 4.18 GB | unchanged |

**Why pack is unchanged**: git packs blobs once with delta compression.
The deleted blobs are now `prune-packable` per `git count-objects -v`,
but physical pack files don't shrink until `git gc` repacks. We did not
repack — that's a separate operation and was not requested.

The daemon auto-committed the cleanup into commit `d19874d9` ("h11
tasks 3-4: PIL RGB→RGBA...") within seconds of staging. This is
expected daemon behavior per AGENTS.md §commit-policy.

## Why is hegemon ~8x bigger than the other repos?

Per-repo measurement of all 26 repos in the daemon's watch set
(2026-07-05 20:30):

| REPO | WT_KB | PACK_KB | TRACKED | KB/TRACK | Outlier? |
|------|------:|--------:|--------:|---------:|----------|
| **dracon-platform** | 119,032,932 | **13,232,729** | 3,919 | **3,376** | 🔥 meta-repo (30 submods) |
| **hegemon** | 2,440,736 | **4,176,501** | 2,210 | **1,889** | 🔥 binary-heavy |
| browser-extensions-shared | 1,351,828 | 512,329 | 3,039 | 168 | — |
| deathrun | 420,132 | 229,011 | 829 | 276 | — |
| endless-td | 1,430,340 | 228,358 | 676 | 337 | — |
| one-mil-girls | 161,448 | 218,409 | 174 | 1,255 | ⚠️ |
| capture-anime-girls | 685,080 | 216,886 | 1,075 | 201 | — |
| junk-runner | 189,320 | 209,558 | 525 | 399 | — |
| neonbreak | 588,612 | 205,964 | 644 | 319 | — |
| dragon-code | 27,492,164 | 184,209 | 439 | 419 | — |
| ai-auto-writer | 6,842,316 | 111,959 | 3,667 | 30 | — |
| .dracon | 3,207,396 | 79,032 | 204 | 387 | — |
| hellhunter | 13,188 | 77,779 | 127 | 612 | — |
| darklord | 1,913,424 | 76,714 | 864 | 88 | — |
| polis | 542,184 | 64,597 | 930 | 69 | — |
| dracon-utilities | 7,085,620 | 37,692 | 289 | 130 | — |
| (8 more repos < 10 MB pack) | — | — | — | — | — |

(`DraconDev` MISSING — repo path is no longer present; daemon is
still listing it but there's no live checkout.)

**Per-game static-dir comparison** (other "HEAVY" games for context):

| Game | static/ size | static % of WT | Backup dirs in static/ |
|------|-------------:|---------------:|-----------------------:|
| **hegemon** | 489 MB | 20% | **0** (just cleaned!) |
| darklord | 585 MB | 30% | 0 |
| endless-td | 472 MB | 33% | 0 |
| capture-anime-girls | 219 MB | 31% | 0 |
| deathrun | 83 MB | 19% | 0 |
| neonbreak | 120 MB | 20% | 0 |
| polis | 26 MB | 4% | 0 |
| one-mil-girls | 53 MB | 32% | 0 |
| hellhunter | 0.8 MB | 6% | 0 |
| junk-runner | 4.7 MB | 2% | 0 |

## Verdict: why is hegemon an outlier?

**Hegemon is the only game with a `gen-*.py` pipeline that regenerates
PNG assets via the mmx AI platform.** Every regen run writes to
`static/assets/<category>/` AND creates a `<category>.backup-r<N>` dir.
Over the r3 → r9 → r10 → r11 cycle, this accumulated ~9 backup variants
per asset category (terrain, mines, dwellings, towns, creatures, etc.).

The 4.18 GB git-pack size breaks down as:

- **~480 MB** = current `static/assets/` (regen output)
- **~350 MB** = pre-v1.0 `.backup-*` cruft (just removed!)
- **~3.4 GB** = prior committed .backup-* content from r3 → r11 cycles (across 4 pack files)

The 3.4 GB is bigger than the cleanup because the daemon has been
committing backup dirs for 12+ regen rounds. Even at the previous
drop rate of 350 MB / cycle, that's 12 × 350 MB = 4.2 GB of cumulated
backup-* history in pack.

Other games don't have this pattern because their assets are
hand-made and never regenerated:

- **darklord** (585 MB static) — large but stable, no gen pipeline
- **one-mil-girls** (53 MB static, but 218 MB pack / 1255 KB/track) —
  released/archived; pack is binary-heavy because it's a frozen
  release with all assets committed
- **polis** / **junk-runner** / **hellhunter** — small static, small
  pack, all source-code-driven (no AI-generated PNGs)

## Operator takeaway

The `.backup-*` cruft was structurally bad hygiene, not a unique
hegemon failure. It accumulated because:

1. **No cleanup-hygiene in the gen pipeline**: scripts write to
   current-assets-dir + a sibling backup dir, but never delete either.
2. **Pre-v1.0 stored everything**: r3 backups were committed during
   the r3 regen when `.gitignore` for backup-* wasn't set.
3. **Regen cadence > cleanup cadence**: at 3943 commits/hour during
   peak regen, the cleanup can't keep up.

Going forward, the durable fix is **.gitignore `<category>.backup-*`**
so backups stay out of git entirely. Per-regex `auto_commit_exclude_patterns`
in `.dracon/dracon-sync.toml` is the per-repo mechanism to do this
without touching the daemon's global policy.

## Files

This doc: `docs/design/hegemon-backup-cruft-comparison-2026-07-05.md`

Related: `docs/design/full-audit-2026-07-05.md`,
`docs/design/hegemon-state-investigation-2026-07-05.md`

Per-repo size: `git ls-tree -rl HEAD | awk '{sum+=$3} END {print sum}'`
(player runs once per repo; ~20 sec for all 26)