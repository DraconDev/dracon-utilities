use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use notify::{Event, RecursiveMode, Watcher};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const BLOCK_BEGIN: &str = "# --- BEGIN DRACON MANAGED BLOCK ---";
const BLOCK_END: &str = "# --- END DRACON MANAGED BLOCK ---";

#[derive(Parser, Debug)]
#[command(name = "dracon-warden")]
#[command(about = "Lightweight Warden runtime")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run forever with filesystem event debounce.
    Daemon,
    /// Run one hardening pass and exit.
    Once,
    /// Show resolved policy path and watch roots.
    Status,
}

#[derive(Debug, Deserialize, Clone)]
struct WardenPolicy {
    #[serde(default)]
    protected_patterns: Vec<String>,
    #[serde(default)]
    plaintext_patterns: Vec<String>,
    #[serde(default)]
    hygiene_patterns: Vec<String>,
    #[serde(default)]
    watch_roots: Vec<String>,
}

impl WardenPolicy {
    fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read policy {}", path.display()))?;
        let policy: Self = toml::from_str(&content)
            .with_context(|| format!("failed to parse policy {}", path.display()))?;
        Ok(policy)
    }

    fn watch_root_paths(&self) -> Vec<PathBuf> {
        self.watch_roots
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect()
    }
}

