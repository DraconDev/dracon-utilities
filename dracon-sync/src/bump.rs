use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use async_trait::async_trait;
use ai_routing_runtime::traits::{AiProvider, AiModelStore};
use ai_routing_runtime::models::{ChatMessage, ChatRequest, ChatResponse, UsageStats};
use ai_routing_runtime::{RoutingMessage, SmartRouter, LaneModelPolicy, ProviderRegistry};
use ai_service::AiService;
use futures::StreamExt;

pub(crate) fn bump_semver_patch(ver: &str) -> Option<String> {
    let parts: Vec<&str> = ver.split('.').collect();
    if parts.len() < 3 {
        return None;
    }
    if !parts[0].chars().all(|c| c.is_ascii_digit())
        || !parts[1].chars().all(|c| c.is_ascii_digit())
        || !parts[2].chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let major: u64 = parts[0].parse().ok()?;
    let minor: u64 = parts[1].parse().ok()?;
    let patch: u64 = parts[2].parse().ok()?;
    Some(format!("{}.{}.{}", major, minor, patch + 1))
}

pub(crate) fn bump_semver_minor(ver: &str) -> Option<String> {
    let parts: Vec<&str> = ver.split('.').collect();
    if parts.len() < 3 {
        return None;
    }
    if !parts[0].chars().all(|c| c.is_ascii_digit())
        || !parts[1].chars().all(|c| c.is_ascii_digit())
        || !parts[2].chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let major: u64 = parts[0].parse().ok()?;
    let minor: u64 = parts[1].parse().ok()?;
    let patch: u64 = parts[2].parse().ok()?;
    Some(format!("{}.{}.{}", major, minor + 1, 0))
}

pub(crate) fn bump_semver_major(ver: &str) -> Option<String> {
    let parts: Vec<&str> = ver.split('.').collect();
    if parts.len() < 3 {
        return None;
    }
    if !parts[0].chars().all(|c| c.is_ascii_digit())
        || !parts[1].chars().all(|c| c.is_ascii_digit())
        || !parts[2].chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let major: u64 = parts[0].parse().ok()?;
    let minor: u64 = parts[1].parse().ok()?;
    let patch: u64 = parts[2].parse().ok()?;
    Some(format!("{}.{}.{}", major + 1, 0, 0))
}

fn extract_version_from_cargo(content: &str) -> Option<String> {
    let mut section = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed.trim_matches(&['[', ']'][..]).trim().to_string();
        }
        if section == "package" || section == "workspace.package" {
            if let Some(rest) = trimmed.strip_prefix("version") {
                let rest = rest.trim_start().trim_start_matches('=').trim();
                if let Some(v) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn extract_version_from_json(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let mut start = 0usize;
    while let Some(idx) = content[start..].find(&needle) {
        let key_pos = start + idx;
        let after_key = key_pos + needle.len();
        let rest = &content[after_key..];
        let colon_rel = rest.find(':')?;
        let after_colon = after_key + colon_rel + 1;
        let rest2 = &content[after_colon..];
        let q1_rel = rest2.find('"')?;
        let q1 = after_colon + q1_rel + 1;
        let rest3 = &content[q1..];
        let q2_rel = rest3.find('"')?;
        let q2 = q1 + q2_rel;
        return Some(content[q1..q2].to_string());
    }
    None
}

pub(crate) fn read_current_version(repo: &Path) -> Option<String> {
    if let Ok(cargo) = std::fs::read_to_string(repo.join("Cargo.toml")) {
        if let Some(version) = extract_version_from_cargo(&cargo) {
            return Some(version);
        }
    }
    if let Ok(pkg) = std::fs::read_to_string(repo.join("package.json")) {
        if let Some(version) = extract_version_from_json(&pkg, "version") {
            return Some(version);
        }
    }
    if let Ok(version_file) = std::fs::read_to_string(repo.join("VERSION")) {
        let trimmed = version_file.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BumpLevel {
    Major,
    Minor,
    Patch,
    None,
}

impl BumpLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            BumpLevel::Major => "major",
            BumpLevel::Minor => "minor",
            BumpLevel::Patch => "patch",
            BumpLevel::None => "none",
        }
    }
}

struct NoOpModelStore;

#[async_trait]
impl AiModelStore for NoOpModelStore {
    async fn get_best_model(&self, _task: &str, _constraints: ai_routing_runtime::routing::SelectionConstraints) -> anyhow::Result<(String, bool)> {
        Ok(("openrouter/free".to_string(), true))
    }
    async fn get_leaderboard(&self, _req: ai_routing_runtime::models::LeaderboardRequest) -> anyhow::Result<ai_routing_runtime::models::LeaderboardResponse> {
        Ok(ai_routing_runtime::models::LeaderboardResponse {
            models: vec![],
            max_quality_score: 0.0,
            max_coding_score: 0.0,
        })
    }
    async fn mark_failure(&self, _model_id: &str) -> anyhow::Result<()> { Ok(()) }
    async fn update_latency(&self, _model_id: &str, _latency_ms: u64) -> anyhow::Result<()> { Ok(()) }
}

struct OpenRouterProvider {
    client: reqwest::Client,
    api_key: String,
    endpoint: String,
}

impl OpenRouterProvider {
    fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            endpoint: "https://openrouter.ai/api/v1/chat/completions".to_string(),
        }
    }
}

