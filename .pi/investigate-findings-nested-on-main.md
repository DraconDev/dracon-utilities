# Investigation findings: nested-on-main migration

Date: 2026-07-02
Investigator: pi (continuation of goal mr3g843f-lajfpg)

## Files examined

1. `/home/dracon/Dev/dracon-utilities/dracon-sync/src/sync.rs` (lines 970-1100, 1102-1272)
2. `/home/dracon/Dev/dracon-utilities/dracon-sync/src/exclude.rs` (lines 1024-1224)
3. `/home/dracon/Dev/dracon-utilities/dracon-sync/src/git/discovery.rs` (lines 19-355, with focus on 50-180)
4. `/home/dracon/Dev/dracon-utilities/dracon-sync/src/exclude.rs` (lines 1186-1226 for is_gitlink_unchanged)
5. `/home/dracon/Dev/dracon-utilities/dracon-sync/src/git/discovery.rs` (lines 275-355 for is_nested_submodule_with_standalone)

## Current architecture summary

The daemon watches the **standalone** `/home/dracon/Dev/<name>/` worktrees for each
game/hegemon submodule of `dracon-platform`. The standalone and the nested
`dracon-platform/web/games/<wip|released>/<name>/` share one gitdir
(`<parent>/.git/modules/web-games-<name>/`).

**Daemon's gitlink-convergence algorithm** (per `daemon-standalone-removal-2026-07-01.md`):
1. After each cycle, daemon walks `.gitmodules` for submodules of each parent repo
2. For each submodule, reads shared gitdir's `refs/heads/main`
3. Compares against parent's tracked gitlink (`git ls-tree HEAD -- <path>`)
4. If they differ, calls `stage_gitlink_updates` which writes the new SHA via `git update-index --cacheinfo 160000,<sha>,<path>` in the parent

**Why the daemon can't currently use the nested path as the canonical worktree**:
- Git forbids two worktrees of the same gitdir to be on the same branch
- If we switch the nested path from detached to `main`, git refuses (because the standalone is on `main`)
- Therefore we must remove the standalone first, then switch the nested to `main`

## Functions to change (annotated)

### 1. `materialize_submodule` — `dracon-sync/src/sync.rs:1102`
**Current behavior**: Creates a separate worktree at `/Dev/<name>/` via `git worktree add` to the standalone target path.
**Required change**: This is the daemon's setup helper that RAN when the operator first set up submodules. After migration, this code path runs against the NESTED path (not the standalone). Concretely, the daemon should:
- Detect submodules of a parent (already done in `git/discovery.rs`)
- If a nested submodule checkout already exists (which it always does for the 10 game/hegemon repos), check it out on `main` and use IT as the watch path
- Skip creating the standalone at `/Dev/<name>/` entirely
**Specific change**: in `materialize_submodule`, accept a target_path that is INSIDE the parent; do `git checkout main` in that path instead of `git worktree add`. This is a behavior split:
- Old: `git worktree add --detach <target_path> <sha>`
- New (for nested): `cd <nested_path> && git checkout main` (refuses detached HEAD, must be main since this path is the canonical worktree)
**Backward compatibility**: keep the existing `git worktree add` path for any future submodule that's not yet nested (i.e., a fresh `git submodule add` case).

### 2. `shared_submodule_canonical_head_sha` — `dracon-sync/src/exclude.rs:1140`
**Current behavior**: Reads `refs/heads/main` from the shared gitdir — assumes the standalone is on `main`.
**Required change**: This function's behavior is already correct for nested-on-main architecture (since the nested path will be on `main` and commits on `main` advance `refs/heads/main`). No change needed, but the docstring/comments need updating to reflect that nested (not standalone) is now the worktree on main.
**Risk**: Confirmed no logic change required. The function only reads `refs/heads/main` from the shared gitdir's on-disk file, regardless of which worktree is "primary."

