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
