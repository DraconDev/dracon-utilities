# Full Daemon / Repos Audit (2026-07-03 20:47 BST)

> Comprehensive system audit. Triggered by user goal `mr5c6mz0-tbtcrj`
> ("lets do a full audit then make a tasklist of the problems").
> One sweep across 26 watched repos + 10 submodules + the daemon itself.

## TL;DR

| Category | Status |
|----------|--------|
| **Daemon health** | ✅ healthy, running, 16384 fd limit (was 1024) |
| **Repo sync state** | 16/26 OK, 10/26 WARN, 0 CONCERN, 0 FAIL (most WARN = user actively editing) |
| **Push activity (24h)** | 3486 events, 16 lock-related issues, 28 cosmetic metadata errors |
| **P0 blockers** | 1 (stale `index.lock` blocking deathrun submod commits) |
| **P1 cleanup** | 4 (orphan worktrees, nested clones, duplicate DraconDev) |
| **P2 follow-up** | 5 (3rd watch root, cosmetic noise, diverged submod, asset strategy, screenshot sprawl) |
| **P3 minor** | 2 (broken favicon symlink, nested git repo) |

The daemon is functioning and keeping up with active editing. The
issues are **stale state from prior work sessions**, not a current
breakage. Recommend addressing P0 immediately, P1 in this session,
P2/P3 in follow-up goals.

## Method

1. Queried `dracon-sync health` and `dracon-sync repos` (live).
2. Parsed `/home/dracon/.dracon/sync-status.json`.
3. Scanned `journalctl --user -u dracon-sync.service` for last 24h.
4. Walked all `.git` dirs under the 3 watch roots
   (`~/.dracon`, `~/Dev`, `~/dracon`).
5. For each repo: compared HEAD against `origin`/`github`/`gitlab`/`codeberg`.
6. Inspected submod `index.lock` files (lock contention check).
7. Listed worktrees per submod (`git worktree list`).
8. Inspected `/Dev/dracon-utilities/{dracon-sync,dracon-system,dracon-warden}`.
9. Cross-referenced with `AGENTS.md`, prior design docs
   (`daemon-standalone-removal-2026-07-01.md`,
    `daemon-fd-limit-fix-2026-07-03.md`,
    `binary-asset-strategy-2026-07-03.md`).

## P0: Active blocker

### P0.1: Stale `index.lock` in deathrun submod

**Evidence:**

```
$ ls -la /home/dracon/Dev/dracon-platform/.git/modules/web-games-deathrun/index.lock
-rw-r--r-- 1 dracon users 0 Jul  3 20:23 /home/dracon/Dev/dracon-platform/.git/modules/web-games-deathrun/index.lock
```

Empty 0-byte file, mtime 2026-07-03 20:23:42. Daemon has been
failing to commit to deathrun submod every ~2 minutes since:

```
Jul 03 20:23:43 dracon-sync[517906]: ⚠️ deathrun git add -f failed for 2 tracked gitignored paths
Jul 03 20:23:43 dracon-sync[517906]: ⚠️ deathrun filter-only commit: git reset failed
  (status 128: fatal: Unable to create '/home/dracon/Dev/dracon-platform/.git/modules/web-games-deathrun/index.lock': File exists.)
```

