use ai_adapters::HttpProviderAdapter;
use ai_lanes::ChatMessage;
use ai_router::AiProvider;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub endpoint: String,
    pub model: String,
    pub api_key_env: String,
    pub auth_header: String,
    pub auth_prefix: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AiProvidersConfig {
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
            let api_key = match std::env::var(&pc.api_key_env) {
                Ok(k) if !k.is_empty() => k,
                _ => continue,
            };

            let adapter: Arc<dyn AiProvider> = Arc::new(
                HttpProviderAdapter::new_with_auth(
                    api_key,
                    pc.endpoint,
                    pc.model,
                    &pc.auth_header,
                    &pc.auth_prefix,
                )
            );

            providers.push((pc.name.clone(), adapter));
            eprintln!("📡 AI: {} ready ({})", pc.name, pc.model);
        }

        Self { providers }
    }

    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".dracon/ai/providers.json")
    }

    fn load_config(path: &PathBuf) -> Result<AiProvidersConfig> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config: AiProvidersConfig = serde_json::from_str(&content)
            .with_context(|| "failed to parse providers.json")?;
        Ok(config)
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> anyhow::Result<String> {
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
