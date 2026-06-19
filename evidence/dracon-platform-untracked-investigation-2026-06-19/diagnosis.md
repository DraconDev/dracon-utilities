# Dracon-Platform Untracked Files Investigation (2026-06-19)

## Context

After fixing the author regression on dracon-platform (4 pi-authored
commits rewritten to DraconDev, force-pushed to all 4 remotes), the
`dracon-sync repos` table still showed 252 untracked files and 4
modified files. The daemon's `untracked_exclude_patterns = []`
global config means ALL untracked files should be auto-committed.

## Root Cause: Lock File Contention

Investigation of `journalctl --user -u dracon-sync.service --since "1h ago"`
revealed the daemon is in a **lock file contention loop**:

```
Jun 19 19:28:43 nixos dracon-sync[188760]: ⚠️ /home/dracon/Dev/dracon-platform git add failed for 7818 paths: [".dracon/data/keys/owner_nixos.pub", ...]
Jun 19 19:28:17 nixos dracon-sync[188760]: 🔄 trailing-drain: clearing 1 stuck in_flight entries: {"/home/dracon/Dev/dracon-utilities"}
Jun 19 19:27:43 nixos dracon-sync[188760]: 🔄 trailing-drain: clearing 2 stuck in_flight entries: {"/home/dracon/Dev/dracon-platform", "/home/dracon/Dev/dracon-utilities"}
Jun 19 19:27:19 nixos dracon-sync[188760]: ⚠️ /home/dracon/Dev/dracon-platform git add failed for 955 paths: [...]
```

### Pattern

1. Daemon tries to `git add` thousands of untracked files
2. `git add` fails because `.git/index.lock` already exists
   (from a prior daemon run, the operator's rebase, or a crash)
3. Trailing-drain clears the stuck in_flight entry
4. Daemon retries, but lock file reappears
5. **Infinite loop** — daemon never successfully commits

### Evidence

- `journalctl` shows `git add failed` every 30-60 seconds
- `trailing-drain: clearing 1 stuck in_flight entries` appears
  between failures
- During the operator's rebase, daemon logged
  `has rebase in progress, skipping (manual intervention required)`
- After rebase, daemon resumed trying but kept failing on lock

### Why Lock File Appears

Likely causes:
1. **Stale locks from prior daemon crashes** — daemon doesn't
   always clean up `.git/index.lock` on abnormal exit
2. **Concurrent git operations** — the operator's rebase held
   a lock for ~10 minutes; daemon retried every 3s
3. **The `git add` command itself** may fail to clean up its own
   lock if interrupted by a signal

## Verification

```
$ git add .gitattributes
ok 1 file changed, 53 insertions(+)
$ git add .gitignore .github/FUNDING.yml
ok 3 files changed, 160 insertions(+)
```

When run manually (with no competing process), `git add` succeeds.
The failure is not a file content issue — it's a lock contention issue.

## Resolution

Since the daemon is stuck in a lock contention loop, the cleanest
resolution is to **manually commit the untracked files in batches**,
then let the daemon resume normal operation once the working tree
is clean.

### Approach

1. Remove any stale `.git/index.lock`
2. `git add` untracked files in batches of ~500 per commit
3. `git commit` each batch with a descriptive message
4. Push to all 4 remotes (no force-push needed — these are new
   commits on top of the rewritten history)
5. After working tree is clean, daemon should resume normal
   auto-commit behavior

### Files to commit

- 252 untracked files (after stash pop)
- 4 modified files (from the stashed layout-width work)

Breakdown by type (from earlier `git status --short` analysis):
- 3,806 PNG (down from initial 8,037 — daemon committed some)
- 944 MD
- 839 JPG
- 623 TS
- 324 MJS
- 255 SVG
- 236 Svelte
- 210 MP3
- 183 JSON
- 128 RS

## Related Design Docs

- `/home/dracon/Dev/dracon-utilities/docs/design/ownership-investigation-2026-06-15.md`
  — documents the 3 per-repo ownership overrides
- `AGENTS.md` — daemon commit-all policy and per-repo override
  mechanism
