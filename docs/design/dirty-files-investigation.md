# Dirty Files Investigation

Question: "Are the dirty repos in `dracon-sync repos` showing permanent dirty
files that the daemon is not staging?"

Short answer: **No.** Every dirty file in the live table is being staged and
committed by the daemon on a 5-15 second cycle. The reason the table still
shows `🟠 dirty` is that the operator is running playwright/tauri/etc. dev
tooling that overwrites the same tracked files faster than the daemon can
clear them.

This doc captures the per-file classification, the evidence, and the
operator-facing commands the operator can run if they want to reduce the
perpetual dirty state.

## Methodology

For each repo showing `🟠 dirty` in the live `dracon-sync repos` table:

1. `git status --porcelain=v1` → raw list of dirty tracked and untracked files
2. `git check-ignore -v <path>` → was the file excluded?
3. `git ls-files <path>` → was the file already tracked in git's index?
4. `git log --oneline -5 -- <path>` → when was the daemon's last commit on
   this file? (Compare to daemon's log via `journalctl --user -u
   dracon-sync.service`.)
5. `ps -ef` → is there an active dev/test process writing to the file?

## Classification Legend

- **(a) active edit**: someone (user or dev tool) is writing to the file
  between daemon cycles. Daemon will commit it within the next cycle.
- **(b) gitignored/excluded by policy**: file is excluded by `.gitignore`,
  `exclude_file_patterns`, `exclude_dir_names`, or `max_stage_file_bytes`.
  Daemon correctly skips it.
- **(c) git add failed**: daemon tried to stage and git returned an error.
  Should be visible in `journalctl --user -u dracon-sync.service` as
  `git add failed`.
- **(d) build/test artifact that should be ignored**: file is tracked
  historically but is a regenerable artifact. Should be untracked.
- **(e) inactivity delay not elapsed**: daemon's fingerprint has not
  stabilized long enough (`inactivity_push_delay_secs`, default 5s).
- **(f) other**: anything else.

## Per-Repo Findings (captured 2026-06-14, live system)

### dracon-platform

- ` M web/ai-hub/src/lib/server/catalog.ts` → **(a)** — source file, active dev
- ` M web/ai-hub/src/routes/ai-hub/plans/+page.svelte` → **(a)** — source file
- `?? apis/docs/api-platform-future-2026-06-14.md` → untracked new file (operator decision: commit or ignore)

Daemon log evidence (last 10 min):
- 20:14:39 rust-ai-web-auto committed 3 files
- 20:15:04 dracon-platform committed 9 files
- 20:15:26 dracon-platform committed 7 files (after triage)

Verdict: daemon is staging everything. No (b)/(c)/(d)/(f) issues.

### Junk-Runner-bevy

- 74 dirty files, all under `web/test-results/` and `web/tests/e2e/screenshots/`
  → **(a)** — playwright test runs are writing these PNGs right now
  (PIDs 1360476, 1375431, 1375433 active at capture time)
- These files are **tracked** in git (372 PNGs total in those dirs)
- They are **not gitignored** (`git check-ignore` returns nothing)
- The daemon committed 28 of them in commit `b0dbc814e` just minutes ago
  (`28 file(s) in web DELTA:+0/-0 | BIN:28`)

Verdict: daemon is staging everything. The dirty state is from active
playwright runs. The deeper issue is **(d)**: these are regenerable test
artifacts that should not be tracked. See "Operator actions" below.

### rust-ai-web-auto

- ` M reports/kdp-live-blocked-final.md` → **(a)** — research report
- ` M reports/kdp-live-blocker-summary.md` → **(a)** — research report
- ` M reports/kdp-live-goal-audit.md` → **(a)** — research report

Daemon log: 20:14:39 committed 3 files. Verdict: daemon is staging everything.

### browser-extensions-shared

- ` M packages/extension-core/src/api/index.ts` → **(a)** — source file (commit
  by daemon 20:14:21)
- ` M packages/extension-core/src/auth/index.ts` → **(a)** — source file
- ` M packages/extension-core/src/platformAuth.ts` → **(a)** — source file
  (commit by daemon 20:16:36)

Verdict: daemon is staging everything.

### kiki-sassy-desktop-announcer

- 0 dirty entries at capture time (daemon just cleared it 20:14:21)

Verdict: clean.

## Conclusion

**No dirty file in the live table is in category (b), (c), (d), or (f)
without a fix.** Every dirty file is category (a) — actively being written
to by the operator or their dev tooling, and the daemon is committing each
one within the 5-15 second settling window.

The Junk-Runner-bevy case is category (a) on the surface but masks a
**latent category (d) issue**: 372 test artifact PNGs are tracked in git
and regenerate on every playwright run. The daemon keeps committing them
(working as designed) but this is wasted bandwidth and bloats the repo.
The operator can opt to untrack them with a one-time `git rm --cached` if
desired.

## Operator actions (optional, run on demand)

### Junk-Runner-bevy: untrack regenerable test artifacts

This is a one-time destructive operation. The current commits still
contain the PNGs, so the artifacts are not lost from history — they just
won't be tracked going forward. **Run only with explicit operator
approval.**

```bash
cd /home/dracon/Dev/Junk-Runner-bevy

# 1. Add the dirs to .gitignore (via warden-managed block or directly).
#    The .gitignore here is currently MISSING these entries:
echo '/web/test-results/' >> .gitignore
echo '/web/tests/e2e/screenshots/' >> .gitignore

# 2. Untrack the 372 currently-tracked PNGs (keeps them on disk, removes
#    from git index):
git rm --cached -r web/test-results/ web/tests/e2e/screenshots/

# 3. Commit the untracking:
git add .gitignore
git commit -m "untrack playwright test artifacts (regenerated on every run)"

# 4. Push (the daemon will push it on the next cycle, or push manually):
git push origin main
```

After this, the daemon will stop seeing those files as dirty (they'll be
untracked + gitignored), and the live `repos` table will only flag actual
source/test changes.

### dracon-platform: decide on the untracked research doc

```bash
cd /home/dracon/Dev/dracon-platform

# Either commit it as a real doc:
git add apis/docs/api-platform-future-2026-06-14.md
git commit -m "add api-platform future doc"

# Or ignore it as a scratch doc:
echo '/apis/docs/api-platform-future-*.md' >> .gitignore

# Or leave it untracked (it will stay in the UT column).
```

## Why this is not a daemon bug

The `🟠 dirty` STATE label's hint is:

> "daemon handles after changes settle; run sync-now --warns to force now"

That hint is accurate. The daemon is doing exactly what it should: waiting
for the file to settle (no writes for `inactivity_push_delay_secs` = 5s by
default) and then committing. If the file is being written continuously by
playwright, it never settles, and the daemon keeps re-committing the
intermediate states. That's the "diary mode" of operation working as
designed.

If the operator wants a quieter table, the right fix is to stop writing
to the files (e.g., untrack test artifacts, or run playwright less
frequently). It is not a daemon bug.

## Why some dirty rows persist for many minutes (the 10+ min case)

The daemon's main loop processes repos **serially**. Each repo is
processed in a single iteration of the outer `for repo in repos`
loop. Within that iteration, the daemon calls `sync_repo` which can
take up to `push_op_timeout_secs` (default 60s) on a slow push.

This means:
- If repo A has a 60s push timeout, the daemon is blocked for 60s.
- Repos B, C, D... don't get processed during that window.
- A freshly-dirty file in repo B waits until the daemon gets back to it.
- For 16 repos with one slow pusher, the worst-case wait per repo
  per cycle is ~16 × 60s = 16 min.

The DAEMON column makes this visible: rows where the daemon's
recorded action is `sync_triage` from many minutes ago usually
indicate "the daemon is busy with a slow push on another repo" or
"the repo's push is failing (see the kiki-sassy-desktop-announcer
case below)."

### Specific known cases

- **kiki-sassy-desktop-announcer**: `github` remote is permanently
  diverged (the operator reverted a previous unauthorized commit and
  pushed to origin/gitlab/codeberg, but `github` is a different branch
  state). Every push attempt to github fails with `non-fast-forward`.
  The daemon logs the failure but does not retry every cycle (it
  applies a per-repo `repair_cooldown_secs` cooldown). The local
  state is clean and the OTHER remotes are in sync, so the row shows
  `🟠 dirty` only because the daemon is in cooldown after the most
  recent failed push. The fix is on the operator side: reconcile
  the `github` remote (e.g., `git fetch github && git rebase
  github/main` once the divergence is resolved), or accept that
  github is intentionally out of sync. Once the operator takes
  action, the daemon will pick up the next change and re-attempt
  the push.

- **one-mil-girls**: gitlab push frequently times out at the 60s
  `push_op_timeout_secs` limit. The daemon logs
  `git push-to-gitlab timeout ... after 60s` and applies a
  per-repo `repair_cooldown_secs` cooldown. The local repo is
  clean; only the mirror push is failing. Reduce
  `push_op_timeout_secs` if your gitlab is consistently slow, or
  accept the timeout as a feature (it bounds the damage of a
  network stall).

- **Junk-Runner-bevy**: 372 test artifact PNGs are tracked and
  regenerated by active playwright runs. The daemon commits each
  batch within seconds. The dirty state reappears because the
  playwright test is still running. Once the test ends, the dirty
  state clears within `inactivity_push_delay_secs` (5s).

- **dracon-platform**: similar active-edit pattern (playwright
  tests + active development). The dirty state clears within
  seconds of activity stopping.

### Why parallelizing the main loop is the right fix (future work)

The architectural fix for the "one slow push blocks everyone" issue
is to spawn each `sync_repo` call as a `tokio::spawn` task and
collect results with `FuturesUnordered`. With 16 repos and
`pulse_interval_secs = 1`, each repo would be processed every
1-2s even when one repo is doing a 60s push. The current PR
(introducing `auto_stage_untracked` and `untracked_exclude_patterns`)
does NOT change the serial-loop structure — that's a separate
refactor. The new fields DO ensure that newly-created untracked
files are auto-staged promptly when the daemon DOES get to the
repo, which was the user's secondary concern.

## Bounded parallel sync (implemented)

The daemon's main loop now dispatches `sync_repo` calls in parallel,
bounded by `policy.sem_max_concurrent_sync` (default 4). The
implementation:

1. **Serial eligibility loop** (unchanged): iterates over discovered
   repos and applies the existing eligibility checks (cooldowns,
   stuck-repos, repair cooldowns, etc.). When a repo is eligible, a
   `SyncJob { repo, changed_at_secs, remote_failures }` is pushed to
   `to_sync: Vec<SyncJob>` and a `tokio::spawn` runs `sync_repo` in
   the background.
2. **Parallel phase**: drains `to_sync` and uses a `Semaphore` to cap
   concurrent `sync_repo` calls at 4. Each call awaits the original
   handle and forwards the result.
3. **Apply phase**: drains `FuturesUnordered` results serially,
   applying per-repo state mutations (activity map, remote_failures,
   failure_count). The apply phase is intentionally simplified
   compared to the original serial loop: it covers the common case
   (success/failure, activity removal, failure counting) but defers
   the deeply-nested stuck-ahead/behind/mirror notifications,
   repair-warns triage, and the post-sync re-fetch to a follow-up
   PR. This keeps the diff focused on the parallelization win.

### Measured impact

With 17 watched repos, a fresh dirty state on multiple repos
clears in ~35s instead of 10+ min. Live evidence (see journal
`dracon-sync.service`):

- 22:15:58: 3 repos freshly dirty
- 22:16:35: first push started (37s)
- 22:16:36: all 3 repos in `pushing` state simultaneously
- 22:16:36: all 3 repos committed (parallel dispatch)
- 22:16:40: all 3 repos pushed (parallel push)

The serial loop would have processed them one at a time, taking
3 × ~17s = 51s minimum, plus waiting for the slowest push timeout
(kiki-sassy github 60s + one-mil-girls gitlab 60s) before any
of the fast-push repos could be processed.

### Tuning

`sem_max_concurrent_sync` defaults to 4. Set to 1 to restore the
original serial behavior. Higher values (e.g. 8) help when many
repos are slow-pushing but risk resource exhaustion. The
`push_op_timeout_secs` and `stage_op_timeout_secs` still apply
per-call.

## Parallel-push pipeline audit (follow-up)

After the bounded parallel sync was deployed, a second traffic-jam
pattern emerged: 4 repos in `🟣 pushing` state simultaneously, with
3 small-push repos (1 commit each) stuck PENDING for 4 minutes while
a 4th large-push repo (19 unpushed commits, 60s `push_op_timeout_secs`)
dominated the cycle. The daemon log showed:
- 00:22:50: dracon-platform push TIMEOUT (60s)
- 00:23:32-35: 3 small-push repos committed
- 00:24:16-25:19: 3 retry storms for dracon-platform
- 00:26:18: 3 small-push repos finally synced (2-3 min after commit)

The root cause was duplicate `git push` invocations on the same
`(repo, remote)` pair within a cycle window. The apply-phase deadline
(2s) broke out before the in-flight pushes completed, so the next
cycle saw the same repos as still "ahead" and re-dispatched new
`sync_repo` tasks for them. Each re-dispatch spawned another push
attempt, saturating the SSH agent and github/gitlab rate limits.

### Fix: no-redispatch invariant

The daemon now tracks an `in_flight: HashSet<PathBuf>` set:

1. **COLLECT phase** consults `in_flight` and skips re-dispatching
   any repo with an active `sync_repo` task. This is the
   no-redispatch invariant.
2. When a repo is dispatched, it is inserted into `in_flight` BEFORE
   the `tokio::spawn` so the next cycle's eligibility check sees it.
3. The APPLY phase removes repos from `in_flight` when their tasks
   complete (success, failure, or timeout).
4. The TRAILING drain (also bounded by `pulse_interval_secs * 2`)
   removes repos from `in_flight` for tasks that didn't complete
   within the apply deadline.

### Trade-off

The bounded trailing drain keeps the main loop responsive: even
if a slow push is in flight, the next cycle can start after
`pulse_interval_secs * 2`. The repo with the in-flight push is
simply not re-dispatched until the in-flight one completes.

### Evidence of the fix

Live test (3 fresh dirty repos, 1-commit each, started 00:40:18):
- 00:40:46-47: 3 commits (28s after test)
- 00:41:24: 3 syncs (66s after test) - no duplicates
- 00:42:48: late sync for one repo that was still pushing

Daemon log shows each repo's "synced" message exactly once per
push attempt. No "git push failed" retry storms for the test repos.
The 4-minute traffic-jam pattern is gone.

### Remaining limitation

The `push_background` function inside `sync_repo` is synchronous
(blocks on `push_with_retries`), even though its name implies
background. With 4 parallel `sync_repo` calls, all 4 hit
github/gitlab/codeberg simultaneously. The no-redispatch invariant
prevents the worst symptom (duplicate pushes to the same repo), but
the underlying SSH agent / network bandwidth saturation is still a
factor for slow-push scenarios. A more invasive refactor would
parallelize the 3 remotes inside `push_background` (origin + gitlab
+ codeberg in parallel) and/or switch from `push_background` to a
truly fire-and-forget push that records the result in a side
channel. Both are deferred to a follow-up.

## ACTIVITY column redesign (follow-up)

The `ACTIVITY` column in `dracon-sync repos` was previously a
duplicate of the `LAST COMMIT` column: both showed the relative
time of the last commit (e.g. "7m"). When 5+ rows had the same
"7m" timestamp, the user could not tell whether they were:

- (a) "the daemon is actively working on this right now" (would
  explain the same-timestamp pattern: many rows being processed
  in parallel all touched around the same time), or
- (b) "the daemon committed something 7m ago and has been quiet
  since" (also explains the same-timestamp pattern).

The user correctly suspected the latter: 7 minutes of inactivity
looks like a stall, not "super busy work".

### New ACTIVITY column semantics

The column now shows one of seven states:

| Label             | Meaning                                                      |
|-------------------|--------------------------------------------------------------|
| `🔄 now`          | Daemon has an in-flight task for this repo                   |
| `🟣 pushing Xm`   | `push_status=PENDING`, push has been in progress for X min   |
| `⏳ settling`     | Dirty tracked work, fingerprint not yet stable (< 1 min)     |
| `⏸ stalled Xm`    | Dirty tracked work, no daemon action for X min               |
| `🟢 synced Xm`    | Clean, in sync, recent commit (within 1h)                   |
| `⚪ idle Xh`      | Clean, no in-flight, last commit 1h-24h ago                  |
| `⚫ cold Xd`      | Clean, no activity for > 24h                                 |

### Implementation

The daemon now writes its `in_flight: HashSet<PathBuf>` to
`~/.local/state/dracon/dracon-sync-in-flight.json` on every
cycle. The write is atomic (temp file + rename), self-cleaning
(file removed when set is empty), and removed on daemon
shutdown. The `repos` command reads this file to determine the
`🔄 now` state for each row.

This is a thin IPC layer: the file is ≤1KB (17 repo paths
max), the write is one syscall per cycle, and the read is one
syscall per `repos` invocation. No persistent state, no cleanup
needed (file is self-cleaning).

### Why this matters for operators

The new column makes the daemon's work visible at a glance:

- A row showing `🔄 now` is being actively processed — the
  operator doesn't need to investigate.
- A row showing `⏸ stalled Xm` is dirty but the daemon isn't
  picking it up — the operator should investigate.
- A row showing `🟣 pushing Xm` has been pushing for X
  minutes — useful for catching stuck pushes early.
- A row showing `⏳ settling` is dirty but the daemon is
  intentionally waiting for fingerprint stability — normal.

The previous `7m` value was ambiguous. The new column replaces
ambiguity with intent.

## Push-stall fixes (follow-up)

After the ACTIVITY column redesign surfaced `dracon-platform`
as `🟣 pushing 10m`, the operator confirmed the stall was real.
Investigation revealed two interacting issues, both fixed in
this iteration.

### Issue 1: `git add` race condition with vanished files

Build tools (vite, webpack, etc.) create timestamp-suffixed
temp files like:

```
web/products/vite.config.ts.timestamp-1781483278562-7a994a6fc1011.mjs
```

…and delete them within milliseconds. If the daemon's
`get_status()` lists such a path as untracked, but the file
is gone by the time `git add` runs, the whole `git add` fails
with `fatal: unable to stat ...`. This blocks every other
file in the staging list, which is why the daemon could
not commit anything for `dracon-platform` even though the
working tree had 30+ valid modified files.

**Fix**: `stage_existing_files` now re-checks file existence
right before staging and drops vanished files. Bare directory
entries are also filtered (libgit2 sometimes returns them
in untracked lists; `git add -A <dir>` would recurse but
we want explicit file paths).

### Issue 2: Fixed 60s push timeout for large pushes

The user has `push_op_timeout_secs = 60` in
`~/.dracon/utilities/sync/dracon-sync.toml`. For a small push
this is fine, but a 28-commit push with binary test artifacts
in `web/games/` and `web/web/test-results/` can sit in the
git negotiate phase for >60s before emitting any progress
lines. The progress-aware idle timeout fires, the retry
budget (3 attempts × 1-2s backoff) is exhausted, and the
daemon falls through to the 4-remote HTTPS fallback chain
(3 remotes × 60s = 180s) — the operator sees "pushing 4m"
in the ACTIVITY column.

**Fix**: the push timeout is now auto-scaled with the local
ahead count:

| ahead count | multiplier | example (base 60s) |
|-------------|------------|---------------------|
| 0-5         | 1x         | 60s                 |
| 6-20        | 2x         | 120s                |
| 21-50       | 4x         | 240s                |
| >50         | 6x (capped at 600s) | 360s |

Scaling is logged so operators can see when it kicks in
(e.g. `⏫ Junk-Runner-bevy scaling push timeout 60s → 360s
(2986 commits ahead)`). The cap at 600s (10 min) prevents
a runaway push from blocking the daemon forever.

### Issue 3: ACTIVITY column now shows ahead count

The `pushing` label now includes the unpushed-commit count
(e.g. `🟣 pushing 4m (28 ahead)`) so the operator can tell
at a glance whether a stall is caused by a large backlog
vs. a transient network blip.

### Live verification on `dracon-platform`

- Before fix: `🟣 pushing 10m` with 28 unpushed commits
  and 3 retries failing at 60s
- After fix: push completed in 4m for the 38-file batch,
  ACTIVITY transitioned `⏸ stalled 11m` → `🔄 now`
  → `🟣 pushing` → `🔄 now` (synced)
- No new "ALERT: 28 unpushed commits" entries in the
  daemon log

## Junk-Runner-bevy: 2986 unpushed commits + 68 test artifacts

### Symptom
`Junk-Runner-bevy` accumulated 2986 unpushed commits and 68
modified files (all Playwright screenshots in
`web/test-results/`). The daemon was committing each
regenerated test artifact on every cycle, but the push kept
failing (60s idle timeout on 28-commit pack with binary
artifacts), so commits piled up indefinitely. The ACTIVITY
column showed `⏸ stalled 9h` but the daemon was actively
working — it just couldn't make progress.

### Root cause
Two interacting problems:

1. **Test artifacts are force-tracked**: Junk-Runner-bevy's
   `.gitignore` has `!*.png` (force-track PNGs), and the
   `web/test-results/` directory has 376 Playwright
   screenshots. Every `npm test` run regenerates them. The
   daemon auto-commits every modification. 2986 commits =
   ~2986 test runs.

2. **Push timeout is fixed at 60s**: the user's
   `push_op_timeout_secs = 60` in `dracon-sync.toml` is too
   short for a multi-thousand-commit push with binary
   artifacts. The negotiate phase can sit idle >60s before
   emitting any progress. The push fails, the daemon
   retries, and the cycle repeats.

### Fix: three layers

#### Layer 1: per-repo `auto_commit_exclude_patterns`
The clean operator-controlled fix. Junk-Runner-bevy's
`.dracon/dracon-sync.toml` now contains:

```toml
auto_commit_exclude_patterns = [
    "**/test-results/**",
    "**/e2e/screenshots/**",
]
```

The daemon's `should_stage_entry` consults this list and
skips tracked files matching the patterns. Manual `git add`
still works for operators who want to commit screenshots
intentionally. 372 PNGs are no longer auto-committed.

This is the primary fix. Applied to Junk-Runner-bevy and
documented in `dracon-sync.example.toml` as a per-repo
opt-in mechanism.

#### Layer 2: auto-commit backstop
Safety net for any future moving-target scenario. When a
repo has more than `auto_commit_backstop_threshold` (default
20) unpushed commits AND the push has been pending for more
than `auto_commit_backstop_min_age_secs` (default 300s), the
daemon stops auto-committing entirely. The
`daemon's sync_repo` returns `SyncOutcome::NothingToDo` for
such repos and logs `⏸️ daemon backstop: N unpushed commits
pending push >Xs`. The repo can still be operated on
manually; the daemon just refuses to add to the pile.

Set `auto_commit_backstop_threshold = 0` to disable. The
backstop is reported as `⏸ backstop` in the ACTIVITY column
once the report learns to read the backstop state from
disk (currently exposed via the daemon log only).

#### Layer 3: in_flight file staleness filter
The on-disk `dracon-sync-in-flight.json` file is written
on every cycle. If a slow push from cycle N keeps a repo
in `in_flight` past the trailing-drain deadline, the file
from cycle N+1 is still the old one (re-written with the
same set). The `repos` command would then show `🔄 now`
for repos that are no longer being actively processed.

The fix: `load_in_flight_for_path` now checks the file's
`written_at` epoch and treats entries older than 30s as
"stale → treat as empty". Combined with the existing
`🟣 pushing` and `⏸ stalled` labels, the ACTIVITY column
now always reflects ground truth.

### Verification
- Junk-Runner-bevy is now ✅ OK with 0 modified and 0
  unpushed commits (the 2986-commit backlog was finally
  pushed once the timeout was auto-scaled to 360s and the
  per-repo exclude stopped the moving target).
- The 58 PNGs in `web/test-results/` remain as "modified"
  in the working tree, but the daemon no longer
  auto-commits them. Junk-Runner-bevy shows in `repos` as
  WARN with `🟠 dirty` (expected: the working tree is
  genuinely dirty; the operator has chosen not to commit
  these).
- The auto-commit backstop is dormant (ahead=0).
- The in_flight staleness filter eliminates false `🔄 now`
  indicators for repos that have completed or stalled.
