use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
enum ProviderStatus {
    AuthFailed,
    RateLimited { until: Instant },
    Healthy,
}

fn provider_health() -> &'static Mutex<HashMap<String, ProviderStatus>> {
    static HEALTH: OnceLock<Mutex<HashMap<String, ProviderStatus>>> = OnceLock::new();
    HEALTH.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: &str) -> Self {
        ChatMessage {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub env: String,
    pub endpoint: String,
    pub model: String,
    #[serde(default = "default_auth_header")]
    pub auth_header: String,
    #[serde(default = "default_auth_prefix")]
    pub auth_prefix: String,
    #[serde(default)]
    pub is_google_api: bool,
}

fn default_auth_header() -> String {
    "Authorization".to_string()
}

fn default_auth_prefix() -> String {
    "Bearer ".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiConfig {
    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Serialize)]
struct RequestMessage {
    role: String,
    content: String,
}

impl From<ChatMessage> for RequestMessage {
    fn from(msg: ChatMessage) -> Self {
        RequestMessage {
            role: msg.role,
            content: msg.content,
        }
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<RequestMessage>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

pub struct SimpleAiService {
    providers: Vec<ProviderConfig>,
    client: reqwest::Client,
}

impl SimpleAiService {
    pub fn new() -> Self {
        let config_path = Self::config_path();
        let config = match Self::load_config(&config_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("📡 AI: no config at {}: {}", config_path.display(), e);
                return Self {
                    providers: Vec::new(),
                    client: reqwest::Client::new(),
                };
            }
        };

        for pc in &config.providers {
            let name = &pc.name;
            if let Some(_key) = Self::get_api_key(&pc.env) {
                eprintln!("📡 AI: {} ready (key found)", name);
            } else {
                eprintln!("📡 AI: {} configured but no API key (set {} env var)", name, pc.env);
            }
        }

        let active_providers: Vec<ProviderConfig> = config
            .providers
            .into_iter()
            .filter(|p| Self::get_api_key(&p.env).is_some())
            .collect();

        Self {
            providers: active_providers,
            client: reqwest::Client::new(),
        }
    }

    fn get_api_key(env_name: &str) -> Option<String> {
        if let Ok(key) = std::env::var(env_name) {
            if !key.is_empty() {
                return Some(key);
            }
        }

        let secrets_dir = Self::secrets_path();

        if let Ok(entries) = std::fs::read_dir(&secrets_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "env") {
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
                                        return Some(value.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".dracon/ai.toml")
    }

    fn secrets_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".dracon/utilities/sync")
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
        self.providers.iter().map(|p| p.name.clone()).collect()
    }

    pub async fn reset_health() {
        provider_health().lock().await.clear();
    }

    pub async fn test_provider(&self, name: &str) -> Result<(bool, String)> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "Say exactly 'OK'.".to_string(),
        }];

        for pc in &self.providers {
            if pc.name != name {
                continue;
            }
            match self.call_provider(pc, messages.clone()).await {
                Ok(content) => return Ok((true, content)),
                Err(e) => return Ok((false, e.to_string())),
            }
        }
        Ok((false, "provider not found".to_string()))
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let mut last_error = None;

        for pc in &self.providers {
            // Check provider health before calling
            {
                let health = provider_health().lock().await;
                match health.get(&pc.name) {
                    Some(ProviderStatus::AuthFailed) => {
                        continue;
                    }
                    Some(ProviderStatus::RateLimited { until }) => {
                        if Instant::now() < *until {
                            continue;
                        }
                    }
                    _ => {}
                }
            }

            match self.call_provider(pc, messages.clone()).await {
                Ok(content) => {
                    provider_health().lock().await.insert(pc.name.clone(), ProviderStatus::Healthy);
                    return Ok(content);
                }
                Err(e) => {
                    let msg = e.to_string().to_lowercase();
                    eprintln!("⚠️ AI {} failed: {}", pc.name, e);
                    let mut health = provider_health().lock().await;
                    if msg.contains("401") || msg.contains("403") || msg.contains("unauthorized") || msg.contains("api key") {
                        eprintln!("🔑 {}: auth error (skipping for session)", pc.name);
                        health.insert(pc.name.clone(), ProviderStatus::AuthFailed);
                    } else if msg.contains("429") || msg.contains("rate limit") {
                        eprintln!("⏳ {}: rate limited (skipping 60s)", pc.name);
                        health.insert(pc.name.clone(), ProviderStatus::RateLimited {
                            until: Instant::now() + Duration::from_secs(60),
                        });
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("no AI providers available")))
    }

    async fn call_provider(&self, provider: &ProviderConfig, messages: Vec<ChatMessage>) -> Result<String> {
        let Some(api_key) = Self::get_api_key(&provider.env) else {
            anyhow::bail!("no API key for {}", provider.env);
        };

        if provider.is_google_api {
            return self.call_google_api(provider, &api_key, messages).await;
        }

        let request_messages: Vec<RequestMessage> = messages.into_iter().map(|m| m.into()).collect();
        let request = ChatRequest {
            model: provider.model.clone(),
            messages: request_messages,
        };

        let url = format!("{}/chat/completions", provider.endpoint.trim_end_matches('/'));

        let response = self
            .client
            .post(&url)
            .header(&provider.auth_header, format!("{}{}", provider.auth_prefix, api_key))
            .json(&request)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .context("request failed")?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("{}: {} - {}", status, text.chars().take(100).collect::<String>(), provider.name);
        }

        let chat_resp: ChatResponse = response.json().await.context("parse response")?;

        chat_resp
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .context("no choices in response")
    }

    async fn call_google_api(&self, provider: &ProviderConfig, api_key: &str, messages: Vec<ChatMessage>) -> Result<String> {
        #[derive(Serialize)]
        struct GoogleRequest {
            contents: Vec<Content>,
        }

        #[derive(Serialize)]
        struct Content {
            role: String,
            parts: Vec<Part>,
        }

        #[derive(Serialize)]
        struct Part {
            text: String,
        }

        #[derive(Deserialize)]
        struct GoogleResponse {
            candidates: Vec<Candidate>,
        }

        #[derive(Deserialize)]
        struct Candidate {
            content: ContentResponse,
        }

        #[derive(Deserialize)]
        struct ContentResponse {
            parts: Vec<TextPart>,
        }

        #[derive(Deserialize)]
        struct TextPart {
            text: String,
        }

        let google_messages: Vec<Content> = messages
            .into_iter()
            .map(|m| Content {
                role: if m.role == "user" { "user" } else { "model" }.to_string(),
                parts: vec![Part { text: m.content }],
            })
            .collect();

        let request = GoogleRequest {
            contents: google_messages,
        };

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            provider.endpoint.trim_end_matches('/'),
            provider.model,
            api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .context("google api request failed")?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Google API {}: {}", status, text.chars().take(200).collect::<String>());
        }

        let google_resp: GoogleResponse = response.json().await.context("parse google response")?;

        google_resp
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .context("no text in google response")
    }
}

