# Codeberg Quota Leak Fix — 2026-07-13

## What (from audit on 2026-07-13)

Codeberg account is at **85.0000 GiB used / 85.00 GiB grace quota (99.5%)**.
Daemon has been failing push with `remote: Forgejo: Quota exceeded` on
every repo.

The 85 GiB is split across 86 repos (47 private + 39 public). Top 10 private
repos account for 73.8 GiB (87%).

## Two-track problem

After auditing the 17 heaviest repos' git history AND live untracked dir
state, the 85 GiB codeberg total decomposes into:

| Bucket                                | Size     | Where seen                       |
|---------------------------------------|----------|----------------------------------|
| **Intentional game art** (PNGs/MP3/FBX) | 21 GiB   | `static/assets/`, `screenshots/`, `assets/` in 17 game repos |
| **Dracon-platform git-tracked `.pi/` + test-results + verify-screenshots + audit-binary** | 10.29 GiB | historical commits in 17 repos |
| **Live untracked collection dirs in working trees** | 3.5 MiB  | `.pi/`, `.state-recon/`, `chrome-screenshots/`, etc. |
| **Stray home-dir leak** (`~/`)        | 58 MiB   | browser-extensions-shared (untracked, was about to push) |

Two distinct problems:

1. **Forward leak**: any new untracked `.pi/`, `test-results/`, etc.
   the daemon auto-commits to codeberg. Real risk: 2025-2026 the daemon
   repeatedly committed the same kind of evidence and it piled up.
2. **Historical pile**: 21 GiB of intentional art (committed via
   `git add`) plus 10 GiB of accumulated session evidence. This dominates
   the 85 GiB.

## What we shipped in this design doc's release (v0.112.15)

**Forward-only fix.** Add 9 DIR-level patterns to
`default_untracked_exclude_patterns` to stop future accumulation, plus
a new `scan-bloat` subcommand so future novel directory names surface
to the operator instead of silently accumulating.

The 21 GiB historical intentional art is **not touched** — it's the
user's cargo, not a leak. The 10 GiB historical session evidence is
also **not yet cleaned** in this release; cleaning it requires
`git filter-repo --invert-paths` + force-push across 17 repos, which
is documented below as a deferred next step.

## The 9 DIR-level patterns (verified empirically)

After auditing the actual untracked dirs in 17 watched repos, here are
the unambiguous collection directory names that consistently recurred
and produced no false positives on intentional content:

| Pattern                  | What it catches (verified live)                            | Historical size |
|--------------------------|------------------------------------------------------------|----------------:|
| `**/.pi/**`              | `.pi/`, `.pi-tmp/`, `.pi-goals/`, `.pi-tasks/`, `.pi/mmx-out/` | 4.36 GiB |
| `**/test-results/**`     | Playwright dumps (named with git SHA)                      | 2.40 GiB |
| `**/verify-screenshots/**` | verification harness output                              | 0.76 GiB |
| `**/__screenshots__/**`  | Python e2e framework convention                           | small |
| `**/.state-recon/**`     | agent probe dirs                                          | small |
| `**/chrome-screenshots/**` | chrome agent output                                    | 1.56 GiB |
| `**/chrome-*/**`         | chrome-fixes, chrome-consistency, etc.                    | small |
| `**/sign-in-flash-audit/**` | one-off verification dir                              | small |
| `**/~/**`                | home-dir leak (e.g. browser-extensions-shared `~/` artifact)| 58 MiB |

**Why DIR-level only (not extension-level):** the operator explicitly
stated "i dont wnat to filte out all pngs that is too intensse [sic],
we cna jsut filter out folders that is a colleection of screnshots".
Filtering `.png` would catch the 21 GiB of intentional game art.
Filtering only `*.png` inside audit dirs (multi-glob) was rejected
because the daemon's matcher can't express multi-`**` globs.

**Verified preservation:** `test_default_untracked_exclude_patterns_preserves_intentional_content`
in `dracon-sync/src/policy.rs` asserts each of these DOES NOT match:

```
web/screenshots/one-mil-girls-screenshots/01-title.png  (1mg marketing)
docs/audit-event-2026-07-06.md                          (audit REPORT)
scripts/audit-uiux-2026-06-26.mjs                      (audit SCRIPT)
static/assets/texture.png                               (intentional game art)
assets/audio/theme.mp3                                  (intentional asset)
src/lib.rs                                               (source)
```

## The `scan-bloat` discovery loop

New CLI:

```
dracon-sync scan-bloat [--min-size-mib <N>] [--min-repo-count <N>] [--json]
```

Walks every watched repo via `git ls-files --others --exclude-standard
--directory`. For each untracked directory:

1. Apply the existing `untracked_exclude_patterns` matcher. If matched,
   skip (already covered by static list).
2. Skip `node_modules/`, `target/`, `dist/`, `build/` paths.
3. Aggregate by leaf name across repos. e.g. `test-results/` recurring
   in 7 repos becomes one bucket row.
4. Filter by thresholds (default: ≥ 5 MiB total, ≥ 2 repos). Singletons
   and tiny dirs are noise.
5. Emit sorted-by-size report with suggested glob per bucket
   (`**/<leaf>/**`).

Live output on the 28 watched repos (default thresholds):

