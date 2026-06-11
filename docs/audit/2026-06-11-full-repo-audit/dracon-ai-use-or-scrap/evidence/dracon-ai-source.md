
===== dracon-ai/src/main.rs =====
use ai_runtime_config::resolve_ai_runtime_config;
use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use dracon_ai_contracts::{RoutingTask, SelectionConstraints};
use dracon_ai_runtime_contracts::models::{ChatMessage, ChatRequest};
use dracon_ai_runtime_contracts::traits::AiProvider;
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
