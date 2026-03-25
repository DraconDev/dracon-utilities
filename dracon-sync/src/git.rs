use anyhow::{Context, Result};
use dracon_git::{
    types::{DiffFile, FileStatus, RepoStatus},
    GitService,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::sleep;

use crate::exclude::{can_restore_entry, is_excluded_change_path, is_large_untracked, should_stage_entry};
use crate::policy::{git_binary, std_git_command, tokio_git_command, timestamp_secs};

pub(crate) fn discover_git_repos(roots: &[PathBuf], _excluded_dir_names: &BTreeSet<String>) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    for root in roots {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() && path.join(".git").exists() {
                    repos.push(path);
                }
            }
        }
    }
    repos
}

pub(crate) fn has_origin_remote(repo: &Path) -> bool {
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

pub(crate) async fn run_cmd_with_timeout(
    repo: &Path,
    program: &str,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
) -> Result<()> {
    let label = format!("{} {}", program, op_label);
    let child = TokioCommand::new(program)
        .args(args)
        .current_dir(repo)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
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
    let mut removed = 0usize;
    for path in staged {
        if !is_excluded_change_path(&path, excluded_dir_names) {
            continue;
        }
        let status = tokio_git_command()
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

pub(crate) async fn unstage_oversized_paths(repo: &Path, max_stage_file_bytes: u64) -> Result<usize> {
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
        let status = tokio_git_command()
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

pub(crate) fn current_branch(repo: &Path) -> Option<String> {
    std_git_command()
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

pub(crate) fn remote_branch_exists(repo: &Path, branch: &str) -> bool {
    std_git_command()
        .args(["show-ref", "--verify", "--quiet"])
        .arg(format!("refs/remotes/origin/{branch}"))
        .current_dir(repo)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) fn set_upstream_to_branch(repo: &Path, branch: &str) -> Result<()> {
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
    // Step 1: Get object IDs from commits ahead of upstream
    let rev_list = std_git_command()
        .args(["rev-list", "--objects", "@{u}..HEAD"])
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed rev-list in {}", repo.display()))?;
    if !rev_list.status.success() {
        return Ok(Vec::new());
    }

    // Step 2: Batch-check object types and sizes (no shell involved)
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

    // Step 3: Filter blobs > min_bytes in Rust (no shell, no awk)
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
    out.sort_by(|a, b| b.0.cmp(&a.0));
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

    // Fallback to deprecated git filter-branch
    eprintln!(
        "⚠️ git-filter-repo not found, using deprecated filter-branch. Install git-filter-repo for better performance."
    );
    let mut index_filter = String::from("git rm -r --cached --ignore-unmatch");
    for path in paths_to_remove {
        index_filter.push_str(" '");
        index_filter.push_str(&path.replace('\'', "'\\''"));
        index_filter.push('\'');
    }

    let rewrite = std_git_command()
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

pub(crate) async fn restore_paths(repo: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }

    // Prefer `git restore` (newer git). Fallback to `reset` + `checkout`.
    let mut args: Vec<String> = Vec::new();
    args.push("restore".to_string());
    args.push("--staged".to_string());
    args.push("--worktree".to_string());
    args.push("--".to_string());
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
    let _ = run_git_with_timeout(repo, &reset_ref, 30, "reset").await;

    let mut checkout: Vec<String> = Vec::new();
    checkout.push("checkout".to_string());
    checkout.push("--".to_string());
    checkout.extend(paths.iter().cloned());
    let checkout_ref: Vec<&str> = checkout.iter().map(|s| s.as_str()).collect();
    run_git_with_timeout(repo, &checkout_ref, 30, "checkout").await
}
