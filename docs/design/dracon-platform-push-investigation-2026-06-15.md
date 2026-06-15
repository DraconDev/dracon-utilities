# `dracon-platform` push investigation (2026-06-15)

## Context

The operator looked at the live `dracon-sync repos` report
and said: "ok we seem to look better but the platform still
looks sus". 13 of 14 watched repos showed `✅ OK`, but
`dracon-platform` showed `⚠ WARN` with `🟣 pushing 15m (2
ahead)`.

## Investigation

### Live state at start

```
2  ⚠ WARN  dracon-platform  main  20 MOD  0 STG  44 UT
       2 AHEAD  0 BEHIND  PUSH=PENDING
       LAST: 2c02792a2a5…  fix(site+home): ship follow-ups
                              C, D, E from goal 1765192b audit
       PUSHED: 1h 23m
       ACTIVITY: 🟣 pushing 15m (2 ahead)
       STATE: 🟣 pushing
       HINT: daemon will push after changes settle
```

### Local main vs all 4 remotes

```
Local main:  2c02792a2a51
origin:      f62661cbf293   (2 behind)
github:      f62661cbf293   (2 behind)
gitlab:      f62661cbf293   (2 behind)
codeberg:    f62661cbf293   (2 behind)
```

Local was 2 commits ahead of all 4 remotes:
- `f62661cbf → 2f549e92c`: `docs(audit): full alignment audit
  confirms omega-only state across all surfaces`
- `2f549e92c → 2c02792a2`: `fix(site+home): ship follow-ups
  C, D, E from goal 1765192b audit`

### Daemon logs

`journalctl --user -u dracon-sync.service` showed:

```
Jun 15 16:21:30  📝 committed 7 file(s) in dracon-platform
Jun 15 16:26:11  📝 committed 2 file(s) in dracon-platform
Jun 15 16:32:05  📝 committed 3 file(s) in dracon-platform
Jun 15 16:37:12  📝 committed 1 file(s) in dracon-platform
```

The daemon repeatedly committed to `dracon-platform` across
4 restarts (PIDs 3065752, 3342381, 3429547, 3755146), but
**never logged a `🔁 synced` or push success for
`dracon-platform`**. Every other repo that committed
showed a `🔁 synced` line shortly after; `dracon-platform`
just stopped at the commit.

### Stuck-push ledger

`dracon-platform` was NOT in `~/.local/state/dracon/
dracon-sync-stuck-push-repos.json`. The ledger had 16
entries:
- 15 `/tmp/.tmp*/test-repo` entries (from prior unit tests,
  not real repos)
- 1 `/home/dracon/Dev/kiki-sassy-desktop-announcer` entry
  (historical, from a prior goal's transient push error)

None were the cause of the WARN.

### In-flight ledger

`~/.local/state/dracon/dracon-sync-in-flight.json`
contained 6 repos including `dracon-platform`. The
in-flight entry is informational; it doesn't block the
push.

### Manual push

```
git push github main --verbose   # → f62661cbf..2c02792a2 OK
git push gitlab main --verbose   # → f62661cbf..2c02792a2 OK
git push codeberg main --verbose # → f62661cbf..2c02792a2 OK
```

All 3 SSH remotes accepted the push. No hooks, no auth
issues, no errors. The 2 ahead commits are now on
`github`, `gitlab`, and `codeberg`.

### Why the daemon didn't push

The daemon was busy cycling through `Junk-Runner-bevy`
every ~45 seconds. `Junk-Runner-bevy` is 3011+ commits
ahead and the daemon scales its `push_op_timeout_secs`
to 360s for that repo. The daemon appears to serialize
repo processing per cycle, so a long `Junk-Runner-bevy`
push window blocks the daemon from progressing to push
other repos. By the time the daemon returns to
`dracon-platform`, the operator's local working tree
has accumulated more changes, and the cycle repeats.

In other words: `dracon-platform` was commit-busy (the
daemon kept staging and committing 1-7 files at a time
from the working tree) but the push step was being
deferred to a future cycle that never arrived in time
to be visible in the report.

This is **not a daemon bug** — it's a starvation pattern
where a single high-traffic repo (`Junk-Runner-bevy`)
dominates the daemon's cycle. The fix (if the operator
wants it) is to give `Junk-Runner-bevy` its own daemon
process, or to throttle `Junk-Runner-bevy` polling
during peak hours.

## Resolution

The push is now resolved. All 3 SSH remotes
(`github`, `gitlab`, `codeberg`) and the `origin` HTTPS
remote are aligned at `2c02792a2a51` with 0 ahead/behind.

```
Local main:  2c02792a2a51
origin:      2c02792a2a51   (aligned)
github:      2c02792a2a51   (aligned)
gitlab:      2c02792a2a51   (aligned)
codeberg:    2c02792a2a51   (aligned)
```

The remaining `⚠ WARN` in the live report is purely the
**dirty working tree** in `dracon-platform`:

- 18 modified tracked files (real work, intentional)
- 0 staged
- 45 untracked files (mostly screenshots, audit docs,
  test specs, and `web/.pi-tmp/*` scratch dirs)

The `🟠 dirty` state with the hint "daemon handles after
changes settle; run sync-now --warns to force now" is the
daemon correctly reporting operator in-progress work. The
operator has two options:

1. **Continue working.** The WARN is informational. The
   daemon will continue staging and committing the
   working tree changes as the operator saves files.
2. **Force commit the dirty work now.** Run
   `dracon-sync sync-now /home/dracon/Dev/dracon-platform
   --warns` to commit the 18 MOD + 45 UT immediately and
   push them. This will create a new commit on `main`
   with the operator's in-progress work.

**This investigation did NOT auto-commit the operator's
work.** The constraint "Do not delete, untrack, or ignore
user notes, screenshots, audit evidence, local task
state" applies. The operator's 18 MOD + 45 UT is their
own intentional work and is preserved untouched.

## Verification

```
$ git ls-remote origin    refs/heads/main  → 2c02792a2a51
$ git ls-remote github    refs/heads/main  → 2c02792a2a51
$ git ls-remote gitlab    refs/heads/main  → 2c02792a2a51
$ git ls-remote codeberg  refs/heads/main  → 2c02792a2a51
$ git rev-parse main                    → 2c02792a2a51

$ git rev-list --left-right --count github/main...main
0	0
```

All 4 remotes aligned with local. 0 force-pushes
occurred. 0 history rewrites occurred. The operator's
working tree is preserved.

## Cleanup recommendations (informational, not done)

1. **Stuck-push ledger junk**: 15 `/tmp/.tmp*/test-repo`
   entries pollute the ledger. The unit tests should
   clean up their temp repos (or the daemon should
   filter ledger entries to only known watch-root repos
   on read).
2. **kiki-sassy stuck entry**: historical, from a prior
   goal's transient push error. Can be cleared with
   `dracon-sync repair stuck-unstuck
   /home/dracon/Dev/kiki-sassy-desktop-announcer`.
3. **Junk-Runner-bevy starvation**: the daemon cycles
   through Junk-Runner-bevy every 45s and scales its
   push timeout to 360s, which can starve other repos
   of push attention during peak cycles. Could be
   mitigated by giving Junk-Runner-bevy its own daemon
   instance or by polling it less frequently.

These are tech-debt items, not blockers. The push issue
is resolved. The operator can clean up the ledger and
tune the daemon later.

## RESOLUTION (FINAL — 2026-06-15)

The investigation surfaced a real daemon bug that
prevented `dracon-platform` (and any other slow-to-push
repo) from being processed.

### The trailing-drain bug

The daemon's `in_flight: HashSet<PathBuf>` is supposed
to prevent re-dispatching a repo while its `sync_repo`
task is running. The design has two phases that drain
the in_flight set:

1. **Apply phase**: drains tasks that completed within
   `apply_deadline_secs = pulse_interval_secs * 2`
   (default 2s).
2. **Trailing drain**: drains leftover tasks with the
   same 2s deadline.

**The bug**: on trailing-drain timeout, the unfinished
tasks were dropped from `in_flight_tasks` (which goes
out of scope) but their entries in `in_flight` were
NEVER cleared. The result: a slow sync task (e.g. a
60s push on `dracon-platform`) would stay in
`in_flight` forever, causing the COLLECT phase of every
subsequent cycle to skip the repo. The repo would never
be processed again until the daemon restarted.

### The fix

The fix is in `daemon.rs`'s trailing-drain code:

```rust
// BUGFIX (2026-06-15): track dispatched repos in a
// local set, and on trailing-drain completion or
// timeout, clear any `in_flight` entries that were
// not drained.
let mut dispatched_this_cycle: HashSet<PathBuf> = in_flight.clone();
loop {
    // ... drain tasks as before ...
    if let Ok((repo, ...)) = joined {
        in_flight.remove(&repo);
        dispatched_this_cycle.remove(&repo);
    }
}
// BUGFIX: clear remaining dispatched entries
if !dispatched_this_cycle.is_empty() {
    eprintln!("🔄 trailing-drain: clearing {} stuck in_flight entries: {:?}",
              dispatched_this_cycle.len(), dispatched_this_cycle);
    for repo in &dispatched_this_cycle {
        in_flight.remove(repo);
    }
}
```

This breaks the no-redispatch invariant for slow tasks,
but the invariant was never achievable for slow tasks
anyway (they always timed out). The trade-off is:
re-dispatching a slow task is recoverable (the new task
will fail with a lock conflict or remote rejection),
while permanent skip is not.

### Regression test

`test_trailing_drain_clears_stuck_in_flight` in
`daemon.rs` simulates the data structure: 3 repos are
inserted into `in_flight`, 1 is drained, and 2 timeout.
The fix clears the remaining 2 from `in_flight`. After
the fix, `in_flight` is empty.

### Verification

After the fix was deployed and the daemon restarted:

```
Jun 15 18:40:27 dracon-sync[1737439]: 🔄 trailing-drain: clearing 2 stuck in_flight entries: {Junk-Runner-bevy, dracon-platform}
Jun 15 18:41:23 dracon-sync[1737439]: 📝 committed 25 file(s) in /home/dracon/Dev/dracon-platform
Jun 15 18:42:25 dracon-sync[1737439]: 🔄 trailing-drain: clearing 1 stuck in_flight entries: {Junk-Runner-bevy}
Jun 15 18:43:00 dracon-sync[1737439]: 📝 committed 1 file(s) in /home/dracon/Dev/Junk-Runner-bevy
```

`dracon-platform` was committed (25 files: 18 modified
tracked + 7 untracked that matched auto-commit
patterns) and pushed to all 3 remotes at
`391c44aec955`. The 39 remaining untracked files are
scratch directories (`web/.pi-tmp/`, screenshots, audit
dirs) that don't match the auto-commit patterns.

Final live state:

```
📦 14 repos  ✅ OK 14  ⚠️  WARN 0  ❌ CONCERN 0  ⛔ init/status failed: 0

│ 2  ┆ ✅ OK  ┆ dracon-platform  ┆ main  ┆ 0  ┆ 0  ┆ 39  ┆ 0  ┆ 0  ┆ OK  ┆ 391c44aec95… 25 file(s)  ┆ 2m  ┆ 🟢 synced 2m  ┆ DraconDev  ┆ ⚪ untracked-only  ┆ healthy │
```

All 14 repos are `✅ OK` and `healthy`. The goal is
fully resolved.
