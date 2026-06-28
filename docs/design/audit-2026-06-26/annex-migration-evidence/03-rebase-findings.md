# Rebase Test Findings — 2026-06-28

## Summary

`git pull --rebase` (or `git rebase codeberg/main-temp`) WORKS to resolve the divergence. The only blocker is warden's smudge filter rewriting `web/games/.env.ovh` during checkout, which git sees as a working-tree modification and refuses to overwrite.

## Evidence

### Worktree rebase test 1 (detached HEAD)

- Started: ahead=3059, behind=1
- Rebase proceeded normally
- **21 of 3064 commits replayed successfully with ZERO conflicts**
- Then aborted at commit 22 because:
  ```
  error: Your local changes to the following files would be overwritten by merge:
      web/games/.env.ovh
  ```
- After 21 commits, ahead=20, behind=0, `merge-base --is-ancestor codeberg/main-temp HEAD` = YES

### Cause

`web/games/.env.ovh` is tracked by git with `filter=dracon` in `.gitattributes`. The committed blob is ENCRYPTED (warden's clean filter encrypts on commit). On checkout, warden's smudge filter DECRYPTS to plaintext. So:

- Committed blob = encrypted (e.g., `-----BEGIN DRACON ENCRYPTED-----...`)
- Working tree = plaintext (the empty placeholder body)
- During rebase, git's checkout tries to write the encrypted blob back, but the working tree already has plaintext → "would be overwritten"

### Fix

`git rebase --autostash` automatically stashes the decrypted `.env.ovh` before each pick, applies the encrypted blob, then re-applies the stash. This is the standard pattern for warden-tracked files.

## Recommendation

Use this exact sequence to resolve the divergence (no force-push needed):

```bash
cd /home/dracon/Dev/dracon-platform
git checkout main-temp
git rebase --autostash codeberg/main-temp
```

This replays local 3065 commits on top of codeberg's tip, autostashing warden's decrypted `.env.ovh` each step. Result: local main-temp becomes an ancestor of codeberg/main-temp + 3064 commits ahead. Push will then succeed with a normal `git push codeberg main-temp` (no force needed because we did NOT discard anything — we ADDED codeberg's commit to local's history).

## Size considerations

The 1.71 GB of unpushed binary objects will still need to transfer to codeberg. That's a separate concern from the divergence. Annex migration will resolve the size issue permanently.
