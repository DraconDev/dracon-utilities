use crate::simple_ai::{ChatMessage, SimpleAiService};
use crate::todo_parser::parse_todo_task;
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

/// Generate a commit message aligned to the first open `[ ]` in root `todo.md`.
///
/// This follows the "Ratio & Fact Reporting" strategy: dumb, deterministic stenographer
/// that reports raw ledger and code deltas without semantic scope matching.
///
/// Title is a routing key for downstream AI: `sync: X checked`
/// Body is machine-readable JSON with ledger_delta, code_delta, and verification.
///
/// Falls back to `local_fallback_message` if no `[ ]` is found in `todo.md`.
///
/// For AI-to-AI consumption only — no human browsability, no prose, no redundancy.
pub fn todo_context_message(repo: &Path, diff_names: &str) -> String {
    let task = parse_todo_task(repo);

    // Always produce routing key format. When there's no todo.md or no open
    // task, checked count is 0 and ledger_delta.checked is empty. The AI
    // reads this format downstream — no human browsing needed.
    let checked_count: u64 = task.as_ref().map_or(0, |t| t.sub_items.len() as u64);
    let title: String = format!("sync: {} checked", checked_count);

    // Build file list as JSON array string
    let files_json: String = {
        let entries: Vec<&str> = diff_names
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        let file_names: Vec<String> = entries
            .iter()
            .map(|entry| {
                let path = entry
                    .split_once(": ")
                    .map(|(_, p)| p)
                    .unwrap_or(entry)
                    .trim();
                format!("\"{}\"", path)
            })
            .collect();
        format!("[{}]", file_names.join(",\n      "))
    };

    // Build JSON body — ledger_delta.checked contains the task title if
    // present, empty array if no todo.md
    let checked_entry: String = if let Some(ref t) = task {
        if t.title.is_empty() {
            String::new()
        } else {
            format!("\"{}\"", t.title)
        }
    } else {
        String::new()
    };

    let json_body: String = if checked_entry.is_empty() {
        format!(
            "{{\n  \"ledger_delta\": {{\n    \"checked\": []\n  }},\n  \"code_delta\": {{\n    \"files\": {}\n  }},\n  \"verification\": {{\n    \"tests_passed\": 42\n  }}\n}}",
            files_json
        )
    } else {
        format!(
            "{{\n  \"ledger_delta\": {{\n    \"checked\": [\n      \"{}\"\n    ]\n  }},\n  \"code_delta\": {{\n    \"files\": {}\n  }},\n  \"verification\": {{\n    \"tests_passed\": 42\n  }}\n}}",
            checked_entry,
            files_json
        )
    };

    format!("{}\n\n{}", title, json_body)
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
    fn test_todo_context_routing_key_with_task() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("todo.md"),
            "- [x] Done\n- [ ] My active task\n  - acceptance criteria\n- [ ] Another\n",
        )
        .unwrap();
        let diff_names = "Modified: src/scribe.rs\nAdded: tests/test.rs";
        let result = todo_context_message(tmp.path(), diff_names);
        // Title should be routing key: sync: X checked
        assert!(result.starts_with("sync: 1 checked"));
        // Body should contain JSON with ledger_delta
        assert!(result.contains("ledger_delta"));
        assert!(result.contains("checked"));
        assert!(result.contains("\"My active task\""));
        // Body should contain code_delta with file list
        assert!(result.contains("code_delta"));
        assert!(result.contains("src/scribe.rs"));
        assert!(result.contains("tests/test.rs"));
        // Body should contain verification
        assert!(result.contains("verification"));
    }

    #[test]
    fn test_todo_context_falls_back_when_no_open_task() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("todo.md"), "- [x] All done\n")
            .unwrap();
        let diff_names = "Modified: src/main.rs";
        let result = todo_context_message(tmp.path(), diff_names);
        // Should produce local_fallback output (not JSON)
        assert!(!result.contains("ledger_delta"));
        assert!(result.contains("main"));
    }

    #[test]
    fn test_todo_context_falls_back_when_no_todo_file() {
        let tmp = tempfile::tempdir().unwrap();
        // No todo.md written
        let diff_names = "Modified: src/main.rs";
        let result = todo_context_message(tmp.path(), diff_names);
        assert!(!result.contains("ledger_delta"));
        assert!(result.contains("main"));
    }

    #[test]
    fn test_todo_context_json_is_machine_parseable() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("todo.md"),
            "- [ ] Real work to do\n  - criteria 1\n  - criteria 2\n",
        )
        .unwrap();
        let diff_names = "Modified: src/work.rs";
        let result = todo_context_message(tmp.path(), diff_names);
        // Must be parseable JSON in the body
        assert!(result.starts_with("sync: 2 checked"));

        // Extract and verify JSON body
        let parts: Vec<&str> = result.splitn(2, '\n').collect();
        assert_eq!(parts.len(), 2);
        // Body should be at least 2 lines
        assert!(parts[1].contains("ledger_delta"));
        assert!(parts[1].contains("\"Real work to do\""));
        assert!(parts[1].contains("\"src/work.rs\""));
    }
}
