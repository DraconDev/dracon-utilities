# Daemon Staging Fix - Honest Assessment (2026-06-19)

## What Was Fixed

The fingerprint stability check no longer blocks untracked
file commits. The fingerprint now includes `untracked_files`
as a 6th component, so the daemon can distinguish between
"files are being edited" (wait for stability) and "new files
were added" (commit immediately).

## Measured Improvement

| Metric | Before Fix | After Fix |
|--------|-----------|-----------|
| Untracked count (dracon-platform) | 273+ | 0 (when agent paused) |
| Batch size | 1-55 files | 6-104 files |
| Settling time | Hours | Seconds |
| Trailing-drain frequency | Every cycle | Reduced but present |
| Daemon version | 0.1.5 | 0.1.12 |

## Limitations Discovered

### 1. Active agent throughput

When Playwright (or any active agent) is creating files
continuously, the daemon cannot keep up. The daemon's
commit cycle is ~25 seconds, and it can commit 6-10 files
per cycle. An active agent may create 10+ files per second.

**Root cause**: The daemon's commit cycle includes:
- Status check (~1s)
- Fingerprint comparison (~1s)
- Stage files (~1s for 10 files, scales linearly)
- Commit (~1s)
- Push (~5-10s for small commits)

The total cycle time is bounded by the slowest step.
For small commits, push dominates. For large commits,
stage dominates.

### 2. Trailing-drain still happens

The trailing-drain message appears when the repo stays
dirty after a commit. This is because:
1. Commit completes, repo marked clean
2. New files appear (from active agent)
3. Repo marked dirty again
4. Previous in_flight entry still in the set
5. Trailing-drain clears it
6. Cycle repeats

The fingerprint fix reduced the frequency but did not
eliminate it.

### 3. Pre-existing branch divergence

Multiple repos have a `master` branch on remotes that
diverged from `main` on 2026-05-05 (when `main` was
renamed from `master`). This is pre-existing and out
of scope. The active branch is `main` for all repos.

Repos with divergent `master` branches:
- ai-auto-writer: 1006 ahead on github/codeberg/gitlab
- browser-extensions-shared: 9431 ahead on codeberg/gitlab
- dracon-code: 2637 ahead on github, 7901 on codeberg/gitlab
- DraconDev: 552 ahead on github, 544 on gitlab
- dracon-platform: 12971 ahead on codeberg/gitlab
- pully-fully-pull-based-fleet-reconciler: 2786 ahead

### 4. Repo count claim was wrong

The original verification claimed "13 repos" but there
are 11 repos in /home/dracon/Dev/ (not 13). The .dracon
and live repos are at /home/dracon/.dracon and a separate
location, not in /home/dracon/Dev/.

## Systemic Solution Status

The fingerprint fix is a real systemic improvement:
- Untracked files no longer trigger the stability wait
- The daemon processes new files as soon as they're added
- Batch sizes are larger (104 files vs 1-55)
- The settling behavior is reduced from hours to seconds

The fix does NOT solve:
- Active agent throughput limitation
- Trailing-drain when repo stays dirty
- Pre-existing branch divergence

## Future Improvements (Out of Scope)

1. **Reduce cycle time**: The 25-second cycle time could be
   reduced to 5-10 seconds with daemon code changes
2. **Parallel processing**: The daemon processes one repo
   at a time per cycle. Parallel processing would help
3. **Batch staging**: Stage all untracked files in one
   command instead of one-at-a-time

## Verification

- dracon-platform untracked count: 0 (verified)
- All 11 repos in /home/dracon/Dev/ at 0/0 on main
- Daemon v0.1.12 running with the fingerprint fix
- 565 tests pass
- Code change committed and pushed
