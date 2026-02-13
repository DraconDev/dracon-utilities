use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dracon_ai_contracts::{RoutingTask, SelectionConstraints};
use dracon_ai_runtime_contracts::traits::AiProvider;
use std::io::Read;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use std::io::IsTerminal;

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
                return run_repl(&router, &mut lane).await;
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

fn color_enabled() -> bool {
    // Enable colors only when stdout is a terminal and NO_COLOR is not set.
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn ansi(code: &str, s: &str) -> String {
    if !color_enabled() {
        return s.to_string();
    }
    format!("\x1b[{}m{}\x1b[0m", code, s)
}

fn prompt_label(lane: &RoutingTask) -> String {
    let tool = ansi("1;36", "dracon-ai"); // bold cyan
    let lane_txt = match lane {
        RoutingTask::General => "general",
        RoutingTask::Coding => "coding",
        RoutingTask::Fast => "fast",
        RoutingTask::Custom(v) => v.as_str(),
    };
    let lane = ansi("33", lane_txt); // yellow
    format!("{}[{}]", tool, lane)
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

async fn run_repl(
    router: &ai_routing_runtime::SmartRouter<dyn AiProvider>,
    lane: &mut RoutingTask,
) -> Result<()> {
    let title = ansi("1;36", "dracon-ai");
    let dim = |s: &str| ansi("90", s); // bright black / dim
    eprintln!(
        "{} {}",
        title,
        dim("interactive mode. Ctrl-D or /exit to quit. Use /intent <name> to change intent.")
    );
    eprintln!("{}", dim("Tip: use /paste then paste multi-line text, end with /end."));
    let stdin = tokio::io::stdin();
    let mut reader = tokio::io::BufReader::new(stdin);
    let mut stdout = tokio::io::stdout(); // model output
    let mut stderr = tokio::io::stderr(); // prompt + meta

    loop {
        let p = format!("{}> ", prompt_label(lane));
        stderr.write_all(p.as_bytes()).await?;
        stderr.flush().await?;

        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
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
        if line == "/help" || line == "/?" {
            eprintln!("{}", dim("Commands:"));
            eprintln!("{}", dim("  /intent <name>   set intent/lane hint"));
            eprintln!("{}", dim("  /paste           begin multi-line paste (end with /end)"));
            eprintln!("{}", dim("  /exit            quit"));
            continue;
        }
        if let Some(rest) = line.strip_prefix("/intent ") {
            let next = normalize_intent(rest);
            *lane = intent_to_lane(&next);
            eprintln!(
                "{} {} {}",
                ansi("1;36", "intent"),
                dim("set to"),
                ansi("33", &next)
            );
            continue;
        }
        if line == "/paste" {
            eprintln!(
                "{} {}",
                ansi("1;36", "paste"),
                dim("mode: paste your text, then type /end on its own line.")
            );
            let mut buf = String::new();
            loop {
                let mut pline = String::new();
                let n = reader.read_line(&mut pline).await?;
                if n == 0 {
                    break;
                }
                let trimmed = pline.trim_end_matches(&['\r', '\n'][..]).trim();
                if trimmed == "/end" || trimmed == "/done" {
                    break;
                }
                buf.push_str(&pline);
            }
            let prompt = buf.trim();
            if prompt.is_empty() {
                eprintln!("{}", dim("paste: empty input, cancelled"));
                continue;
            }
            match ask_with_router(router, lane.clone(), prompt).await {
                Ok(text) => {
                    stdout.write_all(text.as_bytes()).await?;
                    if !text.ends_with('\n') {
                        stdout.write_all(b"\n").await?;
                    }
                    stdout.flush().await?;
                }
                Err(err) => eprintln!("{}: {}", ansi("1;31", "error"), err),
            }
            continue;
        }

        match ask_with_router(router, lane.clone(), line).await {
            Ok(text) => {
                stdout.write_all(text.as_bytes()).await?;
                if !text.ends_with('\n') {
                    stdout.write_all(b"\n").await?;
                }
                stdout.flush().await?;
            }
            Err(err) => eprintln!("{}: {}", ansi("1;31", "error"), err),
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
