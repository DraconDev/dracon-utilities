# dracon-sync v0.112.23

**Released:** 2026-07-19
**Type:** Patch (UI rendering fix)
**Severity:** High — `repos` table layout was broken at any terminal width

## Summary

v0.112.23 fixes the `repos` table layout that was causing cells to
wrap over multiple lines in BOTH compact and full tiers. This was a
class-of-bugs in the column-constraint strategy (using
`LowerBoundary` instead of `Absolute` for cells with variable-length
content) combined with no in-cell truncation.

## Root cause

`comfy-table`'s `LowerBoundary(Width::Fixed(N))` is misleading: it
does NOT cap the column at N. It says "column must be AT LEAST N,
but may grow to fit content". For cells like `LAST COMMIT` whose
content is unbounded, the column grew to fit the longest subject
(152 chars for our auto-commit messages) and broke the table.

The full-tier constraint was 23 columns with a 300-col minimum width,
which fits in any >=300-col terminal — but if even one column grows,
comfy-table's `ContentArrangement::Dynamic` allows OTHER columns to
shrink to compensate. Net result: cell wrapping in narrow columns
and column growing in wide columns, both destroying readability.

## Fix

1. **Switch LAST COMMIT, AUTHOR, STATUS, PUSH-TO** from `LowerBoundary`
   to `Absolute` so the columns are truly fixed at the listed width.
2. **Truncate cell content** with `truncate_unicode_width()` before
   passing to comfy-table. Cell width = column constraint minus
   comfy-table's 2-col cell padding.
3. **Truncate HINT, ACTIVITY, STATE+ACT, STATE, DAEMON, AUTHOR**
   cells with `truncate_unicode_width` to match their column
   constraints, even though they're `LowerBoundary` (to avoid
   the "one long cell eats the whole table" failure mode).
4. **Bump full-tier threshold** from 300 to 315 cols (sum 287 + 24
   borders = 311 fits in 315).
5. **Bump STATUS column from 11 to 13 cols** so `🚫 unowned`
   (11 cols) fits with padding.

## Truncation budgets

| Column | Width (Full) | Truncate budget |
|---|---:|---:|
| STATUS | 13 | (truncated internally) |
| LAST COMMIT | 17 | 15 |
| PUSH-TO | 32 | 30 |
| HINT | 15 | 13 |
| ACTIVITY | 11 | 9 |
| AUTHOR | 11 | 9 |
| STATE | 15 | 13 |
| DAEMON | 15 | 13 |

## New regression test

`test_long_commit_subject_truncated_to_last_commit_width` —
constructs a 152-char commit subject (the realistic worst case) and
verifies `truncate_unicode_width()` produces a single-line result that
fits in the LAST COMMIT column width. Without truncation, the cell
would be 152 chars wide and break the table.

## Test discipline

| Check | Result |
|---|---|
| `cargo build --release --locked` | ✅ green |
| `cargo test --workspace --locked` | ✅ **916 passed, 0 failed, 3 ignored** (was 915 at v0.112.22, +1 new test) |
| `cargo clippy --workspace --locked -- -D warnings` | ✅ clean |
| `cargo deny check` | ✅ clean |

## Live daemon

- v0.112.23 deployed to `/home/dracon/.local/bin/dracon-sync`
- Live tally: `📦 31 repos · ✅ CLEAN 27 · 🔄 ACTIVE 4 · ⚠️ WARN 0 · ❌ CONCERN 0`
- All 30 data rows now render on single lines (was 5-line wrapping
  in some rows for `LAST COMMIT`, `HINT`, `STATE+ACT`)

## Verification chain

```bash
cd /home/dracon/Dev/dracon-utilities
cargo build --release --locked              # ✅ green
cargo test --release --workspace --locked   # ✅ 916 passed
cargo clippy --workspace --locked -- -D warnings  # ✅ clean
cargo deny check                            # ✅ clean
COLUMNS=400 ~/.local/bin/dracon-sync repos  # ✅ 30 rows, 1 line each
```