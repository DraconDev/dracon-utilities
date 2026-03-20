use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Deserialize)]
struct RoutingPolicy {
    #[serde(rename = "lane_model_policy")]
    lane_model_policy: HashMap<String, Vec<String>>,
}

fn resolve_openrouter_key() -> Option<String> {
    let env_path = dirs::home_dir()?.join(".dracon/ai/secrets/openrouter.env");
    let content = std::fs::read_to_string(&env_path).ok()?;
    for line in content.lines() {
        if line.starts_with("OPENROUTER_API_KEY=") {
            return Some(line.split('=').nth(1)?.trim().to_string());
        }
    }
    None
}

fn resolve_free_models() -> Vec<String> {
    let policy_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dracon/ai/routing-policy.json");
    
    let content = match std::fs::read_to_string(&policy_path) {
        Ok(c) => c,
        Err(_) => return vec!["openrouter/google/gemma-3-27b-it:free".to_string()],
    };
    
    let policy: RoutingPolicy = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(_) => return vec!["openrouter/google/gemma-3-27b-it:free".to_string()],
    };
    
    policy
        .lane_model_policy
        .get("free:*")
        .map(|models| {
            models
                .iter()
                .filter(|id| !id.contains("claude") && !id.contains("gpt-4") && !id.contains("o1-"))
                .cloned()
                .collect()
        })
        .unwrap_or_else(|| vec!["openrouter/google/gemma-3-27b-it:free".to_string()])
}

fn collect_git_log(repo: &Path) -> String {
    use std::process::Command as StdCommand;
    let output = StdCommand::new("git")
        .args(["log", "--format=%s%n  files: %(trailers:key=file,valueonly)", "-20"])
        .current_dir(repo)
        .output();
    
    output
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|_| "no git history".to_string())
}

fn collect_git_diff(repo: &Path) -> String {
    use std::process::Command as StdCommand;
    let output = StdCommand::new("git")
        .args(["diff", "--stat", "-5"])
        .current_dir(repo)
        .output();
    
    output
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

fn collect_git_files(repo: &Path) -> String {
    use std::process::Command as StdCommand;
    let output = StdCommand::new("git")
        .args(["log", "--oneline", "--name-only", "-10"])
        .current_dir(repo)
        .output();
    
    output
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

fn read_blueprint(repo: &Path) -> String {
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

fn read_current_state(repo: &Path) -> String {
    let state_path = repo.join(".dracon/project-state.md");
    std::fs::read_to_string(&state_path).unwrap_or_default()
}

#[derive(Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    max_tokens: i32,
}

#[derive(Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
}

#[derive(Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterMessageResponse,
}

#[derive(Deserialize)]
struct OpenRouterMessageResponse {
    content: Option<String>,
}