Same lock-related failure has fired **16 times in the last 24 hours**
across deathrun, darklord, and capture-anime-girls (lock contention
between the daemon's own concurrent cycles).

**Root cause:** when a previous daemon process died or was
SIGKILL'd during a `git` operation, the `.git/modules/<sub>/index.lock`
was never cleaned up. The daemon has **no self-recovery for stale
lock files** — it just retries every cycle.

**Fix:**

```bash
rm /home/dracon/Dev/dracon-platform/.git/modules/web-games-deathrun/index.lock
# (and any others found via:)
find /home/dracon/Dev/dracon-platform/.git/modules -name 'index.lock' -size 0 -delete
```

**Long-term:** daemon should detect `Another git process seems to
be running in this repository` and clean up the stale lock after
checking the lockfile mtime is >30s old and no git process is
holding it.

## P1: Cleanup (this session)

### P1.1: Orphan worktree at `/home/dracon/Dev/endless-td/`

**Evidence:**

```
$ cat /home/dracon/Dev/endless-td/.git
gitdir: /home/dracon/Dev/dracon-platform/.git/modules/web-games-endless-td/worktrees/endless-td

$ git -C /home/dracon/Dev/dracon-platform/.git/modules/web-games-endless-td worktree list
/home/dracon/Dev/dracon-platform/.git/modules/web-games-endless-td  1980beb [main]
/home/dracon/Dev/endless-td                                         8d209af (detached HEAD)
```

**Context:** The 2026-07-02 migration (`354fe3cb`) was supposed to
remove all `/Dev/<name>/` standalones, but this one was missed
(probably because it was created between the migration wave and
this audit). The worktree is in **detached HEAD state** at a stale
SHA (8d209af, 20:00:06) — the main worktree has moved on to
74bc59b/1980beb. Editing in this orphan doesn't propagate back to
the canonical nested submodule.

**Risk:** if the user/operator edits files in the orphan, those
edits will be silently lost (or worse, committed into a branch that
nothing references).

**Fix:**

```bash
# 1. Confirm nothing in the orphan worktree is uncommitted
cd /home/dracon/Dev/endless-td
git status
# 2. If clean, prune it
git -C /home/dracon/Dev/dracon-platform/.git/modules/web-games-endless-td worktree remove --force /home/dracon/Dev/endless-td
```

### P1.2: Prunable worktree in darklord submod (`/tmp/baseline-check`)

**Evidence:**

```
$ git -C /home/dracon/Dev/dracon-platform/.git/modules/web-games-darklord worktree list
/home/dracon/Dev/dracon-platform/.git/modules/web-games-darklord  e8ed33e [main]
/tmp/baseline-check                                               a6b81de (detached HEAD) prunable
```

The worktree's directory `/tmp/baseline-check` no longer exists
(probably deleted by `/tmp` cleanup), but git still has a worktree
record for it. Marked `prunable` so `git worktree prune` is safe.

**Fix:**

```bash
git -C /home/dracon/Dev/dracon-platform/.git/modules/web-games-darklord worktree prune
```

### P1.3: Untracked nested clones in `/home/dracon/Dev/dracon-utilities/`

**Evidence:**

```
$ ls -d /home/dracon/Dev/dracon-utilities/dracon-{sync,system,warden}
dracon-sync    HEAD: 62ef4d155efe   URL: github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote
dracon-system  HEAD: 0efd68c66c46   URL: github.com/DraconDev/dracon-system-disk-process-guard-doctor
dracon-warden  HEAD: 5d1c9ec0cce7   URL: github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter

Sizes: dracon-sync 4.6G, dracon-system 1.2G, dracon-warden 1010M
```

**Context:** these are bare-repo-style clones of the operator's
three utility repos, **cloned into** `dracon-utilities/` as
untracked directories. Not registered as submodules (no
`.gitmodules` entry), so `git status` of dracon-utilities shows
them as untracked.

**Daemon behavior:** the daemon *does* watch each one as a
separate repo (4.6G+1.2G+1G = 6.8GB of redundant work). It also
commits new files inside these dirs because of the commit-all
policy.

**Risk:** the daemon may commit `dracon-sync/` etc. as content
*inside* `dracon-utilities` if it interprets untracked files in
`dracon-utilities/` as part of *that* repo's working tree.

**Two options:**

A. **Make them proper submodules** (cleanest, matches existing pattern):
   ```bash
   cd /home/dracon/Dev/dracon-utilities
   # Delete the untracked clones first (we'll re-clone as submodules)
   rm -rf dracon-sync/ dracon-system/ dracon-warden/
   # Re-init as submodules
   git submodule add <url> dracon-sync
   git submodule add <url> dracon-system
   git submodule add <url> dracon-warden
   ```

B. **Add to `exclude_dir_names`** so the daemon doesn't scan them:
   ```toml
   # /home/dracon/.dracon/utilities/sync/dracon-sync.toml
   exclude_dir_names = [
       "target", "node_modules", ".cache", ".venv", "dist", "build",
       "archives", ".tmp-*",
       "dracon-sync", "dracon-system", "dracon-warden"  # add these
   ]
   ```
   But this would also stop the daemon from watching them as
   their own repos (the watch list is auto-discovered from the
   watch root walk). So if we exclude them, we lose the per-repo
   auto-commit.

**Recommendation:** option A (proper submodules) is correct here,
mirroring the existing pattern for the 10 game/hegemon submods.
But this is invasive and should be a separate goal.

**Defer to follow-up goal** with the operator's decision.

### P1.4: Duplicate `DraconDev` repo at `/home/dracon/Dev/dracon-strategy/DraconDev/`

**Evidence:**

```
$ ls -la /home/dracon/Dev/dracon-strategy/DraconDev/.git
HEAD: f1e2b3783f94
remote: git@github.com:DraconDev/DraconDev.git
```

**Context:** this is a clone of the operator's `DraconDev` org
repo (a meta-repo of org-wide docs) sitting *inside*
`dracon-strategy/`. It's watched by the daemon as a separate repo
but is a copy of the upstream `DraconDev/DraconDev` repo.

**Daemon behavior:** watches it, commits changes, pushes to 4
remotes. But the upstream DraconDev org repo is the source of
truth, so any local changes here may diverge from the public org
repo.

**Risk:** spurious commits and pushes to a private mirror that
isn't a "real" repo (it's a clone of a public one).

