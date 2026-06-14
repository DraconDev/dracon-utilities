# Sync Push Classification

**Status:** Approved · **Date:** 2026-06-13

## Purpose

Document the deterministic rules the `dracon-sync` daemon uses to
classify a repository's push state and decide what to do about it. This
is a *behavioral* contract: it ties together four pieces of code
(`daemon.rs`, `report.rs`, `git/push.rs`, `git/multi_remote.rs`) and
three CLI surfaces (`dracon-sync repos`, `dracon-sync repair concerns`,
`dracon-sync repair stuck-list`).

## Why this exists

The user-visible `repos` table used to be inconsistent with the
`repair concerns` command and with the daemon's own retry behaviour.
Three logic defects were the root cause:

1. `STUCK_PUSH` fired on **any** `ahead > 0`, even when the daemon had
   not yet tried to push the repo in this cycle.
2. The multi-remote push loop retried **permanent** rejections
   (protected branch, hook declined) until the retry budget was
   exhausted, then logged a new incident every cycle.
3. `repair concerns` still used the old `ahead > 0 → concern` rule after
   the `repos` table had been fixed, so the two surfaces disagreed on
   which repos were concerns.

The fix is a small set of explicit rules. The rules are simple, the
incident ledger is the single source of truth for "the daemon tried
recently and failed", and the four code sites are forced to consult it.

## Invariants (must always hold)

1. **`repos` table and `repair concerns` agree.** If a repo is a
   CONCERN in the `repos` table, `dracon-sync repair concerns` (in
   dry-run) must list it. The dry-run count and the table count must
   be equal. The `stuck-push` repair filter uses the same recent-push-
   failure requirement as the table's `STUCK_PUSH` flag.
2. **No retry or transport fallback on permanent rejections.** A push
   error matching the permanent-rejection regex set returns immediately
   from `multi_remote::push_to_named_remote`, `push_with_retries()`, and
   the lower-level `push_with_transport_fallbacks()` path without
   consuming a retry slot or trying HTTPS fallback.
3. **One incident per (repo, cycle) for permanent rejections.** The
   daemon logs a single `permanent_push_rejection` incident per cycle;
   the next cycle may log another only if the daemon attempted the
   push again.
4. **AHEAD without a recorded push failure is PENDING, not STUCK.** The
   `recent_push_failure` window is exactly 10 minutes (600s) and is
   checked against the incident ledger, not against in-memory state.
5. **Hints must match the row classification.** A dirty repo with unpushed
   commits but no recent push failure is still a `WARN` row, so its hint
   says the daemon will push after changes settle instead of suggesting
   `repair-concerns`.
6. **Intentional isolation is not a hidden concern.** A repo whose
   `.dracon/dracon-sync.toml` sets `intentional_no_upstream = true` is
   recognized as intentionally untracked by any remote. The
   `repos` table replaces `NO_UPSTREAM` with the explicit
   `INTENTIONAL_NO_UPSTREAM` flag, the `PUSHED` column shows
   `INTENTIONAL` (rendered green), and `dracon-sync repair concerns`
   skips the repo entirely. The auto-repair path must never run
   `git push -u origin HEAD` for these repos.
7. **The `STATE` column is the user-readable summary of the row.** A
   repo with `modified > 0` or `staged > 0` and no unpushed commits is
   `stalled` — this is the "we changed files but then stopped" case
   the operator asked about. The previous HEAD commit timestamp is not
   treated as proof that someone is still editing. The thresholds
   (`active_commit_minutes`, `committing_commit_minutes`,
   `cold_commit_minutes`) live in the global policy with optional
   per-repo overrides in `RepoPolicyOverride`.

## Classification rules

For a repo with a valid `origin` and a tracking `upstream` branch:

