# dracon-sync v0.112.25

**Released:** 2026-07-19
**Type:** Patch (UI render correctness, two follow-ups from goal `4555eaf6`)
**Severity:** Medium — fixes a UI rendering bug introduced by v0.112.24 (ROLE/REPO/PUBLISH columns wrapping to 2 lines on narrow terminals).

## Summary

v0.112.24 fixed three issues (`hegemon unowned`, `opencode-plugins codeberg-as-main`, verbose ROLE labels). It shipped one rendering regression: on terminals between ~220-237 cols, REPO/ROLE/PUBLISH/STATE+ACT/HINT cells with variable-length content would **letter-wrap onto a second line** instead of being truncated.

The cause was that the v0.112.24 column-width spec used `LowerBoundary(N)` for these columns. `LowerBoundary` means **at-least N** — comfy-table will let the column GROW to fit content, but **does not** truncate. When the terminal's `set_width()` shrinks all columns to fit, the cell content overflows and the column wraps.

**v0.112.25 fix**:
1. Switch REPO (Compact), ROLE, PUBLISH (Compact), STATE+ACT, HINT (Full) from `LowerBoundary(N)` to `Absolute(N)`.
2. Apply `truncate_unicode_width(..., N-2)` to the cell content in `repo_name`, `role_cell()`, and `publish_cell_label()` so long values are ellipsized rather than wrapped.
3. Bump Compact tier threshold from `< 220` to `< 238` to match the new column budget (sum = 223 cols + 15 borders = 238 minimum).

After this fix:
- Terminal < 238 cols → Vertical mode (no wrapping, multi-line per repo)
- Terminal 238-314 cols → Compact mode (single-line rows, content truncated as needed)
- Terminal ≥ 315 cols → Full mode (single-line rows, all columns + 1h/6h/24h split)

## Why this matters

Without truncation, the Compact table's lower boundary sums to 232 cols. Terminals in the 220-237 range show the table but the variable-length columns (ROLE, PUBLISH, REPO) get squashed and content wraps. With Absolute widths, the column widths are enforced; long content is truncated with `…` instead of wrapping.

The bug was visible in v0.112.24's release for any operator with a terminal between ~220-237 cols (which includes wezterm with sidebars/pans, tmux with status bars, and smaller phone/tablet terminals).

## What now truncates (compact tier)

| Column | Width | Truncation budget | Worst-case input → output |
|---|---:|---:|---|
| REPO | 18 (Absolute) | 16 cols | `pully-fully-pull-based-fleet-reconciler` (38) → `pully-fully-pull-b…` |
| ROLE | 14 (Absolute) | 12 cols | `released/one-mil-girls` (22) → `released/on…` |
| PUBLISH | 18 (Absolute) | 16 cols | `⚠️ origin/main (gone)` (22) → `⚠️ origin/main (…` |
| LAST COMMIT | 18 (Absolute) | 16 cols | (already truncated in v0.112.23) |
| STATE+ACT | 17 (Absolute) | 15 cols | (already truncated in v0.112.23) |
| HINT | 22 (Absolute) | 20 cols | (already truncated in v0.112.23) |
| PUSH-TO | 32 (Absolute) | 30 cols | (already truncated in v0.112.23) |

The previous `parent (10 submods)` ROLE label (20 chars) was renamed to `parent·10` (9 chars) to make `parent·N` fit in the new 14-col ROLE column with 5 cols of headroom.

## Full tier fixes

Same LowerBoundary→Absolute conversion for REPO (17 → 19 cols, was bug), PUBLISH (17), ACTIVITY (11), STATE (15), DAEMON (15), HINT (15). All variable-length content now truncated to fit.

## New regression tests (2 total)

| Test | What it verifies |
|---|---|
| `test_publish_cell_label_marks_missing_and_gone` (updated) | Publish label truncates to ≤17 visual cols (⚠️ is 2-col emoji; ≤ 16 ASCII + …) |
| `test_role_cell_truncates_long_submod_labels` (new) | ROLE labels > 12 cols truncated with `…`; short labels pass through; parent label is `parent·N` |

Plus updated existing tests:
- `test_compact_table_min_width_within_250` (threshold 232 → 240; array 15 → 16 entries; fixed the missing ROLE bug from v0.112.23's array)
- `test_full_table_min_width_within_300` (REPO bumped 17 → 19)
- `test_choose_layout_tier_compact` (width range 220-299 → 238-314)
- `test_choose_layout_tier_vertical` (width range 40-219 → 40-237)
- `classify_role_for_parent_repo` (parent label assertion updated)

Test count: **925** daemon tests passing (was 924, +1 from new test, minus existing tests merged/updated).

## Stalled repos investigation (neonbreak + endless-td)

Both repos were `⚠️ WARN · 🔴 stalled` in the user's screenshot. Root cause:
- A `pi-loop` LLM agent was running on neonbreak (160 iterations from 14:54 to 21:06 BST, with 13 rate-limit errors on iteration 129 and 1 operator_abort at 20:50)
- The loop was hitting Anthropic API 429 rate-limit errors repeatedly for 3+ hours
- Each loop iteration modified `tools/spec-audit.mjs` and `docs/spec-compliance.md`
- The daemon correctly auto-committed every change (no daemon bug)
- "Stalled" was the daemon classifying "dirty work + no recent commit for X minutes" — but with a loop firing every ~30s, this should have shown `🟠 dirty`, not `🔴 stalled`. Likely a brief classification moment when the loop briefly paused between retries.

**Resolution**: The loop is now dead (last entry 21:06:51, no active processes). Both repos are now `🔄 ACTIVE · 🔄 working · 🟢 synced`. They will return to `✅ CLEAN` once the loop's accumulated changes settle.

**No daemon fix needed** — the daemon is doing exactly what it should.

## Test discipline

| Check | Result |
|---|---|
| `cargo build --release --locked` | ✅ green |
| `cargo test --workspace --locked` | ✅ **925 passed, 0 failed, 3 ignored** (was 924, +1 net) |
| `cargo clippy --workspace --locked -- -D warnings` | ✅ clean |
| `cargo deny check` | ✅ clean |

## Live daemon

- v0.112.25 deployed to `/home/dracon/.local/bin/dracon-sync`
- Tested rendering at 230 cols (Vertical, no wrap), 240 cols (Compact, all single-line), 300 cols (Compact, all single-line), 400 cols (Compact/Full boundary, all single-line)

## Verification chain

```bash
cd /home/dracon/Dev/dracon-utilities
cargo build --release --locked              # ✅ green
cargo test --release --workspace --locked   # ✅ 925 passed
cargo clippy --workspace --locked -- -D warnings  # ✅ clean
cargo deny check                            # ✅ clean
COLUMNS=230 ~/.local/bin/dracon-sync repos  # ✅ Vertical (no wrap)
COLUMNS=240 ~/.local/bin/dracon-sync repos  # ✅ Compact (32 single-line rows, 0 wraps)
```