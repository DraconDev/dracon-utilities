use crate::simple_ai::{ChatMessage, SimpleAiService};
use std::path::Path;

fn collect_git_context(repo: &Path) -> (String, String) {
    let git_log = std::process::Command::new("git")
        .args(["log", "--format=%s%n  files: %(trailers:key=file,valueonly)", "-20"])
        .current_dir(repo)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|_| "no git history".to_string());

    let git_files = std::process::Command::new("git")
        .args(["log", "--oneline", "--name-only", "-10"])
        .current_dir(repo)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    (git_log, git_files)
}

fn collect_blueprint(repo: &Path) -> String {
    let plan_dir = repo.join("plan");
    if !plan_dir.exists() {
        return String::new();
    }

    std::fs::read_dir(&plan_dir)
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                .max_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
        })
        .and_then(|e| std::fs::read_to_string(e.path()).ok())
        .unwrap_or_default()
}

fn cleanup_markdown(input: &str) -> String {
    let mut result: Vec<String> = Vec::new();
    let mut pending_blank = false;

    for line in input.lines() {
        let trimmed = line.trim_end();
        let ltrimmed = trimmed.trim_start();

        let header_info: Option<(usize, &str)> = if ltrimmed.starts_with("### ") {
            Some((3, ltrimmed.strip_prefix("### ").unwrap_or("")))
        } else if ltrimmed.starts_with("## ") {
            Some((2, ltrimmed.strip_prefix("## ").unwrap_or("")))
        } else if ltrimmed.starts_with("# ") {
            Some((1, ltrimmed.strip_prefix("# ").unwrap_or("")))
        } else if ltrimmed.starts_with("###") {
            let rest = ltrimmed.strip_prefix("###").unwrap_or("");
            Some((3, rest.trim_start()))
        } else if ltrimmed.starts_with("##") {
            let rest = ltrimmed.strip_prefix("##").unwrap_or("");
            if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('-') || rest.starts_with(':') {
                Some((2, rest.trim_start()))
            } else {
                None
            }
        } else if ltrimmed.starts_with('#') {
            let rest = ltrimmed.strip_prefix('#').unwrap_or("");
            if rest.is_empty() || rest.starts_with(' ') {
                Some((1, rest.trim_start()))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((level, name)) = header_info {
            if pending_blank && !result.is_empty() && !result.last().map(|l| l.is_empty()).unwrap_or(false) {
                result.push(String::new());
            }
            pending_blank = true;

            let hashes = "#".repeat(level);
            if let Some(colon_pos) = name.find(':') {
                let header_part = name[..colon_pos].trim();
                let content = name[colon_pos + 1..].trim();
                let mut out = format!("{} {}", hashes, header_part);
                if !content.is_empty() {
                    out.push_str("\n\n");
                    out.push_str(content);
                }
                result.push(out);
            } else {
                result.push(format!("{} {}", hashes, name));
            }
        } else if trimmed.is_empty() {
            pending_blank = true;
        } else {
            result.push(trimmed.to_string());
            pending_blank = false;
        }
    }

    while let Some(last) = result.last() {
        if last.is_empty() {
            result.pop();
        } else {
            break;
        }
    }

    result.join("\n") + "\n"
}

fn build_scribe_prompt(repo: &Path, staged_diff_names: &str, staged_diff_content: Option<&str>) -> String {
    let (git_log, _git_files) = collect_git_context(repo);
    let blueprint = collect_blueprint(repo);

    let diff_section = match staged_diff_content {
        Some(content) => format!(
            r#"ACTUAL DIFF (analyze this to understand WHAT changed and WHY):
{content}

FILE SUMMARY:
{staged_diff_names}"#
        ),
        None => format!(
            r#"FILE CHANGES (no diff available, use file names only):
{staged_diff_names}"#
        ),
    };

    let blueprint_section = if blueprint.is_empty() {
        String::new()
    } else {
        format!(
            r#"

PROJECT BLUEPRINT (current goals):
{}"#,
            &blueprint[..blueprint.len().min(500)]
        )
    };

    format!(
        r#"You are a scribe for a software project. Analyze the code changes and write a concise project-state.md.

{diff_section}{blueprint_section}

RECENT COMMITS (for context, do NOT repeat these):
{git_log}

CRITICAL RULES:
- You MUST analyze the ACTUAL DIFF to understand what changed semantically
- Do NOT write "wip checkpoint" — if work is in progress, describe what IS done
- Do NOT write "File: <filename>" — describe what the code DOES
- Do NOT write generic messages like "Updated files" or "Code changes"
- If diff shows a bug fix, describe: "Fix X by doing Y" (the bug AND the fix)
- If diff shows new feature, describe what it does and why it matters
- If diff shows refactoring, describe what changed and why
- If diff shows docs only, write: "docs(scope): describe what documentation was updated"
- Only list genuinely completed items

BAD examples (DO NOT USE):
- "wip checkpoint"
- "File: src/main.rs"
- "Updated files"
- "chore(misc): *   File: `foo.rs`"

GOOD examples:
- "feat(auth): add JWT validation with 5-minute expiry check"
- "fix(http): retry failed requests with exponential backoff (max 3 attempts)"
- "docs(readme): update installation instructions for Ubuntu 24.04"

GENERATE EXACTLY this markdown structure. Each section header MUST have a blank line after it:

# Project State

## Current Focus
specific description of what this commit does

## Completed
- [x] specific change 1
- [x] specific change 2

No preamble. Only output the markdown."#
    )
}

#[cfg(feature = "scribe")]
pub(crate) async fn update_project_state_from_ai(repo: &Path, staged_diff_names: &str, staged_diff_content: Option<String>) -> anyhow::Result<()> {
    let repo_display = repo.display().to_string();

    let service = SimpleAiService::new();
    if service.is_empty() {
        eprintln!("📝 scribe: no AI API keys configured (set OPENROUTER_API_KEY, GEMINI_API_KEY, or NVIDIA_API_KEY)");
        return Ok(());
    }

    let prompt = build_scribe_prompt(repo, staged_diff_names, staged_diff_content.as_deref());

    let messages = vec![ChatMessage::user(&prompt)];

    match service.chat(messages).await {
        Ok(text) => {
            let dracon_dir = repo.join(".dracon");
            std::fs::create_dir_all(&dracon_dir)?;
            let state_path = dracon_dir.join("project-state.md");

            let markdown = if let Some(start) = text.find("# Project State") {
                &text[start..]
            } else {
                &text
            };

            let cleaned = cleanup_markdown(markdown);
            std::fs::write(&state_path, cleaned)?;
            eprintln!("📝 scribe: updated {}/.dracon/project-state.md", repo_display);
        }
        Err(e) => {
            eprintln!("📝 scribe: AI request failed for {}: {} - committing anyway with fallback", repo_display, e);
        }
    }

    Ok(())
}

#[cfg(not(feature = "scribe"))]
pub(crate) async fn update_project_state_from_ai(_repo: &Path, _staged_diff_names: &str, _staged_diff_content: Option<String>) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_markdown_simple_header() {
        assert_eq!(cleanup_markdown("# Header\n\nSome content"), "# Header\nSome content\n");
    }

    #[test]
    fn test_cleanup_markdown_header_levels() {
        let input = "### Level 3\n## Level 2\n# Level 1";
        let result = cleanup_markdown(input);
        assert!(result.contains("### Level 3"));
        assert!(result.contains("## Level 2"));
        assert!(result.contains("# Level 1"));
    }

    #[test]
    fn test_cleanup_markdown_header_with_colon() {
        let input = "## Current Focus: Doing things";
        let result = cleanup_markdown(input);
        assert!(result.contains("## Current Focus\n\nDoing things"));
    }

    #[test]
    fn test_cleanup_markdown_trims_trailing_whitespace() {
        let input = "Content with trailing   \nMore content";
        let result = cleanup_markdown(input);
        assert!(!result.contains("trailing   "));
        assert!(result.contains("Content with trailing"));
    }

    #[test]
    fn test_cleanup_markdown_removes_trailing_blank_lines() {
        let input = "# Header\n\nContent\n\n\n\n";
        let result = cleanup_markdown(input);
        assert_eq!(result, "# Header\nContent\n");
    }

    #[test]
    fn test_cleanup_markdown_preserves_blank_line_between_headers() {
        let input = "# Header\n\n## Section\n\nContent here";
        let result = cleanup_markdown(input);
        assert_eq!(result, "# Header\n\n## Section\nContent here\n");
    }

    #[test]
    fn test_cleanup_markdown_hash_only_without_space() {
        let input = "##NoSpace";
        let result = cleanup_markdown(input);
        assert!(result.contains("## NoSpace") || result.contains("##NoSpace"));
    }

    #[test]
    fn test_cleanup_markdown_header_with_dash_prefix() {
        let input = "## - Item";
        let result = cleanup_markdown(input);
        assert!(result.contains("## - Item"));
    }

    #[test]
    fn test_cleanup_markdown_empty_input() {
        assert_eq!(cleanup_markdown(""), "\n");
    }

    #[test]
    fn test_cleanup_markdown_multiple_blank_lines_collapsed() {
        let input = "Line1\n\n\n\nLine2";
        let result = cleanup_markdown(input);
        assert!(result.contains("Line1\nLine2"));
    }

    #[test]
    fn test_cleanup_markdown_no_extra_blank_before_non_header() {
        let input = "Paragraph\n\n\n\nAnother";
        let result = cleanup_markdown(input);
        assert_eq!(result, "Paragraph\nAnother\n");
    }

    #[test]
    fn test_cleanup_markdown_trailing_whitespace_lines() {
        let input = "Content   \n   \nMore";
        let result = cleanup_markdown(input);
        assert_eq!(result, "Content\nMore\n");
    }
}
