// This file is a temporary shim during git.rs modularization.
// All actual code lives in git/mod.rs and its submodules.
// DO NOT add new code here — add it to the appropriate submodule.

mod branch_ops;
mod diff;
mod discovery;
mod orphan;
mod push;
mod remotes;
mod safety;

use crate::policy::{std_git_command, tokio_git_command, AuthType, RemoteConfig, timestamp_secs};
use anyhow::{Context, Result};
#[allow(dead_code)]
use dracon_git::{
    types::{DiffFile, FileStatus},
    GitService,
};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::sleep;

pub(crate) fn git_ssh_hardening() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    format!(
        "ssh -o BatchMode=yes -F {home}/.dracon/secrets/ssh/config -o ConnectTimeout=10 -o ConnectionAttempts=1 -o ServerAliveInterval=5 -o ServerAliveCountMax=2"
    )
}

#[cfg(test)]
pub(crate) static PATH_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

#[cfg(test)]
#[allow(dead_code)]
fn real_git_path() -> PathBuf {
    if let Ok(custom) = std::env::var("DRACON_SYNC_GIT_BIN") {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    static REAL_GIT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    REAL_GIT.get_or_init(|| {
        for candidate in ["/run/current-system/sw/bin/git", "/usr/bin/git", "/bin/git"] {
            let path = PathBuf::from(candidate);
            if path.exists() {
                return path;
            }
        }
        PathBuf::from("git")
    }).clone()
}

pub(crate) async fn git_diff_head_files(repo: &Path) -> Result<HashSet<PathBuf>> {
    diff::git_diff_head_files(repo).await
}

pub(crate) fn discover_git_repos(
    roots: &[PathBuf],
    excluded_dir_names: &BTreeSet<String>,
    exclude_repos: &[String],
    system_repo: Option<&str>,
) -> Vec<PathBuf> {
    discovery::discover_git_repos(roots, excluded_dir_names, exclude_repos, system_repo)
}

pub(crate) fn has_origin_remote(repo: &Path) -> bool { safety::has_origin_remote(repo) }
pub(crate) fn has_tracking_upstream(repo: &Path) -> bool { safety::has_tracking_upstream(repo) }
pub(crate) fn is_rebase_in_progress(repo: &Path) -> bool { safety::is_rebase_in_progress(repo) }
pub(crate) fn is_merge_in_progress(repo: &Path) -> bool { safety::is_merge_in_progress(repo) }
pub(crate) fn is_cherry_pick_in_progress(repo: &Path) -> bool { safety::is_cherry_pick_in_progress(repo) }
pub(crate) fn is_safe_git_path(path: &Path) -> bool { safety::is_safe_git_path(path) }
pub(crate) fn is_safe_branch_name(branch: &str) -> bool { safety::is_safe_branch_name(branch) }
pub(crate) fn is_git_worktree_file(dot_git: &Path) -> bool { safety::is_git_worktree_file(dot_git) }

pub(crate) async fn kill_descendants(pid: u32) {
    let pid_s = pid.to_string();
    async fn kill_group(pid_s: &str, signal: &str) {
        if let Ok(output) = TokioCommand::new("pkill")
            .args([signal, "-P", pid_s])
            .output()
            .await
        {
            if output.status.success() { return; }
        }
        let _ = TokioCommand::new("kill")
            .args(["-".to_string() + signal, "--".to_string(), "-".to_string() + pid_s])
            .output()
            .await;
    }
    kill_group(&pid_s, "TERM").await;
    sleep(Duration::from_secs(2)).await;
    kill_group(&pid_s, "KILL").await;
}

pub(crate) async fn run_child(
    program: &str,
    args: &[&str],
    kill_delay: Duration,
) -> std::process::ExitStatus {
    let child = TokioCommand::new(program)
        .args(args)
        .kill_on_drop(true)
        .spawn()
        .expect("failed to spawn child");
    let pid = child.id().unwrap_or(0);
    let code = child.code().await;
    if let Some(c) = code {
        std::process::ExitStatus::from_raw(c)
    } else if pid > 0 {
        kill_descendants(pid).await;
        std::process::ExitStatus::from_raw(1)
    } else {
        std::process::ExitStatus::from_raw(1)
    }
}

pub(crate) async fn run_git_with_timeout(
    repo: &Path,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
) -> Result<()> {
    tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        tokio_git_command()
            .args(args)
            .current_dir(repo)
            .output()
            .await
            .with_context(|| format!("git {} timed out after {}s in {}", op_label, timeout_secs, repo.display())),
    )
    .await
    .with_context(|| format!("git {} timed out after {}s", op_label, timeout_secs))?
    .with_context(|| format!("git {} failed in {}", op_label, repo.display()))?;
    Ok(())
}

