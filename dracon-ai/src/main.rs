use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use dracon_ai_contracts::{RoutingTask, SelectionConstraints};
use dracon_ai_runtime_contracts::models::{ChatMessage, ChatRequest, UsageStats};
use dracon_ai_runtime_contracts::traits::AiProvider;
use futures::StreamExt;
use serde::Serialize;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

#[derive(Parser)]
#[command(
    name = "dracon-ai",
    about = "AI Gateway (thin CLI over dracon-libs AI runtime)",
    version
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Cmd {
    /// 💬 Send a prompt to the gateway (or start interactive chat if no prompt is provided)
    Chat {
        /// Intent/lane hint (e.g. commit, engineer, verify)
        #[arg(short, long, default_value = "engineer")]
        intent: String,

        /// Pin a specific model id (must exist in dracon-libs runtime config).
        /// Example: `--model z-ai/glm-4.7-flash`
        #[arg(short, long)]
        model: Option<String>,

        /// Read prompt from stdin (entire stream).
        /// You can also use `-` as PROMPT.
        #[arg(long)]
        stdin: bool,

        /// Read prompt from a file.
        #[arg(long)]
        file: Option<PathBuf>,

        /// Output as JSON (non-interactive only).
        #[arg(long)]
        json: bool,

        /// Disable streaming (collect full response then print).
        #[arg(long)]
        no_stream: bool,

        /// Prompt text. If omitted, enters interactive mode. Use `-` to read from stdin.
        #[arg(value_name = "PROMPT", num_args = 0.., trailing_var_arg = true)]
        prompt: Vec<String>,
    },

    /// 🧠 Run a command locally, capture its output, and ask the AI about it.
    /// Uses `sh -lc` so pipes/redirection work.
    Cmd {
        /// Intent/lane hint (e.g. commit, engineer, verify)
        #[arg(short, long, default_value = "engineer")]
        intent: String,

        /// Pin a specific model id (must exist in dracon-libs runtime config).
        #[arg(short, long)]
        model: Option<String>,

        /// Timeout (seconds) for the command execution.
        #[arg(long, default_value_t = 10)]
        timeout_secs: u64,

        /// Max bytes captured from stdout+stderr combined.
        #[arg(long, default_value_t = 200_000)]
        max_bytes: usize,

        /// Output as JSON.
        #[arg(long)]
        json: bool,

        /// Disable streaming (collect full response then print).
        #[arg(long)]
        no_stream: bool,

        /// Shell command to run (quoted as needed).
        #[arg(value_name = "COMMAND", num_args = 1.., trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// 📜 Show AI runtime status (resolved policy + active/dev models)
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Keep logging opt-in; this tool should be quiet by default.
    let _ = env_logger::try_init();

    let cli = Cli::parse();
    let cmd = cli.cmd.unwrap_or(Cmd::Chat {
        intent: "engineer".to_string(),
        model: None,
        stdin: false,
        file: None,
        json: false,
        no_stream: false,
        prompt: vec![],
    });

    match cmd {
        Cmd::Chat {
            intent,
            model,
            stdin,
            file,
            json,
            no_stream,
            prompt,
        } => {
            let intent = normalize_intent(&intent);
            let mut lane = intent_to_lane(&intent);
            let mut pinned_model = model;

            let router = build_router()?;

            let is_interactive = prompt.is_empty() && !stdin && file.is_none();
            if is_interactive {
                if json {
                    return Err(anyhow!("--json is not supported in interactive mode"));
                }
                return run_repl(&router, &mut lane, &mut pinned_model).await;
            }

            let prompt = resolve_prompt_text(stdin, file.as_deref(), &prompt)?;
            let response = ask_one(
                &router,
                lane,
                pinned_model.as_deref(),
                &prompt,
                OutputMode {
                    stream: !no_stream && !json && std::io::stdout().is_terminal(),
                    json,
                },
            )
            .await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                print!("{}", response.content);
                if !response.content.ends_with('\n') {
                    println!();
                }
            }
            Ok(())
        }
        Cmd::Cmd {
            intent,
            model,
            timeout_secs,
            max_bytes,
            json,
            no_stream,
            command,
        } => {
            let intent = normalize_intent(&intent);
            let lane = intent_to_lane(&intent);
            let router = build_router()?;

            let cmd_s = command.join(" ");
            let capture = run_shell_capture(&cmd_s, Duration::from_secs(timeout_secs), max_bytes)
                .await
                .with_context(|| format!("command capture failed: {cmd_s}"))?;

            let prompt = format!(
                "Analyze the output of this command and propose next steps.\n\n\
Command:\n```\n{}\n```\n\n\
Exit status: {}\n\n\
Captured output:\n```\n{}\n```",
                cmd_s,
                capture.status_code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string()),
                capture.output
            );

            let response = ask_one(
                &router,
                lane,
                model.as_deref(),
                &prompt,
                OutputMode {
                    stream: !no_stream && !json && std::io::stdout().is_terminal(),
                    json,
                },
            )
            .await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                print!("{}", response.content);
                if !response.content.ends_with('\n') {
                    println!();
                }
            }
            Ok(())
        }
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
        }
    }
}

