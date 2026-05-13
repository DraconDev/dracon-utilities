use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::policy::{AuthType, RemoteConfig};
use crate::secrets::{load_secret, sync_secrets_dir};

/// Directory for visibility sync cache files.
fn visibility_cache_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/state/dracon/visibility-sync")
}

/// Path to the cache file for a given repo.
fn visibility_cache_path(repo_name: &str) -> PathBuf {
    visibility_cache_dir().join(format!("{}.last", repo_name))
}

/// Check whether the visibility cache is fresh (within `interval_hours`).
fn is_visibility_cache_fresh(repo_name: &str, interval_hours: u64) -> bool {
    let path = visibility_cache_path(repo_name);
    if !path.exists() {
        return false;
    }
    let Ok(content) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Ok(last_ts) = content.trim().parse::<u64>() else {
        return false;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let interval_secs = interval_hours.saturating_mul(3600);
    now.saturating_sub(last_ts) < interval_secs
}

/// Write the current timestamp to the visibility cache for a repo.
fn update_visibility_cache(repo_name: &str) {
    let dir = visibility_cache_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("⚠️ failed to create visibility cache dir {}: {}", dir.display(), e);
        return;
    }
    let path = visibility_cache_path(repo_name);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if let Err(e) = std::fs::write(&path, now.to_string()) {
        eprintln!("⚠️ failed to write visibility cache {}: {}", path.display(), e);
    }
}

/// Parse `owner/repo` from a GitHub remote URL.
/// Supports both SSH (`git@github.com:owner/repo.git`) and HTTPS (`https://github.com/owner/repo.git`).
pub(crate) fn parse_github_owner_repo(remote_url: &str) -> Option<(String, String)> {
    // SSH: git@github.com:owner/repo.git
    if let Some(colon) = remote_url.rfind(':') {
        let after_colon = &remote_url[colon + 1..];
        let clean = after_colon.strip_suffix(".git").unwrap_or(after_colon);
        if let Some(slash) = clean.find('/') {
            return Some((clean[..slash].to_string(), clean[slash + 1..].to_string()));
        }
    }
    // HTTPS: https://github.com/owner/repo.git
    if let Some(host_start) = remote_url.find("github.com/") {
        let after_host = &remote_url[host_start + 11..];
        let clean = after_host.strip_suffix(".git").unwrap_or(after_host);
        if let Some(slash) = clean.find('/') {
            return Some((clean[..slash].to_string(), clean[slash + 1..].to_string()));
        }
    }
    None
}

