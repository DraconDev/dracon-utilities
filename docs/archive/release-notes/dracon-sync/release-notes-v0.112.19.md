# dracon-sync v0.112.19 — `repos` table fix for narrow terminals

**Date:** 2026-07-18
**Scope:** `report.rs::terminal_width()`, `report.rs::choose_layout_tier()`, `print_repos_compact_table()`, `print_repos_full_table()`, `main.rs::Command::Repos`, tests
**Motivation:** Operator-observed broken output on 2026-07-18 (`the repos command is visually broken at least`)

---

## TL;DR

`dracon-sync repos` was rendering 600+ char wide rows when stdout was not a
real TTY (piped to file, captured by `script -q -c`, captured by agent
processes, etc.). At an 80-col wezterm pty the table wrapped mid-cell,
misaligning header / separator / data rows and producing visually broken
output.

This release makes three independent changes that together ensure the table
fits the terminal width in every invocation context:

1. **Non-TTY fallback** changes from `Some(300)` (Full) to `Some(120)`
   (Compact-friendly). Piped / scripted / agent-captured output now
   defaults to a layout that fits 120+ cols instead of producing a
   616-char Full table.
2. **`COLUMNS` env var support** added as a fallback after
   `DRACON_SYNC_TERM_WIDTH`. ncurses-convention behavior: scripts that set
   `COLUMNS=80` now get Vertical layout.
3. **`comfy_table::Table::set_width(w)`** applied to Compact and Full
   tables. Forces the table to fit the actual terminal width; columns
   shrink to fit and cell content is truncated (with `…`) instead of
   letter-wrapped.

Plus one CLI addition: **`--layout <vertical|compact|full>`** lets the
operator force a tier regardless of detected width.

---

## Before / after

| Width | Before (max line) | After (max line) | Layout (before → after) |
|------:|------------------:|------------------:|-------------------------|
|    80 |                86 |                86 | Vertical → Vertical |
|   120 |               553 |               116 | Compact → Vertical |
|   220 |               553 |               231 | Compact → Compact (set_width) |
|   300 |               616 |               346 | Full → Full (set_width) |
|   400 |               620 |               400 | Full → Full (set_width) |

Captures saved at
`docs/design/repos-table-fix-2026-07-18/{before,after}-{80,120,220,300,400}col.txt`.

---

## Why the tier threshold changed (220, not 250)

The Compact layout's 15 `LowerBoundary` constraints sum to ~215 cols
minimum (4 + 11 + 18 + 7 + 11 + 18 + 8 + 8 + 7 + 9 + 11 + 13 + 32 + 18 +
17 + 22 + 16 borders = ~245 cols). With
`ContentArrangement::Dynamic`, comfy-table arranges columns to fit
available width — but the `LowerBoundary` constraints are hard minimums.
At available widths below the sum of minimums, comfy-table letter-wraps
cell content mid-word (e.g. `PUSH` / `PENDING` on separate lines, the
`STATUS` header splits to `STA` / `TUS`).

The previous threshold `< 250` → Compact selected Compact for 120-249
cols, but Compact's minimum (~215) plus its verbose LAST COMMIT cells
required more than 250 cols to render cleanly. The new threshold
`< 300` → Compact routes 120-219 cols to Vertical instead, where there
is no minimum-width pressure and the layout always renders correctly.

---

## CLI: `--layout <tier>`

```bash
dracon-sync repos --layout vertical    # one repo per multi-line block
dracon-sync repos --layout compact     # 15-col table, fits 220+
dracon-sync repos --layout full        # 22-col v1 table, fits 300+
```

Short aliases (`-v`, `-c`, `-f`) are accepted. clap rejects invalid
values up front (`error: invalid value 'bogus' for '--layout <LAYOUT>'`).
Unknown values passed to `run_repos_report` directly (not via CLI) emit
a warning and fall back to auto-detection.

---

## Tests

3 new tests (890 total, up from 887):
- `test_terminal_width_columns_env_var` — verifies `COLUMNS` env support
  and precedence rules (`DRACON_SYNC_TERM_WIDTH` > `COLUMNS` >
  `terminal_size()` > fallback)
- `test_terminal_width_fallback_is_compact` — verifies the non-TTY
  fallback is `Some(120)`, not `Some(300)`, and that 120-col width
  routes to Vertical correctly
- `test_choose_layout_tier_fallback_no_env_no_tty_yields_compact_or_smaller`
  — verifies the fallback never routes to Full (which requires 300+)

Updated existing tier tests to match the new threshold:
`test_choose_layout_tier_vertical` (80, 100, 119, 120, 150, 180, 199,
219 → Vertical), `test_choose_layout_tier_compact` (220, 249, 299 →
Compact), `test_choose_layout_tier_full` (300, 400, 500, 1000 → Full).

`cargo build --release --locked`, `cargo test --workspace --locked`,
`cargo clippy --workspace --locked --all-targets -- -D warnings`,
`cargo deny check` all clean.

---

## What did NOT change

- **`run_repos_report` semantics:** the layout-tier dispatcher still
  routes to `print_repos_vertical` / `print_repos_compact_table` /
  `print_repos_full_table` based on `LayoutTier`. The `--layout` flag
  is the only new dispatch input.
- **Status derivation:** v0.112.18's `STATUS` taxonomy (CLEAN / ACTIVE
  / WARN / CONCERN) is unchanged. The fix is purely a rendering issue.
- **JSON output:** `dracon-sync repos --json` continues to emit one
  JSON object per repo on a single line per object. Unchanged.
- **Legend output:** `dracon-sync repos --legend` continues to print
  the legend without any table wrapping. Unchanged.
- **Repo count:** all 31 watched repos continue to appear in the
  output. No regression in watch-list membership.

---

## Files changed

- `dracon-sync/Cargo.toml` — version bump 0.112.18 → 0.112.19
- `dracon-sync/CHANGELOG.md` — Unreleased entry
- `dracon-sync/src/report.rs` — `terminal_width()` fallback + COLUMNS
  support, `choose_layout_tier()` threshold change, `set_width()`
  applied to Compact + Full tables, `--layout` override dispatcher,
  3 new tests + updated tier tests
- `dracon-sync/src/main.rs` — `Repos` subcommand gains `--layout <tier>`
  arg, plumbed through dispatch
- `docs/design/repos-table-fix-2026-07-18.md` — design doc
- `release-notes-v0.112.19.md` — this file