#[async_trait]
impl AiProvider for OpenRouterProvider {
    async fn generate_response(&self, prompt: &str) -> anyhow::Result<String> {
        let request = serde_json::json!({
            "model": "openrouter/free",
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 20
        });
        
        let response = self.client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenRouter error: {}", err_text));
        }
        
        #[derive(Deserialize)]
        struct OpenRouterResponse {
            choices: Vec<Choice>,
        }
        #[derive(Deserialize)]
        struct Choice {
            message: Message,
        }
        #[derive(Deserialize)]
        struct Message {
            content: Option<String>,
        }
        
        let body: OpenRouterResponse = response.json().await?;
        body.choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("no content in response"))
    }

    async fn stream_response(&self, prompt: &str) -> anyhow::Result<futures::stream::Iter<std::vec::IntoIter<Result<String>>>> {
        let response = self.generate_response(prompt).await?;
        Ok(futures::stream::iter(vec![Ok(response)]))
    }

    async fn ask(&self, req: ChatRequest) -> anyhow::Result<futures::stream::Iter<std::vec::IntoIter<Result<ChatResponse>>>> {
        let content = self.generate_response(&req.messages.first().map(|m| m.content.as_str()).unwrap_or("")).await?;
        Ok(futures::stream::iter(vec![Ok(ChatResponse {
            token: content.clone(),
            kind: None,
            finish_reason: Some("stop".to_string()),
            usage: None,
        })]))
    }

    async fn ask_and_collect(&self, req: ChatRequest) -> anyhow::Result<(String, Option<UsageStats>)> {
        let content = self.generate_response(&req.messages.first().map(|m| m.content.as_str()).unwrap_or("")).await?;
        Ok((content, None))
    }
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

fn resolve_routing_policy() -> LaneModelPolicy {
    let policy_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".dracon/ai/routing-policy.json");
    
    let content = std::fs::read_to_string(&policy_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    
    LaneModelPolicy::from_entries(
        value.get("lane_model_policy")
            .and_then(|v| v.as_object())
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| {
                        v.as_array().map(|arr| {
                            (k.clone(), arr.iter().filter_map(|s| s.as_str().map(String::from)).collect())
                        })
                    })
                    .collect()
            })
    )
}

pub async fn ai_decide_bump_level(
    _repo: &Path,
    current_version: &str,
    staged_diff: &str,
    _project_state: &str,
) -> BumpLevel {
    let version_only_patterns = ["Cargo.toml", "package.json", "VERSION", "Cargo.lock"];
    let has_source_changes = staged_diff.lines()
        .filter(|line| !line.is_empty())
        .any(|line| {
            !version_only_patterns.iter().any(|p| line.contains(p))
        });
    
    if !has_source_changes {
        return BumpLevel::None;
    }
    
    let api_key = match resolve_openrouter_key() {
        Some(k) => k,
        None => return BumpLevel::None,
    };
    
    let prompt = format!(r##"Version bump advisor. Respond with ONE WORD only.
Current: {current_version}
Changes: {staged_diff}
Respond: major, minor, patch, or none"##);
    
    let provider = OpenRouterProvider::new(api_key);
    let response = match provider.generate_response(&prompt).await {
        Ok(r) => r.to_lowercase(),
        Err(_) => return BumpLevel::None,
    };
    
    let response = response.trim();
    if response.contains("major") {
        BumpLevel::Major
    } else if response.contains("minor") {
        BumpLevel::Minor
    } else if response.contains("patch") {
        BumpLevel::Patch
    } else {
        BumpLevel::None
    }
}

pub fn apply_version_bump_to_repo(repo: &Path, old_ver: &str, new_ver: &str) -> bool {
    if repo.join("Cargo.toml").exists() {
        if let Ok(content) = std::fs::read_to_string(repo.join("Cargo.toml")) {
            let bumped = content.replace(&format!("version = \"{}\"", old_ver), &format!("version = \"{}\"", new_ver))
                .replace(&format!("version=\"{}\"", old_ver), &format!("version=\"{}\"", new_ver));
            if bumped != content {
                if std::fs::write(repo.join("Cargo.toml"), bumped).is_ok() {
                    return true;
                }
            }
        }
    }
    if repo.join("package.json").exists() {
        if let Ok(content) = std::fs::read_to_string(repo.join("package.json")) {
            let bumped = content.replace(&format!("\"version\": \"{}\"", old_ver), &format!("\"version\": \"{}\"", new_ver));
            if bumped != content {
                if std::fs::write(repo.join("package.json"), bumped).is_ok() {
                    return true;
                }
            }
        }
    }
    if repo.join("VERSION").exists() {
        if std::fs::write(repo.join("VERSION"), format!("{}\n", new_ver)).is_ok() {
            return true;
        }
    }
    false
}