#[derive(Clone, Copy)]
struct OutputMode {
    stream: bool,
    json: bool,
}

#[derive(Debug, Clone, Serialize)]
struct AiCliResponse {
    lane: String,
    selected_model: String,
    content: String,
    usage: Option<UsageStats>,
}

#[derive(Debug)]
struct CommandCapture {
    status_code: Option<i32>,
    output: String,
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
    std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn ansi(code: &str, s: &str) -> String {
    if !color_enabled() {
        return s.to_string();
    }
    format!("\x1b[{}m{}\x1b[0m", code, s)
}

fn dim(s: &str) -> String {
    ansi("90", s)
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

fn history_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".dracon").join("ai").join("history"))
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

fn resolve_prompt_text(stdin_flag: bool, file: Option<&Path>, prompt: &[String]) -> Result<String> {
    if let Some(file) = file {
        return std::fs::read_to_string(file)
            .with_context(|| format!("failed reading file {}", file.display()));
    }

    if stdin_flag || (prompt.len() == 1 && prompt[0].trim() == "-") {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        return Ok(buf);
    }

    if prompt.is_empty() {
        return Err(anyhow!("missing prompt"));
    }
    Ok(prompt.join(" "))
}

fn trim_history(messages: &mut Vec<ChatMessage>) {
    const MAX_MESSAGES: usize = 40; // 1 system + 19 turns
    if messages.len() <= MAX_MESSAGES {
        return;
    }
    let mut keep = Vec::with_capacity(MAX_MESSAGES);
    // Keep the first system message, then the last N-1.
    if let Some(first) = messages.first().cloned() {
        keep.push(first);
    }
    keep.extend(messages.iter().rev().take(MAX_MESSAGES.saturating_sub(1)).cloned().rev());
    *messages = keep;
}

async fn ask_one(
    router: &ai_routing_runtime::SmartRouter<dyn AiProvider>,
    lane: RoutingTask,
    pinned_model: Option<&str>,
    prompt: &str,
    out: OutputMode,
) -> Result<AiCliResponse> {
    let mut messages = vec![ChatMessage {
        role: "user".to_string(),
        content: prompt.to_string(),
    }];
    ask_with_messages(router, lane, pinned_model, &mut messages, out).await
}

