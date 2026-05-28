# Dracon Utilities Audit — 2026-05-28

## Status: IN PROGRESS

## Recently Completed

### ✅ F1: New Branch Auto-Push (sync.rs)
**Problem:** New local branches with no upstream tracking were never pushed. `ahead == 0` blocked the push, and `auto_pull_merge` didn't detect new branches since there's no tracking ref to compare against.

**Fix:** Three changes in `sync.rs`:
1. `push_with_blob_check` — pushes even when `ahead == 0` if the branch has no upstream tracking
2. `handle_ahead_push` — same logic applied after auto_pull_merge
3. `filter_only_cleared` handling — correctly return NothingToDo when all changes are filtered out by clean/smudge so daemon applies cooldown instead of trying to commit

**Files changed:** `dracon-sync/src/sync.rs`
**Build:** ✅ passes `cargo build --release`
**Installed:** ✅ daemon restarted with new binary
**Tested:** Created `test-auto-push` branch → the sync daemon picked it up and pushed it to all 4 remotes (origin, github, codeberg, gitlab) on the next cycle.

---

## Audit Tasklist

### Category: Core Functionality

---

#### ✅ F1 (COMPLETED — see above)

---

#### F2: `auto_pull_merge` and new branch tracking
**Finding: NOT A BUG — works as designed.**
`auto_pull_merge` at `sync.rs:279` requires ALL FOUR conditions:
```rust
if policy.auto_pull && ctx.has_origin && ctx.has_upstream && initial_status.behind > 0 ...
```
A brand-new branch has `has_upstream = false` (no `@{upstream}`), so `auto_pull_merge` correctly **skips** — it doesn't error, it just does nothing for that branch. This is the right behavior because there's no remote ref to pull from. The push happens via `handle_ahead_push` / `push_with_blob_check` instead, which DID push the new branch.

**When `has_upstream` DOES become `true`:** After `push_with_blob_check` pushes `HEAD:refs/heads/<branch>` to origin, git automatically creates `refs/remotes/origin/<branch>` tracking ref. On the NEXT sync cycle, `has_tracking_upstream()` returns `true` and the branch participates in pull/push cycles normally.

**No action needed.**

---

#### F3: `push_with_retries` and mirror failure handling
**Finding: Each mirror is tried independently, but `push_with_retries` is for origin, not mirrors.**

- `push_with_retries` (`git/push.rs:126`) retries the same remote up to `retries` times with backoff. It uses `git push origin HEAD`.
- It does NOT use `push_mirror_remotes`. Mirrors are managed separately.
- `push_mirror_remotes` (`git/multi_remote.rs:58`) calls `push_to_all_remotes`, which iterates each mirror sequentially, trying each one once, with retries per-mirror via `push_to_named_remote`.
- `push_to_named_remote` retries SSH attempt, then HTTPS fallback, then SSH retry. All mirrors in the `remotes` config are attempted in priority order.
- If mirror N fails, mirror N+1 is still attempted.
- If ALL mirrors fail, `remote_failures` map is populated per-mirror, but no overall error is returned from `push_mirror_remotes` — the function continues and records results.

**BUG:** `push_mirror_remotes` returns `Vec<(String, Result)>` — a list of per-mirror results. The caller (`stage_commit_and_push`) calls `push_with_blob_check` which only pushes origin. Mirror pushes happen separately from `push_to_named_remote` inside commit pipeline? Need to verify.

Let me trace the actual call path for mirror pushes:

1. `stage_commit_and_push` → calls `push_with_blob_check(ctx, 1)` at line 1287
2. `push_with_blob_check` (sync.rs:985) → calls `push_mirror_remotes` inside `push_repo_to_all_remotes`? Or routes through origin push only?

Actually, looking at `push_with_blob_check`, it calls `push_with_retries` for origin with `ctx.push_op_timeout_secs` and `ctx.retries`. The mirror multi-remote push appears to be called via `push_repo_to_all_remotes` somewhere else in the pipeline. I need to check what `push_with_retries` actually does — it calls `git push origin HEAD`, not `push_mirror_remotes`. This means mirrors are NOT pushed as part of post-commit push. This is a significant finding.

Let me check one more thing: how does `push_mirror_remotes` get invoked.