fn resolve_policy_path() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("DRACON_WARDEN_POLICY") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Ok(p);
        }
    }

    if let Ok(custom) = std::env::var("DRACON_SECURITY_POLICY") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Ok(p);
        }
    }

    let home = dirs::home_dir().context("home not found")?;
    let candidates = [
        home.join("dracon/utilities/warden/dracon-warden.toml"),
        home.join("dracon/utilities/warden/dracon-security.toml"),
        home.join("dracon/utilities/warden/config.toml"),
        home.join("dracon/security/dracon-security.toml"),
    ];

    for p in &candidates {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    Err(anyhow::anyhow!(
        "policy not found. checked: {} (or DRACON_WARDEN_POLICY/DRACON_SECURITY_POLICY)",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn discover_git_repos(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut repos = BTreeSet::new();

    for root in roots {
        if root.join(".git").exists() {
            repos.insert(root.clone());
        }

        let walker = walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                if e.depth() == 0 {
                    return true;
                }
                if name == "target" || name == "node_modules" || name == ".cache" || name == ".direnv" {
                    return false;
                }
                true
            });

        for entry in walker.filter_map(|e| e.ok()) {
            if !entry.file_type().is_dir() {
                continue;
            }
            if entry.file_name() == ".git" {
                if let Some(parent) = entry.path().parent() {
                    repos.insert(parent.to_path_buf());
                }
            }
        }
    }

    repos.into_iter().collect()
}

fn replace_managed_block(current: &str, managed_block: &str) -> String {
    if let Some(start) = current.find(BLOCK_BEGIN) {
        if let Some(end_rel) = current[start..].find(BLOCK_END) {
            let end = start + end_rel + BLOCK_END.len();
            let tail = current[end..].trim_start_matches(&['\r', '\n'][..]);
            let mut out = String::new();
            out.push_str(&current[..start]);
            if !out.ends_with('\n') && !out.is_empty() {
                out.push('\n');
            }
            out.push_str(managed_block);
            if !tail.is_empty() {
                out.push('\n');
                out.push_str(tail);
            } else if !managed_block.ends_with('\n') {
                out.push('\n');
            }
            return out;
        }
    }

    let mut out = current.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(managed_block);
    if !managed_block.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn build_gitignore_block(policy: &WardenPolicy) -> String {
    let mut lines = Vec::new();
    lines.push(BLOCK_BEGIN.to_string());
    lines.push("# managed by dracon-warden".to_string());
    for p in &policy.hygiene_patterns {
        lines.push(p.clone());
    }
    for p in &policy.protected_patterns {
        lines.push(format!("!{}", p));
    }
    for p in &policy.plaintext_patterns {
        lines.push(format!("!{}", p));
    }
    lines.push(BLOCK_END.to_string());
    lines.join("\n")
}

fn build_gitattributes_block(policy: &WardenPolicy) -> String {
    let mut lines = Vec::new();
    lines.push(BLOCK_BEGIN.to_string());
    lines.push("# managed by dracon-warden".to_string());
    for p in &policy.protected_patterns {
        lines.push(format!("{} filter=dracon diff=dracon merge=dracon", p));
    }
    for p in &policy.plaintext_patterns {
        lines.push(format!("{} -filter -diff -merge", p));
    }
    lines.push(BLOCK_END.to_string());
    lines.join("\n")
}

fn apply_managed_file(path: &Path, block: &str) -> Result<bool> {
    let current = fs::read_to_string(path).unwrap_or_default();
    let next = replace_managed_block(&current, block);
    if next != current {
        fs::write(path, next).with_context(|| format!("failed writing {}", path.display()))?;
        return Ok(true);
    }
    Ok(false)
}

fn harden_repo(repo: &Path, policy: &WardenPolicy) -> Result<(bool, bool)> {
    let gitignore_path = repo.join(".gitignore");
    let gitattributes_path = repo.join(".gitattributes");

    let gitignore_changed = apply_managed_file(&gitignore_path, &build_gitignore_block(policy))?;
    let gitattributes_changed = apply_managed_file(&gitattributes_path, &build_gitattributes_block(policy))?;

    Ok((gitignore_changed, gitattributes_changed))
}

fn harden_all(policy: &WardenPolicy) -> Result<()> {
    let roots = policy.watch_root_paths();
    let repos = discover_git_repos(&roots);

    let mut changed = 0usize;
    for repo in repos {
        match harden_repo(&repo, policy) {
            Ok((a, b)) => {
                if a || b {
                    changed += 1;
                    println!("🔒 hardened {}", repo.display());
                }
            }
            Err(e) => eprintln!("⚠️ harden failed for {}: {}", repo.display(), e),
        }
    }

    println!("✅ hardening pass complete (repos changed: {})", changed);
    Ok(())
}

fn should_process_event(event: &Event, roots: &[PathBuf]) -> bool {
    let ignore_fragments = ["/target/", "/node_modules/", "/.cache/", "/.git/objects/", "/.git/index.lock"];

    for p in &event.paths {
        let s = p.to_string_lossy();
        if ignore_fragments.iter().any(|f| s.contains(f)) {
            continue;
        }
        if roots.iter().any(|r| p.starts_with(r)) {
            return true;
        }
    }
    false
}

fn run_daemon(policy_path: PathBuf) -> Result<()> {
    let policy = WardenPolicy::load(&policy_path)?;
    let roots = policy.watch_root_paths();
    if roots.is_empty() {
        return Err(anyhow::anyhow!("no valid watch_roots in policy"));
    }

    harden_all(&policy)?;

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;

    for root in &roots {
        watcher.watch(root, RecursiveMode::Recursive)?;
    }

    println!("🛡️ dracon-warden active. Monitoring {:?}", roots);

    let mut last_run = Instant::now();
    let debounce = Duration::from_secs(2);
    let mut dirty = false;

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
                if should_process_event(&event, &roots) {
                    dirty = true;
                }
            }
            Ok(Err(e)) => {
                eprintln!("⚠️ watch error: {}", e);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(anyhow::anyhow!("watch channel disconnected"));
            }
        }

        if dirty && last_run.elapsed() >= debounce {
            let policy = WardenPolicy::load(&policy_path)?;
            harden_all(&policy)?;
            last_run = Instant::now();
            dirty = false;
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let policy_path = resolve_policy_path()?;

    match cli.cmd {
        Command::Status => {
            let policy = WardenPolicy::load(&policy_path)?;
            println!("📜 POLICY: {}", policy_path.display());
            println!("🛡️ ROOTS: {:?}", policy.watch_root_paths());
        }
        Command::Once => {
            let policy = WardenPolicy::load(&policy_path)?;
            harden_all(&policy)?;
        }
        Command::Daemon => {
            run_daemon(policy_path)?;
        }
    }

    Ok(())
}
