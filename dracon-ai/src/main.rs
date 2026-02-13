use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dracon_ai_contracts::{RoutingTask, SelectionConstraints};
use dracon_ai_runtime_contracts::traits::AiProvider;

#[derive(Parser)]
#[command(
    name = "dracon-ai",
    about = "AI Gateway (thin CLI over dracon-libs AI runtime)",
    version
)]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Cmd {
    /// 💬 Send a prompt to the gateway with specific intent
    Chat { 
        /// Intent/lane hint (e.g. commit, engineer, verify)
        #[arg(short, long, default_value = "engineer")]
        intent: String,
        /// The prompt message
        prompt: String 
    },
    /// 📜 Show AI runtime status (resolved policy + active/dev models)
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Chat { intent, prompt } => {
            let intent = normalize_intent(&intent);
            let lane = intent_to_lane(&intent);
            let response = ask_once(lane, &prompt).await?;
            println!("{}", response);
            Ok(())
        },
        Cmd::Status => {
            let resolved = ai_runtime_config::resolve_ai_runtime_config();
            println!("📜 AI_RUNTIME: dracon-libs policy + secrets (ai-runtime-config)");
            println!("📦 PROVIDERS: {}", resolved.provider_specs.len());
            println!("✅ ACTIVE_MODELS: {}", resolved.active_model_ids.len());
            for id in &resolved.active_model_ids {
                println!("  - {}", id);
            }
            println!("🧪 DEV_MODELS: {}", resolved.dev_model_ids.len());
            for id in &resolved.dev_model_ids {
                println!("  - {}", id);
            }
            Ok(())
        },
    }
}

fn normalize_intent(intent: &str) -> String {
    intent.trim().to_ascii_lowercase()
}

fn intent_to_lane(intent: &str) -> RoutingTask {
    match intent {
        "commit" | "engineer" | "coding" => RoutingTask::Coding,
        "verify" | "fast" | "summary" => RoutingTask::Fast,
        "general" => RoutingTask::General,
        other => RoutingTask::Custom(other.to_string()),
    }
}

async fn ask_once(lane: RoutingTask, prompt: &str) -> Result<String> {
    let resolved = ai_runtime_config::resolve_ai_runtime_config();

    let mut registry: ai_routing_runtime::ProviderRegistry<dyn AiProvider> =
        ai_routing_runtime::ProviderRegistry::new();
    for spec in &resolved.provider_specs {
        let provider: std::sync::Arc<dyn AiProvider> =
            std::sync::Arc::new(ai_runtime_adapters::GenericOpenAIAdapter::new_with_auth(
                spec.api_key.clone(),
                spec.endpoint.clone(),
                spec.payload_model.clone(),
                spec.auth_header_name.clone(),
                spec.auth_header_prefix.clone(),
            ));
        registry.register(&spec.model_id, provider);
    }

    if resolved.active_model_ids.is_empty() && resolved.dev_model_ids.is_empty() {
        return Err(anyhow!(
            "No active/dev models configured. Check dracon-libs platform policy + secrets."
        ));
    }

    let router = ai_routing_runtime::SmartRouter::new(
        registry,
        resolved.dev_model_ids.clone(),
        resolved.active_model_ids.clone(),
        resolved.lane_model_policy.clone(),
    );

    let messages = vec![ai_routing_runtime::RoutingMessage {
        role: "user".to_string(),
        content: prompt.to_string(),
    }];

    let (provider, _trace) = router
        .route_with_trace(
            "default",
            Some(lane),
            None,
            &messages,
            SelectionConstraints::default(),
        )
        .await?;

    provider.generate_response(prompt).await
}

#[cfg(test)]
mod tests {
    use super::{Cli, Cmd};
    use clap::Parser;

    #[test]
    fn parses_chat_with_default_intent() {
        let cli = Cli::try_parse_from(["dracon-ai", "chat", "hello world"]).expect("chat parses");
        match cli.cmd {
            Cmd::Chat { intent, prompt } => {
                assert_eq!(intent, "engineer");
                assert_eq!(prompt, "hello world");
            }
            _ => panic!("expected chat command"),
        }
    }

    #[test]
    fn parses_chat_with_explicit_intent() {
        let cli = Cli::try_parse_from(["dracon-ai", "chat", "--intent", "commit", "ship it"])
            .expect("chat with explicit intent parses");
        match cli.cmd {
            Cmd::Chat { intent, prompt } => {
                assert_eq!(intent, "commit");
                assert_eq!(prompt, "ship it");
            }
            _ => panic!("expected chat command"),
        }
    }

    #[test]
    fn parses_status_subcommand() {
        let cli = Cli::try_parse_from(["dracon-ai", "status"]).expect("status parses");
        assert!(matches!(cli.cmd, Cmd::Status));
    }
}