/// Query GitHub for the visibility of a repo using `gh api`.
/// Returns `true` if the repo is private, `false` if public.
/// On any error (gh not installed, no auth, network failure), returns `true` as the safe default.
fn get_github_visibility(owner: &str, repo: &str) -> bool {
    let output = match std::process::Command::new("gh")
        .args(["api", &format!("repos/{}/{}", owner, repo), "--jq", ".private"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("⚠️ gh api failed (is gh installed?): {}", e);
            return true;
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("⚠️ gh api failed: {}", stderr.trim());
        return true;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // "true" → private, "false" → public, anything else → safe default (private)
    stdout == "true"
}

/// Set GitLab repo visibility using `curl` with PRIVATE-TOKEN.
/// `private=true` means private, `private=false` means public.
fn set_gitlab_visibility(owner: &str, repo: &str, token: &str, private: bool) -> Result<()> {
    let visibility = if private { "private" } else { "public" };
    let url = format!(
        "https://gitlab.com/api/v4/projects/{}%2F{}",
        owner, repo
    );
    let output = std::process::Command::new("curl")
        .args([
            "-s", "-o", "/dev/null", "-w", "%{http_code}",
            "-H", &format!("PRIVATE-TOKEN: {}", token),
            "-X", "PUT",
            "-d", &format!("visibility={}", visibility),
            &url,
        ])
        .output()
        .with_context(|| "curl failed to run for GitLab visibility update")?;

    let code = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match code.as_str() {
        "200" => Ok(()),
        "401" => Err(anyhow::anyhow!("GitLab visibility update failed: unauthorized (invalid token)")),
        "404" => Err(anyhow::anyhow!("GitLab visibility update failed: repo not found")),
        _ => Err(anyhow::anyhow!("GitLab visibility update failed: HTTP {}", code)),
    }
}

/// Set Codeberg repo visibility using `curl` with Authorization token.
/// `private=true` means private, `private=false` means public.
fn set_codeberg_visibility(owner: &str, repo: &str, token: &str, private: bool) -> Result<()> {
    let url = format!("https://codeberg.org/api/v1/repos/{}/{}", owner, repo);
    let json = format!("{{\"private\":{}}}", private);
    let output = std::process::Command::new("curl")
        .args([
            "-s", "-o", "/dev/null", "-w", "%{http_code}",
            "-H", &format!("Authorization: token {}", token),
            "-H", "Content-Type: application/json",
            "-X", "PATCH",
            "-d", &json,
            &url,
        ])
        .output()
        .with_context(|| "curl failed to run for Codeberg visibility update")?;

    let code = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match code.as_str() {
        "200" => Ok(()),
        "401" => Err(anyhow::anyhow!("Codeberg visibility update failed: unauthorized (invalid token)")),
        "404" => Err(anyhow::anyhow!("Codeberg visibility update failed: repo not found")),
        _ => Err(anyhow::anyhow!("Codeberg visibility update failed: HTTP {}", code)),
    }
}

/// Query GitHub for the current visibility of the origin repo, then update
/// all configured mirrors (GitLab, Codeberg) to match.
///
/// This function is **non-fatal**: errors are logged but never propagated,
/// so a visibility sync failure will never break the git push pipeline.
pub(crate) fn sync_mirror_visibility(
    origin_url: &str,
    remotes: &[RemoteConfig],
    repo_name: &str,
    interval_hours: u64,
) {
    // Check cache first
    if is_visibility_cache_fresh(repo_name, interval_hours) {
        return;
    }

    let Some((owner, gh_repo)) = parse_github_owner_repo(origin_url) else {
        eprintln!("⚠️ could not parse GitHub owner/repo from origin URL: {}", origin_url);
        return;
    };

    let github_private = get_github_visibility(&owner, &gh_repo);
    let visibility_str = if github_private { "private" } else { "public" };

    if crate::policy::debug_enabled() {
        eprintln!(
            "🐛 GitHub repo {}/{} is {}",
            owner, gh_repo, visibility_str
        );
    }

    for remote in remotes {
        if remote.auth_type == AuthType::GitLab {
            let token_var = remote.auto_create_token_var.as_deref().unwrap_or("GITLAB_TOKEN");
            if let Some(token) = load_secret(token_var, &sync_secrets_dir()) {
                let resolved_name = remote.resolve_repo_name(repo_name);
                if let Err(e) = set_gitlab_visibility(&remote.auto_create_account, &resolved_name, &token, github_private) {
                    eprintln!("⚠️ failed to set GitLab visibility for {}: {}", resolved_name, e);
                } else if crate::policy::debug_enabled() {
                    eprintln!("🐛 set GitLab {}/{} to {}", remote.auto_create_account, resolved_name, visibility_str);
                }
            } else {
                eprintln!("⚠️ no GITLAB_TOKEN for visibility sync on {}", remote.name);
            }
        }

        if remote.auth_type == AuthType::Codeberg {
            let token_var = remote.auto_create_token_var.as_deref().unwrap_or("CODEBERG_TOKEN");
            if let Some(token) = load_secret(token_var, &sync_secrets_dir()) {
                let resolved_name = remote.resolve_repo_name(repo_name);
                if let Err(e) = set_codeberg_visibility(&remote.auto_create_account, &resolved_name, &token, github_private) {
                    eprintln!("⚠️ failed to set Codeberg visibility for {}: {}", resolved_name, e);
                } else if crate::policy::debug_enabled() {
                    eprintln!("🐛 set Codeberg {}/{} to {}", remote.auto_create_account, resolved_name, visibility_str);
                }
            } else {
                eprintln!("⚠️ no CODEBERG_TOKEN for visibility sync on {}", remote.name);
            }
        }
    }

    // Update cache even on partial failures — we don't want to hammer APIs
    // on every sync cycle when a token is permanently missing.
    update_visibility_cache(repo_name);
}

/// Check GitHub visibility at repo creation time and return whether the
/// repo should be created as private. If `sync_visibility` is disabled,
/// always returns `true` (private).
pub(crate) fn github_visibility_at_creation(
    owner: &str,
    repo_name: &str,
    sync_visibility: bool,
) -> bool {
    if !sync_visibility {
        return true;
    }
    get_github_visibility(owner, repo_name)
}

/// Update the `auto_create_account` and `--private` flag for GitHub repo creation
/// based on the visibility setting. When `sync_visibility` is true, queries GitHub
/// to determine if the repo already exists and what its visibility is.
///
/// This is a no-op wrapper around `create_repo_on_github` for the `sync_visibility=false`
/// case; the real value is when `sync_visibility=true` and we need to match existing
/// repo visibility on GitHub.
pub(crate) fn create_repo_on_github_with_visibility(
    account: &str,
    repo_name: &str,
    sync_visibility: bool,
) -> Result<String> {
    let private = if sync_visibility {
        get_github_visibility(account, repo_name)
    } else {
        true
    };

    let mut cmd = std::process::Command::new("gh");
    cmd.args(["repo", "create", repo_name]);
    if private {
        cmd.arg("--private");
    } else {
        cmd.arg("--public");
    }

    if let Some(token) = load_secret("GH_TOKEN", &sync_secrets_dir()) {
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

/// Same as `create_repo_on_gitlab` but with visibility control.
pub(crate) fn create_repo_on_gitlab_with_visibility(
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

    if let Some(token) = load_secret("GITLAB_TOKEN", &sync_secrets_dir()) {
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

/// Same as `create_repo_on_codeberg` but with visibility control.
pub(crate) async fn create_repo_on_codeberg_with_visibility(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_owner_repo_ssh() {
        let result = parse_github_owner_repo("git@github.com:DraconDev/my-repo.git");
        assert_eq!(result, Some(("DraconDev".to_string(), "my-repo".to_string())));
    }

    #[test]
    fn test_parse_github_owner_repo_https() {
        let result = parse_github_owner_repo("https://github.com/DraconDev/my-repo.git");
        assert_eq!(result, Some(("DraconDev".to_string(), "my-repo".to_string())));
    }

    #[test]
    fn test_parse_github_owner_repo_no_git_suffix() {
        let result = parse_github_owner_repo("git@github.com:DraconDev/my-repo");
        assert_eq!(result, Some(("DraconDev".to_string(), "my-repo".to_string())));
    }

    #[test]
    fn test_parse_github_owner_repo_invalid_url() {
        let result = parse_github_owner_repo("not-a-url");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_github_owner_repo_gitlab_url() {
        let result = parse_github_owner_repo("git@gitlab.com:someone/repo.git");
        // Should parse as (someone, repo) since the parser is generic enough
        assert_eq!(result, Some(("someone".to_string(), "repo".to_string())));
    }

    #[test]
    fn test_visibility_cache_not_fresh_when_missing() {
        let repo_name = "test_repo_that_should_not_exist_12345";
        assert!(!is_visibility_cache_fresh(repo_name, 24));
    }

    #[test]
    fn test_visibility_cache_fresh_when_recent() {
        let repo_name = "test_cache_fresh";
        update_visibility_cache(repo_name);
        assert!(is_visibility_cache_fresh(repo_name, 24));
        // Cleanup
        let _ = std::fs::remove_file(visibility_cache_path(repo_name));
    }

    #[test]
    fn test_visibility_cache_stale_when_old() {
        let repo_name = "test_cache_stale";
        let path = visibility_cache_path(repo_name);
        let old_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(25 * 3600);
        std::fs::create_dir_all(visibility_cache_dir()).unwrap();
        std::fs::write(&path, old_ts.to_string()).unwrap();
        assert!(!is_visibility_cache_fresh(repo_name, 24));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_visibility_cache_updates_timestamp() {
        let repo_name = "test_cache_update";
        let path = visibility_cache_path(repo_name);
        // Write old timestamp
        let old_ts = "1000";
        std::fs::create_dir_all(visibility_cache_dir()).unwrap();
        std::fs::write(&path, old_ts).unwrap();
        // Update
        update_visibility_cache(repo_name);
        let new_content = std::fs::read_to_string(&path).unwrap();
        let new_ts = new_content.trim().parse::<u64>().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(new_ts > 1000);
        assert!(new_ts <= now);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_github_visibility_at_creation_disabled() {
        // When sync_visibility is false, always private
        assert!(github_visibility_at_creation("DraconDev", "test", false));
    }

    #[test]
    fn test_get_github_visibility_returns_safe_default_on_error() {
        // With no gh installed (or in test env), should return true (private)
        let result = get_github_visibility("nonexistent-owner-12345", "nonexistent-repo-67890");
        assert!(result, "safe default should be private");
    }

    #[test]
    fn test_sync_mirror_visibility_skips_when_cache_fresh() {
        let repo_name = "test_skip_cached";
        update_visibility_cache(repo_name);
        // Should return immediately without error even with bad remotes
        let remotes = vec![RemoteConfig {
            name: "gitlab".to_string(),
            push_url: "git@gitlab.com:test/repo.git".to_string(),
            auto_create: false,
            auto_create_account: "test".to_string(),
            auth_type: AuthType::GitLab,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
            repo_name_map: Default::default(),
            force_push_when_behind: false,
        }];
        sync_mirror_visibility("git@github.com:DraconDev/test.git", &remotes, repo_name, 24);
        // If we got here without panicking, the cache skip worked
        let _ = std::fs::remove_file(visibility_cache_path(repo_name));
    }

    #[test]
    fn test_sync_mirror_visibility_handles_unparseable_origin() {
        let repo_name = "test_bad_origin";
        let remotes: Vec<RemoteConfig> = vec![];
        // Should not panic on unparseable URL
        sync_mirror_visibility("not-a-valid-url", &remotes, repo_name, 0);
        let _ = std::fs::remove_file(visibility_cache_path(repo_name));
    }

    #[test]
    fn test_parse_github_owner_repo_with_dots() {
        let result = parse_github_owner_repo("git@github.com:DraconDev/.dracon.git");
        assert_eq!(result, Some(("DraconDev".to_string(), ".dracon".to_string())));
    }

    #[test]
    fn test_parse_github_owner_repo_with_name_mapping() {
        let result = parse_github_owner_repo("https://github.com/my-org/some-repo.git");
        assert_eq!(result, Some(("my-org".to_string(), "some-repo".to_string())));
    }
}
