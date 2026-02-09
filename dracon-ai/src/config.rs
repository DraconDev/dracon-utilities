use anyhow::{Context, Result};
use std::{fs, path::PathBuf, collections::HashMap};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelRoute {
    pub provider: String,
    pub model: String,
    pub fallback_model: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AiConfig {
    pub intents: HashMap<String, ModelRoute>,
    pub providers: HashMap<String, String>,
    pub local_endpoint: Option<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        let mut intents = HashMap::new();
        intents.insert("commit".to_string(), ModelRoute { 
            provider: "openrouter".to_string(), 
            model: "google/gemini-2.0-flash-exp:free".to_string(),
            fallback_model: None 
        });
        intents.insert("engineer".to_string(), ModelRoute { 
            provider: "openrouter".to_string(), 
            model: "anthropic/claude-3.5-sonnet".to_string(),
            fallback_model: Some("google/gemini-2.0-pro-exp-02-05:free".to_string())
        });

        let mut providers = HashMap::new();
        providers.insert("openrouter".to_string(), "https://openrouter.ai/api/v1".to_string());
        providers.insert("anthropic".to_string(), "https://api.anthropic.com/v1".to_string());

        Self {
            intents,
            providers,
            local_endpoint: Some("http://localhost:8080/v1".to_string()),
        }
    }
}

impl AiConfig {
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() { return Ok(Self::default()); }
        let s = fs::read_to_string(path)?;
        toml::from_str(&s).context("Failed to parse technical intelligence policy")
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::dir()?;
        fs::create_dir_all(&dir)?;
        
        let mut s = String::new();
        s.push_str("# =============================================================================\n");
        s.push_str("# 📡  DRACON INTELLIGENCE POLICY\n");
        s.push_str("# =============================================================================\n\n");
        s.push_str("### 📜 INTENT ROUTING\n# Maps 'Intent' to specific models.\n\n");
        s.push_str(&toml::to_string_pretty(self)?);
        fs::write(Self::path()?, s)?;
        Ok(())
    }

    pub fn dir() -> Result<PathBuf> { let home = dirs::home_dir().unwrap(); Ok(home.join("dracon/ai")) }
    pub fn path() -> Result<PathBuf> { Ok(Self::dir()?.join("dracon-ai.toml")) }
}
