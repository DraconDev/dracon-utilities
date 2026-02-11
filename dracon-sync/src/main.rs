use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dracon_git::{build_sync_commit_payload, GitService};
use dracon_protocols::git::{
    DiffFile as ProtoDiffFile, FileStatus as ProtoFileStatus, RepoStatus as ProtoRepoStatus,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::process::Command as TokioCommand;
use tokio::time::{sleep, Duration};

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
    /// One-off report across discovered repositories.
    Repos {
        /// Show only concern repos.
        #[arg(long)]
        only_concern: bool,
        /// Show only warn repos.
        #[arg(long, conflicts_with = "only_concern")]
        only_warn: bool,
    },
    /// Repair concern repos (dry-run by default; use --apply to execute).
    RepairConcerns {
        /// Execute git operations to repair concerns.
        #[arg(long)]
        apply: bool,
        /// Only repair this repository path.
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Override push timeout seconds for this run.
        #[arg(long)]
        push_timeout_secs: Option<u64>,
        /// Retry count for push operations.
        #[arg(long, default_value_t = 3)]
        push_retries: u32,
        /// Allow rewrite of large blobs even when paths are outside excluded dirs.
        #[arg(long)]
        rewrite_large_any: bool,
    },
    /// Repair warn repos (dirty-only triage; dry-run by default).
    RepairWarns {
        /// Execute git operations to repair warns.
        #[arg(long)]
        apply: bool,
        /// Only repair this repository path.
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Run one sync pass.
    Once,
    /// Run continuous sync loop.
    Daemon,
    /// Sync a specific repository now.
    SyncNow { repo: PathBuf },
    /// Open sync policy in the system editor.
    EditConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct SyncPolicy {
    #[serde(default)]
    system_repo: String,
    #[serde(default = "default_pulse_interval")]
    pulse_interval_secs: u64,
    #[serde(default = "default_inactivity_push_delay_secs")]
    inactivity_push_delay_secs: u64,
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
    #[serde(default = "default_pull_op_timeout_secs")]
    pull_op_timeout_secs: u64,
    #[serde(default = "default_push_op_timeout_secs")]
    push_op_timeout_secs: u64,
    #[serde(default = "default_repo_sync_timeout_secs")]
    repo_sync_timeout_secs: u64,
    #[serde(default = "default_true")]
    auto_repair_concerns: bool,
    #[serde(default)]
    auto_rewrite_large_blobs: bool,
    #[serde(default = "default_push_retries")]
    push_retries: u32,
}

fn default_true() -> bool {
    true
}

fn default_pulse_interval() -> u64 {
    1
}

fn default_inactivity_push_delay_secs() -> u64 {
    5
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

fn default_pull_op_timeout_secs() -> u64 {
    30
}

fn default_push_op_timeout_secs() -> u64 {
    300
}

fn default_repo_sync_timeout_secs() -> u64 {
    420
}

fn default_push_retries() -> u32 {
    3
}

const DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepoFilter {
    All,
    Concern,
    Warn,
}

#[derive(Debug, Serialize)]
struct IncidentRecord {
    ts_unix: u64,
    scope: String,
    repo: String,
    reason: String,
    action: String,
    backup_branch: Option<String>,
    result: String,
    details: Option<String>,
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
        if policy.pull_op_timeout_secs == 0 {
            policy.pull_op_timeout_secs = default_pull_op_timeout_secs();
        }
        if policy.push_op_timeout_secs == 0 {
            policy.push_op_timeout_secs = default_push_op_timeout_secs();
        }
        if policy.repo_sync_timeout_secs == 0 {
            policy.repo_sync_timeout_secs = default_repo_sync_timeout_secs();
        }
        if policy.push_retries == 0 {
            policy.push_retries = default_push_retries();
        }
        if policy.inactivity_push_delay_secs == 0 {
            policy.inactivity_push_delay_secs = default_inactivity_push_delay_secs();
        }
        policy.pull_op_timeout_secs = policy.pull_op_timeout_secs.max(5);
        policy.push_op_timeout_secs = policy.push_op_timeout_secs.max(10);
        policy.repo_sync_timeout_secs = policy.repo_sync_timeout_secs.max(
            policy
                .push_op_timeout_secs
                .saturating_add(30)
                .max(policy.pull_op_timeout_secs.saturating_add(30)),
        );
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

async fn run_git_with_timeout_env(
    repo: &Path,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
    env: &[(&str, &str)],
) -> Result<()> {
    let mut cmd = TokioCommand::new("git");
    cmd.args(args)
        .current_dir(repo)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn git {} in {}", op_label, repo.display()))?;

    let pid = child.id();
    match tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait()).await {
        Ok(Ok(status)) => {
            if status.success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "git {} failed in {} with status {}",
                    op_label,
                    repo.display(),
                    status
                ))
            }
        }
        Ok(Err(e)) => Err(anyhow::anyhow!(
            "git {} failed in {}: {}",
            op_label,
            repo.display(),
            e
        )),
        Err(_) => {
            if let Some(pid) = pid {
                kill_descendants(pid).await;
            }
            let _ = child.start_kill();
            let _ = child.wait().await;
            Err(anyhow::anyhow!(
                "git {} timeout in {} after {}s",
                op_label,
                repo.display(),
                timeout_secs
            ))
        }
    }
}

fn origin_url(repo: &Path) -> Option<String> {
    let out = StdCommand::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if url.is_empty() {
        None
    } else {
        Some(url)
    }
}

fn github_https_url(origin: &str) -> Option<String> {
    if let Some(rest) = origin.strip_prefix("git@github.com:") {
        return Some(format!("https://github.com/{}", rest));
    }
    if let Some(rest) = origin.strip_prefix("ssh://git@github.com/") {
        return Some(format!("https://github.com/{}", rest));
    }
    if origin.starts_with("https://github.com/") {
        return Some(origin.to_string());
    }
    None
}

async fn push_with_transport_fallbacks(
    repo: &Path,
    timeout_secs: u64,
    op_label: &str,
) -> Result<()> {
    let ssh_hardening = "ssh -o ConnectTimeout=10 -o ConnectionAttempts=1 -o ServerAliveInterval=5 -o ServerAliveCountMax=2";
    match run_git_with_timeout_env(
        repo,
        &["push", "origin", "HEAD"],
        timeout_secs,
        &format!("{op_label}-ssh-hardened"),
        &[("GIT_SSH_COMMAND", ssh_hardening)],
    )
    .await
    {
        Ok(()) => return Ok(()),
        Err(e) => {
            let origin = origin_url(repo).unwrap_or_default();
            if let Some(https) = github_https_url(&origin) {
                let branch = current_branch(repo).unwrap_or_else(|| "master".to_string());
                let refspec = format!("HEAD:refs/heads/{branch}");
                run_git_with_timeout(
                    repo,
                    &["push", &https, &refspec],
                    timeout_secs,
                    &format!("{op_label}-https-fallback"),
                )
                .await
                .with_context(|| format!("ssh fallback failed first: {}", e))
            } else {
                Err(e)
            }
        }
    }
}

async fn push_with_retries(
    repo: &Path,
    timeout_secs: u64,
    retries: u32,
    op_label: &str,
) -> Result<()> {
    let attempts = retries.max(1);
    let mut last_err: Option<anyhow::Error> = None;
    let mut timeout_seen = false;
    for attempt in 1..=attempts {
        match run_git_with_timeout(repo, &["push", "origin", "HEAD"], timeout_secs, op_label).await
        {
            Ok(()) => return Ok(()),
            Err(e) => {
                let err_text = e.to_string();
                let is_timeout = err_text.contains("timeout");
                timeout_seen |= is_timeout;
                last_err = Some(e);
                if attempt < attempts && is_timeout {
                    let backoff = (attempt as u64).min(5);
                    eprintln!(
                        "⏱️ push retry {}/{} for {} after {}s",
                        attempt + 1,
                        attempts,
                        repo.display(),
                        backoff
                    );
                    sleep(Duration::from_secs(backoff)).await;
                    continue;
                }
                break;
            }
        }
    }
    if timeout_seen {
        if let Ok(()) = push_with_transport_fallbacks(repo, timeout_secs, op_label).await {
            return Ok(());
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("push failed")))
}

