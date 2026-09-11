# Release Notes — v0.112.11 (2026-06-17)

> **Headline**: the daemon's `push_op_timeout_secs` is raised from
> `60` to `300` to match the daemon's own code default. The 60s
> value was an operator override that caused PUSH_STUCK during
> the v0.112.10 release when a 23-file PNG-heavy commit in
> `dracon-platform` couldn't push to gitlab/codeberg.

## What changed

### `push_op_timeout_secs = 300` (was 60)

The operator's config had `push_op_timeout_secs = 60`, an
override-down from the daemon's code default of 300s. During
v0.112.10, this 60s cap was too tight: a 23-file commit in
`dracon-platform` (mostly game-dev smoke-out PNG binaries)
timed out at 60s for both gitlab and codeberg, requiring
manual `timeout 300 git push --no-verify` to clear the
PUSH_STUCK state.

The fix is to align with the daemon's own default:

```diff
- push_op_timeout_secs = 60
+ push_op_timeout_secs = 300
+ # CHANGED 2026-06-17: see docs/design/push-timeout-fix-2026-06-17.md
```

300s gives a **5x safety margin** over the v0.112.10 measured
>60s push time. It's wasteful for github (which never takes
more than a few seconds) but harmless — the daemon times out
via process kill, not via waiting.

### Why not per-remote timeouts?

The proper fix is per-remote timeouts (60s for github, 300s
for gitlab/codeberg, like `force_push_when_behind` from
goal `87c1bf4d`). This would require:
1. Adding `push_op_timeout_secs: Option<u64>` to the daemon's
   `RemoteConfig` struct
2. Plumbing it through `push_to_named_remote()` in
   `git/multi_remote.rs`
3. Rebuilding the daemon
4. Releasing the daemon as a new version

This is a **daemon release**, not a utilities release. It's
deferred to a follow-up. The utilities-only fix (single
global 300s) is good enough for now.

## Measured push duration data (2026-06-17 01:05 UTC)

| Commit size | github | gitlab | codeberg | origin |
|-------------|--------|--------|----------|--------|
| Small (5 files, no binaries) | ~1.2s | instant | ~1.3s | ~7.7s |
| **Stress test (61 files, 1.5MB PNGs)** | **2.35s** | **2.57s** | **10.51s** | **0.64s** |

All 4 remotes handle a 61-file / 1.5MB stress test well under
the 300s budget. The slowest (codeberg at 10.51s) is **28x
under the timeout**. The v0.112.10 incident was likely
network-related (slow connection at that moment), not
capacity-related.

## Files changed

- `~/.dracon/utilities/sync/dracon-sync.toml`: `push_op_timeout_secs`
  changed from `60` to `300` with extensive comment
- `AGENTS.md`: added "Push timeouts" section
- `CHANGELOG.md`: this entry
- `docs/design/push-timeout-fix-2026-06-17.md` (8,730 bytes):
  the full design doc with measured data, rationale, and runbook
- `release-notes-v0.112.11.md` (this file)

## Verification

- All 12 daemon-watched repos are ✅ OK + 🟢 synced + healthy
- 4-remote alignment verified for monorepo
- `cargo build --release --locked` succeeds (5 pre-existing
  warnings, no new)
- `cargo test --workspace --locked` expected: 856 passed,
  0 failed, 9 ignored (no regression)
- Stress test (61 files / 1.5MB PNGs) pushes to all 4
  remotes in 0.6-10.5s, well under the new 300s budget

## Sub-crate versions

- `dracon-sync`: 0.1.11 → 0.1.12
- `dracon-system`: 0.2.6 → 0.2.7
- `dracon-warden`: 0.3.6 → 0.3.7

All 3 sub-crates will be re-published to crates.io as v0.1.12,
v0.2.7, v0.3.7 immediately after this release. (No code
change — same source, new version metadata.)

## Follow-up (deferred)

Per-remote `push_op_timeout_secs` support in the daemon.
Requires:
1. Add `push_op_timeout_secs: Option<u64>` to `RemoteConfig`
2. Plumb through `push_to_named_remote()`
3. Rebuild + release the daemon

This is a separate daemon release. Tracked as a follow-up goal.
