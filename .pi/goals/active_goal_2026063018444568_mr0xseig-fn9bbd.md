{
  "version": 3,
  "id": "mr0xseig-fn9bbd",
  "objective": "Implement the parent-gitlink propagation fix in `dracon-sync`: when the daemon sees a modified-tracked entry whose path is a gitlink (mode 160000 in `git ls-tree HEAD <path>`), stage it WITHOUT recursion into the submodule working tree (run `git add <path>` instead of `git add -A -- <path>`) so the parent records the new submodule SHA in its index. After this fix, a parent with a stale submodule pointer is committed automatically in the next daemon cycle, instead of showing `MOD=1` indefinitely.",
  "status": "active",
  "autoContinue": true,
  "usage": {
    "tokensUsed": 70065,
    "activeSeconds": 131
  },
  "sisyphus": false,
  "createdAt": "2026-06-30T17:44:45.688Z",
  "updatedAt": "2026-06-30T17:47:14.331Z",
  "activePath": ".pi/goals/active_goal_2026063018444568_mr0xseig-fn9bbd.md",
  "taskList": {
    "tasks": [
      {
        "id": "fix-1",
        "title": "Add `is_gitlink()` helper in `dracon-sync/src/exclude.rs`",
        "status": "complete",
        "completedAt": "2026-06-30T17:45:57.539Z",
        "evidence": "fix-1 complete. Helper `is_gitlink(repo, path) -> bool` added to src/exclude.rs. 4 unit tests cover the four path shapes (tracked gitlink, regular file, untracked sibling subrepo, missing path). All 4",
        "verificationContract": "Public test confirming `is_gitlink(repo, \"submod\")` returns true when the path is a 160000 entry in `git ls-tree HEAD <path>` and false otherwise (regular tracked file, untracked dir, missing path).",
        "subtasks": [
          {
            "id": "fix-1a",
            "title": "Write unit test fixtures (parent + tracked gitlink + untracked dir)",
            "status": "complete",
            "completedAt": "2026-06-30T17:45:49.593Z",
            "evidence": "4 unit tests added in src/exclude.rs::tests covering: (1) `test_is_gitlink_returns_true_for_tracked_gitlink` — creates parent + submod, registers submodule via `git add submod` (creates 160000 entry),"
          },
          {
            "id": "fix-1b",
            "title": "Implement `pub(crate) fn is_gitlink(repo, path)` returning `bool` based on `git ls-tree HEAD` output prefix `160000`",
            "status": "complete",
            "completedAt": "2026-06-30T17:45:53.833Z",
            "evidence": "Implemented `pub(crate) fn is_gitlink(repo: &Path, path: &Path) -> bool` in src/exclude.rs at line ~610 (right above `is_gitlink_unchanged`). The function runs `git ls-tree HEAD -- <path>` and returns"
          }
        ]
      },
      {
        "id": "fix-2",
        "title": "Partition staged paths in `sync_repo` into `gitlink_paths` + `regular_paths`",
        "status": "pending",
        "verificationContract": "Code change at `sync_repo` so the `to_stage` list is split into two: paths whose `is_gitlink()` returns true go to `gitlink_paths`; the rest go to `regular_paths` (existing path).",
        "subtasks": [
          {
            "id": "fix-2a",
            "title": "Split `to_stage` partition in `sync_repo_with_ahead_since` based on `is_gitlink()`",
            "status": "pending"
          },
          {
            "id": "fix-2b",
            "title": "Pass the two partitions through `stage_commit_and_push` to a new `stage_existing_files` call that distinguishes gitlink-add from regular-add",
            "status": "pending"
          }
        ]
      },
      {
        "id": "fix-3",
        "title": "Extend `stage_existing_files` to emit `git add <path>` (no `-A`) for gitlink entries",
        "status": "pending",
        "verificationContract": "Code change so a gitlink path is added to the index without recursion. The internal `expanded.push(p)` at the `.git exists()` branch should now push the bare path (kept in the gitlink-partition), and the `git add` invocation for that partition must drop `-A` so git treats it as a pointer update.",
        "subtasks": [
          {
            "id": "fix-3a",
            "title": "Make the top-level and inner-recursion submodule-skip branches emit the path as a gitlink-staging entry instead of dropping it",
            "status": "pending"
          },
          {
            "id": "fix-3b",
            "title": "Adjust the `git add` invocation: split into `git add <gitlink>` (non-recursive) and `git add -A -- <regular>` (recursive), keeping `.gitignore` partition logic intact",
            "status": "pending"
          }
        ]
      },
      {
        "id": "fix-4",
        "title": "Add regression test for the parent-gitlink propagation case",
        "status": "pending",
        "verificationContract": "Test that creates a parent repo with a tracked gitlink, advances the inner submodule HEAD, calls `sync_repo` or `stage_existing_files` and asserts the parent's index now points to the new submodule SHA.",
        "subtasks": [
          {
            "id": "fix-4a",
            "title": "Create test that mimics `web-auto/rust-ai-web-auto` scenario (real `.git/` directory, not `.gitmodules`-declared submodule) and asserts parent index gets the new SHA after sync",
            "status": "pending"
          },
          {
            "id": "fix-4b",
            "title": "Create second test for the `.gitmodules`-declared case (worktree shared with parent) and assert the same behavior",
            "status": "pending"
          }
        ]
      },
      {
        "id": "fix-5",
        "title": "Build + run full test suite + end-to-end on web-auto",
        "status": "pending",
        "verificationContract": "`cargo build --release --locked` → 0 errors, no new warnings. `cargo test --locked` → previous count +2 new tests, 0 failures. `dracon-sync sync-now /home/dracon/Dev/web-auto` produces a new parent commit that updates the `rust-ai-web-auto` gitlink to its current submodule HEAD. `git ls-remote github refs/heads/main` for web-auto shows the new commit.",
        "subtasks": [
          {
            "id": "fix-5a",
            "title": "Build release + cargo test (expect 645+ tests passing, 0 fail)",
            "status": "pending"
          },
          {
            "id": "fix-5b",
            "title": "Replace `/home/dracon/.local/bin/dracon-sync`, restart daemon, run sync-now on web-auto, verify gitlink commit + push to all 3 remotes",
            "status": "pending"
          }
        ]
      }
    ],
    "blockCompletion": false,
    "proposedAt": "2026-06-30T17:44:45.689Z"
  }
}

