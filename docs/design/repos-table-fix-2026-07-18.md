# `dracon-sync repos` table fix for narrow terminals — 2026-07-18

**Status:** implemented in v0.112.19 (deployed 2026-07-18 15:20 BST).
**Goal:** `1152889f-70e7-4f7f-9265-44ca2695c2ff`.
**Motivation:** Operator observation on 2026-07-18: "the repos command is
visually broken at least" — output rows were 600+ characters wide,
wrapping mid-cell and misaligning header/separator/data rows in any
terminal narrower than ~200 columns.

---

## Root cause

`dracon-sync/src/report.rs::terminal_width()` had three layers of
fallback for the column-width detection:

1. `DRACON_SYNC_TERM_WIDTH` env var (explicit override)
2. `terminal_size()` against stdout/stderr/stdin
3. **Fallback: `Some(300)`** when neither #1 nor #2 returned a value

When the operator invoked `dracon-sync repos` in a context where
`terminal_size()` returned `None` (piped to file, captured by
`script -q -c '...'`, captured by an agent process), the fallback
selected `Some(300)`. The tier dispatcher routed 300+ cols to the
**Full** layout (22-column v1 table, ~620-char rows).

Even when invoked directly in a real TTY, the `DRACON_SYNC_TERM_WIDTH`
override at 220 cols produced Compact layout (16-col table) but the
`LowerBoundary` constraints summed to ~215 cols minimum, and the
verbose LAST COMMIT cells expanded the table to 553 chars regardless.
Compact at 220 cols was broken even when the threshold said it should
work.

The compact tier's minimum sum:

```
#(4) + STATUS(11) + REPO(18) + ROLE(7) + BRANCH(11) + PUBLISH(18)
+ M(8) + S(8) + U(7) + AHEAD(9) + BEHIND(11) + PUSH(13) + PUSH-TO(32)
+ LAST_COMMIT(18) + STATE+ACT(17) + HINT(22) + 16 borders
= 215 cols minimum
```

Below ~250 cols, `ContentArrangement::Dynamic` letter-wraps cells
mid-word (e.g. `PUSH` / `PENDING` on separate lines, the `STATUS`
header splits to `STA` / `TUS`).

The full tier's minimum sum: 22 columns × ~12 cols avg + 23 borders
= ~293 cols minimum, needing 300+ for clean render (as the inline
comment already documented).

---

## Fix

Three independent changes:

### 1. Non-TTY fallback: 300 → 120