### 3. `is_gitlink_unchanged` — `dracon-sync/src/exclude.rs:1186`
**Current behavior**: Compares parent's tracked gitlink (`git ls-tree HEAD`) against:
   (a) Shared gitdir's `refs/heads/main` (canonical head; preferred)
   (b) Nested submodule's checkout HEAD (fallback)
Returns true if both match the parent's gitlink (signals "no gitlink update needed").
**Required change**: With nested-on-main, the nested submodule's HEAD == `refs/heads/main` (because nested-on-main reads its HEAD from main). The current logic returns `git rev-parse HEAD` which will return `refs/heads/main`'s SHA when on main. **No logic change needed**, but verify that the `git rev-parse HEAD` command actually returns the new SHA after a commit (it should, since commits on a worktree on main advance `refs/heads/main` and `git rev-parse HEAD` returns that).
**Verification needed**: a unit test that commits on a worktree on main, then verifies `git rev-parse HEAD` returns the new SHA.

### 4. `stage_gitlink_updates` — `dracon-sync/src/sync.rs:970`
**Current behavior**: Reads `shared_submodule_canonical_head_sha` and uses `git update-index --cacheinfo` to set the parent's gitlink to the canonical head.
**Required change**: **No logic change needed.** The function reads the shared gitdir's `refs/heads/main` directly, regardless of which worktree is on main. With nested-on-main, this continues to work — the nested commit advances `refs/heads/main`, and `stage_gitlink_updates` reads it correctly.

### 5. `is_nested_submodule_with_standalone` — `dracon-sync/src/git/discovery.rs:275`
**Current behavior**: SKIPS the nested submodule from the daemon's repos list because it duplicates a standalone row. Returns true when (a) the path is a worktree-style `.git` file, (b) the gitdir is under `<discovered_parent>/.git/modules/`, and (c) a standalone worktree exists at the watch root.
**Required change**: This filter is precisely the thing that prevents the daemon from watching the nested path. We need to **invert it** so that the nested path is preferred and the standalone is the duplicate.
**Specific change**: rewrite as `is_nested_submodule_with_no_standalone` or `is_duplicate_standalone`:
- Return true ONLY if there's ALSO a standalone at `/Dev/<name>/`
- Use this to filter STANDALONES out of the repos list, not the nested path
- For migrated games (no standalone), the nested path stays in the repos list

### 6. Submodule worktree candidate computation — `dracon-sync/src/git/discovery.rs:95-130`
**Current behavior**: For each parent's submodule, computes the candidate worktree path as `<watch_root>/<submodule_basename>/` (e.g. `/Dev/polis`). Pushes this to `repos` so the daemon materializes the standalone there.
**Required change**: After migration, the nested path IS the worktree. The candidate path computation must change:
- If the submodule's nested path already exists (which it does for all 10 game/hegemon repos), use the nested path directly
- Skip creating the standalone candidate entirely
- For NEW submodules (truly fresh), fall back to a standalone at `/Dev/<name>/` for backward compat
**Specific change**: in the submodule-candidate loop, FIRST check if the nested path exists and has a `.git` file (gitfile); if so, use that path as the candidate. ELSE fall back to the standalone path.

### 7. `sync-status.json` watch list — operator-facing
**Current entries**: `/Dev/polis`, `/Dev/hellhunter`, etc.
**Required change**: Replace with `/Dev/dracon-platform/web/games/wip/polis`, `/Dev/dracon-platform/web/games/wip/hellhunter`, etc.
**Caution**: Watch-list changes don't require daemon code; just update the JSON. But removing the old `/Dev/<name>/` paths requires `git worktree remove /Dev/<name>` per game.

## Files NOT requiring changes

- `dracon-sync/src/main.rs` — module registration unchanged
- `dracon-sync/src/policy.rs` — policy parsing unchanged
- `dracon-sync/src/git/staging.rs` — file staging logic unchanged (the daemon operates on whatever repo path is in the watch list)
- `dracon-sync/src/git/push.rs` — push logic unchanged (it operates on whatever repo path is passed)