pub(crate) async fn run_git_with_timeout_env(
    repo: &Path,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
    env_vars: &[(String, String)],
) -> Result<std::process::ExitStatus> {
    let mut cmd = tokio_git_command();
    cmd.args(args).current_dir(repo);
    for (k, v) in env_vars {
        cmd.env(k, v);
    }
    tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        cmd.output(),
    )
    .await
    .with_context(|| format!("git {} timed out after {}s", op_label, timeout_secs))?
    .map(|o| o.status)
    .with_context(|| format!("git {} failed in {}", op_label, repo.display()))
}

async fn git_askpass_script(token: &str) -> Result<PathBuf> {
    let script = format!("#!/bin/bash\necho '{}'\n", token);
    let tmp = std::env::temp_dir().join(format!("dracon-askpass-{}", std::process::id()));
    tokio::fs::write(&tmp, script).await?;
    let mut perms = std::fs::metadata(&tmp)?.permissions();
    perms.set_mode(0o700);
    std::fs::set_permissions(&tmp, perms)?;
    Ok(tmp)
}

async fn git_askpass_script(_token: &str) -> Result<PathBuf> {
    let tmp = std::env::temp_dir().join(format!("dracon-askpass-{}", std::process::id()));
    tokio::fs::write(&tmp, "#!/bin/bash\necho ''\n").await?;
    let mut perms = std::fs::metadata(&tmp)?.permissions();
    perms.set_mode(0o700);
    std::fs::set_permissions(&tmp, perms)?;
    Ok(tmp)
}

pub(crate) fn origin_url(repo: &Path) -> Option<String> { remotes::get_remote_url(repo, "origin") }
pub(crate) fn strip_url_credentials(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            let port = parsed.port().map(|p| format!(":{}", p)).unwrap_or_default();
            let path = parsed.path();
            return format!("{}://{}{}{}", parsed.scheme(), host, port, path);
        }
    }
    url.to_string()
}
pub(crate) fn github_https_url(origin: &str) -> Option<String> {
    if !origin.contains("github.com") { return None; }
    let stripped = strip_url_credentials(origin);
    if stripped.starts_with("git@") {
        Some(stripped.replace("git@github.com:", "https://github.com/"))
    } else { Some(stripped) }
}
pub(crate) fn gitlab_https_url(origin: &str) -> Option<String> {
    if !origin.contains("gitlab") { return None; }
    Some(strip_url_credentials(origin))
}
pub(crate) fn codeberg_https_url(origin: &str) -> Option<String> {
    if !origin.contains("codeberg") { return None; }
    Some(strip_url_credentials(origin))
}

pub(crate) async fn push_with_transport_fallbacks(
    repo: &Path,
    remote: &str,
    branch: &str,
    timeout_secs: u64,
) -> Result<()> {
    push::push_with_transport_fallbacks(repo, remote, branch, timeout_secs).await
}

pub(crate) async fn push_with_retries(
    repo: &Path,
    timeout_secs: u64,
    retries: u32,
    op_label: &str,
) -> Result<()> {
    push::push_with_retries(repo, timeout_secs, retries, op_label).await
}