| State | `repos` flag | `repair concerns`? |
|-------|--------------|---------------------|
| Clean, no unpushed commits | `OK` | No |
| Unpushed commits (ahead > 0), no recent push failure | `PENDING` | No |
| Unpushed commits (ahead > 0), recent push failure (< 10 min) | `STUCK_PUSH` | Yes |
| Behind remote (behind > 0) | `STUCK_PULL` | Yes |
| No `origin` remote | `NO_ORIGIN` | Yes |
| No tracking `upstream` branch | `NO_UPSTREAM` | Yes |
| No tracking `upstream` branch, repo flagged `intentional_no_upstream = true` | `INTENTIONAL_NO_UPSTREAM` | No (skipped) |

`recent_push_failure` is true when the incident ledger contains an
entry for this repo with `scope = "sync"` or `scope = "mirror"` and
`result` containing `"fail"` within the last 600 seconds. The window is
intentionally short: it captures the daemon's most recent attempt, not
a historical "this repo is flaky" signal. Operators who want a
longer-term "unhealthy repo" view should grep the ledger directly.

## Permanent rejection regex set

`is_permanent_push_rejection(err_msg: &str) -> bool` in
`dracon-sync/src/git/push.rs` returns `true` when the push error
contains any of:

- `pre-receive hook declined` — generic server-side hook rejection
- `protected branch` — GitLab/Codeberg/GitHub protected branch guard
- `not allowed to push` — GitLab's "You are not allowed to push code
  to protected branches" message
- `deny updating` — server-side deny-updating config
- `hook declined` — generic fallback for any pre-receive / update hook
  rejection (lowercased substring match)

The function does **not** match:

- `non-fast-forward` — recoverable via rebase or pull
- `connection timed out` / `connection refused` — transient, worth
  retrying with HTTPS fallback
- `failed to push some refs` (alone) — too generic; usually accompanied
  by one of the permanent patterns above

New permanent patterns can be added freely; the function is `pub(crate)`
and has 4 unit tests guarding the current set.

## Retry policy

```
attempt push
  ↓
on success: done
  ↓
on failure:
  if is_permanent_push_rejection(err):
    log incident, return error (NO retry, NO fallback)
  else if is_push_rejected(err) and force_when_behind:
    pull + retry once
  else if push_retries > 0:
    sleep + retry (with HTTPS fallback on persistent timeout)
  else:
    log incident, return error
```

`push_retries` (default 3) is consumed only by the transient-retry
branch. Permanent rejections never burn a retry slot.

## Incident-ledger contract

Every push attempt (success or failure) MUST log to
`~/.local/state/dracon/dracon-sync-incidents.jsonl`. Successful pushes
log a `result: "ok"` entry; failures log `result: "fail"` with the
error message in `details`. The `recent_push_failure` window scans the
last 500 lines (the ledger is read tail-first) for entries matching the
repo path. Operators pruning the ledger should keep at least the last
10 minutes of entries or the `STUCK_PUSH` signal will lag.

## Cooldown interaction

`stage_cooldown_secs` (default 3600) pauses a repo's auto-staging for
the configured duration after a `git add` timeout. This is independent
of the push classification: a repo in stage cooldown is not flagged
`STUCK_PUSH` (it never got to the push step), but it is invisible to
the rest of the sync loop until the cooldown elapses. The cooldown
is per-repo; other repos are unaffected. The daemon loop enforces this
cooldown directly, so the repo is skipped until the timer expires.

## Out of scope (intentionally not changed)

- `auto_github_private` repo creation is not classified. If `gh repo
  create` fails, the daemon retries with HTTPS fallback; a permanent
  failure (e.g., 403 from the GitHub API) is logged but does not
  block other remotes.
- The webhook notifier fires for every push failure regardless of
  classification. The classification determines whether the daemon
  retries; the webhook is purely informational.
- `repair stuck-list` is the operator's escape hatch for repos that
  have been `STUCK_PUSH` long enough to require manual intervention.
  It is unaffected by this classification and always lists the same
  set of repos (those with origin/upstream that have failed recently).
