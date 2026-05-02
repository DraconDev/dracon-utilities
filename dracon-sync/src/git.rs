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
        Ok(()) => Ok(()),
        Err(e) => {
            let origin = origin_url(repo).unwrap_or_default();
            if let Some(https) = github_https_url(&origin) {
                let branch = current_branch(repo).unwrap_or_else(|| "master".to_string());
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

pub(crate) fn has_only_main_branch(repo: &Path) -> bool {
    let has_main = std_git_command()
        .args(["rev-parse", "--verify", "refs/heads/main"])
        .current_dir(repo)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !has_main {
        return false;
    }
    let has_master = std_git_command()
        .args(["rev-parse", "--verify", "refs/heads/master"])
        .current_dir(repo)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    has_main && !has_master
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

pub(crate) async fn consolidate_to_master(repo: &Path) -> Result<()> {
    let branch = current_branch(repo).unwrap_or_else(|| "master".to_string());
    if branch != "master" {
        std_git_command()
            .args(["checkout", "master"])
            .current_dir(repo)
            .status()
            .with_context(|| format!("failed to checkout master in {}", repo.display()))?;
    }
    // Delete local main if it exists
    if let Err(e) = std_git_command()
        .args(["branch", "-D", "main"])
        .current_dir(repo)
        .status()
    {
        eprintln!("⚠️ failed to delete local main branch: {}", e);
    }
    // Delete remote main if it exists
    if let Err(e) = std_git_command()
        .args(["push", "origin", "--delete", "main"])
        .current_dir(repo)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
    {
        eprintln!("⚠️ failed to delete remote main branch: {}", e);
    }
    // Ensure master has upstream tracking
    if has_origin_remote(repo) && !has_tracking_upstream(repo) {
        if let Err(e) =
            push_with_retries(repo, 60, 3, "consolidate-to-master").await
        {
            eprintln!("⚠️ failed to push master with upstream: {}", e);
        }
    }
    Ok(())
}

pub(crate) async fn rename_main_to_master(repo: &Path) -> Result<()> {
    let branch = current_branch(repo).unwrap_or_else(|| "main".to_string());
    if branch == "main" {
        std_git_command()
            .args(["branch", "-m", "main", "master"])
            .current_dir(repo)
            .status()
            .with_context(|| format!("failed to rename main to master in {}", repo.display()))?;
    }
    if has_origin_remote(repo) {
        if let Err(e) =
            push_with_retries(repo, 60, 3, "rename-main-to-master").await
        {
            eprintln!("⚠️ failed to push master to origin: {}", e);
        }
        if let Err(e) = std_git_command()
            .args(["push", "origin", "--delete", "main"])
            .current_dir(repo)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
        {
            eprintln!("⚠️ failed to delete remote main: {}", e);
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

pub(crate) fn detect_large_blobs_ahead(repo: &Path, min_bytes: u64) -> Result<Vec<(u64, String)>> {
    let timeout_secs = 60;
    
    let start = std::time::Instant::now();
    
    let rev_list = std_git_command()
        .args(["rev-list", "--objects", "@{u}..HEAD"])
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed rev-list in {}", repo.display()))?;
    if !rev_list.status.success() {
        return Ok(Vec::new());
    }
    
    if start.elapsed() > Duration::from_secs(timeout_secs) {
        eprintln!("⚠️ detect_large_blobs_ahead timed out during rev-list for {}", repo.display());
        return Ok(Vec::new());
    }

    let mut cat_file = std_git_command()
        .args(["cat-file", "--batch-check=%(objectname) %(objecttype) %(objectsize) %(rest)"])
        .current_dir(repo)
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
    if let Ok(val) = std::env::var(env_name) {
        if !val.is_empty() {
            return Some(val);
        }
    }
    let secrets_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dracon/utilities/sync/secrets");
    if let Ok(entries) = std::fs::read_dir(&secrets_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "env") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }
                        if let Some((key, value)) = line.split_once('=') {
                            if key.trim() == env_name {
                                let value = value.trim();
                                if !value.is_empty() {
                                    return Some(value.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
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

    for (remote_name, create_result) in auto_create_all_remotes(remotes, &repo_name) {
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
) -> Result<()> {
    let branch = current_branch(repo).unwrap_or_else(|| "master".to_string());
    let refspec = format!("HEAD:refs/heads/{}", branch);
    let ssh_hardening = "ssh -o ConnectTimeout=10 -o ConnectionAttempts=1 -o ServerAliveInterval=5 -o ServerAliveCountMax=2";

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
                last_err = Some(e);
                if attempt < retries.max(1) {
                    sleep(Duration::from_secs(attempt as u64)).await;
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("push to {} failed", remote_name)))
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
        let result = push_to_named_remote(repo, &remote.name, timeout_secs, retries).await;
        results.push((remote.name.clone(), result));
    }
    results
}

pub(crate) fn create_repo_on_github(account: &str, repo_name: &str) -> Result<String> {
    let output = std::process::Command::new("gh")
        .args(["repo", "create", repo_name, "--private"])
        .output()
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
    let output = std::process::Command::new("glab")
        .args(["repo", "create", repo_name, "--visibility", "private"])
        .output()
        .with_context(|| "glab repo create failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already exists") || stderr.contains("Name already exists") {
            return Ok(format!("git@gitlab.com:{}/{}.git", account, repo_name));
        }
        anyhow::bail!("glab repo create failed: {}", stderr.trim());
    }

    Ok(format!("git@gitlab.com:{}/{}.git", account, repo_name))
}

pub(crate) fn create_repo_on_codeberg(token: &str, account: &str, repo_name: &str, api_endpoint: &str) -> Result<String> {
    let runtime = tokio::runtime::Handle::current();
    let result = runtime.block_on(async {
        let client = reqwest::Client::new();
        let response = client
            .post(api_endpoint)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "name": repo_name,
                "private": true,
                "default_branch": "master"
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
    });
    result
}

pub(crate) fn auto_create_repo(config: &RemoteConfig, repo_name: &str) -> Result<String> {
    match config.auth_type {
        AuthType::GitHub => create_repo_on_github(&config.auto_create_account, repo_name),
        AuthType::GitLab => create_repo_on_gitlab(&config.auto_create_account, repo_name),
        AuthType::Codeberg => {
            let token_var = config.auto_create_token_var.as_deref().unwrap_or("CODEBERG_TOKEN");
            let token = load_secret(token_var)
                .with_context(|| format!("missing token for Codeberg (set {} env var or ~/.dracon/utilities/sync/secrets/*.env file)", token_var))?;
            let endpoint = config.api_endpoint.as_deref().unwrap_or("https://codeberg.org/api/v1/repos");
            create_repo_on_codeberg(&token, &config.auto_create_account, repo_name, endpoint)
        }
        AuthType::Generic => anyhow::bail!("Generic auth cannot auto-create repos"),
    }
}

pub(crate) fn auto_create_all_remotes(remotes: &[RemoteConfig], repo_name: &str) -> Vec<(String, Result<String>)> {
        let mut results = Vec::new();
        for remote in remotes {
            if remote.auto_create {
                let result = auto_create_repo(remote, repo_name);
                results.push((remote.name.clone(), result));
            }
        }
        results
    }
}

#[allow(dead_code)]
#[allow(unused_imports)]
mod tests {
    use super::*;

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
        std::env::set_var("TEST_LOAD_SECRET_TOKEN", tmp_val);
        let result = load_secret("TEST_LOAD_SECRET_TOKEN");
        std::env::remove_var("TEST_LOAD_SECRET_TOKEN");
        assert_eq!(result, Some(tmp_val.to_string()));
    }

    #[test]
    fn test_load_secret_empty_env_var() {
        std::env::set_var("TEST_LOAD_SECRET_EMPTY", "");
        let result = load_secret("TEST_LOAD_SECRET_EMPTY");
        std::env::remove_var("TEST_LOAD_SECRET_EMPTY");
        assert_eq!(result, None);
    }

    #[test]
    fn test_load_secret_missing() {
        assert_eq!(load_secret("TEST_NONEXISTENT_SECRET_VAR_XYZ"), None);
    }

    #[test]
    fn test_get_remote_url_nonexistent_remote() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
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
        std::process::Command::new("git")
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
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::process::Command::new("git")
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
        std::process::Command::new("git")
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
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::process::Command::new("git")
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
        std::process::Command::new("git")
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
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::process::Command::new("git")
            .args(["remote", "add", "origin", "git@github.com:Test/repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add origin");
        std::process::Command::new("git")
            .args(["remote", "add", "stale", "git@github.com:stale/repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add stale");

        super::remove_stale_remotes(&repo, &["github"]).expect("remove_stale_remotes");

        let remotes = multi_remote::list_remotes(&repo);
        assert!(remotes.contains(&"origin".to_string()), "origin must be preserved");
        assert!(!remotes.contains(&"stale".to_string()), "stale not in keep list, should be removed");
    }

    #[test]
    fn test_remove_stale_remotes_removes_nonkept() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::process::Command::new("git")
            .args(["remote", "add", "origin", "git@github.com:Test/repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add origin");
        std::process::Command::new("git")
            .args(["remote", "add", "mirror1", "git@mirror1.example.com:repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add mirror1");
        std::process::Command::new("git")
            .args(["remote", "add", "mirror2", "git@mirror2.example.com:repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add mirror2");

        super::remove_stale_remotes(&repo, &["mirror1"]).expect("remove_stale_remotes");

        let remotes = multi_remote::list_remotes(&repo);
        assert!(remotes.contains(&"origin".to_string()), "origin always preserved");
        assert!(remotes.contains(&"mirror1".to_string()), "kept remote mirror1 preserved");
        assert!(!remotes.contains(&"mirror2".to_string()), "non-kept remote mirror2 removed");
    }

    #[test]
    fn test_remove_stale_remotes_idempotent_when_empty() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::process::Command::new("git")
            .args(["remote", "add", "origin", "git@github.com:Test/repo.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add origin");

        super::remove_stale_remotes(&repo, &[]).expect("remove_stale_remotes with empty keep list");

        let remotes = multi_remote::list_remotes(&repo);
        assert_eq!(remotes, vec!["origin"]);
    }

}
