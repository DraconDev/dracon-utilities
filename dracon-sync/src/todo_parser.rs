//! Parse root `todo.md` to find the first open `[ ]` task.
//!
//! Used by the scribe's todo-context mode to generate commit messages
//! aligned to the active task, instead of generic file-stem summaries.

use std::path::Path;

/// A single open task extracted from the first `- [ ]` in `todo.md`.
#[derive(Debug, Clone)]
pub struct TodoTask {
    /// 1-indexed line number where `- [ ]` was found.
    pub line_number: usize,
    /// The task title text after `- [ ] `.
    pub title: String,
    /// Indented bullets or notes under the task, up to the next task or header.
    pub sub_items: Vec<String>,
}

/// Read root `<repo>/todo.md` and return the first open `[ ]` task.
///
/// Returns `None` if:
/// - File doesn't exist or can't be read
/// - No `- [ ]` line is found (all tasks done or in-progress `[~]`)
/// - The file has no open tasks
///
/// Code blocks (triple-backtick) are skipped so decorative `- [ ]` inside
/// examples are not matched.
pub fn parse_todo_task(repo: &Path) -> Option<TodoTask> {
    let todo_path = repo.join("todo.md");
    let content = std::fs::read_to_string(todo_path).ok()?;

    let lines: Vec<&str> = content.lines().collect();
    let mut in_code_block = false;

    for (i, line) in lines.iter().enumerate() {
        // Track code block boundaries
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        // Look for `- [ ]` at the start of a line (possibly indented)
        // Must be `[ ]` exactly, not `[x]` or `[~]`.
        if let Some(task_title) = is_open_task_line(trimmed) {
            let sub_items = collect_sub_items(&lines, i + 1, &mut in_code_block);
            return Some(TodoTask {
                line_number: i + 1,
                title: task_title.to_string(),
                sub_items,
            });
        }
    }

    None
}

/// Check if a line is an open task marker `- [ ]`.
/// Returns the task title text after the marker, or `None`.
fn is_open_task_line(line: &str) -> Option<&str> {
    let line = line.trim();
    // Match: `- [ ] ` or `* [ ] ` — the marker has a space between `[` and `]`
    // This distinguishes `[ ]` (open) from `[x]` or `[~]` (not open).
    if let Some(rest) = line.strip_prefix("- [ ") {
        // rest starts with `] ` or is just `]`
        if rest.starts_with("] ") {
            Some(rest[2..].trim())
        } else if rest == "]" {
            Some("")
        } else {
            None
        }
    } else if let Some(rest) = line.strip_prefix("* [ ") {
        if rest.starts_with("] ") {
            Some(rest[2..].trim())
        } else if rest == "]" {
            Some("")
        } else {
            None
        }
    } else {
        None
    }
}

/// Collect sub-items from lines after the task title.
///
/// Sub-items are indented lines (bullet points, notes) that belong to the task.
/// Stops at:
/// - Next `- [ ]` or `- [x]` at the same depth
/// - A `##` or `#` header
/// - End of file
/// - A blank line followed by a non-indented non-blank line (section break)
fn collect_sub_items(lines: &[&str], start: usize, in_code_block: &mut bool) -> Vec<String> {
    let mut items = Vec::new();
    let mut seen_blank_line_after_content = false;

    for line in lines[start..].iter() {
        let trimmed = line.trim();

        // Track code blocks
        if trimmed.starts_with("```") {
            *in_code_block = !*in_code_block;
            continue;
        }

        // Stop at headers
        if trimmed.starts_with('#') && !trimmed.starts_with("```") {
            break;
        }

        // Stop at next task marker at the same depth (not indented)
        if !line.starts_with(char::is_whitespace) {
            if is_open_task_line(trimmed).is_some()
                || is_closed_task_line(trimmed).is_some()
                || is_inprogress_task_line(trimmed).is_some()
            {
                break;
            }
        }

        // Blank line handling: if we've seen content, a blank line followed
        // by more indented content is still part of the task.
        // A blank line followed by a non-indented line is a section break.
        if trimmed.is_empty() {
            seen_blank_line_after_content = !items.is_empty();
            continue;
        }

        // Only collect indented lines (sub-items)
        if line.starts_with(char::is_whitespace) || trimmed.starts_with('-') || trimmed.starts_with('*') {
            items.push(trimmed.to_string());
            seen_blank_line_after_content = false;
        } else if seen_blank_line_after_content {
            // Non-indented, non-blank line after a blank → section break
            break;
        }
    }

    items
}

