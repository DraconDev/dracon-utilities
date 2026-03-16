use anyhow::{Context, Result};
use serde_json::json;
use std::path::Path;
use std::process::Command as StdCommand;

/// Observe a repo using AI and update .dracon/project-state.md
pub async fn update_project_state(repo: &Path) -> Result<()> {
    let context = collect_context(repo);
    let response = call_ai(&context).await?;
    let dracon_dir = repo.join(".dracon");
    std::fs::create_dir_all(&dracon_dir)?;
    let state_path = dracon_dir.join("project-state.md");

    // Extract markdown from response
    let markdown = if let Some(start) = response.find("# Project State") {
        response[start..].trim().to_string()
    } else {
        response.trim().to_string()
    };

    std::fs::write(&state_path, &markdown)
        .with_context(|| format!("writing {}", state_path.display()))?;
    eprintln!("📝 scribe: updated {}", state_path.display());

    Ok(())
}

struct RepoContext {
    git_log: String,
    git_files: String,
    blueprint: String,
}

fn collect_context(repo: &Path) -> RepoContext {
    let git_log = StdCommand::new("git")
        .args(["log", "--format=%s", "-20"])
        .current_dir(repo)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|_| "no git history".to_string());

    let git_files = StdCommand::new("git")
        .args(["log", "--oneline", "--name-only", "-10"])
        .current_dir(repo)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let blueprint = {
        let plan_dir = repo.join("plan");
        if plan_dir.exists() {
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
        } else {
            String::new()
        }
    };

    RepoContext {
        git_log,
        git_files,
        blueprint,
    }
}

async fn call_ai(ctx: &RepoContext) -> Result<String> {
    let resolved = ai_runtime_config::resolve_ai_runtime_config();

    // Find the first active provider with an API key
    let provider = resolved
        .openai_providers
        .iter()
        .find(|p| !p.api_keys.is_empty() && !p.api_keys[0].is_empty())
        .ok_or_else(|| anyhow::anyhow!("no AI provider configured with API key"))?;

    let api_key = &provider.api_keys[0];
    let endpoint = &provider.endpoint;
    let model = &provider.payload_model;

    let prompt = format!(
        r#"You are a scribe for a software project. Analyze the git history and write a concise project-state.md.

## Recent Git Log
{}

## Recent File Changes
{}

## Blueprint
{}

Write a project-state.md with EXACTLY this format (no preamble):

# Project State

## Current Focus
{{one line: what the project is actively working on}}

## Completed
- [x] {{recent completed work}}

## In Progress
- [ ] {{what's actively being worked on}}

## Open Issues
- {{blockers or things that look broken}}

Keep it factual. Infer from evidence, don't make things up."#,
        ctx.git_log, ctx.git_files, ctx.blueprint
    );

    let body = json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 1000,
        "temperature": 0.3,
    });

    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));

    let resp = client
        .post(&url)
        .header(
            &provider.auth_header_name,
            format!("{}{}", provider.auth_header_prefix, api_key),
        )
        .json(&body)
        .send()
        .await
        .with_context(|| format!("AI request to {}", endpoint))?;

    let status = resp.status();
    if !status.is_success() {
        // Keep auth failure messages short (happens on every sync when key is missing)
        if status == reqwest::StatusCode::UNAUTHORIZED {
            anyhow::bail!("AI provider: auth failed (check ~/.dracon/ai/secrets/)");
        }
        let err_text = resp.text().await.unwrap_or_default();
        anyhow::bail!("AI provider returned {}: {}", status, err_text);
    }

    let json: serde_json::Value = resp.json().await?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("AI response missing content"))?;

    Ok(content.to_string())
}