fn build_scribe_prompt(repo: &Path) -> String {
    let git_log = collect_git_log(repo);
    let git_files = collect_git_files(repo);
    let git_diff = collect_git_diff(repo);
    let blueprint = read_blueprint(repo);
    let current_state = read_current_state(repo);
    
    let mut prompt = r#"You are a scribe for a software project. Analyze the git history and current state, then update project-state.md.

## Current project-state.md (if any)
"#
    .to_string();
    prompt.push_str(&current_state);
    prompt.push_str("\n\n## Recent Git Log\n");
    prompt.push_str(&git_log);
    prompt.push_str("\n## Recent File Changes\n");
    prompt.push_str(&git_files);
    prompt.push_str("\n## Diff Stats\n");
    prompt.push_str(&git_diff);
    
    if !blueprint.is_empty() {
        prompt.push_str("\n## Blueprint\n");
        prompt.push_str(&blueprint);
    }
    
    prompt.push_str(r##"

## Instructions
Write a project-state.md file with EXACTLY this format (no preamble, no explanation):

# Project State

## Current Focus
{{one line: what the project is actively working on, based on recent commits and blueprint}}

## Completed
- [x] {{recent completed work from the log}}

## In Progress
- [x] {{what's actively being worked on based on recent file patterns}}

## Open Issues
- {{anything that looks broken or blocked based on the evidence}}

Keep it factual. Infer from the evidence, don't make things up. If unclear, say so.
Write ONLY the markdown, nothing else. Start your response with the line "# Project State" and nothing else before it."##);
    
    prompt
}

async fn call_openrouter(client: &Client, api_key: &str, model: &str, prompt: &str) -> Result<String> {
    let request = OpenRouterRequest {
        model: model.to_string(),
        messages: vec![OpenRouterMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        max_tokens: 1024,
    };
    
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(140))
        .json(&request)
        .send()
        .await
        .context("failed to send request to OpenRouter")?;
    
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    
    if !status.is_success() {
        return Err(anyhow!("OpenRouter returned {}: {}", status, text));
    }
    
    let body: OpenRouterResponse = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(v) => {
            serde_json::from_value(v).map_err(|e| anyhow!("response struct mismatch: {} - raw: {}", e, &text[..text.len().min(200)]))?
        }
        Err(e) => {
            return Err(anyhow!("failed to parse OpenRouter response as JSON: {} - raw: {}", e, &text[..text.len().min(500)]));
        }
    };
    
    body.choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .ok_or_else(|| anyhow!("no choices in OpenRouter response or content was null"))
}

fn extract_markdown(text: &str) -> &str {
    if let Some(start) = text.find("# Project State") {
        &text[start..]
    } else {
        text.trim()
    }
}

#[cfg(feature = "scribe")]
pub(crate) async fn update_project_state_from_ai(repo: &Path) -> anyhow::Result<()> {
    let repo_display = repo.display().to_string();
    let api_key = match resolve_openrouter_key() {
        Some(k) => k,
        None => {
            eprintln!("📝 scribe: skipped (no OPENROUTER_API_KEY)");
            return Ok(());
        }
    };
    
    let models = resolve_free_models();
    let prompt = build_scribe_prompt(repo);
    
    let client = Client::new();
    
    let result = timeout(
        Duration::from_secs(150),
        async {
            let mut last_err = None;
            for model in &models {
                match call_openrouter(&client, &api_key, model, &prompt).await {
                    Ok(text) => return Ok((model.clone(), text)),
                    Err(e) => {
                        let err_str = e.to_string().to_lowercase();
                        // Rate limit or temporary failure - try next model
                        if err_str.contains("rate limit") 
                            || err_str.contains("429") 
                            || err_str.contains("no choices") 
                            || err_str.contains("null") 
                            || err_str.contains("timeout") {
                            last_err = Some(e);
                            continue;
                        }
                        // Permanent error - stop trying
                        return Err(e);
                    }
                }
            }
            Err(last_err.unwrap_or_else(|| anyhow!("all models failed")))
        },
    )
    .await;
    
    match result {
        Ok(Ok((model_used, text))) => {
            let markdown = extract_markdown(&text);
            let dracon_dir = repo.join(".dracon");
            std::fs::create_dir_all(&dracon_dir)?;
            let state_path = dracon_dir.join("project-state.md");
            std::fs::write(&state_path, markdown.trim())?;
            eprintln!("📝 scribe: updated {}/.dracon/project-state.md (model: {})", repo_display, model_used);
            Ok(())
        }
        Ok(Err(e)) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("401") || err_str.contains("unauthorized") {
                eprintln!("📝 scribe: skipped (invalid API key)");
                return Ok(());
            }
            eprintln!("📝 scribe: failed for {}: {}", repo_display, e);
            Err(e)
        }
        Err(_) => {
            eprintln!("📝 scribe: timed out after 150s for {}", repo_display);
            Ok(())
        }
    }
}

#[cfg(not(feature = "scribe"))]
pub(crate) async fn update_project_state_from_ai(_repo: &Path) -> anyhow::Result<()> {
    Ok(())
}
