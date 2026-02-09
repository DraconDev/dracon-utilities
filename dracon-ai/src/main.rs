use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf, process::Command, io::{self, Write}};
use reqwest::Client;
use serde_json::json;

mod config;
use config::{AiConfig, ModelRoute};

#[derive(Parser)]
#[command(name = "dracon-ai", about = "Intelligence Manager - Autonomous AI Gateway", version)]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Cmd {
    /// 🛠️  Perform guided intelligence setup
    Install,
    /// 💬 Send a prompt to the gateway with specific intent
    Chat { 
        /// Intent (e.g. commit, engineer, verify)
        #[arg(short, long, default_value = "engineer")]
        intent: String,
        /// The prompt message
        prompt: String 
    },
    /// 📜 Show intelligence status and active model routes
    Status,
    /// ⚙️ Open the intelligence policy in your system editor
    Edit,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Install => {
            println!("🛠️  DRACON INTELLIGENCE GUIDED SETUP\n");
            let mut cfg = AiConfig::load()?;
            
            print!("Enter primary AI provider endpoint (default: {:?}): ", cfg.providers.get("openrouter"));
            io::stdout().flush()?;
            let mut input = String::new(); io::stdin().read_line(&mut input)?;
            let input = input.trim();
            if !input.is_empty() { cfg.providers.insert("primary".to_string(), input.to_string()); }

            cfg.save()?;
            println!("\n✅ Intelligence setup complete. Policy: {:?}", AiConfig::path()?); Ok(())
        },
        Cmd::Chat { intent, prompt } => {
            let cfg = AiConfig::load()?;
            let route = cfg.intents.get(&intent).ok_or_else(|| anyhow!("Unknown intent: {}", intent))?;
            let response = call_provider(&cfg, route, &prompt).await?;
            println!("{}", response); Ok(())
        },
        Cmd::Status => {
            let cfg = AiConfig::load()?;
            println!("📜 POLICY: {:?}", AiConfig::path()?);
            println!("📡 INTENTS: {}", cfg.intents.len());
            for (intent, route) in &cfg.intents { println!("  - {}: {} via {}", intent, route.model, route.provider); }
            Ok(())
        },
        Cmd::Edit => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
            Command::new(editor).arg(AiConfig::path()?).status()?; Ok(())
        }
    }
}

async fn call_provider(cfg: &AiConfig, route: &ModelRoute, prompt: &str) -> Result<String> {
    let client = Client::new();
    let url = cfg.providers.get(&route.provider).ok_or_else(|| anyhow!("Unknown provider: {}", route.provider))?;
    
    // SAFETY: Key shielding via Security Manager path convention
    let key_path = dirs::home_dir().unwrap().join("dracon/security/keys").join(format!("{}.key", route.provider));
    let key = if key_path.exists() { fs::read_to_string(key_path)?.trim().to_string() } 
              else { std::env::var(format!("{}_KEY", route.provider.to_uppercase())).unwrap_or_default() };

    if key.is_empty() { return Err(anyhow!("No API key found for {}. Place it in: dracon/security/keys/{}.key", route.provider, route.provider)); }

    let res = client.post(format!("{}/chat/completions", url))
        .header("Authorization", format!("Bearer {}", key))
        .json(&json!({
            "model": route.model,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send().await?;

    let json: serde_json::Value = res.json().await?;
    let content = json["choices"][0]["message"]["content"].as_str()
        .ok_or_else(|| anyhow!("Malformed AI response: {:?}", json))?;
    
    Ok(content.to_string())
}
