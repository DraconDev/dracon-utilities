pub(crate) mod branch_ops;
pub(crate) mod diff;
pub(crate) mod discovery;
pub(crate) mod orphan;
pub(crate) mod push;
pub(crate) mod remotes;
pub(crate) mod safety;

#[cfg(test)]
pub(crate) static PATH_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

#[allow(dead_code)]
pub(crate) fn load_secret(env_name: &str) -> Option<String> {
    crate::secrets::load_secret(env_name)
}

pub(crate) fn git_ssh_hardening() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    format!(
        "ssh -o BatchMode=yes -F {home}/.dracon/secrets/ssh/config -o ConnectTimeout=10 -o ConnectionAttempts=1 -o ServerAliveInterval=5 -o ServerAliveCountMax=2"
    )
}

pub(crate) fn origin_url(repo: &Path) -> Option<String> {
    get_remote_url(repo, "origin")
}

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
    if !origin.contains("github.com") {
        return None;
    }
    let stripped = strip_url_credentials(origin);
    if stripped.starts_with("git@") {
        Some(stripped.replace("git@github.com:", "https://github.com/"))
    } else {
        Some(stripped)
    }
}

pub(crate) fn gitlab_https_url(origin: &str) -> Option<String> {
    if !origin.contains("gitlab") {
        return None;
    }
    let stripped = strip_url_credentials(origin);
    if stripped.starts_with("git@") {
        if let Some(parsed) = stripped.strip_prefix("git@") {
            if let Some(rest) = parsed.strip_prefix("gitlab.") {
                if let Some((host_and_path,)) = rest.split_once(':') {
                    return Some(format!("https://gitlab.{}", host_and_path));
                }
            }
        }
        None
    } else {
        Some(stripped)
    }
}

pub(crate) fn codeberg_https_url(origin: &str) -> Option<String> {
    if !origin.contains("codeberg") && !origin.contains(" forgejo") {
        return None;
    }
    let stripped = strip_url_credentials(origin);
    if stripped.starts_with("git@") {
        if let Some(parsed) = stripped.strip_prefix("git@") {
            if let Some(rest) = parsed.strip_prefix("codeberg.org:") {
                return Some(format!("https://codeberg.org/{}", rest));
            }
        }
        None
    } else {
        Some(stripped)
    }
}

pub(crate) fn current_branch(repo: &Path) -> Option<String> {
    std_git_command()
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()?
        .stdout
        .split_whitespace()
        .next()
        .map(|s| s.to_string())
}

pub(crate) fn has_only_master_branch(repo: &Path) -> bool {
    let branches: Vec<String> = std_git_command()
        .args(["branch", "--format=%(refname:short)"])
        .current_dir(repo)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(|l| l.to_string()).collect())
        .unwrap_or_default();

    branches.len() == 1 && branches[0] == "master"
}

pub(crate) fn has_both_main_and_master(repo: &Path) -> bool {
    let branches: Vec<String> = std_git_command()
        .args(["branch", "--format=%(refname:short)"])
        .current_dir(repo)
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.to_string())
                .collect()
        })
        .unwrap_or_default();

    branches.contains(&"main".to_string()) && branches.contains(&"master".to_string())
}

pub(crate) fn remote_branch_exists(repo: &Path, branch: &str) -> bool {
    std_git_command()
        .args(["ls-remote", "--heads", "--quiet", "origin", branch])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub(crate) fn set_upstream_to_branch(repo: &Path, branch: &str) -> Result<()> {
    std_git_command()
        .args(["branch", "--set-upstream-to=origin/{}", branch])
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed to set upstream for {}", branch))?;
    Ok(())
}

pub(crate) fn get_remote_url(repo: &Path, name: &str) -> Option<String> {
    std_git_command()
        .args(["remote", "get-url", name])
        .current_dir(repo)
        .output()
        .ok()?
        .stdout
        .split_whitespace()
        .map(|s| s.to_string())
        .next()
}

pub(crate) fn list_remotes(repo: &Path) -> Vec<String> {
    std_git_command()
        .args(["remote", "--format=%(refname)"])
        .current_dir(repo)
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.strip_prefix("refs/remotes/").map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

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

#[cfg(test)]
pub(crate) fn acquire_path_lock() -> parking_lot::MutexGuard<'static, ()> {
    PATH_LOCK.lock()
}

// Re-exports from submodules
pub(crate) use branch_ops::{consolidate_to_main, prune_other_default_branch, rename_master_to_main, rewrite_ahead_paths};
pub(crate) use diff::{cli_diff_entries, git_diff_head_files, git_name_status_entries, parse_name_status_line, staged_paths};
pub(crate) use discovery::{discover_git_repos, discover_git_repos_recursive};
pub(crate) use orphan::{detect_orphan_origin, fix_orphan_origin};
pub(crate) use push::{push_mirror_remotes, push_to_all_remotes, push_to_named_remote, push_with_retries, push_with_transport_fallbacks};
pub(crate) use remotes::{auto_create_all_remotes, auto_create_repo, configure_all_remotes, create_repo_on_codeberg, create_repo_on_github, create_repo_on_gitlab, remove_stale_remotes};
pub(crate) use safety::{has_origin_remote, has_tracking_upstream, is_git_worktree_file, is_merge_in_progress, is_rebase_in_progress, is_cherry_pick_in_progress, is_safe_branch_name, is_safe_git_path};