pub(crate) fn run_git_capture_output(repo: &Path, args: &[&str], op_label: &str) -> Result<String> {
    diff::run_git_capture_output(repo, args, op_label)
}

pub(crate) async fn git_list_paths(repo: &Path, args: &[&str]) -> Result<Vec<PathBuf>> {
    diff::git_list_paths(repo, args).await
}

pub(crate) fn parse_name_status_line(line: &str) -> Option<(PathBuf, FileStatus)> {
    diff::parse_name_status_line(line)
}

pub(crate) async fn git_name_status_entries(repo: &Path, args: &[&str]) -> Result<Vec<(PathBuf, FileStatus)>> {
    diff::git_name_status_entries(repo, args).await
}

pub(crate) fn fallback_status_rank(status: &FileStatus) -> u8 {
    diff::fallback_status_rank(status)
}

pub(crate) async fn cli_diff_entries(repo: &Path) -> Result<Vec<DiffFile>> {
    diff::cli_diff_entries(repo).await
}

pub(crate) async fn repo_diff_entries(repo: &Path) -> Result<Vec<DiffFile>> {
    diff::repo_diff_entries(repo).await
}

pub(crate) async fn staged_paths(repo: &Path) -> Result<Vec<PathBuf>> {
    diff::staged_paths(repo).await
}

pub(crate) async fn unstage_excluded_paths(repo: &Path, excluded_dir_names: &BTreeSet<String>) -> Result<usize> {
    diff::unstage_excluded_paths(repo, excluded_dir_names).await
}

pub(crate) async fn unstage_oversized_paths(repo: &Path, max_stage_file_bytes: u64) -> Result<usize> {
    diff::unstage_oversized_paths(repo, max_stage_file_bytes).await
}

pub(crate) fn current_branch(repo: &Path) -> Option<String> {
    if let Ok(content) = std::fs::read_to_string(repo.join(".git").join("HEAD")) {
        if let Some(ref_name) = content.trim().strip_prefix("ref: refs/heads/") {
            return Some(ref_name.to_string());
        }
    }
    std_git_command()
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None })
        .filter(|s| !s.is_empty())
}

pub(crate) fn has_only_master_branch(repo: &Path) -> bool {
    branch_ops::has_only_master_branch_impl(repo)
}

pub(crate) fn has_both_main_and_master(repo: &Path) -> bool {
    branch_ops::has_both_main_and_master_impl(repo)
}

pub(crate) async fn consolidate_to_main(repo: &Path) -> Result<()> {
    branch_ops::consolidate_to_main(repo).await
}

pub(crate) async fn rename_master_to_main(repo: &Path) -> Result<()> {
    branch_ops::rename_master_to_main(repo).await
}

pub(crate) async fn prune_other_default_branch(repo: &Path) {
    branch_ops::prune_other_default_branch(repo).await
}

