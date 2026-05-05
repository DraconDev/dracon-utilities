use anyhow::{Context, Result};
#[allow(dead_code)]
use dracon_git::{
    types::{DiffFile, FileStatus},
    GitService,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::sleep;

use crate::exclude::is_excluded_change_path;
use crate::policy::{std_git_command, tokio_git_command, timestamp_secs, AuthType, RemoteConfig};

pub(crate) const GIT_SSH_HARDENING: &str = "ssh -o ConnectTimeout=10 -o ConnectionAttempts=1 -o ServerAliveInterval=5 -o ServerAliveCountMax=2";

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

/// Get the list of files that actually differ from HEAD (filter-aware).
/// Unlike `git status`, `git diff HEAD` applies clean filters and correctly
/// ignores files that only differ due to smudge filter decryption.
pub(crate) async fn git_diff_head_files(repo: &Path) -> Result<Vec<String>> {
    let repo = repo.to_path_buf();
    let outcome = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<String>> {
            let output = std::process::Command::new("git")
                .current_dir(&repo)
                .args(["diff", "HEAD", "--name-only", "-z"])
                .output()?;
            if !output.status.success() {
                anyhow::bail!("git diff HEAD exited with {}", output.status);
            }
            let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .split('\0')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            Ok(files)
        }),
    ).await;
    let inner = match outcome {
        Ok(inner) => inner,
        Err(_) => return Err(anyhow::anyhow!("git diff HEAD timed out")),
    };
    match inner {
        Ok(Ok(files)) => Ok(files),
        Ok(Err(e)) => Err(anyhow::anyhow!("git diff HEAD task failed: {}", e)),
        Err(e) => Err(anyhow::anyhow!("git diff HEAD task failed: {}", e)),
    }
}

pub(crate) fn discover_git_repos(
    roots: &[PathBuf],
    excluded_dir_names: &BTreeSet<String>,
    exclude_repos: &[String],
    system_repo: Option<&str>,
) -> Vec<PathBuf> {
    let exclude_set: std::collections::HashSet<PathBuf> =
        exclude_repos.iter().map(PathBuf::from).collect();
    let mut repos = Vec::new();
    for root in roots {
        discover_git_repos_recursive(root, excluded_dir_names, &mut repos, 0, 4);
    }
    repos.retain(|r| !exclude_set.contains(r));

    // Always include system_repo if it exists and is a git repo
    if let Some(system) = system_repo {
        let system_path = PathBuf::from(system);
        if system_path.exists() && system_path.join(".git").exists()
            && !repos.contains(&system_path) && !exclude_set.contains(&system_path)
        {
            repos.push(system_path);
        }
    }

    repos
}

fn is_git_worktree_file(dot_git: &Path) -> bool {
    std::fs::read_to_string(dot_git)
        .map(|content| content.trim().starts_with("gitdir:"))
        .unwrap_or(false)
}

fn is_safe_git_path(path: &Path) -> bool {
    if path.is_absolute() {
        return false;
    }
    let mut components = path.components();
    if let Some(first) = components.next() {
        if first.as_os_str() == ".." {
            return false;
        }
    }
    if let Some(first) = components.next() {
        if first.as_os_str() == ".." {
            return false;
        }
    }
    if path.to_string_lossy().starts_with('-') {
        return false;
    }
    true
}

fn is_safe_branch_name(branch: &str) -> bool {
    if branch.is_empty() {
        return false;
    }
    if branch.starts_with('-') {
        return false;
    }
    if branch.contains("..") {
        return false;
    }
    if branch.contains('\n') || branch.contains('\r') || branch.contains('\0') {
        return false;
    }
    if branch.ends_with('.') {
        return false;
    }
    if branch.contains('\\') || branch.contains('~') || branch.contains('^') || branch.contains(':') {
        return false;
    }
    if branch.contains('?') || branch.contains('*') || branch.contains('[') {
        return false;
    }
    if branch.contains(' ') {
        return false;
    }
    true
}

fn discover_git_repos_recursive(
    dir: &Path,
    excluded_dir_names: &BTreeSet<String>,
    repos: &mut Vec<PathBuf>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("⚠️ cannot read directory {}: {}", dir.display(), e);
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("⚠️ cannot read entry in {}: {}", dir.display(), e);
                continue;
            }
        };
        let path = entry.path();
        if !path.is_dir() || path.is_symlink() {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if excluded_dir_names.contains(&name) || name == "objects" {
            continue;
        }
        let dot_git = path.join(".git");
        if dot_git.exists() && (dot_git.is_dir() || is_git_worktree_file(&dot_git)) {
            repos.push(path.clone());
        }
        if name.starts_with('.') {
            continue;
        }
        discover_git_repos_recursive(&path, excluded_dir_names, repos, depth + 1, max_depth);
    }
}

pub(crate) fn has_origin_remote(repo: &Path) -> bool {
    let config_path = repo.join(".git").join("config");
    if let Ok(config) = std::fs::read_to_string(&config_path) {
        return config.lines().any(|line| line.trim() == "[remote \"origin\"]");
    }
    std_git_command()
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub(crate) fn has_tracking_upstream(repo: &Path) -> bool {
    let config_path = repo.join(".git").join("config");
    if let Ok(config) = std::fs::read_to_string(&config_path) {
        if let Some(branch) = current_branch(repo) {
            let section = format!("[branch \"{}\"]", branch);
            if let Some(pos) = config.find(&section) {
                let after = &config[pos + section.len()..];
                let next_section = after.find('[').unwrap_or(after.len());
                let branch_config = &after[..next_section];
                return branch_config.contains("remote = ") && branch_config.contains("merge = ");
            }
        }
        return false;
    }
    // Config file not readable (worktree, symlink, etc.) —
    // fall back to git subprocess which handles these cases natively.
    std_git_command()
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub(crate) fn is_rebase_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("rebase-merge").exists()
        || repo.join(".git").join("rebase-apply").exists()
}

pub(crate) fn is_merge_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("MERGE_HEAD").exists()
}

pub(crate) fn is_cherry_pick_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("CHERRY_PICK_HEAD").exists()
}

pub(crate) async fn kill_descendants(pid: u32) {
    let pid_s = pid.to_string();

    async fn kill_group(pid_s: &str, signal: &str) {
        if let Ok(output) = TokioCommand::new("pkill")
            .args([signal, "-P", pid_s])
            .output()
            .await
        {
            if output.status.success() {
                return;
            }
        }
        let _ = TokioCommand::new("kill")
            .args(["-".to_string() + signal, "--".to_string(), "-".to_string() + pid_s])
            .output()
            .await;
    }

    kill_group(&pid_s, "TERM").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    kill_group(&pid_s, "KILL").await;
}

pub(crate) async fn run_child(
    mut child: tokio::process::Child,
    workdir: &Path,
    timeout_secs: u64,
    label: &str,
) -> Result<()> {
    let pid = child.id();
    match tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait()).await {
        Ok(Ok(status)) => {
            if status.success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "{} failed in {} with status {}",
                    label,
                    workdir.display(),
                    status
                ))
            }
        }
        Ok(Err(e)) => Err(anyhow::anyhow!(
            "{} failed in {}: {}",
            label,
            workdir.display(),
            e
        )),
        Err(_) => {
            if let Some(pid) = pid {
                kill_descendants(pid).await;
            }
            let _ = child.start_kill();
            let _ = child.wait().await;
            Err(anyhow::anyhow!(
                "{} timeout in {} after {}s",
                label,
                workdir.display(),
                timeout_secs
            ))
        }
    }
}

pub(crate) async fn run_git_with_timeout(
    repo: &Path,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
) -> Result<()> {
    let label = format!("git {}", op_label);
    let child = tokio_git_command()
        .args(args)
        .current_dir(repo)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {} in {}", label, repo.display()))?;
    run_child(child, repo, timeout_secs, &label).await
}

pub(crate) async fn run_git_with_timeout_env(
    repo: &Path,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
    env: &[(&str, &str)],
) -> Result<()> {
    let label = format!("git {}", op_label);
    let mut cmd = tokio_git_command();
    cmd.args(args)
        .current_dir(repo)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn {} in {}", label, repo.display()))?;
    run_child(child, repo, timeout_secs, &label).await
}