async fn ask_with_messages(
    router: &ai_routing_runtime::SmartRouter<dyn AiProvider>,
    lane: RoutingTask,
    pinned_model: Option<&str>,
    messages: &mut Vec<ChatMessage>,
    out: OutputMode,
) -> Result<AiCliResponse> {
    trim_history(messages);
    let routing_msgs = messages
        .iter()
        .map(|m| ai_routing_runtime::RoutingMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect::<Vec<_>>();

    let mut constraints = SelectionConstraints::default();
    if let Some(model) = pinned_model {
        constraints.allowed_model_ids = vec![model.to_string()];
    }

    let (provider, trace) = router
        .route_with_trace(
            "default",
            Some(lane.clone()),
            None,
            &routing_msgs,
            constraints.clone(),
        )
        .await?;

    let req = ChatRequest {
        project_id: "default".to_string(),
        messages: messages.clone(),
        client_intent: Some(lane.clone()),
        routing_constraints: constraints,
        resolved_service_level: None,
    };

    if out.json {
        // JSON output must be deterministic and self-contained; collect fully.
        let (content, usage) = provider.ask_and_collect(req).await?;
        return Ok(AiCliResponse {
            lane: lane.as_task_key().to_string(),
            selected_model: trace.selected_model,
            content,
            usage,
        });
    }

    if !out.stream {
        let (content, usage) = provider.ask_and_collect(req).await?;
        return Ok(AiCliResponse {
            lane: lane.as_task_key().to_string(),
            selected_model: trace.selected_model,
            content,
            usage,
        });
    }

    // Streaming: stdout only, no extra decoration.
    let mut stream = provider.ask(req).await?;
    let mut buf = String::new();
    let mut usage: Option<UsageStats> = None;
    let mut stdout = tokio::io::stdout();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        if !chunk.token.is_empty() {
            stdout.write_all(chunk.token.as_bytes()).await?;
            stdout.flush().await?;
            buf.push_str(&chunk.token);
        }
        if chunk.usage.is_some() {
            usage = chunk.usage;
        }
    }

    Ok(AiCliResponse {
        lane: lane.as_task_key().to_string(),
        selected_model: trace.selected_model,
        content: buf,
        usage,
    })
}

async fn run_shell_capture(cmd: &str, timeout: Duration, max_bytes: usize) -> Result<CommandCapture> {
    async fn read_limited(mut r: impl AsyncRead + Unpin, limit: usize) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = r.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            let remaining = limit.saturating_sub(out.len());
            if remaining == 0 {
                break;
            }
            let take = n.min(remaining);
            out.extend_from_slice(&buf[..take]);
            if out.len() >= limit {
                break;
            }
        }
        Ok(out)
    }

    let mut child = Command::new("sh")
        .arg("-lc")
        .arg(cmd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("failed to spawn shell command: {cmd}"))?;

    let mut stdout = child.stdout.take().ok_or_else(|| anyhow!("missing stdout"))?;
    let mut stderr = child.stderr.take().ok_or_else(|| anyhow!("missing stderr"))?;

    let per_stream_limit = max_bytes.saturating_div(2).max(4096);
    let out_task = tokio::spawn(async move { read_limited(&mut stdout, per_stream_limit).await });
    let err_task = tokio::spawn(async move { read_limited(&mut stderr, per_stream_limit).await });

    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(s)) => Some(s),
        Ok(Err(_)) => None,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            None
        }
    };

    let out_bytes = tokio::time::timeout(timeout, out_task)
        .await
        .map_err(|_| anyhow!("stdout capture timeout after {:?}", timeout))???
        ;
    let err_bytes = tokio::time::timeout(timeout, err_task)
        .await
        .map_err(|_| anyhow!("stderr capture timeout after {:?}", timeout))???
        ;

    let mut combined = Vec::new();
    combined.extend_from_slice(&out_bytes);
    if !out_bytes.is_empty() && !err_bytes.is_empty() {
        combined.extend_from_slice(b"\n");
    }
    combined.extend_from_slice(&err_bytes);

    let mut s = String::from_utf8_lossy(&combined).to_string();
    if s.len() > max_bytes {
        s.truncate(max_bytes);
    }
    if s.is_empty() {
        s = "<no output>".to_string();
    }

    Ok(CommandCapture {
        status_code: status.and_then(|s| s.code()),
        output: s,
    })
}