pub(crate) fn remote_branch_exists(repo: &Path, branch: &str) -> bool {
    if !is_safe_branch_name(branch) { return false; }
    std_git_command()
        .args(["ls-remote", "--heads", "--quiet", "origin", branch])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub(crate) fn set_upstream_to_branch(repo: &Path, branch: &str) -> Result<()> {
    if !is_safe_branch_name(branch) { return Err(anyhow::anyhow!("branch name '{}' is unsafe", branch)); }
    let status = std_git_command()
        .args(["branch", "--set-upstream-to=origin/".to_string() + branch, branch])
        .current_dir(repo)
        .status()
        .with_context(|| format!("failed to set upstream for {}", repo.display()))?;
    if status.success() { Ok(()) } else { Err(anyhow::anyhow!("set-upstream failed for {} -> origin/{}", repo.display(), branch)) }
}

pub(crate) async fn detect_large_blobs_ahead(repo: &Path, min_bytes: u64) -> Result<Vec<(u64, String)>> {
    branch_ops::detect_large_blobs_ahead(repo, min_bytes).await
}

pub(crate) fn top_level_dir(path: &str) -> Option<String> {
    path.split('/').next().map(|s| s.to_string())
}

pub(crate) fn rewrite_ahead_paths(
    repo: &Path,
    paths_to_remove: &[String],
    backup_prefix: &str,
) -> Result<Option<String>> {
    branch_ops::rewrite_ahead_paths(repo, paths_to_remove, backup_prefix)
}

pub(crate) async fn restore_paths(repo: &Path, paths: &[String]) -> Result<()> {
    diff::restore_paths(repo, paths).await
}

#[allow(dead_code)]
pub(crate) fn load_secret(env_name: &str) -> Option<String> {
    crate::secrets::load_secret(env_name)
}

pub(crate) fn configure_all_remotes(repo: &Path, remotes: &[RemoteConfig], repo_name: &str) {
    remotes::configure_all_remotes(repo, remotes, repo_name)
}

pub(crate) async fn push_mirror_remotes(repo: &Path, remotes: &[RemoteConfig]) -> Vec<(String, Result<()>)> {
    remotes::push_mirror_remotes(repo, remotes).await
}

pub(crate) fn get_remote_url(repo: &Path, name: &str) -> Option<String> {
    remotes::get_remote_url(repo, name)
}

pub(crate) fn list_remotes(repo: &Path) -> Vec<String> {
    remotes::list_remotes(repo)
}

pub(crate) fn remove_stale_remotes(repo: &Path, keep: &[&str]) -> Result<()> {
    remotes::remove_stale_remotes(repo, keep)
}

pub(crate) async fn push_to_named_remote(
    repo: &Path,
    remote_name: &str,
    branch: &str,
    force_when_behind: bool,
    timeout_secs: u64,
) -> Result<()> {
    remotes::push_to_named_remote(repo, remote_name, branch, force_when_behind, timeout_secs).await
}

#[derive(Debug, Clone, Copy)]
pub enum Divergence {
    Ahead,
    Behind,
    Diverged,
    UpToDate,
}

pub(crate) async fn diagnose_divergence(repo: &Path, remote_name: &str, branch: &str) -> Result<Divergence> {
    remotes::diagnose_divergence(repo, remote_name, branch).await
}

pub(crate) async fn push_to_all_remotes(
    repo: &Path,
    remotes: &[RemoteConfig],
    branch: &str,
    timeout_secs: u64,
) -> Vec<(String, Result<()>)> {
    remotes::push_to_all_remotes(repo, remotes, branch, timeout_secs).await
}

pub(crate) fn create_repo_on_github(account: &str, repo_name: &str) -> Result<String> {
    remotes::create_repo_on_github(account, repo_name)
}

pub(crate) fn create_repo_on_gitlab(account: &str, repo_name: &str, private: bool) -> Result<String> {
    remotes::create_repo_on_gitlab(account, repo_name, private)
}

pub(crate) async fn create_repo_on_codeberg(token: &str, account: &str, repo_name: &str, api_endpoint: &str, private: bool) -> Result<String> {
    remotes::create_repo_on_codeberg(token, account, repo_name, api_endpoint, private).await
}

pub(crate) async fn auto_create_repo(config: &RemoteConfig, repo_name: &str, private: bool) -> Result<String> {
    remotes::auto_create_repo(config, repo_name, private).await
}

pub(crate) async fn auto_create_all_remotes(remotes: &[RemoteConfig], repo_name: &str, private: bool) -> Vec<(String, Result<String>)> {
    remotes::auto_create_all_remotes(remotes, repo_name, private).await
}

pub(crate) fn detect_orphan_origin(repo: &Path) -> Option<(String, String)> {
    orphan::detect_orphan_origin(repo)
}

pub(crate) fn fix_orphan_origin(repo: &Path, canonical_url: &str) -> Result<()> {
    orphan::fix_orphan_origin(repo, canonical_url)
}

#[cfg(test)]
pub(crate) fn acquire_path_lock() -> parking_lot::MutexGuard<'static, ()> {
    PATH_LOCK.lock()
}