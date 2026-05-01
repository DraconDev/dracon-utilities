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

fn load_secret(env_name: &str) -> Option<String> {
    if let Ok(key) = std::env::var(env_name) {
        if !key.is_empty() {
            return Some(key);
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
        &["push", remote_name, "HEAD"],
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
        .args(["repo", "create", repo_name, "--private"])
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
    let output = std::process::Command::new("curl")
        .args([
            "-s", "-w", "%{http_code}",
            "-X", "POST",
            api_endpoint,
            "-H", &format!("Authorization: Bearer {}", token),
            "-H", "Content-Type: application/json",
            "-d", &serde_json::json!({
                "name": repo_name,
                "private": true,
                "default_branch": "master"
            }).to_string(),
        ])
        .output()
        .with_context(|| "curl codeberg repo create failed")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let status_code = stdout.trim_end_matches(|c: char| !c.is_ascii_digit()).parse::<u16>().unwrap_or(0);
    let response_body = stdout.trim_end_matches(|c: char| c.is_ascii_digit());

    if status_code == 409 || status_code == 422 {
        return Ok(format!("git@codeberg.org:{}/{}.git", account, repo_name));
    }

    if !output.status.success() || !(200..=299).contains(&status_code) {
        anyhow::bail!("codeberg repo create failed ({}): {} {}", status_code, stderr.trim(), response_body);
    }

    Ok(format!("git@codeberg.org:{}/{}.git", account, repo_name))
}

pub(crate) fn auto_create_repo(config: &RemoteConfig, repo_name: &str) -> Result<String> {
    match config.auth_type {
        AuthType::GitHub => create_repo_on_github(&config.auto_create_account, repo_name),
        AuthType::GitLab => create_repo_on_gitlab(&config.auto_create_account, repo_name),
        AuthType::Codeberg => {
            let token = std::env::var(config.auto_create_token_var.as_deref().unwrap_or("CODEBERG_TOKEN"))
                .with_context(|| format!("missing env var {}", config.auto_create_token_var.as_deref().unwrap_or("CODEBERG_TOKEN")))?;
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

}
