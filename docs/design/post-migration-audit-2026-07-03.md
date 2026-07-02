# POST-MIGRATION AUDIT (2026-07-03 00:55 UTC)

This is a status audit of the 10 game/hegemon submodules after the bulk
migration completed 2026-07-02 17:40 UTC (goal `354fe3cb-9d0f-4d15-aa6e-fb2e0c55f918`).
The user requested this audit via the goal `mr45llyf-y7ksfx` ("ok lets run
an audit") at 2026-07-02 23:46 UTC.

## Summary verdict

**9 of 10 games are in the expected post-migration steady state. 1 game
(hegemon) has REGRESSED in the past 12 hours** — `/Dev/hegemon` standalone
was re-materialized, the nested submodule path's HEAD file was overwritten
from `ref: refs/heads/main` to a raw SHA, and the daemon's push to github
is failing with "destination is not a full refname".

This is a structural regression of the fix deployed in goal
`mr3g843f-lajfpg` (2026-07-02). Two daemon bugs regressed at the same
time, both affecting hegemon specifically:

1. `is_on_main_branch` returns false for hegemon's nested submodule path
   because the shared gitdir's HEAD is a SHA (not `ref: refs/heads/main`).
   This causes `materialize_pending_submodules` to re-create `/Dev/hegemon`.

2. `git push` to github is failing with "destination is not a full refname"
   because the worktree-style HEAD is detached and the `HEAD:refs/heads/main`
   refspec fix from goal `mr3g843f-lajfpg` is not being applied correctly
   for this case.

## Section 1: 10 game/hegemon submodules

| Game | Standalone | Branch | Local SHA | IN-SYNC | State |
|------|-----------|--------|-----------|---------|-------|
| polis | GONE ✓ | main | 4361d81 | 4/4 | ✓ healthy |
| darklord | GONE ✓ | main | f045d08 | 4/4 | ✓ healthy |
| neonbreak | GONE ✓ | main | 74c5f26 | 3/4 | ✓ healthy (gitlab/codeberg behind) |
| hellhunter | GONE ✓ | main | 17ff635 | 4/4 | ✓ healthy |
| **hegemon** | **EXISTS ✗** | **detached✓** | **0ad7a7d** | **3/4** | **✗ REGRESSED** |
| one-mil-girls | GONE ✓ | main | 2f5038a | 4/4 | ✓ healthy |
| capture-anime-girls | GONE ✓ | main | 953bcb0 | 4/4 | ✓ healthy |
| endless-td | GONE ✓ | main | 05ac9e6 | 4/4 | ✓ healthy |
| deathrun | GONE ✓ | main | 16ac315 | 4/4 | ✓ healthy |
| junk-runner | GONE ✓ | main | 0cf92b0 | 4/4 | ✓ healthy |

Notes on the "branch" column for hegemon: `git branch --show-current`
returns empty (detached HEAD), but `git rev-parse HEAD` matches
`git rev-parse refs/heads/main`. The daemon sees this as "branch: HEAD"
because git's HEAD is a SHA.

## Section 2: 16 other watched repos

| Repo | State | Branch | IN-SYNC |
|------|-------|--------|---------|
| dracon-platform | WARN | main | 4/4 |
| dracon-utilities | WARN | main | 4/4 |
| browser-extensions-shared | OK | main | 4/4 |
| ai-auto-writer | OK | main | 4/4 |
| dracon-sync | OK | main | 4/4 |
| web-auto | OK | main | 3/4 (gitlab behind, pre-existing) |
| avid | OK | main | 4/4 |
| rust-ai-web-auto | OK | main | 4/4 |
| .dracon | OK | main | 4/4 |
| dracon-system | OK | main | 4/4 |
| pully-fully-pull-based-fleet-reconciler | OK | main | 4/4 |
| pi-plugins | OK | main | 4/4 |
| dracon-code | OK | main | 4/4 |
| dracon-strategy | OK | main | 4/4 |
| DraconDev | OK | main | 2/4 (gitlab+codeberg behind master→main rename, pre-existing) |
| dracon-warden | OK | main | 4/4 |

## Section 3: hegemon regression root cause

### Timeline (from daemon log + reflog)

| Time (BST) | Event |
|-----------|-------|
| 2026-07-02 17:40 | Bulk migration complete. hegemon nested on `main`, `/Dev/hegemon` GONE |
| 2026-07-02 ~21:00 | Investigation of github 2GB limit (goal `mr3wg8q0-m71lhj`) — test commit added then removed |
| 2026-07-03 00:43:55 | **`git checkout 07d1d70`** on nested submodule — detached HEAD |
| 2026-07-03 00:46:21 | **Daemon: `Materializing submodule web-games-hegemon -> /home/dracon/Dev/hegemon`** |
| 2026-07-03 00:46:43 | **Daemon: github push fails with `error: The destination you provided is not a full refname`** |
| 2026-07-03 00:49:17 | **Daemon: `pull --no-rebase origin HEAD: Fast-forward`** — overwrites shared HEAD to SHA |
| 2026-07-03 00:49:25 | Daemon: github push fails again (same error) |
| 2026-07-03 00:53:07 | Daemon: github push fails again |
| 2026-07-03 00:54:24 | Daemon: still trying |

The Unix timestamps on the reflog confirm this sequence:
```
0ad7a7d HEAD@{0}: pull --no-rebase origin HEAD: Fast-forward
0ad7a7d HEAD@{1}: checkout: moving from main to 07d1d705614311e75e05cbfc7a1a8e111a3c32c8
```
The `checkout` from `main` to a raw SHA is what corrupted the shared HEAD
file from `ref: refs/heads/main` to a SHA. Subsequent commits went onto
the detached HEAD instead of advancing `refs/heads/main`.

### Why is_on_main_branch returns false

```rust
// daemon.rs:2043 (current code)
fn is_on_main_branch(path: &Path) -> bool {
    let dot_git = path.join(".git");
    if !dot_git.exists() { return false; }
    
    let head_path = if dot_git.is_file() {
        // .git is a file (worktree-style)
        let content = std::fs::read_to_string(&dot_git).unwrap();
        let gitdir_rel = content.trim().strip_prefix("gitdir:").unwrap().trim();
        let resolved = path.canonicalize().unwrap().join(gitdir_rel);
        resolved.canonicalize().unwrap().join("HEAD")
    } else {
        dot_git.canonicalize().unwrap().join("HEAD")
    };
    
    let head_content = std::fs::read_to_string(&head_path).unwrap();
    head_content.starts_with("ref: refs/heads/")
}
```

For hegemon's nested submodule path:
- `.git` file → `gitdir: ../../../../.git/modules/web-games-hegemon`
- Resolved to `/home/dracon/Dev/dracon-platform/.git/modules/web-games-hegemon`
- This is the **shared gitdir root**, NOT a `<shared>/worktrees/<X>` subdir
- `HEAD` file at this location currently contains `0ad7a7dbda03f67b07060b70161f27ee5edc43ca` (a SHA)

So the function correctly reports "detached" — the bug is not in
`is_on_main_branch` itself, it's in the **invariant** it relies on.

The invariant was: "if the nested submodule is on `main`, the shared
gitdir's HEAD will be `ref: refs/heads/main`". This holds for polis,
darklord, etc. (verified). But it does NOT hold for hegemon because
**someone ran `git checkout <sha>` on the nested submodule path**, which
detached the shared HEAD file.

The proper fix would be: if `is_on_main_branch` returns false AND the
nested path is a "naked" gitdir pointer (not a worktree subdir), the
daemon should **rewind the shared HEAD** to `ref: refs/heads/main` rather
than materialize a redundant standalone worktree.

### Why github push fails with "destination is not a full refname"

The fix in goal `mr3g843f-lajfpg` was in `git/push.rs`: use
`HEAD:refs/heads/main` refspec when the worktree is detached, instead of
unqualified `HEAD`. This is supposed to handle the detached case.

But the daemon log shows the bug is back: `error: The destination you
provided is not a full refname`. This suggests `current_branch` (called
inside `push_with_transport_fallbacks` to determine the refspec) is
returning the empty string or "HEAD" for hegemon — and the refspec
construction is producing `HEAD:refs/heads/HEAD` or `:refs/heads/main`,
both of which are invalid.

The bug path:
1. `current_branch` reads `/home/dracon/Dev/dracon-platform/.git/modules/web-games-hegemon/HEAD`
2. That file contains a SHA, not a ref
3. `current_branch` returns `None` (or "HEAD" after the literal-HEAD filter)
4. `push.rs` builds `HEAD:refs/heads/main` refspec
5. github rejects it because of the worktree-style gitdir (the refspec
   is computed against the shared gitdir, not the worktree's local HEAD)

This is exactly the regression of the original bug from
`mr3g843f-lajfpg`.

## Section 4: What this audit reveals

The audit shows that the **bulk migration was structurally complete** at
17:40 UTC on 2026-07-02, but the fix is **fragile** in the face of:

1. User activity that detaches the nested submodule HEAD
2. Daemon push retries on the github 2GB limit
3. The shared gitdir's HEAD file being read from the wrong location

A more robust fix would:
1. Make `is_on_main_branch` detect the "naked gitdir pointer" case and
   rewind HEAD to `ref: refs/heads/main` if needed
2. Make `current_branch` correctly handle the "naked gitdir pointer" by
   always reading from `<shared_gitdir>/HEAD` and treating detached as
   a fallback to `refs/heads/main` if that ref exists at the same SHA

## Recommended next steps (operator decision)

1. **Stop the daemon** to prevent further commits on the detached hegemon HEAD
2. **Reset hegemon's shared HEAD** to `ref: refs/heads/main` (and remove
   `/Dev/hegemon` standalone worktree again)
3. **Re-deploy the daemon fix** for the detached-HEAD case to prevent
   this from happening again with other games

This is a **read-only audit** — no remediation performed.