```
🔎 Scanned 28 repo(s) for untracked bloat (thresholds: ≥ 5 MiB total, ≥ 2 repos).

DIRECTORY                            SIZE   REPOS    FILES  SUGGESTED EXCLUDE
-----------------------------------------------------------------------------------------------
web                             13.24 MiB       2       22  **/web/**
-----------------------------------------------------------------------------------------------
(TOTAL)                         13.24 MiB

💡 Each row suggests a pattern like `**/<dir>/**` that you can add
   to `untracked_exclude_patterns` in `~/.dracon/utilities/sync/dracon-sync.toml`
   (global) or per-repo at `<repo>/.dracon/dracon-sync.toml`.
```

With relaxed thresholds (`--min-size-mib 1 --min-repo-count 1`), 7
buckets surface:

| Leaf | Size | Verdict |
|---|---:|---|
| `dracon-sync` | 5.36 GiB | build artifacts from per-crate compile; should be in parent `.gitignore` |
| `assets` | 176.50 MiB | intentional game art; do **not** exclude |
| `test-books` | 72.33 MiB | ai-auto-writer content drafts; do **not** exclude |
| `~` | 55.85 MiB | home-dir leak; now caught by `**/~/**` |
| `artifacts` | 34.82 MiB | `~/.dracon` system CI artifacts; system scope |
| `.state-recon` | 29.83 MiB | agent probe; now caught by `**/.state-recon/**` |
| `web` | 13.24 MiB | submodule working tree; intentional |

**Auto-discovery captures future tools.** When a new agent or harness
drops a new directory name (`~verify-logs-2026-08/`,
`__harness_out__/`, `screenshots-2026-q4/`), running
`scan-bloat` surfaces it for operator review. The operator decides:
add to `untracked_exclude_patterns` (catch-all), add to
`.gitignore` (commit-aware), or leave intentional.

## Deferred: historical cleanup (10 GiB of `.pi/` + test-results)

**Status: NOT EXECUTED in v0.112.15.** This section is the design
plan that would clean the historical 10 GiB out of git history.

For each of the 17 heaviest git-tracked repos, run:

```bash
cd "$repo"
TS=$(date +%s)
git branch backup/pre-quota-leak-cleanup-$TS main  # safety fuse
git filter-repo --invert-paths \
  --path-glob '*.pi/**' \
  --path-glob '*test-results/**' \
  --path-glob '*verify-screenshots/**' \
  --path-glob '*__screenshots__/**' \
  --path-glob '*.state-recon/**' \
  --path-glob '*chrome-screenshots/**' \
  --path-glob '*chrome-*/**' \
  --path-glob '*sign-in-flash-audit/**' \
  --path-glob '*~/**' \
  --force
git reflog expire --expire=now --all
git gc --prune=now --aggressive
git push --force-with-lease=codeberg/main:main codeberg main
git push --force-with-lease=github/main:main github main
git push --force-with-lease=gitlab/main:main gitlab main
```

**Safety:** `backup/pre-quota-leak-cleanup-<ts>` is local-only. If
the rewrite goes wrong, the rollback is
`git reset --hard backup/pre-quota-leak-cleanup-<ts>`.

**Per-phase override:** `auto_repair_concerns = false` in each repo's
`.dracon/dracon-sync.toml` during the loop, to prevent the daemon
from running its own filter-repo during our manual pass.

**Why deferred:** the per-repo force-push loop touches codeberg/github/
gitlab with `git filter-repo` rewrites, which is the largest blast
radius in this whole design. Combined with the 21 GiB of intentional
art needing separate scope review (the operator has stated "not yet"
for that), this part is a separate go-decision. The forward-only fix
in v0.112.15 is the safer first step.

## Backward compatibility

- All 12 baseline patterns from 2026-06-15 are preserved unchanged.
- Per-repo `auto_commit_exclude_patterns` still works.
- World policy override `untracked_exclude_patterns = []` still works
  (and currently IS empty per
  `~/.dracon/utilities/sync/dracon-sync.toml` 2026-06-17 — the new 9
  patterns inherit only into repos that DON'T override the field).
- `tests/e2e/__screenshots__` was an existing per-repo entry in
  `Junk-Runner-bevy/.dracon/dracon-sync.toml`; now covered by the
  global default `**/__screenshots__/**`. Per-repo override can be
  removed when convenient.

## Test/Build/Deny bar

- `cargo build --release --locked` — clean (0 warnings)
- `cargo build --tests --locked` — clean (0 warnings)
- `cargo test --workspace --locked` — 848 passed, 3 ignored, 0 failed
- `cargo deny check` — workspace + 3 per-crate, all 4 OK

## Live verification pre/post

**Pre-v0.112.15** (codeberg API): 90,851,072,256 bytes used
(85.0000 GiB / 85.00 GiB grace quota, 99.5%).

**Post-v0.112.15**: unchanged. The historical 85 GiB is not addressed
by this release. Forward prevention only.

**Post-deferred-cleanup (when executed)**: expect a 5-15 GiB drop
in codeberg quota usage after the 17-repo filter-repo loop completes
and codeberg stat refreshes (5-10 min). Verify with:

```bash
source ~/.dracon/secrets/pat/codeberg.env
curl -s -H "Authorization: token $CODEBERG_TOKEN" "https://codeberg.org/api/v1/user" \
  | jq '.quota_used, .quota_limit'
```

## See also

- `release-notes-v0.112.15.md` — release notes for this change.
- `CHANGELOG.md` `[Unreleased]` — canonical changelog entry.
- `AUDIT_REPOS_2026-07-10.md` — pre-existing codeberg size audit.
- `docs/archive/audits-2026-07/AUDIT-3-UTILITIES-RERUN-2026-07-11.md` — fresh utility audit that
  confirmed `default_untracked_exclude_patterns` was the right
  intervention point.
- `commit-all-policy-2026-06-15.md` — the operator's
  "commit-all unless super-good reason" principle that this fix
  honors (we add PATTERNS to the super-good-reason list; we do NOT
  change the operator's commit-all default).
</content>
</invoke>