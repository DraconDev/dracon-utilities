use crate::ai::SimpleAiService;
use ai_lanes::ChatMessage;
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
    let mut current_section_content: Option<String> = None;

    fn is_header(line: &str) -> bool {
        line.trim_start().starts_with("# ")
            || line.trim_start().starts_with("## ")
            || line.trim_start().starts_with("### ")
    }

    fn extract_header_name(line: &str) -> Option<&str> {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            Some(&trimmed[2..])
        } else if trimmed.starts_with("## ") {
            Some(&trimmed[3..])
        } else if trimmed.starts_with("### ") {
            Some(&trimmed[4..])
        } else {
            None
        }
    }

    fn header_with_colon(name: &str) -> String {
        if let Some(colon_pos) = name.find(':') {
            let header_part = name[..colon_pos].trim();
            let content = name[colon_pos + 1..].trim();
            let mut out = format!("## {}", header_part);
            if !content.is_empty() {
                out.push_str("\n\n");
                out.push_str(content);
            }
            out
        } else {
            format!("## {}", name)
        }
    }

    for line in input.lines() {
        let trimmed = line.trim_end();

        if is_header(trimmed) {
            if let Some(content) = current_section_content.take() {
                if !content.is_empty() {
                    result.push(content);
                }
            }
            if !result.is_empty() && !result.last().map(|l| l.is_empty()).unwrap_or(false) {
                result.push(String::new());
            }

            if let Some(name) = extract_header_name(trimmed) {
                result.push(header_with_colon(name));
            } else {
                result.push(trimmed.to_string());
            }
            current_section_content = Some(String::new());
        } else if trimmed.is_empty() {
            if let Some(content) = current_section_content.as_mut() {
                if !content.is_empty() {
                    content.push_str("\n\n");
                }
            }
        } else {
            if let Some(content) = current_section_content.as_mut() {
                if !content.is_empty() && !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(trimmed);
            }
        }
    }

    if let Some(content) = current_section_content.take() {
        if !content.is_empty() {
            result.push(content);
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

fn build_scribe_prompt(repo: &Path, staged_diff: &str) -> String {
    let (git_log, git_files) = collect_git_context(repo);
    let blueprint = collect_blueprint(repo);

    format!(
        r#"You are a scribe for a software project. Write a concise project-state.md.

STAGED CHANGES (PRIMARY source):
{staged_diff}

CONTEXT (file names changed):
{git_files}

GENERATE EXACTLY this markdown structure. Each section header MUST have a blank line after it:

# Project State

## Current Focus
ONE LINE describing what changed - be specific like "Fix bug in auth token validation" not generic "Update code"

## Completed
- [x] item 1
- [x] item 2

## In Progress
- [x] item 1

## Open Issues
- issue 1

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

    let messages = vec![ChatMessage::user(prompt)];

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
