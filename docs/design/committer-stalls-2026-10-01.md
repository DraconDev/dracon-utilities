# Committer stalls — 2026-10-01

## Cause and repair

The classification spawn gate in dracon-sync 0.113.91 used
`until.is_none_or(|until| now < *until)`. This admits jobs DURING a
cooldown, removes the deadline, and blocks jobs AFTER expiry. It explains
both dracon-strategy's 20,000+ rapid failures and the long quiet stalls.
The corrected gate is `now >= *until`; the regression covers missing,
future, exact, and expired deadlines.

A targeted `sync-now` successfully committed and synced freeport (19 files),
junk-runner (1), capture-anime-girls (39), doomtap (44), and dracon-platform
(42). Strategy's existing commit was pushed fast-forward to github/gitlab.
No history rewrites or worktree resets were used.

Strategy's `ai-auto-music/catalog/cookbook.json` was 69,649,482 bytes;
Warden's required clean filter refuses text over 67,108,864 bytes. Under
the existing shared flock, the catalog was atomically compacted to
53,784,216 bytes with parsed data equality verified. The shared JS JSON
writer now compacts output when formatted JSON reaches 60 MiB. Its
regression verifies a large formatted catalog fits and preserves all records.
Other writers that do not use this helper can still reintroduce formatting;
a catalog exceeding 64 MiB even when compact needs a separate storage design.

## Verification

The recovered repos were reported synced by `repos --summary`. Active loops
can introduce new edits immediately after each successful cycle.
Full-workspace test/build/clippy attempts encountered a concurrent syntax
error in dracon-system, so dracon-sync-only validation was run separately.
Cargo deny passed. The catalog lock suite passed all three tests.

Existing unrelated concerns include vanished monster-minecraft and the old
nested folder-auto-banner path, and stale alternate remotes. These are not
the cause of the recovered commit stalls; retiring or restoring them needs
an operator decision about their intended locations/projects.