impl Default for SimpleAiService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_user_constructor() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_chat_message_user_empty() {
        let msg = ChatMessage::user("");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "");
    }

    #[test]
    fn test_chat_message_from_trait() {
        let msg = ChatMessage::user("test");
        let req: RequestMessage = msg.into();
        assert_eq!(req.role, "user");
        assert_eq!(req.content, "test");
    }

    #[test]
    fn test_default_auth_header() {
        assert_eq!(default_auth_header(), "Authorization");
    }

    #[test]
    fn test_default_auth_prefix() {
        assert_eq!(default_auth_prefix(), "Bearer ");
    }

    #[test]
    fn test_provider_status_debug() {
        let status = ProviderStatus::Healthy;
        assert_eq!(format!("{:?}", status), "Healthy");
    }

    #[test]
    fn test_provider_status_rate_limited() {
        use std::time::Instant;
        let until = Instant::now();
        let status = ProviderStatus::RateLimited { until };
        assert!(format!("{:?}", status).contains("RateLimited"));
    }

    #[test]
    fn test_provider_status_auth_failed() {
        let status = ProviderStatus::AuthFailed;
        assert_eq!(format!("{:?}", status), "AuthFailed");
    }

    #[test]
    fn test_chat_message_preserves_content() {
        let long_text = "This is a very long message content that should be preserved exactly as is";
        let msg = ChatMessage::user(long_text);
        assert_eq!(msg.content, long_text);
    }

    #[test]
    fn test_chat_message_special_characters() {
        let msg = ChatMessage::user("Hello\nWorld\t!@#$%");
        assert_eq!(msg.content, "Hello\nWorld\t!@#$%");
    }
}
