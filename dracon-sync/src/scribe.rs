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
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
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
            Some((3, &ltrimmed[4..]))
        } else if ltrimmed.starts_with("## ") {
            Some((2, &ltrimmed[3..]))
        } else if ltrimmed.starts_with("# ") {
            Some((1, &ltrimmed[2..]))
        } else if ltrimmed.starts_with("###") {
            let rest = if ltrimmed.len() >= 3 { &ltrimmed[3..] } else { "" };
            Some((3, rest.trim_start()))
        } else if ltrimmed.starts_with("##") {
            let rest = if ltrimmed.len() >= 2 { &ltrimmed[2..] } else { "" };
            if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('-') || rest.starts_with(':') {
                Some((2, rest.trim_start()))
            } else {
                None
            }
        } else if ltrimmed.starts_with('#') {
            let rest = if ltrimmed.len() >= 1 { &ltrimmed[1..] } else { "" };
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
    let (git_log, git_files) = collect_git_context(repo);
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

RULES:
- Read the ACTUAL DIFF to understand what changed semantically (function signatures, logic changes, bug fixes)
- Do NOT just list file names — describe what the code changes DO
- Be specific: "Add retry logic to HTTP client with exponential backoff" not "Modified http.rs"
- If diff shows a bug fix, describe the bug and the fix
- If diff shows a new feature, describe what it does
- Only list items that are genuinely completed by this change
- "In Progress" should only contain work that is clearly incomplete from the diff
- "Open Issues" should only contain real blockers visible in the code, not "None currently"
- Omit "In Progress" and "Open Issues" sections entirely if there's nothing meaningful to say

GENERATE EXACTLY this markdown structure. Each section header MUST have a blank line after it:

# Project State

## Current Focus
ONE LINE: specific description of what this commit does

## Completed
- [x] specific change 1
- [x] specific change 2

No preamble. Only output the markdown."#
    )
}

#[cfg(feature = "scribe")]
pub(crate) async fn update_project_state_from_ai(repo: &Path, staged_diff: &str) -> anyhow::Result<()> {
    let repo_display = repo.display().to_string();

    let service = SimpleAiService::new();
    if service.is_empty() {
        eprintln!("📝 scribe: no AI API keys configured (set OPENROUTER_API_KEY, GEMINI_API_KEY, or NVIDIA_API_KEY)");
        return Ok(());
    }

    let prompt = build_scribe_prompt(repo, staged_diff);

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
            eprintln!("📝 scribe: AI request failed for {}: {}", repo_display, e);
        }
    }

    Ok(())
}

#[cfg(not(feature = "scribe"))]
pub(crate) async fn update_project_state_from_ai(_repo: &Path, _staged_diff: &str) -> anyhow::Result<()> {
    Ok(())
}
