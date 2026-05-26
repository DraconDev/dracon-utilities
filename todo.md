# TODO

## Active Work

- [x] Fix commit message generation — all commits say "update main" instead of "sync: N checked"
  - `todo_commit_messages = true` is set in policy but daemon produces generic messages
  - Root cause: `is_noise_only` logic conflates version bumping with commit message selection
  - `deterministic_decide_bump_level` returns `BumpLevel::None` for most changes → `is_noise_only = true`
  - When `is_noise_only` is true, code uses `local_fallback` but the actual output is "update main", not "sync: N checked"
  - Fix: call `todo_context_message` directly when `todo_commit_messages` is true, before noise detection
  - ✅ Fixed: rewrote `todo_context_message` to return task text as subject + diff summary as body
  - ✅ Fixed: updated `sync.rs` message selection to use task text directly without `category(scope):` prefix
  - ✅ Fixed: renamed `is_noise_only` to `noise_for_bump` to clarify it only affects version bumping
  - Expected: commits like `chore(sync): sync: 2 checked\n\n{...}`

## Completed

- [x] Add `todo_commit_messages` config toggle
- [x] Implement `todo_context_message()` — routing key title + JSON body
- [x] Add `parse_todo_task()` — reads root todo.md, finds first open `[ ]` task
- [x] Add `local_fallback_message()` — file-stem summary fallback
- [x] Wire into `sync.rs` commit path
- [x] Add 13 tests for todo_parser, 11 tests for scribe
- [x] Add auto_create for GitHub, GitLab, Codeberg
- [x] Exclude `.dracon` and `.ralph` from `exclude_dir_names`
- [x] Add binary freshness check to `install.sh`
- [x] Full audit of 28 repos — all OK, 0 WARN, 0 CONCERN
- [x] Archive 5 redundant markdown files
- [x] Fix broken remotes on `dracon-voice-notifications` and `ai-vid-editor`
- [x] Delete ghost repos
