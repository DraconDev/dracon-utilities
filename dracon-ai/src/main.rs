use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use dracon_ai_runtime_contracts::routing::{RoutingTask, SelectionConstraints};
use dracon_ai_runtime_contracts::models::{ChatMessage, ChatRequest, UsageStats};
use dracon_ai_runtime_contracts::traits::AiProvider;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::{IsTerminal, Read};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Default, Deserialize)]
struct DraconAiConfig {
    /// System repo root (usually ~/dracon)
    system_root: Option<PathBuf>,
    /// NixOS config root (usually ~/dracon/nixos)
    nixos_root: Option<PathBuf>,
    /// Whether do-mode should auto-run a Nix context probe when task mentions Nix/NixOS.
    #[serde(default)]
    do_auto_probe_nix: bool,
}

fn env_bool(name: &str) -> Option<bool> {
    let v = std::env::var(name).ok()?;
    let v = v.trim().to_ascii_lowercase();
    match v.as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn resolve_config_path() -> Option<PathBuf> {
    // Keep consistent with other utilities: system repo is the canonical policy owner.
    // Allow override for experimentation.
    if let Ok(p) = std::env::var("DRACON_AI_CONFIG") {
        return Some(PathBuf::from(p));
    }
    let home = dirs::home_dir()?;
    Some(
        home.join(".dracon")
            .join("utilities")
            .join("ai")
            .join("dracon-ai.toml"),
    )
}

fn load_config() -> DraconAiConfig {
    let Some(path) = resolve_config_path() else {
        return DraconAiConfig::default();
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return DraconAiConfig::default();
    };
    toml::from_str(&raw).unwrap_or_else(|e| {
        eprintln!("⚠️ failed to parse {}: {}", path.display(), e);
        DraconAiConfig::default()
    })
}

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
    /// 🛠️ Computer-context assistant (plans commands and executes by default)
    Do {
        /// Plan only (do not execute). Default is execute.
        #[arg(long, alias = "no-apply")]
        plan: bool,

        /// Allow potentially destructive shell commands (sudo/rm/etc).
        #[arg(long)]
        dangerous: bool,

        /// Keep interactive do-mode in the current terminal (do not spawn a new tab).
        #[arg(long)]
        same_terminal: bool,

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

        /// Keep interactive chat in the current terminal (do not spawn a new tab/window).
        #[arg(long)]
        same_terminal: bool,

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

    /// 📝 Observe a repo and update its project-state.md (scribe)
    Scribe {
        /// Path to the git repository to observe.
        #[arg(value_name = "REPO")]
        repo: PathBuf,
    },

    /// 🔧 Initialize ~/.dracon/ai/ config directory with templates
    Setup {
        /// Re-discover models from all existing keys (skip prompt)
        #[arg(long)]
        refresh: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Keep logging opt-in; this tool should be quiet by default.
    let _ = env_logger::try_init();

    let cli = Cli::parse();
    let cmd = cli.cmd.unwrap_or(Cmd::Do {
        // Default is APPLY ON. This tool is meant to change the computer state.
        // Use `--plan` (or DRACON_AI_APPLY=0) for plan-only.
        plan: env_bool("DRACON_AI_APPLY").map(|v| !v).unwrap_or(false),
        dangerous: env_bool("DRACON_AI_DANGEROUS").unwrap_or(false),
        same_terminal: false,
        max_steps: 5,
        timeout_secs: 20,
        max_bytes: 200_000,
        json: false,
        task: vec![],
    });

    match cmd {
        Cmd::Do {
            plan,
            dangerous,
            same_terminal,
            max_steps,
            timeout_secs,
            max_bytes,
            json,
            task,
        } => {
            let router = build_router()?;
            let apply = !plan;
            let task = if task.is_empty() {
                if json {
                    return Err(anyhow!("--json is not supported in interactive mode"));
                }
                if !same_terminal {
                    match spawn_new_terminal_tab(&["do".to_string(), "--same-terminal".to_string()])
                    {
                        Ok(true) => return Ok(()),
                        Ok(false) => {
                            eprintln!(
                                "{}",
                                dim("do: could not spawn a new tab; continuing in current terminal (pass --same-terminal to silence).")
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "{}",
                                dim(&format!(
                                    "do: spawn failed: {e}. continuing in current terminal (pass --same-terminal to silence)."
                                ))
                            );
                        }
                    }
                }
                return run_do_repl(
                    &router,
                    apply,
                    dangerous,
                    max_steps,
                    timeout_secs,
                    max_bytes,
                )
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
            same_terminal,
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
                if !same_terminal {
                    match spawn_new_terminal_tab(&[
                        "chat".to_string(),
                        "--intent".to_string(),
                        intent.clone(),
                        "--same-terminal".to_string(),
                    ]) {
                        Ok(true) => return Ok(()),
                        Ok(false) => {
                            eprintln!(
                                "{}",
                                dim("chat: could not spawn a new tab; continuing in current terminal (pass --same-terminal to silence).")
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "{}",
                                dim(&format!(
                                    "chat: spawn failed: {e}. continuing in current terminal (pass --same-terminal to silence)."
                                ))
                            );
                        }
                    }
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
                capture
                    .status_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                redact_output(&capture.output)
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
            let resolved = ai_runtime_adapters::resolve_ai_runtime_config();
            println!("📜 AI_RUNTIME: dracon-libs policy + secrets (ai-runtime-config)");
            println!("📦 PROVIDERS: {} OpenAI + {} Bedrock", resolved.openai_providers.len(), resolved.bedrock_providers.len());
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
        Cmd::Scribe { repo } => {
            run_scribe(&repo).await
        }
        Cmd::Setup { refresh } => {
            run_setup(refresh)
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

#[derive(Debug, Clone)]
struct AgentStep {
    selected_model: String,
    resp: AgentResponse,
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
        "writing" | "write" | "docs" | "documentation" => RoutingTask::Writing,
        "verify" | "fast" | "summary" => RoutingTask::Fast,
        "general" => RoutingTask::General,
        other => RoutingTask::Custom(other.to_string()),
    }
}

fn ansi(color: &str, text: &str) -> String {
    let codes = match color {
        "31" => "31", "32" => "32", "33" => "33", "34" => "34",
        "35" => "35", "36" => "36", "37" => "37", "1" => "1",
        _ => "0",
    };
    format!("\x1b[{}m{}\x1b[0m", codes, text)
}

fn dim(s: &str) -> String { ansi("2", s) }
fn cyan(s: &str) -> String { ansi("36", s) }
fn magenta_bold(s: &str) -> String { ansi("35;1", s) }

fn stderr_is_tty() -> bool {
    std::io::stderr().is_terminal()
}

fn set_title(title: &str) {
    if !stderr_is_tty() {
        return;
    }
    eprint!("\x1b]0;{}\x07", title);
}

fn prompt_label(lane: &RoutingTask) -> String {
    let tool = ansi("1;36", "dracon-ai"); // bold cyan
    let lane_txt = match lane {
        RoutingTask::General => "general",
        RoutingTask::Coding => "coding",
        RoutingTask::Writing => "writing",
        RoutingTask::Fast => "fast",
        RoutingTask::Custom(v) => v.as_str(),
        RoutingTask::Dev => "dev",
        RoutingTask::Free => "free",
    };
    let lane = ansi("33", lane_txt); // yellow
    format!("{}[{}]", tool, lane)
}

struct Spinner {
    stop_tx: Option<oneshot::Sender<()>>,
    handle: tokio::task::JoinHandle<()>,
}

impl Spinner {
    fn start(label: String) -> Self {
        let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            if !stderr_is_tty() {
                return;
            }
            let frames = ["|", "/", "-", "\\"];
            let mut idx = 0usize;
            let mut stderr = tokio::io::stderr();
            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        // Clear line.
                        let _ = stderr.write_all(b"\r\x1b[2K").await;
                        let _ = stderr.flush().await;
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_millis(120)) => {
                        let frame = frames[idx % frames.len()];
                        idx += 1;
                        let line = format!("\r{} {}", frame, label);
                        let _ = stderr.write_all(line.as_bytes()).await;
                        let _ = stderr.flush().await;
                    }
                }
            }
        });
        Self {
            stop_tx: Some(stop_tx),
            handle,
        }
    }

    async fn stop(mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.handle.await;
    }
}

fn history_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".dracon").join("ai").join("history"))
}

