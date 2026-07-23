# Release Notes — v0.112.37 (2026-07-22) — sustained-state desktop notifications

**Headline**: Operator-requested — desktop notifications when
something is wrong **for a while**. **825 daemon tests** (+1),
clippy + deny clean.

## The gap

The daemon already notified for Stuck-Ahead, Stuck-Behind, and
Mirror-Degraded (plus PushFailed / stuck-budget / max-failures from
v0.112.31/33). But two problem classes had **no desktop signal at
all**:

- **Blocked** (merge/rebase in progress, commit-time ownership
  guard): the darklord M10 block sat for ~a day with zero desktop
  notifications.
- **Unowned** (ownership guard skipping a repo): the F0.2 incident
  (daemon's own source repo unowned for 25 minutes) was journal-only.

## What's new

Both join the sustained-state notification loop:

| State | Threshold | Re-arm | Message |
|---|---|---|---|
| **Blocked** | 30 min continuous | 30 min | "Sync Blocked (>30 min) — blocked by a guard or needs manual intervention (merge/rebase or ownership/identity check) — run: dracon-sync repos -s" |
| **Unowned** | 15 min continuous | 30 min | "Repo Unowned (>15 min) — daemon is skipping this repo (untrusted identity) — run: dracon-sync ownership --explain" |

Notifications are `notify-rust` desktop notifications (Critical
urgency, spawned in background) and also recorded in the incident
ledger (`record_sync_alert`), so they appear in `repos` history.

## Implementation

- New `blocked_since` field on `RepoActivity` — set on
  `SyncOutcome::Blocked` (both apply-phase sites), cleared on any
  non-Blocked outcome, so only CONTINUOUS blocks count.
- New `unowned_since` field — set in the ownership-skip branch,
  cleared when the repo classifies as owned again (or the activity
  entry is dropped).
- Both notifications use the v0.112.31 expiring `notify_throttled`
  (re-fires every 30 min while the state persists — no more
  fire-once-forever).
- Extracted `sustained_threshold_met(since, now, threshold)` —
  pure, unit-tested, shared by all four sustained checks
  (ahead/behind/blocked/unowned).

## Test discipline

- `cargo test --workspace --locked` ✅ **825 daemon** (+1), warden
  83, security ~111, system 86 — 0 failed
- `cargo clippy --workspace --locked -- -D warnings` ✅ clean
- `cargo deny check` ✅ clean
