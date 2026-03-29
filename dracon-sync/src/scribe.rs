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

fn build_scribe_prompt(repo: &Path, staged_diff: &str) -> String {
    let (git_log, git_files) = collect_git_context(repo);
    let blueprint = collect_blueprint(repo);

    format!(
        r#"You are a scribe for a software project. Your job is to write a concise project-state.md.

CRITICAL: The STAGED CHANGES below are the PRIMARY source of truth. Focus on WHAT actually changed, not historical patterns.

## Current Staged Changes (PRIMARY - what is about to be committed)
{staged_diff}

## Recent File Changes (secondary context)
{git_files}

## Recent Git Log (do NOT copy phrasing from these - they may be generic)
{git_log}

## Blueprint (goals)
{blueprint}

## Instructions
Write a project-state.md with EXACTLY this format (no preamble, no explanation). MUST include newlines between every section:

```
# Project State

## Current Focus
{{one line: specific description of WHAT changed, e.g. "Remove Azure Tenant ID pattern that caused false positives" NOT generic "integration" or "finalizing"}}

## Completed
- [x] {{specific completed work}}

## In Progress
- [x] {{what's actively being worked on}}

## Open Issues
- {{anything broken or blocked}}
```

AVOID: generic phrases like "integrating", "finalizing", "AI version", "scribe functionality" - be specific.
Write ONLY the markdown, nothing else."#
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

            std::fs::write(&state_path, markdown.trim())?;
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