fn run_git_capture_output(repo: &Path, args: &[&str], op_label: &str) -> Result<String> {
    let output = StdCommand::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed to run git {} in {}", op_label, repo.display()))?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(text)
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

async fn staged_paths(repo: &Path) -> Result<Vec<PathBuf>> {
    git_list_paths(repo, &["diff", "--cached", "--name-only"]).await
}

async fn unstage_excluded_paths(
    repo: &Path,
    excluded_dir_names: &BTreeSet<String>,
) -> Result<usize> {
    let staged = staged_paths(repo).await?;
    let mut removed = 0usize;
    for path in staged {
        if !is_excluded_change_path(&path, excluded_dir_names) {
            continue;
        }
        let status = TokioCommand::new("git")
            .args(["reset", "-q", "HEAD", "--"])
            .arg(&path)
            .current_dir(repo)
            .status()
            .await
            .with_context(|| {
                format!("failed to unstage {} in {}", path.display(), repo.display())
            })?;
        if status.success() {
            removed += 1;
        }
    }
    Ok(removed)
}

async fn unstage_oversized_paths(repo: &Path, max_stage_file_bytes: u64) -> Result<usize> {
    let staged = staged_paths(repo).await?;
    let mut removed = 0usize;
    for path in staged {
        let full = repo.join(&path);
        let meta = match std::fs::metadata(&full) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() || meta.len() <= max_stage_file_bytes {
            continue;
        }
        let status = TokioCommand::new("git")
            .args(["reset", "-q", "HEAD", "--"])
            .arg(&path)
            .current_dir(repo)
            .status()
            .await
            .with_context(|| {
                format!(
                    "failed to unstage oversized path {} in {}",
                    path.display(),
                    repo.display()
                )
            })?;
        if status.success() {
            removed += 1;
            eprintln!(
                "🧹 removed oversized staged path {} ({} bytes)",
                full.display(),
                meta.len()
            );
        }
    }
    Ok(removed)
}

fn current_branch(repo: &Path) -> Option<String> {
    StdCommand::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
}

fn remote_branch_exists(repo: &Path, branch: &str) -> bool {
    StdCommand::new("git")
        .args(["show-ref", "--verify", "--quiet"])
        .arg(format!("refs/remotes/origin/{branch}"))
        .current_dir(repo)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn set_upstream_to_branch(repo: &Path, branch: &str) -> Result<()> {
    let target = format!("origin/{branch}");
    let status = StdCommand::new("git")
        .args(["branch", "--set-upstream-to"])
        .arg(&target)
        .arg(branch)
        .current_dir(repo)
        .status()
        .with_context(|| format!("failed to set upstream for {}", repo.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "set-upstream failed for {} -> {}",
            repo.display(),
            target
        ))
    }
}

fn detect_large_blobs_ahead(repo: &Path, min_bytes: u64) -> Result<Vec<(u64, String)>> {
    let script = format!(
        "git rev-list --objects @{{u}}..HEAD | \
         git cat-file --batch-check='%(objectname) %(objecttype) %(objectsize) %(rest)' | \
         awk '$2==\"blob\" && $3>{} {{printf \"%s\\t%s\\n\", $3, $4}}' | sort -nr",
        min_bytes
    );
    let output = StdCommand::new("sh")
        .args(["-lc", &script])
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed large-blob scan in {}", repo.display()))?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    for line in stdout.lines() {
        let mut parts = line.splitn(2, '\t');
        let size = parts
            .next()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);
        let path = parts.next().map(str::trim).unwrap_or_default().to_string();
        if size > 0 && !path.is_empty() {
            out.push((size, path));
        }
    }
    Ok(out)
}

fn top_level_dir(path: &str) -> Option<String> {
    path.split('/').next().map(|s| s.to_string())
}

fn timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn incident_ledger_path(policy_path: &Path) -> PathBuf {
    policy_path
        .parent()
        .map(|d| d.join("dracon-sync-incidents.jsonl"))
        .unwrap_or_else(|| PathBuf::from("dracon-sync-incidents.jsonl"))
}

fn append_incident_record(policy_path: &Path, record: &IncidentRecord) {
    let path = incident_ledger_path(policy_path);
    let line = match serde_json::to_string(record) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("⚠️ incident serialize failed: {}", e);
            return;
        }
    };
    let parent = path.parent().map(Path::to_path_buf);
    if let Some(dir) = parent {
        let _ = std::fs::create_dir_all(dir);
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut file) => {
            use std::io::Write;
            if let Err(e) = writeln!(file, "{}", line) {
                eprintln!("⚠️ incident write failed ({}): {}", path.display(), e);
            }
        }
        Err(e) => eprintln!("⚠️ incident open failed ({}): {}", path.display(), e),
    }
}

fn repo_state_flags(
    status: &dracon_git::types::RepoStatus,
    has_origin: bool,
    has_upstream: bool,
) -> Vec<String> {
    let mut flags = Vec::new();
    if !status.is_clean {
        flags.push("DIRTY".to_string());
    }
    if status.ahead > 0 {
        flags.push(format!("AHEAD:{}", status.ahead));
    }
    if status.behind > 0 {
        flags.push(format!("BEHIND:{}", status.behind));
    }
    if !has_origin {
        flags.push("NO_ORIGIN".to_string());
    }
    if has_origin && !has_upstream {
        flags.push("NO_UPSTREAM".to_string());
    }
    if status.ahead > 0 && has_origin && has_upstream {
        flags.push("STUCK_PUSH".to_string());
    }
    if status.behind > 0 && has_origin && has_upstream {
        flags.push("STUCK_PULL".to_string());
    }
    if flags.is_empty() {
        flags.push("OK".to_string());
    }
    flags
}

fn repo_is_concern(status: &dracon_git::types::RepoStatus, has_origin: bool, has_upstream: bool) -> bool {
    status.ahead > 0 || status.behind > 0 || !has_origin || (has_origin && !has_upstream)
}

fn repo_is_warn(status: &dracon_git::types::RepoStatus, has_origin: bool, has_upstream: bool) -> bool {
    !repo_is_concern(status, has_origin, has_upstream) && !status.is_clean
}

fn repo_hint(flags: &[String], warn: bool, concern: bool) -> String {
    if flags.iter().any(|f| f == "NO_ORIGIN") {
        return "set origin remote".to_string();
    }
    if flags.iter().any(|f| f == "NO_UPSTREAM") {
        return "run repair-concerns --apply (set upstream)".to_string();
    }
    if flags.iter().any(|f| f.starts_with("AHEAD:")) {
        return "run repair-concerns --apply (push or rewrite)".to_string();
    }
    if flags.iter().any(|f| f.starts_with("BEHIND:")) {
        return "run repair-concerns --apply (pull/rebase)".to_string();
    }
    if warn {
        return "run repair-warns --apply".to_string();
    }
    if concern {
        return "run repair-concerns --apply".to_string();
    }
    "healthy".to_string()
}

fn push_large_blob_threshold_bytes(policy: &SyncPolicy) -> u64 {
    policy
        .max_stage_file_bytes
        .min(DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES)
}