fn build_router() -> Result<ai_routing_runtime::SmartRouter<dyn AiProvider>> {
    let resolved = ai_runtime_adapters::resolve_ai_runtime_config();

    let mut registry: ai_routing_runtime::ProviderRegistry<dyn AiProvider> =
        ai_routing_runtime::ProviderRegistry::new();
    for spec in &resolved.openai_providers {
        let provider: std::sync::Arc<dyn AiProvider> =
            std::sync::Arc::new(ai_runtime_adapters::GenericOpenAIAdapter::new_with_auth(
                spec.api_keys.first().cloned().unwrap_or_default(),
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

fn has_gui_session() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some()
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        (meta.permissions().mode() & 0o111) != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let full = dir.join(bin);
        if is_executable_file(&full) {
            return Some(full);
        }
    }
    None
}

fn spawn_new_terminal_tab(args: &[String]) -> Result<bool> {
    fn status_ok(bin: &str, args: &[&str]) -> bool {
        std::process::Command::new(bin)
            .args(args)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    // Prefer tmux when present: no GUI dependency, and "new tab" is a new tmux window.
    if std::env::var_os("TMUX").is_some() {
        if which("tmux").is_some() {
            let exe = std::env::current_exe().context("current_exe")?;
            let cmd = shell_join(exe.display().to_string().as_str(), args);
            if status_ok("tmux", &["new-window", "-n", "dracon-ai", &cmd]) {
                return Ok(true);
            }
        }
    }

    // GUI terminal emulators.
    if !has_gui_session() {
        return Ok(false);
    }

    let exe = std::env::current_exe().context("current_exe")?;
    let exe_s = exe.to_string_lossy().to_string();

    // WezTerm: try a new tab in an existing instance first.
    if which("wezterm").is_some() {
        // `wezterm cli spawn` returns quickly (good for success/failure detection).
        let mut a = vec!["cli", "spawn", "--new-tab", "--", &exe_s];
        let a_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        a.extend_from_slice(&a_refs);
        if status_ok("wezterm", &a) {
            return Ok(true);
        }
    }

    // Kitty: remote-control new tab only (we don't spawn a new window because the user asked for tabs).
    if which("kitty").is_some() {
        if std::env::var_os("KITTY_WINDOW_ID").is_some() {
            let mut a = vec![
                "@",
                "launch",
                "--type=tab",
                "--title",
                "dracon-ai",
                "--",
                &exe_s,
            ];
            let a_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            a.extend_from_slice(&a_refs);
            if status_ok("kitty", &a) {
                return Ok(true);
            }
        }
    }

    // GNOME Terminal (tabs).
    if which("gnome-terminal").is_some() {
        let mut c = std::process::Command::new("gnome-terminal");
        c.args(["--tab", "--title=dracon-ai", "--", &exe_s])
            .args(args);
        if c.spawn().is_ok() {
            return Ok(true);
        }
    }

    // Konsole (tabs).
    if which("konsole").is_some() {
        if std::process::Command::new("konsole")
            .args(["--new-tab", "-p", "tabtitle=dracon-ai", "-e", &exe_s])
            .args(args)
            .spawn()
            .is_ok()
        {
            return Ok(true);
        }
    }

    Ok(false)
}

fn shell_escape_simple(s: &str) -> String {
    // Minimal shell escaping for tmux command string: wrap in single quotes and escape single quotes.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn shell_join(exe: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(1 + args.len());
    parts.push(shell_escape_simple(exe));
    for a in args {
        parts.push(shell_escape_simple(a));
    }
    parts.join(" ")
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
    keep.extend(
        messages
            .iter()
            .rev()
            .take(MAX_MESSAGES.saturating_sub(1))
            .cloned()
            .rev(),
    );
    *messages = keep;
}

fn agent_system_prompt() -> String {
    // Hard rule: the agent must reply with pure JSON so the CLI can parse and act on it.
    // Keep this strict and repetitive; models tend to drift into Markdown otherwise.
    [
        "You are dracon-ai, a computer-context assistant.",
        "Your job is to propose shell commands to accomplish the user's task, then react to captured outputs.",
        "Environment: NixOS. Prefer declarative changes in the system repo and Nix tooling.",
        "Reply with ONLY a single JSON object matching this schema:",
        r#"{ "done": boolean, "summary": string, "commands": [ { "cmd": string, "why": string } ], "final_answer": string|null }"#,
        "Rules:",
        "- If you need to execute commands, set done=false and provide 1-3 commands.",
        "- Commands must be safe, minimal, and non-destructive by default.",
        "- Do NOT use `nix-env` (imperative installs) unless explicitly requested; prefer editing the Nix config under the system repo and rebuilding.",
        "- Avoid global `pip install` by default. Prefer Nix shells/devshells or project-local venv/uv workflows. If unsure, ask for the project context.",
        "- JSON must be strict RFC8259: no trailing commas, no comments, no NaN/Infinity.",
        "- Do not include markdown fences, no prose outside JSON, no backticks.",
        "- When finished, set done=true and provide final_answer.",
    ]
    .join("\n")
}

fn extract_first_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    
    for (i, ch) in s[start..].char_indices() {
        if in_str {
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..=start + i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn strip_trailing_commas(json: &str) -> String {
    // Minimal "JSON5 -> JSON" cleanup: remove trailing commas like:
    //   { "a": 1, }
    //   [1, 2,]
    // We do not attempt to support comments or other JSON5 features.
    let mut out = String::with_capacity(json.len());
    let mut chars = json.chars().peekable();
    let mut in_str = false;
    let mut esc = false;
    while let Some(ch) = chars.next() {
        if in_str {
            out.push(ch);
            if esc {
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_str = true;
                out.push(ch);
            }
            ',' => {
                // Peek ahead to see if this is a trailing comma before } or ].
                let mut look = chars.clone();
                while matches!(look.peek(), Some(c) if c.is_whitespace()) {
                    look.next();
                }
                if matches!(look.peek(), Some('}') | Some(']')) {
                    // Skip this comma.
                } else {
                    out.push(ch);
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Redact common secret patterns from command output before sending to AI provider.
fn redact_output(output: &str) -> String {
    let secret_patterns = [
        "password", "passwd", "secret", "token", "api_key", "apikey",
        "authorization", "bearer", "private_key", "ssh_key",
    ];
    
    // Collect lines first to avoid borrow issues
    let lines: Vec<&str> = output.lines().collect();
    let mut result = String::with_capacity(output.len());
    
    for line in &lines {
        let lower = line.to_ascii_lowercase();
        let mut redacted = false;
        for pat in &secret_patterns {
            if lower.contains(pat) && (lower.contains('=') || lower.contains(':')) {
                result.push_str(&format!("[REDACTED: contains {}]", pat));
                result.push('\n');
                redacted = true;
                break;
            }
        }
        if !redacted {
            result.push_str(line);
            result.push('\n');
        }
    }
    
    // Truncate very long outputs to limit token usage and accidental leak
    if result.len() > 20_000 {
        result.truncate(20_000);
        result.push_str("\n... [output truncated]");
    }
    result
}

fn is_dangerous_shell(cmd: &str) -> bool {
    let c = cmd.to_ascii_lowercase();
    let c = c.trim();
    // Prevent accidental foot-guns and data exfiltration.
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
        // Network exfiltration patterns
        || c.contains("curl ")
        || c.contains("wget ")
        || c.contains("nc ")
        || c.contains("ncat ")
        || c.contains("ssh ")
        || c.contains("scp ")
        || c.contains("sftp ")
        || c.contains("|nc ")
        || c.contains("|ncat ")
        // Dangerous permissions
        || c.contains("chmod 777")
        || c.contains("chmod -r 777")
        // Dangerous execution
        || c.contains(" eval ")
        || c.starts_with("eval ")
        || c.contains(" exec ")
        || c.starts_with("exec ")
}

fn run_setup(refresh: bool) -> Result<()> {
    use std::io::Write;

    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
    let ai_dir = home.join(".dracon").join("ai");
    let secrets_dir = ai_dir.join("secrets");
    std::fs::create_dir_all(&secrets_dir)?;

    println!("🔧 dracon-ai setup");
    println!();

    if !refresh {
        println!("Supported providers (auto-detected from key format):");
        println!("  OpenRouter  — sk-or-...  (recommended, broadest free coverage)");
        println!("  OpenAI      — sk-...");
        println!("  Google AI   — AIza...");
        println!("  NVIDIA      — nvapi-...");
        println!();
        println!("Enter your API key (or press Enter to skip):");
        print!("> ");
        std::io::stdout().flush()?;

        let mut key_input = String::new();
        std::io::stdin().read_line(&mut key_input)?;
        let key = key_input.trim();

        if !key.is_empty() {
            let provider = detect_provider(key);
            let (provider_id, endpoint, env_name) = provider_info(&provider);
            let env_path = secrets_dir.join(format!("{}.env", provider_id));
            std::fs::write(&env_path, format!("{}={}\n", env_name, key))?;
            println!("✅ Key saved to {}", env_path.display());
        }
    }

    // Discover from ALL existing keys
    let all_keys = load_all_keys(&secrets_dir);
    if all_keys.is_empty() {
        println!();
        println!("No API keys found. Add one:");
        println!("  echo 'OPENROUTER_API_KEY=sk-or-...' > ~/.dracon/utilities/sync/ai/secrets/openrouter.env");
        println!("  dracon-ai setup --refresh");
        return Ok(());
    }

    println!();
    println!("🔍 Discovering models from {} provider(s)...", all_keys.len());

    let mut all_providers = Vec::new();
    let mut all_active_ids = Vec::new();

    for (provider_id, endpoint, env_name, key, provider) in &all_keys {
        match discover_models(endpoint, key, *provider) {
            Ok(models) => {
                if models.is_empty() {
                    println!("   {}: no models found", provider_id);
                    continue;
                }
                println!("   {}: {} model(s) discovered", provider_id, models.len());

                for model_id in &models {
                    let full_id = if model_id.starts_with(&format!("{}/", provider_id)) {
                        model_id.clone()
                    } else {
                        format!("{}/{}", provider_id, model_id)
                    };

                    all_providers.push(build_provider_entry(
                        provider_id, endpoint, env_name, model_id, *provider,
                    ));
                    all_active_ids.push(full_id);
                }
            }
            Err(e) => {
                println!("   {}: discovery failed — {}", provider_id, e);
            }
        }
    }

    if all_providers.is_empty() {
        println!();
        println!("No models discovered. Check your API keys.");
        return Ok(());
    }

    let policy = serde_json::json!({
        "providers": all_providers,
        "active_model_ids": all_active_ids,
        "lane_model_policy": {
            "free:*": all_active_ids,
            "*:*": all_active_ids
        }
    });

    let policy_path = ai_dir.join("routing-policy.json");
    std::fs::write(&policy_path, serde_json::to_string_pretty(&policy)?)?;
    println!("✅ {} providers, {} models → {}", all_keys.len(), all_active_ids.len(), policy_path.display());
    println!();
    println!("Run 'dracon-ai status' to verify.");

    Ok(())
}

fn provider_info(provider: &DetectedProvider) -> (&str, &str, &str) {
    match provider {
        DetectedProvider::OpenRouter => ("openrouter", "https://openrouter.ai/api/v1", "OPENROUTER_API_KEY"),
        DetectedProvider::OpenAI => ("openai", "https://api.openai.com/v1", "OPENAI_API_KEY"),
        DetectedProvider::Google => ("google", "https://generativelanguage.googleapis.com/v1beta", "GOOGLE_API_KEY"),
        DetectedProvider::NVIDIA => ("nvidia", "https://integrate.api.nvidia.com/v1", "NVIDIA_API_KEY"),
        DetectedProvider::Unknown => ("custom", "", "CUSTOM_API_KEY"),
    }
}

fn load_all_keys(secrets_dir: &Path) -> Vec<(String, String, String, String, DetectedProvider)> {
    let mut keys = Vec::new();

    let entries = match std::fs::read_dir(secrets_dir) {
        Ok(e) => e,
        Err(_) => return keys,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "env") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((_, value)) = line.split_once('=') {
                let key = value.trim();
                if key.is_empty() {
                    continue;
                }
                let provider = detect_provider(key);
                let (pid, endpoint, env_name) = provider_info(&provider);
                if !endpoint.is_empty() {
                    keys.push((pid.to_string(), endpoint.to_string(), env_name.to_string(), key.to_string(), provider));
                }
            }
        }
    }

    keys
}

#[derive(Clone, Copy)]
enum DetectedProvider {
    OpenRouter,
    OpenAI,
    Google,
    NVIDIA,
    Unknown,
}

fn detect_provider(key: &str) -> DetectedProvider {
    if key.starts_with("sk-or-") || key.starts_with("sk-or-v1-") {
        DetectedProvider::OpenRouter
    } else if key.starts_with("sk-") {
        DetectedProvider::OpenAI
    } else if key.starts_with("AIza") {
        DetectedProvider::Google
    } else if key.starts_with("nvapi-") {
        DetectedProvider::NVIDIA
    } else {
        DetectedProvider::Unknown
    }
}

fn discover_models(endpoint: &str, key: &str, provider: DetectedProvider) -> Result<Vec<String>> {
    let url = format!("{}/models", endpoint.trim_end_matches('/'));
    let client = reqwest::blocking::Client::new();

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", key))
        .send()
        .with_context(|| format!("GET {}", url))?;

    if !resp.status().is_success() {
        anyhow::bail!("HTTP {} from {}", resp.status(), url);
    }

    let body: serde_json::Value = resp.json()?;
    let models = match provider {
        DetectedProvider::OpenRouter => {
            // OpenRouter: filter for free models
            body["data"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter(|m| {
                            let id = m["id"].as_str().unwrap_or("");
                            let pricing = &m["pricing"];
                            let prompt_free = pricing["prompt"].as_str() == Some("0");
                            let is_free = id.contains(":free") || prompt_free;
                            is_free
                        })
                        .filter_map(|m| m["id"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        }
        _ => {
            // Generic: return all model IDs
            body["data"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m["id"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        }
    };

    Ok(models)
}

fn build_provider_entry(
    provider_id: &str,
    endpoint: &str,
    env_name: &str,
    model_id: &str,
    provider: DetectedProvider,
) -> serde_json::Value {
    let auth_prefix = match provider {
        DetectedProvider::Google => "",
        _ => "Bearer ",
    };
    let auth_header = match provider {
        DetectedProvider::Google => "x-goog-api-key",
        _ => "Authorization",
    };

    let full_id = if model_id.starts_with(&format!("{}/", provider_id)) {
        model_id.to_string()
    } else {
        format!("{}/{}", provider_id, model_id)
    };

    serde_json::json!({
        "model_id": full_id,
        "api_key_envs": [env_name],
        "endpoint": endpoint,
        "provider_label": provider_id,
        "auth_header_name": auth_header,
        "auth_header_prefix": auth_prefix,
        "payload_model": model_id
    })
}

async fn run_scribe(repo: &Path) -> Result<()> {
    use std::process::Command as StdCommand;

    // Collect context
    let git_log = StdCommand::new("git")
        .args(["log", "--format=%s%n  files: %(trailers:key=file,valueonly)", "-20"])
        .current_dir(repo)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|_| "no git history".to_string());

    let git_files = StdCommand::new("git")
        .args(["log", "--oneline", "--name-only", "-10"])
        .current_dir(repo)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // Read blueprint if exists
    let blueprint = {
        let plan_dir = repo.join("plan");
        if plan_dir.exists() {
            std::fs::read_dir(&plan_dir)
                .ok()
                .and_then(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
                        .max_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()))
                })
                .and_then(|e| std::fs::read_to_string(e.path()).ok())
                .unwrap_or_default()
        } else {
            String::new()
        }
    };

    let prompt = format!(
        r#"You are a scribe for a software project. Analyze the git history and project state, then write a concise project-state.md.

## Recent Git Log
{git_log}

## Recent File Changes
{git_files}

## Blueprint
{blueprint}

## Instructions
Write a project-state.md file with EXACTLY this format (no preamble, no explanation):

# Project State

## Current Focus
{{one line: what the project is actively working on, based on recent commits and blueprint}}

## Completed
- [x] {{recent completed work from the log}}

## In Progress
- [x] {{what's actively being worked on based on recent file patterns}}

## Open Issues
- {{anything that looks broken or blocked based on the evidence}}

Keep it factual. Infer from the evidence, don't make things up. If unclear, say so.
Write ONLY the markdown, nothing else."#
    );

    // Build router
    let resolved = ai_runtime_adapters::resolve_ai_runtime_config();
    let mut registry: ai_routing_runtime::ProviderRegistry<dyn AiProvider> =
        ai_routing_runtime::ProviderRegistry::new();
    for spec in &resolved.openai_providers {
        let provider: std::sync::Arc<dyn AiProvider> =
            std::sync::Arc::new(ai_runtime_adapters::GenericOpenAIAdapter::new_with_auth(
                spec.api_keys.first().cloned().unwrap_or_default(),
                spec.endpoint.clone(),
                spec.payload_model.clone(),
                spec.auth_header_name.clone(),
                spec.auth_header_prefix.clone(),
            ));
        registry.register(&spec.model_id, provider);
    }

    let router = ai_routing_runtime::SmartRouter::new(
        registry,
        resolved.active_model_ids.clone(),
        resolved.dev_model_ids,
        resolved.lane_model_policy,
    );

    let messages = vec![ai_routing_runtime::RoutingMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    let (provider, _trace) = router
        .route_with_trace(
            "scribe",
            Some(RoutingTask::Free),
            None,
            &messages,
            SelectionConstraints::default(),
        )
        .await?;

    let req = ChatRequest {
        project_id: "scribe".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: messages[0].content.clone(),
        }],
        client_intent: Some(RoutingTask::Free),
        routing_constraints: SelectionConstraints::default(),
        resolved_service_level: None,
    };

    let (text, _usage) = provider.ask_and_collect(req).await?;

    // Write project-state.md
    let dracon_dir = repo.join(".dracon");
    std::fs::create_dir_all(&dracon_dir)?;
    let state_path = dracon_dir.join("project-state.md");

    // Extract just the markdown (in case AI added preamble)
    let markdown = if let Some(start) = text.find("# Project State") {
        &text[start..]
    } else {
        &text
    };

    std::fs::write(&state_path, markdown.trim())?;
    eprintln!("📝 scribe: updated {}", state_path.display());
    println!("{}", markdown.trim());

    Ok(())
}

async fn agent_next(
    router: &ai_routing_runtime::SmartRouter<dyn AiProvider>,
    messages: &[ChatMessage],
) -> Result<AgentStep> {
    let req = ChatRequest {
        project_id: "default".to_string(),
        messages: messages.to_vec(),
        client_intent: Some(RoutingTask::Custom("system".to_string())),
        routing_constraints: SelectionConstraints::default(),
        resolved_service_level: None,
    };
    let (text, selected_model) = {
        let (provider, trace) = router
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
        let (text, _usage) = provider.ask_and_collect(req).await?;
        (text, trace.selected_model)
    };

    let json = extract_first_json_object(&text).ok_or_else(|| anyhow!("agent returned no JSON"))?;
    let resp = match serde_json::from_str::<AgentResponse>(json) {
        Ok(v) => v,
        Err(e) => {
            let repaired = strip_trailing_commas(json);
            serde_json::from_str::<AgentResponse>(&repaired).with_context(|| {
                format!(
                    "failed parsing agent JSON (and repair failed).\nerror={}\njson={}",
                    e, json
                )
            })?
        }
    };

    Ok(AgentStep {
        selected_model,
        resp,
    })
}

async fn agent_next_with_ui(
    router: &ai_routing_runtime::SmartRouter<dyn AiProvider>,
    messages: &[ChatMessage],
    timeout: Duration,
) -> Result<AgentStep> {
    let spinner = Spinner::start(dim("thinking...".to_string().as_str()).to_string());
    let res = tokio::time::timeout(timeout, agent_next(router, messages)).await;
    spinner.stop().await;
    match res {
        Ok(r) => r,
        Err(_) => Err(anyhow!("agent timeout after {:?}", timeout)),
    }
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
    let cfg = load_config();
    let home = dirs::home_dir().unwrap_or_default();
    let system_root = cfg
        .system_root
        .clone()
        .unwrap_or_else(|| home.join("dracon"));
    let nixos_root = cfg
        .nixos_root
        .clone()
        .unwrap_or_else(|| home.join("dracon/nixos"));
    let mut messages: Vec<ChatMessage> = vec![
        ChatMessage {
            role: "system".to_string(),
            content: agent_system_prompt(),
        },
        ChatMessage {
            role: "system".to_string(),
            content: format!(
                "Context: os={} arch={} cwd={} system_root={} nixos_root={}",
                std::env::consts::OS,
                std::env::consts::ARCH,
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .display(),
                system_root.display(),
                nixos_root.display(),
            ),
        },
        ChatMessage {
            role: "user".to_string(),
            content: task.to_string(),
        },
    ];

    let mut commands_ran: Vec<String> = Vec::new();
    let mut last_answer: Option<String> = None;
    let mut repeat_guard: std::collections::BTreeMap<String, u32> =
        std::collections::BTreeMap::new();

    // Optional: if apply is enabled and task is Nix-ish, probe a little state up front.
    // This keeps "do mode" effective without making the user educate the agent each time.
    let task_lc = task.to_ascii_lowercase();
    let mentions_nix = task_lc.contains("nix")
        || task_lc.contains("nixos")
        || task_lc.contains("home-manager")
        || task_lc.contains("flake");
    if apply && cfg.do_auto_probe_nix && mentions_nix {
        let nixos_root = cfg
            .nixos_root
            .clone()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("dracon/nixos"));
        let probes = [
            ("nix --version", "confirm nix is available"),
            (
                "nixos-rebuild --version",
                "confirm rebuild tool is available",
            ),
            ("ls -la", "show cwd"),
            (
                &format!("ls -la {}", nixos_root.display()),
                "show nixos root",
            ),
            (
                &format!("git -C {} status -sb", nixos_root.display()),
                "show nixos git status",
            ),
        ];
        for (cmd, _why) in probes {
            let capture =
                run_shell_capture(cmd, Duration::from_secs(timeout_secs), max_bytes).await?;
            commands_ran.push(cmd.to_string());
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: format!(
                    "Pre-probe output.\ncmd={}\nexit={}\noutput:\n{}",
                    cmd,
                    capture
                        .status_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    redact_output(&capture.output)
                ),
            });
        }
    }

    for step_idx in 0..max_steps {
        let AgentStep {
            selected_model,
            resp: agent,
        } = agent_next_with_ui(router, &messages, Duration::from_secs(60)).await?;
        if agent.done {
            last_answer = agent.final_answer.or(Some(agent.summary));
            break;
        }

        if agent.commands.is_empty() {
            return Err(anyhow!("agent returned done=false but no commands"));
        }

        let sig = format!(
            "{}|{}",
            agent.summary,
            agent
                .commands
                .iter()
                .map(|c| c.cmd.as_str())
                .collect::<Vec<_>>()
                .join(" ; ")
        );
        let n = repeat_guard.entry(sig).or_insert(0);
        *n += 1;
        if *n >= 3 {
            return Ok(DoCliResponse {
                task: task.to_string(),
                content: format!(
                    "Agent appears stuck (repeated the same plan multiple times). Stopping.\nLast summary: {}",
                    agent.summary
                ),
                commands_ran,
            });
        }

        eprintln!(
            "{} {} {}",
            dim(&format!("step {}/{}:", step_idx + 1, max_steps)),
            ansi("33", &selected_model),
            agent.summary
        );
        for (i, c) in agent.commands.iter().enumerate() {
            if c.why.trim().is_empty() {
                eprintln!("{} {}", dim(&format!("  {}.", i + 1)), cyan(&c.cmd));
            } else {
                eprintln!(
                    "{} {} {}",
                    dim(&format!("  {}.", i + 1)),
                    cyan(&c.cmd),
                    dim(&format!("# {}", c.why.trim()))
                );
            }
        }

        if !apply {
            return Ok(DoCliResponse {
                task: task.to_string(),
                content: format!(
                    "Plan only (--plan).\nRemove --plan (or set DRACON_AI_APPLY=1) to allow execution.\n{}",
                    agent.summary
                ),
                commands_ran,
            });
        }

        for c in agent.commands {
            if !dangerous && is_dangerous_shell(&c.cmd) {
                // Don't hard-fail: print the command so the user can copy/paste, but refuse to
                // execute it automatically without an explicit opt-in.
                let mut out = String::new();
                out.push_str("Refused to run potentially dangerous command(s) without --dangerous (or DRACON_AI_DANGEROUS=1).\n");
                out.push_str("You can run these manually, or re-run with --dangerous to let dracon-ai execute them.\n\n");
                out.push_str("Suggested command:\n");
                out.push_str(&c.cmd);
                out.push('\n');
                if !c.why.trim().is_empty() {
                    out.push_str("Reason: ");
                    out.push_str(c.why.trim());
                    out.push('\n');
                }
                return Ok(DoCliResponse {
                    task: task.to_string(),
                    content: out,
                    commands_ran,
                });
            }
            let spinner = Spinner::start(dim(&format!("running: {}", c.cmd)));
            let capture =
                run_shell_capture(&c.cmd, Duration::from_secs(timeout_secs), max_bytes).await?;
            spinner.stop().await;
            commands_ran.push(c.cmd.clone());
            let msg = format!(
                "Command executed.\ncmd={}\nexit={}\noutput:\n{}",
                c.cmd,
                capture
                    .status_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                redact_output(&capture.output)
            );
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: msg,
            });
        }
    }

    if last_answer.is_none() {
        // If we hit the step limit after executing commands, do one final non-executing
        // reasoning pass so the user still gets a useful answer.
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: "You have reached the step limit. Produce a final answer now. Set done=true and commands=[].".to_string(),
        });
        if let Ok(step) = agent_next_with_ui(router, &messages, Duration::from_secs(45)).await {
            // Even if the model fails to follow the done=true instruction, we still want to
            // return something useful instead of a dead-end string.
            last_answer = step.resp.final_answer.or(Some(step.resp.summary.clone()));
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

    set_title("dracon-ai do");
    eprintln!(
        "{} {}",
        ansi("1;36", "dracon-ai"),
        dim("do mode. Type a task, Ctrl-D or /exit. /help for commands.")
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
    let mut last_task: Option<String> = None;

    loop {
        let prompt = match (cur_apply, cur_dangerous) {
            (true, true) => "do(apply=on,danger=on)> ",
            (true, false) => "do(apply=on,danger=off)> ",
            (false, true) => "do(apply=off,danger=on)> ",
            (false, false) => "do(apply=off,danger=off)> ",
        };
        let line = tokio::task::block_in_place(|| rl.readline(prompt));
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
                    eprintln!(
                        "{}",
                        dim("  /apply on|off       toggle execution (default on)")
                    );
                    eprintln!(
                        "{}",
                        dim("  /dangerous on|off   allow dangerous commands (sudo/rm/etc)")
                    );
                    eprintln!(
                        "{}",
                        dim("  /config             show resolved dracon-ai config")
                    );
                    eprintln!("{}", dim("  do so | do it       re-run last task (uses current apply/dangerous toggles)"));
                    eprintln!("{}", dim("  /exit               quit"));
                    continue;
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
                if line == "/config" {
                    let cfg = load_config();
                    let home = dirs::home_dir().unwrap_or_default();
                    let sr = cfg
                        .system_root
                        .clone()
                        .unwrap_or_else(|| home.join("dracon"));
                    let nr = cfg
                        .nixos_root
                        .clone()
                        .unwrap_or_else(|| home.join("dracon/nixos"));
                    eprintln!(
                        "{} system_root={} nixos_root={} do_auto_probe_nix={}",
                        dim("config:"),
                        sr.display(),
                        nr.display(),
                        cfg.do_auto_probe_nix
                    );
                    continue;
                }

                // UX sugar: allow "do so"/"do it" to re-run last task without retyping.
                let mut effective = line.to_string();
                if matches!(line, "do so" | "do it" | "run it") {
                    if let Some(t) = last_task.clone() {
                        effective = t;
                    } else {
                        eprintln!("{}", dim("no previous task to re-run"));
                        continue;
                    }
                }

                // Inline toggles: allow appending flag-like words in the REPL.
                if effective.contains("--apply") {
                    cur_apply = true;
                    effective = effective.replace("--apply", "").trim().to_string();
                }
                if effective.contains("--plan") {
                    cur_apply = false;
                    effective = effective.replace("--plan", "").trim().to_string();
                }
                if effective.contains("--no-apply") {
                    cur_apply = false;
                    effective = effective.replace("--no-apply", "").trim().to_string();
                }
                if effective.contains("--dangerous") {
                    cur_dangerous = true;
                    effective = effective.replace("--dangerous", "").trim().to_string();
                }
                if effective.is_empty() {
                    continue;
                }

                last_task = Some(effective.clone());
                let resp = run_do_task(
                    router,
                    &effective,
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

async fn run_shell_capture(
    cmd: &str,
    timeout: Duration,
    max_bytes: usize,
) -> Result<CommandCapture> {
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

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("missing stdout"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("missing stderr"))?;

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

    set_title("dracon-ai chat");
    let title = ansi("1;36", "dracon-ai");
    eprintln!(
        "{} {}",
        title,
        dim("interactive mode. Ctrl-D or /exit to quit. /help for commands.")
    );
    eprintln!(
        "{}",
        dim("Tip: /paste then paste multi-line text, end with /end.")
    );
    eprintln!(
        "{}",
        dim("Tip: /cmd <shell> captures local output into context (logs, status, etc).")
    );

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
                    eprintln!(
                        "{}",
                        dim("  /paste           begin multi-line paste (end with /end)")
                    );
                    eprintln!(
                        "{}",
                        dim("  /cmd <shell>     run local command, add output to context")
                    );
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
                                    redact_output(&capture.output)
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

                // REPL UX: collect-then-render.
                // This keeps the view readable (labels, selected model id) and avoids spinner/stream
                // interleaving in terminals.
                let out = OutputMode {
                    stream: false,
                    json: false,
                };

                let spinner = Spinner::start(dim("thinking...".to_string().as_str()).to_string());
                let resp = ask_with_messages(router, lane.clone(), None, &mut messages, out).await;
                spinner.stop().await;

                match resp {
                    Ok(resp) => {
                        print_assistant_header(&resp.selected_model);
                        print_markdownish(&resp.content);
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

fn print_assistant_header(selected_model: &str) {
    let who = magenta_bold("assistant");
    if selected_model.trim().is_empty() {
        println!("{who}");
        return;
    }
    println!(
        "{} {}",
        who,
        dim(&format!("(model: {})", ansi("33", selected_model)))
    );
}

fn print_markdownish(s: &str) {
    // Tiny renderer: code fences get a different color so "thinking prose" vs "code" is obvious.
    let mut in_code = false;
    for line in s.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            println!("{}", dim(line));
            continue;
        }
        if in_code {
            println!("{}", cyan(line));
        } else {
            println!("{}", line);
        }
    }
    if !s.ends_with('\n') {
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Cmd};
    use clap::Parser;

    #[test]
    fn agent_json_trailing_comma_is_repaired() {
        let bad = r#"{
  "done": false,
  "summary": "x",
  "commands": [
    { "cmd": "echo hi", "why": "x", }
  ],
  "final_answer": null,
}"#;
        let repaired = super::strip_trailing_commas(bad);
        let v: super::AgentResponse =
            serde_json::from_str(&repaired).expect("repaired json parses");
        assert!(!v.done);
        assert_eq!(v.commands.len(), 1);
        assert_eq!(v.commands[0].cmd, "echo hi");
    }

    #[test]
    fn parses_chat_with_default_intent() {
        let cli =
            Cli::try_parse_from(["dracon-ai", "chat", "hello", "world"]).expect("chat parses");
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

    #[test]
    fn parses_do_subcommand() {
        let cli = Cli::try_parse_from(["dracon-ai", "do", "add", "nix", "package", "ripgrep"])
            .expect("do parses");
        match cli.cmd.expect("cmd") {
            Cmd::Do { task, plan, .. } => {
                assert!(!plan);
                assert_eq!(task, vec!["add", "nix", "package", "ripgrep"]);
            }
            _ => panic!("expected do"),
        }
    }

    #[test]
    fn parses_do_plan() {
        let cli = Cli::try_parse_from(["dracon-ai", "do", "--plan", "echo", "hi"]).expect("do");
        match cli.cmd.expect("cmd") {
            Cmd::Do { plan, .. } => assert!(plan),
            _ => panic!("expected do"),
        }
    }
}
