# dracon-sync v0.112.27

**Released:** 2026-07-20
**Type:** Minor feature (operator UX — two-tier `repos` command)
**Severity:** Low — operator-requested change to make `repos` glance-friendly.

## Summary

The `dracon-sync repos` command had grown to 16 columns (ROLE, BRANCH, PUBLISH, M/S/U counts, AHEAD, BEHIND, PUSH, PUSH-TO, LAST COMMIT, STATE+ACT, HINT). For the common "is anything broken?" check, this is too noisy. The operator now needs two views:

1. **Glance view** (`repos --summary` / `-s`): 3-column table — STATUS, REPO, WHAT. Rendered as a proper `comfy-table` with UTF8_FULL_CONDENSED borders. WHAT = `activity + dirty-counts + hint` joined by ` · `.
2. **Detailed view** (default `repos`): unchanged. 16-column Compact/Full table for deep inspection.

For the most common health-check pattern, combine them: `repos -s --only-concern` (only the broken ones, glance view).

**R1 (2026-07-20)** — Operator feedback: "the summary needs to be a table." R0 used `println!` with manual spacing which broke alignment under ANSI color codes. R1 uses `comfy-table` with `UTF8_FULL_CONDENSED` preset, fixed-width `#` / `STATUS` / `REPO` columns (`Absolute` widths), and a `Dynamic` WHAT column that absorbs leftover terminal width.

**R2 (2026-07-20)** — Operator feedback: "the authors are wrong, we're freestyling some of it." The summary's `by {author}` suffix was `git log -1 --format=%an` — the git commit author of the most recent commit. For a solo operator who freestyles git identities across repos (`DraconDev` / `dracon` / `darklord-dev`), this reads as "different people" when it's all the same operator, which is misleading noise in a glance view. R2 drops the `by {author}` suffix from the summary WHAT entirely. The detailed 16-column table keeps the author (it has a dedicated column and is part of the full record); the summary trades it for width + clarity. WHAT is now `activity + dirty-counts + push-status-if-stuck + hint`.

## What `--summary` shows

```
┌────┬────────────┬────────────────────────┬────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ #  ┆ STATUS     ┆ REPO                   ┆ WHAT                                                                                                       │
╞════╪════════════╪════════════════════════╪════════════════════════════════════════════════════════════════════════════════════════════════════════════╡
│ 1  ┆ 🔄 ACTIVE  ┆ polis                  ┆ ⏳ dirty ? · 2 mod + 1 ut · daemon handles after changes settle; run sync-now --warns to force now │
│ 2  ┆ 🔄 ACTIVE  ┆ endless-td             ┆ ⏳ dirty 0m · 1 mod · daemon handles after changes settle; run sync-now --warns to force now    │
│ 3  ┆ 🔄 ACTIVE  ┆ junk-runner            ┆ ⏳ dirty 2m · 1 mod + 3 stg · daemon handles after changes settle; run sync-now --warns to force now │
│ 4  ┆ 🔄 ACTIVE  ┆ deathrun               ┆ ⏳ dirty 3m · 1 mod · .git exceeds 2 GB (github limit) — may fail to push to github                  │
│ 5  ┆ ✅ CLEAN   ┆ nexus-new-tab          ┆ ⚪ idle 14h · healthy                                                                          │
│ 6  ┆ ✅ CLEAN   ┆ one-mil-girls          ┆ ⚫ cold 12d · healthy                                                                          │
└────┴────────────┴────────────────────────┴───────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

Each row tells the operator:
- **#** — row number in the summary (severity-sorted if `--summary-by-severity` is set)
- **STATUS** — is this repo broken? (`❌ CONCERN` / `⚠️ WARN` / `🔄 ACTIVE` / `✅ CLEAN`)
- **REPO** — which one
- **WHAT** — what state is it in (`⏳ dirty 0m`), what kind of dirty work (`3 mod + 3 stg`), what should I do (`run repair-concerns --apply`)

## Flags

| Flag | Short | Effect |
|---|---|---|
| `--summary` | `-s` | Switch to 3-column glance view (STATUS · REPO · WHAT) |
| `--summary-by-severity` | | Sort by severity: concerns first, clean last. Default sort is `updated` (same as the detailed view). |
| `--only-concern` | | Filter to only `❌ CONCERN` rows (works with both views) |
| `--only-warn` | | Filter to only `⚠️ WARN` rows |

The `repos --help` text shows the new flags with full descriptions.

## Why two views instead of one

The detailed table's 16 columns are useful when you already know what you're investigating ("what's the publish state of hegemon?"). The glance view is for when you don't — you just want to scan the system state and notice anomalies.

Trying to merge both into a single "smart" view that adapts column count based on detected attention was rejected because:
- A column hidden on one row but visible on another breaks visual scan patterns (the eye expects alignment)
- The detailed view's value comes from having ALL info available; stripping columns defeats the purpose
- Two clearly-named commands (`repos` vs `repos --summary`) with predictable output are easier to script against

## New helpers

- `severity_tier(row)` — returns 0 (concern) / 1 (warn) / 2 (active) / 3 (clean) for severity-sort.
- `summary_what(row, budget)` — builds the WHAT string: `activity + dirty-counts + push-status (if stuck) + hint`, joined by ` · `, truncated to budget. (Author intentionally omitted — see R2.)
- `print_repos_summary(...)` — renders the 3-column table using `comfy-table` with the UTF8_FULL_CONDENSED preset. The `#` / `STATUS` / `REPO` columns use `Absolute(N)` widths; the WHAT column is `Dynamic` and absorbs leftover terminal width.

