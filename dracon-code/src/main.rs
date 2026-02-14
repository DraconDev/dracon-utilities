use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(
    name = "dracon-code",
    about = "Repo scaffolding + context persistence utility (do.md + plan/)",
    version
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Cmd {
    /// Initialize do.md + plan/ scaffolding in the current repo directory.
    Init {
        /// Overwrite existing files.
        #[arg(long)]
        force: bool,
    },

    /// Append a message to plan/chat.md (timestamped).
    Append {
        /// Read message from stdin.
        #[arg(long)]
        stdin: bool,

        /// Optional explicit author (defaults to $USER).
        #[arg(long)]
        author: Option<String>,

        /// Optional model id (defaults to $DRACON_AI_SELECTED_MODEL / $DRACON_AI_MODEL if set).
        #[arg(long)]
        model: Option<String>,

        /// Message text. If omitted, must use --stdin.
        #[arg(value_name = "TEXT", num_args = 0..)]
        text: Vec<String>,
    },

    /// Write a quick repo snapshot into plan/CONTEXT.md.
    Snapshot {
        /// Overwrite the file instead of appending.
        #[arg(long)]
        overwrite: bool,
    },
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn write_file(path: &Path, content: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn append_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    f.write_all(content.as_bytes())
        .with_context(|| format!("append {}", path.display()))?;
    Ok(())
}

fn is_git_repo(dir: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git_output(dir: &Path, args: &[&str]) -> String {
    match Command::new("git").args(args).current_dir(dir).output() {
        Ok(o) => {
            let mut s = String::new();
            s.push_str(&String::from_utf8_lossy(&o.stdout));
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            s.trim().to_string()
        }
        Err(_) => "<git failed>".to_string(),
    }
}

fn templates_do_md() -> &'static str {
    r#"# do.md

Single human-owned file. Dump intent here.

Rules:
- You edit `do.md`.
- Tools/AI edit `plan/`.
- Git is the history.

## Now
- [ ] <write the task>

"#
}

fn templates_plan_readme() -> &'static str {
    r#"# plan/

AI/tool-managed state.

Human rule: do not edit files in `plan/` directly (except to delete/reset them if you want a hard reset).

Files:
- `roadmap.md`: queue / plan steps
- `chat.md`: execution log / transcripts
- `CONTEXT.md`: snapshots (git status, environment notes)
- `DECISIONS.md`: decision log (why we chose X)
"#
}

fn templates_plan_roadmap() -> &'static str {
    r#"# Roadmap

This file is tool/AI-managed.

- [ ] <next steps>
"#
}

fn templates_plan_chat() -> &'static str {
    r#"# Chat Log

This file is append-only.
"#
}

fn templates_plan_context() -> &'static str {
    r#"# Context

This file is tool/AI-managed.
"#
}

fn templates_plan_decisions() -> &'static str {
    r#"# Decisions

Append decisions with a date + rationale.
"#
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().context("current_dir")?;

    match cli.cmd {
        Cmd::Init { force } => {
            if !is_git_repo(&cwd) {
                return Err(anyhow!("not a git repo (run inside a repo)"));
            }

            write_file(&cwd.join("do.md"), templates_do_md(), force)?;
            write_file(
                &cwd.join("plan").join("README.md"),
                templates_plan_readme(),
                force,
            )?;
            write_file(
                &cwd.join("plan").join("roadmap.md"),
                templates_plan_roadmap(),
                force,
            )?;
            write_file(
                &cwd.join("plan").join("chat.md"),
                templates_plan_chat(),
                force,
            )?;
            write_file(
                &cwd.join("plan").join("CONTEXT.md"),
                templates_plan_context(),
                force,
            )?;
            write_file(
                &cwd.join("plan").join("DECISIONS.md"),
                templates_plan_decisions(),
                force,
            )?;

            println!("Initialized do.md + plan/ (force={})", force);
            Ok(())
        }
        Cmd::Append {
            stdin,
            author,
            model,
            text,
        } => {
            let msg = if stdin {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            } else if text.is_empty() {
                return Err(anyhow!("missing TEXT (or pass --stdin)"));
            } else {
                text.join(" ")
            };

            let author = author
                .or_else(|| std::env::var("USER").ok())
                .unwrap_or_else(|| "unknown".to_string());
            let model = model
                .or_else(|| std::env::var("DRACON_AI_SELECTED_MODEL").ok())
                .or_else(|| std::env::var("DRACON_AI_MODEL").ok());

            let mut entry = String::new();
            entry.push('\n');
            entry.push_str("## ");
            entry.push_str(&now_unix().to_string());
            entry.push('\n');
            entry.push_str("- author: ");
            entry.push_str(&author);
            entry.push('\n');
            if let Some(m) = model {
                entry.push_str("- model: ");
                entry.push_str(&m);
                entry.push('\n');
            }
            entry.push('\n');
            entry.push_str(msg.trim_end());
            entry.push('\n');

            append_file(&cwd.join("plan").join("chat.md"), &entry)?;
            println!("Appended to plan/chat.md");
            Ok(())
        }
        Cmd::Snapshot { overwrite } => {
            if !is_git_repo(&cwd) {
                return Err(anyhow!("not a git repo (run inside a repo)"));
            }

            let mut out = String::new();
            out.push('\n');
            out.push_str("## snapshot ");
            out.push_str(&now_unix().to_string());
            out.push('\n');
            out.push_str("cwd: ");
            out.push_str(&cwd.display().to_string());
            out.push('\n');
            out.push_str("\n### git status -sb\n");
            out.push_str(&git_output(&cwd, &["status", "-sb"]));
            out.push('\n');
            out.push_str("\n### git diff --stat\n");
            out.push_str(&git_output(&cwd, &["diff", "--stat"]));
            out.push('\n');

            let path = cwd.join("plan").join("CONTEXT.md");
            if overwrite {
                fs::write(&path, out).with_context(|| format!("write {}", path.display()))?;
            } else {
                append_file(&path, &out)?;
            }
            println!("Wrote plan/CONTEXT.md (overwrite={})", overwrite);
            Ok(())
        }
    }
}
