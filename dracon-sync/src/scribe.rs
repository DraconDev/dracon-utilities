use crate::simple_ai::{ChatMessage, SimpleAiService};
use std::path::Path;

/// Build the **system** prompt — authoritative instructions the AI should follow.
/// This is delivered as a system-level message, which models treat as binding
/// instructions rather than user data they can override.
fn build_system_instructions() -> String {
    r#"You are generating a git commit subject line for a code change.

You will receive a CURRENT CHANGE in the subsequent user message.
You may also receive PREVIOUS DIFFS and RECENT COMMIT SUBJECTS as background context.

RULES:
- Output ONE line: the commit subject (no body, no markdown, no preamble)
- Describe the CURRENT CHANGE specifically — what it does and why
- Do NOT describe previous diffs — those are background only
- Do NOT repeat recent commit subjects
- Use conventional commit style if natural: type(scope): description
- If fixing a bug: "fix(scope): what was wrong and how it was fixed"
- If adding feature: "feat(scope): what was added"
- If refactoring: "refactor(scope): what changed"
- If docs only: "docs(scope): what documentation was updated"
- Keep under 72 characters
- Do NOT wrap in quotes or backticks
- Do NOT start with a dash or bullet

BAD (too generic):
- wip checkpoint
- Updated files
- Code changes
- File: src/main.rs

GOOD (specific and semantic):
- fix(auth): validate JWT expiry before accepting tokens
- feat(sync): add push retry with HTTPS fallback on SSH timeout
- refactor(warden): extract key generation into separate module
- docs(readme): add installation steps for Nix users"#
        .to_string()
}

/// Build the **user** message — untrusted data (diff content, file names).
/// The content below the marker line is untrusted data extracted from git.
/// It is delivered as a user-level message, which the model knows is
/// user-generated input (not authoritative instructions).
fn build_user_content(
    current_diff: &str,
    current_diff_names: &str,
    recent_diffs: &[String],
    recent_subjects: &[String],
) -> String {
    let prev_diffs_section = if recent_diffs.is_empty() {
        String::new()
    } else {
        let entries: Vec<String> = recent_diffs
            .iter()
            .enumerate()
            .map(|(i, d)| {
                format!(
                    "--- PREVIOUS DIFF {} (background context only) ---\n{}--- END ---",
                    i + 1,
                    d
                )
            })
            .collect();
        format!("\n\nPREVIOUS DIFFS (background only — do NOT describe these, just use for understanding work trajectory):\n{}", entries.join("\n\n"))
    };

    let subjects_section = if recent_subjects.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nRECENT COMMIT SUBJECTS (for context, do NOT repeat these):\n{}",
            recent_subjects.join("\n")
        )
    };

    format!(
        r#"CURRENT CHANGE (THIS is what you must describe):
--- CURRENT DIFF ---
{current_diff}
--- END ---

CURRENT FILES:
{current_diff_names}{prev_diffs_section}{subjects_section}"#
    )
}

fn collect_recent_diffs(repo: &Path, count: usize) -> Vec<String> {
    let count_arg = format!("-{}", count);
    let output = match std::process::Command::new("git")
        .args([
            "log",
            &count_arg,
            "--pretty=format:%H",
            "--diff-filter=ACDMRT",
        ])
        .current_dir(repo)
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let hashes: Vec<&str> = output
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let mut diffs = Vec::new();
    for hash in hashes {
        let diff = match std::process::Command::new("git")
            .args([
                "diff",
                "--stat",
                "--unified=1",
                &format!("{}^..{}", hash, hash),
            ])
            .current_dir(repo)
            .output()
        {
            Ok(o) if o.status.success() => {
                let d = String::from_utf8_lossy(&o.stdout).to_string();
                if d.lines().count() > 50 {
                    d.lines().take(50).collect::<Vec<_>>().join("\n") + "\n... (truncated)"
                } else {
                    d
                }
            }
            _ => continue,
        };
        diffs.push(diff);
    }
    diffs
}

