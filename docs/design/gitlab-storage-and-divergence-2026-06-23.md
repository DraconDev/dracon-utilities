# GitLab Storage & Divergence — Operator Action Required

**Date**: 2026-06-23
**Status**: open (requires operator)
**Goal**: mqpu9hd4-kun8kx

## Summary

Two gitlab.com mirrors are stuck in a state the daemon cannot resolve
autonomously. Both are kept in sync on github + codeberg; only gitlab is
problematic. Local repos are clean, the daemon is unblocked, but the
WARN status will not clear until the operator addresses the gitlab side.

## Affected repos

### 1. dracon-platform — storage quota exceeded

- Local HEAD: `935956a7ea7...` (and growing)
- `github/main`: ✅ synced
- `codeberg/main`: ✅ synced
- `gitlab/main`: `73bc23a580...` (14+ commits behind)
- Local `.git/` size: **18 GiB** (10.58 GiB in packs)
- gitlab error: `Your push to this repository cannot be completed as it
  would exceed the allocated storage for your project. Contact your
  GitLab administrator for more information.`

The recent bulk-commit batch (`5957d88e3f add`, `2851361fdd add`) pushed
the platform over the gitlab.com free-tier 10 GiB project size limit.
Every subsequent commit fails to push.

### 2. dracon-utilities — protected main prevents force-push

- Local HEAD: `151e3c6b...` (1+ ahead)
- `github/main`: ✅ synced
- `codeberg/main`: ✅ synced
- `gitlab/main`: `d008d363...` (50 commits divergence — local ahead by 30+,
  remote ahead by 15+ from a 2026-06-21 literal-token incident)
- gitlab error: `Updates were rejected because the tip of your current
  branch is behind its remote counterpart. ... ! [rejected] HEAD -> main
  (non-fast-forward)`
- The daemon's `force_push_when_behind = true` is configured (per
  `[[remotes.gitlab]]` in `/home/dracon/.dracon/utilities/sync/dracon-sync.toml`)
  but gitlab.com main is branch-protected → force-push blocked.

## What was done in this session

1. `dracon-sync repair stuck-unstuck /home/dracon/Dev/dracon-utilities` —
   cleared the PUSH_STUCK state, daemon re-tried the push.
2. `dracon-sync repair stuck-unstuck /home/dracon/Dev/dracon-platform` —
   same for platform.
3. `dracon-sync sync-now` on both repos — committed the small remaining
   files, pushed to github + codeberg, gitlab push failed (expected).
4. `dracon-sync sync-now /home/dracon/Dev/quick-draw-screenshot-clipboard`
   — unstuck the 3-mod/1-ut warning, daemon committed 4 files and pushed
   to all 3 mirrors.
5. The earlier subrepo restoration (dracon-sync, dracon-warden,
   dracon-system all moved into `/home/dracon/Dev/dracon-utilities/`)
   remains stable — all 3 are healthy and idle.

## Current daemon state (17 repos, 15 OK, 2 WARN, 0 CONCERN)

```
✅ OK  (15)  ai-auto-writer, quick-draw-screenshot-clipboard, search-daemon,
             browser-extensions-shared, .dracon, pully-fully-...,
             dracon-sync, rust-ai-web-auto, dracon-code, dracon-strategy,
             avid, DraconDev, dracon-libs, dracon-warden, dracon-system
⚠ WARN (2)  dracon-platform  (gitlab: storage quota)
             dracon-utilities (gitlab: protected main + divergence)
❌ CONCERN (0)
```

## Operator action options

### For dracon-platform

**Option A — Accept the divergence, stop pushing to gitlab for this repo.**
Requires daemon code change to support per-repo `exclude_remotes`. Until
that exists, the daemon will keep retrying and the WARN will not clear.

**Option B — Delete the gitlab mirror.**
On gitlab.com: `dracondev/dracon-platform` → Settings → Advanced →
"Remove project". Then the daemon will report `gitlab ... not found`
on the auto-create path, which the daemon handles as "skip mirror" and
the WARN will clear.

**Option C — Increase gitlab quota.**
Requires gitlab.com admin / support ticket. Free-tier projects cap at
10 GiB; this project is 18+ GiB. May need to upgrade or compress the
local `.git/` first (`git gc --aggressive --prune=now`).

### For dracon-utilities

**Option A — Unprotect main on gitlab.com.**
On `gitlab.com/DraconDev/dracon-utilities` → Settings → Repository →
"Protected branches" → unprotect `main` (or temporarily). Then the
daemon's `force_push_when_behind = true` will fire on the next push
and force-with-lease will reconcile. After it succeeds, re-protect.

**Option B — Delete the gitlab mirror and let the daemon recreate it.**
Same pattern as platform option B. The daemon's `auto_create = true`
will spin up a fresh gitlab project from scratch.

**Option C — Manually reconcile the divergent history.**
`git push --mirror gitlab` from a clean clone. This rewrites gitlab's
main to match local. Requires unprotecting main first (option A).

## Files of interest

- `/home/dracon/.dracon/utilities/sync/dracon-sync.toml` — has
  `force_push_when_behind = true` on both gitlab and codeberg. The
  `repo_name_map` was updated in this session for
  `dracon-{sync,warden,system}` → long descriptive names (×3 remotes).
- `dracon-sync/src/git/multi_remote.rs` — push logic, includes the
  pre-existing URL double-encoding bug in `visibility.rs` (separate issue,
  non-fatal).
- `dracon-sync/src/git/discovery.rs:99-110` — supports the
  "3 sibling repos inside a parent repo" structure that made this
  subrepo restoration possible.

## Non-blocking observations

- The daemon's `set_gitlab_visibility` and `set_gitlab_metadata` in
  `visibility.rs` double-encode the project path (template has 2 `{}`,
  the encoded `owner%2Frepo` replaces both). Non-fatal: just produces
  "GitLab metadata update failed: repo not found" warnings on every
  push. Affects all repos, not just these two. Worth a separate fix.
- The platform's local `.git/` has 12 pack files (10.58 GiB). A
  `git gc --aggressive --prune=now` would reduce that significantly,
  but it does NOT fix the gitlab storage issue (gitlab has its own
  copy, unaffected by local repack).