pub(crate) fn origin_url(repo: &Path) -> Option<String> {
    let out = std_git_command()
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

pub(crate) fn strip_url_credentials(url: &str) -> String {
    if let Some(stripped) = url.strip_prefix("https://") {
        if let Some(at_pos) = stripped.find('@') {
            return format!("https://{}", &stripped[at_pos + 1..]);
        }
    }
    url.to_string()
}

pub(crate) fn github_https_url(origin: &str) -> Option<String> {
    if let Some(rest) = origin.strip_prefix("git@github.com:") {
        return Some(format!("https://github.com/{}", rest));
    }
    if let Some(rest) = origin.strip_prefix("ssh://git@github.com/") {
        return Some(format!("https://github.com/{}", rest));
    }
    if origin.starts_with("https://github.com/") {
        return Some(strip_url_credentials(origin));
    }
    None
}

pub(crate) async fn push_with_transport_fallbacks(
    repo: &Path,
    timeout_secs: u64,
    op_label: &str,
) -> Result<()> {
    let ssh_hardening = crate::git::GIT_SSH_HARDENING;
    match run_git_with_timeout_env(
        repo,
        &["push", "origin", "HEAD"],
        timeout_secs,
        &format!("{op_label}-ssh-hardened"),
        &[("GIT_SSH_COMMAND", ssh_hardening)],
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(e) => {
            let origin = origin_url(repo).unwrap_or_default();
            if let Some(https) = github_https_url(&origin) {
                let branch = current_branch(repo).unwrap_or_else(|| "main".to_string());
                if !is_safe_branch_name(&branch) {
                    eprintln!("⚠️ branch name '{}' is unsafe, skipping https fallback", branch);
                    return Err(e);
                }
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

pub(crate) async fn push_with_retries(
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

pub(crate) fn run_git_capture_output(repo: &Path, args: &[&str], op_label: &str) -> Result<String> {
    let output = std_git_command()
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed to run git {} in {}", op_label, repo.display()))?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(text)
}

pub(crate) async fn git_list_paths(repo: &Path, args: &[&str]) -> Result<Vec<PathBuf>> {
    let output = tokio_git_command()
        .args(args)
        .current_dir(repo)
        .output()
        .await
        .with_context(|| format!("failed to run git {:?} in {}", args, repo.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            eprintln!("⚠️ git {:?} failed in {}: {}", args, repo.display(), stderr.trim());
        }
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

pub(crate) fn parse_name_status_line(line: &str) -> Option<(PathBuf, FileStatus)> {
    let mut parts = line.split('\t');
    let status_raw = parts.next()?.trim();
    if status_raw.is_empty() {
        return None;
    }
    let status_char = status_raw.chars().next()?;
    let (path, status) = match status_char {
        'M' => (parts.next()?, FileStatus::Modified),
        'A' => (parts.next()?, FileStatus::Added),
        'D' => (parts.next()?, FileStatus::Deleted),
        'T' => (parts.next()?, FileStatus::TypeChange),
        'R' => {
            let _old = parts.next()?;
            let new = parts.next()?;
            (new, FileStatus::Renamed)
        }
        _ => return None,
    };
    Some((PathBuf::from(path.trim()), status))
}

pub(crate) async fn git_name_status_entries(
    repo: &Path,
    args: &[&str],
) -> Result<Vec<(PathBuf, FileStatus)>> {
    let output = tokio_git_command()
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
        .filter_map(parse_name_status_line)
        .collect::<Vec<_>>())
}

pub(crate) fn fallback_status_rank(status: &FileStatus) -> u8 {
    match status {
        FileStatus::Deleted => 5,
        FileStatus::Renamed => 4,
        FileStatus::TypeChange => 3,
        FileStatus::Added => 2,
        FileStatus::Modified => 1,
        FileStatus::Unknown => 0,
    }
}

pub(crate) async fn cli_diff_entries(repo: &Path) -> Result<Vec<DiffFile>> {
    let mut entries: BTreeMap<PathBuf, FileStatus> = BTreeMap::new();

    for args in [
        &["diff", "--name-status"][..],
        &["diff", "--cached", "--name-status"][..],
    ] {
        for (path, status) in git_name_status_entries(repo, args).await? {
            let should_replace = entries
                .get(&path)
                .map(|old| fallback_status_rank(&status) >= fallback_status_rank(old))
                .unwrap_or(true);
            if should_replace {
                entries.insert(path, status);
            }
        }
    }

    for path in git_list_paths(repo, &["ls-files", "--others", "--exclude-standard"]).await? {
        let should_replace = entries
            .get(&path)
            .map(|old| fallback_status_rank(&FileStatus::Added) >= fallback_status_rank(old))
            .unwrap_or(true);
        if should_replace {
            entries.insert(path, FileStatus::Added);
        }
    }

    Ok(entries
        .into_iter()
        .map(|(path, status)| DiffFile {
            path,
            status,
        })
        .collect())
}

pub(crate) async fn repo_diff_entries(repo: &Path) -> Result<Vec<DiffFile>> {
    let svc = GitService::new(repo)?;
    let mut entries = svc.get_diff_entries().await?;
    if entries.is_empty() {
        let fallback_entries = cli_diff_entries(repo).await?;
        if !fallback_entries.is_empty() {
            entries = fallback_entries;
        }
    }
    Ok(entries)
}

pub(crate) async fn staged_paths(repo: &Path) -> Result<Vec<PathBuf>> {
    git_list_paths(repo, &["diff", "--cached", "--name-only"]).await
}

pub(crate) async fn unstage_excluded_paths(
    repo: &Path,
    excluded_dir_names: &BTreeSet<String>,
) -> Result<usize> {
    let staged = staged_paths(repo).await?;
    let mut to_unstage = Vec::new();
    for path in staged {
        if !is_safe_git_path(&path) {
            eprintln!("⚠️ skipping unsafe path {} in {}", path.display(), repo.display());
            continue;
        }
        if is_excluded_change_path(&path, excluded_dir_names) {
            to_unstage.push(path);
        }
    }
    if to_unstage.is_empty() {
        return Ok(0);
    }

    let removed = to_unstage.len();
    for chunk in to_unstage.chunks(50) {
        let mut cmd = tokio_git_command();
        cmd.args(["reset", "-q", "HEAD", "--"])
            .current_dir(repo)
            .kill_on_drop(true);
        for path in chunk {
            cmd.arg(path);
        }
        let status = cmd.status()
            .await
            .with_context(|| {
                format!("failed to unstage paths in {}", repo.display())
            })?;
        if !status.success() {
            // If one chunk fails, it might be due to a specific path error.
            // Just log it and continue with other chunks.
            eprintln!("⚠️ failed to unstage a chunk of paths in {}", repo.display());
        }
    }
    Ok(removed)
}

pub(crate) async fn unstage_oversized_paths(repo: &Path, max_stage_file_bytes: u64) -> Result<usize> {
    let staged = staged_paths(repo).await?;
    let mut to_unstage = Vec::new();
    for path in staged {
        if !is_safe_git_path(&path) {
            eprintln!("⚠️ skipping unsafe path {} in {}", path.display(), repo.display());
            continue;
        }
        let full = repo.join(&path);
        let meta = match std::fs::metadata(&full) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() || meta.len() <= max_stage_file_bytes {
            continue;
        }
        to_unstage.push(path);
    }
    if to_unstage.is_empty() {
        return Ok(0);
    }

    let removed = to_unstage.len();
    for chunk in to_unstage.chunks(50) {
        let mut cmd = tokio_git_command();
        cmd.args(["reset", "-q", "HEAD", "--"])
            .current_dir(repo)
            .kill_on_drop(true);
        for path in chunk {
            cmd.arg(path);
        }
        let status = cmd.status()
            .await
            .with_context(|| {
                format!("failed to unstage oversized paths in {}", repo.display())
            })?;
        if !status.success() {
            eprintln!("⚠️ failed to unstage a chunk of oversized paths in {}", repo.display());
        }
    }
    Ok(removed)
}

pub(crate) fn current_branch(repo: &Path) -> Option<String> {
    let head_path = repo.join(".git").join("HEAD");
    if let Ok(content) = std::fs::read_to_string(&head_path) {
        let trimmed = content.trim();
        if let Some(ref_name) = trimmed.strip_prefix("ref: refs/heads/") {
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
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
}

pub(crate) fn has_only_master_branch(repo: &Path) -> bool {
    let has_master = std_git_command()
        .args(["rev-parse", "--verify", "refs/heads/master"])
        .current_dir(repo)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !has_master {
        return false;
    }
    let has_main = std_git_command()
        .args(["rev-parse", "--verify", "refs/heads/main"])
        .current_dir(repo)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    has_master && !has_main
}

pub(crate) fn has_both_main_and_master(repo: &Path) -> bool {
    let config_path = repo.join(".git").join("config");
    let has_local_branches = if let Ok(config) = std::fs::read_to_string(&config_path) {
        config.lines().any(|l| l.trim() == "[branch \"main\"]")
            && config.lines().any(|l| l.trim() == "[branch \"master\"]")
    } else {
        false
    };
    if has_local_branches {
        return true;
    }
    // Fallback: check with git (suppress stderr for detached HEAD repos)
    let has_main = std_git_command()
        .args(["rev-parse", "--verify", "refs/heads/main"])
        .current_dir(repo)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let has_master = std_git_command()
        .args(["rev-parse", "--verify", "refs/heads/master"])
        .current_dir(repo)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    has_main && has_master
}

pub(crate) async fn consolidate_to_main(repo: &Path) -> Result<()> {
    let branch = current_branch(repo).unwrap_or_else(|| "main".to_string());
    if branch != "main" {
        std_git_command()
            .args(["checkout", "main"])
            .current_dir(repo)
            .status()
            .with_context(|| format!("failed to checkout main in {}", repo.display()))?;
    }
    if let Err(e) = std_git_command()
        .args(["branch", "-D", "master"])
        .current_dir(repo)
        .status()
    {
        eprintln!("⚠️ failed to delete local master branch: {}", e);
    }
    if let Err(e) = std_git_command()
        .args(["push", "origin", "--delete", "master"])
        .current_dir(repo)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
    {
        eprintln!("⚠️ failed to delete remote master branch: {}", e);
    }
    if has_origin_remote(repo) && !has_tracking_upstream(repo) {
        if let Err(e) =
            push_with_retries(repo, 60, 3, "consolidate-to-main").await
        {
            eprintln!("⚠️ failed to push main with upstream: {}", e);
        }
    }
    Ok(())
}

pub(crate) async fn rename_master_to_main(repo: &Path) -> Result<()> {
    let branch = current_branch(repo).unwrap_or_else(|| "main".to_string());
    if branch == "master" {
        std_git_command()
            .args(["branch", "-m", "master", "main"])
            .current_dir(repo)
            .status()
            .with_context(|| format!("failed to rename master to main in {}", repo.display()))?;
    }
    if has_origin_remote(repo) {
        if let Err(e) =
            push_with_retries(repo, 60, 3, "rename-master-to-main").await
        {
            eprintln!("⚠️ failed to push main to origin: {}", e);
        }
        if let Err(e) = std_git_command()
            .args(["push", "origin", "--delete", "master"])
            .current_dir(repo)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
        {
            eprintln!("⚠️ failed to delete remote master: {}", e);
        }
    }
    Ok(())
}

/// Delete the "other" default branch if it exists, preventing dual-branch drift.
/// If current branch is master → delete main. If current is main → delete master.
pub(crate) async fn prune_other_default_branch(repo: &Path) {
    let branch = current_branch(repo);
    let other = match branch.as_deref() {
        Some("master") => "main",
        Some("main") => "master",
        _ => return,
    };
    let other_str = other.to_string();
    let repo_has_origin = has_origin_remote(repo);
    let repo = repo.to_path_buf();
    let repo_for_second = repo.clone();
    let other_str_inner = other_str.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || {
        std_git_command()
            .args(["branch", "-D", &other_str_inner])
            .current_dir(&repo)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    })
    .await
    {
        eprintln!("⚠️ failed to delete local {} branch: {}", other_str, e);
    }
    if repo_has_origin {
        let other_str_inner = other_str.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || {
            std_git_command()
                .args(["push", "origin", "--delete", &other_str_inner])
                .current_dir(&repo_for_second)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
        })
        .await
        {
            eprintln!("⚠️ failed to delete remote {} branch: {}", other_str, e);
        }
    }
}

pub(crate) fn remote_branch_exists(repo: &Path, branch: &str) -> bool {
    if !is_safe_branch_name(branch) {
        eprintln!("⚠️ branch name '{}' is unsafe, returning false", branch);
        return false;
    }
    std_git_command()
        .args(["show-ref", "--verify", "--quiet"])
        .arg(format!("refs/remotes/origin/{branch}"))
        .current_dir(repo)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) fn set_upstream_to_branch(repo: &Path, branch: &str) -> Result<()> {
    if !is_safe_branch_name(branch) {
        return Err(anyhow::anyhow!("branch name '{}' is unsafe", branch));
    }
    let target = format!("origin/{branch}");
    let status = std_git_command()
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

pub(crate) async fn detect_large_blobs_ahead(repo: &Path, min_bytes: u64) -> Result<Vec<(u64, String)>> {
    let timeout_secs = 60;
    let repo = repo.to_path_buf();
    let repo_display = repo.display().to_string();
    
    tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        tokio::task::spawn_blocking(move || -> Result<Vec<(u64, String)>> {
            let rev_list = std_git_command()
                .args(["rev-list", "--objects", "@{u}..HEAD"])
                .current_dir(&repo)
                .output()
                .with_context(|| format!("failed rev-list in {}", repo.display()))?;
            if !rev_list.status.success() {
                return Ok(Vec::new());
            }
            
            let mut cat_file = std_git_command()
                .args(["cat-file", "--batch-check=%(objectname) %(objecttype) %(objectsize) %(rest)"])
                .current_dir(&repo)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()
                .with_context(|| format!("failed cat-file in {}", repo.display()))?;
            
            if let Some(mut stdin) = cat_file.stdin.take() {
                use std::io::Write;
                stdin.write_all(&rev_list.stdout)?;
            }
            let output = cat_file.wait_with_output()?;
            if !output.status.success() {
                return Ok(Vec::new());
            }
            
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut out: Vec<(u64, String)> = stdout
                .lines()
                .filter_map(|line| {
                    let mut parts = line.split_whitespace();
                    let _oid = parts.next()?;
                    let obj_type = parts.next()?;
                    let size_str = parts.next()?;
                    let path = parts.next()?;
                    if obj_type == "blob" {
                        let size = size_str.parse::<u64>().ok()?;
                        if size > min_bytes {
                            return Some((size, path.to_string()));
                        }
                    }
                    None
                })
                .collect();
            out.sort_by_key(|a| a.0);
            Ok(out)
        }),
    )
    .await
    .with_context(|| "timed out waiting for spawn_blocking in detect_large_blobs_ahead")?
    .with_context(|| format!("detect_large_blobs_ahead timed out (>{}s) for {}", timeout_secs, repo_display))?
}

pub(crate) fn top_level_dir(path: &str) -> Option<String> {
    path.split('/').next().map(|s| s.to_string())
}

pub(crate) fn rewrite_ahead_paths(
    repo: &Path,
    paths_to_remove: &[String],
    backup_prefix: &str,
) -> Result<Option<String>> {
    if paths_to_remove.is_empty() {
        return Ok(None);
    }
    let backup_branch = format!("{backup_prefix}-{}", timestamp_secs());
    let create_backup = std_git_command()
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

    // Try git-filter-repo first (preferred, faster, actively maintained)
    let filter_repo_available = std_git_command()
        .args(["filter-repo", "--version"])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if filter_repo_available {
        let mut args: Vec<String> = vec![
            "filter-repo".to_string(),
            "--invert-paths".to_string(),
            "--force".to_string(),
        ];
        for path in paths_to_remove {
            args.push("--path".to_string());
            args.push(path.clone());
        }
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let rewrite = std_git_command()
            .args(&args_ref)
            .current_dir(repo)
            .status()
            .with_context(|| format!("failed filter-repo in {}", repo.display()))?;
        if !rewrite.success() {
            return Err(anyhow::anyhow!(
                "filter-repo failed in {} (backup: {})",
                repo.display(),
                backup_branch
            ));
        }
        return Ok(Some(backup_branch));
    }

    let filter_branch_available = std_git_command()
        .args(["filter-branch", "--version"])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if filter_branch_available {
        let mut args: Vec<String> = vec![
            "filter-branch".to_string(),
            "--force".to_string(),
            "--index-filter".to_string(),
        ];
        let filter_expr = "git rm -r --cached --ignore-unmatch".to_string();
        args.push(filter_expr);
        args.extend(paths_to_remove.iter().cloned());
        args.push("--".to_string());
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let rewrite = std_git_command()
            .args(&args_ref)
            .current_dir(repo)
            .status()
            .with_context(|| format!("failed filter-branch in {}", repo.display()))?;
        if !rewrite.success() {
            return Err(anyhow::anyhow!(
                "filter-branch failed in {} (backup: {})",
                repo.display(),
                backup_branch
            ));
        }
        return Ok(Some(backup_branch));
    }

    Err(anyhow::anyhow!(
        "Neither git-filter-repo nor git-filter-branch available in {}. Install git-filter-repo (pip install git-filter-repo) or git-filter-branch to rewrite history (backup branch: {})",
        repo.display(),
        backup_branch
    ))
}

pub(crate) async fn restore_paths(repo: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }

    // Prefer `git restore` (newer git). Fallback to `reset` + `checkout`.
    let mut args = vec!["restore".to_string(), "--staged".to_string(), "--worktree".to_string(), "--".to_string()];
    args.extend(paths.iter().cloned());
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    if run_git_with_timeout(repo, &args_ref, 30, "restore").await.is_ok() {
        return Ok(());
    }

    let mut reset: Vec<String> = Vec::new();
    reset.push("reset".to_string());
    reset.push("HEAD".to_string());
    reset.push("--".to_string());
    reset.extend(paths.iter().cloned());
    let reset_ref: Vec<&str> = reset.iter().map(|s| s.as_str()).collect();
    if let Err(e) = run_git_with_timeout(repo, &reset_ref, 30, "reset").await {
        eprintln!("⚠️ git reset fallback failed for {}: {}", repo.display(), e);
        return Err(anyhow::anyhow!("restore failed: git restore failed and reset fallback also failed: {}", e));
    }

    let mut checkout: Vec<String> = Vec::new();
    checkout.push("checkout".to_string());
    checkout.push("--".to_string());
    checkout.extend(paths.iter().cloned());
    let checkout_ref: Vec<&str> = checkout.iter().map(|s| s.as_str()).collect();
    run_git_with_timeout(repo, &checkout_ref, 30, "checkout").await
}

#[allow(dead_code)]
pub(crate) fn load_secret(env_name: &str) -> Option<String> {
    crate::secrets::load_secret(env_name, &crate::secrets::sync_secrets_dir())
}

pub(crate) mod multi_remote {
    use super::*;

    pub(crate) fn ensure_remote(repo: &Path, name: &str, url: &str) -> Result<()> {
    let existing = get_remote_url(repo, name);
    match existing {
        Some(cur) if cur == url => Ok(()),
        Some(_) => {
            std_git_command()
                .args(["remote", "set-url", name, url])
                .current_dir(repo)
                .status()
                .with_context(|| format!("git remote set-url {} in {}", name, repo.display()))?;
            Ok(())
        }
        None => {
            std_git_command()
                .args(["remote", "add", name, url])
                .current_dir(repo)
                .status()
                .with_context(|| format!("git remote add {} in {}", name, repo.display()))?;
            Ok(())
        }
    }
}

pub(crate) fn configure_all_remotes(repo: &Path, remotes: &[RemoteConfig], repo_name: &str) {
    for remote in remotes {
        let url = remote.resolve_push_url(repo_name);
        if let Err(e) = ensure_remote(repo, &remote.name, &url) {
            eprintln!("⚠️ failed to configure remote {} for {}: {}", remote.name, repo.display(), e);
        }
    }
}

pub(crate) async fn push_mirror_remotes(
    repo: &Path,
    remotes: &[RemoteConfig],
    timeout_secs: u64,
    retries: u32,
) -> Vec<(String, Result<()>)> {
    let repo_name = repo.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    configure_all_remotes(repo, remotes, &repo_name);

    for (remote_name, create_result) in auto_create_all_remotes(remotes, &repo_name).await {
        match create_result {
            Ok(_) => {}
            Err(e) => {
                eprintln!("⚠️ auto-create failed for {} on {}: {}", repo_name, remote_name, e);
            }
        }
    }

    let all_remote_names: Vec<_> = remotes.iter().map(|r| r.name.as_str()).collect();
    if let Err(e) = remove_stale_remotes(repo, &all_remote_names) {
        eprintln!("⚠️ failed to clean stale remotes for {}: {}", repo.display(), e);
    }

    push_to_all_remotes(repo, remotes, timeout_secs, retries).await
}

pub(crate) fn get_remote_url(repo: &Path, name: &str) -> Option<String> {
    let output = std_git_command()
        .args(["remote", "get-url", name])
        .current_dir(repo)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

pub(crate) fn list_remotes(repo: &Path) -> Vec<String> {
    let output = std_git_command()
        .args(["remote"])
        .current_dir(repo)
        .output()
        .ok();
    match output {
        Some(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(String::from)
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn remove_stale_remotes(repo: &Path, keep: &[&str]) -> Result<()> {
    let current = list_remotes(repo);
    let keep_set: std::collections::HashSet<_> = keep.iter().collect();
    for remote in current {
        if remote == "origin" {
            continue;
        }
        if !keep_set.contains(&remote.as_str()) {
            std_git_command()
                .args(["remote", "remove", &remote])
                .current_dir(repo)
                .status()
                .with_context(|| format!("git remote remove {} in {}", remote, repo.display()))?;
        }
    }
    Ok(())
}

pub(crate) async fn push_to_named_remote(
    repo: &Path,
    remote_name: &str,
    timeout_secs: u64,
    retries: u32,
    force_when_behind: bool,
) -> Result<()> {
    let branch = current_branch(repo).unwrap_or_else(|| "main".to_string());
    let refspec = format!("HEAD:refs/heads/{}", branch);
    let ssh_hardening = crate::git::GIT_SSH_HARDENING;

    let attempt_ssh = run_git_with_timeout_env(
        repo,
        &["push", remote_name, &refspec],
        timeout_secs,
        &format!("push-to-{}", remote_name),
        &[("GIT_SSH_COMMAND", ssh_hardening)],
    ).await;

    if attempt_ssh.is_ok() {
        return Ok(());
    }

    let remote_url = get_remote_url(repo, remote_name)
        .ok_or_else(|| anyhow::anyhow!("remote {} not found", remote_name))?;
    if let Some(https) = github_https_url(&remote_url) {
        if is_safe_branch_name(&branch) {
            let https_push = run_git_with_timeout(
                repo,
                &["push", &https, &refspec],
                timeout_secs,
                &format!("push-to-{}https", remote_name),
            ).await;
            if https_push.is_ok() {
                return Ok(());
            }
        }
    }

    let mut last_err = None;
    for attempt in 1..=retries.max(1) {
        match run_git_with_timeout(repo, &["push", remote_name, "HEAD"], timeout_secs, &format!("push-to-{}", remote_name)).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                let is_rejected = e.to_string().contains("non-fast-forward")
                    || e.to_string().contains("failed to push some refs")
                    || e.to_string().contains("[rejected]")
                    || e.to_string().contains("Updates were rejected");
                if is_rejected && force_when_behind {
                    match diagnose_divergence(repo, remote_name, &branch).await {
                        Ok(Divergence::RemotePurelyBehind) => {
                            let force_result = run_git_with_timeout(
                                repo,
                                &["push", "--force-with-lease", remote_name, &format!("HEAD:refs/heads/{}", branch)],
                                timeout_secs,
                                &format!("force-push-to-{}", remote_name),
                            ).await;
                            if force_result.is_ok() {
                                return Ok(());
                            }
                        }
                        Ok(Divergence::Divergent) | Err(_) => {
                            last_err = Some(e);
                        }
                    }
                } else {
                    last_err = Some(e);
                }
                if attempt < retries.max(1) {
                    sleep(Duration::from_secs(attempt as u64)).await;
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("push to {} failed", remote_name)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Divergence {
    RemotePurelyBehind,
    Divergent,
}

pub(crate) async fn diagnose_divergence(repo: &Path, remote_name: &str, branch: &str) -> Result<Divergence> {
    let local_head = run_git_capture_output(repo, &["rev-parse", "HEAD"], "rev-parse")?;
    let local_head = local_head.trim();
    let remote_ref = format!("refs/remotes/{}/{}", remote_name, branch);

    let rev_list_output = run_git_capture_output(
        repo,
        &["rev-list", "--left-right", "--count", &format!("{}...{}", local_head, remote_ref)],
        "rev-list",
    )?;

    let counts: Vec<&str> = rev_list_output.trim().split('\t').collect();
    if counts.len() != 2 {
        return Ok(Divergence::Divergent);
    }

    let _local_ahead: u32 = counts[0].parse().unwrap_or(0);
    let remote_ahead: u32 = counts[1].parse().unwrap_or(0);

    if remote_ahead == 0 {
        Ok(Divergence::RemotePurelyBehind)
    } else {
        Ok(Divergence::Divergent)
    }
}

pub(crate) async fn push_to_all_remotes(
    repo: &Path,
    remotes: &[RemoteConfig],
    timeout_secs: u64,
    retries: u32,
) -> Vec<(String, Result<()>)> {
    let mut sorted = remotes.to_vec();
    sorted.sort_by_key(|r| r.priority);

    let mut results = Vec::new();
    for remote in sorted {
        let result = push_to_named_remote(repo, &remote.name, timeout_secs, retries, remote.force_push_when_behind).await;
        results.push((remote.name.clone(), result));
    }
    results
}

pub(crate) fn create_repo_on_github(account: &str, repo_name: &str) -> Result<String> {
    let mut cmd = std::process::Command::new("gh");
    cmd.args(["repo", "create", repo_name, "--private"]);

    // PAT from ~/.dracon/utilities/sync/secrets/github.env
    // Falls back to gh's stored auth (gh auth login) if PAT is not found.
    // TODO: Consider making PAT mandatory if it proves reliable over time.
    if let Some(token) = load_secret("GH_TOKEN") {
        cmd.env("GH_TOKEN", token);
    }

    let output = cmd.output()
        .with_context(|| "gh repo create failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Name already exists") || stderr.contains("already exists") {
            return Ok(format!("git@github.com:{}/{}.git", account, repo_name));
        }
        anyhow::bail!("gh repo create failed: {}", stderr.trim());
    }

    Ok(format!("git@github.com:{}/{}.git", account, repo_name))
}

pub(crate) fn create_repo_on_gitlab(account: &str, repo_name: &str) -> Result<String> {
    let mut cmd = std::process::Command::new("glab");
    cmd.args(["repo", "create", repo_name, "--private"]);
    
    // Load token from ~/.dracon/utilities/sync/secrets/*.env or env var
    if let Some(token) = load_secret("GITLAB_TOKEN") {
        cmd.env("GITLAB_TOKEN", token);
    }
    
    let output = cmd.output()
        .with_context(|| "glab repo create failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already exists") || stderr.contains("Name already exists") || stderr.contains("has already been taken") {
            return Ok(format!("git@gitlab.com:{}/{}.git", account, repo_name));
        }
        anyhow::bail!("glab repo create failed: {}", stderr.trim());
    }

    Ok(format!("git@gitlab.com:{}/{}.git", account, repo_name))
}

pub(crate) async fn create_repo_on_codeberg(token: &str, account: &str, repo_name: &str, api_endpoint: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post(api_endpoint)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "name": repo_name,
            "private": true,
            "default_branch": "main"
        }))
        .send()
        .await
        .with_context(|| "reqwest codeberg repo create failed")?;

    let status = response.status();
    if status.as_u16() == 409 || status.as_u16() == 422 {
        return Ok(format!("git@codeberg.org:{}/{}.git", account, repo_name));
    }

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("codeberg repo create failed ({}): {}", status, body);
    }

    Ok(format!("git@codeberg.org:{}/{}.git", account, repo_name))
}

pub(crate) async fn auto_create_repo(config: &RemoteConfig, repo_name: &str) -> Result<String> {
    match config.auth_type {
        AuthType::GitHub => create_repo_on_github(&config.auto_create_account, repo_name),
        AuthType::GitLab => create_repo_on_gitlab(&config.auto_create_account, repo_name),
        AuthType::Codeberg => {
            let token_var = config.auto_create_token_var.as_deref().unwrap_or("CODEBERG_TOKEN");
            let token = load_secret(token_var)
                .with_context(|| format!("missing token for Codeberg (set {} env var or ~/.dracon/utilities/sync/secrets/*.env file)", token_var))?;
            let endpoint = config.api_endpoint.as_deref().unwrap_or("https://codeberg.org/api/v1/user/repos");
            create_repo_on_codeberg(&token, &config.auto_create_account, repo_name, endpoint).await
        }
        AuthType::Generic => anyhow::bail!("Generic auth cannot auto-create repos"),
    }
}

pub(crate) async fn auto_create_all_remotes(remotes: &[RemoteConfig], repo_name: &str) -> Vec<(String, Result<String>)> {
        let mut results = Vec::new();
        for remote in remotes {
            if remote.auto_create {
                let resolved_name = remote.resolve_repo_name(repo_name);
                let result = auto_create_repo(remote, &resolved_name).await;
                results.push((remote.name.clone(), result));
            }
        }
        results
    }
}

/// Detect if origin URL points to an orphan -N suffixed repo.
/// Returns Some((current_url, canonical_url)) if orphan detected, None otherwise.
pub(crate) fn detect_orphan_origin(repo: &Path) -> Option<(String, String)> {
    let current = multi_remote::get_remote_url(repo, "origin")?;
    // Pattern: .../repo-name-N.git or .../repo-name-N (where N is one or more digits)
    // Examples: git@github.com:DraconDev/dracon-demons-9.git
    //           git@github.com:DraconDev/dracon-libs-4.git
    let path_part = current.rsplit('/').next()?;
    let (repo_part, suffix) = if let Some(dot) = path_part.rfind(".") {
        (&path_part[..dot], &path_part[dot..])
    } else {
        (path_part, "")
    };
    // Check for -N at the end (only single-digit: -1 through -9)
    // The suffix bug only created -1 through -9, so higher numbers are likely
    // legitimate version suffixes (e.g., api-v2, project-2024)
    if let Some(dash) = repo_part.rfind("-") {
        let suffix_num = &repo_part[dash + 1..];
        if suffix_num.len() == 1 && suffix_num.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            let prefix = &current[..current.len() - path_part.len()];
            let canonical_repo = &repo_part[..dash];
            let canonical = format!("{}{}{}", prefix, canonical_repo, suffix);
            return Some((current, canonical));
        }
    }
    None
}

/// Fix origin URL by setting it to the canonical (non-orphan) URL.
/// Also updates upstream tracking for the current branch if it was set.
pub(crate) fn fix_orphan_origin(repo: &Path, canonical_url: &str) -> Result<()> {
    std_git_command()
        .args(["remote", "set-url", "origin", canonical_url])
        .current_dir(repo)
        .status()
        .with_context(|| format!("git remote set-url origin {} in {}", canonical_url, repo.display()))?;

    // Update upstream tracking for current branch if upstream was set
    if let Some(branch) = current_branch(repo) {
        if has_tracking_upstream(repo) {
            let _ = std_git_command()
                .args(["branch", "--set-upstream-to", &format!("origin/{}", branch), &branch])
                .current_dir(repo)
                .output();
        }
    }

    Ok(())
}

#[cfg(test)]
pub(crate) fn acquire_path_lock() -> parking_lot::MutexGuard<'static, ()> {
    loop {
        if let Some(guard) = PATH_LOCK.try_lock() {
            return guard;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[allow(dead_code)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use crate::git::multi_remote::{diagnose_divergence, push_to_named_remote, Divergence};
    use std::os::unix::fs::PermissionsExt;
    use crate::test_helpers::{EnvRestorer, test_git_cmd};

    #[test]
    fn test_strip_url_credentials_https_with_creds() {
        let url = "https://user:pass@github.com/owner/repo.git";
        let result = strip_url_credentials(url);
        assert_eq!(result, "https://github.com/owner/repo.git");
    }

    #[test]
    fn test_strip_url_credentials_https_without_creds() {
        let url = "https://github.com/owner/repo.git";
        let result = strip_url_credentials(url);
        assert_eq!(result, url);
    }

    #[test]
    fn test_strip_url_credentials_git_url() {
        let url = "git@github.com:owner/repo.git";
        let result = strip_url_credentials(url);
        assert_eq!(result, url);
    }

    #[test]
    fn test_github_https_url_with_embedded_newline() {
        let url = "git@github.com:owner/repo.git\n";
        let result = github_https_url(url);
        assert_eq!(result, Some("https://github.com/owner/repo.git\n".to_string()));
    }

    #[test]
    fn test_github_https_url_ssh_with_colon_path() {
        let url = "git@github.com:owner/repo";
        let result = github_https_url(url);
        assert_eq!(result, Some("https://github.com/owner/repo".to_string()));
    }

    #[test]
    fn test_github_https_url_non_github_returns_none() {
        let url = "https://gitlab.com/owner/repo.git";
        let result = github_https_url(url);
        assert!(result.is_none());
    }

    #[test]
    fn test_strip_url_credentials_with_at_sign() {
        let url = "https://user:token@github.com/owner/repo.git";
        let result = strip_url_credentials(url);
        assert_eq!(result, "https://github.com/owner/repo.git");
    }

    #[test]
    fn test_strip_url_credentials_no_credentials() {
        let url = "https://github.com/owner/repo.git";
        let result = strip_url_credentials(url);
        assert_eq!(result, url);
    }

    #[test]
    fn test_fallback_status_rank_ordering() {
        assert!(fallback_status_rank(&FileStatus::Deleted) > fallback_status_rank(&FileStatus::Modified));
        assert!(fallback_status_rank(&FileStatus::Renamed) > fallback_status_rank(&FileStatus::Added));
        assert!(fallback_status_rank(&FileStatus::TypeChange) > fallback_status_rank(&FileStatus::Unknown));
    }

    #[test]
    fn test_parse_name_status_line_valid_lines() {
        assert_eq!(parse_name_status_line("M\tfile.rs"), Some((PathBuf::from("file.rs"), FileStatus::Modified)));
        assert_eq!(parse_name_status_line("A\tnew.rs"), Some((PathBuf::from("new.rs"), FileStatus::Added)));
        assert_eq!(parse_name_status_line("D\tdeleted.rs"), Some((PathBuf::from("deleted.rs"), FileStatus::Deleted)));
    }

    #[test]
    fn test_parse_name_status_line_renamed() {
        let result = parse_name_status_line("R\told.rs\tnew.rs");
        assert!(result.is_some());
        let (path, status) = result.unwrap();
        assert_eq!(path, PathBuf::from("new.rs"));
        assert_eq!(status, FileStatus::Renamed);
    }

    #[test]
    fn test_parse_name_status_line_invalid_status() {
        assert!(parse_name_status_line("X\tfile.rs").is_none());
        assert!(parse_name_status_line("",).is_none());
    }

    #[test]
    fn test_top_level_dir_simple() {
        assert_eq!(top_level_dir("src/main.rs"), Some("src".to_string()));
        assert_eq!(top_level_dir("docs/readme.md"), Some("docs".to_string()));
    }

    #[test]
    fn test_top_level_dir_single_component() {
        assert_eq!(top_level_dir("main.rs"), Some("main.rs".to_string()));
    }

    #[test]
    fn test_top_level_dir_empty() {
        assert_eq!(top_level_dir(""), Some("".to_string()));
    }

    #[test]
    fn test_top_level_dir_path_with_multiple_slashes() {
        assert_eq!(top_level_dir("src///nested/main.rs"), Some("src".to_string()));
    }

    #[test]
    fn test_is_git_worktree_file_gitdir_prefix() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let dot_git = tmp.path().join(".git");
        std::fs::write(&dot_git, "gitdir: /path/to/worktree").expect("write .git file");
        assert!(is_git_worktree_file(&dot_git));
    }

    #[test]
    fn test_is_git_worktree_file_regular_git_dir() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let dot_git = tmp.path().join(".git");
        std::fs::write(&dot_git, "ref: refs/heads/main").expect("write .git file");
        assert!(!is_git_worktree_file(&dot_git));
    }

    #[test]
    fn test_is_git_worktree_file_nonexistent() {
        let dot_git = std::path::Path::new("/nonexistent/.git");
        assert!(!is_git_worktree_file(dot_git));
    }

    #[test]
    fn test_is_git_worktree_file_with_whitespace() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let dot_git = tmp.path().join(".git");
        std::fs::write(&dot_git, "gitdir: /path/to/worktree\n").expect("write .git file");
        assert!(is_git_worktree_file(&dot_git));
    }

    #[test]
    fn test_load_secret_from_env() {
        let tmp_val = "test_token_abc123";
        let _guard = EnvRestorer::new("TEST_LOAD_SECRET_TOKEN", tmp_val);
        let result = load_secret("TEST_LOAD_SECRET_TOKEN");
        assert_eq!(result, Some(tmp_val.to_string()));
    }

    #[test]
    fn test_load_secret_empty_env_var() {
        let _guard = EnvRestorer::new("TEST_LOAD_SECRET_EMPTY", "");
        let result = load_secret("TEST_LOAD_SECRET_EMPTY");
        assert_eq!(result, None);
    }

    #[test]
    fn test_load_secret_missing() {
        assert_eq!(load_secret("TEST_NONEXISTENT_SECRET_VAR_XYZ"), None);
    }

    #[test]
    fn test_load_secret_from_file() {
        let tmp_home = tempfile::TempDir::new().expect("temp dir");
        let _lock = acquire_path_lock();
        let _guard = EnvRestorer::new("HOME", &tmp_home.path().to_string_lossy());
        let _token_guard = EnvRestorer::remove("TEST_FILE_SECRET_TOKEN");

        let secrets_dir = tmp_home.path().join(".dracon/utilities/sync/secrets");
        std::fs::create_dir_all(&secrets_dir).expect("create secrets dir");
        std::fs::write(secrets_dir.join("test.env"), "TEST_FILE_SECRET_TOKEN=file_token_abc123\n").expect("write env file");

        let result = load_secret("TEST_FILE_SECRET_TOKEN");

        assert_eq!(result, Some("file_token_abc123".to_string()));
    }

    #[test]
    fn test_load_secret_file_with_comments_and_blank_lines() {
        let tmp_home = tempfile::TempDir::new().expect("temp dir");
        let _lock = acquire_path_lock();
        let _guard = EnvRestorer::new("HOME", &tmp_home.path().to_string_lossy());
        let _comments_guard = EnvRestorer::remove("COMMENTED_SECRET_TOKEN");

        let secrets_dir = tmp_home.path().join(".dracon/utilities/sync/secrets");
        std::fs::create_dir_all(&secrets_dir).expect("create secrets dir");
        std::fs::write(
            secrets_dir.join("weird.env"),
            "# This is a comment\n\nTOKEN_BEFORE=value_before\n\nCOMMENTED_SECRET_TOKEN=commented_token_xyz\n# Another comment\nTOKEN_AFTER=value_after\n",
        )
        .expect("write env file");

        let result = load_secret("COMMENTED_SECRET_TOKEN");

        assert_eq!(result, Some("commented_token_xyz".to_string()));
    }

    #[test]
    fn test_load_secret_env_takes_precedence_over_file() {
        let tmp_home = tempfile::TempDir::new().expect("temp dir");
        let _lock = acquire_path_lock();
        let _guard = EnvRestorer::new("HOME", &tmp_home.path().to_string_lossy());
        let _prec_guard = EnvRestorer::new("PRECEDENCE_SECRET", "env_value");

        let secrets_dir = tmp_home.path().join(".dracon/utilities/sync/secrets");
        std::fs::create_dir_all(&secrets_dir).expect("create secrets dir");
        std::fs::write(secrets_dir.join("another.env"), "PRECEDENCE_SECRET=file_value\n").expect("write env file");

        let result = load_secret("PRECEDENCE_SECRET");

        assert_eq!(result, Some("env_value".to_string()));
    }

    #[test]
    fn test_get_remote_url_nonexistent_remote() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        assert_eq!(multi_remote::get_remote_url(&repo, "origin"), None);
    }

    #[test]
    fn test_list_remotes_empty() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        assert!(multi_remote::list_remotes(&repo).is_empty());
    }

    #[test]
    fn test_list_remotes_one_remote() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "origin", "git@github.com:Test/repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add");
        let remotes = multi_remote::list_remotes(&repo);
        assert_eq!(remotes, vec!["origin"]);
    }

    #[test]
    fn test_ensure_remote_adds_new() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");

        multi_remote::ensure_remote(&repo, "github", "git@github.com:Test/repo.git").expect("ensure_remote");

        let url = multi_remote::get_remote_url(&repo, "github");
        assert_eq!(url, Some("git@github.com:Test/repo.git".to_string()));
    }

    #[test]
    fn test_ensure_remote_updates_existing() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "github", "git@github.com:Old/repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add");

        multi_remote::ensure_remote(&repo, "github", "git@github.com:New/repo.git").expect("ensure_remote");

        let url = multi_remote::get_remote_url(&repo, "github");
        assert_eq!(url, Some("git@github.com:New/repo.git".to_string()));
    }

    #[test]
    fn test_ensure_remote_idempotent() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");

        multi_remote::ensure_remote(&repo, "github", "git@github.com:Test/repo.git").expect("ensure_remote 1");
        multi_remote::ensure_remote(&repo, "github", "git@github.com:Test/repo.git").expect("ensure_remote 2");

        let remotes = multi_remote::list_remotes(&repo);
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0], "github");
    }

    #[test]
    fn test_remove_stale_remotes_preserves_origin() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "origin", "git@github.com:Test/repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add origin");
        test_git_cmd()
            .args(["remote", "add", "stale", "git@github.com:stale/repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add stale");

        crate::git::multi_remote::remove_stale_remotes(&repo, &["github"]).expect("remove_stale_remotes");

        let remotes = multi_remote::list_remotes(&repo);
        assert!(remotes.contains(&"origin".to_string()), "origin must be preserved");
        assert!(!remotes.contains(&"stale".to_string()), "stale not in keep list, should be removed");
    }

    #[test]
    fn test_remove_stale_remotes_removes_nonkept() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "origin", "git@github.com:Test/repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add origin");
        test_git_cmd()
            .args(["remote", "add", "mirror1", "git@mirror1.example.com:repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add mirror1");
        test_git_cmd()
            .args(["remote", "add", "mirror2", "git@mirror2.example.com:repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add mirror2");

        crate::git::multi_remote::remove_stale_remotes(&repo, &["mirror1"]).expect("remove_stale_remotes");

        let remotes = multi_remote::list_remotes(&repo);
        assert!(remotes.contains(&"origin".to_string()), "origin always preserved");
        assert!(remotes.contains(&"mirror1".to_string()), "kept remote mirror1 preserved");
        assert!(!remotes.contains(&"mirror2".to_string()), "non-kept remote mirror2 removed");
    }

    #[test]
    fn test_remove_stale_remotes_idempotent_when_empty() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "origin", "git@github.com:Test/repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add origin");

        crate::git::multi_remote::remove_stale_remotes(&repo, &[]).expect("remove_stale_remotes with empty keep list");

        let remotes = multi_remote::list_remotes(&repo);
        assert_eq!(remotes, vec!["origin"]);
    }

    #[test]
    fn test_configure_all_remotes_single_remote() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");

        let remotes = vec![RemoteConfig {
            name: "mirror".to_string(),
            push_url: "git@mirror.example.com:{account}/{repo}.git".to_string(),
            auto_create: false,
            auto_create_account: "myorg".to_string(),
            auth_type: AuthType::GitHub,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
            repo_name_map: Default::default(),
            force_push_when_behind: false,
        }];

        crate::git::multi_remote::configure_all_remotes(&repo, &remotes, "my-repo");

        let url = multi_remote::get_remote_url(&repo, "mirror");
        assert_eq!(url, Some("git@mirror.example.com:myorg/my-repo.git".to_string()));
    }

    #[test]
    fn test_configure_all_remotes_multiple_remotes() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");

        let remotes = vec![
            RemoteConfig {
                name: "github".to_string(),
                push_url: "git@github.com:{account}/{repo}.git".to_string(),
                auto_create: false,
                auto_create_account: "testuser".to_string(),
                auth_type: AuthType::GitHub,
                priority: 50,
                api_endpoint: None,
                auto_create_token_var: None,
                repo_name_map: Default::default(),
                force_push_when_behind: false,
            },
            RemoteConfig {
                name: "gitlab".to_string(),
                push_url: "git@gitlab.com:{account}/{repo}.git".to_string(),
                auto_create: false,
                auto_create_account: "testuser".to_string(),
                auth_type: AuthType::GitLab,
                priority: 50,
                api_endpoint: None,
                auto_create_token_var: None,
                repo_name_map: Default::default(),
                force_push_when_behind: false,
            },
        ];

        crate::git::multi_remote::configure_all_remotes(&repo, &remotes, "multi-repo");

        let github_url = multi_remote::get_remote_url(&repo, "github");
        assert_eq!(github_url, Some("git@github.com:testuser/multi-repo.git".to_string()));

        let gitlab_url = multi_remote::get_remote_url(&repo, "gitlab");
        assert_eq!(gitlab_url, Some("git@gitlab.com:testuser/multi-repo.git".to_string()));
    }

    #[test]
    fn test_configure_all_remotes_idempotent() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");

        let remotes = vec![RemoteConfig {
            name: "origin".to_string(),
            push_url: "git@github.com:user/repo.git".to_string(),
            auto_create: false,
            auto_create_account: "user".to_string(),
            auth_type: AuthType::GitHub,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
            repo_name_map: Default::default(),
            force_push_when_behind: false,
        }];

        crate::git::multi_remote::configure_all_remotes(&repo, &remotes, "repo");
        crate::git::multi_remote::configure_all_remotes(&repo, &remotes, "repo");

        let remotes_list = multi_remote::list_remotes(&repo);
        assert_eq!(remotes_list.len(), 1);
        assert_eq!(remotes_list[0], "origin");
    }

    #[tokio::test]
    async fn test_auto_create_all_remotes_empty_when_no_auto_create() {
        let remotes = vec![
            RemoteConfig {
                name: "mirror1".to_string(),
                push_url: "git@mirror1.example.com:repo.git".to_string(),
                auto_create: false,
                auto_create_account: "".to_string(),
                auth_type: AuthType::GitHub,
                priority: 50,
                api_endpoint: None,
                auto_create_token_var: None,
                repo_name_map: Default::default(),
                force_push_when_behind: false,
            },
            RemoteConfig {
                name: "mirror2".to_string(),
                push_url: "git@mirror2.example.com:repo.git".to_string(),
                auto_create: false,
                auto_create_account: "".to_string(),
                auth_type: AuthType::GitLab,
                priority: 50,
                api_endpoint: None,
                auto_create_token_var: None,
                repo_name_map: Default::default(),
                force_push_when_behind: false,
            },
        ];

        let results = crate::git::multi_remote::auto_create_all_remotes(&remotes, "test-repo").await;
        assert!(results.is_empty(), "should return empty vec when no remotes have auto_create=true");
    }

    #[tokio::test]
    async fn test_auto_create_all_remotes_generic_error() {
        let remotes = vec![RemoteConfig {
            name: "generic".to_string(),
            push_url: "git@generic.example.com:repo.git".to_string(),
            auto_create: true,
            auto_create_account: "testuser".to_string(),
            auth_type: AuthType::Generic,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
            repo_name_map: Default::default(),
            force_push_when_behind: false,
        }];

        let results = crate::git::multi_remote::auto_create_all_remotes(&remotes, "test-repo").await;
        assert_eq!(results.len(), 1);
        assert!(results[0].1.is_err(), "Generic auth should return error");
        let err_msg = format!("{}", results[0].1.as_ref().unwrap_err());
        assert!(err_msg.contains("cannot auto-create"), "error should mention auto-create not supported");
    }

    #[test]
    fn test_auto_create_all_remotes_codeberg_missing_token() {
        // Make load_secret look in a temp dir so real secrets file isn't found
        let tmp_home = tempfile::TempDir::new().expect("temp dir");
        let _home_guard = EnvRestorer::new("HOME", &tmp_home.path().to_string_lossy());
        let _codeberg_guard = EnvRestorer::remove("CODEBERG_TOKEN");

        let remotes = vec![RemoteConfig {
            name: "codeberg".to_string(),
            push_url: "git@codeberg.org:{account}/{repo}.git".to_string(),
            auto_create: true,
            auto_create_account: "testuser".to_string(),
            auth_type: AuthType::Codeberg,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
            repo_name_map: Default::default(),
            force_push_when_behind: false,
        }];

        let results = crate::git::multi_remote::auto_create_all_remotes(&remotes, "test-repo");


        assert_eq!(results.len(), 1);
        assert!(results[0].1.is_err(), "Codeberg without token should return error");
        let err_msg = format!("{}", results[0].1.as_ref().unwrap_err());
        assert!(err_msg.contains("missing token") || err_msg.contains("CODEBERG_TOKEN"), "error should mention missing token");
    }

    #[test]
    fn test_auto_create_all_remotes_github_success() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let gh_mock = tmp.path().join("gh");
        std::fs::write(&gh_mock, "#!/bin/sh\nexit 0\n").expect("write gh mock");
        std::fs::set_permissions(&gh_mock, std::fs::Permissions::from_mode(0o755)).expect("chmod gh");
        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let remotes = vec![RemoteConfig {
            name: "origin".to_string(),
            push_url: "git@github.com:{account}/{repo}.git".to_string(),
            auto_create: true,
            auto_create_account: "testaccount".to_string(),
            auth_type: AuthType::GitHub,
            priority: 1,
            api_endpoint: None,
            auto_create_token_var: None,
            repo_name_map: Default::default(),
            force_push_when_behind: false,
        }];

        let results = crate::git::multi_remote::auto_create_all_remotes(&remotes, "test-repo");
        assert_eq!(results.len(), 1);
        let url = results[0].1.as_ref().unwrap();
        assert_eq!(url, "git@github.com:testaccount/test-repo.git");
    }

    #[test]
    fn test_auto_create_all_remotes_gitlab_success() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let glab_mock = tmp.path().join("glab");
        std::fs::write(&glab_mock, "#!/bin/sh\nexit 0\n").expect("write glab mock");
        std::fs::set_permissions(&glab_mock, std::fs::Permissions::from_mode(0o755)).expect("chmod glab");
        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let remotes = vec![RemoteConfig {
            name: "origin".to_string(),
            push_url: "git@gitlab.com:{account}/{repo}.git".to_string(),
            auto_create: true,
            auto_create_account: "testaccount".to_string(),
            auth_type: AuthType::GitLab,
            priority: 1,
            api_endpoint: None,
            auto_create_token_var: None,
            repo_name_map: Default::default(),
            force_push_when_behind: false,
        }];

        let results = crate::git::multi_remote::auto_create_all_remotes(&remotes, "test-repo");
        assert_eq!(results.len(), 1);
        let url = results[0].1.as_ref().unwrap();
        assert_eq!(url, "git@gitlab.com:testaccount/test-repo.git");
    }

    #[test]
    fn test_create_repo_on_codeberg_success_201() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            let response = "HTTP/1.1 201 Created\r\nContent-Length: 0\r\n\r\n";
            std::io::Write::write_all(&mut stream, response.as_bytes()).expect("write");
        });

        let url = format!("http://127.0.0.1:{}/api/v1/repos", port);
        let result = crate::git::multi_remote::create_repo_on_codeberg("test_token", "testuser", "myrepo", &url);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "git@codeberg.org:testuser/myrepo.git");
    }

    #[test]
    fn test_create_repo_on_codeberg_conflict_409() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            let response = "HTTP/1.1 409 Conflict\r\nContent-Length: 0\r\n\r\n";
            std::io::Write::write_all(&mut stream, response.as_bytes()).expect("write");
        });

        let url = format!("http://127.0.0.1:{}/api/v1/repos", port);
        let result = crate::git::multi_remote::create_repo_on_codeberg("test_token", "testuser", "myrepo", &url);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "git@codeberg.org:testuser/myrepo.git");
    }

    #[test]
    fn test_create_repo_on_codeberg_unprocessable_422() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            let response = "HTTP/1.1 422 Unprocessable Entity\r\nContent-Length: 0\r\n\r\n";
            std::io::Write::write_all(&mut stream, response.as_bytes()).expect("write");
        });

        let url = format!("http://127.0.0.1:{}/api/v1/repos", port);
        let result = crate::git::multi_remote::create_repo_on_codeberg("test_token", "testuser", "myrepo", &url);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "git@codeberg.org:testuser/myrepo.git");
    }

    #[test]
    fn test_create_repo_on_codeberg_unauthorized_401() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 1024];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            let body = r#"{"message": "Unauthorized"}"#;
            let response = format!(
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            std::io::Write::write_all(&mut stream, response.as_bytes()).expect("write");
        });

        let url = format!("http://127.0.0.1:{}/api/v1/repos", port);
        let result = crate::git::multi_remote::create_repo_on_codeberg("bad_token", "testuser", "myrepo", &url);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("401") || err_msg.contains("Unauthorized"), "error should mention 401: {}", err_msg);
    }

    #[tokio::test]
    async fn test_push_to_named_remote_fails_on_invalid_remote() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "origin", "git@invalid.example.com:repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add");

        let result = crate::git::multi_remote::push_to_named_remote(&repo, "origin", 1, 0, false).await;
        assert!(result.is_err(), "push to invalid remote should fail");
    }

    #[tokio::test]
    async fn test_push_to_all_remotes_returns_all_results() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "mirror1", "git@invalid1.example.com:repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add mirror1");
        test_git_cmd()
            .args(["remote", "add", "mirror2", "git@invalid2.example.com:repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add mirror2");

        let remotes = vec![
            RemoteConfig {
                name: "mirror1".to_string(),
                push_url: "git@invalid1.example.com:repo.git".to_string(),
                auto_create: false,
                auto_create_account: "".to_string(),
                auth_type: AuthType::GitHub,
                priority: 10,
                api_endpoint: None,
                auto_create_token_var: None,
                repo_name_map: Default::default(),
                force_push_when_behind: false,
            },
            RemoteConfig {
                name: "mirror2".to_string(),
                push_url: "git@invalid2.example.com:repo.git".to_string(),
                auto_create: false,
                auto_create_account: "".to_string(),
                auth_type: AuthType::GitHub,
                priority: 20,
                api_endpoint: None,
                auto_create_token_var: None,
                repo_name_map: Default::default(),
                force_push_when_behind: false,
            },
        ];

        let results = crate::git::multi_remote::push_to_all_remotes(&repo, &remotes, 1, 0).await;
        assert_eq!(results.len(), 2, "should return results for both remotes");
        assert_eq!(results[0].0, "mirror1", "lower priority should be first");
        assert_eq!(results[1].0, "mirror2", "higher priority should be second");
        assert!(results[0].1.is_err(), "mirror1 push should fail");
        assert!(results[1].1.is_err(), "mirror2 push should fail");
    }

    #[tokio::test]
    async fn test_push_mirror_remotes_empty_when_no_remotes() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");

        let results = crate::git::multi_remote::push_mirror_remotes(&repo, &[], 1, 0).await;
        assert!(results.is_empty(), "should return empty results for empty remotes");
    }

    #[test]
    fn test_create_repo_on_github_success() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let gh_mock = tmp.path().join("gh");
        std::fs::write(&gh_mock, "#!/bin/sh\nexit 0\n").expect("write gh mock");
        std::fs::set_permissions(&gh_mock, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let result = multi_remote::create_repo_on_github("testuser", "my-repo");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "git@github.com:testuser/my-repo.git");
    }

    #[test]
    fn test_create_repo_on_github_already_exists_returns_url_without_suffix() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let gh_mock = tmp.path().join("gh");
        std::fs::write(
            &gh_mock,
            "#!/bin/sh\necho 'Name already exists' >&2\nexit 1\n",
        )
        .expect("write gh mock");
        std::fs::set_permissions(&gh_mock, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let result = multi_remote::create_repo_on_github("testuser", "dracon-demons");

        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(!url.contains("-1"), "should NOT have suffix -1: {}", url);
        assert_eq!(url, "git@github.com:testuser/dracon-demons.git");
    }

    #[test]
    fn test_create_repo_on_github_pat_passed_as_env_var() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let gh_mock = tmp.path().join("gh");
        std::fs::write(
            &gh_mock,
            "#!/bin/sh\nif [ -n \"$GH_TOKEN\" ]; then echo 'PAT received' >&2; fi\nexit 0\n",
        )
        .expect("write gh mock");
        std::fs::set_permissions(&gh_mock, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));
        let _gh_guard = EnvRestorer::new("GH_TOKEN", "test_pat_from_env");

        let result = multi_remote::create_repo_on_github("testuser", "test-repo");

        assert!(result.is_ok());
    }

    #[test]
    fn test_create_repo_on_gitlab_success() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let glab_mock = tmp.path().join("glab");
        std::fs::write(&glab_mock, "#!/bin/sh\nexit 0\n").expect("write glab mock");
        std::fs::set_permissions(&glab_mock, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let result = multi_remote::create_repo_on_gitlab("testuser", "my-repo");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "git@gitlab.com:testuser/my-repo.git");
    }

    #[test]
    fn test_create_repo_on_gitlab_already_exists_returns_url_without_suffix() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let glab_mock = tmp.path().join("glab");
        std::fs::write(
            &glab_mock,
            "#!/bin/sh\necho 'Repository has already been taken' >&2\nexit 1\n",
        )
        .expect("write glab mock");
        std::fs::set_permissions(&glab_mock, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let result = multi_remote::create_repo_on_gitlab("testuser", "dracon-demons");

        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(!url.contains("-1"), "should NOT have suffix -1: {}", url);
        assert_eq!(url, "git@gitlab.com:testuser/dracon-demons.git");
    }

    #[test]
    fn test_create_repo_on_gitlab_network_error() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let glab_mock = tmp.path().join("glab");
        std::fs::write(
            &glab_mock,
            "#!/bin/sh\necho 'Connection timeout' >&2\nexit 128\n",
        )
        .expect("write glab mock");
        std::fs::set_permissions(&glab_mock, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let result = multi_remote::create_repo_on_gitlab("testuser", "test-repo");

        assert!(result.is_err());
    }

    #[test]
    fn test_create_repo_on_gitlab_token_passed_as_env_var() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let glab_mock = tmp.path().join("glab");
        std::fs::write(&glab_mock, "#!/bin/sh\nif [ -n \"$GITLAB_TOKEN\" ]; then echo 'Token received'; fi\nexit 0\n").expect("write glab mock");
        std::fs::set_permissions(&glab_mock, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));
        let _glab_guard = EnvRestorer::new("GITLAB_TOKEN", "test_gitlab_token");

        let result = multi_remote::create_repo_on_gitlab("testuser", "test-repo");

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_push_with_retries_succeeds_first_attempt() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let bare = tmp.path().join("bare.git");
        test_git_cmd()
            .args(["init", "--bare", &bare.to_string_lossy()])
            .output()
            .expect("git init --bare");
        let repo = tmp.path().join("repo");
        test_git_cmd()
            .args(["init", "-q", &repo.to_string_lossy()])
            .output()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "origin", &bare.to_string_lossy()])
            .current_dir(&repo)
            .output()
            .expect("git remote add");
        std::fs::write(repo.join("f"), "content").expect("write file");
        test_git_cmd()
            .args(["add", "f"])
            .current_dir(&repo)
            .output()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output()
            .expect("git commit");

        let result = crate::git::push_with_retries(&repo, 5, 3, "test-push").await;
        assert!(result.is_ok(), "push should succeed on first attempt: {:?}", result);
    }

    #[tokio::test]
    async fn test_push_with_retries_retries_then_succeeds() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let counter = tmp.path().join("call_counter");
        std::fs::write(&counter, "0").expect("write counter");

        let real_git = real_git_path();
        let fail_script = tmp.path().join("git");
        let counter_path = counter.display().to_string();
        std::fs::write(&fail_script, format!(
            "#!/bin/sh\n\
            count=$(cat {counter})\n\
            if [ \"$count\" -lt 1 ]; then\n\
                echo \"simulated failure\" >&2\n\
                echo $((count+1)) > {counter}\n\
                exit 1\n\
            fi\n\
            exec {real_git} \"$@\"\n\
            ",
            counter = counter_path,
            real_git = real_git.display()
        )).expect("write fail script");
        std::fs::set_permissions(&fail_script, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let bare = tmp.path().join("bare.git");
        std::process::Command::new(real_git.as_path())
            .args(["init", "--bare", &bare.to_string_lossy()])
            .output()
            .expect("git init --bare");
        let repo = tmp.path().join("repo");
        std::process::Command::new(real_git.as_path())
            .args(["init", "-q", &repo.to_string_lossy()])
            .output()
            .expect("git init");
        std::process::Command::new(real_git.as_path())
            .args(["remote", "add", "origin", &bare.to_string_lossy()])
            .current_dir(&repo)
            .output()
            .expect("git remote add");
        std::fs::write(repo.join("f"), "content").expect("write file");
        std::process::Command::new(real_git.as_path())
            .args(["add", "f"])
            .current_dir(&repo)
            .output()
            .expect("git add");
        std::process::Command::new(real_git.as_path())
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output()
            .expect("git commit");

        drop(_lock);
        let result = crate::git::push_with_retries(&repo, 5, 3, "test-push-retry").await;
        assert!(result.is_ok(), "push should eventually succeed after retry: {:?}", result);
    }

    #[tokio::test]
    async fn test_push_with_retries_exhausts_retries_and_fails() {
        let tmp = tempfile::TempDir::new().expect("temp dir");

        let real_git = real_git_path();
        let always_fail = tmp.path().join("git");
        std::fs::write(&always_fail, "#!/bin/sh\n\
            echo 'always fail' >&2\n\
            exit 1\n\
            ").expect("write fail git");
        std::fs::set_permissions(&always_fail, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let bare = tmp.path().join("bare.git");
        std::process::Command::new(real_git.as_path())
            .args(["init", "--bare", &bare.to_string_lossy()])
            .output()
            .expect("git init --bare");
        let repo = tmp.path().join("repo");
        std::process::Command::new(real_git.as_path())
            .args(["init", "-q", &repo.to_string_lossy()])
            .output()
            .expect("git init");
        std::process::Command::new(real_git.as_path())
            .args(["remote", "add", "origin", &bare.to_string_lossy()])
            .current_dir(&repo)
            .output()
            .expect("git remote add");
        std::fs::write(repo.join("f"), "content").expect("write file");
        std::process::Command::new(real_git.as_path())
            .args(["add", "f"])
            .current_dir(&repo)
            .output()
            .expect("git add");
        std::process::Command::new(real_git.as_path())
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output()
            .expect("git commit");

        drop(_guard);
        drop(_lock);

        let _git_bin_guard = EnvRestorer::new("DRACON_SYNC_GIT_BIN", &always_fail.to_string_lossy());
        let result = crate::git::push_with_retries(&repo, 1, 2, "test-push-fail").await;

        assert!(result.is_err(), "push should fail after exhausting retries");
    }

    #[tokio::test]
    async fn test_push_with_transport_fallbacks_ssh_succeeds_no_fallback() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let bare = tmp.path().join("bare.git");
        test_git_cmd()
            .args(["init", "--bare", &bare.to_string_lossy()])
            .output()
            .expect("git init --bare");
        let repo = tmp.path().join("repo");
        test_git_cmd()
            .args(["init", "-q", &repo.to_string_lossy()])
            .output()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "origin", &bare.to_string_lossy()])
            .current_dir(&repo)
            .output()
            .expect("git remote add");
        std::fs::write(repo.join("f"), "content").expect("write file");
        test_git_cmd()
            .args(["add", "f"])
            .current_dir(&repo)
            .output()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output()
            .expect("git commit");

        let result = crate::git::push_with_transport_fallbacks(&repo, 5, "test-push").await;
        assert!(result.is_ok(), "SSH push should succeed: {:?}", result);
    }

    #[tokio::test]
    async fn test_push_with_transport_fallbacks_ssh_fails_https_fallback_succeeds() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let real_git = real_git_path();
        let fail_git = tmp.path().join("git");
        let real_git_path_str = real_git.display().to_string();
        std::fs::write(&fail_git, format!(
            "#!/bin/sh\n\
            if echo \"$@\" | grep -q 'GIT_SSH_COMMAND'; then\n\
                echo 'SSH failure' >&2\n\
                exit 128\n\
            fi\n\
            exec {real_git_path_str} \"$@\"\n\
            "
        )).expect("write fail git");
        std::fs::set_permissions(&fail_git, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let bare = tmp.path().join("bare.git");
        std::process::Command::new(real_git.as_path())
            .args(["init", "--bare", &bare.to_string_lossy()])
            .output()
            .expect("git init --bare");
        let bare_url = format!("file://{}", bare.to_string_lossy());
        let repo = tmp.path().join("repo");
        std::process::Command::new(real_git.as_path())
            .args(["init", "-q", &repo.to_string_lossy()])
            .output()
            .expect("git init");
        std::process::Command::new(real_git.as_path())
            .args(["remote", "add", "origin", &bare_url])
            .current_dir(&repo)
            .output()
            .expect("git remote add");
        std::fs::write(repo.join("f"), "content").expect("write file");
        std::process::Command::new(real_git.as_path())
            .args(["add", "f"])
            .current_dir(&repo)
            .output()
            .expect("git add");
        std::process::Command::new(real_git.as_path())
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output()
            .expect("git commit");

        drop(_lock);
        let result = crate::git::push_with_transport_fallbacks(&repo, 5, "test-push-fb").await;
        assert!(result.is_ok(), "HTTPS fallback should succeed after SSH failure: {:?}", result);
    }

    #[tokio::test]
    async fn test_push_with_transport_fallbacks_both_fail() {
        let tmp = tempfile::TempDir::new().expect("temp dir");

        let real_git = real_git_path();
        let always_fail = tmp.path().join("git");
        std::fs::write(&always_fail, "#!/bin/sh\necho 'always fail' >&2\nexit 1\n")
            .expect("write fail git");
        std::fs::set_permissions(&always_fail, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let bare = tmp.path().join("bare.git");
        std::process::Command::new(real_git.as_path())
            .args(["init", "--bare", &bare.to_string_lossy()])
            .output()
            .expect("git init --bare");
        let bare_url = format!("file://{}", bare.to_string_lossy());
        let repo = tmp.path().join("repo");
        std::process::Command::new(real_git.as_path())
            .args(["init", "-q", &repo.to_string_lossy()])
            .output()
            .expect("git init");
        std::process::Command::new(real_git.as_path())
            .args(["remote", "add", "origin", &bare_url])
            .current_dir(&repo)
            .output()
            .expect("git remote add");
        std::fs::write(repo.join("f"), "content").expect("write file");
        std::process::Command::new(real_git.as_path())
            .args(["add", "f"])
            .current_dir(&repo)
            .output()
            .expect("git add");
        std::process::Command::new(real_git.as_path())
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output()
            .expect("git commit");

        drop(_guard);
        drop(_lock);

        let _git_bin_guard = EnvRestorer::new("DRACON_SYNC_GIT_BIN", &always_fail.to_string_lossy());
        let result = crate::git::push_with_transport_fallbacks(&repo, 1, "test-push-both-fail").await;

        assert!(result.is_err(), "both SSH and HTTPS should fail");
    }

    #[tokio::test]
    async fn test_push_to_named_remote_ssh_success() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let real_git = real_git_path();
        let bare = tmp.path().join("bare.git");
        std::process::Command::new(real_git.as_path())
            .args(["init", "--bare", &bare.to_string_lossy()])
            .output()
            .expect("git init --bare");
        let repo = tmp.path().join("repo");
        std::process::Command::new(real_git.as_path())
            .args(["init", "-q", "-b", "master", &repo.to_string_lossy()])
            .output()
            .expect("git init");
        std::process::Command::new(real_git.as_path())
            .args(["remote", "add", "mirror", &bare.to_string_lossy()])
            .current_dir(&repo)
            .output()
            .expect("git remote add");
        std::fs::write(repo.join("f"), "content").expect("write file");
        std::process::Command::new(real_git.as_path())
            .args(["add", "f"])
            .current_dir(&repo)
            .output()
            .expect("git add");
        std::process::Command::new(real_git.as_path())
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output()
            .expect("git commit");

        let result = multi_remote::push_to_named_remote(&repo, "mirror", 5, 0, false).await;
        assert!(result.is_ok(), "SSH push to named remote should succeed: {:?}", result);
    }

    #[tokio::test]
    async fn test_push_to_named_remote_ssh_fails_https_fallback() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let real_git = real_git_path();
        let fail_git = tmp.path().join("git");
        let real_git_path_str = real_git.display().to_string();
        std::fs::write(&fail_git, format!(
            "#!/bin/sh\n\
            if echo \"$@\" | grep -q 'GIT_SSH_COMMAND'; then\n\
                echo 'SSH failure' >&2\n\
                exit 128\n\
            fi\n\
            exec {real_git_path_str} \"$@\"\n\
            "
        )).expect("write fail git");
        std::fs::set_permissions(&fail_git, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let bare = tmp.path().join("bare.git");
        std::process::Command::new(real_git.as_path())
            .args(["init", "--bare", &bare.to_string_lossy()])
            .output()
            .expect("git init --bare");
        let bare_url = format!("file://{}", bare.to_string_lossy());
        let repo = tmp.path().join("repo");
        std::process::Command::new(real_git.as_path())
            .args(["init", "-q", "-b", "master", &repo.to_string_lossy()])
            .output()
            .expect("git init");
        std::process::Command::new(real_git.as_path())
            .args(["remote", "add", "mirror", &bare_url])
            .current_dir(&repo)
            .output()
            .expect("git remote add");
        std::fs::write(repo.join("f"), "content").expect("write file");
        std::process::Command::new(real_git.as_path())
            .args(["add", "f"])
            .current_dir(&repo)
            .output()
            .expect("git add");
        std::process::Command::new(real_git.as_path())
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output()
            .expect("git commit");

        drop(_lock);
        let result = multi_remote::push_to_named_remote(&repo, "mirror", 5, 0, false).await;
        assert!(result.is_ok(), "HTTPS fallback should succeed after SSH failure: {:?}", result);
    }

    #[tokio::test]
    async fn test_push_to_named_remote_unsafe_branch_skips_https_fallback() {
        let tmp = tempfile::TempDir::new().expect("temp dir");

        let real_git = real_git_path();
        let always_fail = tmp.path().join("git");
        std::fs::write(&always_fail, "#!/bin/sh\necho 'SSH failure' >&2\nexit 128\n")
            .expect("write fail git");
        std::fs::set_permissions(&always_fail, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let _lock = acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new("PATH", &format!("{}:{}", tmp.path().to_string_lossy(), orig_path));

        let bare = tmp.path().join("bare.git");
        std::process::Command::new(real_git.as_path())
            .args(["init", "--bare", &bare.to_string_lossy()])
            .output()
            .expect("git init --bare");
        let bare_url = format!("file://{}", bare.to_string_lossy());
        let repo = tmp.path().join("repo");
        std::process::Command::new(real_git.as_path())
            .args(["init", "-q", &repo.to_string_lossy()])
            .output()
            .expect("git init");
        std::process::Command::new(real_git.as_path())
            .args(["checkout", "--orphan", "deploy/prod"])
            .current_dir(&repo)
            .output()
            .expect("git checkout -b deploy/prod");
        std::process::Command::new(real_git.as_path())
            .args(["remote", "add", "mirror", &bare_url])
            .current_dir(&repo)
            .output()
            .expect("git remote add");
        std::fs::write(repo.join("f"), "content").expect("write file");
        std::process::Command::new(real_git.as_path())
            .args(["add", "f"])
            .current_dir(&repo)
            .output()
            .expect("git add");
        std::process::Command::new(real_git.as_path())
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output()
            .expect("git commit");

        drop(_guard);
        drop(_lock);

        let _git_bin_guard = EnvRestorer::new("DRACON_SYNC_GIT_BIN", &always_fail.to_string_lossy());
        let result = multi_remote::push_to_named_remote(&repo, "mirror", 1, 0, false).await;

        assert!(result.is_err(), "push should fail");
    }

    #[tokio::test]
    async fn test_run_git_with_timeout_succeeds() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write file");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(&repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .status()
            .expect("git commit");

        let result = run_git_with_timeout(&repo, &["status"], 10, "status").await;
        assert!(result.is_ok(), "git status should succeed: {:?}", result);
    }

    #[tokio::test]
    async fn test_run_git_with_timeout_env_injects_env_vars() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write file");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(&repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .status()
            .expect("git commit");

        let result = run_git_with_timeout_env(
            &repo,
            &["log", "--format=%s"],
            10,
            "log",
            &[("GIT_AUTHOR_NAME", "Test Author"), ("GIT_COMMITTER_NAME", "Test Committer")],
        ).await;
        assert!(result.is_ok(), "git log with env vars should work: {:?}", result);
    }

    #[tokio::test]
    async fn test_restore_paths_uses_git_restore_fallback_chain() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "original content").expect("write file");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(&repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .status()
            .expect("git commit");

        std::fs::write(repo.join("file.txt"), "modified content").expect("write modified");

        let result = restore_paths(&repo, &["file.txt".to_string()]).await;
        assert!(result.is_ok(), "restore_paths should succeed: {:?}", result);
        let content = std::fs::read_to_string(repo.join("file.txt")).expect("read file");
        assert_eq!(content, "original content", "file should be restored to original content");
    }

    #[tokio::test]
    async fn test_diagnose_divergence_remote_purely_behind() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(&repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .status()
            .expect("git commit");

        let local_commit = {
            let output = test_git_cmd()
                .args(["rev-parse", "HEAD"])
                .current_dir(&repo)
                .output()
                .expect("git rev-parse");
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };

        test_git_cmd()
            .args(["remote", "add", "mirror", "git@mirror.example.com:repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add");

        test_git_cmd()
            .args(["update-ref", "refs/remotes/mirror/master", &local_commit])
            .current_dir(&repo)
            .status()
            .expect("git update-ref");

        let result = diagnose_divergence(&repo, "mirror", "master").await;
        assert!(result.is_ok(), "diagnose_divergence should succeed");
        assert_eq!(result.unwrap(), Divergence::RemotePurelyBehind, "remote with no extra commits should be purely behind");
    }

    #[tokio::test]
    async fn test_diagnose_divergence_divergent() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(&repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .status()
            .expect("git commit");

        test_git_cmd()
            .args(["remote", "add", "mirror", "git@mirror.example.com:repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add");

        let (local_commit, remote_commit) = {
            let local = test_git_cmd()
                .args(["rev-parse", "HEAD"])
                .current_dir(&repo)
                .output()
                .expect("git rev-parse")
                .stdout;
            let local = String::from_utf8_lossy(&local).trim().to_string();

            test_git_cmd()
                .args(["commit", "--allow-empty", "-m", "other commit"])
                .current_dir(&repo)
                .status()
                .expect("git commit --allow-empty");
            let remote = test_git_cmd()
                .args(["rev-parse", "HEAD"])
                .current_dir(&repo)
                .output()
                .expect("git rev-parse")
                .stdout;
            let remote = String::from_utf8_lossy(&remote).trim().to_string();
            (local, remote)
        };

        test_git_cmd()
            .args(["update-ref", "refs/remotes/mirror/master", &remote_commit])
            .current_dir(&repo)
            .status()
            .expect("git update-ref");

        test_git_cmd()
            .args(["reset", "--hard", &local_commit])
            .current_dir(&repo)
            .status()
            .expect("git reset");

        let result = diagnose_divergence(&repo, "mirror", "master").await;
        assert!(result.is_ok(), "diagnose_divergence should succeed");
        assert_eq!(result.unwrap(), Divergence::Divergent, "remote with commits local lacks should be divergent");
    }

    #[tokio::test]
    async fn test_push_to_named_remote_auto_force_when_behind() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let real_git = real_git_path();

        let bare = tmp.path().join("bare.git");
        std::process::Command::new(real_git.as_path())
            .args(["init", "--bare", &bare.to_string_lossy()])
            .output()
            .expect("git init --bare");

        let repo = tmp.path().join("repo");
        std::process::Command::new(real_git.as_path())
            .args(["init", "-q", "-b", "master", &repo.to_string_lossy()])
            .output()
            .expect("git init");
        std::process::Command::new(real_git.as_path())
            .args(["remote", "add", "mirror", &bare.to_string_lossy()])
            .current_dir(&repo)
            .output()
            .expect("git remote add");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        std::process::Command::new(real_git.as_path())
            .args(["add", "."])
            .current_dir(&repo)
            .output()
            .expect("git add");
        std::process::Command::new(real_git.as_path())
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .output()
            .expect("git commit");

        std::process::Command::new(real_git.as_path())
            .args(["commit", "--allow-empty", "-m", "other commit"])
            .current_dir(&repo)
            .output()
            .expect("git commit");

        let remote_commit = {
            let output = std::process::Command::new(real_git.as_path())
                .args(["rev-parse", "HEAD"])
                .current_dir(&repo)
                .output()
                .expect("git rev-parse");
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };

        std::process::Command::new(real_git.as_path())
            .args(["update-ref", "refs/remotes/mirror/master", &remote_commit])
            .current_dir(&repo)
            .output()
            .expect("git update-ref");

        std::process::Command::new(real_git.as_path())
            .args(["reset", "--hard", "HEAD^"])
            .current_dir(&repo)
            .output()
            .expect("git reset");

        drop(acquire_path_lock());
        let result = push_to_named_remote(&repo, "mirror", 5, 0, true).await;
        assert!(result.is_ok(), "push with force_when_behind=true should succeed when remote is purely behind: {:?}", result);
    }

    #[tokio::test]
    async fn test_push_to_named_remote_no_auto_force_when_divergent() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(&repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .status()
            .expect("git commit");

        test_git_cmd()
            .args(["remote", "add", "mirror", "git@mirror.example.com:repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add");

        let (_local_commit, remote_commit) = {
            let local = test_git_cmd()
                .args(["rev-parse", "HEAD"])
                .current_dir(&repo)
                .output()
                .expect("git rev-parse");
            let _local = String::from_utf8_lossy(&local.stdout).trim().to_string();
            test_git_cmd()
                .args(["commit", "--allow-empty", "-m", "other commit"])
                .current_dir(&repo)
                .status()
                .expect("git commit");
            let output = test_git_cmd()
                .args(["rev-parse", "HEAD"])
                .current_dir(&repo)
                .output()
                .expect("git rev-parse");
            let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (local, remote)
        };

        test_git_cmd()
            .args(["update-ref", "refs/remotes/mirror/master", &remote_commit])
            .current_dir(&repo)
            .status()
            .expect("git update-ref");

        test_git_cmd()
            .args(["reset", "--hard", "HEAD^"])
            .current_dir(&repo)
            .status()
            .expect("git reset");

        drop(acquire_path_lock());
        let result = push_to_named_remote(&repo, "mirror", 5, 0, true).await;
        assert!(result.is_err(), "push with force_when_behind=true should fail when remote is divergent: {:?}", result);
    }

    #[tokio::test]
    async fn test_push_to_named_remote_no_auto_force_when_disabled() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(&repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(&repo)
            .status()
            .expect("git commit");

        test_git_cmd()
            .args(["remote", "add", "mirror", "git@mirror.example.com:repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add");

        test_git_cmd()
            .args(["commit", "--allow-empty", "-m", "other commit"])
            .current_dir(&repo)
            .status()
            .expect("git commit");

        let remote_commit = {
            let output = test_git_cmd()
                .args(["rev-parse", "HEAD"])
                .current_dir(&repo)
                .output()
                .expect("git rev-parse");
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };

        test_git_cmd()
            .args(["update-ref", "refs/remotes/mirror/master", &remote_commit])
            .current_dir(&repo)
            .status()
            .expect("git update-ref");

        test_git_cmd()
            .args(["reset", "--hard", "HEAD^"])
            .current_dir(&repo)
            .status()
            .expect("git reset");

        drop(acquire_path_lock());
        let result = push_to_named_remote(&repo, "mirror", 5, 0, false).await;
        assert!(result.is_err(), "push with force_when_behind=false should fail with rejected error");
    }

    #[test]
    fn test_detect_orphan_origin_detects_single_digit_suffix() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        test_git_cmd()
            .args(["init", "-q"])
            .current_dir(repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "origin", "git@github.com:DraconDev/dracon-demons-9.git"])
            .current_dir(repo)
            .status()
            .expect("git remote add");

        let result = detect_orphan_origin(repo);
        assert!(result.is_some(), "should detect -9 suffix");
        let (current, canonical) = result.unwrap();
        assert_eq!(current, "git@github.com:DraconDev/dracon-demons-9.git");
        assert_eq!(canonical, "git@github.com:DraconDev/dracon-demons.git");
    }

    #[test]
    fn test_detect_orphan_origin_ignores_multi_digit_suffix() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        test_git_cmd()
            .args(["init", "-q"])
            .current_dir(repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "origin", "git@github.com:DraconDev/project-2024.git"])
            .current_dir(repo)
            .status()
            .expect("git remote add");

        let result = detect_orphan_origin(repo);
        assert!(result.is_none(), "should NOT detect -2024 as orphan (multi-digit)");
    }

    #[test]
    fn test_detect_orphan_origin_ignores_legitimate_version() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        test_git_cmd()
            .args(["init", "-q"])
            .current_dir(repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "origin", "git@github.com:DraconDev/api-v2.git"])
            .current_dir(repo)
            .status()
            .expect("git remote add");

        let result = detect_orphan_origin(repo);
        assert!(result.is_none(), "should NOT detect -v2 as orphan (not pure digits)");
    }

    #[test]
    fn test_detect_orphan_origin_no_suffix() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        test_git_cmd()
            .args(["init", "-q"])
            .current_dir(repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["remote", "add", "origin", "git@github.com:DraconDev/dracon-demons.git"])
            .current_dir(repo)
            .status()
            .expect("git remote add");

        let result = detect_orphan_origin(repo);
        assert!(result.is_none(), "should NOT detect normal repo name as orphan");
    }

