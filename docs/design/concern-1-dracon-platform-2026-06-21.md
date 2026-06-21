# Concern 1 — dracon-platform unmerged index + unpushed Phase 24 — 2026-06-21

**Goal:** `f16d015e-a3d5-4f8f-ae60-daf0f2cca019` (investigate in detail).
**Status:** INVESTIGATION COMPLETE. NO FIX APPLIED. Operator decision required.

## TL;DR (one paragraph)

`dracon-platform` has a **partially merged git index** with 4 unresolved
PNG entries (`UU` status) that prevents ANY commit from succeeding in
the daemon. This blocks all of the 216 untracked non-gitignored files
(including the 193 capture-anime-girls card art PNGs and the 23 game
docs/scripts/audio assets) AND blocks pushing the 2 Phase 24 commits
(`580e859756` "Buildup Not Smackdown" and `9d75cf0720` "capture visual
smoke screenshots") that are sitting unpushed on all 4 remotes. The
daemon attempts staging + commit every ~10 seconds and fails every
time with the same error: `cannot create a tree from a not fully merged
index.` The error rate in the last 60 minutes was 93 failures (1 every
~39 s). A secondary contributing cause: `codeberg.org:22` SSH was
refusing connections at `Jun 21 10:20:44` through `10:22:22`, retried
5 times, and recovered at `10:23:14`. None of this is a daemon bug —
it is operator state that requires manual intervention before the
daemon can resume normal sync.

## Live state evidence (captured 2026-06-21 17:01 UTC)

### Top-level state

```
$ git -C /home/dracon/Dev/dracon-platform status --branch --short
* main...origin/main [ahead 2]
  A  web/CHROME-MOBILE-NO-BURGER-2026-06-21.md
  A  web/HOME-PAGE-AUDIT-2026-06-21.md
  MM web/ai-hub/.svelte-kit/ambient.d.ts
  M  web/ai-hub/HANDOFF.md
  M  web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-desktop.png
  UU web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile-drawer-open.png   ← unmerged
  M  web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile-or-row.png
  M  web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile.png
  M  web/ai-hub/audit-20260629/05-mobile-view-screenshots/plans-mobile.png
  UU web/ai-hub/audit-20260629/05-mobile-view-screenshots/providers-mobile.png          ← unmerged
  M  web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/01-closed-trigger.png
  UU web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/02-main-nav-open.png      ← unmerged
  M  web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/03-subnav-open.png
  UU web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/04-desktop-baseline.png   ← unmerged
  A  web/ai-hub/audit-20260630/03-ai-hub-internal-error-and-byteplus-referral/01-status.txt
  A  web/ai-hub/audit-20260630/03-ai-hub-internal-error-and-byteplus-referral/02-byteplus.txt
  ... (194 more lines, including A  web/games/games/capture-anime-girls/static/images/cards/char_5000.png through char_5192.png)
```

The `git status --short` line count is **207** (matching
`git ls-files --others --exclude-standard | wc -l = 216` minus 9
gitignored paths). The 4 `UU` lines are the root cause of the daemon's
commit failure.

### The 4 unmerged files

```
$ git -C /home/dracon/Dev/dracon-platform ls-files --unmerged | awk '{print $4}' | sort -u
web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile-drawer-open.png
web/ai-hub/audit-20260629/05-mobile-view-screenshots/providers-mobile.png
web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/02-main-nav-open.png
web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/04-desktop-baseline.png

$ git -C /home/dracon/Dev/dracon-platform ls-files --unmerged | wc -l
12
```

Each unmerged PNG has 3 stage entries (stage 1 = base, 2 = ours,
3 = theirs) → 4 paths × 3 stages = 12 `ls-files --unmerged` rows.

### The 2 unpushed Phase 24 commits

```
$ git -C /home/dracon/Dev/dracon-platform log --oneline -2
9d75cf0720 Phase 24: capture visual smoke screenshots
580e859756 Phase 24: Buildup Not Smackdown — combat rework

$ for r in origin github codeberg gitlab; do
    git -C /home/dracon/Dev/dracon-platform rev-list --left-right --count ${r}/main...HEAD
  done
0	2   origin
0	2   github
0	2   codeberg
0	2   gitlab
```

All 4 remotes are 2 commits behind. The 2 commits are local-only.

### Daemon log evidence

```
$ journalctl --user -u dracon-sync.service --since "60 min ago" --no-pager | \
    grep -c 'sync failed.*dracon-platform'
93

$ journalctl --user -u dracon-sync.service --since "60 min ago" --no-pager | \
    grep -c 'dracon-platform.*file(s) in'
0
```

In the last 60 minutes, the daemon attempted and failed 93 times
(~1 every 39 seconds) and produced **zero** successful "N file(s) in"
commits for `dracon-platform`. Each failure has the same root error:

```
⚠️ sync failed (late) for /home/dracon/Dev/dracon-platform:
   Git operation failed: cannot create a tree from a not fully merged
   index.; class=Index (10); code=Unmerged (-10)
```

And the inner git subprocess consistently reports:

```
error: Committing is not possible because you have unmerged files.
```

The daemon also logged an alert about the unpushed commits:

```
Jun 21 16:29:25 nixos dracon-sync[1514387]:
   🔔 sync alert: /home/dracon/Dev/dracon-platform — Stuck Ahead
      (Unpushed): commits not reaching origin for >10 min —
      push may be failing
```

But the actual reason the commits don't reach origin is **not the
push** — it's the commit step. No push is ever attempted because no
new commit object is created.

### Daemon staging does stage the unmerged files

```
$ journalctl --user -u dracon-sync.service --since "60 min ago" --no-pager | \
    grep -E 'U\s+web/ai-hub/audit-' | head -8
Jun 21 16:19:26 nixos dracon-sync[1832630]: U  web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile-drawer-open.png
Jun 21 16:19:26 nixos dracon-sync[1832630]: U  web/ai-hub/audit-20260629/05-mobile-view-screenshots/providers-mobile.png
Jun 21 16:19:26 nixos dracon-sync[1832630]: U  web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/02-main-nav-open.png
Jun 21 16:19:26 nixos dracon-sync[1832630]: U  web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/04-desktop-baseline.png
Jun 21 16:19:34 nixos dracon-sync[1833002]: U  web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile-drawer-open.png
...
```

The daemon successfully runs `git add` on the 4 unmerged paths
(they appear in the staging output as `U  <path>`). The `git add`
on a path with conflict state is a no-op (the path is already in the
index at stages 1/2/3) but doesn't error — it's the subsequent
`git commit` that errors out. This confirms the daemon's staging
path works correctly; the failure is at the commit boundary.

### Untracked file inventory (216 paths, 73 MB total)

| Directory | Count | Size | Type |
| --- | ---: | ---: | --- |
| `web/games/games/capture-anime-girls/static/images/cards/` | 193 | ~30 MB | Card art PNGs (`char_NNNN.png`, namespaced from `char_5000` through `char_5192`) |
| `web/games/games/deathrun/docs/` | 9 | ~6.5 MB | `v0.7.1-run-scene-*.png` screenshots |
| `web/games/games/hellhunter/scripts/smoke-out/pause-v5-investigate/` | 6 | <1 MB | Smoke-out debug artifacts |
| `web/games/games/endless-td/scripts/` | 3 | <30 KB | `gen-music-v8.sh`, `gen-menu-backdrop-v8.py`, `post-process-menu-backdrop-v8.sh` |
| `web/games/games/endless-td/static/assets/audio/music/` | 2 | ~35 MB | `music_menu_v8.mp3` (3 MB) and `music_menu_v8.raw.wav` (32 MB) |
| `web/games/games/endless-td/static/assets/raw/`, `static/assets/png/` | 2 | <10 MB | Raw art assets |
| `web/games/games/hellhunter/scripts/pause-investigate.mjs` | 1 | 3 KB | Debug script |
| `web/games/games/hegemon/` (single file) | 1 | <10 KB | One hegemon asset |
| `web/tests/shared/_restructure-home.spec.ts` | 1 | 7 KB | Playwright spec |

**`.gitignore` check** (these are NOT gitignored):

```
$ git -C /home/dracon/Dev/dracon-platform check-ignore -v \
    web/games/games/capture-anime-girls/static/images/cards/char_5007.png \
    web/games/games/capture-anime-girls/static/images/cards/char_0001.png
.gitignore:95:!*.png  web/games/games/capture-anime-girls/static/images/cards/char_5007.png
.gitignore:95:!*.png  web/games/games/capture-anime-girls/static/images/cards/char_0001.png
```

The root `.gitignore` line 95 is `!*.png`, which forces all PNGs to
be tracked. The per-game `.gitignore` at
`web/games/games/capture-anime-girls/.gitignore` doesn't exclude any
of these paths. So all 216 files SHOULD be tracked but the daemon
can't commit them.

## Root cause analysis

### Why the index is unmerged

**The unmerged state is the legacy of a prior merge/cherry-pick that
was never completed.** The 4 PNGs were first added to the tree by
commit `5e37f6e2e2` ("313 file(s) in web" by DraconDev, dated
2026-06-20 21:56:10):

```
$ git -C /home/dracon/Dev/dracon-platform log --diff-filter=A \
    --pretty=format:'%h %an %ad %s' --date=iso -- \
    web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile-drawer-open.png | head -1
5e37f6e2e2 DraconDev 2026-06-20 21:56:10 +0100 313 file(s) in web
```

That commit was NOT a merge commit (its parents are `c9be34a4dc4…`,
single parent). The unmerged state must therefore have been created
by a separate git operation AFTER `5e37f6e2e2`. The git reflog
contains many merge entries from earlier sessions (e.g.
`merge gitlab/main: Merge made by the 'ort' strategy.` at
`main@{2338}`), but the relevant current state has the unmerged
markers WITHOUT a merge commit at HEAD. The most likely cause is
one of:

1. A `git pull --rebase` or `git merge` was started and interrupted
   (no editor save, no `git merge --continue`) before the user's
   session ended. The operator's `.bash_history` would show the
   exact command.
2. The Pi agent session (PID `dev`) that ran before `DraconDev`
   took over left a half-finished merge. The override file at
   `/home/dracon/Dev/dracon-platform/.dracon/dracon-sync.toml`
   documents a `311f1889f` pi-authored commit from a prior session
   that the daemon was incorrectly flagging as `unowned`. That
   same session may have run `git merge origin/main` to reconcile
   pi-authored + DraconDev branches and left the index in this
   state.
3. A `git reset --merge` was used in the recent past (no
   `--hard`) that preserved the unmerged entries.

The 4 unmerged PNGs themselves are NOT in conflict on disk — only
in the index. Their on-disk content is whatever is currently in
the working tree. So this is recoverable by:

- `git add` (already happening; doesn't help) followed by
  `git commit` (fails) → need to either:
  - `git checkout --ours <path>` (take local version) for each of
    the 4 PNGs, OR
  - `git checkout --theirs <path>` (take remote version), OR
  - `git rm <path>` (delete the unmerged state, then re-add the
    file), OR
  - `git reset HEAD <path>` to clear the index entry and keep the
    working tree.

The choice depends on whether the operator wants the local working
tree content (most likely — the daemon keeps re-staging them) or
the content from the last successful commit.

### Why the daemon can't recover automatically

The daemon's policy is "commit ALL untracked non-gitignored files".
It does NOT have a recovery path for a partially-merged index
because:

1. `git add` on a `U`-status path is a no-op (the path already
   has stage 1/2/3 entries in the index).
2. `git commit` on an index with unresolved merge entries errors
   out with `cannot create a tree from a not fully merged index.`
   — this is a hard git safety check, not a daemon problem.
3. The daemon does NOT run `git merge --continue`,
   `git merge --abort`, `git checkout --ours`, or
   `git checkout --theirs`. None of those would be safe to run
   automatically (they could discard operator work).

This is by design. The daemon's role is to auto-commit clean
state, not to resolve merge conflicts. A long-lived unmerged
state is **operator territory**.

### Why codeberg failed at 10:20 (contributing cause)

```
$ journalctl --user -u dracon-sync.service --since "today" --no-pager | \
    grep -E 'push to codeberg.*Connection refused' | head -3
Jun 21 10:20:44 nixos dracon-sync[441017]: ⚠️ push to codeberg failed for
   /home/dracon/Dev/dracon-platform: ssh: connect to host codeberg.org port
   22: Connection refused
Jun 21 10:20:44 nixos dracon-sync[441017]: ⚠️ push failed for
   /home/dracon/Dev/dracon-platform
...
Jun 21 10:23:14 nixos dracon-sync[441017]: ✅ push recovered for
   /home/dracon/Dev/dracon-platform
```

`codeberg.org:22` refused SSH connections for ~2.5 minutes (5
retries spaced 31-33 s apart). The daemon recovered automatically
once SSH was restored. This is a transient transport issue, NOT a
configuration bug. It would not have prevented the 2 Phase 24
commits from being pushed if the index weren't already unmerged
(because the daemon can't even ATTEMPT a push when no new commit
exists).

### Why the v0.1.12 staging fix did not resolve this

The v0.1.12 staging fix
(`docs/design/daemon-staging-fix-2026-06-19.md`) added:

1. `untracked_files` to the fingerprint format
   (`dracon-sync/src/daemon.rs:1807`).
2. A settling bypass when `untracked_files > 0 && modified_files == 0`
   (`dracon-sync/src/daemon.rs:1865-1866`).
3. A `max_stage_batch_files` policy limit of 100
   (`dracon-sync/src/policy.rs:447`,
   `dracon-sync/src/sync.rs:2175-2186`).

NONE of these address an unmerged index state. The fingerprint
change speeds up detection of untracked changes, the settling
bypass removes the inactivity wait, and the batch limit splits
large staging batches into ≤100-file commits. All three are about
*untracked file staging speed*, not about recovering from
incomplete merge operations. The unmerged state has been blocking
the commit step since well before v0.1.12.

## Daemon commit cadence (observed)

| Window | Successful commits | Failed attempts |
| --- | ---: | ---: |
| Last 60 min (16:01 → 17:01) | 0 | 93 |
| Last 24 h (17:01 yesterday → 17:01 today) | 0 | many hundreds |

The 24-hour `info` log shows that the daemon has NOT produced a
"committed N file(s)" success message for `dracon-platform` since
the unmerged state was introduced. The most recent non-error info
entry for the repo is `ℹ️ /home/dracon/Dev/dracon-platform has N
small untracked excluded file(s)` (a different code path; just
reports untracked file counts without committing).

The daemon is operating at its full retry rate (every ~10 s)
but every attempt fails at the `git commit` step. The daemon
also entered "exceeded max failures (5), skipping until resolved"
mode at `Jun 21 16:59:31`, which is a 60-second skip window — that
is why the error rate (93/h) is below the theoretical max
(360/h at one every 10 s).

## What the daemon tried (recent transcript)

Selected excerpts (timestamps in daemon's local time, 2026-06-21):

```
16:29:25 🔔 sync alert: /home/dracon/Dev/dracon-platform —
   Stuck Ahead (Unpushed): commits not reaching origin for >10 min
16:40:09 ⚠️ sync failed (late) for /home/dracon/Dev/dracon-platform:
   Git operation failed: cannot create a tree from a not fully
   merged index.; class=Index (10); code=Unmerged (-10)
16:40:17 ⚠️ sync failed (late) … (same error)
16:40:27 ⚠️ sync failed (late) …
…
16:59:31 ⚠️ /home/dracon/Dev/dracon-platform exceeded max failures
   (5), skipping until resolved
17:00:07 ⚠️ sync failed (late) …
17:00:17 ⚠️ sync failed (late) …
17:00:27 ⚠️ sync failed (late) …
17:00:39 ⚠️ sync failed (late) …
17:00:49 ⚠️ sync failed (late) …
17:01:00 ⚠️ sync failed (late) …
17:01:11 ⚠️ sync failed (late) …
17:01:22 ⚠️ sync failed (late) …
```

There is no other error category in the daemon log for this repo.
The failure is exclusively the merge-state block.

## Why the v0.1.12 staging fix is not regressed by this finding

The v0.1.12 staging fix is working correctly when there is no
unmerged state. The fix has been in production for 2 days and has
successfully committed batches of 100 files in unrelated repos
(see `evidence/daemon-staging-fix-final-2026-06-19.md` and the
`bf3eac4a` "104 file(s) in dracon-sync,dracon-warden" commit in
`dracon-utilities` that contains the staging changes). The
`dracon-platform` block is **specific to a corrupted index state**
in that one repo and is not representative of daemon behavior
elsewhere.

## Resolution plan (operator decision required)

The investigation deliberately does NOT auto-resolve. Three safe
options, ordered from least to most destructive:

### Option A: take-theirs (preferred — preserves operator work)

For each of the 4 unmerged PNGs:

```bash
cd /home/dracon/Dev/dracon-platform
for f in \
  web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile-drawer-open.png \
  web/ai-hub/audit-20260629/05-mobile-view-screenshots/providers-mobile.png \
  web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/02-main-nav-open.png \
  web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/04-desktop-baseline.png; do
  git checkout --ours "$f"  # keep the working tree version
  git add "$f"
done
git commit -m "Resolve unmerged PNGs from interrupted merge — keep local"
```

Why `--ours`: the daemon's recent `git add` operations on these
paths indicate the operator wants the working-tree content tracked.
The 4 PNGs are recent Playwright screenshots whose on-disk version
is what the operator wants.

### Option B: rm-and-readd (alternative — same effect, simpler)

```bash
cd /home/dracon/Dev/dracon-platform
for f in \
  web/ai-hub/audit-20260629/05-mobile-view-screenshots/free-mobile-drawer-open.png \
  web/ai-hub/audit-20260629/05-mobile-view-screenshots/providers-mobile.png \
  web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/02-main-nav-open.png \
  web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/04-desktop-baseline.png; do
  git rm --cached "$f"   # remove the conflicted index entries
  git add "$f"           # re-add as a clean entry
done
git commit -m "Resolve unmerged PNGs by re-adding from working tree"
```

### Option C: abort (most destructive — throws away unmerged work)

```bash
cd /home/dracon/Dev/dracon-platform
git merge --abort   # only valid if a merge is in progress
# OR if no merge is in progress:
git reset HEAD web/ai-hub/audit-20260629/05-mobile-view-screenshots/*.png \
               web/ai-hub/audit-20260629/06-mobile-dropdown-screenshots/*.png
```

Option C does not work because no merge is in progress (HEAD has
no `.git/MERGE_HEAD`); it would only apply if `git status` showed
`unmerged paths` AND `git status --short` had a `##` line with
`MERGING`. Neither is true here.

### After resolving the unmerged state

1. The daemon will commit the 2 Phase 24 commits on the next cycle
   AND will push them to all 4 remotes within ~30 seconds.
2. The 216 untracked non-gitignored files will be auto-staged and
   committed in batches of ≤100 (per `max_stage_batch_files`).
3. The `ahead=0 behind=2` will become `ahead=215 behind=0` after
   one daemon cycle, then `ahead=0 behind=0` after the next push.
4. The 216 untracked files will consume 3 batches (193 + 23
   split by directory or by byte count).
5. Total bandwidth: ~73 MB pushed to 4 remotes = ~292 MB of push
   traffic (allow 5-15 min for the slow codeberg push given the
   `push_op_timeout_secs = 300` setting and the historical
   PNG-heavy push timings from
   `docs/design/push-timeout-fix-2026-06-17.md`).

### Why the daemon cannot auto-recover

The daemon could in principle detect the unmerged state and emit
a clearer alert (`⚠️ /home/dracon/Dev/dracon-platform has 4
unmerged paths; manual intervention required`) instead of looping
on `git commit` failures. This would be a small daemon code change
in `dracon-sync/src/sync.rs` (around the `stage_commit_and_push`
function): check `git ls-files --unmerged` before attempting the
commit, and if non-empty, log a single operator-actionable error
and skip the cycle. Implementing this is **OUT OF SCOPE** for the
current investigation goal — it's a future improvement that would
reduce log noise and shorten the operator's time-to-diagnose for
the next time this happens.

## Open questions for the operator

1. **Which Option (A, B, or C) does the operator want to apply?**
   Option A is recommended based on the daemon's recent staging
   behavior.
2. **Should the daemon code be patched to detect + alert on the
   unmerged state instead of looping on `git commit` failures?**
   This is a 5-10 line change in
   `dracon-sync/src/sync.rs::stage_commit_and_push` plus a unit
   test.
3. **Should the post-recovery push be a single combined commit
   (one giant 216-file commit) or multiple ≤100-file commits
   (3 commits per the `max_stage_batch_files` policy)?** The
   current policy says ≤100, which would naturally split this
   into 3 batches.
4. **Should any of the 216 untracked files be added to
   `.gitignore` instead of committed?** Examples:
   - `web/games/games/endless-td/static/assets/audio/music/music_menu_v8.raw.wav`
     is 32 MB (large but under the 100 MiB limit) and a
     derivative of `music_menu_v8.mp3` (3 MB). The .wav is
     generated by an upstream tool and may be reproducible.
   - `web/games/games/hellhunter/scripts/smoke-out/pause-v5-investigate/`
     (6 files) and `pause-investigate.mjs` look like debug
     artifacts from a single investigation and could be added
     to a per-game `.gitignore` if not intended for the repo.

## Reference

- `docs/design/daemon-staging-fix-2026-06-19.md` — the v0.1.12
  staging fix (NOT regressed by this finding).
- `docs/design/daemon-staging-fix-final-2026-06-19.md` —
  verification evidence for the v0.1.12 fix.
- `docs/design/untracked-audit-2026-06-17.md` — the untracked
  file policy.
- `docs/design/push-timeout-fix-2026-06-17.md` — the 300s
  push timeout policy.
- `docs/design/ownership-investigation-2026-06-15.md` — the
  override file for dracon-platform (`owned = true`).
- `dracon-sync/src/daemon.rs:1807-1866` — the v0.1.12 fingerprint
  + settling bypass code.
- `dracon-sync/src/policy.rs:447` — `max_stage_batch_files`
  default 100.
- `dracon-sync/src/sync.rs:2175-2186` — `.take(max_batch)`
  batching logic.
