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

fn resolve_free_model() -> String {
    let policy_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dracon/ai/routing-policy.json");
    
    let content = match std::fs::read_to_string(&policy_path) {
        Ok(c) => c,
        Err(_) => return "openrouter/google/gemma-3-27b-it:free".to_string(),
    };
    
    let policy: RoutingPolicy = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(_) => return "openrouter/google/gemma-3-27b-it:free".to_string(),
    };
    
    policy
        .lane_model_policy
        .get("free:*")
        .and_then(|models| {
            models
                .iter()
                .find(|id| !id.contains("claude") && !id.contains("gpt-4") && !id.contains("o1-"))
                .cloned()
        })
        .unwrap_or_else(|| "openrouter/google/gemma-3-27b-it:free".to_string())
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
    content: String,
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
    
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("OpenRouter returned {}: {}", status, text));
    }
    
    let body: OpenRouterResponse = response
        .json()
        .await
        .context("failed to parse OpenRouter response")?;
    
    body.choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| anyhow!("no choices in OpenRouter response"))
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
    
    let model = resolve_free_model();
    let prompt = build_scribe_prompt(repo);
    
    let client = Client::new();
    let model_clone = model.clone();
    let prompt_clone = prompt.clone();
    
    let result = timeout(
        Duration::from_secs(150),
        async {
            call_openrouter(&client, &api_key, &model_clone, &prompt_clone).await
        },
    )
    .await;
    
    match result {
        Ok(Ok(text)) => {
            let markdown = extract_markdown(&text);
            let dracon_dir = repo.join(".dracon");
            std::fs::create_dir_all(&dracon_dir)?;
            let state_path = dracon_dir.join("project-state.md");
            std::fs::write(&state_path, markdown.trim())?;
            eprintln!("📝 scribe: updated {}/.dracon/project-state.md", repo_display);
            Ok(())
        }
        Ok(Err(e)) => {
            if e.to_string().contains("401") || e.to_string().contains("Unauthorized") {
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