fn is_closed_task_line(line: &str) -> Option<&str> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("- [x] ") {
        Some(rest.trim())
    } else if let Some(rest) = line.strip_prefix("- [X] ") {
        Some(rest.trim())
    } else if let Some(rest) = line.strip_prefix("* [x] ") {
        Some(rest.trim())
    } else if let Some(rest) = line.strip_prefix("* [X] ") {
        Some(rest.trim())
    } else {
        None
    }
}

fn is_inprogress_task_line(line: &str) -> Option<&str> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("- [~] ") {
        Some(rest.trim())
    } else if let Some(rest) = line.strip_prefix("* [~] ") {
        Some(rest.trim())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_todo(repo: &tempfile::TempDir, content: &str) {
        let path = repo.path().join("todo.md");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "{}", content).unwrap();
    }

    #[test]
    fn test_finds_first_open_task() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "\
- [x] Done task
- [ ] First open task
  - some detail
- [ ] Second open task (should not be picked)
");
        let task = parse_todo_task(tmp.path()).unwrap();
        assert_eq!(task.title, "First open task");
        assert_eq!(task.line_number, 2);
        assert_eq!(task.sub_items, vec!["- some detail"]);
    }

    #[test]
    fn test_returns_none_when_all_done() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "\
- [x] Task one
- [x] Task two
- [x] Task three
");
        assert!(parse_todo_task(tmp.path()).is_none());
    }

    #[test]
    fn test_returns_none_when_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        // No todo.md written
        assert!(parse_todo_task(tmp.path()).is_none());
    }

    #[test]
    fn test_returns_none_when_empty_file() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "");
        assert!(parse_todo_task(tmp.path()).is_none());
    }

    #[test]
    fn test_skips_in_progress_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "\
- [~] In progress task
- [ ] Next open task
");
        let task = parse_todo_task(tmp.path()).unwrap();
        assert_eq!(task.title, "Next open task");
    }

    #[test]
    fn test_skips_code_blocks() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "\
# Test

```markdown
- [ ] This is inside a code block, should be ignored
```

- [ ] Real task
");
        let task = parse_todo_task(tmp.path()).unwrap();
        assert_eq!(task.title, "Real task");
    }

    #[test]
    fn test_collects_sub_items() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "\
- [ ] Main task
  - Sub bullet one
  - Sub bullet two
  Some plain text indented note
- [x] Next task
");
        let task = parse_todo_task(tmp.path()).unwrap();
        assert_eq!(task.title, "Main task");
        assert!(task.sub_items.len() >= 2);
        assert!(task.sub_items.iter().any(|s| s.contains("Sub bullet one")));
    }

    #[test]
    fn test_stops_at_header() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "\
- [ ] My task
  - detail

## Next Section
- [ ] Other task
");
        let task = parse_todo_task(tmp.path()).unwrap();
        assert_eq!(task.title, "My task");
        // Should NOT include items from "Next Section"
        assert!(task.sub_items.iter().all(|s| !s.contains("Other task")));
    }

    #[test]
    fn test_handles_star_bullet_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "\
* [ ] Star bullet task
  - detail
");
        let task = parse_todo_task(tmp.path()).unwrap();
        assert_eq!(task.title, "Star bullet task");
    }

    #[test]
    fn test_handles_empty_title() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "\
- [ ] 
");
        let task = parse_todo_task(tmp.path()).unwrap();
        assert_eq!(task.title, "");
    }

    #[test]
    fn test_sub_items_stops_at_same_level_task() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "\
- [ ] Task one
  - detail for one
- [ ] Task two
  - detail for two
");
        let task = parse_todo_task(tmp.path()).unwrap();
        assert_eq!(task.title, "Task one");
        assert_eq!(task.sub_items.len(), 1);
        assert!(task.sub_items[0].contains("detail for one"));
    }

    #[test]
    fn test_picks_first_open_even_if_deep() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "\
## Section
- [x] Done
  - old detail

## Next
- [ ] First open
  - important detail

- [ ] Another
");
        let task = parse_todo_task(tmp.path()).unwrap();
        assert_eq!(task.title, "First open");
        assert!(task.sub_items.iter().any(|s| s.contains("important detail")));
    }

    #[test]
    fn test_no_sub_items_when_only_task() {
        let tmp = tempfile::tempdir().unwrap();
        write_todo(&tmp, "- [ ] Only task\n");
        let task = parse_todo_task(tmp.path()).unwrap();
        assert_eq!(task.title, "Only task");
        assert!(task.sub_items.is_empty());
    }
}
// FIXME: test
