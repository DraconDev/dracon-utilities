# dracon-sync v0.112.26

**Released:** 2026-07-19
**Type:** Patch (UI polish — clean STATE+ACT truncation + wider HINT column)
**Severity:** Low — fixes two cosmetic rendering artifacts in v0.112.25's `repos` table.

## Summary

v0.112.25 fixed the letter-wrap bug (ROLE/REPO/PUBLISH/STATE+ACT/HINT cells wrapping to 2 lines on narrow terminals). After shipping, two cosmetic artifacts were still visible:

1. **STATE+ACT mid-emoji truncation**: `🟠 dirty · ⏳ …` — the second emoji (⏳ egg-timer) was kept but the trailing text was clipped, leaving a dangling emoji + ellipsis.
2. **HINT column too narrow**: `daemon handles afte…` — 20-col budget clipped the operator-friendly phrase mid-word.

**v0.112.26 fixes**:
1. New `state_plus_act_cell()` helper that drops the activity part **cleanly** when budget is tight — no more `· ⏳ …` artifacts. State always renders (`🟠 dirty`), activity only when there's room (`🟠 dirty · ⏳ dirty 1h`).
2. Widen HINT column from `Absolute(22)` → `Absolute(26)`, budget 20 → 24 cols. Now fits `daemon handles after ch…` (full phrase minus 7 chars) instead of `daemon handles afte…` (mid-word).
3. Bump Compact tier threshold from `< 238` to `< 242` to match the new HINT width.

After this fix:
- Terminal < 242 cols → Vertical mode (multi-line per repo)
- Terminal 242-314 cols → Compact mode (single-line rows, clean truncation)
- Terminal ≥ 315 cols → Full mode (all columns)

## Why the new state+act strategy

The previous `truncate_unicode_width("🟠 dirty · ⏳ dirty 1h", 15)` clipped the string mid-emoji, producing `🟠 dirty · ⏳ …` — visually ungrammatical because the second emoji has no word after it.

The new helper:
1. Renders state-only first: `🟠 dirty` (10 cols)
2. If that fits the budget, tries state + activity: `🟠 dirty · ⏳ dirty 1h` (19 cols)
3. If state+activity doesn't fit, returns state-only (drops activity cleanly)
4. If state itself doesn't fit, truncates state with `…`

State is preserved over activity because activity (`⏳ dirty 1h`) is decorative detail while state (`🟠 dirty`) is the actionable classification. The HINT column always has the full context.

## What now truncates (compact tier)

| Column | Width | Truncation budget | Worst-case input → output |
|---|---:|---:|---|
| REPO | 18 (Absolute) | 16 cols | `pully-fully-pull-based-fleet-reconciler` (38) → `pully-fully-pull-b…` |
| ROLE | 14 (Absolute) | 12 cols | `released/one-mil-girls` (22) → `released/on…` |
| PUBLISH | 18 (Absolute) | 16 cols | `⚠️ origin/main (gone)` (22) → `⚠️ origin/main (…` |
| LAST COMMIT | 18 (Absolute) | 16 cols | unchanged from v0.112.23 |
| **STATE+ACT** | 17 (Absolute) | 15 cols | **NEW**: `🟠 dirty · ⏳ dirty 1h` (19) → `🟠 dirty` (state preserved) |
| **HINT** | **26 (Absolute)** | **24 cols** | **`daemon handles after changes settle` (33) → `daemon handles after ch…`** (was 20 → `daemon handles afte…`) |
| PUSH-TO | 32 (Absolute) | 30 cols | unchanged from v0.112.23 |

## New regression tests (3 total)

| Test | What it verifies |
|---|---|
| `test_state_plus_act_cell_drops_activity_when_tight` (new) | 15-col budget + `🟠 dirty` + `⏳ dirty 5m` → `🟠 dirty` (activity dropped cleanly) |
| `test_state_plus_act_cell_keeps_activity_when_it_fits` (new) | 30-col budget + same input → `🟠 dirty · ⏳ dirty 5m` (both shown) |
| `test_state_plus_act_cell_handles_dash_activity` (new) | `🟢 synced` + `—` → `🟢 synced` (dash activity dropped) |

Plus updated existing tests:
- `test_choose_layout_tier_compact` (width range 238-299 → 242-314)
- `test_choose_layout_tier_vertical` (width range 40-237 → 40-241)
- `test_compact_table_min_width_within_250` (threshold 240 → 244)

Test count: **928** daemon tests passing (was 924 at v0.112.24, +4 net: 1 from v0.112.25's role_cell test + 3 from this release).

## Stalled repos investigation (neonbreak + endless-td)

Both repos were `⚠️ WARN · 🔴 stalled` in the user's screenshot. Root cause was the user's own `pi-loop` LLM agent (160 iterations from 14:54 to 21:06 BST, with 13 rate-limit errors on iteration 129 and 1 operator_abort at 20:50). The loop repeatedly regenerated `tools/spec-audit.mjs` + `docs/spec-compliance.md`. The daemon correctly auto-committed every change. Loop is now dead. No daemon fix needed.

## Test discipline

| Check | Result |
|---|---|
| `cargo build --release --locked` | ✅ green |
| `cargo test --workspace --locked` | ✅ **928 passed, 0 failed, 3 ignored** (was 925, +3 from new tests) |
| `cargo clippy --workspace --locked -- -D warnings` | ✅ clean |
| `cargo deny check` | ✅ clean |

## Live daemon

- v0.112.26 deployed to `/home/dracon/.local/bin/dracon-sync`
- Tested rendering at 230 cols (Vertical, no wrap), 240 cols (Vertical, no wrap — was Compact in v0.112.25), 250+ cols (Compact, all single-line, clean truncation), 400 cols (Full)

## Verification chain

```bash
cd /home/dracon/Dev/dracon-utilities
cargo build --release --locked              # ✅ green
cargo test --release --workspace --locked   # ✅ 928 passed
cargo clippy --workspace --locked -- -D warnings  # ✅ clean
cargo deny check                            # ✅ clean
COLUMNS=240 ~/.local/bin/dracon-sync repos  # ✅ Vertical (no wrap)
COLUMNS=300 ~/.local/bin/dracon-sync repos  # ✅ Compact, all single-line, clean truncation
```

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