fn rewrite_ahead_paths(
    repo: &Path,
    paths_to_remove: &[String],
    backup_prefix: &str,
) -> Result<Option<String>> {
    if paths_to_remove.is_empty() {
        return Ok(None);
    }
    let backup_branch = format!("{backup_prefix}-{}", timestamp_secs());
    let create_backup = StdCommand::new("git")
        .args(["branch", &backup_branch])
        .current_dir(repo)
        .status()
        .with_context(|| format!("failed backup branch in {}", repo.display()))?;
    if !create_backup.success() {
        return Err(anyhow::anyhow!(
            "failed to create backup branch {} in {}",
            backup_branch,
            repo.display()
        ));
    }

    let mut index_filter = String::from("git rm -r --cached --ignore-unmatch");
    for path in paths_to_remove {
        index_filter.push(' ');
        index_filter.push_str(path);
    }

    let rewrite = StdCommand::new("git")
        .args([
            "filter-branch",
            "--force",
            "--index-filter",
            &index_filter,
            "--prune-empty",
            "@{u}..HEAD",
        ])
        .current_dir(repo)
        .status()
        .with_context(|| format!("failed history rewrite in {}", repo.display()))?;
    if !rewrite.success() {
        return Err(anyhow::anyhow!(
            "history rewrite failed in {} (backup: {})",
            repo.display(),
            backup_branch
        ));
    }

    Ok(Some(backup_branch))
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
    let blob_threshold = push_large_blob_threshold_bytes(policy);

    if policy.auto_pull && has_origin && has_upstream {
        match run_git_with_timeout(
            repo,
            &["pull", "--rebase", "--autostash"],
            policy.pull_op_timeout_secs,
            "pull/rebase",
        )
        .await
        {
            Ok(()) => {}
            Err(e) => eprintln!("⚠️ pull/rebase skipped for {}: {}", repo.display(), e),
        }
    } else if policy.auto_pull && !has_origin {
        eprintln!(
            "ℹ️ skip pull/rebase for {} (no origin remote)",
            repo.display()
        );
    } else if policy.auto_pull && has_origin && !has_upstream {
        eprintln!(
            "ℹ️ skip pull/rebase for {} (no tracking upstream on current branch)",
            repo.display()
        );
    }

    let unstaged = unstage_excluded_paths(repo, excluded_dir_names).await?;
    if unstaged > 0 {
        eprintln!(
            "🧹 removed {} staged excluded paths in {}",
            unstaged,
            repo.display()
        );
    }
    let unstaged_oversized = unstage_oversized_paths(repo, policy.max_stage_file_bytes).await?;
    if unstaged_oversized > 0 {
        eprintln!(
            "🧹 removed {} oversized staged paths in {}",
            unstaged_oversized,
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
                let ahead_large = detect_large_blobs_ahead(repo, blob_threshold).unwrap_or_default();
                if !ahead_large.is_empty() {
                    eprintln!(
                        "⚠️ skip push for {}: large blob(s) above {} bytes in ahead range ({} found)",
                        repo.display(),
                        blob_threshold,
                        ahead_large.len()
                    );
                    return Ok(true);
                }
                match run_git_with_timeout(
                    repo,
                    &["push", "origin", "HEAD"],
                    policy.push_op_timeout_secs,
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
        let ahead_large = detect_large_blobs_ahead(repo, blob_threshold).unwrap_or_default();
        if !ahead_large.is_empty() {
            eprintln!(
                "⚠️ skip push for {}: large blob(s) above {} bytes in ahead range ({} found)",
                repo.display(),
                blob_threshold,
                ahead_large.len()
            );
            return Ok(false);
        }
        match run_git_with_timeout(
            repo,
            &["push", "origin", "HEAD"],
            policy.push_op_timeout_secs,
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
            Duration::from_secs(policy.repo_sync_timeout_secs),
            sync_repo(&repo, &policy, &excluded_dir_names),
        )
        .await
        {
            Err(_) => {
                eprintln!(
                    "⚠️ repo sync timeout for {} after {}s",
                    repo.display(),
                    policy.repo_sync_timeout_secs
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
    if policy.auto_repair_concerns {
        if let Err(e) = run_repair_concerns(
            policy_path,
            true,
            None,
            Some(policy.push_op_timeout_secs),
            policy.push_retries,
            policy.auto_rewrite_large_blobs,
        )
        .await
        {
            eprintln!("⚠️ auto-repair concerns failed: {}", e);
        }
    }
    Ok(())
}

async fn run_daemon(policy_path: PathBuf) -> Result<()> {
    #[derive(Debug, Clone)]
    struct RepoActivity {
        fingerprint: String,
        changed_at: Instant,
    }

    let mut activity: HashMap<PathBuf, RepoActivity> = HashMap::new();

    loop {
        let policy = match SyncPolicy::load(&policy_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("⚠️ failed loading policy: {}", e);
                sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        let scan_interval = policy.pulse_interval_secs.max(1);
        let inactivity_delay = Duration::from_secs(policy.inactivity_push_delay_secs.max(1));
        let roots = policy.watch_root_paths();
        let excluded_dir_names = excluded_dir_names_set(&policy);
        let repos = discover_git_repos(&roots, &excluded_dir_names);
        let repo_set: BTreeSet<PathBuf> = repos.iter().cloned().collect();
        activity.retain(|repo, _| repo_set.contains(repo));

        if let Some(reason) = freeze_reason(&policy_path) {
            println!("⏸️ sync daemon paused ({})", reason);
            sleep(Duration::from_secs(scan_interval)).await;
            continue;
        }

        for repo in repos {
            let svc = match GitService::new(&repo) {
                Ok(svc) => svc,
                Err(e) => {
                    eprintln!("⚠️ {} init_failed: {}", repo.display(), e);
                    continue;
                }
            };
            let status = match svc.get_status().await {
                Ok(status) => status,
                Err(e) => {
                    eprintln!("⚠️ {} status_failed: {}", repo.display(), e);
                    continue;
                }
            };
            let has_local_or_pending_work =
                !status.is_clean || status.ahead > 0 || status.behind > 0;
            if !has_local_or_pending_work {
                activity.remove(&repo);
                continue;
            }

            let fingerprint = format!(
                "{}:{}:{}:{}:{}",
                status.branch, status.modified_files, status.staged_files, status.ahead, status.behind
            );
            let now = Instant::now();
            let Some(entry) = activity.get_mut(&repo) else {
                activity.insert(
                    repo.clone(),
                    RepoActivity {
                        fingerprint,
                        changed_at: now,
                    },
                );
                continue;
            };
            if entry.fingerprint != fingerprint {
                entry.fingerprint = fingerprint;
                entry.changed_at = now;
                continue;
            }
            if now.duration_since(entry.changed_at) < inactivity_delay {
                continue;
            }

            match tokio::time::timeout(
                Duration::from_secs(policy.repo_sync_timeout_secs),
                sync_repo(&repo, &policy, &excluded_dir_names),
            )
            .await
            {
                Err(_) => {
                    eprintln!(
                        "⚠️ repo sync timeout for {} after {}s",
                        repo.display(),
                        policy.repo_sync_timeout_secs
                    );
                }
                Ok(Ok(true)) => println!("🔁 synced {}", repo.display()),
                Ok(Ok(false)) => {}
                Ok(Err(e)) => eprintln!("⚠️ sync failed for {}: {}", repo.display(), e),
            }

            if policy.auto_repair_concerns {
                if let Err(e) = run_repair_concerns(
                    &policy_path,
                    true,
                    Some(repo.clone()),
                    Some(policy.push_op_timeout_secs),
                    policy.push_retries,
                    policy.auto_rewrite_large_blobs,
                )
                .await
                {
                    eprintln!("⚠️ auto-repair concerns failed for {}: {}", repo.display(), e);
                }
            }

            activity.remove(&repo);
        }

        sleep(Duration::from_secs(scan_interval)).await;
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let shortened: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", shortened)
}

fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
}

fn paint(value: &str, code: &str) -> String {
    if colors_enabled() {
        format!("\x1b[{}m{}\x1b[0m", code, value)
    } else {
        value.to_string()
    }
}

async fn git_log_field(repo: &Path, format: &str) -> Option<String> {
    let output = TokioCommand::new("git")
        .args(["log", "-1", &format!("--pretty=format:{}", format)])
        .current_dir(repo)
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

async fn git_log_unix_timestamp(repo: &Path) -> Option<i64> {
    git_log_field(repo, "%ct")
        .await
        .and_then(|s| s.parse::<i64>().ok())
}

async fn run_repos_report(policy_path: &Path, filter: RepoFilter) -> Result<()> {
    #[derive(Debug)]
    struct RepoRow {
        repo: PathBuf,
        state_flags: Vec<String>,
        branch: String,
        modified: usize,
        staged: usize,
        ahead: usize,
        behind: usize,
        last_hash: String,
        last_author: String,
        last_when: String,
        last_msg: String,
        last_unix: i64,
        concern: bool,
        warn: bool,
        hint: String,
    }

    let policy = SyncPolicy::load(policy_path)?;
    let roots = policy.watch_root_paths();
    let excluded_dir_names = excluded_dir_names_set(&policy);
    let repos = discover_git_repos(&roots, &excluded_dir_names);
    let mut rows: Vec<RepoRow> = Vec::new();
    let mut init_or_status_failures = 0usize;

    for repo in repos {
        let svc = match GitService::new(&repo) {
            Ok(svc) => svc,
            Err(e) => {
                init_or_status_failures += 1;
                println!(
                    "{} {} | init_failed: {}",
                    paint("❌", "31"),
                    repo.display(),
                    e
                );
                continue;
            }
        };

        let status = match svc.get_status().await {
            Ok(status) => status,
            Err(e) => {
                init_or_status_failures += 1;
                println!(
                    "{} {} | status_failed: {}",
                    paint("❌", "31"),
                    repo.display(),
                    e
                );
                continue;
            }
        };

        let has_origin = has_origin_remote(&repo);
        let has_upstream = has_tracking_upstream(&repo);

        let flags = repo_state_flags(&status, has_origin, has_upstream);

        let last_hash = status
            .last_commit_hash
            .as_deref()
            .map(|h| truncate(h, 12))
            .unwrap_or_else(|| "-".to_string());
        let last_msg = status
            .last_commit_msg
            .as_deref()
            .map(|m| truncate(m, 72))
            .unwrap_or_else(|| "-".to_string());
        let last_author = git_log_field(&repo, "%an")
            .await
            .unwrap_or_else(|| "-".to_string());
        let last_when = git_log_field(&repo, "%ar")
            .await
            .unwrap_or_else(|| "-".to_string());
        let last_unix = git_log_unix_timestamp(&repo).await.unwrap_or(0);

        let concern = repo_is_concern(&status, has_origin, has_upstream);
        let warn = repo_is_warn(&status, has_origin, has_upstream);
        let hint = repo_hint(&flags, warn, concern);

        rows.push(RepoRow {
            repo,
            state_flags: flags,
            branch: status.branch,
            modified: status.modified_files,
            staged: status.staged_files,
            ahead: status.ahead,
            behind: status.behind,
            last_hash,
            last_author,
            last_when,
            last_msg,
            last_unix,
            concern,
            warn,
            hint,
        });
    }

    rows.sort_by(|a, b| b.last_unix.cmp(&a.last_unix));

    let concern_count_all = rows.iter().filter(|r| r.concern).count();
    let warn_count_all = rows.iter().filter(|r| r.warn).count();
    let ok_count_all = rows
        .len()
        .saturating_sub(concern_count_all + warn_count_all);
    match filter {
        RepoFilter::All => {}
        RepoFilter::Concern => rows.retain(|r| r.concern),
        RepoFilter::Warn => rows.retain(|r| r.warn),
    }

    let concern_count = rows.iter().filter(|r| r.concern).count();
    let warn_count = rows.iter().filter(|r| r.warn).count();
    let ok_count = rows.len().saturating_sub(concern_count + warn_count);

    println!("📜 POLICY: {}", policy_path.display());
    match filter {
        RepoFilter::All => {}
        RepoFilter::Concern => {
            println!(
                "📊 FILTER: only concern repos (showing {} of {})",
                rows.len(),
                concern_count_all
            );
        }
        RepoFilter::Warn => {
            println!(
                "📊 FILTER: only warn repos (showing {} of {})",
                rows.len(),
                warn_count_all
            );
        }
    }
    println!(
        "📦 REPOS: {}  {} {}  {} {}  {} {}  ❌ {}{}",
        rows.len(),
        paint("OK", "32"),
        ok_count,
        paint("WARN", "33"),
        warn_count,
        paint("CONCERN", "31"),
        concern_count,
        init_or_status_failures,
        match filter {
            RepoFilter::All => String::new(),
            RepoFilter::Concern | RepoFilter::Warn => format!(
                "  (all: OK {} WARN {} CONCERN {})",
                ok_count_all, warn_count_all, concern_count_all
            ),
        }
    );
    println!("🕒 SORT: last modified (newest first)");
    println!();

    for (idx, row) in rows.iter().enumerate() {
        let severity = if row.concern {
            paint("CONCERN", "31")
        } else if row.warn {
            paint("WARN", "33")
        } else {
            paint("OK", "32")
        };

        println!("{}. [{}] {}", idx + 1, severity, row.repo.display(),);
        println!(
            "   updated={} branch={} state={} modified={} staged={} ahead={} behind={}",
            row.last_when,
            row.branch,
            row.state_flags.join(","),
            row.modified,
            row.staged,
            row.ahead,
            row.behind
        );
        println!(
            "   last={} by {} {}",
            row.last_hash, row.last_author, row.last_msg
        );
        println!("   hint={}", row.hint);
        println!();
    }

    Ok(())
}

async fn run_repair_concerns(
    policy_path: &Path,
    apply: bool,
    only_repo: Option<PathBuf>,
    push_timeout_override: Option<u64>,
    push_retries: u32,
    rewrite_large_any: bool,
) -> Result<()> {
    let policy = SyncPolicy::load(policy_path)?;
    let roots = policy.watch_root_paths();
    let excluded_dir_names = excluded_dir_names_set(&policy);
    let mut repos = discover_git_repos(&roots, &excluded_dir_names);
    if let Some(target_repo) = only_repo {
        repos.retain(|r| r == &target_repo);
        if repos.is_empty() {
            println!(
                "⚠️ target repo not discovered in policy roots: {}",
                target_repo.display()
            );
            return Ok(());
        }
    }
    let push_timeout_secs = push_timeout_override
        .unwrap_or(policy.push_op_timeout_secs)
        .max(10);
    let push_retries = push_retries.max(1);
    let blob_threshold = push_large_blob_threshold_bytes(&policy);

    let mut concerns = 0usize;
    let mut attempted_ops = 0usize;
    let mut succeeded_ops = 0usize;
    let mut manual_only = 0usize;
    let mut resolved = 0usize;

    println!("📜 POLICY: {}", policy_path.display());
    println!(
        "🛠️ MODE: {}",
        if apply {
            "APPLY (mutating)"
        } else {
            "DRY-RUN (no changes)"
        }
    );
    println!(
        "⚙️ PUSH: timeout={}s retries={}",
        push_timeout_secs, push_retries
    );

    for repo in repos {
        let svc = match GitService::new(&repo) {
            Ok(svc) => svc,
            Err(e) => {
                eprintln!("⚠️ {} init_failed: {}", repo.display(), e);
                continue;
            }
        };
        let status = match svc.get_status().await {
            Ok(status) => status,
            Err(e) => {
                eprintln!("⚠️ {} status_failed: {}", repo.display(), e);
                continue;
            }
        };

        let has_origin = has_origin_remote(&repo);
        let mut has_upstream = has_tracking_upstream(&repo);
        let is_concern = repo_is_concern(&status, has_origin, has_upstream);
        if !is_concern {
            continue;
        }
        concerns += 1;
        let flags = repo_state_flags(&status, has_origin, has_upstream);
        let reason = flags.join(",");

        println!(
            "\n🔎 {}  state: ahead={} behind={} clean={} origin={} upstream={}",
            repo.display(),
            status.ahead,
            status.behind,
            status.is_clean,
            has_origin,
            has_upstream
        );

        if !has_origin {
            manual_only += 1;
            println!("   manual: NO_ORIGIN (configure remote before sync can repair)");
            append_incident_record(
                policy_path,
                &IncidentRecord {
                    ts_unix: timestamp_secs(),
                    scope: "concern".to_string(),
                    repo: repo.display().to_string(),
                    reason: reason.clone(),
                    action: "manual_no_origin".to_string(),
                    backup_branch: None,
                    result: "manual".to_string(),
                    details: Some("configure origin remote".to_string()),
                },
            );
            continue;
        }

        if !has_upstream {
            attempted_ops += 1;
            println!("   plan: set upstream via `git push -u origin HEAD`");
            if apply {
                match run_git_with_timeout(
                    &repo,
                    &["push", "-u", "origin", "HEAD"],
                    push_timeout_secs,
                    "push -u",
                )
                .await
                {
                    Ok(()) => {
                        succeeded_ops += 1;
                        has_upstream = true;
                        println!("   ok: upstream configured");
                        append_incident_record(
                            policy_path,
                            &IncidentRecord {
                                ts_unix: timestamp_secs(),
                                scope: "concern".to_string(),
                                repo: repo.display().to_string(),
                                reason: reason.clone(),
                                action: "set_upstream_push_u".to_string(),
                                backup_branch: None,
                                result: "ok".to_string(),
                                details: None,
                            },
                        );
                    }
                    Err(e) => {
                        println!("   fail: upstream configure failed: {}", e);
                        append_incident_record(
                            policy_path,
                            &IncidentRecord {
                                ts_unix: timestamp_secs(),
                                scope: "concern".to_string(),
                                repo: repo.display().to_string(),
                                reason: reason.clone(),
                                action: "set_upstream_push_u".to_string(),
                                backup_branch: None,
                                result: "fail".to_string(),
                                details: Some(e.to_string()),
                            },
                        );
                        continue;
                    }
                }
            }
        }

        if status.behind > 0 && has_upstream {
            attempted_ops += 1;
            println!("   plan: pull --rebase --autostash");
            if apply {
                match run_git_with_timeout(
                    &repo,
                    &["pull", "--rebase", "--autostash"],
                    policy.pull_op_timeout_secs,
                    "pull/rebase",
                )
                .await
                {
                    Ok(()) => {
                        succeeded_ops += 1;
                        println!("   ok: pulled");
                        append_incident_record(
                            policy_path,
                            &IncidentRecord {
                                ts_unix: timestamp_secs(),
                                scope: "concern".to_string(),
                                repo: repo.display().to_string(),
                                reason: reason.clone(),
                                action: "pull_rebase_autostash".to_string(),
                                backup_branch: None,
                                result: "ok".to_string(),
                                details: None,
                            },
                        );
                    }
                    Err(e) => {
                        println!("   fail: pull failed: {}", e);
                        append_incident_record(
                            policy_path,
                            &IncidentRecord {
                                ts_unix: timestamp_secs(),
                                scope: "concern".to_string(),
                                repo: repo.display().to_string(),
                                reason: reason.clone(),
                                action: "pull_rebase_autostash".to_string(),
                                backup_branch: None,
                                result: "fail".to_string(),
                                details: Some(e.to_string()),
                            },
                        );
                    }
                }
            }
        }

        if status.ahead > 0 && has_upstream {
            attempted_ops += 1;
            println!("   plan: push origin HEAD");
            if apply {
                let mut push_ok = false;
                match push_with_retries(&repo, push_timeout_secs, push_retries, "push").await {
                    Ok(()) => {
                        succeeded_ops += 1;
                        push_ok = true;
                        println!("   ok: pushed");
                        append_incident_record(
                            policy_path,
                            &IncidentRecord {
                                ts_unix: timestamp_secs(),
                                scope: "concern".to_string(),
                                repo: repo.display().to_string(),
                                reason: reason.clone(),
                                action: "push_origin_head".to_string(),
                                backup_branch: None,
                                result: "ok".to_string(),
                                details: None,
                            },
                        );
                    }
                    Err(e) => {
                        println!("   fail: push failed: {}", e);

                        let large = detect_large_blobs_ahead(&repo, blob_threshold)
                            .unwrap_or_default();
                        if !large.is_empty() {
                            println!(
                                "   detect: large blobs in ahead range ({} entries)",
                                large.len()
                            );
                            let mut dirs = BTreeSet::new();
                            for (_, path) in &large {
                                if let Some(dir) = top_level_dir(path) {
                                    if is_excluded_dir_name(&dir, &excluded_dir_names) {
                                        dirs.insert(dir);
                                    }
                                }
                            }
                            let dirs: Vec<String> = dirs.into_iter().collect();
                            let rewrite_paths: Vec<String> = if !dirs.is_empty() {
                                dirs
                            } else if rewrite_large_any {
                                let mut unique = BTreeSet::new();
                                for (_, p) in &large {
                                    unique.insert(p.clone());
                                }
                                unique.into_iter().collect()
                            } else {
                                Vec::new()
                            };

                            if rewrite_paths.is_empty() {
                                println!("   manual: large blobs found but not in excluded dirs");
                                append_incident_record(
                                    policy_path,
                                    &IncidentRecord {
                                        ts_unix: timestamp_secs(),
                                        scope: "concern".to_string(),
                                        repo: repo.display().to_string(),
                                        reason: reason.clone(),
                                        action: "large_blob_detected".to_string(),
                                        backup_branch: None,
                                        result: "manual".to_string(),
                                        details: Some(format!(
                                            "threshold={} entries={} rewrite_allowed=false",
                                            blob_threshold,
                                            large.len()
                                        )),
                                    },
                                );
                            } else {
                                println!(
                                    "   plan: rewrite ahead history removing paths {:?}",
                                    rewrite_paths
                                );
                                match rewrite_ahead_paths(
                                    &repo,
                                    &rewrite_paths,
                                    "backup/pre-sync-largeblob-fix",
                                ) {
                                    Ok(Some(backup_branch)) => {
                                        let backup_branch_for_log = backup_branch.clone();
                                        println!(
                                            "   ok: rewrite complete (backup branch: {})",
                                            backup_branch
                                        );
                                        match push_with_retries(
                                            &repo,
                                            push_timeout_secs,
                                            push_retries,
                                            "push-after-rewrite",
                                        )
                                        .await
                                        {
                                            Ok(()) => {
                                                succeeded_ops += 1;
                                                push_ok = true;
                                                println!("   ok: pushed after rewrite");
                                                append_incident_record(
                                                    policy_path,
                                                    &IncidentRecord {
                                                        ts_unix: timestamp_secs(),
                                                        scope: "concern".to_string(),
                                                        repo: repo.display().to_string(),
                                                        reason: reason.clone(),
                                                        action: "rewrite_then_push".to_string(),
                                                        backup_branch: Some(backup_branch_for_log),
                                                        result: "ok".to_string(),
                                                        details: Some(format!(
                                                            "paths={:?}",
                                                            rewrite_paths
                                                        )),
                                                    },
                                                );
                                            }
                                            Err(e2) => {
                                                println!(
                                                    "   fail: push after rewrite failed: {}",
                                                    e2
                                                );
                                                append_incident_record(
                                                    policy_path,
                                                    &IncidentRecord {
                                                        ts_unix: timestamp_secs(),
                                                        scope: "concern".to_string(),
                                                        repo: repo.display().to_string(),
                                                        reason: reason.clone(),
                                                        action: "rewrite_then_push".to_string(),
                                                        backup_branch: Some(backup_branch),
                                                        result: "fail".to_string(),
                                                        details: Some(e2.to_string()),
                                                    },
                                                );
                                            }
                                        }
                                    }
                                    Ok(None) => {}
                                    Err(rewrite_err) => {
                                        println!("   fail: rewrite failed: {}", rewrite_err);
                                        append_incident_record(
                                            policy_path,
                                            &IncidentRecord {
                                                ts_unix: timestamp_secs(),
                                                scope: "concern".to_string(),
                                                repo: repo.display().to_string(),
                                                reason: reason.clone(),
                                                action: "rewrite_large_blob".to_string(),
                                                backup_branch: None,
                                                result: "fail".to_string(),
                                                details: Some(rewrite_err.to_string()),
                                            },
                                        );
                                    }
                                }
                            }
                        } else {
                            let branch = current_branch(&repo).unwrap_or_default();
                            let dry_run = run_git_capture_output(
                                &repo,
                                &["push", "--dry-run", "origin", "HEAD"],
                                "push --dry-run",
                            )
                            .unwrap_or_default();
                            let looks_branch_mismatch =
                                dry_run.to_ascii_lowercase().contains("up-to-date");
                            if looks_branch_mismatch
                                && !branch.is_empty()
                                && remote_branch_exists(&repo, &branch)
                                && has_tracking_upstream(&repo)
                            {
                                println!(
                                    "   plan: align upstream to origin/{} (possible branch mismatch)",
                                    branch
                                );
                                match set_upstream_to_branch(&repo, &branch) {
                                    Ok(()) => {
                                        println!("   ok: upstream realigned");
                                        match push_with_retries(
                                            &repo,
                                            push_timeout_secs,
                                            push_retries,
                                            "push-after-upstream-align",
                                        )
                                        .await
                                        {
                                            Ok(()) => {
                                                succeeded_ops += 1;
                                                push_ok = true;
                                                println!("   ok: pushed after upstream align");
                                                append_incident_record(
                                                    policy_path,
                                                    &IncidentRecord {
                                                        ts_unix: timestamp_secs(),
                                                        scope: "concern".to_string(),
                                                        repo: repo.display().to_string(),
                                                        reason: reason.clone(),
                                                        action: "realign_upstream_then_push".to_string(),
                                                        backup_branch: None,
                                                        result: "ok".to_string(),
                                                        details: Some(format!(
                                                            "branch={}",
                                                            branch
                                                        )),
                                                    },
                                                );
                                            }
                                            Err(e2) => {
                                                println!(
                                                    "   fail: push after upstream align failed: {}",
                                                    e2
                                                );
                                                append_incident_record(
                                                    policy_path,
                                                    &IncidentRecord {
                                                        ts_unix: timestamp_secs(),
                                                        scope: "concern".to_string(),
                                                        repo: repo.display().to_string(),
                                                        reason: reason.clone(),
                                                        action: "realign_upstream_then_push".to_string(),
                                                        backup_branch: None,
                                                        result: "fail".to_string(),
                                                        details: Some(e2.to_string()),
                                                    },
                                                );
                                            }
                                        }
                                    }
                                    Err(set_err) => {
                                        println!("   fail: upstream align failed: {}", set_err)
                                    }
                                }
                            }
                        }
                    }
                }
                if !push_ok {
                    append_incident_record(
                        policy_path,
                        &IncidentRecord {
                            ts_unix: timestamp_secs(),
                            scope: "concern".to_string(),
                            repo: repo.display().to_string(),
                            reason: reason.clone(),
                            action: "push_origin_head".to_string(),
                            backup_branch: None,
                            result: "fail".to_string(),
                            details: Some("push did not clear concern".to_string()),
                        },
                    );
                }
                if push_ok {
                    if let Ok(next_after_push) = svc.get_status().await {
                    if next_after_push.ahead > 0 {
                        let branch = current_branch(&repo).unwrap_or_default();
                        if !branch.is_empty() && remote_branch_exists(&repo, &branch) {
                            println!(
                                "   plan: realign upstream to origin/{} (ahead still > 0 after push)",
                                branch
                            );
                            match set_upstream_to_branch(&repo, &branch) {
                                Ok(()) => println!("   ok: upstream realigned"),
                                Err(e) => println!("   fail: upstream realign failed: {}", e),
                            }
                        }
                    }
                    }
                }
            }
        }

        if apply {
            if let Ok(next) = svc.get_status().await {
                let still_concern = next.ahead > 0
                    || next.behind > 0
                    || !has_origin_remote(&repo)
                    || (has_origin_remote(&repo) && !has_tracking_upstream(&repo));
                if !still_concern {
                    resolved += 1;
                    println!("   resolved: concern cleared");
                    append_incident_record(
                        policy_path,
                        &IncidentRecord {
                            ts_unix: timestamp_secs(),
                            scope: "concern".to_string(),
                            repo: repo.display().to_string(),
                            reason,
                            action: "verify_resolved".to_string(),
                            backup_branch: None,
                            result: "ok".to_string(),
                            details: None,
                        },
                    );
                } else {
                    println!(
                        "   remaining: ahead={} behind={} origin={} upstream={}",
                        next.ahead,
                        next.behind,
                        has_origin_remote(&repo),
                        has_tracking_upstream(&repo)
                    );
                    append_incident_record(
                        policy_path,
                        &IncidentRecord {
                            ts_unix: timestamp_secs(),
                            scope: "concern".to_string(),
                            repo: repo.display().to_string(),
                            reason,
                            action: "verify_resolved".to_string(),
                            backup_branch: None,
                            result: "remaining".to_string(),
                            details: Some(format!(
                                "ahead={} behind={}",
                                next.ahead, next.behind
                            )),
                        },
                    );
                }
            }
        }
    }

    println!("\n✅ concern management summary");
    println!("   concerns_found: {}", concerns);
    println!("   operations_planned: {}", attempted_ops);
    println!("   operations_succeeded: {}", succeeded_ops);
    println!("   manual_only: {}", manual_only);
    if apply {
        println!("   concerns_resolved_now: {}", resolved);
    } else {
        println!("   dry_run: true (rerun with --apply to execute)");
    }
    println!("   ledger: {}", incident_ledger_path(policy_path).display());

    Ok(())
}

async fn run_repair_warns(policy_path: &Path, apply: bool, only_repo: Option<PathBuf>) -> Result<()> {
    let policy = SyncPolicy::load(policy_path)?;
    let roots = policy.watch_root_paths();
    let excluded_dir_names = excluded_dir_names_set(&policy);
    let mut repos = discover_git_repos(&roots, &excluded_dir_names);
    if let Some(target_repo) = only_repo {
        repos.retain(|r| r == &target_repo);
        if repos.is_empty() {
            println!(
                "⚠️ target repo not discovered in policy roots: {}",
                target_repo.display()
            );
            return Ok(());
        }
    }

    let mut warns = 0usize;
    let mut attempted = 0usize;
    let mut succeeded = 0usize;

    println!("📜 POLICY: {}", policy_path.display());
    println!(
        "🧹 WARN MODE: {}",
        if apply {
            "APPLY (mutating)"
        } else {
            "DRY-RUN (no changes)"
        }
    );

    for repo in repos {
        let svc = match GitService::new(&repo) {
            Ok(svc) => svc,
            Err(e) => {
                eprintln!("⚠️ {} init_failed: {}", repo.display(), e);
                continue;
            }
        };
        let status = match svc.get_status().await {
            Ok(status) => status,
            Err(e) => {
                eprintln!("⚠️ {} status_failed: {}", repo.display(), e);
                continue;
            }
        };
        let has_origin = has_origin_remote(&repo);
        let has_upstream = has_tracking_upstream(&repo);
        if !repo_is_warn(&status, has_origin, has_upstream) {
            continue;
        }
        warns += 1;
        let flags = repo_state_flags(&status, has_origin, has_upstream);
        let reason = flags.join(",");
        println!(
            "\n🟡 {}  state={} modified={} staged={}",
            repo.display(),
            reason,
            status.modified_files,
            status.staged_files
        );
        println!("   plan: run normal sync triage (stage/commit/push)");
        if !apply {
            append_incident_record(
                policy_path,
                &IncidentRecord {
                    ts_unix: timestamp_secs(),
                    scope: "warn".to_string(),
                    repo: repo.display().to_string(),
                    reason,
                    action: "dry_run_sync_triage".to_string(),
                    backup_branch: None,
                    result: "planned".to_string(),
                    details: None,
                },
            );
            continue;
        }

        attempted += 1;
        match tokio::time::timeout(
            Duration::from_secs(policy.repo_sync_timeout_secs),
            sync_repo(&repo, &policy, &excluded_dir_names),
        )
        .await
        {
            Err(_) => {
                println!(
                    "   fail: sync timeout after {}s",
                    policy.repo_sync_timeout_secs
                );
                append_incident_record(
                    policy_path,
                    &IncidentRecord {
                        ts_unix: timestamp_secs(),
                        scope: "warn".to_string(),
                        repo: repo.display().to_string(),
                        reason,
                        action: "sync_triage".to_string(),
                        backup_branch: None,
                        result: "fail".to_string(),
                        details: Some(format!(
                            "timeout={}s",
                            policy.repo_sync_timeout_secs
                        )),
                    },
                );
            }
            Ok(Ok(changed)) => {
                succeeded += 1;
                println!("   ok: triage complete changed={}", changed);
                append_incident_record(
                    policy_path,
                    &IncidentRecord {
                        ts_unix: timestamp_secs(),
                        scope: "warn".to_string(),
                        repo: repo.display().to_string(),
                        reason,
                        action: "sync_triage".to_string(),
                        backup_branch: None,
                        result: "ok".to_string(),
                        details: Some(format!("changed={}", changed)),
                    },
                );
            }
            Ok(Err(e)) => {
                println!("   fail: sync triage failed: {}", e);
                append_incident_record(
                    policy_path,
                    &IncidentRecord {
                        ts_unix: timestamp_secs(),
                        scope: "warn".to_string(),
                        repo: repo.display().to_string(),
                        reason,
                        action: "sync_triage".to_string(),
                        backup_branch: None,
                        result: "fail".to_string(),
                        details: Some(e.to_string()),
                    },
                );
            }
        }
    }

    println!("\n✅ warn management summary");
    println!("   warns_found: {}", warns);
    println!("   operations_planned: {}", warns);
    println!("   operations_attempted: {}", attempted);
    println!("   operations_succeeded: {}", succeeded);
    if !apply {
        println!("   dry_run: true (rerun with --apply to execute)");
    }
    println!("   ledger: {}", incident_ledger_path(policy_path).display());
    Ok(())
}

fn open_policy_in_editor(policy_path: &Path) -> Result<()> {
    let mut editors = Vec::new();
    if let Ok(visual) = std::env::var("VISUAL") {
        if !visual.trim().is_empty() {
            editors.push(visual);
        }
    }
    if let Ok(editor) = std::env::var("EDITOR") {
        if !editor.trim().is_empty() {
            editors.push(editor);
        }
    }
    for fallback in ["nvim", "vim", "nano", "vi"] {
        editors.push(fallback.to_string());
    }

    for editor in editors {
        match StdCommand::new(editor.trim()).arg(policy_path).status() {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => {
                return Err(anyhow::anyhow!(
                    "editor exited non-zero ({}). policy: {}",
                    status,
                    policy_path.display()
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "failed to launch editor '{}' for {}: {}",
                    editor,
                    policy_path.display(),
                    e
                ));
            }
        }
    }

    Err(anyhow::anyhow!(
        "no editor available. set VISUAL or EDITOR to open {}",
        policy_path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let unique = format!(
                "{}_{}_{}",
                prefix,
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time")
                    .as_nanos()
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn test_policy() -> SyncPolicy {
        SyncPolicy {
            system_repo: String::new(),
            pulse_interval_secs: 5,
            inactivity_push_delay_secs: 3,
            auto_commit: true,
            auto_pull: true,
            auto_push: true,
            backup_policy: String::new(),
            backup_dir: String::new(),
            watch_roots: vec![],
            extra_remotes: HashMap::new(),
            exclude_dir_names: vec!["target".into(), "node_modules".into()],
            max_stage_file_bytes: 1024,
            pull_op_timeout_secs: 10,
            push_op_timeout_secs: 10,
            repo_sync_timeout_secs: 40,
            auto_repair_concerns: true,
            auto_rewrite_large_blobs: false,
            push_retries: 2,
        }
    }

    fn mk_status(
        is_clean: bool,
        ahead: usize,
        behind: usize,
        modified_files: usize,
        staged_files: usize,
    ) -> dracon_git::types::RepoStatus {
        dracon_git::types::RepoStatus {
            branch: "master".to_string(),
            ahead,
            behind,
            modified_files,
            staged_files,
            is_clean,
            last_commit_msg: None,
            last_commit_hash: None,
        }
    }

    #[test]
    fn defaults_are_stable() {
        assert!(default_true());
        assert_eq!(default_pulse_interval(), 1);
        assert_eq!(default_inactivity_push_delay_secs(), 5);
        assert!(default_exclude_dir_names().contains(&"target".to_string()));
        assert_eq!(default_max_stage_file_bytes(), 100 * 1024 * 1024);
        assert_eq!(default_pull_op_timeout_secs(), 30);
        assert_eq!(default_push_op_timeout_secs(), 300);
        assert_eq!(default_repo_sync_timeout_secs(), 420);
    }

    #[test]
    fn normalized_dir_name_handles_wrapping_and_case() {
        assert_eq!(normalized_dir_name("/Target/"), "target");
        assert_eq!(normalized_dir_name("Node_Modules"), "node_modules");
        assert_eq!(normalized_dir_name(""), "");
    }

    #[test]
    fn excluded_checks_work() {
        let mut p = test_policy();
        p.exclude_dir_names = vec!["Target".into(), "build".into()];
        let set = excluded_dir_names_set(&p);
        assert!(is_excluded_dir_name("target", &set));
        assert!(is_excluded_dir_name("TARGET", &set));
        assert!(is_excluded_change_path(Path::new("a/Build/x.txt"), &set));
        assert!(!is_excluded_change_path(Path::new("a/src/x.txt"), &set));
    }

    #[test]
    fn should_stage_entry_respects_rules() {
        let td = TempDir::new("sync_should_stage");
        let repo = td.path().join("repo");
        std::fs::create_dir_all(&repo).expect("repo");
        let excluded = BTreeSet::from(["target".to_string()]);

        let deleted = dracon_git::types::DiffFile {
            path: PathBuf::from("target/missing.bin"),
            status: dracon_git::types::FileStatus::Deleted,
        };
        assert!(should_stage_entry(&repo, &deleted, &excluded, 10));

        let excluded_file = dracon_git::types::DiffFile {
            path: PathBuf::from("target/file.bin"),
            status: dracon_git::types::FileStatus::Modified,
        };
        assert!(!should_stage_entry(&repo, &excluded_file, &excluded, 10));

        let big_path = repo.join("big.bin");
        std::fs::write(&big_path, vec![1u8; 64]).expect("write big");
        let big = dracon_git::types::DiffFile {
            path: PathBuf::from("big.bin"),
            status: dracon_git::types::FileStatus::Modified,
        };
        assert!(!should_stage_entry(&repo, &big, &BTreeSet::new(), 16));

        let missing = dracon_git::types::DiffFile {
            path: PathBuf::from("gone.bin"),
            status: dracon_git::types::FileStatus::Modified,
        };
        assert!(should_stage_entry(&repo, &missing, &BTreeSet::new(), 16));
    }

    #[test]
    fn freeze_helpers_work() {
        let _guard = env_lock().lock().expect("lock");
        std::env::remove_var("DRACON_SYNC_FREEZE");

        let td = TempDir::new("sync_freeze");
        let policy = td.path().join("dracon-sync.toml");
        std::fs::write(&policy, "").expect("policy");

        let markers = freeze_marker_paths(&policy);
        assert_eq!(markers.len(), 2);
        assert!(freeze_reason(&policy).is_none());

        std::fs::write(markers[0].clone(), "").expect("marker");
        let reason = freeze_reason(&policy).expect("freeze reason");
        assert!(reason.contains("marker"));

        std::env::set_var("DRACON_SYNC_FREEZE", "1");
        assert_eq!(
            freeze_reason(&policy).as_deref(),
            Some("env DRACON_SYNC_FREEZE")
        );
        std::env::remove_var("DRACON_SYNC_FREEZE");
    }

    #[test]
    fn truncate_and_paint_behave() {
        assert_eq!(truncate("short", 10), "short");
        assert!(truncate("very long value", 8).ends_with('…'));
        let _guard = env_lock().lock().expect("lock");
        std::env::set_var("NO_COLOR", "1");
        assert_eq!(paint("x", "31"), "x");
        std::env::remove_var("NO_COLOR");
    }

    #[test]
    fn discover_git_repos_finds_and_excludes() {
        let td = TempDir::new("sync_discover");
        let root = td.path().join("root");
        std::fs::create_dir_all(root.join("repo-a/.git")).expect("repo-a");
        std::fs::create_dir_all(root.join("target/repo-b/.git")).expect("repo-b");
        std::fs::create_dir_all(root.join("nested/repo-c/.git")).expect("repo-c");
        let excluded = BTreeSet::from(["target".to_string()]);

        let repos = discover_git_repos(&[root], &excluded);
        let as_text = repos
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>();
        assert!(as_text.iter().any(|s| s.contains("repo-a")));
        assert!(as_text.iter().any(|s| s.contains("repo-c")));
        assert!(!as_text.iter().any(|s| s.contains("repo-b")));
    }

    #[test]
    fn normalization_scenarios_repeated() {
        for i in 0..240usize {
            let input = if i % 3 == 0 {
                format!("/TaRgEt/{i}/")
            } else if i % 3 == 1 {
                format!("NODE_MODULES/{i}")
            } else {
                format!("build/{i}")
            };
            let out = normalized_dir_name(&input);
            assert_eq!(out, out.to_ascii_lowercase());
            assert!(!out.starts_with('/'));
            assert!(!out.ends_with('/'));
        }
    }

    #[test]
    fn repo_state_classification_paths() {
        let clean = mk_status(true, 0, 0, 0, 0);
        assert!(!repo_is_concern(&clean, true, true));
        assert!(!repo_is_warn(&clean, true, true));

        let dirty = mk_status(false, 0, 0, 3, 1);
        assert!(!repo_is_concern(&dirty, true, true));
        assert!(repo_is_warn(&dirty, true, true));

        let ahead = mk_status(true, 2, 0, 0, 0);
        assert!(repo_is_concern(&ahead, true, true));
        assert!(!repo_is_warn(&ahead, true, true));
    }

    #[test]
    fn repo_state_flags_and_hint_are_consistent() {
        let st = mk_status(false, 7, 0, 2, 1);
        let flags = repo_state_flags(&st, true, true);
        assert!(flags.iter().any(|f| f == "DIRTY"));
        assert!(flags.iter().any(|f| f == "AHEAD:7"));
        assert!(flags.iter().any(|f| f == "STUCK_PUSH"));
        let hint = repo_hint(&flags, false, true);
        assert!(hint.contains("repair-concerns"));
    }

    #[test]
    fn push_blob_threshold_is_guardrailed() {
        let mut p = test_policy();
        p.max_stage_file_bytes = 200 * 1024 * 1024;
        assert_eq!(
            push_large_blob_threshold_bytes(&p),
            DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES
        );
        p.max_stage_file_bytes = 50 * 1024 * 1024;
        assert_eq!(push_large_blob_threshold_bytes(&p), 50 * 1024 * 1024);
    }

    #[test]
    fn incident_ledger_write_roundtrip() {
        let td = TempDir::new("sync_ledger");
        let policy = td.path().join("dracon-sync.toml");
        std::fs::write(&policy, "watch_roots=[]").expect("policy");
        let record = IncidentRecord {
            ts_unix: 1,
            scope: "concern".to_string(),
            repo: "/tmp/repo".to_string(),
            reason: "AHEAD:1".to_string(),
            action: "push_origin_head".to_string(),
            backup_branch: None,
            result: "ok".to_string(),
            details: Some("d".to_string()),
        };
        append_incident_record(&policy, &record);
        let ledger = incident_ledger_path(&policy);
        let body = std::fs::read_to_string(&ledger).expect("ledger");
        assert!(!body.trim().is_empty());
        let first = body.lines().next().expect("line");
        let parsed: Value = serde_json::from_str(first).expect("json");
        assert_eq!(parsed["scope"], "concern");
        assert_eq!(parsed["result"], "ok");
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
                "⏳ INACTIVITY_PUSH_DELAY: {}s",
                policy.inactivity_push_delay_secs
            );
            println!(
                "⏸️ FREEZE: {}",
                freeze
                    .map(|r| format!("ON ({})", r))
                    .unwrap_or_else(|| "OFF".to_string())
            );
            println!(
                "⚙️ FLAGS: auto_commit={} auto_pull={} auto_push={} auto_repair_concerns={} auto_rewrite_large_blobs={}",
                policy.auto_commit,
                policy.auto_pull,
                policy.auto_push,
                policy.auto_repair_concerns,
                policy.auto_rewrite_large_blobs
            );
            println!("📏 MAX_STAGE_FILE_BYTES: {}", policy.max_stage_file_bytes);
            println!(
                "🧱 PUSH_BLOB_THRESHOLD_BYTES: {}",
                push_large_blob_threshold_bytes(&policy)
            );
            println!("🚫 EXCLUDE_DIRS: {:?}", policy.exclude_dir_names);
            println!(
                "⏱️ TIMEOUTS: pull={}s push={}s repo={}s retries={}",
                policy.pull_op_timeout_secs,
                policy.push_op_timeout_secs,
                policy.repo_sync_timeout_secs,
                policy.push_retries
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
        Command::Repos {
            only_concern,
            only_warn,
        } => {
            let filter = if only_concern {
                RepoFilter::Concern
            } else if only_warn {
                RepoFilter::Warn
            } else {
                RepoFilter::All
            };
            run_repos_report(&policy_path, filter).await?;
        }
        Command::RepairConcerns {
            apply,
            repo,
            push_timeout_secs,
            push_retries,
            rewrite_large_any,
        } => {
            run_repair_concerns(
                &policy_path,
                apply,
                repo,
                push_timeout_secs,
                push_retries,
                rewrite_large_any,
            )
            .await?;
        }
        Command::RepairWarns { apply, repo } => {
            run_repair_warns(&policy_path, apply, repo).await?;
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
        Command::EditConfig => {
            open_policy_in_editor(&policy_path)?;
        }
    }

    Ok(())
}
