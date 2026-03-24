use ai_adapters::GenericOpenAIAdapter;
use ai_router::models::{ChatMessage, ChatRequest};
use ai_router::routing::{RoutingTask, SelectionConstraints};
use ai_router::traits::{AiModelStore, AiProvider};
use ai_routing_service::{AiService, LaneModelPolicy, ProviderRegistry};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

struct RoutingPolicySpec {
    model_id: String,
    api_keys: Vec<String>,
    endpoint: String,
    payload_model: String,
    auth_header_name: String,
    auth_header_prefix: String,
}

struct RoutingPolicyConfig {
    providers: Vec<RoutingPolicySpec>,
    active_model_ids: Vec<String>,
    dev_model_ids: Vec<String>,
    fallback_chain: LaneModelPolicy,
}

fn load_routing_policy() -> Result<RoutingPolicyConfig> {
    let path = dirs::home_dir()
        .context("no home dir")?
        .join(".dracon/ai/routing-policy.json");
    
    if !path.exists() {
        anyhow::bail!("routing policy not found at {}", path.display());
    }
    
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    
    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| "failed to parse routing policy JSON")?;
    
    let providers_array = parsed.get("providers")
        .and_then(|v| v.as_array())
        .context("providers array not found")?;
    
    let mut providers = Vec::new();
    for provider in providers_array {
        let model_id = provider.get("model_id")
            .and_then(|v| v.as_str())
            .context("model_id missing")?
            .to_string();
        
        let api_key_envs = provider.get("api_key_envs")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();
        
        if api_key_envs.is_empty() {
            continue;
        }
        
        let api_keys: Vec<String> = api_key_envs
            .iter()
            .filter_map(|k| std::env::var(k).ok())
            .collect();

        if api_keys.is_empty() {
            continue;
        }
        
        let endpoint = provider.get("endpoint")
            .and_then(|v| v.as_str())
            .unwrap_or("https://openrouter.ai/api/v1")
            .to_string();
        
        let payload_model = provider.get("payload_model")
            .and_then(|v| v.as_str())
            .unwrap_or(&model_id)
            .to_string();
        
        let auth_header_name = provider.get("auth_header_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Authorization")
            .to_string();
        
        let auth_header_prefix = provider.get("auth_header_prefix")
            .and_then(|v| v.as_str())
            .unwrap_or("Bearer ")
            .to_string();
        
        providers.push(RoutingPolicySpec {
            model_id,
            api_keys,
            endpoint,
            payload_model,
            auth_header_name,
            auth_header_prefix,
        });
    }
    
    let active_model_ids: Vec<String> = parsed.get("active_model_ids")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    
    let dev_model_ids: Vec<String> = active_model_ids.iter()
        .filter(|id| id.contains("hunter-alpha"))
        .cloned()
        .collect();
    
    let fallback_chain = parsed.get("lane_model_policy")
        .and_then(|v| v.as_object())
        .map(|obj| {
            let entries: HashMap<String, Vec<String>> = obj.iter()
                .filter_map(|(k, v)| {
                    v.as_array().map(|arr| {
                        (k.clone(), arr.iter().filter_map(|e| e.as_str().map(String::from)).collect())
                    })
                })
                .collect();
            LaneModelPolicy::from_entries(entries)
        })
        .unwrap_or_else(|| LaneModelPolicy::from_entries(HashMap::new()));
    
    Ok(RoutingPolicyConfig {
        providers,
        active_model_ids,
        dev_model_ids,
        fallback_chain,
    })
}

async fn build_ai_service() -> Result<AiService> {
    let config = load_routing_policy()?;
    
    let mut registry: ProviderRegistry<dyn AiProvider> = ProviderRegistry::new();
    
    for spec in &config.providers {
        if !config.active_model_ids.contains(&spec.model_id) {
            continue;
        }
        let adapter = GenericOpenAIAdapter::new_with_auth_keys(
            spec.api_keys.clone(),
            spec.endpoint.clone(),
            spec.model_id.clone(),
            &spec.auth_header_name,
            &spec.auth_header_prefix,
        );
        registry.register(&spec.model_id, Arc::new(adapter));
    }
    
    let store: Arc<dyn AiModelStore> = Arc::new(NoopModelStore);
    Ok(AiService::new(
        registry,
        store,
        config.dev_model_ids,
        config.active_model_ids,
        config.fallback_chain,
    ).await?)
}

struct NoopModelStore;

#[async_trait::async_trait]
impl AiModelStore for NoopModelStore {
    async fn get_best_model(
        &self,
        _task: &str,
        _constraints: SelectionConstraints,
    ) -> Result<(String, bool)> {
        Err(anyhow::anyhow!("NoopModelStore: no data"))
    }

    async fn get_leaderboard(
        &self,
        _req: ai_router::models::LeaderboardRequest,
    ) -> Result<ai_router::models::LeaderboardResponse> {
        Ok(ai_router::models::LeaderboardResponse::default())
    }

    async fn mark_failure(&self, _model_id: &str) -> Result<()> {
        Ok(())
    }

    async fn update_latency(&self, _model_id: &str, _latency_ms: u64) -> Result<()> {
        Ok(())
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

fn build_scribe_prompt(repo: &Path) -> String {
    let (git_log, git_files) = collect_git_context(repo);
    let blueprint = collect_blueprint(repo);

    format!(
        r#"You are a scribe for a software project. Analyze the git history and project state, then write a concise project-state.md.

## Recent Git Log
{git_log}

## Recent File Changes
{git_files}

## Blueprint
{blueprint}

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
Write ONLY the markdown, nothing else."#
    )
}

#[cfg(feature = "scribe")]
pub(crate) async fn update_project_state_from_ai(repo: &Path) -> anyhow::Result<()> {
    let repo_display = repo.display().to_string();
    let prompt = build_scribe_prompt(repo);

    let service = match build_ai_service().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("📝 scribe: failed to build AI service for {}: {}", repo_display, e);
            return Ok(());
        }
    };

    let req = ChatRequest {
        project_id: "scribe".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: prompt,
        }],
        client_intent: Some(RoutingTask::Free),
        ..Default::default()
    };

    let (text, _) = match service.ask_and_collect(req).await {
        Ok(r) => r,
        Err(e) => {
            if e.to_string().contains("no AI provider")
                || e.to_string().contains("401")
                || e.to_string().contains("Unauthorized")
            {
                eprintln!("📝 scribe: skipped (no API key configured)");
                return Ok(());
            }
            eprintln!("📝 scribe: AI request failed for {}: {}", repo_display, e);
            return Ok(());
        }
    };

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

    Ok(())
}

#[cfg(not(feature = "scribe"))]
pub(crate) async fn update_project_state_from_ai(_repo: &Path) -> anyhow::Result<()> {
    Ok(())
}
