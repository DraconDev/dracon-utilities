use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use dracon_ai_contracts::{RoutingTask, SelectionConstraints};
use dracon_ai_runtime_contracts::models::{ChatMessage, ChatRequest, UsageStats};
use dracon_ai_runtime_contracts::traits::AiProvider;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
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
    /// 🛠️ Computer-context assistant (plans commands, can execute them with --apply)
    Do {
        /// Execute the planned commands (otherwise prints plan only).
        #[arg(long)]
        apply: bool,

        /// Allow potentially destructive shell commands (sudo/rm/etc).
        #[arg(long)]
        dangerous: bool,

        /// Max AI iterations (plan/execute/respond loops).
        #[arg(long, default_value_t = 5)]
        max_steps: u32,

        /// Timeout (seconds) per executed command.
        #[arg(long, default_value_t = 20)]
        timeout_secs: u64,

        /// Max bytes captured per command (stdout+stderr combined).
        #[arg(long, default_value_t = 200_000)]
        max_bytes: usize,

        /// Output final result as JSON.
        #[arg(long)]
        json: bool,

        /// Task description. If omitted, enters interactive mode.
        #[arg(value_name = "TASK", num_args = 0.., trailing_var_arg = true)]
        task: Vec<String>,
    },

    /// 💬 Send a prompt to the gateway (or start interactive chat if no prompt is provided)
    Chat {
        /// Intent/lane hint (e.g. commit, engineer, verify)
        #[arg(short, long, default_value = "engineer")]
        intent: String,

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
    let cmd = cli.cmd.unwrap_or(Cmd::Do {
        apply: std::env::var_os("DRACON_AI_APPLY").is_some(),
        dangerous: std::env::var_os("DRACON_AI_DANGEROUS").is_some(),
        max_steps: 5,
        timeout_secs: 20,
        max_bytes: 200_000,
        json: false,
        task: vec![],
    });

    match cmd {
        Cmd::Do {
            apply,
            dangerous,
            max_steps,
            timeout_secs,
            max_bytes,
            json,
            task,
        } => {
            let router = build_router()?;
            let task = if task.is_empty() {
                if json {
                    return Err(anyhow!("--json is not supported in interactive mode"));
                }
                return run_do_repl(&router, apply, dangerous, max_steps, timeout_secs, max_bytes)
                    .await;
            } else {
                task.join(" ")
            };

            let resp = run_do_task(
                &router,
                &task,
                apply,
                dangerous,
                max_steps,
                timeout_secs,
                max_bytes,
            )
            .await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&resp)?);
            } else {
                print!("{}", resp.content);
                if !resp.content.ends_with('\n') {
                    println!();
                }
            }
            Ok(())
        }
        Cmd::Chat {
            intent,
            stdin,
            file,
            json,
            no_stream,
            prompt,
        } => {
            let intent = normalize_intent(&intent);
            let mut lane = intent_to_lane(&intent);
            let pinned_model: Option<String> = None;

            let router = build_router()?;

            let is_interactive = prompt.is_empty() && !stdin && file.is_none();
            if is_interactive {
                if json {
                    return Err(anyhow!("--json is not supported in interactive mode"));
                }
                return run_chat_repl(&router, &mut lane).await;
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
            timeout_secs,
            max_bytes,
            json,
            no_stream,
            command,
        } => {
            if std::env::var_os("DRACON_AI_ALLOW_CMD").is_none() {
                return Err(anyhow!(
                    "raw cmd execution disabled. Set DRACON_AI_ALLOW_CMD=1 to enable."
                ));
            }
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
                None,
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

#[derive(Debug, Clone, Serialize)]
struct DoCliResponse {
    task: String,
    content: String,
    commands_ran: Vec<String>,
}

#[derive(Debug)]
struct CommandCapture {
    status_code: Option<i32>,
    output: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AgentResponse {
    done: bool,
    summary: String,
    #[serde(default)]
    commands: Vec<AgentCommand>,
    #[serde(default)]
    final_answer: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AgentCommand {
    cmd: String,
    #[serde(default)]
    why: String,
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

fn agent_system_prompt() -> String {
    // Hard rule: the agent must reply with pure JSON so the CLI can parse and act on it.
    // Keep this strict and repetitive; models tend to drift into Markdown otherwise.
    [
        "You are dracon-ai, a computer-context assistant.",
        "Your job is to propose shell commands to accomplish the user's task, then react to captured outputs.",
        "Reply with ONLY a single JSON object matching this schema:",
        r#"{ "done": boolean, "summary": string, "commands": [ { "cmd": string, "why": string } ], "final_answer": string|null }"#,
        "Rules:",
        "- If you need to execute commands, set done=false and provide 1-3 commands.",
        "- Commands must be safe, minimal, and non-destructive by default.",
        "- Do not include markdown fences, no prose outside JSON, no backticks.",
        "- When finished, set done=true and provide final_answer.",
    ]
    .join("\n")
}

fn extract_first_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(&s[start..=end])
}

fn is_dangerous_shell(cmd: &str) -> bool {
    let c = cmd.to_ascii_lowercase();
    let c = c.trim();
    // Very small heuristic set: enough to prevent accidental foot-guns.
    c.starts_with("sudo ")
        || c == "sudo"
        || c.starts_with("rm ")
        || c.contains(" rm ")
        || c.starts_with("mv ")
        || c.contains(" mv ")
        || c.contains(" --force")
        || c.contains(" -rf")
        || c.contains(" mkfs")
        || c.contains(" dd ")
}

async fn agent_next(
    router: &ai_routing_runtime::SmartRouter<dyn AiProvider>,
    messages: &[ChatMessage],
) -> Result<AgentResponse> {
    let req = ChatRequest {
        project_id: "default".to_string(),
        messages: messages.to_vec(),
        client_intent: Some(RoutingTask::Custom("system".to_string())),
        routing_constraints: SelectionConstraints::default(),
        resolved_service_level: None,
    };
    let (text, _usage) = {
        let (provider, _trace) = router
            .route_with_trace(
                "default",
                Some(RoutingTask::Custom("system".to_string())),
                None,
                &messages
                    .iter()
                    .map(|m| ai_routing_runtime::RoutingMessage {
                        role: m.role.clone(),
                        content: m.content.clone(),
                    })
                    .collect::<Vec<_>>(),
                SelectionConstraints::default(),
            )
            .await?;
        provider.ask_and_collect(req).await?
    };

    let json = extract_first_json_object(&text).ok_or_else(|| anyhow!("agent returned no JSON"))?;
    serde_json::from_str::<AgentResponse>(json)
        .with_context(|| format!("failed parsing agent JSON: {}", json))
}

async fn run_do_task(
    router: &ai_routing_runtime::SmartRouter<dyn AiProvider>,
    task: &str,
    apply: bool,
    dangerous: bool,
    max_steps: u32,
    timeout_secs: u64,
    max_bytes: usize,
) -> Result<DoCliResponse> {
    let mut messages: Vec<ChatMessage> = vec![
        ChatMessage {
            role: "system".to_string(),
            content: agent_system_prompt(),
        },
        ChatMessage {
            role: "system".to_string(),
            content: format!(
                "Context: os={} arch={} cwd={}",
                std::env::consts::OS,
                std::env::consts::ARCH,
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .display()
            ),
        },
        ChatMessage {
            role: "user".to_string(),
            content: task.to_string(),
        },
    ];

    let mut commands_ran: Vec<String> = Vec::new();
    let mut last_answer: Option<String> = None;

    for step_idx in 0..max_steps {
        let agent = agent_next(router, &messages).await?;
        if agent.done {
            last_answer = agent.final_answer.or(Some(agent.summary));
            break;
        }

        if agent.commands.is_empty() {
            return Err(anyhow!("agent returned done=false but no commands"));
        }

        eprintln!("{}", dim(&format!("step {}/{}: {}", step_idx + 1, max_steps, agent.summary)));
        for (i, c) in agent.commands.iter().enumerate() {
            eprintln!("{} {}", dim(&format!("  {}.", i + 1)), c.cmd);
        }

        if !apply {
            return Ok(DoCliResponse {
                task: task.to_string(),
                content: format!(
                    "Plan only (set DRACON_AI_APPLY=1 or pass --apply to execute).\n{}",
                    agent.summary
                ),
                commands_ran,
            });
        }

        for c in agent.commands {
            if !dangerous && is_dangerous_shell(&c.cmd) {
                return Err(anyhow!(
                    "refusing dangerous command without --dangerous/DRACON_AI_DANGEROUS: {}",
                    c.cmd
                ));
            }
            eprintln!("{}", dim(&format!("run: {}", c.cmd)));
            let capture =
                run_shell_capture(&c.cmd, Duration::from_secs(timeout_secs), max_bytes).await?;
            commands_ran.push(c.cmd.clone());
            let msg = format!(
                "Command executed.\ncmd={}\nexit={}\noutput:\n{}",
                c.cmd,
                capture
                    .status_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                capture.output
            );
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: msg,
            });
        }
    }

    Ok(DoCliResponse {
        task: task.to_string(),
        content: last_answer.unwrap_or_else(|| "No final answer (max steps reached).".to_string()),
        commands_ran,
    })
}

async fn run_do_repl(
    router: &ai_routing_runtime::SmartRouter<dyn AiProvider>,
    apply: bool,
    dangerous: bool,
    max_steps: u32,
    timeout_secs: u64,
    max_bytes: usize,
) -> Result<()> {
    use rustyline::error::ReadlineError;
    use rustyline::Editor;

    eprintln!(
        "{} {}",
        ansi("1;36", "dracon-ai"),
        dim("do mode. Type a task, Ctrl-D or /exit. Use /apply on|off.")
    );

    let mut rl = Editor::<(), rustyline::history::DefaultHistory>::new()?;
    if let Some(hp) = history_path() {
        if let Some(parent) = hp.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = rl.load_history(&hp);
    }

    let mut cur_apply = apply;
    let mut cur_dangerous = dangerous;

    loop {
        let line = tokio::task::block_in_place(|| rl.readline("do> "));
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
                if let Some(rest) = line.strip_prefix("/apply ") {
                    let v = rest.trim();
                    cur_apply = v == "1" || v == "on" || v == "true" || v == "yes";
                    eprintln!("{}", dim(&format!("apply={}", cur_apply)));
                    continue;
                }
                if let Some(rest) = line.strip_prefix("/dangerous ") {
                    let v = rest.trim();
                    cur_dangerous = v == "1" || v == "on" || v == "true" || v == "yes";
                    eprintln!("{}", dim(&format!("dangerous={}", cur_dangerous)));
                    continue;
                }

                let resp = run_do_task(
                    router,
                    line,
                    cur_apply,
                    cur_dangerous,
                    max_steps,
                    timeout_secs,
                    max_bytes,
                )
                .await?;
                println!("{}", resp.content);
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
    let interrupt = tokio::signal::ctrl_c();
    tokio::pin!(interrupt);

    loop {
        tokio::select! {
            _ = &mut interrupt => {
                // Treat Ctrl-C as "stop streaming" not "crash the whole CLI".
                // Return what we have so far.
                break;
            }
            next = stream.next() => {
                let Some(item) = next else { break; };
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
        .map_err(|_| anyhow!("stdout capture timeout after {:?}", timeout))?
        .map_err(|e| anyhow!("stdout capture join failed: {}", e))??;
    let err_bytes = tokio::time::timeout(timeout, err_task)
        .await
        .map_err(|_| anyhow!("stderr capture timeout after {:?}", timeout))?
        .map_err(|e| anyhow!("stderr capture join failed: {}", e))??;

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

async fn run_chat_repl(
    router: &ai_routing_runtime::SmartRouter<dyn AiProvider>,
    lane: &mut RoutingTask,
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
                    None,
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
