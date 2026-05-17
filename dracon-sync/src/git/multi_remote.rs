//! Multi-remote management — create, configure, and push to multiple git remotes.

use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;

use anyhow::{Context, Result};

use crate::helpers::is_repo_already_exists;
use crate::policy::{std_git_command, AuthType, RemoteConfig};

use super::{current_branch, git_ssh_hardening, is_push_rejected, is_safe_branch_name, load_secret, push_https_fallback, run_git_capture_output, run_git_with_timeout_env};

/// Configure a remote URL. Adds if missing, updates if URL differs.
pub(crate) fn ensure_remote(repo: &Path, name: &str, url: &str) -> Result<()> {
    let existing = get_remote_url(repo, name);
    match existing {
        Some(cur) if cur == url => Ok(()),
        Some(_) => {
            std_git_command()
                .args(["remote", "set-url", name, url])
                .current_dir(repo)
                .status()
                .with_context(|| {
                    format!("git remote set-url {} in {}", name, repo.display())
                })?;
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

/// Configure all remotes from policy for a given repo.
pub(crate) fn configure_all_remotes(repo: &Path, remotes: &[RemoteConfig], repo_name: &str) {
    for remote in remotes {
        let url = remote.resolve_push_url(repo_name);
        if let Err(e) = ensure_remote(repo, &remote.name, &url) {
            eprintln!(
                "⚠️ failed to configure remote {} for {}: {}",
                remote.name,
                repo.display(),
                e
            );
        }
    }
}

/// Push to all mirror remotes, auto-creating repos if configured.
pub(crate) async fn push_mirror_remotes(
    repo: &Path,
    remotes: &[RemoteConfig],
    timeout_secs: u64,
    retries: u32,
    private: bool,
) -> Vec<(String, Result<()>)> {
    let repo_name = repo
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    configure_all_remotes(repo, remotes, &repo_name);

    for (remote_name, create_result) in
        auto_create_all_remotes(remotes, &repo_name, private).await
    {
        match create_result {
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "⚠️ auto-create failed for {} on {}: {}",
                    repo_name, remote_name, e
                );
            }
        }
    }

    let all_remote_names: Vec<_> = remotes.iter().map(|r| r.name.as_str()).collect();
    if let Err(e) = remove_stale_remotes(repo, &all_remote_names) {
        eprintln!(
            "⚠️ failed to clean stale remotes for {}: {}",
            repo.display(),
            e
        );
    }

    push_to_all_remotes(repo, remotes, timeout_secs, retries).await
}

/// Get the URL for a given remote name.
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

/// List all remotes for a repo.
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

/// Remove stale remotes (anything not in the keep list, except origin).
pub(crate) fn remove_stale_remotes(repo: &Path, keep: &[&str]) -> Result<()> {
    let current = list_remotes(repo);
    let keep_set: HashSet<_> = keep.iter().collect();
    for remote in current {
        if remote == "origin" {
            continue;
        }
        if !keep_set.contains(&remote.as_str()) {
            std_git_command()
                .args(["remote", "remove", &remote])
                .current_dir(repo)
                .status()
                .with_context(|| {
                    format!("git remote remove {} in {}", remote, repo.display())
                })?;
        }
    }
    Ok(())
}

/// Push to a named remote with SSH hardening, HTTPS fallback, and retries.
pub(crate) async fn push_to_named_remote(
    repo: &Path,
    remote_name: &str,
    timeout_secs: u64,
    retries: u32,
    force_when_behind: bool,
) -> Result<()> {
    let branch = current_branch(repo).unwrap_or_else(|| "main".to_string());
    let refspec = format!("HEAD:refs/heads/{}", branch);
    let ssh_hardening = git_ssh_hardening();

    let attempt_ssh = run_git_with_timeout_env(
        repo,
        &["push", remote_name, &refspec],
        timeout_secs,
        &format!("push-to-{}", remote_name),
        &[
            ("GIT_SSH_COMMAND", &ssh_hardening),
            ("GIT_TERMINAL_PROMPT", "0"),
        ],
    )
    .await;

    if attempt_ssh.is_ok() {
        return Ok(());
    }

    let remote_url = get_remote_url(repo, remote_name)
        .ok_or_else(|| anyhow::anyhow!("remote {} not found", remote_name))?;

    if is_safe_branch_name(&branch) {
        let fallback_label = format!("push-to-{}", remote_name);
        push_https_fallback(repo, &remote_url, &refspec, timeout_secs, &fallback_label).await?;
    }

    let mut last_err = None;
    for attempt in 1..=retries.max(1) {
        match run_git_with_timeout_env(
            repo,
            &["push", remote_name, "HEAD"],
            timeout_secs,
            &format!("push-to-{}", remote_name),
            &[
                ("GIT_SSH_COMMAND", &ssh_hardening),
                ("GIT_TERMINAL_PROMPT", "0"),
            ],
        )
        .await
        {
            Ok(()) => return Ok(()),
            Err(e) => {
                let is_rejected = is_push_rejected(&e.to_string());
                if is_rejected && force_when_behind {
                    match diagnose_divergence(repo, remote_name, &branch).await {
                        Ok(Divergence::RemotePurelyBehind) => {
                            let force_result = run_git_with_timeout_env(
                                repo,
                                &[
                                    "push",
                                    "--force-with-lease",
                                    remote_name,
                                    &format!("HEAD:refs/heads/{}", branch),
                                ],
                                timeout_secs,
                                &format!("force-push-to-{}", remote_name),
                                &[
                                    ("GIT_SSH_COMMAND", &ssh_hardening),
                                    ("GIT_TERMINAL_PROMPT", "0"),
                                ],
                            )
                            .await;
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

/// Result of divergence diagnosis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Divergence {
    /// Remote is purely behind local (safe to force push).
    RemotePurelyBehind,
    /// Both sides have diverged.
    Divergent,
}

/// Check whether the remote is purely behind or truly diverged.
pub(crate) async fn diagnose_divergence(
    repo: &Path,
    remote_name: &str,
    branch: &str,
) -> Result<Divergence> {
    let local_head = run_git_capture_output(repo, &["rev-parse", "HEAD"], "rev-parse")?;
    let local_head = local_head.trim();
    let remote_ref = format!("refs/remotes/{}/{}", remote_name, branch);

    let rev_list_output = run_git_capture_output(
        repo,
        &[
            "rev-list",
            "--left-right",
            "--count",
            &format!("{}...{}", local_head, remote_ref),
        ],
        "rev-list",
    )?;

    let counts: Vec<&str> = rev_list_output.trim().split('\t').collect();
    if counts.len() != 2 {
        return Ok(Divergence::Divergent);
    }

    let local_ahead: u32 = match counts[0].parse() {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "⚠️ failed to parse local ahead count from rev-list output (\"{}\"): {}",
                counts[0], e
            );
            return Ok(Divergence::Divergent);
        }
    };
    let remote_ahead: u32 = match counts[1].parse() {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "⚠️ failed to parse remote ahead count from rev-list output (\"{}\"): {}",
                counts[1], e
            );
            return Ok(Divergence::Divergent);
        }
    };

    if remote_ahead == 0 && local_ahead > 0 {
        Ok(Divergence::RemotePurelyBehind)
    } else if remote_ahead > 0 {
        Ok(Divergence::Divergent)
    } else {
        Ok(Divergence::RemotePurelyBehind)
    }
}

/// Push to all remotes in priority order.
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
        let result = push_to_named_remote(
            repo,
            &remote.name,
            timeout_secs,
            retries,
            remote.force_push_when_behind,
        )
        .await;
        results.push((remote.name.clone(), result));
    }
    results
}

/// Create a private repo on GitHub using `gh` CLI.
pub(crate) fn create_repo_on_github(account: &str, repo_name: &str) -> Result<String> {
    let mut cmd = std::process::Command::new("gh");
    cmd.args(["repo", "create", repo_name, "--private"]);

    if let Some(token) = load_secret("GH_TOKEN") {
        cmd.env("GH_TOKEN", token);
    }

    let output = cmd.output().with_context(|| "gh repo create failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if is_repo_already_exists(&stderr) {
            return Ok(format!("git@github.com:{}/{}.git", account, repo_name));
        }
        anyhow::bail!("gh repo create failed: {}", stderr.trim());
    }

    Ok(format!("git@github.com:{}/{}.git", account, repo_name))
}