async fn run_repl(
    router: &ai_routing_runtime::SmartRouter<dyn AiProvider>,
    lane: &mut RoutingTask,
    pinned_model: &mut Option<String>,
) -> Result<()> {
    use rustyline::error::ReadlineError;
    use rustyline::Editor;

    let title = ansi("1;36", "dracon-ai");
    eprintln!(
        "{} {}",
        title,
        dim("interactive mode. Ctrl-D or /exit to quit. /help for commands.")
    );
    eprintln!("{}", dim("Tip: /paste then paste multi-line text, end with /end."));
    eprintln!("{}", dim("Tip: /cmd <shell> captures local output into context (logs, status, etc)."));

    let mut rl = Editor::<(), rustyline::history::DefaultHistory>::new()?;
    if let Some(hp) = history_path() {
        if let Some(parent) = hp.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = rl.load_history(&hp);
    }

    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "system".to_string(),
        content: "You are dracon-ai (CLI). Be concise, practical, and command-oriented. If you need repo context, ask for it or request /cmd output.".to_string(),
    }];

    loop {
        let p = format!("{}> ", prompt_label(lane));
        let line = tokio::task::block_in_place(|| rl.readline(&p));

        match line {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                rl.add_history_entry(line)?;
                if let Some(hp) = history_path() {
                    let _ = rl.append_history(&hp);
                }

                if line == "/exit" || line == "/quit" {
                    break;
                }
                if line == "/help" || line == "/?" {
                    eprintln!("{}", dim("Commands:"));
                    eprintln!("{}", dim("  /intent <name>   set intent/lane hint"));
                    eprintln!("{}", dim("  /lane <name>     alias for /intent"));
                    eprintln!("{}", dim("  /model <id>      pin model id (or: /model off)"));
                    eprintln!("{}", dim("  /clear           clear conversation context"));
                    eprintln!("{}", dim("  /paste           begin multi-line paste (end with /end)"));
                    eprintln!("{}", dim("  /cmd <shell>     run local command, add output to context"));
                    eprintln!("{}", dim("  /exit            quit"));
                    continue;
                }
                if let Some(rest) = line.strip_prefix("/intent ") {
                    let next = normalize_intent(rest);
                    *lane = intent_to_lane(&next);
                    eprintln!("{} {}", ansi("1;36", "intent"), ansi("33", &next));
                    continue;
                }
                if let Some(rest) = line.strip_prefix("/lane ") {
                    let next = normalize_intent(rest);
                    *lane = intent_to_lane(&next);
                    eprintln!("{} {}", ansi("1;36", "lane"), ansi("33", &next));
                    continue;
                }
                if line == "/clear" {
                    messages.truncate(1); // keep system
                    eprintln!("{}", dim("context cleared"));
                    continue;
                }
                if let Some(rest) = line.strip_prefix("/model") {
                    let rest = rest.trim();
                    if rest.is_empty() {
                        let cur = pinned_model
                            .as_deref()
                            .unwrap_or("<auto>");
                        eprintln!("{} {}", ansi("1;36", "model"), ansi("33", cur));
                        continue;
                    }
                    let rest = rest.strip_prefix(' ').unwrap_or(rest);
                    if rest == "off" || rest == "auto" || rest == "clear" {
                        *pinned_model = None;
                        eprintln!("{} {}", ansi("1;36", "model"), dim("set to auto"));
                        continue;
                    }
                    *pinned_model = Some(rest.to_string());
                    eprintln!("{} {}", ansi("1;36", "model"), ansi("33", rest));
                    continue;
                }
                if line == "/paste" {
                    eprintln!("{}", dim("paste mode: type /end on its own line to send.")); // meta only
                    let mut buf = String::new();
                    loop {
                        let pl = tokio::task::block_in_place(|| rl.readline("paste> "));
                        match pl {
                            Ok(pl) => {
                                let t = pl.trim_end();
                                if t == "/end" || t == "/done" {
                                    break;
                                }
                                buf.push_str(&pl);
                                buf.push('\n');
                            }
                            Err(ReadlineError::Eof) => break,
                            Err(ReadlineError::Interrupted) => {
                                buf.clear();
                                break;
                            }
                            Err(e) => return Err(anyhow!(e)),
                        }
                    }
                    let prompt = buf.trim();
                    if prompt.is_empty() {
                        eprintln!("{}", dim("paste: empty input, cancelled"));
                        continue;
                    }
                    messages.push(ChatMessage {
                        role: "user".to_string(),
                        content: prompt.to_string(),
                    });
                } else if let Some(rest) = line.strip_prefix("/cmd ") {
                    let cmd = rest.trim();
                    if cmd.is_empty() {
                        eprintln!("{}", dim("cmd: missing command"));
                        continue;
                    }
                    eprintln!("{}", dim("cmd: capturing..."));
                    match run_shell_capture(cmd, Duration::from_secs(10), 200_000).await {
                        Ok(capture) => {
                            messages.push(ChatMessage {
                                role: "system".to_string(),
                                content: format!(
                                    "Command output captured.\nCommand:\n{}\nExit: {}\nOutput:\n{}",
                                    cmd,
                                    capture
                                        .status_code
                                        .map(|c| c.to_string())
                                        .unwrap_or_else(|| "unknown".to_string()),
                                    capture.output
                                ),
                            });
                            eprintln!("{}", dim("cmd: added output to context"));
                            continue;
                        }
                        Err(e) => {
                            eprintln!("{}: {}", ansi("1;31", "error"), e);
                            continue;
                        }
                    }
                } else {
                    messages.push(ChatMessage {
                        role: "user".to_string(),
                        content: line.to_string(),
                    });
                }

                // Ask AI with accumulated messages; stream by default in TTY.
                let out = OutputMode {
                    stream: std::io::stdout().is_terminal(),
                    json: false,
                };

                let mut stdout = tokio::io::stdout();
                match ask_with_messages(
                    router,
                    lane.clone(),
                    pinned_model.as_deref(),
                    &mut messages,
                    out,
                )
                .await
                {
                    Ok(resp) => {
                        // When streaming, resp.content already hit stdout. Ensure newline.
                        if std::io::stdout().is_terminal() {
                            if !resp.content.ends_with('\n') {
                                stdout.write_all(b"\n").await?;
                                stdout.flush().await?;
                            }
                        } else {
                            stdout.write_all(resp.content.as_bytes()).await?;
                            if !resp.content.ends_with('\n') {
                                stdout.write_all(b"\n").await?;
                            }
                            stdout.flush().await?;
                        }

                        messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: resp.content,
                        });
                    }
                    Err(err) => {
                        eprintln!("{}: {}", ansi("1;31", "error"), err);
                        // If the last user message was just added, drop it to avoid poisoning history.
                        if messages.last().map(|m| m.role.as_str()) == Some("user") {
                            messages.pop();
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                eprintln!("{}", dim("^C"));
                continue;
            }
            Err(ReadlineError::Eof) => break,
            Err(e) => return Err(anyhow!(e)),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Cli, Cmd};
    use clap::Parser;

    #[test]
    fn parses_chat_with_default_intent() {
        let cli = Cli::try_parse_from(["dracon-ai", "chat", "hello", "world"]).expect("chat parses");
        match cli.cmd.expect("cmd") {
            Cmd::Chat { intent, prompt, .. } => {
                assert_eq!(intent, "engineer");
                assert_eq!(prompt, vec!["hello", "world"]);
            }
            _ => panic!("expected chat command"),
        }
    }

    #[test]
    fn parses_chat_without_prompt_for_interactive_mode() {
        let cli = Cli::try_parse_from(["dracon-ai", "chat"]).expect("chat parses");
        match cli.cmd.expect("cmd") {
            Cmd::Chat { intent, prompt, .. } => {
                assert_eq!(intent, "engineer");
                assert!(prompt.is_empty());
            }
            _ => panic!("expected chat command"),
        }
    }

    #[test]
    fn parses_cmd_subcommand() {
        let cli = Cli::try_parse_from(["dracon-ai", "cmd", "echo", "hi"]).expect("cmd parses");
        match cli.cmd.expect("cmd") {
            Cmd::Cmd { command, .. } => assert_eq!(command, vec!["echo", "hi"]),
            _ => panic!("expected cmd"),
        }
    }

    #[test]
    fn parses_status_subcommand() {
        let cli = Cli::try_parse_from(["dracon-ai", "status"]).expect("status parses");
        assert!(matches!(cli.cmd, Some(Cmd::Status)));
    }
}