# Goal Prompt

Implement the parent-gitlink propagation fix in `dracon-sync`: when the daemon sees a modified-tracked entry whose path is a gitlink (mode 160000 in `git ls-tree HEAD <path>`), stage it WITHOUT recursion into the submodule working tree (run `git add <path>` instead of `git add -A -- <path>`) so the parent records the new submodule SHA in its index. After this fix, a parent with a stale submodule pointer is committed automatically in the next daemon cycle, instead of showing `MOD=1` indefinitely.

## Progress

- Status: running
- Auto-continue: on
- Sisyphus mode: no
- Time spent: 2m11s
- Tokens used: 70K (70,065) tokens
## Tasks

<!-- blockCompletion: false -->
- [x] fix-1: Add `is_gitlink()` helper in `dracon-sync/src/exclude.rs` — evidence: fix-1 complete. Helper `is_gitlink(repo, path) -> bool` added to src/exclude.rs. 4 unit tests cover the four path shapes (tracked gitlink, regular file, untracked sibling subrepo, missing path). All 4
- [ ] fix-2: Partition staged paths in `sync_repo` into `gitlink_paths` + `regular_paths` — contract: Code change at `sync_repo` so the `to_stage` list is split into two: paths whose `is_gitlink()` returns true go to `gitlink_paths`; the rest go to `regular_paths` (existing path).
- [ ] fix-3: Extend `stage_existing_files` to emit `git add <path>` (no `-A`) for gitlink entries — contract: Code change so a gitlink path is added to the index without recursion. The internal `expanded.push(p)` at the `.git exists()` branch should now push the bare path (kept in the gitlink-partition), and the `git add` invocation for that partition must drop `-A` so git treats it as a pointer update.
- [ ] fix-4: Add regression test for the parent-gitlink propagation case — contract: Test that creates a parent repo with a tracked gitlink, advances the inner submodule HEAD, calls `sync_repo` or `stage_existing_files` and asserts the parent's index now points to the new submodule SHA.
- [ ] fix-5: Build + run full test suite + end-to-end on web-auto — contract: `cargo build --release --locked` → 0 errors, no new warnings. `cargo test --locked` → previous count +2 new tests, 0 failures. `dracon-sync sync-now /home/dracon/Dev/web-auto` produces a new parent commit that updates the `rust-ai-web-auto` gitlink to its current submodule HEAD. `git ls-remote github refs/heads/main` for web-auto shows the new commit.

