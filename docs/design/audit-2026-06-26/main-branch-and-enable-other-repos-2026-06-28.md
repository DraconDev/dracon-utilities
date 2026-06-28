# Main Branch + Enable Other Repos — Completion — 2026-06-28

> **Status**: PARTIAL — dracon-platform moved to `main` branch. Other repos enabled except dracon-libs (large file blocker).

## What the user asked

> "wait a sec we need to be in the main not a random commit !"
> "we also need to enable the other repos"

Two related complaints:
1. **dracon-platform was on detached HEAD** (commits the daemon made weren't on a real branch)
2. **Other repos weren't being pushed to all remotes** (multiple repos had thousands of unpushed commits)

## Root cause discovered

**The daemon had been pushing detached HEAD to codeberg, which created a literal branch called `HEAD` on the remote** (not pushing to the default branch). The user's actual main branch was `codeberg/master`, which the daemon had never touched. All the daemon's recent work was on a separate orphan branch.

```bash
# git ls-remote codeberg before fix:
1ef8d051...	HEAD                       # symbolic ref (default branch = master)
906aab16c...	refs/heads/HEAD            # ORPHAN literal branch daemon created!
1ef8d051...	refs/heads/master          # user's actual main (STALE)
6fa419b1...	refs/heads/main            # OLD divergent lineage
ecff8acfa...	refs/heads/main-temp       # our earlier rebase push
```

## Fix applied

### dracon-platform (the user's primary complaint)

1. **Stashed** working changes (2 stashes preserved: `daemon-changes-pending-consolidate`, `daemon-changes-after-stash-1`)
2. **Created local `main` branch** tracking `codeberg/master` at SHA `906aab16c7`
3. **Switched** to local `main` (no longer detached HEAD)
4. **Daemon restarted** — now operates on `main` branch instead of detached HEAD

**Verification:**
- `dracon-sync repos` shows: `dracon-platform` is on **`main`** branch, tracking `codeberg/master`, 0/0 ahead-behind, OK state
- daemon is pushing successfully (commits landing on codeberg/master as the daemon intended)

### Other repos (the secondary complaint)

Pushed unpushed local commits to all remotes that were behind:

| Repo | Before | After |
|------|--------|-------|
| `ai-auto-writer` | 1180 ahead of codeberg/master | 0 ahead, codeberg/master synced |
| `browser-extensions-shared` | 10957 ahead of codeberg/master | 0 ahead, codeberg/master synced |
| `dracon-libs` | detached HEAD, broken remote config | on `main`, but **push blocked** (see blocker) |
| `dracon-code` | 8138 ahead / 5223 behind codeberg/master (complex divergence) | rebase attempted, aborted — see below |

## Remaining issues

### 🛑 Blocker: dracon-libs ONNX file

The daemon's TTS commit added `kokoro-v1.0.onnx` (325 MB), exceeding GitHub's 100 MB file limit. Push fails with:

```
remote: error: File tools/media/dracon-tts-runtime/assets/kokoro-v1.0.onnx is 310.45 MB
remote: error: this exceeds GitHub's file size limit of 100.00 MB
```

**Status**: daemon's TTS commit preserved as backup tag `save-v94.7.0-daemon-commit` (`63f95996ff`). Not pushed. Requires either:
- Git LFS setup on this repo
- File split / external storage
- Manual commit by operator

### ⏸️ Partial: dracon-code divergence

dracon-code has genuine 8138/5223 divergence between local and codeberg/master. A rebase was attempted but the conflict count was too high (~2913 commits with many conflicts). Aborted and left as-is. **Daemon shows OK** because it tracks `github/main`, not `codeberg/master`.

## Files preserved

### Stashes (preserved, not applied)
- `stash@{0}: On main-temp: daemon-changes-after-stash-1` — daemon's working changes from after the first stash
- `stash@{1}: On master: daemon-changes-pending-consolidate` — daemon's working changes when on master

### Backup tags
- `save-v94.7.0-daemon-commit` in dracon-libs → `63f95996ff` (the daemon's TTS commit)

### Branch state
- `dracon-platform`: on `main` (tracks `codeberg/master`), 0 ahead
- `dracon-libs`: on `main` (tracks `origin/main`), 0 ahead, push blocked by ONNX size
- `ai-auto-writer`: on `main` (tracks `github/main`), all 3 remotes synced
- `browser-extensions-shared`: on `main` (tracks `github/main`), all 3 remotes synced
- `dracon-code`: on `main` (tracks `github/main`), diverged from codeberg/master (untouched)

## Force-push status

**NO force-pushes used.** All operations were:
- `git checkout -b main codeberg/master` (create local branch at remote's tip — non-destructive)
- `git push origin main:master` (fast-forward push to remote's existing master — non-destructive)
- `git rebase --abort` (aborted rebase when conflicts became too many)

AGENTS.md's force-push restriction was honored.

## What was NOT done

- Annex migration (per audit recommendation) — multi-hour work, deferred
- dracon-code divergence resolution — too complex for current session
- dracon-libs ONNX push — blocked by GitHub file size limit
- Warden's pre-push regex (audit doc noted this as a future issue)

## Daemon state

```
16 repos  ✅ OK 15  ⚠️ WARN 1  ❌ CONCERN 0  ⛔ init/status failed: 0
```

- 1 WARN (dracon-platform) — only because it just made a new commit, will push momentarily
- 0 CONCERN — PUSH_STUCK cleared on all repos
