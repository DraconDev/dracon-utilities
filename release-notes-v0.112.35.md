# Release Notes — v0.112.35 (2026-07-22) — activity-label date parser

**Headline**: One-line-class UI fix. **821 daemon tests** (+1),
clippy + deny clean.

## Fix: repos with commits older than ~2 weeks lost their activity indicator

`activity_label` (the `repos` WHAT cell) used
`parse_relative_minutes_to_u64`, a unit-limited duplicate of the
report's full `parse_relative_minutes` — it handled only
seconds/minutes/hours/days. Any repo whose last commit was "N
weeks/months/years ago" got `None`, and the cell rendered the bare
state (`healthy`) with no activity indicator — spotted live on
`DraconDev` (last commit "4 weeks ago").

`parse_relative_minutes_to_u64` now delegates to the complete,
already-tested `parse_relative_minutes` (single source of truth).
Verified live: `DraconDev` now shows `⚫ cold 28d · healthy`.

Regression test:
`test_parse_relative_minutes_to_u64_handles_weeks_months_years`.

## Test discipline

- `cargo test --workspace --locked` ✅ **821 daemon** (+1), warden 83,
  security ~111, system 86 — 0 failed
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean
