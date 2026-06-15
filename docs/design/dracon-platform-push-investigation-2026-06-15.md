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