/// Create a repo on GitLab using `glab` CLI.
pub(crate) fn create_repo_on_gitlab(
    account: &str,
    repo_name: &str,
    private: bool,
) -> Result<String> {
    let mut cmd = std::process::Command::new("glab");
    cmd.args(["repo", "create", repo_name]);
    if private {
        cmd.arg("--private");
    } else {
        cmd.arg("--public");
    }

    if let Some(token) = load_secret("GITLAB_TOKEN") {
        cmd.env("GITLAB_TOKEN", token);
    }

    let output = cmd.output().with_context(|| "glab repo create failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if is_repo_already_exists(&stderr) {
            return Ok(format!("git@gitlab.com:{}/{}.git", account, repo_name));
        }
        anyhow::bail!("glab repo create failed: {}", stderr.trim());
    }

    Ok(format!("git@gitlab.com:{}/{}.git", account, repo_name))
}

/// Create a repo on Codeberg via REST API.
pub(crate) async fn create_repo_on_codeberg(
    token: &str,
    account: &str,
    repo_name: &str,
    api_endpoint: &str,
    private: bool,
) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post(api_endpoint)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "name": repo_name,
            "private": private,
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

/// Auto-create a repo on the given remote platform.
pub(crate) async fn auto_create_repo(
    config: &RemoteConfig,
    repo_name: &str,
    private: bool,
) -> Result<String> {
    match config.effective_auth_type() {
        AuthType::GitHub => create_repo_on_github(&config.auto_create_account, repo_name),
        AuthType::GitLab => {
            create_repo_on_gitlab(&config.auto_create_account, repo_name, private)
        }
        AuthType::Codeberg => {
            let token_var = config
                .auto_create_token_var
                .as_deref()
                .unwrap_or("CODEBERG_TOKEN");
            let token = load_secret(token_var)
                .with_context(|| format!("missing token for Codeberg (set {} env var or ~/.dracon/utilities/sync/secrets/*.env file)", token_var))?;
            let endpoint = config
                .api_endpoint
                .as_deref()
                .unwrap_or("https://codeberg.org/api/v1/user/repos");
            create_repo_on_codeberg(
                &token,
                &config.auto_create_account,
                repo_name,
                endpoint,
                private,
            )
            .await
        }
        AuthType::Generic => anyhow::bail!("Generic auth cannot auto-create repos"),
    }
}

/// Auto-create all configured remotes for a repo.
pub(crate) async fn auto_create_all_remotes(
    remotes: &[RemoteConfig],
    repo_name: &str,
    private: bool,
) -> Vec<(String, Result<String>)> {
    let mut results = Vec::new();
    for remote in remotes {
        if remote.auto_create {
            let resolved_name = remote.resolve_repo_name(repo_name);
            let result = auto_create_repo(remote, &resolved_name, private).await;
            results.push((remote.name.clone(), result));
        }
    }
    results
}
