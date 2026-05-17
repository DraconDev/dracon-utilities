//! Branch operations — current branch, main/master management, upstream tracking.

use std::path::Path;
use anyhow::{Context, Result};

use super::{has_origin_remote, has_tracking_upstream, is_safe_branch_name};

/// Get the current branch name from HEAD ref or git CLI.
pub(crate) fn current_branch(repo: &Path) -> Option<String> {
    let head_path = repo.join(".git").join("HEAD");
    if let Ok(content) = std::fs::read_to_string(&head_path) {
        let trimmed = content.trim();
        if let Some(ref_name) = trimmed.strip_prefix("ref: refs/heads/") {
            return Some(ref_name.to_string());
        }
    }
    crate::policy::std_git_command()
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

/// Whether the repo has a master branch but NOT a main branch.
pub(crate) fn has_only_master_branch(repo: &Path) -> bool {
    use crate::policy::std_git_command;
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

/// Whether the repo has BOTH main and master branches.
pub(crate) fn has_both_main_and_master(repo: &Path) -> bool {
    use crate::policy::std_git_command;
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

/// Consolidate to main: checkout main, delete local+remote master, push with upstream.
pub(crate) async fn consolidate_to_main(repo: &Path) -> Result<()> {
    use crate::policy::std_git_command;
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
        if let Err(e) = super::push_with_retries(repo, 60, 3, "consolidate-to-main").await {
            eprintln!("⚠️ failed to push main with upstream: {}", e);
        }
    }
    Ok(())
}

/// Rename local master to main and update remote tracking.
pub(crate) async fn rename_master_to_main(repo: &Path) -> Result<()> {
    use crate::policy::std_git_command;
    let branch = current_branch(repo).unwrap_or_else(|| "main".to_string());
    if branch == "master" {
        std_git_command()
            .args(["branch", "-m", "master", "main"])
            .current_dir(repo)
            .status()
            .with_context(|| format!("failed to rename master to main in {}", repo.display()))?;
    }
    if has_origin_remote(repo) {
        if let Err(e) = super::push_with_retries(repo, 60, 3, "rename-master-to-main").await {
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
    use crate::policy::std_git_command;
    let branch = current_branch(repo);
    let other = match branch.as_deref() {
        Some("master") => "main",
        Some("main") => "master",
        _ => return,
    };
    let other_str = other.to_string();
    let repo_has_origin = has_origin_remote(repo);
    let repo_b = repo.to_path_buf();
    let repo_c = repo_b.clone();
    let other_b = other_str.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || {
        std_git_command()
            .args(["branch", "-D", &other_b])
            .current_dir(&repo_b)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
    })
    .await
    {
        eprintln!("⚠️ failed to delete local {} branch: {}", other_str, e);
    }
    if repo_has_origin {
        let other_c = other_str.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || {
            std_git_command()
                .args(["push", "origin", "--delete", &other_c])
                .current_dir(&repo_c)
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

/// Check if a branch exists on the remote origin.
pub(crate) fn remote_branch_exists(repo: &Path, branch: &str) -> bool {
    use crate::policy::std_git_command;
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

/// Set the upstream tracking branch for a local branch.
pub(crate) fn set_upstream_to_branch(repo: &Path, branch: &str) -> Result<()> {
    use crate::policy::std_git_command;
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
