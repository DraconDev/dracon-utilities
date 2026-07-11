# V2 Card Design — Snapshot (2026-06-16)

> Reference snapshot of the **v2 card design** for the `dracon-sync repos`
> table. Captured here after the design was reverted to the v1
> `comfy_table`-based layout on 2026-06-27 (see
> `docs/design/repo-remote-visibility-v3-revert-2026-06-27.md`).
>
> The original snapshot file (`src/report_v2_snapshot.rs`, 237 KiB /
> 6339 lines) was removed from the source tree on 2026-07-11 to stop
> shipping dead tracked code. Its content is summarized below so the
> design can be restored without digging through git history.

## Why this design existed

The v2 design introduced a richer card layout for `dracon-sync repos`:

- `render_repo_card` — a multi-line card per repo with per-field
  labeling, state color, and a compact legend.
- `render_push_to_with_icons` — the multi-remote publish column
  rendered with per-remote status glyphs (✅ / 🟣 / ⚠️ / etc.)
  instead of plain text.

It added:

- `StateCause::icon()` — a glyph per state cause (Unowned, Ahead,
  Behind, Diverged, Dirty, etc.).
- `state_cause_as_str` — a machine-parseable string form of the
  state cause (e.g. `unowned:untrusted_origin`).
- A multi-line legend, subject truncation, and publish-label
  shortening in the card footer.

## Origin

Introduced across these commits (per the original snapshot header):

- `3eb648f` — added `render_push_to_with_icons`, `render_repo_card`.
- `7a525cb` — removed `format_push_to_remotes_cell`,
  `StateCause::icon()`, `state_cause_as_str` (part of the v2 iteration).
- `78f5a68` — multi-line legend, subject truncation.
- `14a19d3` — publish label shortening, hint truncation.

## Why it was reverted

The operator (DraconDev) reverted on 2026-06-27:

> "i am not over the new dracon-sync repos table its less informative
> than the one we have before"

The v1 `comfy_table`-based layout was judged more information-dense
per row, so the daemon's `run_repos_report()` was switched back to
call the `comfy_table` renderer instead of `render_repo_card()`.

## How to re-enable the v2 design

1. Move the rendering functions (`render_repo_card`,
   `render_push_to_with_icons`) back to `src/report.rs`.
2. Remove the `format_push_to_remotes_cell`, `StateCause::icon()`, and
   `state_cause_as_str()` restoration from `src/report.rs` (these were
   re-added when v1 was restored; they conflict with the v2 glyph
   approach).
3. Update the main loop in `run_repos_report()` to call
   `render_repo_card()` instead of the `comfy_table`-based rendering.

The prior `src/report_v2_snapshot.rs` is recoverable at git commit
`4f287f1` in the `dracon-sync` repo if a line-level diff against the
current `src/report.rs` is needed.
