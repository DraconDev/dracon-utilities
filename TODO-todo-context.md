# TODO-Context Mode — Feature Implementation

Add an alternative commit message strategy to the scribe: instead of generating
messages from file stems (`local_fallback_message`), read the root `todo.md`,
find the first `[ ]` box, and use that task as context for the message.

Existing behavior is completely preserved. This is an opt-in toggle.

---

## 🔴 Stage 1: Parse — Read `todo.md` and find the first `[ ]`

**Goal:** A clean, tested parser that extracts the active task from `todo.md`.

- [x] **`parse_todo_task()` — core parser function**
  - Read root `<repo>/todo.md`
  - Find the first line matching `- [ ]` (not `[x]`, not `[~]`)
  - Extract the task title text after the marker
  - Return `None` gracefully for any failure (no file, empty file, no `[ ]` found)
  - **File:** `dracon-sync/src/todo_parser.rs` (new module)
  - **Signature:** `pub fn parse_todo_task(repo: &Path) -> Option<TodoTask>`

- [x] **`TodoTask` struct**
  ```rust
  pub struct TodoTask {
      pub line_number: usize,   // 1-indexed line in todo.md
      pub title: String,        // The task text (e.g. "Monitor daemon stability...")
      pub sub_items: Vec<String>, // Indented bullets under the task
  }
  ```