fn collect_recent_subjects(repo: &Path, count: usize) -> Vec<String> {
    let count_arg = format!("-{}", count);
    match std::process::Command::new("git")
        .args(["log", &count_arg, "--pretty=format:%s"])
        .current_dir(repo)
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

/// Deterministic fallback when no todo.md / no open task.
/// Groups by action (Modified, Added, Deleted) and reports counts + file names.
/// Format: "Modified 2, Added 1: file1.rs, file2.rs; new.rs"
fn deterministic_diff_summary(diff_names: &str) -> String {
    let entries: Vec<(&str, &str)> = diff_names
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| l.split_once(": "))
        .map(|(status, path)| (status.trim(), path.trim()))
        .collect();

    if entries.is_empty() {
        return "No files changed".to_string();
    }

    // Group by status
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (status, path) in &entries {
        groups.entry(status).or_default().push(path);
    }

    let total = entries.len();
    if total == 1 {
        let (status, path) = entries[0];
        return format!("{} {}", status, path);
    }

    // Build summary: "Modified 2, Added 1: file1.rs, file2.rs; new.rs"
    let mut parts: Vec<String> = Vec::new();
    for (status, paths) in &groups {
        let count = paths.len();
        let names: Vec<&str> = paths.iter().take(2).copied().collect();
        let name_str = if names.len() < paths.len() {
            format!("{}, ...", names.join(", "))
        } else {
            names.join(", ")
        };
        parts.push(format!("{} {}: {}", status, count, name_str));
    }

    parts.join("; ").to_string()
}

/// Scan the staged diff for task state transitions (`[ ]` → `[x]` / `[~]`).
///
/// Returns Vec<(task_text, new_state, file)> — one entry per transition found.
/// \"[x]\" means completed, \"[~]\" means in-progress.
///
/// Strategy: parse `git diff --cached -U0` to find adjacent `- [ ]` / `+ [x]` pairs.
/// The diff itself IS the receipt — we don't read `todo.md` directly.
fn scan_diff_for_transitions(repo: &Path) -> Vec<(String, String, String)> {
    let output = match std::process::Command::new("git")
        .args(["diff", "--cached", "-U0"])
        .current_dir(repo)
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut transitions = Vec::new();
    let mut current_file = String::new();
    let lines: Vec<&str> = output.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("+++ b/") {
            current_file = line.trim_start_matches("+++ b/").to_string();
        }

        // Look for `- [ ]` line — the old (open) task line
        if line.starts_with('-') && contains_open_task(&line[1..]) {
            let old_text = extract_task_text(&line[1..], "[ ]");
            // Scan forward within same hunk (until next @@ or file header) for matching + [x] / [~]
            let mut j = i + 1;
            while j < lines.len() {
                let next = lines[j];
                if next.starts_with("@@")
                    || next.starts_with("diff --git")
                    || next.starts_with("--- ")
                {
                    break;
                }
                if let Some(new_line) = next.strip_prefix('+') {
                    let (marker, state) = if new_line.contains("[x]") {
                        ("[x]", "[x]")
                    } else if new_line.contains("[~]") {
                        ("[~]", "[~]")
                    } else {
                        ("", "")
                    };
                    if !marker.is_empty() {
                        let new_text = extract_task_text(new_line, marker);
                        // Match: task text should be similar (same after stripping markers)
                        if old_text.as_deref() == new_text.as_deref() {
                            if let Some(task) = new_text {
                                if !task.is_empty() {
                                    transitions.push((task, state.to_string(), current_file.clone()));
                                }
                            }
                        }
                    }
                }
                j += 1;
            }
        }

        i += 1;
    }

    transitions
}

fn contains_open_task(line: &str) -> bool {
    line.contains("[ ]")
}

fn extract_task_text(line: &str, marker: &str) -> Option<String> {
    if let Some(pos) = line.find(marker) {
        let after = line[pos + marker.len()..].trim();
        if after.is_empty() {
            None
        } else {
            Some(after.to_string())
        }
    } else {
        None
    }
}

