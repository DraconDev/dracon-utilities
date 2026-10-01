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

## Final deployment

The corrected release binary was atomically installed at
`~/.local/bin/dracon-sync`, with the previous binary saved at
`/tmp/dracon-sync-before-cooldown-fix-20261001`. The service was restarted
and reported healthy, freeze off, policy valid. The new PID automatically
committed and synced fresh platform edits, confirming scheduler recovery.
Validation: dracon-sync 1,149 unit tests + 10 integration tests passed
(3 ignored), release build and clippy passed, cargo deny clean.

## Identity follow-up — operator confirmed ownership

On 2026-10-01 the operator explicitly confirmed all watched repositories
belong to DraconDev and requested canonical identity everywhere. All 35
watched repositories now have local `user.name = DraconDev` and
`user.email = dracsharp@gmail.com`; effective `git var GIT_AUTHOR_IDENT`
was verified for each. This matches the pre-existing global Git identity.
The historical Come Get Me alias `come-get-me-dev` /
`come-get-me-dev@dracon.local` was explicitly added to the sync policy's
trusted lists. Its warning is now cleared. No historical commits were
rewritten or manufactured. Older TOUCHED authors remain historical records.
SIGHUP reloads the daemon policy without interrupting service.

The concurrent dracon-system syntax error has since been repaired by its
active session; `cargo check -p dracon-system --locked` succeeds.

Follow-up full-workspace validation completed successfully: 1906
tests passed (9 ignored), release build succeeded,
workspace clippy with `-D warnings` clean, cargo deny clean. The earlier
concurrent dracon-system build blocker is resolved.

## TOUCHED and recurring Strategy dirtiness

The operator asked for TOUCHED to show DraconDev for all confirmed aliases.
`git_log_meta` now uses `%aN`, Git's mailmap-aware author name, rather than
raw `%an`. The legend documents the mapping. A global mailmap at
`~/.config/git/dracon-identities.mailmap` maps seven explicitly confirmed
loop identities; raw commit author, hash, and time are preserved. An actual
Git fixture regression verifies mapped output and unchanged hash/raw author.
All 35 latest authors resolve to DraconDev with `%aN`.

Strategy's catalog was rewritten to 69,680,424 bytes at 12:08 because
`wave11-micro1-sync.service` had run since the previous day and retained the
old imported JS writer. Changing the module on disk did not update that
process. The service was restarted (retaining its existing wave12 drop-in),
then the live catalog was compacted under shared flock with parsed data
comparison, to 53,808,741 bytes. `sync-now` committed and synced it. The
restarted process now loads the bounded writer for subsequent results.

Workspace validation again intersected a new in-progress dracon-system
syntax error (line 6575), so this report-only change is validated separately
against dracon-sync. No edits to that other active session were made.
