# Daemon FD Limit Fix (2026-07-03 16:02 BST)

## Symptom

Starting at ~15:50 BST 2026-07-03, `dracon-sync` daemon logs started
showing `Resource temporarily unavailable (os error 11)` (EAGAIN) on
multiple repos:

```
Jul 03 16:00:30 nixos dracon-sync[331780]: ⚠️ sync failed for /home/dracon/Dev/avid: Resource temporarily unavailable (os error 11)
Jul 03 16:00:31 nixos dracon-sync[331780]: ⚠️ sync failed for /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls: failed to spawn git add in ...
Jul 03 16:00:31 nixos dracon-sync[331780]: ⚠️ sync failed for /home/dracon/Dev/dracon-platform/web/games/wip/darklord: Resource temporarily unavailable (os error 11)
```

The daemon's `Max open files` soft limit was **1024** (systemd default).
Each `git` operation the daemon spawns (commit, push, fetch, etc.)
consumes 3-4 file descriptors (pipes for stdout/stderr + .git/
internals). With 26 watched repos and active commits on multiple
submods at once, the daemon was exhausting the 1024 FD limit.

## Cascading failure

When the daemon hit EAGAIN, it would:
1. Fail to spawn `git` for the current repo
2. Mark the in-flight entry as failed
3. The retry loop kept trying → 12+ "stuck in_flight" entries
4. Daemon's view of the world went into "trailing-drain" recovery mode

In parallel, the **hegemon github push loop** was creating
6+ stuck `git push --no-verify github HEAD` processes (each one
hanging on github's pack-size limit check). These processes held
FDs and prevented the daemon from cleanly restarting, requiring
a SIGKILL of the daemon's children + the daemon itself.

## Fix

Added `LimitNOFILE=16384` to
`/home/dracon/.config/systemd/user/dracon-sync.service`:

```ini
# Raise fd limit: 1024 (systemd default) is too low for git operations on 26 repos
# Daemon opens .git/, .git/index.lock, pipes for git stdout/stderr, etc. per repo
# 16384 matches systemd's DefaultLimitNOFILE in user.conf and supports the 26 watched
# repos with headroom. EAGAIN (os error 11) = "Resource temporarily unavailable" was
# the visible symptom under the old 1024 limit.
LimitNOFILE=16384
```

Then:
```bash
systemctl --user daemon-reload
pkill -9 -P 331780  # kill stuck git push children
systemctl --user restart dracon-sync.service
```

## Verification

Post-restart state of all 10 submods (16:08 BST):

| Repo | IN-SYNC |
|------|---------|
| polis | 4/4 ✓ |
| darklord | 4/4 ✓ |
| neonbreak | 4/4 ✓ |
| hellhunter | 4/4 ✓ |
| hegemon | 3/4 (github EMPTY structural, 2GB pack limit) |
| one-mil-girls | 4/4 ✓ |
| capture-anime-girls | 4/4 ✓ |
| endless-td | 4/4 ✓ |
| deathrun | 4/4 ✓ |
| junk-runner | 4/4 ✓ |

New daemon PID 475499 reports `Max open files = 16384` (was 1024).
Daemon is actively committing and pushing new work.

## Why 16384

- 1024 (systemd default) → too low
- 524288 (system hard limit) → wasteful, no real benefit
- 16384 → matches systemd's `DefaultLimitNOFILE` in `user.conf`,
  gives ~16x headroom over the previous value, well within the
  524288 hard limit
- The 26-repo workload uses ~50-100 FDs at peak (observed via
  `ls /proc/$PID/fd`); 16384 is generous without being silly

## Long-term follow-up

hegemon's github push retry will still hang the daemon's
hegemon-repo in_flight entry for ~54 minutes per retry
(`Stuck Push Retry: retrying after 3241s`). When the daemon
gets around to a real solution (see
`docs/design/hegemon-binary-content-strategy-2026-07-03.md`
deferred work), the github push will either succeed
(no longer over 2GB) or be removed from the daemon's
remote list.

Until then, the daemon's behavior is correct: it pushes
to working remotes (origin/codeberg/gitlab) on every cycle
and treats the github remote as "stuck" with a long retry
backoff. Local→codeberg→gitlab→origin stays in sync.