pub fn local_fallback_message(diff_names: &str) -> String {
    let entries: Vec<&str> = diff_names
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    if entries.is_empty() {
        return "chore: update files".to_string();
    }

    let mut stems: Vec<String> = Vec::new();
    for entry in entries.iter().take(3) {
        let path = entry
            .split_once(": ")
            .map(|(_, p)| p)
            .unwrap_or(entry)
            .trim();
        let stem = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(path);
        if !stems.iter().any(|s| s == stem) {
            stems.push(stem.to_string());
        }
    }

    let extra = entries.len().saturating_sub(stems.len());
    let suffix = if extra > 0 {
        format!(" and {} file{}", extra, if extra > 1 { "s" } else { "" })
    } else {
        String::new()
    };

    let desc = stems.join(", ");
    format!("update {}{}", desc, suffix)
}

/// Generate a commit message from the staged diff.
///
/// **Subject**: always `deterministic_diff_summary(diff_names)` — reliable ground truth.
/// **Body**: enriched with `Task transitions:` when the diff shows `[ ]` → `[x]` / `[~]` changes.
///
/// No longer reads root `todo.md` directly. Instead, scans the git diff for checked boxes —
/// the Worker's own diff IS the receipt of what was completed.
pub fn todo_context_message(repo: &Path, diff_names: &str) -> String {
    // Subject is always the deterministic diff summary — never wrong
    let summary = deterministic_diff_summary(diff_names);

    // Scan diff for task transitions the Worker made in changed files
    let transitions = scan_diff_for_transitions(repo);

    // Build body: Task transitions (if any) + file list
    let mut body_parts: Vec<String> = Vec::new();

    if !transitions.is_empty() {
        body_parts.push("Task transitions:".to_string());
        for (task_text, state, file) in &transitions {
            body_parts.push(format!("  {} {} ({})", state, task_text, file));
        }
    }

    // File list
    let files: Vec<&str> = diff_names
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    let file_count = files.len();
    if file_count > 0 {
        body_parts.push(format!(
            "{} file{} changed:",
            file_count,
            if file_count == 1 { "" } else { "s" }
        ));
        for f in &files {
            body_parts.push(format!("  {}", f));
        }
    }

    let body = body_parts.join("\n");
    if body.is_empty() {
        summary
    } else {
        format!("{}\n\n{}", summary, body)
    }
}

#[cfg(feature = "scribe")]
pub(crate) async fn generate_commit_message(
    repo: &Path,
    staged_diff_names: &str,
    staged_diff_content: Option<String>,
) -> Option<String> {
    let service = SimpleAiService::new();
    if service.is_empty() {
        eprintln!("📝 scribe: no AI providers, using local fallback");
        return None;
    }

    let current_diff = staged_diff_content
        .as_deref()
        .unwrap_or("(no diff content available)");
    let recent_diffs = collect_recent_diffs(repo, 10);
    let recent_subjects = collect_recent_subjects(repo, 10);

    let system_prompt = build_system_instructions();
    let user_content = build_user_content(
        current_diff,
        staged_diff_names,
        &recent_diffs,
        &recent_subjects,
    );

    let messages = vec![
        ChatMessage::system(&system_prompt),
        ChatMessage::user(&user_content),
    ];

    match service.chat(messages).await {
        Ok(text) => {
            let subject = text.lines().next().unwrap_or("").trim().to_string();
            if subject.is_empty() {
                eprintln!("📝 scribe: AI returned empty subject, using local fallback");
                return None;
            }
            // Post-processing: defense-in-depth against AI output that
            // echoes back the untrusted diff content as instructions.
            let lower = subject.to_lowercase();
            if lower.starts_with("i will")
                || lower.starts_with("i cannot")
                || lower.starts_with("i am")
                || lower.starts_with("you are")
            {
                eprintln!(
                    "📝 scribe: rejected AI output (possible injection echo), using local fallback"
                );
                return None;
            }
            if subject.len() > 100 {
                let truncated: String = subject.chars().take(97).collect();
                eprintln!(
                    "📝 scribe: generated commit subject (truncated): {}",
                    truncated
                );
                Some(format!("{}...", truncated))
            } else {
                eprintln!("📝 scribe: generated commit subject: {}", subject);
                Some(subject)
            }
        }
        Err(e) => {
            eprintln!("📝 scribe: AI request failed: {} — using local fallback", e);
            None
        }
    }
}

