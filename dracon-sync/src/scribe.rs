use ai_adapters::HttpProviderAdapter;
use ai_lanes::{AiRequest, ChatMessage, Lane};
use ai_router::AiProvider;
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

struct SimpleAiService {
    providers: Vec<(String, Arc<dyn AiProvider>)>,
}

impl SimpleAiService {
    fn new() -> Self {
        let mut providers = Vec::new();

        if let Some(key) = std::env::var("OPENROUTER_API_KEY").ok().filter(|k| !k.is_empty()) {
            let adapter = Arc::new(HttpProviderAdapter::new_with_auth(
                key,
                "https://openrouter.ai/api/v1".to_string(),
                "google/gemini-2.0-flash-thinking-exp".to_string(),
                "Authorization",
                "Bearer ",
            ));
            providers.push(("openrouter".to_string(), adapter));
            eprintln!("📡 AI: OpenRouter ready");
        }

        if let Some(key) = std::env::var("GEMINI_API_KEY").ok().filter(|k| !k.is_empty()) {
            let adapter = Arc::new(HttpProviderAdapter::new_with_auth(
                key,
                "https://generativelanguage.googleapis.com/v1beta".to_string(),
                "gemini-2.0-flash-exp".to_string(),
                "x-goog-api-key",
                "",
            ));
            providers.push(("gemini".to_string(), adapter));
            eprintln!("📡 AI: Gemini ready");
        }

        if let Some(key) = std::env::var("NVIDIA_API_KEY").ok().filter(|k| !k.is_empty()) {
            let adapter = Arc::new(HttpProviderAdapter::new_with_auth(
                key,
                "https://integrate.api.nvidia.com/v1".to_string(),
                "nvidia/llama-3.3-nemotron-70b-instruct".to_string(),
                "Authorization",
                "Bearer ",
            ));
            providers.push(("nvidia".to_string(), adapter));
            eprintln!("📡 AI: NVIDIA ready");
        }

        Self { providers }
    }

    fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let mut last_error = None;

        for (name, provider) in &self.providers {
            match provider.ask_and_collect(messages.clone()).await {
                Ok((content, _)) => return Ok(content),
                Err(e) => {
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("401") || msg.contains("unauthorized") || msg.contains("api key") {
                        eprintln!("⚠️ AI {}: auth error (key invalid?)", name);
                    } else if msg.contains("429") || msg.contains("rate limit") {
                        eprintln!("⚠️ AI {}: rate limited, trying next...", name);
                    } else {
                        eprintln!("⚠️ AI {} failed: {}", name, e);
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("no AI providers available")))
    }
}

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
        r#"You are a scribe for a software project. Analyze the git history, current changes, and project state, then write a concise project-state.md.

## Recent Git Log (history context)
{git_log}

## Recent File Changes
{git_files}

## Blueprint (goals)
{blueprint}

## Current Staged Changes (what is about to be committed)
{staged_diff}

## Instructions
Write a project-state.md file with EXACTLY this format (no preamble, no explanation):

# Project State

## Current Focus
{{one line: what the project is actively working on NOW, based on the staged changes, recent commits, and blueprint}}

## Completed
- [x] {{recent completed work from the log}}

## In Progress
- [x] {{what's actively being worked on based on recent file patterns}}

## Open Issues
- {{anything that looks broken or blocked based on the evidence}}

Keep it factual. Infer from the evidence, don't make things up. If unclear, say so.
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