The default sort is `updated` (matching the detailed view's sort). With `--summary-by-severity`, the sort key is `(severity_tier, original_idx)` — within a tier, the original `updated` order is preserved.

## Bug found during development (fixed before release)

R0 had a duplicate `1 ahead` segment when push was pending — the activity already encoded `🟣 pushing 0m (1 ahead)`, but the summary also added a separate `1 ahead`. Test `test_summary_what_pending_push_drops_redundant_ahead_note` enforces the fix: ahead count appears exactly once in the WHAT.

## New regression tests (7 total)

| Test | What it verifies |
|---|---|
| `test_summary_what_clean_idle_repo` | Clean repo summary shows activity + hint, but NOT push status, dirty counts, or author |
| `test_summary_what_dirty_repo_includes_dirty_counts_and_hint` | Dirty repo summary shows `2 mod + 1 ut` + hint + activity, but NOT `by {author}` (omitted in R2) |
| `test_summary_what_pending_push_drops_redundant_ahead_note` | Push PENDING: ahead count appears exactly once (from activity, not duplicated) |
| `test_summary_what_stuck_push_shows_status` | Push STUCK: surfaces as `push: stuck` even though activity doesn't show it |
| `test_severity_tier_ordering` | Tier 0 = concern, 1 = warn, 2 = active, 3 = clean |
| `test_print_repos_summary_renders_as_table` | Smoke test: `print_repos_summary()` runs without panicking on a populated row |
| `test_summary_what_handles_long_hint_with_word_boundary` | Long hint + narrow budget: WHAT ends with `…` or natural sentence end (no mid-word clip) |

Test count: **935** daemon tests passing (was 928 at v0.112.26, +7 from new tests).

## Test discipline

| Check | Result |
|---|---|
| `cargo build --release --locked` | ✅ green |
| `cargo test --workspace --locked` | ✅ **935 passed, 0 failed, 3 ignored** |
| `cargo clippy --workspace --locked -- -D warnings` | ✅ clean |
| `cargo deny check` | ✅ clean |

## Live daemon

- v0.112.27 deployed to `/home/dracon/.local/bin/dracon-sync`
- Tested: `repos -s`, `repos -s --only-concern`, `repos -s --only-warn`, `repos -s --summary-by-severity`

## Verification chain

```bash
cd /home/dracon/Dev/dracon-utilities
cargo build --release --locked                       # ✅ green
cargo test --release --workspace --locked            # ✅ 933 passed
cargo clippy --workspace --locked -- -D warnings     # ✅ clean
cargo deny check                                     # ✅ clean
~/.local/bin/dracon-sync repos --summary             # ✅ glance view
~/.local/bin/dracon-sync repos --summary --only-concern  # ✅ filters work
~/.local/bin/dracon-sync repos --summary --summary-by-severity  # ✅ severity sort
```