#[cfg(not(feature = "scribe"))]
pub(crate) async fn generate_commit_message(
    _repo: &Path,
    _staged_diff_names: &str,
    _staged_diff_content: Option<String>,
) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_instructions_contains_rules() {
        let sys = build_system_instructions();
        assert!(sys.contains("git commit subject"));
        assert!(sys.contains("conventional commit"));
        assert!(sys.contains("72 characters"));
    }

    #[test]
    fn test_build_user_content_contains_current_diff() {
        let content = build_user_content(
            "diff --git a/main.rs\n+fn main()",
            "Modified: main.rs",
            &["previous diff content".to_string()],
            &["feat: old commit".to_string()],
        );
        assert!(content.contains("CURRENT DIFF"));
        assert!(content.contains("diff --git"));
        assert!(content.contains("PREVIOUS DIFF"));
        assert!(content.contains("RECENT COMMIT"));
        // Ensure no instruction-like prefixes leak into user content
        assert!(!content.contains("YOU ARE"));
    }

    #[test]
    fn test_build_user_content_no_previous_diffs() {
        let content = build_user_content(
            "diff --git a/main.rs\n+fn main()",
            "Modified: main.rs",
            &[],
            &[],
        );
        assert!(content.contains("CURRENT DIFF"));
        assert!(!content.contains("PREVIOUS DIFFS"));
        assert!(!content.contains("RECENT COMMIT"));
    }

    #[test]
    fn test_deterministic_diff_single_file() {
        let names = "Modified: src/main.rs";
        let result = deterministic_diff_summary(names);
        assert_eq!(result, "Modified src/main.rs");
    }

    #[test]
    fn test_deterministic_diff_multiple_files() {
        let names = "Modified: src/auth.rs\nAdded: src/jwt.rs\nModified: Cargo.toml\nAdded: lib.rs";
        let result = deterministic_diff_summary(names);
        assert!(result.contains("Modified 2"));
        assert!(result.contains("Added 2"));
        assert!(result.contains("auth.rs"));
        assert!(result.contains("jwt.rs"));
    }

    #[test]
    fn test_deterministic_diff_empty() {
        let result = deterministic_diff_summary("");
        assert_eq!(result, "No files changed");
    }

    #[test]
    fn test_deterministic_diff_groups_by_status() {
        let names = "Modified: src/a.rs\nAdded: src/b.rs\nDeleted: src/c.rs";
        let result = deterministic_diff_summary(names);
        assert!(result.contains("Modified 1"));
        assert!(result.contains("Added 1"));
        assert!(result.contains("Deleted 1"));
    }

    #[test]
    fn test_local_fallback_single_file() {
        let names = "Modified: src/main.rs";
        let result = local_fallback_message(names);
        assert!(result.contains("main"));
    }

    #[test]
    fn test_local_fallback_multiple_files() {
        let names = "Modified: src/auth.rs\nAdded: src/jwt.rs\nModified: Cargo.toml\nAdded: lib.rs";
        let result = local_fallback_message(names);
        assert!(result.contains("auth"));
        assert!(result.contains("and 1 file"));
    }

    #[test]
    fn test_local_fallback_empty() {
        let result = local_fallback_message("");
        assert_eq!(result, "chore: update files");
    }

    #[test]
    fn test_local_fallback_deduplicates_stems() {
        let names = "Modified: src/auth.rs\nAdded: tests/auth.rs";
        let result = local_fallback_message(names);
        let count = result.matches("auth").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_todo_context_no_transitions_pure_diff() {
        // Non-git temp dir: git diff --cached fails → no transitions → pure fallback
        let tmp = tempfile::tempdir().unwrap();
        let diff_names = "Modified: src/main.rs\nAdded: src/lib.rs";
        let result = todo_context_message(tmp.path(), diff_names);
        // Subject is deterministic diff summary (BTreeMap: Added before Modified alphabetically)
        assert!(result.starts_with("Added 1: src/lib.rs; Modified 1: src/main.rs"));
        // Body has file list
        assert!(result.contains("2 files changed:"));
        assert!(result.contains("Modified: src/main.rs"));
        assert!(result.contains("Added: src/lib.rs"));
        // No Task transitions block
        assert!(!result.contains("Task transitions:"));
    }

    #[test]
    fn test_todo_context_with_transitions() {
        // Real git repo with staged [ ] → [x] transition
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir(&repo).unwrap();

        // Init git repo
        std::process::Command::new("git").args(["init"]).current_dir(&repo).output().unwrap();
        std::process::Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(&repo).output().unwrap();
        std::process::Command::new("git").args(["config", "user.name", "Test"]).current_dir(&repo).output().unwrap();

        // Write initial file with open task
        std::fs::write(repo.join("tasks.md"), "- [ ] Fix the login bug\n").unwrap();
        std::process::Command::new("git").args(["add", "tasks.md"]).current_dir(&repo).output().unwrap();
        std::process::Command::new("git").args(["commit", "-m", "init"]).current_dir(&repo).output().unwrap();

        // Modify to [x] and stage
        std::fs::write(repo.join("tasks.md"), "- [x] Fix the login bug\n").unwrap();
        std::process::Command::new("git").args(["add", "tasks.md"]).current_dir(&repo).output().unwrap();

        let diff_names = "Modified: tasks.md";
        let result = todo_context_message(&repo, diff_names);
        // Subject is deterministic diff summary (not close(todo):)
        assert!(result.starts_with("Modified tasks.md"));
        // Body has Task transitions block
        assert!(result.contains("Task transitions:"));
        assert!(result.contains("[x]"));
        assert!(result.contains("Fix the login bug"));
        assert!(result.contains("tasks.md"));
        // Body has file list
        assert!(result.contains("1 file changed:"));
    }

    #[test]
    fn test_todo_context_with_in_progress_transition() {
        // [ ] → [~] transition
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir(&repo).unwrap();

        std::process::Command::new("git").args(["init"]).current_dir(&repo).output().unwrap();
        std::process::Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(&repo).output().unwrap();
        std::process::Command::new("git").args(["config", "user.name", "Test"]).current_dir(&repo).output().unwrap();

        std::fs::write(repo.join("todo.md"), "- [ ] Start refactor\n").unwrap();
        std::process::Command::new("git").args(["add", "todo.md"]).current_dir(&repo).output().unwrap();
        std::process::Command::new("git").args(["commit", "-m", "init"]).current_dir(&repo).output().unwrap();

        // [ ] → [~]
        std::fs::write(repo.join("todo.md"), "- [~] Start refactor\n").unwrap();
        std::process::Command::new("git").args(["add", "todo.md"]).current_dir(&repo).output().unwrap();

        let diff_names = "Modified: todo.md";
        let result = todo_context_message(&repo, diff_names);
        assert!(result.contains("Task transitions:"));
        assert!(result.contains("[~]"));
        assert!(result.contains("Start refactor"));
        assert!(!result.contains("[x]")); // no [x] transitions
    }

    #[test]
    fn test_todo_context_falls_back_when_no_todo_file() {
        let tmp = tempfile::tempdir().unwrap();
        // No todo.md written, no git repo → pure fallback
        let diff_names = "Modified: src/main.rs";
        let result = todo_context_message(tmp.path(), diff_names);
        // Falls back to deterministic diff summary
        assert!(result.starts_with("Modified src/main.rs"));
        assert!(result.contains("Modified: src/main.rs"));
        assert!(result.contains("\n\n"));
    }
}