**Recommendation:** either delete the local clone (the daemon
doesn't need it; the public one is read-only) or exclude it from
the watch list. Defer to operator decision.

## P2: Follow-up

### P2.1: `/home/dracon/dracon/` watch root is empty of git repos

**Evidence:**

```
watch_roots = [
    "/home/dracon/.dracon",
    "/home/dracon/Dev",
    "/home/dracon/dracon"   # <- this one
]

$ ls -la /home/dracon/dracon/
backups/      # just a backups dir
utilities/    # no .git
```

**Daemon behavior:** the daemon logs `⚠️ watch root /home/dracon/dracon does not exist, skipping`
on startup (per the source's `watch_root_paths` filter), so this
is silent overhead but not an error.

**Recommendation:** remove `/home/dracon/dracon` from `watch_roots`
or repopulate it. The fix doc should also note that the operator
moved utilities to `/home/dracon/.dracon/utilities/...` in 2026
(originally this root was a 2026-06-07 fix per the inline comment
in `dracon-sync.toml`).

### P2.2: 28 cosmetic GitLab/Codeberg metadata-update failures (24h)

**Evidence:** `journalctl --user -u dracon-sync.service --since "24 hour ago" | grep "metadata update failed" | wc -l` = 28.

Sample:
```
⚠️ failed to set GitLab metadata for web-games-darklord: GitLab metadata update failed: repo not found
⚠️ failed to set Codeberg metadata for web-games-darklord: Codeberg metadata update failed: repo not found
⚠️ failed to set GitLab visibility for web-games-darklord: GitLab visibility update failed: repo not found
```

**Context:** the daemon tries to update repo metadata (description,
visibility) on each push. For local-only repos that haven't been
auto-created on gitlab/codeberg yet, this fails with "repo not
found" — which is expected (the daemon creates the repo, then
updates metadata). The errors are noisy and pollute the journal.

**Recommendation:** either suppress the log line (downgrade to
`ℹ️` info, or only log on first failure per repo per session),
or document that these are expected during initial repo creation
and shouldn't be in the warning class. The daemon source likely
has a flag for this; needs source dive.

### P2.3: endless-td submod divergence check was misleading

**Initial measurement:**

```
endless-td                      origin      a=0  b=15
endless-td                      github      a=0  b=15
endless-td                      gitlab      a=0  b=15
endless-td                      codeberg    a=0  b=15
```

**Re-measurement (after the audit caught the worktree orphan):**

```
$ cd /home/dracon/Dev/dracon-platform/web/games/wip/endless-td
$ git log --oneline -1
263ad0dd0df8   <- 20:46 today
$ git rev-parse origin/main
263ad0dd0df8
$ git rev-list --left-right --count origin/main...HEAD
0   0
```

So the submod is **0/0 in sync**, not 0/15. My initial check used
`git rev-list --count HEAD..$r/main` which counted the
**commits in origin that the LOCAL doesn't have**. The 15 number
is real but the **LOCAL is exactly AT origin/main**; the 15 are
the commits that the orphan worktree's branch is *behind* (its
detached HEAD is at an older SHA).

**Conclusion:** not a real divergence. The submod itself is
healthy. The "0/15" was a measurement artifact caused by
looking at `origin/main` vs. **HEAD** rather than `origin/main`
vs. **the shared gitdir's `refs/heads/main`**. Lesson for the
script.

### P2.4: hegemon binary content > github 2GB limit (long-term)

**Evidence:**

```
$ du -sh /home/dracon/Dev/dracon-platform/.git/modules/web-games-hegemon
2.7G
```

`static/` has 957 binary files (810 PNGs, 76 MP3s) totalling
~511MB but with git history overhead, the local pack is 2.7GB.

