use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dracon_git::{GitService, build_sync_commit_payload};
use dracon_protocols::git::{DiffFile as ProtoDiffFile, FileStatus as ProtoFileStatus, RepoStatus as ProtoRepoStatus};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use tokio::process::Command as TokioCommand;
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
    #[serde(default = "default_exclude_dir_names")]
    exclude_dir_names: Vec<String>,
    #[serde(default = "default_max_stage_file_bytes")]
    max_stage_file_bytes: u64,
}

fn default_true() -> bool {
    true
}

fn default_pulse_interval() -> u64 {
    300
}

fn default_exclude_dir_names() -> Vec<String> {
    [
        "target",
        "node_modules",
        ".cache",
        ".direnv",
        ".venv",
        "dist",
        "build",
        "archives",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_max_stage_file_bytes() -> u64 {
    100 * 1024 * 1024
}

impl SyncPolicy {
    fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read policy {}", path.display()))?;
        let mut policy: Self = toml::from_str(&content)
            .with_context(|| format!("failed to parse policy {}", path.display()))?;
        if policy.exclude_dir_names.is_empty() {
            policy.exclude_dir_names = default_exclude_dir_names();
        }
        if policy.max_stage_file_bytes == 0 {
            policy.max_stage_file_bytes = default_max_stage_file_bytes();
        }
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

fn normalized_dir_name(value: &str) -> String {
    value.trim_matches('/').to_ascii_lowercase()
}

fn excluded_dir_names_set(policy: &SyncPolicy) -> BTreeSet<String> {
    policy
        .exclude_dir_names
        .iter()
        .map(|d| normalized_dir_name(d))
        .filter(|d| !d.is_empty())
        .collect()
}

fn is_excluded_dir_name(name: &str, excluded_dir_names: &BTreeSet<String>) -> bool {
    excluded_dir_names.contains(&normalized_dir_name(name))
}

fn is_excluded_change_path(path: &Path, excluded_dir_names: &BTreeSet<String>) -> bool {
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .any(|name| is_excluded_dir_name(name, excluded_dir_names))
}

fn should_stage_entry(
    repo: &Path,
    entry: &dracon_git::types::DiffFile,
    excluded_dir_names: &BTreeSet<String>,
    max_stage_file_bytes: u64,
) -> bool {
    if matches!(entry.status, dracon_git::types::FileStatus::Deleted) {
        return true;
    }

    if is_excluded_change_path(&entry.path, excluded_dir_names) {
        return false;
    }

    let full_path = repo.join(&entry.path);
    match std::fs::metadata(&full_path) {
        Ok(meta) if meta.is_file() => {
            if meta.len() > max_stage_file_bytes {
                eprintln!(
                    "ℹ️ skip large file {} ({} bytes > {} bytes)",
                    full_path.display(),
                    meta.len(),
                    max_stage_file_bytes
                );
                return false;
            }
            true
        }
        Ok(_) => true,
        Err(_) => true,
    }
}

fn env_freeze_enabled() -> bool {
    matches!(
        std::env::var("DRACON_SYNC_FREEZE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn debug_enabled() -> bool {
    matches!(
        std::env::var("DRACON_SYNC_DEBUG")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn freeze_marker_paths(policy_path: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(dir) = policy_path.parent() {
        paths.push(dir.join(".freeze"));
        paths.push(dir.join("freeze"));
    }
    paths
}

fn freeze_reason(policy_path: &Path) -> Option<String> {
    if env_freeze_enabled() {
        return Some("env DRACON_SYNC_FREEZE".to_string());
    }

    for marker in freeze_marker_paths(policy_path) {
        if marker.exists() {
            return Some(format!("marker {}", marker.display()));
        }
    }

    None
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

fn discover_git_repos(roots: &[PathBuf], excluded_dir_names: &BTreeSet<String>) -> Vec<PathBuf> {
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
                !is_excluded_dir_name(&name, excluded_dir_names)
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

const PULL_OP_TIMEOUT_SECS: u64 = 20;
const PUSH_OP_TIMEOUT_SECS: u64 = 60;
const REPO_SYNC_TIMEOUT_SECS: u64 = 90;

fn has_origin_remote(repo: &Path) -> bool {
    std::process::Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_tracking_upstream(repo: &Path) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn kill_descendants(pid: u32) {
    let pid_s = pid.to_string();
    let _ = TokioCommand::new("pkill")
        .args(["-TERM", "-P", &pid_s])
        .output()
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = TokioCommand::new("pkill")
        .args(["-KILL", "-P", &pid_s])
        .output()
        .await;
}

async fn run_git_with_timeout(
    repo: &Path,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
) -> Result<()> {
    let mut child = TokioCommand::new("git")
        .args(args)
        .current_dir(repo)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn git {} in {}", op_label, repo.display()))?;

    let pid = child.id();
    match tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait()).await {
        Ok(Ok(status)) => {
            if status.success() {
                return Ok(());
            }
            return Err(anyhow::anyhow!(
                "git {} failed in {} with status {}",
                op_label,
                repo.display(),
                status
            ));
        }
        Ok(Err(e)) => {
            return Err(anyhow::anyhow!(
                "git {} failed in {}: {}",
                op_label,
                repo.display(),
                e
            ));
        }
        Err(_) => {
            if let Some(pid) = pid {
                kill_descendants(pid).await;
            }
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(anyhow::anyhow!(
                "git {} timeout in {} after {}s",
                op_label,
                repo.display(),
                timeout_secs
            ));
        }
    }
}

async fn git_list_paths(repo: &Path, args: &[&str]) -> Result<Vec<PathBuf>> {
    let output = TokioCommand::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .await
        .with_context(|| format!("failed to run git {:?} in {}", args, repo.display()))?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect())
}

async fn cli_diff_entries(repo: &Path) -> Result<Vec<dracon_git::types::DiffFile>> {
    let mut paths = BTreeSet::new();
    for args in [
        &["diff", "--name-only"][..],
        &["diff", "--cached", "--name-only"][..],
        &["ls-files", "--others", "--exclude-standard"][..],
    ] {
        for p in git_list_paths(repo, args).await? {
            paths.insert(p);
        }
    }

    Ok(paths
        .into_iter()
        .map(|path| dracon_git::types::DiffFile {
            path,
            status: dracon_git::types::FileStatus::Modified,
        })
        .collect())
}

async fn sync_repo(
    repo: &Path,
    policy: &SyncPolicy,
    excluded_dir_names: &BTreeSet<String>,
) -> Result<bool> {
    let svc = GitService::new(repo)?;
    if !svc.is_git_repo().await? {
        if debug_enabled() {
            eprintln!("🐛 {} is not recognized as git repo", repo.display());
        }
        return Ok(false);
    }
    let has_origin = has_origin_remote(repo);
    let has_upstream = has_tracking_upstream(repo);

    if policy.auto_pull && has_origin && has_upstream {
        match run_git_with_timeout(
            repo,
            &["pull", "--rebase", "--autostash"],
            PULL_OP_TIMEOUT_SECS,
            "pull/rebase",
        )
        .await
        {
            Ok(()) => {}
            Err(e) => eprintln!("⚠️ pull/rebase skipped for {}: {}", repo.display(), e),
        }
    } else if policy.auto_pull && !has_origin {
        eprintln!("ℹ️ skip pull/rebase for {} (no origin remote)", repo.display());
    } else if policy.auto_pull && has_origin && !has_upstream {
        eprintln!(
            "ℹ️ skip pull/rebase for {} (no tracking upstream on current branch)",
            repo.display()
        );
    }

    let mut status = svc.get_status().await?;
    let mut entries = svc.get_diff_entries().await?;
    if debug_enabled() {
        eprintln!(
            "🐛 {} status: clean={} modified={} staged={} entries(libgit2)={}",
            repo.display(),
            status.is_clean,
            status.modified_files,
            status.staged_files,
            entries.len()
        );
    }
    if entries.is_empty() {
        let fallback_entries = cli_diff_entries(repo).await?;
        if !fallback_entries.is_empty() {
            status.is_clean = false;
            status.modified_files = fallback_entries.len();
            entries = fallback_entries;
            if debug_enabled() {
                eprintln!(
                    "🐛 {} fallback entries(cli)={} => forcing dirty",
                    repo.display(),
                    status.modified_files
                );
            }
        }
    }

    if !status.is_clean && policy.auto_commit {
        let filtered_entries: Vec<_> = entries
            .into_iter()
            .filter(|e| {
                should_stage_entry(repo, e, excluded_dir_names, policy.max_stage_file_bytes)
            })
            .collect();
        if debug_enabled() {
            eprintln!(
                "🐛 {} filtered_entries={}",
                repo.display(),
                filtered_entries.len()
            );
        }
        if !filtered_entries.is_empty() {
            let proto_status = to_proto_status(&status);
            let proto_entries = to_proto_entries(&filtered_entries);
            let msg = build_sync_commit_payload(repo, &proto_status, &proto_entries);
            let stage_paths: Vec<String> = filtered_entries
                .iter()
                .map(|e| e.path.to_string_lossy().to_string())
                .collect();
            svc.add_paths(&stage_paths).await?;
            svc.commit(&msg).await?;
            if policy.auto_push && has_origin {
                match run_git_with_timeout(
                    repo,
                    &["push", "origin", "HEAD"],
                    PUSH_OP_TIMEOUT_SECS,
                    "push",
                )
                .await
                {
                    Ok(()) => {}
                    Err(e) => eprintln!("⚠️ push skipped for {}: {}", repo.display(), e),
                }
            } else if policy.auto_push && !has_origin {
                eprintln!("ℹ️ skip push for {} (no origin remote)", repo.display());
            }
            return Ok(true);
        }
    }

    if policy.auto_push && status.ahead > 0 && has_origin {
        match run_git_with_timeout(
            repo,
            &["push", "origin", "HEAD"],
            PUSH_OP_TIMEOUT_SECS,
            "push",
        )
        .await
        {
            Ok(()) => {}
            Err(e) => eprintln!("⚠️ push skipped for {}: {}", repo.display(), e),
        }
    } else if policy.auto_push && status.ahead > 0 && !has_origin {
        eprintln!("ℹ️ skip push for {} (no origin remote)", repo.display());
    }

    Ok(false)
}

async fn run_once(policy_path: &Path) -> Result<()> {
    if let Some(reason) = freeze_reason(policy_path) {
        println!("⏸️ sync frozen ({})", reason);
        return Ok(());
    }

    let policy = SyncPolicy::load(policy_path)?;
    let roots = policy.watch_root_paths();
    let excluded_dir_names = excluded_dir_names_set(&policy);
    let repos = discover_git_repos(&roots, &excluded_dir_names);

    let mut changed = 0usize;
    for repo in repos {
        match tokio::time::timeout(
            Duration::from_secs(REPO_SYNC_TIMEOUT_SECS),
            sync_repo(&repo, &policy, &excluded_dir_names),
        )
        .await
        {
            Err(_) => {
                eprintln!(
                    "⚠️ repo sync timeout for {} after {}s",
                    repo.display(),
                    REPO_SYNC_TIMEOUT_SECS
                );
            }
            Ok(Ok(true)) => {
                changed += 1;
                println!("🔁 synced {}", repo.display());
            }
            Ok(Ok(false)) => {}
            Ok(Err(e)) => eprintln!("⚠️ sync failed for {}: {}", repo.display(), e),
        }
    }

    println!("✅ sync pass complete (repos changed: {})", changed);
    Ok(())
}

async fn run_daemon(policy_path: PathBuf) -> Result<()> {
    loop {
        let interval = SyncPolicy::load(&policy_path)
            .map(|p| p.pulse_interval_secs.max(5))
            .unwrap_or(300);

        if let Some(reason) = freeze_reason(&policy_path) {
            println!("⏸️ sync daemon paused ({})", reason);
        } else if let Err(e) = run_once(&policy_path).await {
            eprintln!("⚠️ sync pass failed: {}", e);
        }

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
            let roots = policy.watch_root_paths();
            let excluded_dir_names = excluded_dir_names_set(&policy);
            let repos = discover_git_repos(&roots, &excluded_dir_names);
            let freeze = freeze_reason(&policy_path);
            println!("📜 POLICY: {}", policy_path.display());
            println!("🔁 ROOTS: {:?}", roots);
            println!("📦 REPOS_DISCOVERED: {}", repos.len());
            println!("⏱️ PULSE: {}s", policy.pulse_interval_secs);
            println!(
                "⏸️ FREEZE: {}",
                freeze
                    .map(|r| format!("ON ({})", r))
                    .unwrap_or_else(|| "OFF".to_string())
            );
            println!(
                "⚙️ FLAGS: auto_commit={} auto_pull={} auto_push={}",
                policy.auto_commit, policy.auto_pull, policy.auto_push
            );
            println!(
                "📏 MAX_STAGE_FILE_BYTES: {}",
                policy.max_stage_file_bytes
            );
            println!("🚫 EXCLUDE_DIRS: {:?}", policy.exclude_dir_names);
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
            if let Some(reason) = freeze_reason(&policy_path) {
                println!("⏸️ sync frozen ({})", reason);
                return Ok(());
            }
            let policy = SyncPolicy::load(&policy_path)?;
            let excluded_dir_names = excluded_dir_names_set(&policy);
            if sync_repo(&repo, &policy, &excluded_dir_names).await? {
                println!("🔁 synced {}", repo.display());
            } else {
                println!("✅ no sync changes {}", repo.display());
            }
        }
    }

    Ok(())
}
