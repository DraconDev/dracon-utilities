//! Nix flake version update via PR.
//!
//! After a version bump, if `nix_auto_update` is enabled and the repo has a `flake.nix`,
//! this module updates the flake version field and creates a PR via `gh`.

use std::path::Path;
use std::process::Command;

use anyhow::Context;
use crate::git::git_ssh_hardening;

const FLAKE_NIX: &str = "flake.nix";

pub fn has_flake_nix(repo: &Path) -> bool {
    repo.join(FLAKE_NIX).is_file()
}

pub fn update_flake_version(repo: &Path, new_version: &str) -> anyhow::Result<bool> {
    let flake_path = repo.join(FLAKE_NIX);
    let content = std::fs::read_to_string(&flake_path)?;

    let updated = update_version_in_flake_nix(&content, new_version)?;
    if updated == content {
        return Ok(false);
    }

    std::fs::write(&flake_path, updated)?;
    Ok(true)
}

fn update_version_in_flake_nix(content: &str, new_version: &str) -> anyhow::Result<String> {
    let mut result = String::with_capacity(content.len());
    let mut changed = false;

    let mut in_package = false;
    let mut in_build_rust_package = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            in_build_rust_package = false;
        } else if trimmed.ends_with("= rustPlatform.buildRustPackage {")
            || trimmed.ends_with("= pkgs.rustPlatform.buildRustPackage {") {
            in_build_rust_package = true;
        } else if trimmed.ends_with("};") {
            in_build_rust_package = false;
        } else if trimmed.starts_with(char::is_alphabetic) && trimmed.contains(" = ") && !trimmed.contains("{") {
            in_build_rust_package = false;
        }

        if (in_package || in_build_rust_package) && line.contains("version = \"") {
            if let Some(start_idx) = line.find("version = \"") {
                let after_quote = start_idx + 10; // skip "version = ""
                if let Some(_) = line[after_quote..].find('"') {
                    result.push_str(&line[..start_idx]);
                    result.push_str("version = \"");
                    result.push_str(new_version);
                    result.push_str("\"");
                    result.push('\n');
                    changed = true;
                    continue;
                }
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    if !changed {
        return Ok(content.to_string());
    }
    Ok(result)
}

pub async fn create_flake_pr(repo: &Path, new_version: &str) -> anyhow::Result<crate::release::ReleaseStep> {
    let repo_name = extract_repo_name(repo)?;
    let default_branch = detect_default_branch(repo).unwrap_or_else(|| "main".to_string());

    let branch_name = format!("chore/update-flake-v{}", new_version);
    let branch_name_check = branch_name.clone();
    let repo_name_for_check = repo_name.clone();
    let title = format!("chore: update flake.nix to v{}", new_version);
    let body = format!(
        "Update flake.nix version field to v{} after release bump.\n\n\
         Auto-created by dracon-sync.",
        new_version
    );

    let check = tokio::task::spawn_blocking(move || {
        Command::new("gh")
            .args(["pr", "list", "--repo", &repo_name_for_check, "--head", &branch_name_check, "--json", "number"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output()
    })
    .await?;

    if let Ok(output) = check {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if json.as_array().map(|arr| !arr.is_empty()).unwrap_or(false) {
                return Ok(crate::release::ReleaseStep::Skipped(format!(
                    "flake PR branch '{}' already exists",
                    branch_name
                )));
            }
        }
    }

    let commit_msg = format!("chore: update flake.nix to v{}\n\nAuto-commit by dracon-sync", new_version);

    run_git_for_nix_pr(repo, &branch_name, &commit_msg).await?;

    let result = tokio::task::spawn_blocking(move || {
        Command::new("gh")
            .args([
                "pr", "create",
                "--repo", &repo_name,
                "--base", &default_branch,
                "--head", &branch_name,
                "--title", &title,
                "--body", &body,
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
    })
    .await?;

    match result {
        Ok(output) if output.status.success() => {
            let pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            eprintln!("📄 Created flake PR: {}", pr_url);
            Ok(crate::release::ReleaseStep::NixFlakePRCreated(pr_url))
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No commit") || stderr.contains("everything up-to-date") {
                Ok(crate::release::ReleaseStep::Skipped("flake.nix already at latest version".to_string()))
            } else {
                Ok(crate::release::ReleaseStep::Failed {
                    step: "gh pr create".to_string(),
                    error: stderr.trim().to_string(),
                })
            }
        }
        Err(e) => Ok(crate::release::ReleaseStep::Failed {
            step: "gh pr create".to_string(),
            error: e.to_string(),
        }),
    }
}

async fn run_git_for_nix_pr(repo: &Path, branch_name: &str, commit_msg: &str) -> anyhow::Result<()> {
    use crate::git::run_git_with_timeout;

    run_git_with_timeout(repo, &["checkout", "-b", branch_name], 30, "nix-pr-branch").await?;

    run_git_with_timeout(repo, &["add", "flake.nix"], 30, "nix-pr-add").await?;

    match run_git_with_timeout(
        repo,
        &["commit", "-m", commit_msg],
        30,
        "nix-pr-commit",
    ).await {
        Ok(_) => {}
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("nothing to commit") || msg.contains("no changes") {
                return Err(anyhow::anyhow!("no changes to commit for flake.nix"));
            }
            return Err(e);
        }
    }

    let ssh_cmd = git_ssh_hardening();
    let env = [
        ("GIT_SSH_COMMAND", ssh_cmd.as_str()),
        ("GIT_TERMINAL_PROMPT", "0"),
    ];

    use crate::git::run_git_with_timeout_env;
    run_git_with_timeout_env(repo, &["push", "-u", "origin", branch_name], 120, "nix-pr-push", &env).await?;

    // Return to the previous branch so the sync cycle continues normally
    run_git_with_timeout(repo, &["checkout", "-"], 30, "nix-pr-return").await?;

    Ok(())
}

fn extract_repo_name(repo: &Path) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .context("failed to get origin URL")?;

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if url.starts_with("git@") {
        Ok(url.strip_prefix("git@")
            .and_then(|s| s.split_once(':'))
            .map(|(_, path)| path.trim_end_matches(".git").to_string())
            .unwrap_or_else(|| url.clone()))
    } else if url.starts_with("https://") {
        Ok(url.trim_start_matches("https://")
            .trim_end_matches(".git")
            .split('/')
            .skip(1)
            .collect::<Vec<_>>()
            .join("/"))
    } else {
        Ok(url.clone())
    }
}

fn detect_default_branch(repo: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    let ref_name = String::from_utf8_lossy(&output.stdout).trim();
    ref_name.strip_prefix("refs/remotes/origin/").map(String::from)
}

#[allow(dead_code)]
pub fn flake_has_hardcoded_version(flake_content: &str) -> bool {
    flake_content.contains("version = \"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_version_in_flake_nix_basic() {
        let content = r#"{
  description = "My app";

  packages.x86_64-linux.default = pkgs.rustPlatform.buildRustPackage {
    pname = "my-app";
    version = "1.0.0";
    src = ./.;
  };
}"#;
        let updated = update_version_in_flake_nix(content, "1.1.0").unwrap();
        assert!(updated.contains(r#"version = "1.1.0""#));
        assert!(!updated.contains("version = \"1.0.0\""));
    }

    #[test]
    fn test_update_version_in_flake_nix_no_package_section() {
        let content = r#"{
  description = "No package here";
}"#;
        let updated = update_version_in_flake_nix(content, "2.0.0").unwrap();
        assert_eq!(updated, content);
    }

    #[test]
    fn test_update_version_in_flake_nix_tiles_style() {
        let content = r#"{
  tiles = rustPlatform.buildRustPackage {
    pname = "tiles";
    version = "14.0.0";
    src = ./.;
    cargoLock = {
      lockFile = ./Cargo.lock;
    };
  };
}"#;
        let updated = update_version_in_flake_nix(content, "15.0.0").unwrap();
        assert!(updated.contains(r#"version = "15.0.0""#));
        assert!(!updated.contains("version = \"14.0.0\""));
    }

    #[test]
    fn test_update_version_in_flake_nix_multiple_packages() {
        let content = r#"{
  packages.x86_64-linux.default = rustPlatform.buildRustPackage {
    pname = "my-app";
    version = "0.1.0";
    src = ./.;
  };
}"#;
        let updated = update_version_in_flake_nix(content, "0.2.0").unwrap();
        assert!(updated.contains(r#"version = "0.2.0""#));
    }

    #[test]
    fn test_has_flake_nix_false_for_no_file() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(!has_flake_nix(dir.path()));
    }

    #[test]
    fn test_has_flake_nix_true_for_file() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("flake.nix"), "{}").unwrap();
        assert!(has_flake_nix(dir.path()));
    }

    #[test]
    fn test_flake_has_hardcoded_version_true() {
        assert!(flake_has_hardcoded_version(r#"version = "1.0.0""#));
    }

    #[test]
    fn test_flake_has_hardcoded_version_false() {
        assert!(!flake_has_hardcoded_version(r#"name = "tiles""#));
    }
}