#[test]
    fn test_fix_orphan_origin_updates_remote_url() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        test_git_cmd()
            .args(["init", "-q"])
            .current_dir(repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(repo)
            .status()
            .expect("git commit");
        test_git_cmd()
            .args(["remote", "add", "origin", "git@github.com:DraconDev/dracon-demons-9.git"])
            .current_dir(repo)
            .status()
            .expect("git remote add");

        let result = fix_orphan_origin(repo, "git@github.com:DraconDev/dracon-demons.git");
        assert!(result.is_ok(), "fix_orphan_origin should succeed");

        let url = multi_remote::get_remote_url(repo, "origin").unwrap();
        assert_eq!(url, "git@github.com:DraconDev/dracon-demons.git");
    }

    #[test]
    fn test_fix_orphan_origin_updates_upstream_tracking() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        let bare = tmp.path().join("bare.git");
        test_git_cmd()
            .args(["init", "-q", "--bare", bare.to_str().unwrap()])
            .status()
            .expect("git init bare");
        test_git_cmd()
            .args(["init", "-q", "-b", "main"])
            .current_dir(repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(repo)
            .status()
            .expect("git commit");
        test_git_cmd()
            .args(["remote", "add", "origin", bare.to_str().unwrap()])
            .current_dir(repo)
            .status()
            .expect("git remote add");
        test_git_cmd()
            .args(["push", "-u", "origin", "main"])
            .current_dir(repo)
            .status()
            .expect("git push");

        test_git_cmd()
            .args(["remote", "set-url", "origin", "git@github.com:DraconDev/dracon-demons-9.git"])
            .current_dir(repo)
            .status()
            .expect("git remote set-url");

        let result = fix_orphan_origin(repo, "git@github.com:DraconDev/dracon-demons.git");
        assert!(result.is_ok(), "fix_orphan_origin should succeed");

        let url = multi_remote::get_remote_url(repo, "origin").unwrap();
        assert_eq!(url, "git@github.com:DraconDev/dracon-demons.git");

        let upstream_info = {
            let output = test_git_cmd()
                .args(["branch", "-vv", "--no-color"])
                .current_dir(repo)
                .output()
                .expect("git branch -vv");
            String::from_utf8_lossy(&output.stdout).to_string()
        };
        assert!(upstream_info.contains("origin/main"), "branch should track origin/main after fix");
    }

    #[tokio::test]
    async fn test_consolidate_to_main_deletes_master_and_keeps_main() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        let bare = tmp.path().join("bare.git");
        test_git_cmd()
            .args(["init", "-q", "--bare", bare.to_str().unwrap()])
            .status()
            .expect("git init bare");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .current_dir(repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(repo)
            .status()
            .expect("git commit");
        test_git_cmd()
            .args(["remote", "add", "origin", bare.to_str().unwrap()])
            .current_dir(repo)
            .status()
            .expect("git remote add");
        test_git_cmd()
            .args(["push", "-u", "origin", "master"])
            .current_dir(repo)
            .status()
            .expect("git push");

        test_git_cmd()
            .args(["checkout", "-b", "main"])
            .current_dir(repo)
            .status()
            .expect("git checkout main");
        test_git_cmd()
            .args(["commit", "--allow-empty", "-m", "main commit"])
            .current_dir(repo)
            .status()
            .expect("git commit main");
        test_git_cmd()
            .args(["push", "-u", "origin", "main"])
            .current_dir(repo)
            .status()
            .expect("git push main");

        let result = consolidate_to_main(repo).await;
        assert!(result.is_ok(), "consolidate_to_main should succeed");

        let local_branches = {
            let output = test_git_cmd()
                .args(["branch"])
                .current_dir(repo)
                .output()
                .expect("git branch");
            String::from_utf8_lossy(&output.stdout).to_string()
        };
        assert!(local_branches.contains("main"), "main branch should exist");
        assert!(!local_branches.contains("master"), "master local branch should be deleted");
    }

    #[tokio::test]
    async fn test_rename_master_to_main_renames_and_deletes_remote_master() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        let bare = tmp.path().join("bare.git");
        test_git_cmd()
            .args(["init", "-q", "--bare", bare.to_str().unwrap()])
            .status()
            .expect("git init bare");
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .current_dir(repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(repo)
            .status()
            .expect("git commit");
        test_git_cmd()
            .args(["remote", "add", "origin", bare.to_str().unwrap()])
            .current_dir(repo)
            .status()
            .expect("git remote add");
        test_git_cmd()
            .args(["push", "-u", "origin", "master"])
            .current_dir(repo)
            .status()
            .expect("git push");

        let result = rename_master_to_main(repo).await;
        assert!(result.is_ok(), "rename_master_to_main should succeed");

        let current = {
            let output = test_git_cmd()
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(repo)
                .output()
                .expect("git rev-parse");
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };
        assert_eq!(current, "main", "should be on main branch after rename");
    }

    #[test]
    fn test_has_only_master_branch_detects_master_only() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        test_git_cmd()
            .args(["init", "-q"])
            .current_dir(repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(repo)
            .status()
            .expect("git commit");
        test_git_cmd()
            .args(["checkout", "-b", "master"])
            .current_dir(repo)
            .status()
            .expect("git checkout master");

        let result = has_only_master_branch(repo);
        assert!(result, "should detect master-only repo");
    }

    #[test]
    fn test_has_only_master_branch_ignores_main_and_master() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        test_git_cmd()
            .args(["init", "-q"])
            .current_dir(repo)
            .status()
            .expect("git init");
        test_git_cmd()
            .args(["checkout", "-b", "master"])
            .current_dir(repo)
            .status()
            .expect("git checkout master");
        test_git_cmd()
            .args(["branch", "main"])
            .current_dir(repo)
            .status()
            .expect("git branch main");

        let result = has_only_master_branch(repo);
        assert!(!result, "should not detect when both main and master exist");
    }

    #[tokio::test]
    async fn test_prune_other_default_branch_deletes_main_when_on_master() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        test_git_cmd()
            .args(["init", "-q", "-b", "master"])
            .current_dir(repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(repo)
            .status()
            .expect("git commit");
        test_git_cmd()
            .args(["checkout", "-b", "main"])
            .current_dir(repo)
            .status()
            .expect("git checkout main");
        test_git_cmd()
            .args(["checkout", "master"])
            .current_dir(repo)
            .status()
            .expect("git checkout master");

        prune_other_default_branch(repo).await;

        let local_branches = {
            let output = test_git_cmd()
                .args(["branch"])
                .current_dir(repo)
                .output()
                .expect("git branch");
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|s| s.trim_start_matches('*').trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        };
        assert!(local_branches.contains(&"master".to_string()), "master should still exist: {:?}", local_branches);
        assert!(!local_branches.contains(&"main".to_string()), "main should be deleted: {:?}", local_branches);
    }

    #[tokio::test]
    async fn test_prune_other_default_branch_deletes_master_when_on_main() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path();
        test_git_cmd()
            .args(["init", "-q", "-b", "main"])
            .current_dir(repo)
            .status()
            .expect("git init");
        std::fs::write(repo.join("file.txt"), "content").expect("write");
        test_git_cmd()
            .args(["add", "."])
            .current_dir(repo)
            .status()
            .expect("git add");
        test_git_cmd()
            .args(["commit", "-m", "init"])
            .current_dir(repo)
            .status()
            .expect("git commit");
        test_git_cmd()
            .args(["checkout", "-b", "master"])
            .current_dir(repo)
            .status()
            .expect("git checkout master");
        test_git_cmd()
            .args(["checkout", "main"])
            .current_dir(repo)
            .status()
            .expect("git checkout main");

        prune_other_default_branch(repo).await;

        let local_branches = {
            let output = test_git_cmd()
                .args(["branch"])
                .current_dir(repo)
                .output()
                .expect("git branch");
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|s| s.trim_start_matches('*').trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        };
        assert!(local_branches.contains(&"main".to_string()), "main should still exist: {:?}", local_branches);
        assert!(!local_branches.contains(&"master".to_string()), "master should be deleted: {:?}", local_branches);
    }
}
