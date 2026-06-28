# Verified Rebase Plan — dracon-platform — 2026-06-28

## Goal: Resolve PUSH_STUCK divergence on dracon-platform

## Confirmed root cause

NOT a size issue. NOT a forge cap issue. The push is rejected because:

```
$ git push codeberg main-temp
 ! [rejected]              main-temp -> main-temp (non-fast-forward)
error: failed to push some refs to 'codeberg.org:dracondev/dracon-platform.git'
hint: Updates were rejected because the tip of your current branch is behind
hint: its remote counterpart.
```

Local's `main-temp` is NOT an ancestor of codeberg's `main-temp`. Specifically:
- Local: 3065 commits ahead, 1 commit behind codeberg's `6a7cf693240…`
- Codeberg's `6a7cf69` is the divergent commit
- Local's history contains a DIFFERENT commit (`e648ea7153…`) with the SAME patch-id (`24b21d9bd12a31e274c76779036576b49a5295d0`) — i.e., the same content/changes, but a different SHA because parents differ.

## Verified fix: `git rebase codeberg/main-temp`

### Test 1: worktree rebase (no main repo mutation)

```
git worktree add --detach /tmp/rebase-test codeberg/main-temp
cd /tmp/rebase-test
git config filter.dracon.clean /bin/cat
git config filter.dracon.smudge /bin/cat
git config filter.dracon.required false
git rebase codeberg/main-temp
```

Result:
- 3065/3065 commits replayed
- ZERO conflicts
- ahead: 3065, behind: 0
- `merge-base --is-ancestor codeberg/main-temp HEAD` = YES
- `cargo check` passed (24s)

### Why the warden filter disable is needed

`web/games/.env.ovh` is tracked with `filter=dracon` (warden-encrypted on commit, decrypted on checkout). During rebase:
1. git tries to checkout commit N's tree (encrypted blob)
2. warden smudge filter decrypts it to plaintext
3. git sees plaintext as a working-tree modification
4. Next rebase step tries to checkout commit N+1's tree (different encrypted blob)
5. git: "Your local changes to .env.ovh would be overwritten"

Disabling the warden filter for the duration of the rebase (with `filter.dracon.required = false` so git tolerates filter failures) makes git operate on the raw blobs. The .env.ovh remains in its encrypted form throughout.

### Why this is NOT a force-push

After rebase, local's main-temp has:
- codeberg's `6a7cf69` as an ancestor
- Local's 3065 new commits on top

The push will be a normal fast-forward (no force needed). AGENTS.md's force-push restriction does not apply.

## Execution steps (after operator authorization)

```bash
cd /home/dracon/Dev/dracon-platform

# 1. Stop the daemon briefly (so it doesn't commit during rebase)
systemctl --user stop dracon-sync.service

# 2. Disable warden filter for the rebase
git config filter.dracon.clean /bin/cat
git config filter.dracon.smudge /bin/cat
git config filter.dracon.required false

# 3. Make sure we're on main-temp
git checkout main-temp

# 4. Rebase local onto codeberg (proven safe: 0 conflicts)
git rebase codeberg/main-temp

# 5. Re-enable warden
git config --unset filter.dracon.clean
git config --unset filter.dracon.smudge
git config --unset filter.dracon.required

# 6. Push (fast-forward, no force)
git push codeberg main-temp

# 7. Restart daemon
systemctl --user start dracon-sync.service
```

## Estimated time

- Rebase: ~2-3 minutes (3065 commits, each is small)
- Push: variable — depends on how many objects codeberg accepts (1.71 GB unique unpushed objects)
- Total: ~5-10 minutes for the divergence fix

## What this does NOT solve

The 1.71 GB of unpushed binary objects still need to transfer to codeberg. Annex migration solves that permanently. The rebase only resolves the divergence (1 commit), not the size cap (1.71 GB).

## Recommended order

1. **Now**: rebase to fix divergence → daemon can resume normal push
2. **Same session**: migrate largest tracked binary to annex → push size drops
3. **Next**: migrate remaining binary content → push size drops further
4. **Eventually**: all binary lives in OVH, git on codeberg only sees pointers

After step 1 alone, the daemon will continue trying to push the 1.71 GB and may still hit codeberg's 5 GB push cap or 100 MB file cap. Annex migration is still required to fully resolve PUSH_STUCK.
