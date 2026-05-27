# TODO

## Active Work

- [ ] Rewrite `todo_context_message` to scan diff for task transitions instead of reading root `todo.md`
  - [ ] Replace `parse_todo_task(repo)` with a function that runs `git diff --cached -U0` on the repo and scans hunks for `[ ]` → `[x]` / `[~]` state changes
  - [ ] Detect transitions: line was `- - [ ]` before, now `+ - [x]` or `+ - [~]` — extract the task text from the new line
  - [ ] Subject: always use `deterministic_diff_summary(diff_names)` — no `close(todo):` prefix, always reliable
  - [ ] Body: append `Task transitions:` block when any transitions found, with file:line reference
  - [ ] Fallback: when no transitions detected, commmit body is just the file list (same as today's fallback)
- [ ] Update all scribe tests for the new format
  - [ ] `test_todo_context_with_task` → test that transitions appear in body when diff contains `[ ]` → `[x]`
  - [ ] `test_todo_context_falls_back_when_no_open_task` → test that no `Task transitions:` block when no transitions
  - [ ] `test_todo_context_falls_back_when_no_todo_file` → still falls back to pure diff
  - [ ] `test_todo_context_task_text_with_files` → test mixed transitions (`[x]` + `[~]`)
  - [ ] New test: transition diff contains `[ ]` → `[~]` for in-progress tasks
- [ ] Install and verify in production
  - [ ] Build release binary, stop daemon, copy, start daemon
  - [ ] Create a test commit with `[x]` transitions, verify body shows task transitions
  - [ ] Create a test commit with no transitions, verify pure diff fallback