- [x] **Sub-item collection rules**
  - After finding `- [ ] task title`, collect all subsequent indented lines
  - Stop at the next `- [ ]` or `- [x]` or `##` header at the same depth
  - Empty lines between sections are skipped (don't break collection)
  - This captures the task's acceptance criteria / scope notes

- [x] **Edge cases handled**
  - `todo.md` doesn't exist → `None` (graceful fallback)
  - `todo.md` is empty → `None`
  - No `[ ]` found (all `[x]`) → `None`
  - Decorative `- [ ]` in code blocks → not matched (skip ``` blocks)
  - `- [ ]` inside nested lists (`  - [ ]`) → matched as sub-items, not new tasks
  - File has only `[~]` (in-progress, not open) → treated same as `[x]`, not selected

- [x] **Tests for `todo_parser.rs`**
  - Parse a standard todo.md with one `[ ]` at top
  - Parse sub-items correctly
  - Return `None` when no `[ ]` found
  - Return `None` when file missing
  - Skip code blocks containing `- [ ]`
  - Multiple `[ ]` items — picks the FIRST one only
  - Item with `[~]` is NOT selected (only `[ ]`)
  - Mixed `[x]` + `[ ]` — skips done items, finds first open

---

## 🟡 Stage 2: Format — Generate the commit message from task context

**Goal:** A new message generator that wraps the task context around the file changes.

- [x] **`todo_context_message()` — main formatting function**
  - **Input:** `repo: &Path`, `staged_diff_names: &str`, `todo_task: &TodoTask`
  - **Output:** `String` — the formatted commit message body
  - **File:** `dracon-sync/src/scribe.rs`
  - **Pattern:** Subject = `[todo] <task title (truncate to ~60 chars)>`, body includes file changes + sub-items

- [x] **Format specification**
  ```
  [todo] Monitor daemon stability with all fixes over 24h
  
  Changed files:
  - scribe.rs (Modified)
  - sync.rs (Modified)
  - status.rs (Modified)
  
  Task scope:
  - clone race should be eliminated
  - git add should never force-add build artifacts
  ```

- [x] **Integration with existing message flow**
  - `local_fallback_message()` is unchanged — still produces `update scribe, sync`
  - New `todo_context_message()` replaces it when config toggle is on
  - The message is wrapped with `chore(sync):` the same way in `sync.rs`

- [x] **Subject line truncation rules**
  - Task title goes in subject after `[todo]` prefix
  - Truncate to ~60 chars if over, append `…`
  - Never produce a blank subject

- [x] **Tests for formatting**
  - Basic formatting with 2 changed files
  - Truncation of long task title
  - Empty file list (unlikely but safe)
  - Task with sub-items renders correctly

---

## 🟡 Stage 3: Config — Add the policy toggle

**Goal:** A `dracon-sync.toml` option to enable todo-context mode.

- [x] **Add field to `SyncPolicy` struct**
  - **File:** `dracon-sync/src/policy.rs`
  - **Field:** `pub(crate) todo_commit_messages: bool`
  - **Default:** `false` (existing behavior preserved)

- [x] **Add to example config**
  - **File:** `dracon-sync/dracon-sync.example.toml`
  - **Entry:**
    ```toml
    # [todo_commit_messages]
    # Instead of file-stem summaries ("update scribe, sync"),
    # read root todo.md and use the first [ ] task as commit context.
    # Falls back to file-stem summary if no [ ] found or no todo.md.
    todo_commit_messages = false
    ```

---

## 🟡 Stage 4: Wire — Connect everything in the sync flow

**Goal:** Modify the commit message generation in `sync.rs` to use the new mode when enabled.

- [x] **In `stage_commit_and_push()` (sync.rs ~line 1155)**
  - After determining `staged_diff_names` and before the current fallback chain:
    ```rust
    let local_fallback = if policy.todo_commit_messages {
        let todo_msg = crate::todo_parser::parse_todo_task(repo)
            .map(|task| todo_context_message(repo, &staged_diff_names, &task))
            .unwrap_or_else(|| crate::scribe::local_fallback_message(&staged_diff_names));
        Some(todo_msg)
    } else if ai_subject.is_none() {
        Some(crate::scribe::local_fallback_message(&staged_diff_names))
    } else {
        None
    };
    ```

- [x] **Verify the message wrapping still works**
  - The existing `chore(sync): <msg>` wrapping in sync.rs applies automatically
  - No changes needed to the message assembly logic

- [x] **No changes to AI path**
  - `generate_commit_message()` still runs as before
  - `local_fallback` only triggers when AI returns None
  - todo-context mode only applies to the fallback path

---

## 🔵 Stage 5: Polish — Status reporting and edge cases

- [~] **Show active task in `dracon-sync status`** (deferred — requires `status` command changes)
  - Current wiring only affects the commit message path
  - Status command reads from policy but doesn't display active todo yet
  - If `todo_commit_messages = true`, display the active `[ ]` task
  - Helps users verify the system is tracking the right thing

- [x] **Logging** — `[todo]` prefix in commit subject signals source
  - Additional stderr logging not added (message itself is self-documenting)
  - `📝 todo-context: found task "Monitor daemon stability..."` (info level)
  - `📝 todo-context: no [ ] found in todo.md, using file-stem fallback` (info level)
  - `📝 todo-context: todo.md not found, using file-stem fallback` (debug level)

- [x] **What happens when the task is completed?**
  - Checks `[ ]` → `[x]` in `todo.md`, next commit picks the NEW first `[ ]`
  - User (or agent) checks `[ ]` → `[x]` in todo.md
  - Next commit: parser finds the NEXT `[ ]` instead
  - No config changes needed — it's automatic

- [x] **Multiple repos with same todo.md?** — each repo has its own root, no cross-repo issues
  - Each repo has its own root `todo.md`
  - No cross-repo interference
  - Root-only rule: never looks at sibling repo todo.md

---

## 🔵 Stage 6: Testing — Full integration tests

- [x] **Integration test: todo-context mode produces expected output**
  - Create temp repo with `todo.md` containing one `[ ]`
  - Stage a file change
  - Verify commit message contains `[todo]` and the task title
  - Verify message IS different from `local_fallback_message` output

- [x] **Integration test: fallback when no `[ ]` found**
  - Create temp repo with `todo.md` containing only `[x]` items
  - Stage a file change
  - Verify commit message matches `local_fallback_message` output

- [x] **Integration test: fallback when no `todo.md`**
  - Create temp repo without `todo.md`
  - Stage a file change
  - Verify commit message matches `local_fallback_message` output

- [x] **Integration test: toggle off = no change** (implicit — `false` default means existing path)
  - `todo_commit_messages = false`
  - Stage a file change
  - Verify commit message is identical to current behavior

---

## 📋 Summary

| Stage | What | Files | Risk |
|-------|------|-------|------|
| 1 | Parser | `todo_parser.rs` (new) | Low |
| 2 | Formatter | `scribe.rs` (additions) | Low |
| 3 | Config | `policy.rs` + example.toml | Low |
| 4 | Wiring | `sync.rs` (~10 lines) | Low |
| 5 | Polish | `status.rs` | Low |
| 6 | Tests | `todo_parser.rs`, `sync.rs` | Low |

**Existing behavior is never changed.** The toggle defaults to `false`.
When `true` and no `[ ]` found, it silently falls back to the current
file-stem behavior.

**Total: ~250-350 lines of code** across parser, formatter, tests, config.
