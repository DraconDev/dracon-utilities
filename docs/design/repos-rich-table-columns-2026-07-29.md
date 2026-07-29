# repos-rich-table-columns — 2026-07-29

> **v0.113.8** — operator-asked columns (USED / COMMITS / SIZE /
> TOUCHED) added to the rich-table default view. HINT prose column
> dropped. Per-repo drill-down moved to `repos <name>` /
> `--layout vertical`. Architectural rationale and rejected
> alternatives documented here for future agents.

## What the operator asked for

In the operator's words: *"we are obviously checking to see if
anything wrong or can be wrong later"* and *"i would like to
see which repos are used, but also the size of the repo would
be useful too to see if its growing to be a concern, but also
i somewhat leave to your judgement what else would be very
informative, so we are obv checking to see if anything wrong
or can be wrong later"*.

Three concrete diagnostic gaps were identified in the existing
rich-table default view:

1. **"Which repos are used"** — the ACTIVITY column shows the
   daemon's last action timestamp + the activity label, but
   doesn't answer "am I actively touching this repo in the last
   hour?". Operators iterating on multiple repos via glla
   needed a column that flags "this repo is currently in use".

2. **"The size of the repo to see if it's growing to be a
   concern"** — the per-repo `.git` size was already computed
   (via `git count-objects -v`) and cached (1h TTL per the
   `REPO_SIZE_CACHE_TTL_SECS` policy), but only surfaced in the
   per-repo detail view. The CONCERN column already had a
   `.git exceeds 2 GB (github limit)` hint, but the operator
   had to drill in to see the actual size + whether it was
   growing. Capacity planning needed a fleet-wide size table.

