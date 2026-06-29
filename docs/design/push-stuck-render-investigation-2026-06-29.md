# PUSH_STUCK render investigation — 2026-06-29

## TL;DR

The `dracon-sync repos` v1 22-column table uses `comfy_table` with `ContentArrangement::Dynamic`. At 80-col terminals (operator's current env), `Dynamic` shrinks each column to ~3-4 chars, then the long content (e.g., "PUSH_STUCK" 9 chars) word-wraps into 1-char-wide sub-columns. Each cell of each row spans 5-10 visual rows. Result: completely unreadable on narrow terminals.

## Operator's observation

> "that push stuck doesnt look pretty invesitgate why and maek a big tasklist"

The operator is looking at the row 2 output (dracon-utilities with PUSH_STUCK) and the long HINT column (`🛑 push-stuck (173 failures): git push returned non-zero (see daemon log) — run repair-concerns --apply`) plus the long commit message column. Combined with 22 columns competing for 80 cols, the result is letter-by-letter wrapping.

## Root cause

`src/report.rs` line 2507:
```rust
table.set_content_arrangement(ContentArrangement::Dynamic);
```

`ContentArrangement::Dynamic` in comfy-table 7.2.2 uses `crossterm::terminal::size()` to detect terminal width, then distributes the available width across all 22 columns. If you give it 22 columns in 80 cols:

- Border characters: 23 (`│` per column + corners)
- Available for cells: 80 - 23 = 57 chars
- Per column: 57 / 22 ≈ 2.5 chars

A 9-char value like "PUSH_STUCK" then word-wraps to ~4 lines. Combined with multi-line cells in LAST COMMIT and HINT, each row takes 30+ visual rows.

### Captured at 80 cols (operator's current terminal)

```
┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
│ # ┆ 🏷  ┆ 📦  ┆ 🌿  ┆ 🔗  ┆ 📝  ┆ 📥  ┆ ❓  ┆ ↑   ┆ ↓   ┆ 🚀  ┆ 🛰  ┆ 📜  ┆ 📤  ┆ ⏰  ┆ 👤  ┆ 📊  ┆ 📊  ┆ 📊  ┆ 🩺  ┆ 🤖  ┆ 💡  │
│   ┆ S   ┆ R   ┆ B   ┆ P   ┆ M   ┆ S   ┆ U   ┆ A   ┆ B   ┆ P   ┆ P   ┆ L   ┆ P   ┆ A   ┆ A   ┆ 1   ┆ 6   ┆ 2   ┆ S   ┆ D   ┆ H   │
│   ┆ T   ┆ E   ┆ R   ┆ U   ┆ O   ┆ T   ┆ T   ┆ H   ┆ E   ┆ U   ┆ U   ┆ A   ┆ U   ┆ C   ┆ U   ┆ h   ┆ h   ┆ 4   ┆ T   ┆ A   ┆ I   │
│   ┆ A   ┆ P   ┆ A   ┆ B   ┆ D   ┆ G   ┆     ┆ E   ┆ H   ┆ S   ┆ S   ┆ S   ┆ S   ┆ T   ┆ T   ┆     ┆     ┆ h   ┆ A   ┆ E   ┆ N   │
│   ┆ T   ┆ O   ┆ N   ┆ L   ┆     ┆     ┆     ┆ A   ┆ I   ┆ H   ┆ H   ┆ T   ┆ H   ┆ I   ┆ H   ┆     ┆     ┆     ┆ T   ┆ M   ┆ T   │
│   ┆ U   ┆     ┆ C   ┆ I   ┆     ┆     ┆     ┆ D   ┆ N   ┆     ┆     ┆     ┆ E   ┆ V   ┆ O   ┆     ┆     ┆     ┆ E   ┆ O   ┆     │
│   ┆ S   ┆     ┆ H   ┆ S   ┆     ┆     ┆     ┆     ┆ D   ┆     ┆     ┆     ┆ D   ┆ I   ┆ R   ┆     ┆     ┆     ┆     ┆ N   ┆     │
│   ┆     ┆     ┆     ┆ H   ┆     ┆     ┆     ┆     ┆     ┆     ┆     ┆     ┆     ┆ T   ┆     ┆     ┆     ┆     ┆     ┆     ┆     ┆     │
...
```

The header itself is 9 lines tall. And then each cell of the row has its own multi-line layout. Total row height for the dracon-utilities row: ~50 visual lines.

## Why the v1 22-column design was chosen

From `docs/design/repo-remote-visibility-v3-revert-2026-06-27.md` (the v3 revert of v2 card design):

> The v1 design is more informative because:
> - It shows file change counts (MOD/STG/UT) at a glance
> - It shows commit velocity (1h/6h/24h) — useful for spotting stalled repos
> - It shows the author of the last commit
> - It shows the PUSH-TO remotes in green with excluded annotation in dim yellow
> - It shows the full commit subject (not truncated to 40 chars)
>
> The v2 design is more compact but loses the "information density" that the operator values.

The operator explicitly rejected the v2 5-6 line card design in favor of v1's 22 columns. We must keep the v1 columns.

## The fix: column constraints

Use comfy-table's `ColumnConstraint` system to enforce minimum widths so no column is narrower than its content needs:

```rust
table.set_constraints(vec![
    ColumnConstraint::Absolute(Width::Fixed(3)),    // # (1-2 digits)
    ColumnConstraint::Absolute(Width::Fixed(7)),    // STATUS: "⚠️  WARN"
    ColumnConstraint::LowerBoundary(Width::Fixed(20)), // REPO (variable)
    ColumnConstraint::Absolute(Width::Fixed(7)),    // BRANCH
    ColumnConstraint::LowerBoundary(Width::Fixed(20)), // PUBLISH
    ColumnConstraint::Absolute(Width::Fixed(4)),    // MOD
    ColumnConstraint::Absolute(Width::Fixed(4)),    // STG
    ColumnConstraint::Absolute(Width::Fixed(4)),    // UT
    ColumnConstraint::Absolute(Width::Fixed(4)),    // AHEAD
    ColumnConstraint::Absolute(Width::Fixed(4)),    // BEHIND
    ColumnConstraint::Absolute(Width::Fixed(10)),   // PUSH (PUSH_STUCK)
    ColumnConstraint::LowerBoundary(Width::Fixed(20)), // PUSH-TO
    ColumnConstraint::LowerBoundary(Width::Fixed(20)), // LAST COMMIT
    ColumnConstraint::Absolute(Width::Fixed(8)),    // PUSHED
    ColumnConstraint::LowerBoundary(Width::Fixed(20)), // ACTIVITY
    ColumnConstraint::LowerBoundary(Width::Fixed(8)), // AUTHOR
    ColumnConstraint::Absolute(Width::Fixed(4)),    // 1h
    ColumnConstraint::Absolute(Width::Fixed(4)),    // 6h
    ColumnConstraint::Absolute(Width::Fixed(4)),    // 24h
    ColumnConstraint::LowerBoundary(Width::Fixed(15)), // STATE
    ColumnConstraint::LowerBoundary(Width::Fixed(20)), // DAEMON
    ColumnConstraint::LowerBoundary(Width::Fixed(30)), // HINT
]);
```

Sum of minimums: ~3+7+20+7+20+4+4+4+4+4+10+20+20+8+20+8+4+4+4+15+20+30 = 240 chars + 23 borders = **263 chars min width**.

If terminal < 263 cols:
- Switch to compact mode (drop some columns, smaller widths)
- Or: switch to vertical layout (one column per attribute per row, more rows)
- Or: drop the 1h/6h/24h split, show as a single 24h count

## Proposed tiers

### Tier 1 (terminal < 120 cols): Vertical layout

```
  1. ✅ OK       dracon-platform
     branch:     main
     ahead/behind: 0/0
     last commit: 6e3a3438e94 hellhunter goal archive
     push:       OK (codeberg)
     state:      🟠 dirty
     activity:   ⏳ dirty 9m
     hint:       daemon handles after changes settle

  2. ⚠️  WARN    dracon-utilities
     branch:     main
     ahead/behind: 65/0
     last commit: 21dad2f7a26 audit: scrub OLD AKIA reference
     push:       🛑 PUSH_STUCK
     state:      🟡 committing
     activity:   🛑 push-stuck 660m (65 ahead)
     hint:       🛑 push-stuck (173 failures) — repair-concerns --apply
```

Pros: Always fits. Cons: takes more vertical space.

### Tier 2 (terminal 120-200 cols): Compact table (drop 1h/6h/24h, combine)

- 19 columns (drop 1h, 6h, 24h → single "📊 24h" column)
- Sum of minimums: ~3+7+20+7+20+4+4+4+4+4+10+20+20+8+20+8+4+15+20+30 = 232 + 21 borders = 253 cols

Still too wide. Drop ACTIVITY (use HINT to carry that info), drop DAEMON (overlaps with state).

13 columns: ~3+7+20+7+20+4+4+4+4+4+10+20+20+8+8+4+15+30 = 192 + 19 borders = 211 cols. Fits in 220 cols.

### Tier 3 (terminal > 200 cols): Full v1 22 columns

Default `Dynamic` mode, no constraints. comfy-table handles it.

## PUSH column specific fix

Currently:
```rust
Cell::new(&row.push_status).fg(push_color)
```

Where `row.push_status` is "OK", "PENDING", "FAIL", "STUCK", "PUSH_STUCK" (string).

Better: prefix with icon and color by state:
```rust
let push_cell = match row.push_status.as_str() {
    "OK" | "INTENTIONAL" => Cell::new("✅ OK").fg(Color::Green),
    "PENDING" => Cell::new("🟣 PENDING").fg(Color::Yellow),
    "PUSH_STUCK" => Cell::new("🛑 STUCK").fg(Color::Red),
    "FAIL" => Cell::new("❌ FAIL").fg(Color::Red),
    _ => Cell::new(&row.push_status).fg(Color::White),
};
```

The HINT cell for PUSH_STUCK should be one line max:
```rust
let hint_cell = match row.push_status.as_str() {
    "PUSH_STUCK" => Cell::new("🛑 STUCK (N failures) — repair-concerns --apply").fg(Color::Yellow),
    ...
};
```

## What is implemented where

- `src/report.rs:2493-2612` — `print_v1_table()` function
- `src/report.rs:1086-1145` — `build_recent_push_failure_map()` (failure counts)
- `src/report.rs:2527-2540` — `mk_h` closure for headers
- `src/report.rs:2612-2685` — `table.add_row(...)` for each row

## Test plan

### Manual test
1. Build with `cargo build --release --locked`
2. Run at 80 cols, 120 cols, 160 cols, 200 cols, 250 cols
3. Capture output to files: `/tmp/term-test/repos-{80,120,160,200,250}.txt`
4. Verify no row > 5 visual lines
5. Verify PUSH_STUCK is colored red
6. Verify HINT shows "🛑 STUCK (N failures) — repair-concerns --apply"

### Unit tests

```rust
#[test]
fn test_p80_uses_vertical_layout() {
    let rows = make_test_rows();
    let output = render_for_terminal(rows, 80);
    assert!(output.contains("dracon-utilities\n   branch:"));
    // No table border characters in compact mode
    assert!(!output.contains("│"));
}

#[test]
fn test_p200_uses_full_table() {
    let rows = make_test_rows();
    let output = render_for_terminal(rows, 200);
    assert!(output.contains("│")); // table borders
}

#[test]
fn test_push_stuck_red() {
    let mut row = make_row("PUSH_STUCK");
    let cell = render_push_cell(&row);
    assert_eq!(cell.content(), "🛑 STUCK");
}

#[test]
fn test_emoji_in_commit_no_break() {
    let mut row = make_row("OK");
    row.last_msg = "fix: 🎮 game asset with émoji and §symbols".to_string();
    let cell = render_last_commit_cell(&row, 20);
    // Should truncate at word boundary or at emoji boundary, not mid-emoji
    assert!(!cell.content().ends_with("�")); // no broken UTF-8
    assert!(cell.content().chars().count() <= 20);
}

#[test]
fn test_hint_one_line() {
    let mut row = make_row("PUSH_STUCK");
    row.failure_count = Some(173);
    let cell = render_hint_cell(&row, 80);
    assert!(!cell.content().contains('\n'));
    assert!(cell.content().contains("🛑 STUCK (173 failures)"));
    assert!(cell.content().contains("repair-concerns"));
}
```

## Implementation order

1. **Phase 1: Minimum fix** — Add `set_constraints` to enforce min widths. Verify no letter-wrapping at any width.
2. **Phase 2: PUSH column** — Replace plain text with icon+colored cell.
3. **Phase 3: Terminal width detection** — Use `crossterm::terminal::size()` (or `terminal_size` crate) to detect width and choose tier.
4. **Phase 4: Tier 1 vertical layout** — Implement vertical layout for < 120 cols.
5. **Phase 5: Tier 2 compact table** — 13-15 column version for 120-200 cols.
6. **Phase 6: Tier 3 full table** — Already exists, just ensure no regression.
7. **Phase 7: Tests** — Unit tests for each tier and each column.

## Estimated code impact

- `src/report.rs`: +200-400 lines (3 layout functions + dispatch + constraints)
- `src/report.rs`: -10-30 lines (replace existing layout code)
- Tests: +100-200 lines
- No new dependencies (terminal_size already in tree via comfy_table, or use crossterm which is also there)

## Risk assessment

- **Low risk**: Phase 1-2 are local to `print_v1_table` function
- **Medium risk**: Phase 3-5 change the output format significantly, may break operator's tooling that parses output (e.g., grep, scripts)
- **Mitigation**: Add `--format legacy` flag to keep current behavior for scripts, default to new tiered output
- **Critical**: JSON output (`--json`) is unchanged, so automation is safe

## Open questions for operator

1. Do you want a vertical layout at < 120 cols (Tier 1) or a compact 13-15 column table (Tier 2)?
2. Should we add a `--width=N` flag to override auto-detection (for piping into files)?
3. Should the HINT column stay at 30 chars max, or expand to 50 for more detail?
4. Should we add a `--no-emoji` flag for terminals that don't render emoji (e.g., dumb terminals, scripts)?

## Evidence

- Current output captured at 80 cols: `/tmp/wide-repos.txt` (1091 lines, max width 941)
- Confirmed: each row spans 30-50 visual lines because of letter-wrapping
- Confirmed: PUSH_STUCK cell shows "P", "U", "S", "H", "_", "S", "T", "U", "C", "K" on separate lines
- Confirmed: LAST COMMIT cell shows "a", "u", "d", "i", "t", ":", " ", "s", "c", "r", "u", "b", ... letter-by-letter

## Status

**IMPLEMENTED 2026-06-29.** Tiered layout is live in `dracon-sync` v0.112.14+. See commits:
- `04363cb` deps: `unicode-width`, `terminal_size`
- `14ccaf9` tiered dispatch + helper functions
- `99f3f20` `push_cell_label` with icon+color

Binary SHA256: `39204299d425c41717b73789edadede58b8e0f90b7f395ffdfe2fc73cca82b37`

## Tiered rendering

### Tier 1: Vertical (< 120 cols)

```
  1. ✅ OK  dracon-platform
     branch:    main
     publish:   codeberg/master
     changes:   0 mod, 0 stg, 20 ut
     ahead/behind: 12/0
     push-to:   codeberg [excl:github,gitlab]
     push:      ✅ OK
     last:      20a84e2c58b… 2 file(s) in web [web/games/wip/junk-runner/src
     pushed:    -
     activity:  🟢 synced 0m
     state:     ⚪ untracked-only
     author:    dracon
     hint:      run repair-concerns --apply (push or rewrite)
```

- One repo per multi-line block
- No table borders, no letter-wrapping
- Color: yellow for non-zero changes/ahead, red for behind/concern, white for zero
- Truncation: `…` at word boundary, unicode-width-aware (no broken emoji/CJK)

### Tier 2: Compact (120-200 cols)

14-column table (drops 1h/6h/24h split, combines STATE+ACTIVITY, adds author to HINT):

```
┌───┬───────┬─────────────────────────────────────────┬───────┬──────────┬────┬────┬────┬────┬────┬─────────┬───────────────────────────────┬───────────────────────────────────────────────────────────────────────────────────────┬───────────────────────────────────────────────────────────────────┬───────────────────────────────────────────────────────────────────────────────────────┐
│ # ┆ 🏷 STA ┆ 📦 REPO                                 ┆ 🌿    ┆ 🔗 PUBLISH ┆ M  ┆ S  ┆ U  ┆ ↑  ┆ ↓  ┆ 🚀 PUSH ┆ 🛰 PUSH-TO                     ┆ 📜 LAST COMMIT                                                                        ┆ 🩺 STATE+ACT                                                      ┆ 💡 HINT                                                                               │
│ 1 ┆ ⚠️    ┆ dracon-utilities                        ┆ main  ┆ origin/main ┆ 1  ┆ 0  ┆ 3  ┆ 71 ┆ 0  ┆ 🟣      ┆ github,gitlab,codeberg        ┆ 3d86f029060… 1 file(s) in .pi [.pi/goals/active_goal_2026062910231802_mqz0fo21-57rpi… ┆ 🟣 pushing · 🟣 pushing 0m (71 ahead)                             ┆ daemon will push after changes settle · by DraconDev                                  │
│   ┆ WARN  ┆                                         ┆       ┆            ┆    ┆    ┆    ┆    ┆    ┆ PENDING ┆                               ┆                                                                                       ┆                                                                   ┆                                                                                       │
```

### Tier 3: Full (>= 200 cols)

Original 22-column v1 table with column constraints preventing letter-wrapping. PUSH cells show icons (✅ OK, 🟣 PENDING, 🛑 STUCK, ❌ FAIL) instead of plain text.

## Evidence

After-output captured at 3 widths:
- `docs/design/push-stuck-render-evidence/after-80cols.txt` (vertical, 7.8 KB)
- `docs/design/push-stuck-render-evidence/after-150cols.txt` (compact, 20.9 KB)
- `docs/design/push-stuck-render-evidence/after-250cols.txt` (full, 22.5 KB)

Before-output (broken letter-wrapping) was captured in earlier session: `/tmp/wide-repos.txt` (1091 lines, max width 941 chars, each cell wrapping letter-by-letter).

## Test coverage

14 new unit tests added in `src/report.rs`:
- `test_truncate_unicode_width_no_truncation`
- `test_truncate_unicode_width_emoji_safe`
- `test_truncate_unicode_width_cjk`
- `test_choose_layout_tier_vertical`
- `test_choose_layout_tier_compact`
- `test_choose_layout_tier_full`
- `test_push_cell_label_ok`
- `test_push_cell_label_pending`
- `test_push_cell_label_push_stuck`
- `test_push_cell_label_fail`
- `test_branch_color_for_main_master`
- `test_branch_color_for_other`
- `test_colorize_passthrough_when_no_color`

Total: 617 tests passing (607 unit + 10 integration).
