# Warden nested-discovery gap → .pi screenshot leak (2026-07-31)

## Symptom

`dracon-sync repos` SIZE column: hellhunter at 1.60 GiB and growing.
Investigation: **1.32 GiB of hellhunter's history is `.pi/` — almost
entirely `chrome-screenshots/` audit frame dumps** (2818 objects),
with commits landing **as recently as the same day**. darklord
(185 MiB) and neonbreak (244 MiB) had the same active leak.

This is exactly the bloat class the 2026-07-23 fleet-wide hygiene
rule (`**/.pi/chrome-screenshots/` in warden's `hygiene_patterns`,
see `audit-screenshot-bloat-deathrun-2026-07-23.md`) was supposed to
prevent.

## Root cause (two stacked gaps)

1. **Warden discovery is one level deep** (`discover_git_repos` in
   `dracon-warden/src/main.rs:134`): it reads the direct children of
   each `repo_roots` entry and checks for `.git`. The nested-on-main
   game repos live at
   `Dev/dracon-platform/web/games/<wip|released>/<name>/` — five
   levels down, never discovered. Their warden-managed `.gitignore`
   blocks dated from the **standalone-worktree era** (pre-2026-07-02)
   and lacked every hygiene pattern added since, including the
   2026-07-23 screenshot rule.

2. **Warden's own plaintext negations defeat file-level hygiene
   patterns**: the managed block emits `!*.png` (and friends) AFTER
   the hygiene patterns, so any *file* pattern matching PNGs is
   re-included. Only *directory* exclusions survive (`git` cannot
   re-include files under an excluded directory — this is why
   endless-td's local `.pi/` rule held, and why the
   `**/.pi/chrome-screenshots/` dir pattern works once present).

## Fix (applied 2026-07-31)

1. `dracon-warden once <path>` for all 9 nested game repos —
   refreshed the stale managed blocks (hooks, filter config,
   gitattributes, .gitignore all updated).
2. **Config**: added the two game tiers as warden `repo_roots` in
   `~/.dracon/utilities/warden/dracon-warden.toml`:
   - `/home/dracon/Dev/dracon-platform/web/games/wip`
   - `/home/dracon/Dev/dracon-platform/web/games/released`

   Zero code change; future warden passes now discover and re-harden
   every nested game repo automatically. Verified: hellhunter,
   darklord, neonbreak all report `.pi/chrome-screenshots/*` IGNORED.

## Deferred (operator decision required)

The **historical** bloat stays in the branches until a filter-repo
cleanup (rewrites published history → bundle-first, `--force-with-
lease`, coordinate with loops):

| repo | .pi history | own .git |
|---|---|---|
| hellhunter | ~1.32 GiB | 1.60 GiB (pack 1.35 GiB — github's 2 GiB limit is the wall) |
| neonbreak | ~244 MiB | 770 MiB |
| darklord | ~185 MiB | 836 MiB |
| junk-runner | (active.jsonl, excluded 2026-07-28) | 1.15 GiB |

With the leaks stopped, growth normalizes, so there is no urgency —
but hellhunter is the repo to watch; a cleanup there reclaims the
most headroom against github's pack limit.

## Follow-up worth considering

- hellhunter also carries 224 MiB of `SPEC_V19.md` + 152 MiB of
  `SPEC.md` revision churn in history (loop rewrites them
  constantly). Not leak-class — real content — but the same
  filter-repo pass could prune old revisions if desired.
- Warden discovery staying one-level-deep is fine **as long as new
  nested tiers get added to `repo_roots`** when created. A recursive
  discovery mode (descend past non-repo dirs, stop at `.git`
  boundaries, keep descending *beside* found repos for nested
  submodules) would make this self-maintaining — candidate warden
  enhancement, not done here.