## Migration steps per game (e.g. junk-runner, the pilot)

1. **Pre-flight**: confirm `junk-runner` is in /Dev/dracon-platform/.git/modules/web-games-junk-runner (yes)
2. **Pre-flight**: confirm nested path exists (/Dev/dracon-platform/web/games/wip/junk-runner/) (yes)
3. **Save the watch-list entry** for recovery
4. **Remove the standalone** worktree:
   - `git -C /Dev/junk-runner worktree remove /Dev/junk-runner --force`
   - OR `git worktree remove --force /Dev/junk-runner` from any worktree of the same gitdir
5. **Switch the nested to main**:
   - `git -C /Dev/dracon-platform/web/games/wip/junk-runner checkout main`
6. **Update sync-status.json**: replace `/Dev/junk-runner` with `/Dev/dracon-platform/web/games/wip/junk-runner`
7. **Test**:
   - Touch a file in /Dev/dracon-platform/web/games/wip/junk-runner/foo.txt
   - Wait 60 seconds
   - Verify `git ls-remote github refs/heads/main | head -c 12` shows a new SHA (vs the old a-after-pilot)
   - Verify the parent (`dracon-platform`) gitlink advanced: `git ls-tree HEAD web/games/wip/junk-runner` shows the new SHA
   - Verify /Dev/dracon-platform has the new commit visible in journalctl
8. **Verify**:
   - cargo build --release --locked
   - cargo test --workspace --locked
   - daemon journalctl --since "1 hour ago" | grep junk-runner
   - dracon-sync repos shows junk-runner as ROLE=submod (not standalone)

## Risks

1. **Gitlink cache**: when we remove the standalone, git may need to refresh its index for the nested. The shared gitdir's `index` (and the nested worktree's `index`) stay in sync because git worktrees share objects but have separate indexes. Removing a worktree does not invalidate the remaining worktree's index. **Confirmed safe.**
2. **Daemon's `worktree_name_map`**: the daemon maps `polis` (nested basename) to `web-games-polis` for URL purposes. **Need to check** if the URL mapping still works when the watch-list path is the nested path (since the nested path's basename is `polis`, not `web-games-polis`). The daemon's `multi_remote` module has a `repo_name_map` that handles this.
3. **Daemon `discover_git_repos`** walks the watch roots (defined by operator config). If `/Dev/dracon-platform` is in the watch roots, the nested submodule paths will be auto-discovered. The current logic SKIPS them as duplicates; we need to invert that.

## Next steps

Hand off to the design task. The design doc should formalize:
- Specific code changes (functions, line ranges, pre/post conditions)
- Migration runbook (per-game steps, with rollback)
- Tests to add (unit + integration)
- Monitoring hooks (what to watch after each migration)
- Rollback procedure (how to re-create the standalone if nested-on-main fails)

## Files summary table

| File | Lines | Function | Change Type |
|------|-------|----------|------------|
| `sync.rs` | 970-1064 | `stage_gitlink_updates` | NO CHANGE (still reads refs/heads/main) |
| `sync.rs` | 1102-1272 | `materialize_submodule` | MODIFY: optional branch for nested-path target |
| `exclude.rs` | 1140-1182 | `shared_submodule_canonical_head_sha` | NO CHANGE (still reads refs/heads/main) |
| `exclude.rs` | 1186-1224 | `is_gitlink_unchanged` | NO CHANGE (git rev-parse HEAD works on main worktree) |
| `discovery.rs` | 50-78 | dedup-pass with `is_nested_submodule_with_standalone` | MODIFY: invert to filter standalones, not nested |
| `discovery.rs` | 95-130 | submodule candidate computation | MODIFY: prefer nested if it exists |
| `discovery.rs` | 275-355 | `is_nested_submodule_with_standalone` | MODIFY: rename/rewrite to `is_duplicate_standalone` |
| `sync-status.json` | operator-controlled | watch-list entries | MODIFY: nested paths replace `/Dev/<name>` paths |
