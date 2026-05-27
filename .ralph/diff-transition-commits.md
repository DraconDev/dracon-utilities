Rewrite `todo_context_message` to scan git diff for task state transitions (`[ ]` → `[x]` / `[~]`) instead of reading root `todo.md`.

## Goals
1. `scan_diff_for_transitions(repo)` — runs `git diff --cached -U0`, scans hunks for `- [ ]` → `+ [x]` or `+ [~]` lines
2. `todo_context_message` uses diff transitions instead of `parse_todo_task`
3. Subject: always `deterministic_diff_summary(diff_names)` — no `close(todo):` prefix
4. Body: `Task transitions:` block when transitions found, else just file list

## Checklist
- [x] Write `scan_diff_for_transitions(repo)` function
- [x] Rewrite `todo_context_message` to use transitions
- [x] Update tests (task transitions, no transitions, mixed, fallback)
- [x] All 479+ tests pass
- [x] Install binary, test in production