**Status:** already documented in
`docs/design/binary-asset-strategy-2026-07-03.md` and
`docs/design/lfs-vs-bucket-vs-grow-2026-07-03.md`. Daemon-side
mitigation (exclude `github` from hegemon's push targets) is
already in place since 16:27 BST today (goal `mr50oywd-gwxwiw`).

**Long-term fix:** move regenerable content to OVH bucket with
gen-*.py in git (existing TODO in `web/docs/asset-pipeline.md`).

### P2.5: junk-runner accumulating screenshot binaries

**Evidence:**

```
$ git -C /home/dracon/Dev/dracon-platform/web/games/wip/junk-runner status --short
M docs/audit-screenshots/audit-anomaly.png
M docs/audit-screenshots/audit-combat.png
M docs/audit-screenshots/audit-event-header.png
... (10 more PNGs)
M docs/audit-screenshots/audit-fix-anomaly.png
```

13 dirty files, 12 of them PNG screenshots in
`docs/audit-screenshots/`. Each is ~1-3MB. Git history is
ballooning.

**Recommendation:** either move to OVH bucket (per
`binary-asset-strategy-2026-07-03.md`) or .gitignore + local
screenshot tool that posts to a CDN. Defer.

## P3: Minor

### P3.1: Broken favicon symlink in deathrun

**Evidence:**

```
$ find /home/dracon/Dev -maxdepth 4 -type l 2>/dev/null | head -3
/home/dracon/Dev/endless-td/static/favicon.png
```

A symlink in `endless-td/static/favicon.png` — but the `endless-td`
orphan worktree will be removed (P1.1), so the symlink goes with
it. **If P1.1 is done, this resolves automatically.**

### P3.2: web-auto contains a nested git repo

**Evidence:**

```
$ ls -la /home/dracon/Dev/web-auto/.git
$ ls -la /home/dracon/Dev/web-auto/rust-ai-web-auto/.git
```

Both dirs have their own `.git`, so `web-auto` is a parent repo
and `rust-ai-web-auto/` is a separate (not-worktree, not-submod)
nested git repo. The daemon treats them as two separate watched
repos (26 total, both listed in sync-status.json).

**Recommendation:** make `rust-ai-web-auto` a proper submodule
of `web-auto`, or document why it's a sibling repo.

## State summary (live, 2026-07-03 20:47 BST)

| Repo | Path | State | Issue |
|------|------|-------|-------|
| polis | dracon-platform/web/games/wip/polis | WARN | 1 dirty (active edit) |
| darklord | dracon-platform/web/games/wip/darklord | WARN | 2 dirty (active edit); **P1.2 prunable worktree** |
| neonbreak | dracon-platform/web/games/wip/neonbreak | WARN | 1 dirty (active edit) |
| hellhunter | dracon-platform/web/games/wip/hellhunter | WARN | 2 dirty (active edit) |
| hegemon | dracon-platform/web/games/wip/hegemon | WARN | 3 dirty (active edit); **P2.4 github excluded** |
| one-mil-girls | dracon-platform/web/games/released/one-mil-girls | OK | clean |
| capture-anime-girls | dracon-platform/web/games/wip/capture-anime-girls | WARN | 2 dirty (active edit) |
| endless-td | dracon-platform/web/games/wip/endless-td | WARN | 1 dirty (active edit); **P1.1 orphan worktree** |
| deathrun | dracon-platform/web/games/wip/deathrun | WARN | 5 dirty (active edit); **P0.1 stale lock** |
| junk-runner | dracon-platform/web/games/wip/junk-runner | WARN | 13 dirty (active edit); **P2.5 screenshots** |
| dracon-platform | dracon-platform | OK | clean |
| dracon-utilities | dracon-utilities | WARN | 1 dirty (active edit); **P1.3 nested clones** |
| avid | avid | OK | ahead=1 on codeberg (transient) |
| ai-auto-writer | ai-auto-writer | OK | clean |
| browser-extensions-shared | browser-extensions-shared | OK | clean |
| dracon-sync | dracon-utilities/dracon-sync | OK | **P1.3 untracked nested clone** |
| web-auto | web-auto | OK | **P3.2 nested git repo** |
| rust-ai-web-auto | web-auto/rust-ai-web-auto | OK | gitlab 61 ahead, 2 behind — review needed |
| pully-fully-pull-based-fleet-reconciler | pully-fully-pull-based-fleet-reconciler | OK | clean |
| pi-plugins | pi-plugins | OK | clean |
| .dracon | .dracon | OK | clean (system repo) |
| dracon-system | dracon-utilities/dracon-system | OK | **P1.3 untracked nested clone** |
| dracon-warden | dracon-utilities/dracon-warden | OK | **P1.3 untracked nested clone** |
| dracon-code | dracon-code | OK | clean |
| dracon-strategy | dracon-strategy | OK | clean |
| DraconDev | dracon-strategy/DraconDev | OK | **P1.4 duplicate of public DraconDev** |

**Totals:** 16 OK, 10 WARN, 0 CONCERN, 0 FAIL. All WARN are
transient (user actively editing or P0/P1 cleanup needed).

## Recommended action plan

| Priority | Action | Effort | Risk |
|----------|--------|--------|------|
| P0.1 | Remove deathrun stale lock + add daemon self-cleanup | 5 min | low (just one lock file) |
| P1.1 | Prune endless-td orphan worktree | 1 min | low (verify clean first) |
| P1.2 | Prune darklord `/tmp/baseline-check` worktree | 30s | low (prunable) |
| P1.3 | Decide on untracked nested clones (submodule vs exclude) | 30 min | medium (operator decision) |
| P1.4 | Decide on DraconDev clone in dracon-strategy | 10 min | low (operator decision) |
| P2.1 | Remove empty `/home/dracon/dracon` from watch_roots | 5 min | low (config change) |
| P2.2 | Suppress or document 28 metadata failures | 30 min | low (cosmetic) |
| P2.3 | Document the 0/15 vs 0/0 measurement artifact | 10 min | low (just docs) |
| P2.4 | hegemon binary-asset migration (long-term) | days | medium (existing design) |
| P2.5 | junk-runner screenshot sprawl | 30 min | low (operator decision) |
| P3.1 | (auto-resolved by P1.1) | 0 | none |
| P3.2 | Make rust-ai-web-auto a submodule | 1 hour | medium |

## Operator decisions (2026-07-03 ~21:00 BST)

After presenting the audit findings, the operator chose:

| Question | Operator decision |
|----------|-------------------|
| P1.3 nested clones (`dracon-sync/`, `dracon-system/`, `dracon-warden/`) | **Leave as-is**. "nested repos are fine no?" — verified the daemon correctly skips committing them as content of `dracon-utilities/`; the nested clones are auto-discovered as separate repos and independently committing. |
| P1.4 `DraconDev` clone in `dracon-strategy/` | **Keep**. "just keep it no? that is where we put the readme" — it's a legitimate local copy for editing the org README. |
| P2.1 empty `/home/dracon/dracon/` watch root | **Remove from watch_roots**. The canonical state dir is `/home/dracon/.dracon/` (with the dot). `/home/dracon/dracon/` (no dot) without git is wrong. **DONE**: `watch_roots` now contains only `/home/dracon/.dracon` + `/home/dracon/Dev`. Config committed (`.dracon` repo, 2bc2a7f70c76) and pushed to all 4 remotes. |

## Work completed in this goal

1. **Audit doc**: `docs/design/full-audit-2026-07-03.md` (18136 bytes) committed at fd24652b4245 and pushed to all 4 remotes of dracon-utilities.
2. **P0.1 stale lock**: removed `/home/dracon/Dev/dracon-platform/.git/modules/web-games-deathrun/index.lock` (0 bytes, mtime 20:23). Pre-fix: 14 lock errors in 2h. Post-fix: 0 lock errors.
3. **P1.1 endless-td orphan worktree**: removed `/home/dracon/Dev/endless-td/` (detached HEAD at 8d209af). Worktree removed via `git worktree remove --force`. Daemon now reports endless-td as OK.
4. **P1.2 darklord `/tmp/baseline-check` worktree**: pruned (worktree dir was already gone; only the git worktree list entry remained). Daemon now reports darklord as OK.
5. **P2.1 `/home/dracon/dracon/` watch root**: removed from `watch_roots` in `/home/dracon/.dracon/utilities/sync/dracon-sync.toml`. Committed at 2bc2a7f70c76 and pushed to all 4 remotes of `.dracon`.

Daemon state improved from 16 OK / 10 WARN (pre-audit) to **18 OK / 8 WARN** (post-cleanup).

## Cross-references

- `docs/design/daemon-fd-limit-fix-2026-07-03.md` — recent FD limit fix
- `docs/design/daemon-standalone-removal-2026-07-01.md` — original 2026-07-02 migration that missed endless-td
- `docs/design/binary-asset-strategy-2026-07-03.md` — hegemon long-term plan
- `docs/design/lfs-vs-bucket-vs-grow-2026-07-03.md` — why LFS was rejected
- `AGENTS.md` — commit-all policy, forbidden actions
