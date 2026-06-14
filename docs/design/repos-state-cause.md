# Repos `STATE` Column

**Status:** Approved · **Date:** 2026-06-14

## Purpose

The `dracon-sync repos` table used to expose the raw signals (last-commit
time, last-push time, dirty count, ahead/behind, push status) without any
synthesis. The user could not tell at a glance whether a repo was
freshly synced, waiting on the daemon, stalled, or cold idle. The `STATE`
column combines those signals into a small fixed vocabulary the user can
scan without thinking.

## Vocabulary

The classifier returns exactly one of these labels per repo:

| Label | Trigger | Colour | Icon |
|-------|---------|--------|------|
| `active` | clean, in sync, and both commit and push are within `active_commit_minutes` (default 5m) | green | 🟢 |
| `committing` | unpushed commits are waiting, or the last commit is within `committing_commit_minutes` but outside the active window | yellow | 🟡 |
| `pushing` | `push_status = PENDING` (the daemon is mid-cycle) | yellow | 🟣 |
| `synced` | clean, in sync, commit/push within `committing_commit_minutes` but outside the active window | green | 🟢 |
| `stalled` | dirty tracked/staged work, or behind/upstream state, is sitting with no push progress | red | 🔴 |
| `dirty` | dirty tracked/staged work that does not fit the stalled/committing cases | yellow | 🟠 |
| `untracked-only` | only untracked files, no modified/staged | white | ⚪ |
| `intentional` | repo flagged `intentional_no_upstream = true` | magenta | 🟣 |
| `failed` | `push_status = FAIL` or `STUCK` | red | ⛔ |
| `idle` | clean, no recent activity, last commit within `cold_commit_minutes` | white | ⚪ |
| `cold` | last commit older than `cold_commit_minutes` (default 24h) | dark grey | ⚫ |
| `healthy` | fallback when nothing else matches | dark grey | ✅ |

## Why the order matters

The classifier is order-dependent. More specific / operator-explicit
labels take precedence over computed fallbacks:

1. `failed` (push failure) and `pushing` (daemon mid-cycle) are
   operator-explicit signals, so they win over every other label.
2. `intentional` (per-repo opt-in flag) is the user's explicit
   declaration that "this repo is intentionally isolated", so it
   wins over the computed staleness labels.
3. `stalled` is the user's "stalling for minutes" pain case, so it
   fires before the looser `committing` and `dirty` fallbacks. The
   `stalled` label is *not* based on the age of the previous HEAD
   commit: if tracked/staged work is sitting in the working tree with
   no unpushed commits, the repo is stalled even when the last commit
   was only a few minutes ago.
4. `untracked-only` is reported as such even when the last commit is
   recent, because the operator's question is "do I have uncommitted
   work?" and untracked files do not count.
5. `active` fires only for clean, in-sync repos whose commit and push
   are both inside the active window. It means "freshly synced", not
   "the user is still editing files right now".
6. `synced` is the clean + in-sync case whose commit/push is within
   the committing window but outside the active window.
7. `committing` covers unpushed commits waiting to settle, or a clean
   repo whose last commit is in the committing window but outside the
   active window.
8. `dirty` is the broad fallback for dirty tracked/staged work that
   does not match the stalled/committing cases.
9. `cold` fires when the last commit is older than the cold threshold
   and the row is otherwise clean.
10. `idle` is the final "clean, no recent activity" label, and
    `healthy` is the universal fallback.

## Thresholds

The thresholds live in the global policy:

```toml
# /etc/dracon-sync.toml or ~/.dracon/utilities/sync/dracon-sync.toml
active_commit_minutes = 5         # default 5
committing_commit_minutes = 60    # default 60
cold_commit_minutes = 1440        # default 1440 (24h)
```

Per-repo overrides are supported via
`<repo>/.dracon/dracon-sync.toml`:

```toml
# Wider active window for a repo with a long build cycle.
active_commit_minutes = 30
committing_commit_minutes = 180
```

The override path uses the same mechanism as
`intentional_no_upstream` — the per-repo TOML is loaded by
`load_repo_override` and merged into the row builder at
classification time.

## `last_push_for_branch` regression

The `PUSHED` column (the relative time of the most recent push) used
to call `git reflog show origin/<branch> --format=%cr -1`. For repos
that were freshly cloned and never fetched again, the remote-tracking
reflog has no entries and the command returns empty output, which
surfaced as a misleading `-` in the `PUSHED` column. The helper now
uses `git log -1 --format=%cr origin/<branch>`, which returns the
committer date of the remote-tracking tip regardless of the reflog
state. The fix is documented in the source as an explicit
implementation note.

## Verification

The classifier has 13 unit tests covering each label plus the per-repo
override path. The `last_push_for_branch` fix has its own regression
test that constructs a freshly-cloned repo with an empty reflog for
`origin/main` and asserts the helper returns a real date.

The full validation suite (fmt, clippy, test, build, deny,
verify-spec, install --dry-run, repos JSON, repair concerns/warns,
doctor, warden, secret scans, three-remote SHA alignment) must stay
green after any change to this classification.
