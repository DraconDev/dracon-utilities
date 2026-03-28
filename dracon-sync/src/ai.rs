use ai_adapters::HttpProviderAdapter;
use ai_lanes::ChatMessage;
use ai_router::AiProvider;
use std::sync::Arc;

pub struct SimpleAiService {
    providers: Vec<(String, Arc<dyn AiProvider>)>,
}

impl SimpleAiService {
    pub fn new() -> Self {
        let mut providers = Vec::new();

        if let Some(key) = std::env::var("OPENROUTER_API_KEY").ok().filter(|k| !k.is_empty()) {
            let adapter: Arc<dyn AiProvider> = Arc::new(HttpProviderAdapter::new_with_auth(
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
            let adapter: Arc<dyn AiProvider> = Arc::new(HttpProviderAdapter::new_with_auth(
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
            let adapter: Arc<dyn AiProvider> = Arc::new(HttpProviderAdapter::new_with_auth(
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

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> anyhow::Result<String> {
        use anyhow::Context;

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