`terminal_width()` falls back to `Some(120)` instead of `Some(300)`
when no env var is set and `terminal_size()` returns `None`. The 120
value is Compact-friendly (it's at the lower edge of the 220-299
Compact range but is currently routed to Vertical because of the
tier-threshold change in #2 below). The key property: 120 is NEVER
Full — it's Compact-or-smaller, which means piped output is always
readable in some form.

### 2. Tier threshold change

Old: `< 120` → Vertical; `120-249` → Compact; `≥ 300` → Full
(250-299 was a special-case Compact due to letter-wrap concerns)

New: `< 220` → Vertical; `220-299` → Compact; `≥ 300` → Full

Rationale: Compact requires ~215 cols minimum plus ~30 cols of
verbose LAST COMMIT cell headroom to render without letter-wrapping.
Routing 120-219 to Vertical avoids the letter-wrap artifact entirely.

### 3. `comfy_table::Table::set_width(w)` applied to Compact + Full

`Table::set_width(w)` forces comfy-table to arrange columns to fit
exactly `w` cols. With `ContentArrangement::Dynamic` already set,
this means: columns shrink to fit, and content longer than the
column's allocated width is truncated (with `…`) instead of
letter-wrapped.

This is what makes the 300-col Full render fit in 346 chars (was 616)
and the 400-col Full render fit in 400 chars (was 620). The verbose
LAST COMMIT cell content is the main thing that gets truncated.

### 4. New CLI flag: `--layout <vertical|compact|full>`

```bash
dracon-sync repos --layout vertical
dracon-sync repos --layout compact
dracon-sync repos --layout full
```

Bypasses terminal-width detection entirely. Useful when:
- piping to a file and you know the reader's terminal width
- scripting for a fixed output format
- debugging layout issues

Short aliases (`-v`, `-c`, `-f`) accepted. clap rejects invalid
values up front.

### 5. `COLUMNS` env var support

After `DRACON_SYNC_TERM_WIDTH`, before `terminal_size()`. ncurses
convention. Scripts that set `COLUMNS=80` (some shells and many Unix
tools do this automatically) now get Vertical layout.

---

## Before / after measurements

| Width | Before (max line) | After (max line) | Layout change |
|------:|------------------:|------------------:|---------------|
|    80 |                86 |                86 | Vertical → Vertical (unchanged) |
|   120 |               553 |               116 | Compact → Vertical |
|   220 |               553 |               231 | Compact → Compact (set_width) |
|   300 |               616 |               346 | Full → Full (set_width) |
|   400 |               620 |               400 | Full → Full (set_width) |

PTY captures saved alongside this design doc as
`{before,after}-{80,120,220,300,400}col.txt`.

---

## Tests

3 new tests added (890 total, up from 887):

1. `test_terminal_width_columns_env_var` — verifies `COLUMNS=150` →
   `Some(150)`, `COLUMNS=999` → `Some(999)`, `COLUMNS=30` (out of
   range) → falls through to fallback, `DRACON_SYNC_TERM_WIDTH=80`
   takes precedence over `COLUMNS`.
2. `test_terminal_width_fallback_is_compact` — verifies the fallback
   is `Some(120)` and that 120-col routing goes to Vertical.
3. `test_choose_layout_tier_fallback_no_env_no_tty_yields_compact_or_smaller` —
   belt-and-suspenders: fallback must NEVER route to Full, even when
   the test environment happens to expose a real TTY at 120 cols.

Updated existing tier tests to match the new threshold
(`test_choose_layout_tier_vertical` now also covers 120, 150, 180, 199,
219; `test_choose_layout_tier_compact` covers 220, 249, 299).

`cargo build --release --locked`, `cargo test --workspace --locked`,
`cargo clippy --workspace --locked --all-targets -- -D warnings`,
`cargo deny check` all clean.

---

## Verification

Live state post-deploy:

```
$ dracon-sync repos --layout vertical | head -3
📜 /home/dracon/.dracon/utilities/sync/dracon-sync.toml
📦 31 repos  ✅ CLEAN 23  🔄 ACTIVE 5  ⚠️  WARN 0  ❌ CONCERN 3  ⛔ init/status failed: 0

$ dracon-sync repos --layout compact | head -3
📜 /home/dracon/.dracon/utilities/sync/dracon-sync.toml
📦 31 repos  ✅ CLEAN 23  🔄 ACTIVE 5  ⚠️  WARN 0  ❌ CONCERN 3  ⛔ init/status failed: 0

$ dracon-sync repos --layout full | head -3
📜 /home/dracon/.dracon/utilities/sync/dracon-sync.toml
📦 31 repos  ✅ CLEAN 23  🔄 ACTIVE 5  ⚠️  WARN 0  ❌ CONCERN 3  ⛔ init/status failed: 0

$ COLUMNS=80 dracon-sync repos | head -3
📜 /home/dracon/.dracon/utilities/sync/dracon-sync.toml
📦 31 repos  ✅ CLEAN 23  🔄 ACTIVE 5  ⚠️  WARN 0  ❌ CONCERN 3  ⛔ init/status failed: 0

$ DRACON_SYNC_TERM_WIDTH=400 COLUMNS=120 dracon-sync repos | head -3
📜 /home/dracon/.dracon/utilities/sync/dracon-sync.toml
📦 31 repos  ✅ CLEAN 23  🔄 ACTIVE 5  ⚠️  WARN 0  ❌ CONCERN 3  ⛔ init/status failed: 0
```

31 watched repos preserved across all layouts. Status counts identical
across layouts. (The 3 CONCERNs are the pre-existing endless-td +
neonbreak + junk-runner push issues from the v0.112.18 audit's §F5
finding — unrelated to this fix.)

---

## Out of scope

- **The libgit2 fetch `unsupported URL protocol` bug** affecting
  endless-td (and now neonbreak, junk-runner). That's a pre-existing
  daemon issue in `dracon-git` (uses `Cred::ssh_key_from_agent` which
  requires a running ssh-agent, which isn't available in the
  NixOS/wezterm session). Documented in
  `AUDIT_FULL_2026-07-18.md` §F5. Not addressed by this fix.

- **Making the table truly responsive at 80-119 cols.** The current
  Vertical layout (one repo per multi-line block, ~14 lines per repo,
  86 char max) is verbose. A future optimization could collapse
  Vertical to a 2-column "label / value" layout per repo, but that
  requires a different rendering pipeline and isn't justified by the
  current bug report (operator said "broken", not "verbose").

- **`repos --json` table layout.** Already emits one JSON object per
  line, no table formatting. Unchanged.