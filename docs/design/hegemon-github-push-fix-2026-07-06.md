# hegemon GitHub push fix + history shrink (2026-07-06)

## Problem (root cause)

Hegemon's GitHub push had been failing because the repo's `.git` pack was
~3.98 GiB — over GitHub's hard **2 GiB-per-pack** limit. The daemon treated
the failure as *retryable*, so every sync cycle re-attempted the doomed push,
re-packing 4 GB and saturating the `sem_max_concurrent_sync=4` budget. This
caused a **27-minute stall** (22:15→22:42) during which 25 other repos were
gated behind the stuck `in_flight` entry.

Two distinct fixes were needed:

1. **Daemon behavior** — stop vainly retrying a *permanent* failure
   (pack-too-large) and fail fast + notify instead.
2. **Repo size** — actually shrink hegemon's history below 2 GiB so GitHub
   accepts the push.

## Part 1 — daemon push-retry classification (dracon-sync `0f6727b`)

Commit `0f6727b` in `dracon-sync` (pushed to origin/github/gitlab/codeberg):

- **`is_pack_too_large(err_msg)`** (`src/git/push.rs`): matches GitHub
  `GH001`, `large files detected`, `pack exceeds`, `exceeds the maximum
  allowed size`, `remote error: pack`, `pack is too large` (case-insensitive).
- Wired into `push_with_transport_fallbacks` and `push_with_retries` to
  **return immediately (no retry)** — treated like a protected-branch /
  hook-declined permanent rejection.
- Wired into `push_to_named_remote` (`src/git/multi_remote.rs`).
- **Proactive GitHub skip** in `push_background` (`src/sync.rs`): measures
  `.git` size via `measure_git_size_bytes` (made `pub(crate)` in
  `report.rs`); if ≥ 2 GiB, GitHub is excluded from `push_mirror_remotes`
  *before* the slow re-pack is even attempted. One-time `log_warn!` +
  `notify_webhook_failure(..., "PACK_TOO_LARGE: ...")`. Records a skip marker
  in `remote_failures["github"]` that **self-heals** once the repo shrinks
  below 2 GiB (the marker is only cleared when the repo actually pushes
  successfully).
- 4 unit tests for `is_pack_too_large` added; `cargo test --workspace
  --locked` green (667 passed, 0 failed).

This means: a transient failure (net blip, GitHub down) still retries, but a
pack-too-large failure fails fast and notifies — no more 27-min stalls.

## Part 2 — shrink hegemon history

### Blob inventory (pre-rewrite, shared gitdir 5.15 GB)
- `static/assets/**` = **2,857 MiB** (incl. 932 MP3 + PNG versions,
  many historical).
- `.pi/**` (gitignored junk) = 1,962 MiB in history.
- Real repo source ≈ 250 MiB.

### Rewrite
- Stopped daemon; backed up working-tree `static/assets` to `/tmp`.
- Cloned shared gitdir to `/tmp/hegemon-rewrite`, ran:
  `git filter-repo --invert-paths --path-glob 'static/assets/**' --path-glob '.pi/**'`
- Pack **3.98 GiB → 95.5 MiB**. New main = `a36b158`.
- Force-pushed `a36b158` to **origin, codeberg, github** (github was empty
  → now has 95 MiB history; primary goal achieved).
- **GitLab NOT force-pushed**: its `main` (`6e718cbf`) is a *protected branch*
  and diverged by 1,760 commits from the rewritten history. Force-pushing
  would destroy those commits. GitLab is excluded instead.

### Safety nets
- Bare backup of OLD history: `/home/dracon/hegemon-pre-filterrepo-backup-20260706.git`
  (4.9 GB). Keep until GitLab is reconciled.
- GitLab remote `6e718cbf` left intact (divergent history preserved).
- Deleted `daemon-standalone` + stray `HEAD` branches that pinned old history
  (re-bloat risk).

### Critical catch — daemon re-committed binaries
After restarting the daemon *before* the `.gitignore` fix landed, the daemon
saw 839 untracked `static/assets` binaries (not yet ignored) and
**auto-committed 776 of them** (`471d6ea`, BIN:776). Fixed by:
- Stopping daemon; `git rebase --onto a36b158 471d6ea main` (dropped the
  binary re-commit).
