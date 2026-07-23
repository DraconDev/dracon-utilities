# Release Notes — v0.112.36 (2026-07-22) — operator-reported fixes

**Headline**: Two fixes from live operator reports on the same
darklord repo. **824 daemon tests** (+3), clippy + deny clean.

## 1. M10 guard honors ownership overrides (darklord WARN)

The v0.112.33 M10 pre-commit identity guard verified the effective
committer identity against the raw trusted lists. darklord uses a
DELIBERATE per-repo identity (`darklord-dev <darklord@dracon.local>`)
AND has `owned = true` in its `.dracon/dracon-sync.toml` — the raw
check blocked every commit: 101 staged files sat for a day, with a
journal warning every ~50s.

`commit_allowed_by_ownership` now has two acceptance paths:

1. `owned = true` in the repo's `.dracon/dracon-sync.toml` — the
   operator explicitly blessed the repo; the commit identity is
   their choice (darklord).
2. `user.email ∈ trusted_emails` AND `user.name ∈ trusted_authors`
   — the F0.1 case (no override, poisoned `test@test`) still blocks.

The guard deliberately does NOT re-adjudicate origin trust — the
daemon loop's ownership gate already did that before dispatch; this
check exists only to catch identity DRIFT after the cached
classification. (An intermediate version used full `detect_ownership`
including origin-URL trust — that broke mirror tests with local-path
origins and was the wrong scope for a commit-time check.)

Also: `Blocked` outcomes (needs-human states) now cool the repo down
300s — the previous ~50s retry cadence churned identical
stage→block cycles and log lines.

New tests: override honored / untrusted blocked / trusted allowed.
Verified live: darklord's 101 files committed (`6d381adb`) and
synced within a minute of deploy.

## 2. WARN status-cell width drift (darklord "visually broken" row)

The WARN status cell used ⚠️ (U+26A0) — `unicode-width` counts it
**1**, but terminals render it **2** cells (emoji presentation).
Every WARN row's separators drifted one column right of the table
frame. Replaced with 🟡 (yellow circle — `unicode-width` 2 =
rendered 2) in the STATUS cell and the tally line. 🟡 keeps the
yellow warning semantics AND matches the activity-dot family
(🟢⚪⚫🟣 — all width-2).

Verified: measured all rows against the frame — every row's right
border now lands at display column 115 (= frame edge).

## Test discipline

- `cargo test --workspace --locked` ✅ **824 daemon** (+3), warden
  83, security ~111, system 86 — 0 failed
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean
