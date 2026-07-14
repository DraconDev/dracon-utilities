# Release notes — `dracon-sync` v0.112.15 — 2026-07-13

Goal: `mrhvbn1s-codeberg-quota-leak-fix`

## What changed

### Daemon: 9 new untracked-exclude patterns

`default_untracked_exclude_patterns()` in `dracon-sync/src/policy.rs` now
includes 9 additional DIR-level patterns for unambiguous collection
directories identified by the codeberg quota audit. These are
**NAME-based**, not extension-based, so they do not collide with
intentional shipping art (PNGs/MP3s in `static/assets/`,
`assets/`, `screenshots/<game>/`):

```
**/.pi/**                  # universal agent dir (.pi/, .pi-tmp/, .pi-goals/, .pi-tasks/, .pi/mmx-out/)
**/test-results/**         # Playwright outputs (named with git SHA)
**/verify-screenshots/**   # verification harness output
**/__screenshots__/**      # Python e2e framework convention
**/.state-recon/**         # agent probe dirs
**/chrome-screenshots/**   # chrome agent output
**/chrome-*/**             # chrome-fixes, chrome-consistency, etc.
**/sign-in-flash-audit/**  # one-off verification dir
**/~/**                    # home-dir leak (caught in browser-extensions-shared)
```

Empirical verification: walked all 17 watched repos. No false positives on:

- `web/screenshots/one-mil-girls-screenshots/*.png` (1mg marketing)
- `docs/audit-*.md` and `scripts/audit-*.mjs` (audit REPORTS + SCRIPTS)
- `static/assets/*.png` and `assets/*.mp3` (intentional game art)

Test `test_default_untracked_exclude_patterns_preserves_intentional_content`
asserts every preservation rule.

### New CLI command: `dracon-sync scan-bloat`

```
dracon-sync scan-bloat [--min-size-mib <N>] [--min-repo-count <N>] [--json]
```

Walks every watched repo, finds untracked collection directories NOT yet
covered by `untracked_exclude_patterns`, aggregates them by leaf name
across repos, and emits a sorted-by-size report with suggested globs.

This is the **operator's manual review loop** for forward compatibility.
When a future tool drops a new directory name (e.g. `~verify-2026-08/`)
into working trees, it surfaces here instead of silently accumulating.

Live output on the 28 watched repos (default thresholds):

```
🔎 Scanned 28 repo(s) for untracked bloat (thresholds: ≥ 5 MiB total, ≥ 2 repos).

DIRECTORY                            SIZE   REPOS    FILES  SUGGESTED EXCLUDE
-----------------------------------------------------------------------------------------------
web                             13.24 MiB       2       22  **/web/**
-----------------------------------------------------------------------------------------------
(TOTAL)                         13.24 MiB
```

Lower thresholds (`--min-size-mib 1 --min-repo-count 1`) surface 6 more
buckets: `dracon-sync/` (5.36 GiB build artifacts), `assets/` (intentional
game art, do NOT exclude), `test-books/` (content drafts),
`~/` (home-dir leak, now caught), `artifacts/` (system CI), `web/` (mixed).

## Backward compatibility

- All 12 baseline patterns from 2026-06-15 are preserved.
- Per-repo `auto_commit_exclude_patterns` still works.
- World policy override `untracked_exclude_patterns = []` still works.
- New patterns only apply to repos that do NOT explicitly set
  `untracked_exclude_patterns` (i.e., they inherit the daemon default).
  Repos that override the field are unaffected.

## Scope

This change is **forward-only**. It prevents new accumulation of the
named patterns from leaking to codeberg. The 85 GiB historical codeberg
content is NOT cleaned by this change. Cleaning the historical content
requires a separate `git filter-repo --invert-paths` history cleanup +
force-push loop, which is documented in
`docs/design/codeberg-quota-leak-fix-2026-07-13.md` as a deferred next
step (the design doc lays out the filter-repo + 3-remotes force-push
sequence, but it is NOT being executed by this release).

## Test/Build/Deny bar (AGENTS.md compliance)

- `cargo build --release --locked` — clean (0 warnings)
- `cargo build --tests --locked` — clean (0 warnings)
- `cargo test --workspace --locked` — 848 passed, 3 ignored, 0 failed
- `cargo deny check` — workspace + 3 per-crate, all 4 OK

## See also

- `docs/design/codeberg-quota-leak-fix-2026-07-13.md` — design doc
  with audit evidence, pattern justifications, preservation rules, and
  the deferred filter-repo plan.
- `CHANGELOG.md` `[Unreleased]` — global changelog entry.
- `AUDIT_REPOS_2026-07-10.md` — pre-existing codeberg size audit
  showing 85 GiB total / 85 GiB grace quota.
</content>
</invoke>