use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dracon_ai_contracts::{RoutingTask, SelectionConstraints};
use dracon_ai_runtime_contracts::traits::AiProvider;
use std::io::{BufRead, Read, Write};

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
        /// Prompt text. If omitted, enters interactive mode. Use `-` to read from stdin.
        #[arg(value_name = "PROMPT", num_args = 0.., trailing_var_arg = true)]
        prompt: Vec<String>,
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
            let mut lane = intent_to_lane(&intent);

            // Build router once per invocation; reused for interactive sessions.
            let router = build_router()?;

            // Interactive mode: `dracon-ai chat` with no prompt args.
            if prompt.is_empty() {
                return run_repl(&router, &mut lane);
            }

            let prompt = if prompt.len() == 1 && prompt[0].trim() == "-" {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            } else {
                prompt.join(" ")
            };

            let response = ask_with_router(&router, lane, &prompt).await?;
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

fn build_router() -> Result<ai_routing_runtime::SmartRouter<dyn AiProvider>> {
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

    Ok(ai_routing_runtime::SmartRouter::new(
        registry,
        resolved.dev_model_ids.clone(),
        resolved.active_model_ids.clone(),
        resolved.lane_model_policy.clone(),
    ))
}

fn run_repl(router: &ai_routing_runtime::SmartRouter<dyn AiProvider>, lane: &mut RoutingTask) -> Result<()> {
    eprintln!("dracon-ai interactive mode. Ctrl-D or /exit to quit. Use /intent <name> to change intent.");
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    loop {
        write!(&mut stdout, "> ")?;
        stdout.flush()?;

        let mut line = String::new();
        let n = stdin.lock().read_line(&mut line)?;
        if n == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "/exit" || line == "/quit" {
            break;
        }
        if let Some(rest) = line.strip_prefix("/intent ") {
            let next = normalize_intent(rest);
            *lane = intent_to_lane(&next);
            eprintln!("intent set: {}", next);
            continue;
        }

        // Execute one prompt. (Run on the current tokio runtime.)
        let response = tokio::runtime::Handle::current().block_on(ask_with_router(router, lane.clone(), line));
        match response {
            Ok(text) => println!("{}", text),
            Err(err) => eprintln!("error: {}", err),
        }
    }
    Ok(())
}

async fn ask_with_router(
    router: &ai_routing_runtime::SmartRouter<dyn AiProvider>,
    lane: RoutingTask,
    prompt: &str,
) -> Result<String> {
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
        let cli = Cli::try_parse_from(["dracon-ai", "chat", "hello", "world"]).expect("chat parses");
        match cli.cmd {
            Cmd::Chat { intent, prompt } => {
                assert_eq!(intent, "engineer");
                assert_eq!(prompt, vec!["hello", "world"]);
            }
            _ => panic!("expected chat command"),
        }
    }

    #[test]
    fn parses_chat_with_explicit_intent() {
        let cli = Cli::try_parse_from(["dracon-ai", "chat", "--intent", "commit", "ship", "it"])
            .expect("chat with explicit intent parses");
        match cli.cmd {
            Cmd::Chat { intent, prompt } => {
                assert_eq!(intent, "commit");
                assert_eq!(prompt, vec!["ship", "it"]);
            }
            _ => panic!("expected chat command"),
        }
    }

    #[test]
    fn parses_chat_without_prompt_for_interactive_mode() {
        let cli = Cli::try_parse_from(["dracon-ai", "chat"]).expect("chat parses");
        match cli.cmd {
            Cmd::Chat { intent, prompt } => {
                assert_eq!(intent, "engineer");
                assert!(prompt.is_empty());
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
