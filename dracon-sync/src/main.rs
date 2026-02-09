use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dracon_git::{GitService, build_sync_commit_payload};
use dracon_protocols::git::{DiffFile as ProtoDiffFile, FileStatus as ProtoFileStatus, RepoStatus as ProtoRepoStatus};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use tokio::time::{Duration, sleep};

#[derive(Parser, Debug)]
#[command(name = "dracon-sync")]
#[command(about = "Dracon sync runtime")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Show resolved policy path and sync scope.
    Status,
    /// Run one sync pass.
    Once,
    /// Run continuous sync loop.
    Daemon,
    /// Sync a specific repository now.
    SyncNow { repo: PathBuf },
}

#[derive(Debug, Deserialize, Clone)]
struct SyncPolicy {
    #[serde(default)]
    system_repo: String,
    #[serde(default = "default_pulse_interval")]
    pulse_interval_secs: u64,
    #[serde(default = "default_true")]
    auto_commit: bool,
    #[serde(default = "default_true")]
    auto_pull: bool,
    #[serde(default = "default_true")]
    auto_push: bool,
    #[serde(default)]
    backup_policy: String,
    #[serde(default)]
    backup_dir: String,
    #[serde(default)]
    watch_roots: Vec<String>,
    #[serde(default)]
    extra_remotes: HashMap<String, String>,
}

fn default_true() -> bool {
    true
}

fn default_pulse_interval() -> u64 {
    300
}

impl SyncPolicy {
    fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
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
    if let Ok(custom) = std::env::var("DRACON_SYNC_POLICY") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Ok(p);
        }
    }

    let home = dirs::home_dir().context("home not found")?;
    let candidates = [
        home.join("dracon/utilities/sync/dracon-sync.toml"),
        home.join("dracon/utilities/sync/config.toml"),
        home.join("dracon/git/dracon-git.toml"),
        home.join("demon/git/dracon-git.toml"),
    ];

    for p in &candidates {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    Err(anyhow::anyhow!(
        "sync policy not found. checked: {} (or DRACON_SYNC_POLICY)",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn is_excluded_dir_name(name: &str) -> bool {
    matches!(
        name,
        "target"
            | "node_modules"
            | ".cache"
            | ".direnv"
            | ".venv"
            | "dist"
            | "build"
            | "archives"
            | ".git"
    )
}

fn to_proto_status(s: &dracon_git::types::RepoStatus) -> ProtoRepoStatus {
    ProtoRepoStatus {
        branch: s.branch.clone(),
        ahead: s.ahead,
        behind: s.behind,
        modified_files: s.modified_files,
        staged_files: s.staged_files,
        is_clean: s.is_clean,
        last_commit_msg: s.last_commit_msg.clone(),
        last_commit_hash: s.last_commit_hash.clone(),
    }
}

fn to_proto_entries(entries: &[dracon_git::types::DiffFile]) -> Vec<ProtoDiffFile> {
    entries
        .iter()
        .map(|e| ProtoDiffFile {
            path: e.path.clone(),
            status: match e.status {
                dracon_git::types::FileStatus::Modified => ProtoFileStatus::Modified,
                dracon_git::types::FileStatus::Added => ProtoFileStatus::Added,
                dracon_git::types::FileStatus::Deleted => ProtoFileStatus::Deleted,
                dracon_git::types::FileStatus::Renamed => ProtoFileStatus::Renamed,
                dracon_git::types::FileStatus::TypeChange => ProtoFileStatus::TypeChange,
                dracon_git::types::FileStatus::Unknown => ProtoFileStatus::Unknown,
            },
        })
        .collect()
}

fn discover_git_repos(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut repos = BTreeSet::new();

    for root in roots {
        if root.join(".git").exists() {
            repos.insert(root.clone());
        }

        let walker = walkdir::WalkDir::new(root)
            .follow_links(false)
            .max_depth(7)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                let name = e.file_name().to_string_lossy();
                !is_excluded_dir_name(&name)
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

async fn sync_repo(repo: &Path, policy: &SyncPolicy) -> Result<bool> {
    let svc = GitService::new(repo)?;
    if !svc.is_git_repo().await? {
        return Ok(false);
    }

    if policy.auto_pull {
        let _ = svc.pull_rebase().await;
    }

    let status = svc.get_status().await?;
    if !status.is_clean && policy.auto_commit {
        let entries = svc.get_diff_entries().await?;
        if !entries.is_empty() {
            let proto_status = to_proto_status(&status);
            let proto_entries = to_proto_entries(&entries);
            let msg = build_sync_commit_payload(repo, &proto_status, &proto_entries);
            svc.commit_all(&msg).await?;
            if policy.auto_push {
                let _ = svc.push().await;
            }
            return Ok(true);
        }
    }

    if policy.auto_push && status.ahead > 0 {
        let _ = svc.push().await;
    }

    Ok(false)
}

async fn run_once(policy_path: &Path) -> Result<()> {
    let policy = SyncPolicy::load(policy_path)?;
    let roots = policy.watch_root_paths();
    let repos = discover_git_repos(&roots);

    let mut changed = 0usize;
    for repo in repos {
        match sync_repo(&repo, &policy).await {
            Ok(true) => {
                changed += 1;
                println!("🔁 synced {}", repo.display());
            }
            Ok(false) => {}
            Err(e) => eprintln!("⚠️ sync failed for {}: {}", repo.display(), e),
        }
    }

    println!("✅ sync pass complete (repos changed: {})", changed);
    Ok(())
}

async fn run_daemon(policy_path: PathBuf) -> Result<()> {
    loop {
        if let Err(e) = run_once(&policy_path).await {
            eprintln!("⚠️ sync pass failed: {}", e);
        }

        let interval = SyncPolicy::load(&policy_path)
            .map(|p| p.pulse_interval_secs.max(5))
            .unwrap_or(300);
        sleep(Duration::from_secs(interval)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let policy_path = resolve_policy_path()?;

    match cli.cmd {
        Command::Status => {
            let policy = SyncPolicy::load(&policy_path)?;
            println!("📜 POLICY: {}", policy_path.display());
            println!("🔁 ROOTS: {:?}", policy.watch_root_paths());
            println!("⏱️ PULSE: {}s", policy.pulse_interval_secs);
            println!(
                "⚙️ FLAGS: auto_commit={} auto_pull={} auto_push={}",
                policy.auto_commit, policy.auto_pull, policy.auto_push
            );
            if !policy.system_repo.is_empty() {
                println!("🏛️ SYSTEM_REPO: {}", policy.system_repo);
            }
            if !policy.backup_policy.is_empty() || !policy.backup_dir.is_empty() {
                println!(
                    "🧰 BACKUP: policy={} dir={}",
                    policy.backup_policy, policy.backup_dir
                );
            }
            println!("🌐 EXTRA_REMOTES: {}", policy.extra_remotes.len());
        }
        Command::Once => {
            run_once(&policy_path).await?;
        }
        Command::Daemon => {
            run_daemon(policy_path).await?;
        }
        Command::SyncNow { repo } => {
            let policy = SyncPolicy::load(&policy_path)?;
            if sync_repo(&repo, &policy).await? {
                println!("🔁 synced {}", repo.display());
            } else {
                println!("✅ no sync changes {}", repo.display());
            }
        }
    }

    Ok(())
}
