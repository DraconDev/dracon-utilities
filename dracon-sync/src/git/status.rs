//! Repository status checks — origin, upstream, conflict state, readiness.

use std::path::Path;

use super::current_branch;

/// Check whether an `origin` remote exists via config or git CLI.
pub(crate) fn has_origin_remote(repo: &Path) -> bool {
    let config_path = repo.join(".git").join("config");
    if let Ok(config) = std::fs::read_to_string(&config_path) {
        return config
            .lines()
            .any(|line| line.trim() == "[remote \"origin\"]");
    }
    crate::policy::std_git_command()
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check whether the current branch has a configured upstream.
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
    crate::policy::std_git_command()
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Whether a rebase operation is in progress.
pub(crate) fn is_rebase_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("rebase-merge").exists()
        || repo.join(".git").join("rebase-apply").exists()
}

/// Whether a merge operation is in progress.
pub(crate) fn is_merge_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("MERGE_HEAD").exists()
}

/// Whether a cherry-pick operation is in progress.
pub(crate) fn is_cherry_pick_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("CHERRY_PICK_HEAD").exists()
}

/// Check if a repository is ready for operations (has valid HEAD with commits).
pub(crate) fn is_repo_ready(repo: &Path) -> bool {
    let head = repo.join(".git").join("HEAD");
    if !head.exists() {
        return false;
    }
    if let Ok(content) = std::fs::read_to_string(&head) {
        if content.trim().is_empty() {
            return false;
        }
    } else {
        return false;
    }
    let git_bin = std::env::var("DRACON_SYNC_GIT_BIN").unwrap_or_else(|_| "git".to_string());
    let output = std::process::Command::new(&git_bin)
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok();
    match output {
        Some(o) => {
            if !o.status.success() {
                return false;
            }
            let hash = String::from_utf8_lossy(&o.stdout).trim().to_string();
            !hash.is_empty()
        }
        None => false,
    }
}