3. **"More detailed info about staged, committed, changes so
   on"** — the 1h/6h/24h commit windows were already collected
   per-row (`commits_1h`, `commits_6h`, `commits_24h` in
   `RepoReportRow`) but only shown in the verbose `--layout
   vertical` (and even there as 3 separate columns). The
   rich table's ACTIVITY column had room for `⏳ dirty 8m · 1
   mod` inline but was truncating the dirty-count tail at 21
   cols.

## What was shipped (v0.113.8)

The rich-table default view grew from **7 → 10 columns** and
the ACTIVITY column widened from **21 → 28 cols**:

| # | STATUS | REPO | ACTIVITY | A/B | PUSH | **USED** | **COMMITS** | **SIZE** | **TOUCHED** |

Each new column answers one of the operator's questions:

### USED — combined human + daemon activity tier

🟢used = daemon active OR human commit in last 1h · 🟡mod = 1h-24h · ⚪idle = 1d-7d · ⚫cold = 7d+

The daemon pushes/commits show up as `🟢used` even without
human commit activity (the operator's loop is running the
repo), so a repo being iterated by glla is correctly marked
active. The 4 tiers are tighter than the standalone ACTIVITY
thresholds (60m/24h/7d) because USED is the
operator-iteration summary, distinct from the fleet-health
summary that ACTIVITY already provides.

**Helper**: `used_label(row: &RepoReportRow) -> String` in
`dracon-sync/src/report.rs:4663`.

### COMMITS — 1h/6h/24h split

`N/N/N` format. `13/45/152` means 13 commits in last 1h,
45 in last 6h, 152 in last 24h. Empty repos render `-/-/-`.

**Helper**: `commits_window_label(row) -> String` in
`dracon-sync/src/report.rs:4702`.

### SIZE — gitdir bytes, color-coded by the github pack-limit concern

Adaptive units (B → KiB → MiB → GiB). Color the cell:
- **Red** at `pack_too_large = true` (github genuinely refuses
  the push; matches the daemon's `PACK_SIZE_WARNING` /
  `pack_too_large_forces_concern` predicate exactly).
- **Yellow** at ≥ 1 GiB gitdir (capacity-planning warning zone,
  irrespective of whether the push is actually broken).
- **White** at < 1 GiB gitdir (normal).

**ADVISOR-CATCH (v0.113.8 follow-up)**: the original `size_label`
colored the cell red based on `git_size_bytes` ≥ 2 GiB. But
`git_size_bytes` (from `git count-objects -v`) is the COMPRESSED
pack-on-disk size, while the daemon's PACK_SIZE_WARNING concern
fires on the PUSHABLE-UNCOMPRESSED blob sum (the bytes that would
actually ship to a remote). These diverge exactly where it
matters: **deathrun** has 4.08 GiB gitdir (pre-gc pack residue)
but is ✅ CLEAN (the pushable is well under 2 GiB
post-orphan-cutover). The original red would falsely read as
"github push broken" when it isn't.

The fix threads `pack_too_large` (the same bool the daemon
uses for PACK_SIZE_WARNING) through `size_label(Option<u64>, bool)`.
Red iff `pack_too_large == true` (the actual github-rejection
condition); yellow iff gitdir ≥ 1 GiB (capacity warning
independent of push). The SIZE cell color and the row's
CONCERN/ACTIVE state are now visually consistent.

**Helper**: `size_label(bytes: Option<u64>) -> (String, Color)`
in `dracon-sync/src/report.rs:4722`.

### TOUCHED — last commit author + when

`<author> <when>` (e.g. `DraconDev 14m`, `dracon 10 sec`).
Long author names truncate to 10 cols with ellipsis.
Empty repos render `- -`.

**Helper**: `touched_label(row: &RepoReportRow) -> String` in
`dracon-sync/src/report.rs:4786`.

## Rejected alternatives (decision rationale)

### Option A — Keep HINT, add columns after

Pro: HINT prose preserved (the operator already knew what
each hint meant).
Con: 12 columns total; at 200-col terminals the table
letter-wraps or auto-shrinks every column. Need to retest
`--legend` and bump `test_full_table_headers_fit_columns`.

**Decision**: rejected. The 10-column set is the maximum
that fits in a 165-col terminal cleanly. Going wider forces
either content truncation or horizontal scrolling on standard
terminal sizes (240 cols is the most common wide-terminal
size).

### Option B — Bar-graph for SIZE

Pro: trend visible at a glance (bar grows over time).
Con: uses more horizontal space (12+ chars per cell), eats
other columns.

**Decision**: rejected for v0.113.8. Could revisit in a
follow-up if the operator wants trend visualization. The
adaptive-units + color-coding approach already shows
"concern-level" at a glance; bar-graph would add visual
weight without adding information.

### Option C — Drop SIZE, keep USED + COMMITS + TOUCHED

Pro: 3 new columns instead of 4; table stays at 9 cols
(160-col minimum).
Con: loses the operator's "is it growing to be a concern"
question, which was the most-asked-for single column.

**Decision**: rejected. The operator explicitly named the
size column as the most valuable addition. Keeping it is
worth the +1 column.

### Option D — Single combined "HEALTH" column

Pro: simpler mental model ("the repo's health in one column").
Con: loses the granular per-axis diagnostics. Operators
asking "is it growing?" want to see the actual size, not a
binary "growing/stable" tag.

**Decision**: rejected. The per-axis columns (USED for use,
COMMITS for cadence, SIZE for capacity, TOUCHED for
recency) are independently valuable for different operator
questions.

### Option E — Fold the 4 columns into ACTIVITY

Pro: 0 new columns, just expands ACTIVITY.
Con: ACTIVITY already shows "what's happening" with dirty
counts and timestamps. Adding USED + COMMITS + SIZE +
TOUCHED into ACTIVITY would either bloat ACTIVITY past the
28-col budget or force truncation that loses the diagnostic
information.

**Decision**: rejected. ACTIVITY = "what's happening now";
the new columns = "how is this repo's health / growth /
recency". They serve different operator questions.

## Trade-offs

**Lost**: the HINT column's prose. The `.git exceeds 2 GB
(github limit)` hint was the most-asked-for hint in the
fleet — it's now implicit from the SIZE cell color (red at
≥ 2 GiB). Other hints (e.g. `intentional legacy isolation,
no upstream configured`) are reachable via `dracon-sync
repos <name>` or `--layout vertical`.

**Gained**: 4 diagnostic columns answering the operator's
3 explicit questions. The information was already collected
and stored in `RepoReportRow`; this release only surfaces
it.

**Wide-terminal-only**: the rich table now requires ≥ 165
cols (was ≥ 90). Operators on 90-164 col terminals get the
Compact tier (16-column `--layout compact` view). The
Compact tier is unchanged from v0.113.7 — it still has the
HINT column + a PUSH-TO column for deep inspection on
narrower terminals.

**v0.113.8 follow-up (advisor-catch)**: the original
`size_label(bytes: Option<u64>) -> (String, Color)` colored
the cell red on `bytes ≥ 2 GiB`. This was wrong because
`git_size_bytes` measures the COMPRESSED pack-on-disk size
while the daemon's PACK_SIZE_WARNING concern fires on the
PUSHABLE-UNCOMPRESSED blob sum. **deathrun** has 4.08 GiB
gitdir but is ✅ CLEAN — the red cell would have falsely
read as "github push broken" when it isn't. Fix:
`size_label(Option<u64>, bool)` now takes `pack_too_large`
explicitly. Red iff `pack_too_large == true` (matches the
daemon's CONCERN predicate); yellow iff gitdir ≥ 1 GiB
(capacity warning independent of push); white otherwise.

## Terminal-width routing

`choose_layout_tier()` in `dracon-sync/src/report.rs:2474`:

| Width | Tier | Columns | Notes |
|---:|---|---|---|
| < 165 | Compact | 16 | Includes HINT + PUSH-TO for drill-down |
| 165-241 | **Rich (default)** | **10** | New USED + COMMITS + SIZE + TOUCHED |
| 242-314 | Compact | 16 | Same as < 165 |
| ≥ 315 | Full | 22 | The 22-column full-table view from v0.113.6 |

The Rich tier is the DEFAULT for terminals ≥ 165 cols (most
modern wide-terminal setups). Compact is the fallback for
narrower terminals.

## Test discipline

6 new unit tests cover the 4 new helpers + the rich-table
layout invariants:

- `test_rich_table_headers_fit_columns` — 10-case header-width
  matrix verifying each header text + 2 padding fits within
  its column minimum. Catches regressions when adding new
  columns.
- `test_used_label_tiers` — 6-case tier matrix for
  `used_label` covering daemon-active + human-recency paths.
- `test_commits_window_label_renders_split` — 4-case format
  matrix for `commits_window_label` including the empty-repo
  `-/-/-` rendering.
- `test_size_label_units_and_colors` — 8-case unit matrix
  (B / KiB / MiB / GiB) + 4 threshold-boundary assertions
  (1.99 GiB yellow, 2.00 GiB red, 1.00 GiB yellow, 999 MiB
  white).
- `test_touched_label_renders_author_and_when` — 3-case
  matrix covering normal / long-author-truncation /
  empty-repo paths.
- `test_rich_table_fits_narrow_terminal` — pins the new
  10-column total width ≤ 165 cols minimum. Catches
  regressions when adding new columns.

Updated tests:

- `test_choose_layout_tier_vertical` — now exercises widths
  165-241 (Rich zone), not 40-241.
- `test_choose_layout_tier_compact` — now covers 90-164
  (was: 242-299) + 242-299 (unchanged).
- `test_choose_layout_tier_fallback_no_env_no_tty_yields_compact_or_smaller`
  — 120 cols now routes to Compact (was Rich).

Total daemon test count: **854** (was **848** in v0.113.7;
the new tests added **6 net new**, minus 0 removed = 854).

`cargo test --workspace --locked` passes (1120+ tests).
`cargo clippy --bin dracon-sync --locked -- -D warnings`
clean.

## Cross-references

- `AGENTS.md` "Test discipline" — the new tests satisfy the
  "New code paths require unit tests" rule.
- `AGENTS.md` "Submodule standalone worktree design" — the
  source lives in `/home/dracon/Dev/dracon-utilities/dracon-sync/`
  as a NESTED STANDALONE git repo. The meta-repo at
  `/home/dracon/Dev/dracon-utilities/` does NOT track the
  source files (they're gitignored via `dracon-sync/`); only
  the meta-files (CHANGELOG, AGENTS.md, docs/) and the
  workspace Cargo.toml/Cargo.lock.
- `docs/design/pack-size-concern-2026-07-28.md` — the
  v0.113.7 release that introduced the `PACK_SIZE_WARNING`
  flag + the github-pack-limit concern. The SIZE column in
  v0.113.8 uses the same threshold (2 GiB) and the same
  color coding (red at the limit).
- `docs/design/repos-table-fix-2026-07-18.md` — the
  historical `--layout vertical` rich view (15 columns,
  full per-repo detail). The new rich-table layout is the
  default view; `--layout vertical` remains the per-repo
  drill-down option.
- `dracon-sync/release-notes-v0.113.8.md` — the per-release
  notes for v0.113.8 with full source-change table,
  before/after fleet state, and trade-off analysis.