- Adding `static/assets/**/*.png|jpg|jpeg|mp3|mp4|mov|webp|gif|wav|ogg`
  patterns to `.gitignore` (JSON/SVG manifests stay tracked; source untouched).
- Force-pushed cleaned history (`75934ca`); `git gc --prune=now` → shared
  gitdir **96M**.
- Re-applied `exclude_remotes = ["gitlab"]` to `.dracon/dracon-sync.toml`
  (lost during the rebase) and committed (`a16b9d1008`).

### Parent gitlink
`dracon-platform` gitlink advanced to the rewritten history
(`a36b158`, then `75934ca`, then `a16b9d1008`).

## Final state (2026-07-07)

| Item | State |
|------|-------|
| Hegemon `.git` size | **96M** (was 5.15 GB) — no 2 GiB warning |
| Hegemon GitHub push | ✅ synced (95 MiB history) |
| Hegemon remotes | origin/codeberg/github synced to `a16b9d1008`; gitlab excluded |
| GitLab | excluded (divergent protected branch `6e718cbf` preserved) |
| Daemon | running new binary (`0f6727b`); active, no stalls |
| `.pi/` | gitignored everywhere (0 tracked) |

## Anti-rebloat rules (must hold — any loosening risks 4 GB re-bloat)

`hegemon/.gitignore`:
```
static/assets/**/*.png
static/assets/**/*.jpg
static/assets/**/*.jpeg
static/assets/**/*.mp3
static/assets/**/*.mp4
static/assets/**/*.mov
static/assets/**/*.webp
static/assets/**/*.gif
static/assets/**/*.wav
static/assets/**/*.ogg
```

`hegemon/.dracon/dracon-sync.toml`:
```toml
exclude_remotes = ["gitlab"]
auto_commit_exclude_patterns = [
    "**/.pi/chrome-screenshots/**",
    "**/.pi/visual-audit/**",
    "**/.pi/investigation/**/*.png",
]
```

## Re-bloat recovery (the `471d6ea` pattern)

If the daemon ever auto-commits binaries again (e.g. someone edits the
`.gitignore` and a binary is staged before the daemon re-reads it):
1. **Stop the daemon first** so it can't commit again.
2. `git -C $WG rebase --onto <slimmed-base> <bad-commit-sha> main`
   (drop the bad commit; replay anything on top — usually just a
   `.gitignore` rule — onto the slimmed base).
3. Force-push all 3 remotes with corrected syntax:
   `git -C $WG push origin main && git -C $WG push codeberg main && git -C $WG push github main`
4. `git -C /home/dracon/Dev/dracon-platform/.git/modules/web-games-hegemon gc --prune=now`
5. Restart the daemon.
6. Verify `du -sh` on the shared gitdir is back under 200 MiB and
   `git log` shows no `BIN:` commits.

## Verification — all 3 remotes in one command

```bash
WG=/home/dracon/Dev/dracon-platform/web/games/wip/hegemon
LOCAL=$(git -C "$WG" rev-parse main)
for r in origin codeberg github; do
  printf "%-8s " "$r"; git -C "$WG" ls-remote --heads "$r" main 2>/dev/null | awk '{print $1}'
done
printf "%-8s local  %s\n" "" "$LOCAL"
```

All four lines must show the same SHA. Any divergence means a stale
remote-tracking ref or a failed force-push that needs remediation.

## To reconcile GitLab later
GitLab `main` (`6e718cbf`, 1,760 commits ahead of rewritten history) is the
**only** remote with unique history. Options:
1. **Rebase its unique commits** onto `a36b158` (the asset-free base), then
   force-push + remove `exclude_remotes = ["gitlab"]`.
2. **Migrate `static/assets/` to an OVH bucket** (the planned long-term fix),
   then the divergence is just asset commits that can be dropped.
3. Leave GitLab excluded permanently (it is a mirror; origin/codeberg/github
   are authoritative).

Do NOT force-push GitLab until the unique 1,760 commits are accounted for.
