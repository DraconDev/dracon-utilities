use ai_adapters::{GeminiAdapter, HttpProviderAdapter};
use ai_lanes::ChatMessage;
use ai_router::AiProvider;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub env: String,
    pub endpoint: String,
    pub model: String,
    #[serde(default = "default_auth_header")]
    pub auth_header: String,
    #[serde(default = "default_auth_prefix")]
    pub auth_prefix: String,
    #[serde(default = "default_adapter_type")]
    pub adapter: String,
}

fn default_auth_header() -> String {
    "Authorization".to_string()
}

fn default_auth_prefix() -> String {
    "Bearer ".to_string()
}

fn default_adapter_type() -> String {
    "http".to_string()
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AiConfig {
    pub providers: Vec<ProviderConfig>,
}

pub struct SimpleAiService {
    providers: Vec<(String, Arc<dyn AiProvider>)>,
}

impl SimpleAiService {
    pub fn new() -> Self {
        let config_path = Self::config_path();
        let config = match Self::load_config(&config_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("📡 AI: no config at {}: {}", config_path.display(), e);
                return Self { providers: Vec::new() };
            }
        };

        let mut providers = Vec::new();

        for pc in config.providers {
            let name = pc.name.clone();

            let api_key = Self::get_api_key(&pc.env);

            if api_key.is_empty() {
                continue;
            }

            let adapter: Arc<dyn AiProvider> = if pc.adapter == "gemini" {
                Arc::new(GeminiAdapter::new_with_auth_keys(
                    vec![api_key],
                    pc.endpoint,
                    pc.model,
                    &pc.auth_header,
                    &pc.auth_prefix,
                ))
            } else {
                Arc::new(HttpProviderAdapter::new_with_auth(
                    api_key,
                    pc.endpoint,
                    pc.model,
                    &pc.auth_header,
                    &pc.auth_prefix,
                ))
            };

            providers.push((name.clone(), adapter));
            eprintln!("📡 AI: {} ready", name);
        }

        Self { providers }
    }

    fn get_api_key(env_name: &str) -> String {
        if let Ok(key) = std::env::var(env_name) {
            if !key.is_empty() {
                return key;
            }
        }

        let secrets_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".dracon/ai/secrets");

        if let Ok(entries) = std::fs::read_dir(&secrets_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "env") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        for line in content.lines() {
                            let line = line.trim();
                            if line.is_empty() || line.starts_with('#') {
                                continue;
                            }
                            if let Some((key, value)) = line.split_once('=') {
                                if key.trim() == env_name {
                                    let value = value.trim();
                                    if !value.is_empty() {
                                        return value.to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        String::new()
    }

    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".dracon/ai.toml")
    }

    fn load_config(path: &PathBuf) -> Result<AiConfig> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config: AiConfig = toml::from_str(&content)
            .with_context(|| "failed to parse ai.toml")?;
        Ok(config)
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    pub fn provider_names(&self) -> Vec<String> {
        self.providers.iter().map(|(n, _)| n.clone()).collect()
    }

    pub async fn test_provider(&self, name: &str) -> Result<(bool, String)> {
        let messages = vec![ChatMessage::user("Say exactly 'OK'.".to_string())];
        
        for (provider_name, provider) in &self.providers {
            if provider_name != name {
                continue;
            }
            match provider.ask_and_collect(messages).await {
                Ok((content, _)) => return Ok((true, content)),
                Err(e) => return Ok((false, e.to_string())),
            }
        }
        Ok((false, "provider not found".to_string()))
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> anyhow::Result<String> {
        let mut last_error = None;

        for (name, provider) in &self.providers {
            match provider.ask_and_collect(messages.clone()).await {
                Ok((content, _)) => return Ok(content),
                Err(e) => {
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("401") || msg.contains("unauthorized") || msg.contains("api key") {
                        eprintln!("⚠️ AI {}: auth error", name);
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
