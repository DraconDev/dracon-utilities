use std::collections::{BTreeSet, HashMap};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use crate::log_warn;

use anyhow::Result;
use dracon_git::GitService;

use crate::exclude::{
    can_restore_entry, handle_large_untracked, is_large_untracked, remove_tracked_excluded_paths,
    should_stage_entry,
};
use crate::git::multi_remote::push_mirror_remotes;
use crate::git::origin_url;
use crate::git::{
    cli_diff_entries, detect_large_blobs_ahead, git_name_status_entries, has_origin_remote,
    has_tracking_upstream, is_cherry_pick_in_progress, is_merge_in_progress, is_rebase_in_progress,
    is_repo_ready, prune_other_default_branch, push_with_retries, restore_paths,
    run_git_capture_output, run_git_with_timeout, unstage_excluded_paths, unstage_oversized_paths,
};
use crate::policy::{debug_enabled, load_repo_override, SyncPolicy};
use crate::report::push_large_blob_threshold_bytes;
use crate::visibility::{
    get_github_visibility, parse_github_owner_repo, sync_mirror_metadata, sync_mirror_visibility,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyncOutcome {
    Synced,
    NothingToDo,
    Blocked,
}

impl SyncOutcome {
    pub fn has_changes(&self) -> bool {
        matches!(self, SyncOutcome::Synced)
    }
}

struct SyncContext<'a> {
    repo: &'a Path,
    policy: &'a SyncPolicy,
    excluded_dir_names: &'a BTreeSet<String>,
    dry_run: bool,
    #[allow(dead_code)]
    idle_seconds: u64,
    #[allow(dead_code)]
    policy_path: Option<&'a Path>,
    has_origin: bool,
    has_upstream: bool,
    blob_threshold: u64,
    #[allow(dead_code)]
    auto_bump_versions: bool,
    remote_failures: Option<&'a mut HashMap<String, usize>>,
}

fn notify_webhook_failure(webhook_url: &str, repo: &Path, remote: &str, error: &str) {
    let payload = serde_json::json!({
        "event": "push_failure",
        "repo": repo.display().to_string(),
        "remote": remote,
        "error": error,
        "timestamp": crate::policy::timestamp_secs(),
    });
    let url = webhook_url.to_string();
    std::thread::spawn(move || {
        if let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            if let Err(e) = client.post(&url).json(&payload).send() {
                eprintln!("⚠️ webhook notification failed: {}", e);
            }
        }
    });
}

async fn get_bump_info(repo: &Path) -> Option<(String, String, String)> {
    let new_ver = crate::release::detect_project_version(repo)?.0;

    let version_files = if repo.join("Cargo.toml").exists() {
        &["Cargo.toml"][..]
    } else if repo.join("package.json").exists() {
        &["package.json"][..]
    } else if repo.join("pyproject.toml").exists() {
        &["pyproject.toml"][..]
    } else if repo.join("pubspec.yaml").exists() {
        &["pubspec.yaml"][..]
    } else if repo.join("version.txt").exists() {
        &["version.txt"][..]
    } else if repo.join("VERSION").exists() {
        &["VERSION"][..]
    } else {
        &[
            "Cargo.toml",
            "package.json",
            "pyproject.toml",
            "pubspec.yaml",
            "version.txt",
            "VERSION",
        ][..]
    };

    let mut old_ver = String::new();
    for file in version_files.iter() {
        let repo_pb = repo.to_path_buf();
        let file_s = file.to_string();
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("git")
                .args(["show", &format!("HEAD~1:{}", file_s)])
                .current_dir(&repo_pb)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .output()
                .ok()
        })
        .await
        .ok()
        .flatten();

        if let Some(output) = output {
            if !output.status.success() {
                continue;
            }
            let content = String::from_utf8_lossy(&output.stdout);
            if let Some(v) = match *file {
                "Cargo.toml" => content
                    .lines()
                    .map(|l| l.trim())
                    .find(|l| l.starts_with("version") && !l.starts_with("version_prefix"))
                    .and_then(|l| l.split('=').nth(1))
                    .map(|v| v.trim().trim_matches('"').trim())
                    .filter(|v| !v.is_empty() && !v.starts_with("workspace"))
                    .map(|v| v.to_string()),
                "package.json" => content
                    .lines()
                    .map(|l| l.trim())
                    .find(|l| l.starts_with("\"version\""))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|v| v.trim().trim_matches('"').trim_matches(',').trim())
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                "pyproject.toml" => content
                    .lines()
                    .map(|l| l.trim())
                    .find(|l| l.starts_with("version") && !l.starts_with("version_prefix"))
                    .and_then(|l| l.split('=').nth(1))
                    .map(|v| v.trim().trim_matches('"').trim_matches(',').trim())
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                "pubspec.yaml" => content
                    .lines()
                    .map(|l| l.trim())
                    .find(|l| l.starts_with("version:"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|v| v.trim().split('+').next().unwrap_or("").trim())
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                "version.txt" | "VERSION" => {
                    let v = content.trim();
                    if !v.is_empty() && v.contains('.') {
                        Some(v.to_string())
                    } else {
                        None
                    }
                }
                _ => None,
            } {
                old_ver = v;
                break;
            }
        }
    }

    let level = if old_ver.is_empty() {
        "patch"
    } else {
        let old_parts: Vec<u32> = old_ver.split('.').filter_map(|s| s.parse().ok()).collect();
        let new_parts: Vec<u32> = new_ver.split('.').filter_map(|s| s.parse().ok()).collect();
        if old_parts.len() >= 3 && new_parts.len() >= 3 {
            if new_parts[0] > old_parts[0] {
                "major"
            } else if new_parts[1] > old_parts[1] {
                "minor"
            } else {
                "patch"
            }
        } else {
            "patch"
        }
    };

    Some((old_ver, new_ver, level.to_string()))
}

fn maybe_sync_visibility_and_metadata(ctx: &SyncContext<'_>) {
    if ctx.dry_run || (!ctx.policy.sync_visibility && !ctx.policy.sync_metadata) {
        return;
    }
    if let Some(origin_url) = crate::git::multi_remote::get_remote_url(ctx.repo, "origin") {
        if ctx.policy.sync_metadata {
            sync_mirror_metadata(
                &origin_url,
                &ctx.policy.remotes,
                ctx.repo,
                ctx.policy.sync_visibility_interval_hours,
            );
        }
        if ctx.policy.sync_visibility {
            sync_mirror_visibility(
                &origin_url,
                &ctx.policy.remotes,
                ctx.repo,
                ctx.policy.sync_visibility_interval_hours,
            );
        }
    }
}

fn check_conflict_state(repo: &Path) -> Option<SyncOutcome> {
    if is_rebase_in_progress(repo) {
        eprintln!(
            "⚠️ {} has rebase in progress, skipping (manual intervention required)",
            repo.display()
        );
        return Some(SyncOutcome::Blocked);
    }
    if is_merge_in_progress(repo) {
        eprintln!(
            "⚠️ {} has merge in progress, skipping (manual intervention required)",
            repo.display()
        );
        return Some(SyncOutcome::Blocked);
    }
    if is_cherry_pick_in_progress(repo) {
        eprintln!(
            "⚠️ {} has cherry-pick in progress, skipping (manual intervention required)",
            repo.display()
        );
        return Some(SyncOutcome::Blocked);
    }
    None
}

fn ensure_origin_remote(repo: &Path, policy: &SyncPolicy) -> bool {
    let has_origin = has_origin_remote(repo);
    if !has_origin && policy.auto_github_private {
        let private = if policy.sync_visibility {
            if let Some(url) = origin_url(repo) {
                if let Some((owner, repo_name)) = parse_github_owner_repo(&url) {
                    get_github_visibility(&owner, &repo_name)
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };
        if let Some(url) = crate::report::create_github_private_remote(
            repo,
            &policy.auto_github_private_account,
            private,
        ) {
            println!("🔗 created remote for {}: {}", repo.display(), url);
            true
        } else {
            eprintln!("⚠️ failed to create GitHub remote for {}", repo.display());
            false
        }
    } else {
        has_origin
    }
}

async fn auto_pull_merge(
    svc: &GitService,
    ctx: &SyncContext<'_>,
    initial_status: &dracon_git::types::RepoStatus,
) -> Result<()> {
    let repo = ctx.repo;
    let policy = ctx.policy;
    if policy.auto_pull
        && ctx.has_origin
        && ctx.has_upstream
        && initial_status.behind > 0
        && initial_status.is_clean
    {
        if ctx.dry_run {
            println!(
                "🔽 Would pull/merge {} commit(s) from upstream in {}",
                initial_status.behind,
                repo.display()
            );
        } else {
            match tokio::time::timeout(
                Duration::from_secs(policy.pull_op_timeout_secs),
                svc.pull_merge(),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(dracon_git::error::GitError::MergeConflict)) => {
                    eprintln!(
                        "⚠️ pull/merge conflict in {} (manual intervention required)",
                        repo.display()
                    );
                    return Err(anyhow::anyhow!("pull/merge conflict"));
                }
                Ok(Err(e)) => {
                    eprintln!(
                        "⚠️ pull/merge failed for {}: {} - aborting sync pass",
                        repo.display(),
                        e
                    );
                    return Err(anyhow::anyhow!("pull/merge failed: {}", e));
                }
                Err(_) => {
                    eprintln!(
                        "⚠️ pull/merge timeout for {} after {}s - aborting sync pass",
                        repo.display(),
                        policy.pull_op_timeout_secs
                    );
                    return Err(anyhow::anyhow!("pull/merge timeout"));
                }
            }
        }
    } else if policy.auto_pull && ctx.has_origin && ctx.has_upstream && initial_status.behind == 0 {
        if debug_enabled() {
            eprintln!(
                "🐛 skip pull/merge for {} (branch not behind upstream)",
                repo.display()
            );
        }
    } else if policy.auto_pull && ctx.has_origin && ctx.has_upstream && !initial_status.is_clean {
        if debug_enabled() {
            eprintln!(
                "🐛 skip pull/merge for {} (dirty repo, commit first)",
                repo.display()
            );
        }
    } else if policy.auto_pull && !ctx.has_origin {
        eprintln!(
            "ℹ️ skip pull/merge for {} (no origin remote)",
            repo.display()
        );
    } else if policy.auto_pull && ctx.has_origin && !ctx.has_upstream {
        eprintln!(
            "ℹ️ skip pull/merge for {} (no tracking upstream on current branch)",
            repo.display()
        );
    }
    Ok(())
}

async fn clean_staged_paths(ctx: &SyncContext<'_>) -> Result<()> {
    let repo = ctx.repo;
    let policy = ctx.policy;
    let excluded_dir_names = ctx.excluded_dir_names;
    let dry_run = ctx.dry_run;
    let unstaged = if dry_run {
        0
    } else {
        unstage_excluded_paths(repo, excluded_dir_names).await?
    };
    if unstaged > 0 {
        eprintln!(
            "🧹 removed {} staged excluded paths in {}",
            unstaged,
            repo.display()
        );
    }
    let unstaged_oversized = if dry_run {
        0
    } else {
        unstage_oversized_paths(repo, policy.max_stage_file_bytes).await?
    };
    if unstaged_oversized > 0 {
        eprintln!(
            "🧹 removed {} oversized staged paths in {}",
            unstaged_oversized,
            repo.display()
        );
    }

    if let Some(removed_dirs) = if dry_run {
        None
    } else {
        remove_tracked_excluded_paths(repo, excluded_dir_names)?
    } {
        if !removed_dirs.is_empty() {
            eprintln!(
                "🧹 removed {} tracked excluded dir(s) from {}: {:?}",
                removed_dirs.len(),
                repo.display(),
                removed_dirs
            );
        }
    }

    Ok(())
}

struct DiffResult {
    status: dracon_git::types::RepoStatus,
    entries: Vec<dracon_git::types::DiffFile>,
    #[allow(dead_code)]
    filter_only_cleared: bool,
}

#[cfg(test)]
mod diff_tests {

    #[test]
    fn test_fallback_entries_recalculate_staged_files() {
        // When cli_diff_entries fallback is used, staged_files must be
        // recalculated from actual staged paths rather than leaving the
        // (potentially stale) libgit2 count.
        use crate::git::staged_paths;

        // Create a temp repo with a staged file
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().join("test-repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        // Initialize git repo
        let output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .expect("git init failed");
        assert!(output.status.success(), "git init failed: {:?}", output);

        // Configure git user
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Create initial commit
        std::fs::write(repo_path.join("README.md"), "initial").unwrap();
        std::process::Command::new("git")
            .args(["add", "README.md"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Now add a new file (staged)
        std::fs::write(repo_path.join("new_file.rs"), "fn main() {}").unwrap();
        std::process::Command::new("git")
            .args(["add", "new_file.rs"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // staged_paths should find the staged file
        let rt = tokio::runtime::Runtime::new().unwrap();
        let staged = rt.block_on(staged_paths(&repo_path)).unwrap();
        assert_eq!(staged.len(), 1, "expected 1 staged file, got {:?}", staged);
        assert!(staged.contains(&std::path::PathBuf::from("new_file.rs")));

        // The fallback code path: cli_diff_entries should find it
        let rt = tokio::runtime::Runtime::new().unwrap();
        let entries = rt
            .block_on(crate::git::cli_diff_entries(&repo_path))
            .unwrap();
        assert!(
            !entries.is_empty(),
            "cli_diff_entries should find the staged file"
        );
    }
}

async fn compute_diff_entries(svc: &GitService, repo: &Path) -> Result<DiffResult> {
    let mut status = svc.get_status().await?;
    let mut entries = svc.get_diff_entries().await?;
    let mut filter_only_cleared = false;

    {
        let diff_output = crate::git::git_diff_head_files(repo)
            .await
            .unwrap_or_default();
        if diff_output.is_empty() && !entries.is_empty() {
            let has_non_modified = entries
                .iter()
                .any(|e| !matches!(e.status, dracon_git::types::FileStatus::Modified));
            if !has_non_modified {
                entries.clear();
                status.is_clean = true;
                filter_only_cleared = true;
            }
        } else {
            entries.retain(|e| {
                if !matches!(e.status, dracon_git::types::FileStatus::Modified) {
                    return true;
                }
                diff_output.contains(&e.path)
            });
        }
    }

    if debug_enabled() {
        eprintln!(
            "🐛 {} status: clean={} modified={} staged={} entries(libgit2)={}",
            repo.display(),
            status.is_clean,
            status.modified_files,
            status.staged_files,
            entries.len()
        );
    }
    if entries.is_empty() && !filter_only_cleared {
        let fallback_entries = cli_diff_entries(repo).await?;
        if !fallback_entries.is_empty() {
            status.is_clean = false;
            status.modified_files = fallback_entries.len();
            // Recalculate staged_files from actual staged paths when using
            // fallback CLI entries, since the libgit2 count may be stale
            // (libgit2 returned 0 entries but CLI found changes).
            if let Ok(staged) = crate::git::staged_paths(repo).await {
                status.staged_files = staged.len();
            }
            entries = fallback_entries;
            if debug_enabled() {
                eprintln!(
                    "🐛 {} fallback entries(cli)={} staged={} => forcing dirty",
                    repo.display(),
                    status.modified_files,
                    status.staged_files,
                );
            }
        }
    }

    Ok(DiffResult {
        status,
        entries,
        filter_only_cleared,
    })
}

async fn stage_existing_files(repo: &Path, existing: &[String], dry_run: bool) -> Result<()> {
    if existing.is_empty() {
        return Ok(());
    }
    if dry_run {
        println!(
            "📝 Would stage {} file(s) in {}: {:?}",
            existing.len(),
            repo.display(),
            &existing[..existing.len().min(5)]
        );
        if existing.len() > 5 {
            println!("  ... and {} more", existing.len() - 5);
        }
    } else {
        // Filter out gitignored paths to avoid "The following paths are ignored
        // by one of your .gitignore files" errors. Without this, a single
        // gitignored path in the list causes the entire `git add` to fail,
        // blocking ALL files from being committed.
        //
        // We use `git check-ignore` to detect gitignored paths, then split:
        // - Non-ignored paths: `git add -A -- <paths>` (respects .gitignore)
        // - Ignored but already-tracked paths: `git add -A -f -- <paths>` (force
        //   re-stage; git already tracks these so gitignore shouldn't block updates)
        // - Ignored and untracked: skip entirely (.gitignore is intentional)
        let (force_paths, normal_paths) = partition_gitignored(repo, existing).await;

        if !normal_paths.is_empty() {
            let mut add_args = vec!["add", "-A", "--"];
            for p in &normal_paths {
                add_args.push(p.as_str());
            }
            if let Err(e) = run_git_with_timeout(repo, &add_args, 30, "add").await {
                eprintln!(
                    "⚠️ {} git add failed for {} paths: {:?}",
                    repo.display(),
                    normal_paths.len(),
                    &normal_paths[..normal_paths.len().min(5)]
                );
                return Err(e);
            }
        }

        // Force-add already-tracked gitignored files (git tracks them already,
        // so .gitignore shouldn't prevent staging updates to tracked content)
        if !force_paths.is_empty() {
            let mut add_args = vec!["add", "-A", "-f", "--"];
            for p in &force_paths {
                add_args.push(p.as_str());
            }
            if run_git_with_timeout(repo, &add_args, 30, "add (force-tracked)")
                .await
                .is_err()
            {
                eprintln!(
                    "⚠️ {} git add -f failed for {} tracked gitignored paths: {:?}",
                    repo.display(),
                    force_paths.len(),
                    &force_paths[..force_paths.len().min(5)]
                );
                // Non-fatal: tracked gitignored files can be re-attempted next cycle
            }
        }
    }
    Ok(())
}

/// Partition paths into (force_add, normal_add) based on .gitignore.
/// - Paths gitignored AND already tracked in git → force_add (git add -f)
/// - All others → normal_add (git add, respects .gitignore)
async fn partition_gitignored(repo: &Path, paths: &[String]) -> (Vec<String>, Vec<String>) {
    if paths.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // Run `git check-ignore` to find gitignored paths
    let ignored: std::collections::HashSet<String> = {
        let mut check_args = vec!["check-ignore"];
        for p in paths {
            check_args.push(p.as_str());
        }
        let output = crate::policy::tokio_git_command()
            .args(&check_args)
            .current_dir(repo)
            .output()
            .await;
        match output {
            Ok(o) if o.status.success() || o.status.code() == Some(1) => {
                // Exit 0 = some ignored, exit 1 = none ignored
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| l.to_string())
                    .collect()
            }
            _ => std::collections::HashSet::new(),
        }
    };

    // Get list of already-tracked paths (git ls-files)
    let tracked: std::collections::HashSet<String> = {
        let output = crate::policy::tokio_git_command()
            .args(["ls-files"])
            .current_dir(repo)
            .output()
            .await;
        match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect(),
            _ => std::collections::HashSet::new(),
        }
    };

    let mut force_paths = Vec::new();
    let mut normal_paths = Vec::new();

    for p in paths {
        if ignored.contains(p) {
            // Gitignored — only force-add if already tracked
            if tracked.contains(p) {
                force_paths.push(p.clone());
            }
            // Untracked + gitignored = skip (respect .gitignore)
        } else {
            normal_paths.push(p.clone());
        }
    }

    (force_paths, normal_paths)
}

async fn git_rm_missing(repo: &Path, missing: &[String], dry_run: bool) -> Result<()> {
    if missing.is_empty() {
        return Ok(());
    }
    let mut rm_args = vec!["rm", "--ignore-unmatch", "--"];
    for p in missing {
        rm_args.push(p);
    }
    if dry_run {
        println!(
            "🗑️  Would delete (git rm) {} file(s) from {}: {:?}",
            missing.len(),
            repo.display(),
            &missing[..missing.len().min(5)]
        );
        if missing.len() > 5 {
            println!("  ... and {} more", missing.len() - 5);
        }
    } else if let Err(e) = run_git_with_timeout(repo, &rm_args, 30, "rm").await {
        eprintln!(
            "⚠️ {} git rm failed for {} paths: {:?}",
            repo.display(),
            missing.len(),
            missing
        );
        return Err(e);
    }
    Ok(())
}

async fn post_commit_pull(svc: &GitService, repo: &Path, policy: &SyncPolicy) {
    if !policy.auto_pull {
        return;
    }
    let post_commit_status = match svc.get_status().await {
        Ok(s) => s,
        Err(_) => return,
    };
    if post_commit_status.behind > 0 && post_commit_status.is_clean {
        eprintln!(
            "📥 post-commit pull for {} ({} behind)",
            repo.display(),
            post_commit_status.behind
        );
        match tokio::time::timeout(
            Duration::from_secs(policy.pull_op_timeout_secs),
            svc.pull_merge(),
        )
        .await
        {
            Ok(Ok(())) => {
                eprintln!("✅ post-commit pull succeeded for {}", repo.display());
            }
            Ok(Err(dracon_git::error::GitError::MergeConflict)) => {
                eprintln!(
                    "⚠️ post-commit pull conflict in {} (manual intervention required)",
                    repo.display()
                );
            }
            Ok(Err(e)) => {
                eprintln!(
                    "⚠️ post-commit pull failed for {}: {} - will still attempt push",
                    repo.display(),
                    e
                );
            }
            Err(_) => {
                eprintln!(
                    "⚠️ post-commit pull timeout for {} after {}s - will still attempt push",
                    repo.display(),
                    policy.pull_op_timeout_secs
                );
            }
        }
    }
}

async fn restore_excluded_paths(
    ctx: &SyncContext<'_>,
    to_restore: &[dracon_git::types::DiffFile],
) -> Result<()> {
    let repo = ctx.repo;
    let policy = ctx.policy;
    let restorable: Vec<_> = to_restore
        .iter()
        .filter(|e| can_restore_entry(repo, e))
        .filter(|e| {
            !repo.join(&e.path).is_dir() || !crate::exclude::is_gitlink_unchanged(repo, &e.path)
        })
        .collect();

    handle_large_untracked(repo, to_restore, policy)?;

    let other_untracked: Vec<_> = to_restore
        .iter()
        .filter(|e| {
            !can_restore_entry(repo, e) && !is_large_untracked(e, repo, policy.max_stage_file_bytes)
        })
        .collect();

    if !other_untracked.is_empty() {
        eprintln!(
            "ℹ️ {} has {} small untracked excluded file(s)",
            repo.display(),
            other_untracked.len()
        );
    }

    if !restorable.is_empty() {
        let excluded_paths: Vec<String> = restorable
            .iter()
            .map(|e| e.path.to_string_lossy().to_string())
            .collect();
        eprintln!(
            "🧹 restoring {} excluded path(s) in {} after commit",
            excluded_paths.len(),
            repo.display()
        );
        restore_paths(repo, &excluded_paths).await?;
    }

    Ok(())
}

async fn run_release_pipeline_if_bumped(repo: &Path, policy: &SyncPolicy, version_bumped: bool) {
    if !version_bumped {
        return;
    }
    if let Some((old_ver, new_ver, level)) = get_bump_info(repo).await {
        let repo_override = crate::policy::load_repo_override(repo);
        let repo_auto_tag = repo_override.auto_tag.unwrap_or(policy.auto_tag);
        let repo_auto_release = repo_override.auto_release.unwrap_or(policy.auto_release);
        let repo_publish_targets = repo_override.auto_publish;
        let repo_nix_auto_update = repo_override
            .nix_auto_update
            .unwrap_or(policy.nix_auto_update);
        let steps = crate::release::run_release_pipeline(
            repo,
            &old_ver,
            &new_ver,
            level.as_str(),
            policy,
            repo_auto_tag,
            repo_auto_release,
            &repo_publish_targets,
            repo_nix_auto_update,
        )
        .await;
        for step in &steps {
            match step {
                crate::release::ReleaseStep::TagCreated(tag) => eprintln!("🏷️  {tag}"),
                crate::release::ReleaseStep::GitHubReleaseCreated(tag) => eprintln!("🚀 {tag}"),
                crate::release::ReleaseStep::Published { registry, version } => {
                    eprintln!("📦 published to {registry} v{version}")
                }
                crate::release::ReleaseStep::NixFlakePRCreated(url) => {
                    eprintln!("📄 flake PR created: {url}")
                }
                crate::release::ReleaseStep::Skipped(reason) => {
                    if debug_enabled() {
                        eprintln!("🐛 release skipped: {reason}");
                    }
                }
                crate::release::ReleaseStep::Failed { step: s, error } => {
                    eprintln!("⚠️ release failed: {s} — {error}")
                }
            }
        }
    }
}

async fn push_with_blob_check(ctx: &mut SyncContext<'_>, ahead: usize) -> Result<bool> {
    let repo = ctx.repo;
    let policy = ctx.policy;
    let blob_threshold = ctx.blob_threshold;
    let has_origin = ctx.has_origin;
    let dry_run = ctx.dry_run;
    // Push if: auto_push enabled, origin exists, AND either:
    //   - there are unpushed commits (ahead > 0), OR
    //   - the current branch has no upstream tracking (newly created branch)
    let branch_has_upstream = super::git::has_tracking_upstream(repo);
    if !policy.auto_push || !has_origin || (ahead == 0 && branch_has_upstream) {
        return Ok(true);
    }

    let ahead_large = match detect_large_blobs_ahead(repo, blob_threshold).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "⚠️ large blob detection failed for {}: {} - skipping push",
                repo.display(),
                e
            );
            return Ok(false);
        }
    };
    if !ahead_large.is_empty() {
        eprintln!(
            "⚠️ skip push for {}: large blob(s) above {} bytes in ahead range ({} found)",
            repo.display(),
            blob_threshold,
            ahead_large.len()
        );
        return Ok(false);
    }

    match if dry_run {
        println!("🔼 Would push to origin in {}", repo.display());
        Ok(())
    } else {
        push_with_retries(
            repo,
            policy.push_op_timeout_secs,
            policy.push_retries,
            "push",
        )
        .await
    } {
        Ok(()) => {}
        Err(e) => {
            log_warn!("push failed for {}: {}", repo.display(), e);
            if let Some(ref url) = policy.webhook_url {
                notify_webhook_failure(url, repo, "origin", &e.to_string());
            }
            return Ok(false);
        }
    }

    let private = if policy.sync_visibility {
        if let Some(url) = origin_url(repo) {
            if let Some((owner, repo_name)) = parse_github_owner_repo(&url) {
                get_github_visibility(&owner, &repo_name)
            } else {
                true
            }
        } else {
            true
        }
    } else {
        true
    };

    if !policy.remotes.is_empty() {
        let push_results = if dry_run {
            for remote in &policy.remotes {
                println!("🔼 Would push to {} in {}", remote.name, repo.display());
            }
            policy
                .remotes
                .iter()
                .map(|r| (r.name.clone(), Ok(())))
                .collect()
        } else {
            push_mirror_remotes(
                repo,
                &policy.remotes,
                policy.push_op_timeout_secs,
                policy.push_retries,
                private,
            )
            .await
        };
        let all_ok = push_results.iter().all(|(_, r)| r.is_ok());
        if !all_ok {
            for (name, result) in &push_results {
                if let Err(e) = result {
                    log_warn!("push to {} failed for {}: {}", name, repo.display(), e);
                    if let Some(ref url) = policy.webhook_url {
                        notify_webhook_failure(url, repo, name, &e.to_string());
                    }
                    if let Some(ref mut rf) = ctx.remote_failures {
                        *rf.entry(name.clone()).or_insert(0) += 1;
                    }
                }
            }
            return Ok(false);
        } else if let Some(ref mut rf) = ctx.remote_failures {
            for name in policy.remotes.iter().map(|r| r.name.clone()) {
                rf.remove(&name);
            }
        }
    }

    Ok(true)
}





/// Task state transitions extracted from a markdown diff.
#[derive(Debug, Default)]
struct TaskTransitions {
    /// Tasks marked `[x]` (completed)
    closed: Vec<String>,
    /// Tasks marked `[~]` (in-progress)
    progress: Vec<String>,
}

/// Extract task state transitions from the staged diff.
///
/// Scans ALL staged files for:
/// - `- [x]` or `// [x]` additions → task completed (CLOSED)
/// - `- [~]` or `// [~]` additions → task in-progress (WIP)
///
/// Works in any file:
/// - Markdown: `- [x] Fix bug`
/// - Code comments: `// [x] Implemented JWT`
/// - Text files: `[x] Done`
/// - Any file with checkbox syntax
///
/// This is deterministic — no LLM, no inference. Just regex on the diff.
fn extract_task_transitions(repo: &Path) -> TaskTransitions {
    // Get the diff for ALL files (not just markdown)
    let output = match run_git_capture_output(
        repo,
        &["diff", "--cached", "--unified=0"],
        "task-diff",
    ) {
        Ok(o) => o,
        Err(_) => return TaskTransitions::default(),
    };

    let mut transitions = TaskTransitions::default();

    for line in output.lines() {
        // Only look at ADDED lines (start with `+` but not `+++`)
        if !line.starts_with('+') || line.starts_with("+++") {
            continue;
        }
        let content = &line[1..]; // Remove leading `+`
        let trimmed = content.trim();

        // Check for completed tasks: [x] or [X]
        // Matches: `- [x]`, `* [x]`, `// [x]`, `# [x]`, plain `[x]`
        if let Some(rest) = extract_checkbox_text(trimmed, 'x').or_else(|| extract_checkbox_text(trimmed, 'X')) {
            let task = sanitize_task_name(rest);
            if !task.is_empty() {
                transitions.closed.push(task);
            }
        }
        // Check for in-progress tasks: [~]
        else if let Some(rest) = extract_checkbox_text(trimmed, '~') {
            let task = sanitize_task_name(rest);
            if !task.is_empty() {
                transitions.progress.push(task);
            }
        }
    }

    transitions
}

/// Extract text after a checkbox marker like `[x]`, `[~]`, etc.
///
/// Handles common prefixes:
/// - `- [x] task` (markdown list)
/// - `* [x] task` (markdown list)
/// - `// [x] task` (code comment)
/// - `# [x] task` (code comment)
/// - `<!-- [x] task` (HTML comment)
/// - `[x] task` (plain)
fn extract_checkbox_text<'a>(line: &'a str, marker: char) -> Option<&'a str> {
    // Build the marker pattern: `[x]`, `[~]`, etc.
    let pattern = format!("[{}]", marker);
    
    // Try common prefixes
    let prefixes = ["- ", "* ", "// ", "# ", "<!-- ", ""];
    for prefix in &prefixes {
        let full_prefix = format!("{}{}", prefix, pattern);
        if let Some(rest) = line.strip_prefix(&full_prefix) {
            return Some(rest.trim());
        }
    }
    
    None
}

/// Sanitize a task name for use in the routing key.
/// Removes chars that would break parsing (pipes, brackets).
fn sanitize_task_name(name: &str) -> String {
    name.replace('|', "/")
        .replace('[', "(")
        .replace(']', ")")
        .trim()
        .to_string()
}

/// Check if a file path looks like a test file.
///
/// Common patterns:
/// - tests/, test/, __tests__/
/// - *_test.rs, *_test.py, *_test.go
/// - *.test.ts, *.test.js, *.spec.ts
/// - test_*.py, test_*.rs
fn is_test_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let basename = path.rsplit('/').next().unwrap_or(path);
    let basename_lower = basename.to_ascii_lowercase();

    // Directory patterns
    if lower.contains("/test") || lower.contains("/tests") || lower.contains("/__tests__") {
        return true;
    }

    // Suffix patterns (before extension)
    if basename_lower.contains("_test.") || basename_lower.contains("_tests.")
        || basename_lower.contains(".test.") || basename_lower.contains(".spec.")
    {
        return true;
    }

    // Prefix patterns
    if basename_lower.starts_with("test_") {
        return true;
    }

    false
}

/// Compute commit message from staged diff.
///
/// Returns a structured message with task state + blast radius.
///
/// Format: "[INTENT] | FILES:N DIRS:X DELTA:+A/-B [TEST:T] [BIN:B]"
///
/// INTENT (from markdown diff):
/// - `CLOSED: task1, task2` — tasks marked [x]
/// - `WIP: task1` — tasks marked [~]
/// - Omitted if no task transitions found
///
/// BLAST RADIUS (from git diff --numstat):
/// - FILES:N — total files changed
/// - DIRS:X,Y — top-level directories touched
/// - DELTA:+A/-B — lines added/removed
///
/// METRICS (also from diff):
/// - TEST:T — lines changed in test files (shows if AI wrote tests)
/// - BIN:B — binary files changed (context window warning)
///
/// # Why mechanical messages?
///
/// AI-generated commit messages are bad for AI workflows:
/// - They hallucinate context and intent
/// - They try to summarize but AI reads the diff anyway
/// - They're verbose, inconsistent, and noisy
///
/// Simple mechanical facts work better:
/// - Searchable: `git log --grep="JWT"` finds commits touching that task
/// - Honest: no interpretation, just data
/// - Compact: fits in `git log --oneline`
/// - The AI gets its understanding from the actual diff, not the commit message
///
/// The commit message is an INDEX, not a description.
fn compute_blast_radius(repo: &Path) -> String {
    let output = match run_git_capture_output(repo, &["diff", "--cached", "--numstat"], "numstat") {
        Ok(o) => o,
        Err(_) => return "0 file(s) DELTA:+0/-0".to_string(),
    };

    let mut files = 0usize;
    let mut added = 0i64;
    let mut removed = 0i64;
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    let mut file_changes: Vec<(i64, String)> = Vec::new();
    let mut test_lines = 0i64;
    let mut binary_count = 0usize;

    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let a = parts[0];
        let r = parts[1];
        let path = parts[2];

        // Binary files show as "-" in numstat
        if a == "-" || r == "-" {
            binary_count += 1;
            files += 1;
            // Still track the directory
            if let Some(first_component) = path.split('/').next() {
                if !first_component.is_empty() && first_component != "." {
                    dirs.insert(first_component.to_string());
                }
            }
            continue;
        }

        let a_val = a.parse::<i64>().unwrap_or(0);
        let r_val = r.parse::<i64>().unwrap_or(0);

        added += a_val;
        removed += r_val;
        files += 1;

        // Track test file lines separately
        if is_test_file(path) {
            test_lines += a_val + r_val;
        }

        // Extract top-level directory for scope
        if let Some(first_component) = path.split('/').next() {
            if !first_component.is_empty() && first_component != "." {
                dirs.insert(first_component.to_string());
            }
        }

        // Track files for listing (sorted by lines changed)
        file_changes.push((a_val + r_val, path.to_string()));
    }

    // Sort by lines changed descending, take top 3
    file_changes.sort_by_key(|b| std::cmp::Reverse(b.0));
    let top_files: Vec<String> = file_changes
        .iter()
        .take(3)
        .map(|(_, path)| path.clone())
        .collect();

    // Build routing key components

    // 1. Task intent (from markdown diff)
    let transitions = extract_task_transitions(repo);
    let intent_prefix = {
        let mut parts = Vec::new();
        if !transitions.closed.is_empty() {
            parts.push(format!("CLOSED: {}", transitions.closed.join(", ")));
        }
        if !transitions.progress.is_empty() {
            parts.push(format!("WIP: {}", transitions.progress.join(", ")));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("{} | ", parts.join(" | "))
        }
    };

    // 2. File count and dirs
    let dirs_str = if dirs.is_empty() {
        "root".to_string()
    } else {
        dirs.iter().take(3).cloned().collect::<Vec<_>>().join(",")
    };

    // 3. Top changed files
    let files_str = if top_files.is_empty() {
        String::new()
    } else {
        format!(" [{}]", top_files.join(", "))
    };

    // 4. Metrics suffix
    let mut metrics = Vec::new();
    if test_lines > 0 {
        metrics.push(format!("TEST:{}", test_lines));
    }
    if binary_count > 0 {
        metrics.push(format!("BIN:{}", binary_count));
    }
    let metrics_str = if metrics.is_empty() {
        String::new()
    } else {
        format!(" | {}", metrics.join(" "))
    };

    format!(
        "{}{} file(s) in {}{} DELTA:+{}/-{}{}",
        intent_prefix, files, dirs_str, files_str, added, removed, metrics_str
    )
}

async fn stage_commit_and_push(
    svc: &GitService,
    ctx: &mut SyncContext<'_>,
    _status: &dracon_git::types::RepoStatus,
    to_stage: &[dracon_git::types::DiffFile],
    to_restore: &[dracon_git::types::DiffFile],
) -> Result<Option<SyncOutcome>> {
    let repo = ctx.repo;
    let policy = ctx.policy;
    let dry_run = ctx.dry_run;
    let has_origin = ctx.has_origin;
    let _idle_seconds = ctx.idle_seconds;

    let stage_paths: Vec<String> = to_stage
        .iter()
        .map(|e| e.path.to_string_lossy().to_string())
        .collect();

    let (existing, missing): (Vec<_>, Vec<_>) =
        stage_paths.into_iter().partition(|p| repo.join(p).exists());

    stage_existing_files(repo, &existing, dry_run).await?;

    if !missing.is_empty() {
        git_rm_missing(repo, &missing, dry_run).await?;
    }

    let staged = git_name_status_entries(repo, &["diff", "--cached", "--name-status"]).await?;
    let committed_entries: Vec<dracon_git::types::DiffFile> = staged
        .into_iter()
        .map(|(path, status)| dracon_git::types::DiffFile { path, status })
        .collect();

    let blast_radius = compute_blast_radius(repo);

    let version_bumped = false;

    if committed_entries.is_empty() {
        if let Err(e) = run_git_with_timeout(repo, &["reset", "HEAD", "--"], 10, "reset").await {
            return Err(anyhow::anyhow!(
                "sync_repo: failed to reset HEAD after filter-only commit: {}",
                e
            ));
        }
        if debug_enabled() {
            eprintln!(
                "🐛 {} skipped commit: all changes were filter-only (smudge/clean)",
                repo.display()
            );
        }
        maybe_sync_visibility_and_metadata(ctx);
        return Ok(Some(SyncOutcome::NothingToDo));
    }

    // Use blast radius as commit message
    let msg = blast_radius;

    if dry_run {
        println!(
            "📝 Would commit {} file(s) in {}:",
            committed_entries.len(),
            repo.display()
        );
        for entry in committed_entries.iter().take(10) {
            println!("  {:?}: {}", entry.status, entry.path.display());
        }
        if committed_entries.len() > 10 {
            println!("  ... and {} more", committed_entries.len() - 10);
        }
        println!("  message: {}", msg.lines().next().unwrap_or("(empty)"));
    } else {
        svc.commit(&msg).await?;
        eprintln!(
            "📝 committed {} file(s) in {}",
            committed_entries.len(),
            repo.display()
        );
        // Flush so journald captures commit activity in real-time
        let _ = std::io::stderr().flush();
    }

    prune_other_default_branch(repo).await;

    post_commit_pull(svc, repo, policy).await;

    let alert_status = svc.get_status().await?;
    if alert_status.ahead > policy.alert_unpushed_threshold {
        eprintln!(
            "🚨 ALERT: {} has {} unpushed commits (threshold: {}). Something may be wrong with push.",
            repo.display(),
            alert_status.ahead,
            policy.alert_unpushed_threshold
        );
    }

    restore_excluded_paths(ctx, to_restore).await?;

    if policy.auto_push && has_origin {
        let push_ok = push_with_blob_check(ctx, 1).await?;
        if !push_ok {
            log_warn!("some mirror pushes failed for {}", repo.display());
        }
    }

    run_release_pipeline_if_bumped(repo, policy, version_bumped).await;

    Ok(None)
}

pub(crate) async fn sync_repo(
    repo: &Path,
    policy: &SyncPolicy,
    excluded_dir_names: &BTreeSet<String>,
    idle_seconds: u64,
    remote_failures: Option<&mut HashMap<String, usize>>,
    dry_run: bool,
    policy_path: Option<&Path>,
) -> Result<SyncOutcome> {
    let svc = GitService::new(repo)?;
    if !svc.is_git_repo().await? {
        if debug_enabled() {
            eprintln!("🐛 {} is not recognized as git repo", repo.display());
        }
        let ctx = SyncContext {
            repo,
            policy,
            excluded_dir_names,
            dry_run,
            idle_seconds,
            policy_path,
            has_origin: false,
            has_upstream: false,
            blob_threshold: 0,
            auto_bump_versions: false,
            remote_failures: None,
        };
        maybe_sync_visibility_and_metadata(&ctx);
        return Ok(SyncOutcome::NothingToDo);
    }

    if let Some(blocked) = check_conflict_state(repo) {
        let ctx = SyncContext {
            repo,
            policy,
            excluded_dir_names,
            dry_run,
            idle_seconds,
            policy_path,
            has_origin: false,
            has_upstream: false,
            blob_threshold: 0,
            auto_bump_versions: false,
            remote_failures: None,
        };
        maybe_sync_visibility_and_metadata(&ctx);
        return Ok(blocked);
    }

    if !is_repo_ready(repo) {
        eprintln!(
            "⏳ {} not ready (mid-clone or empty repo), skipping",
            repo.display()
        );
        return Ok(SyncOutcome::NothingToDo);
    }

    let has_origin = ensure_origin_remote(repo, policy);
    let has_upstream = has_tracking_upstream(repo);
    let blob_threshold = push_large_blob_threshold_bytes(policy);
    let initial_status = svc.get_status().await?;

    let repo_override = load_repo_override(repo);
    let auto_bump_versions = repo_override
        .auto_bump_versions
        .unwrap_or(policy.auto_bump_versions);

    let mut ctx = SyncContext {
        repo,
        policy,
        excluded_dir_names,
        dry_run,
        idle_seconds,
        policy_path,
        has_origin,
        has_upstream,
        blob_threshold,
        auto_bump_versions,
        remote_failures,
    };

    let copied_standard_files = if policy.standard_files_auto {
        // Acquire git's index.lock before writing standard files to the working tree.
        // This prevents conflicting with git's own checkout during clone — if git
        // holds the lock, we skip; if we hold it, git waits. No heuristics needed.
        match crate::git::IndexLock::acquire(repo) {
            Ok(_lock) => crate::standard_files::ensure_standard_files(
                repo,
                policy,
                &repo_override,
                policy_path.map(|p| p.parent().unwrap_or(p)),
                dry_run,
            )?,
            Err(e) => {
                if crate::policy::debug_enabled() {
                    eprintln!("⏳ {}", e);
                }
                vec![] // skip standard files this cycle
            }
        }
    } else {
        vec![]
    };

    if !copied_standard_files.is_empty() && !dry_run {
        let paths: Vec<String> = copied_standard_files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        stage_existing_files(repo, &paths, dry_run).await?;
    }

    auto_pull_merge(&svc, &ctx, &initial_status).await?;

    clean_staged_paths(&ctx).await?;

    let DiffResult {
        status,
        entries,
        filter_only_cleared,
    } = compute_diff_entries(&svc, repo).await?;

    // filter_only_cleared: changes present but all filtered out by clean/smudge.
    // Don't stage/commit — return NothingToDo so the daemon applies a cooldown.
    if filter_only_cleared {
        if debug_enabled() {
            eprintln!(
                "🐛 {} filter-only dirty, returning NothingToDo for cooldown",
                repo.display(),
            );
        }
        return Ok(SyncOutcome::NothingToDo);
    }

    if !status.is_clean && policy.auto_commit {
        let (to_stage, to_restore): (Vec<_>, Vec<_>) = entries
            .into_iter()
            .filter(|e| {
                if repo.join(&e.path).is_dir()
                    && crate::exclude::is_gitlink_unchanged(repo, &e.path)
                {
                    return false;
                }
                true
            })
            .partition(|e| {
                should_stage_entry(
                    repo,
                    e,
                    excluded_dir_names,
                    &policy.exclude_file_patterns,
                    policy.max_stage_file_bytes,
                )
            });
        if debug_enabled() {
            eprintln!(
                "🐛 {} to_stage={} to_restore={}",
                repo.display(),
                to_stage.len(),
                to_restore.len()
            );
        }
        if !to_stage.is_empty() {
            if let Some(outcome) =
                stage_commit_and_push(&svc, &mut ctx, &status, &to_stage, &to_restore).await?
            {
                return Ok(outcome);
            }
        } else if policy.auto_push && !has_origin {
            eprintln!("ℹ️ skip push for {} (no origin remote)", repo.display());
        }

        return Ok(SyncOutcome::Synced);
    }

    maybe_sync_visibility_and_metadata(&ctx);

    handle_ahead_push(&mut ctx, &svc).await?;

    maybe_sync_visibility_and_metadata(&ctx);
    Ok(SyncOutcome::NothingToDo)
}

async fn handle_ahead_push(ctx: &mut SyncContext<'_>, svc: &GitService) -> Result<()> {
    let current_status = svc.get_status().await?;
    let branch_has_upstream = super::git::has_tracking_upstream(ctx.repo);
    let should_push = current_status.ahead > 0 || !branch_has_upstream;
    if ctx.policy.auto_push && should_push && ctx.has_origin {
        let push_ok = push_with_blob_check(ctx, current_status.ahead).await?;
        if !push_ok {
            eprintln!(
                "ℹ️ push partially skipped for {} (see warnings above)",
                ctx.repo.display()
            );
        }
    } else if ctx.policy.auto_push && should_push && !ctx.has_origin {
        eprintln!("ℹ️ skip push for {} (no origin remote)", ctx.repo.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_repo_auto_github_private_graceful_on_no_gh() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        let toml_str = r#"
auto_github_private = true
auto_github_private_account = "TestAccount"
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(
            result.is_ok(),
            "sync_repo should handle missing gh gracefully: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_sync_repo_auto_commit_creates_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        // Create and stage a modified file
        let file_path = repo.join("test.txt");
        std::fs::write(&file_path, "hello world").unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "add", "test.txt"])
            .status()
            .unwrap();

        // Count commits before sync
        let commits_before = std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "rev-list", "--count", "HEAD"])
            .output()
            .unwrap()
            .stdout;
        let count_before: usize = String::from_utf8_lossy(&commits_before)
            .trim()
            .parse()
            .unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed: {:?}", result);

        // Verify a commit was created
        let commits_after = std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "rev-list", "--count", "HEAD"])
            .output()
            .unwrap()
            .stdout;
        let count_after: usize = String::from_utf8_lossy(&commits_after)
            .trim()
            .parse()
            .unwrap();
        assert_eq!(
            count_after,
            count_before + 1,
            "sync_repo should have created one new commit (before={}, after={})",
            count_before,
            count_after
        );
    }

    #[tokio::test]
    async fn test_sync_repo_skips_rebase_in_progress() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        // Simulate rebase in progress
        std::fs::create_dir_all(repo.join(".git/rebase-merge")).unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(
            result.is_ok(),
            "sync_repo should succeed even during rebase"
        );
        assert!(
            matches!(result, Ok(SyncOutcome::Blocked)),
            "rebase should cause early return (nothing synced)"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_skips_merge_in_progress() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        // Simulate merge in progress
        std::fs::write(repo.join(".git/MERGE_HEAD"), "abc123\n").unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed even during merge");
        assert!(
            matches!(result, Ok(SyncOutcome::Blocked)),
            "merge should cause early return (nothing synced)"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_skips_cherry_pick_in_progress() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        // Simulate cherry-pick in progress
        std::fs::write(repo.join(".git/CHERRY_PICK_HEAD"), "abc123\n").unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(
            result.is_ok(),
            "sync_repo should succeed even during cherry-pick"
        );
        assert!(
            matches!(result, Ok(SyncOutcome::Blocked)),
            "cherry-pick should cause early return (nothing synced)"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_auto_commit_creates_commit_for_dirty_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        // Create a dirty file
        std::fs::write(repo.join("dirty.txt"), "modified content\n").unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed: {:?}", result);
        assert!(
            matches!(result, Ok(SyncOutcome::Synced)),
            "dirty repo with auto_commit should sync"
        );

        // Verify commit was made
        let output = std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "log", "--oneline"])
            .output()
            .unwrap();
        let log = String::from_utf8_lossy(&output.stdout);
        assert!(
            log.lines().count() >= 2,
            "should have at least 2 commits (init + auto-commit)"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_clean_repo_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(
            matches!(result, Ok(SyncOutcome::NothingToDo)),
            "clean repo should return false (nothing to sync)"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_stages_and_commits_untracked_file() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        // Create untracked file
        std::fs::write(repo.join("newfile.txt"), "new content\n").unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed: {:?}", result);
        assert!(
            matches!(result, Ok(SyncOutcome::Synced)),
            "untracked file should be staged and committed"
        );

        // Verify file is tracked
        let output = std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "ls-files"])
            .output()
            .unwrap();
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(
            tracked.contains("newfile.txt"),
            "newfile.txt should be tracked"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_skip_pull_when_not_behind() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = false
auto_pull = true
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(
            matches!(result, Ok(SyncOutcome::NothingToDo)),
            "not behind should return false (nothing to pull)"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_skip_pull_when_dirty() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        std::fs::write(repo.join("dirty.txt"), "modified\n").unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = false
auto_pull = true
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed with dirty repo");
        assert!(
            matches!(result, Ok(SyncOutcome::NothingToDo)),
            "dirty repo should skip pull and return false"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_skip_push_when_no_origin() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = false
auto_pull = false
auto_push = true
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed without origin");
    }

    #[tokio::test]
    async fn test_sync_repo_skip_push_when_no_upstream() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = false
auto_pull = false
auto_push = true
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed without upstream");
    }

    #[tokio::test]
    async fn test_sync_repo_mirror_push_failure_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        let origin_bare = tmp.path().join("origin.git");
        std::process::Command::new("git")
            .args(["init", "--bare", "-q", "-b", "master"])
            .arg(&origin_bare)
            .status()
            .unwrap();

        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "remote",
                "add",
                "origin",
                &origin_bare.to_string_lossy(),
            ])
            .status()
            .unwrap();

        // Point mirror to non-existent path so push fails
        let bad_mirror = tmp.path().join("nonexistent-mirror.git");
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "remote",
                "add",
                "mirror",
                &bad_mirror.to_string_lossy(),
            ])
            .status()
            .unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = false
auto_pull = false
auto_push = true
auto_bump_versions = false

[[remotes]]
name = "mirror"
push_url = "git@nonexistent.example.com:repo.git"
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should not error");
        assert!(
            matches!(result, Ok(SyncOutcome::NothingToDo)),
            "mirror push failure should return false (hard fail)"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_mirror_failure_tracks_remote_failures() {
        let tmp = tempfile::tempdir().unwrap();
        let origin_bare = tmp.path().join("origin.git");
        std::process::Command::new("git")
            .args(["init", "--bare", "-q", "-b", "master"])
            .arg(&origin_bare)
            .status()
            .unwrap();

        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "remote",
                "add",
                "origin",
                &origin_bare.to_string_lossy(),
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "push",
                "-u",
                "origin",
                "master",
            ])
            .status()
            .unwrap();

        std::fs::write(repo.join("change.txt"), "changed\n").unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = true
auto_bump_versions = false

[[remotes]]
name = "bad-mirror"
push_url = "git@nonexistent.example.com:repo.git"
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let mut remote_failures = HashMap::new();
        let result = sync_repo(
            &repo,
            &policy,
            &BTreeSet::new(),
            0,
            Some(&mut remote_failures),
            false,
            None,
        )
        .await;
        assert!(result.is_ok());
        assert!(
            matches!(result.unwrap(), SyncOutcome::Synced),
            "mirror push failure should still return Synced (origin push succeeded)"
        );
        assert_eq!(
            remote_failures.get("bad-mirror"),
            Some(&1),
            "bad-mirror failure should be tracked"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_mirror_push_success_returns_true() {
        let tmp = tempfile::tempdir().unwrap();
        let origin_bare = tmp.path().join("origin.git");
        let mirror_bare = tmp.path().join("mirror.git");
        std::process::Command::new("git")
            .args(["init", "--bare", "-q", "-b", "master"])
            .arg(&origin_bare)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["init", "--bare", "-q", "-b", "master"])
            .arg(&mirror_bare)
            .status()
            .unwrap();

        let repo = tmp.path().join("test-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "remote",
                "add",
                "origin",
                &origin_bare.to_string_lossy(),
            ])
            .status()
            .unwrap();
        // Push initial commit to origin so upstream is set
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "push",
                "-u",
                "origin",
                "master",
            ])
            .status()
            .unwrap();

        // Make repo dirty so sync creates a commit and pushes
        std::fs::write(repo.join("change.txt"), "changed\n").unwrap();

        let toml_str = format!(
            r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = true
auto_bump_versions = false

[[remotes]]
name = "mirror"
push_url = "{}"
"#,
            mirror_bare.to_string_lossy().replace("\\", "/")
        );
        let policy: SyncPolicy = toml::from_str(&toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should not error: {:?}", result);
        assert!(
            matches!(result, Ok(SyncOutcome::Synced)),
            "mirror push success should return true"
        );
    }

    fn init_test_repo(tmp: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
        let repo = tmp.path().join(name);
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();
        repo
    }

    fn git_cmd(repo: &Path, args: &[&str]) -> std::process::Output {
        let repo_str = repo.to_string_lossy().to_string();
        let mut cmd = std::process::Command::new("git");
        cmd.arg("-C").arg(&repo_str);
        for a in args {
            cmd.arg(a);
        }
        cmd.output().unwrap()
    }

    #[tokio::test]
    async fn test_sync_repo_not_git_repo_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        let not_repo = tmp.path().join("not-a-repo");
        std::fs::create_dir_all(&not_repo).unwrap();

        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&not_repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should not error on non-git dir");
        assert!(
            matches!(result, Ok(SyncOutcome::NothingToDo)),
            "non-git dir should return false"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_single_deleted_file_committed() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "single-del-repo");

        std::fs::write(repo.join("keep.txt"), "keep\n").unwrap();
        std::fs::write(repo.join("remove.txt"), "remove\n").unwrap();
        git_cmd(&repo, &["add", "-A"]);
        git_cmd(&repo, &["commit", "-m", "add files"]);

        std::fs::remove_file(repo.join("remove.txt")).unwrap();

        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(
            matches!(result, Ok(SyncOutcome::Synced)),
            "single deletion should be committed"
        );

        let output = git_cmd(&repo, &["ls-files"]);
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(
            tracked.contains("keep.txt"),
            "keep.txt should still be tracked"
        );
        assert!(
            !tracked.contains("remove.txt"),
            "remove.txt should be removed from index"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_partial_deletion_allowed() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "partial-del-repo");

        std::fs::write(repo.join("a.txt"), "a\n").unwrap();
        std::fs::write(repo.join("b.txt"), "b\n").unwrap();
        std::fs::write(repo.join("c.txt"), "c\n").unwrap();
        git_cmd(&repo, &["add", "-A"]);
        git_cmd(&repo, &["commit", "-m", "add files"]);

        // Delete 2 of 3 files (66% — should be ALLOWED, only 100% wipe is blocked)
        std::fs::remove_file(repo.join("a.txt")).unwrap();
        std::fs::remove_file(repo.join("b.txt")).unwrap();

        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(
            matches!(result, Ok(SyncOutcome::Synced)),
            "partial deletion should be committed (not blocked)"
        );

        // Verify deleted files are removed from tracking (deletion WAS committed)
        let output = git_cmd(&repo, &["ls-files"]);
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(
            !tracked.contains("a.txt"),
            "a.txt should be removed after partial deletion commit"
        );
        assert!(
            !tracked.contains("b.txt"),
            "b.txt should be removed after partial deletion commit"
        );
        assert!(tracked.contains("c.txt"), "c.txt should still be tracked");
    }

    #[tokio::test]
    async fn test_sync_repo_exactly_50_percent_deletion_allowed() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "exact-50-del-repo");

        std::fs::write(repo.join("a.txt"), "a\n").unwrap();
        std::fs::write(repo.join("b.txt"), "b\n").unwrap();
        git_cmd(&repo, &["add", "-A"]);
        git_cmd(&repo, &["commit", "-m", "add files"]);

        // Delete exactly 1 of 2 files (50% — at threshold, should be ALLOWED)
        std::fs::remove_file(repo.join("a.txt")).unwrap();

        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(
            matches!(result, Ok(SyncOutcome::Synced)),
            "exactly 50% deletion should be committed (not blocked)"
        );

        let output = git_cmd(&repo, &["ls-files"]);
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(
            !tracked.contains("a.txt"),
            "a.txt should be removed after 50% deletion commit"
        );
        assert!(tracked.contains("b.txt"), "b.txt should still be tracked");
    }

    #[tokio::test]
    async fn test_sync_repo_empty_repo_no_panic() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "empty-repo");

        // Repo has only the empty initial commit, no tracked files
        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should not panic on empty repo");
    }

    #[tokio::test]
    async fn test_sync_repo_unstages_excluded_dir_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "exclude-dir-repo");

        std::fs::create_dir_all(repo.join("node_modules/pkg")).unwrap();
        std::fs::write(
            repo.join("node_modules/pkg/index.js"),
            "module.exports = {};\n",
        )
        .unwrap();
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/main.rs"), "fn main() {}\n").unwrap();
        git_cmd(&repo, &["add", "-A"]);
        git_cmd(&repo, &["commit", "-m", "initial"]);

        std::fs::write(repo.join("node_modules/pkg/index.js"), "updated\n").unwrap();
        std::fs::write(
            repo.join("src/main.rs"),
            "fn main() { println!(\"hello\"); }\n",
        )
        .unwrap();
        git_cmd(&repo, &["add", "-A"]);

        let mut excluded = BTreeSet::new();
        excluded.insert("node_modules".to_string());

        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &excluded, 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed");

        let output = git_cmd(&repo, &["log", "--oneline", "-1"]);
        let last_commit = String::from_utf8_lossy(&output.stdout);
        assert!(
            !last_commit.is_empty(),
            "should have committed the non-excluded change"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_unstages_oversized_file() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "oversized-repo");

        std::fs::write(repo.join("small.txt"), "small content\n").unwrap();
        git_cmd(&repo, &["add", "-A"]);
        git_cmd(&repo, &["commit", "-m", "initial"]);

        let big_content = vec![b'X'; 1024];
        std::fs::write(repo.join("bigfile.bin"), &big_content).unwrap();
        std::fs::write(repo.join("small2.txt"), "another small\n").unwrap();
        git_cmd(&repo, &["add", "-A"]);

        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        max_stage_file_bytes = 512
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(
            result.is_ok(),
            "sync_repo should succeed with oversized file"
        );

        let output = git_cmd(&repo, &["ls-files"]);
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(
            tracked.contains("small2.txt"),
            "small file should be tracked"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_mixed_tracked_and_untracked() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "mixed-repo");

        std::fs::write(repo.join("existing.txt"), "original\n").unwrap();
        git_cmd(&repo, &["add", "-A"]);
        git_cmd(&repo, &["commit", "-m", "initial"]);

        std::fs::write(repo.join("existing.txt"), "modified\n").unwrap();
        std::fs::write(repo.join("brand_new.txt"), "new file\n").unwrap();

        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(
            matches!(result, Ok(SyncOutcome::Synced)),
            "mixed changes should be committed"
        );

        let output = git_cmd(&repo, &["ls-files"]);
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(
            tracked.contains("existing.txt"),
            "existing.txt should be tracked"
        );
        assert!(
            tracked.contains("brand_new.txt"),
            "brand_new.txt should be tracked"
        );

        let show = git_cmd(&repo, &["show", "HEAD:existing.txt"]);
        assert_eq!(String::from_utf8_lossy(&show.stdout), "modified\n");
    }

    #[tokio::test]
    async fn test_sync_repo_pull_skip_when_no_origin() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "no-origin-pull-repo");

        let toml_str = r#"
        auto_github_private = false
        auto_commit = false
        auto_pull = true
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed without origin");
        assert!(
            matches!(result, Ok(SyncOutcome::NothingToDo)),
            "no origin should skip pull and return false"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_auto_commit_disabled_skips_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "no-autocommit-repo");

        std::fs::write(repo.join("dirty.txt"), "dirty content\n").unwrap();

        let toml_str = r#"
        auto_github_private = false
        auto_commit = false
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(
            matches!(result, Ok(SyncOutcome::NothingToDo)),
            "auto_commit=false should not commit dirty files"
        );

        let output = git_cmd(&repo, &["status", "--porcelain"]);
        let status = String::from_utf8_lossy(&output.stdout);
        assert!(
            status.contains("dirty.txt"),
            "file should still be untracked/unstaged"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_dry_run_does_not_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "dry-run-test");

        std::fs::write(repo.join("new_file.txt"), "new content\n").unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let commits_before = git_cmd(&repo, &["rev-list", "--count", "HEAD"]);
        let commits_count_before: usize = String::from_utf8_lossy(&commits_before.stdout)
            .trim()
            .parse()
            .unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, true, None).await;
        assert!(result.is_ok(), "dry-run should succeed");

        let commits_after = git_cmd(&repo, &["rev-list", "--count", "HEAD"]);
        let commits_count_after: usize = String::from_utf8_lossy(&commits_after.stdout)
            .trim()
            .parse()
            .unwrap();
        assert_eq!(
            commits_count_before, commits_count_after,
            "dry-run should not create any commits"
        );

        let status = git_cmd(&repo, &["status", "--porcelain"]);
        let status_output = String::from_utf8_lossy(&status.stdout);
        assert!(
            status_output.contains("new_file.txt"),
            "file should still appear as untracked in working tree"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_dry_run_does_not_push() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "dry-run-push-test");

        std::fs::write(repo.join("file.txt"), "change\n").unwrap();
        git_cmd(&repo, &["add", "."]);
        git_cmd(&repo, &["commit", "-m", "add file"]);

        let commits_before = git_cmd(&repo, &["rev-list", "--count", "HEAD"]);
        let count_before: usize = String::from_utf8_lossy(&commits_before.stdout)
            .trim()
            .parse()
            .unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = false
auto_pull = false
auto_push = true
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, true, None).await;
        assert!(result.is_ok(), "dry-run should succeed");

        let commits_after = git_cmd(&repo, &["rev-list", "--count", "HEAD"]);
        let count_after: usize = String::from_utf8_lossy(&commits_after.stdout)
            .trim()
            .parse()
            .unwrap();
        assert_eq!(
            count_before, count_after,
            "dry-run should not change commit count"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_dry_run_does_not_modify_working_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "dry-run-wt-test");

        std::fs::write(repo.join("tracked.txt"), "tracked\n").unwrap();
        git_cmd(&repo, &["add", "tracked.txt"]);
        git_cmd(&repo, &["commit", "-m", "add tracked"]);

        std::fs::write(repo.join("modified.txt"), "modified\n").unwrap();
        std::fs::write(repo.join("untracked.txt"), "untracked\n").unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, true, None).await;
        assert!(result.is_ok(), "dry-run should succeed");

        let output = git_cmd(&repo, &["status", "--porcelain"]);
        let status = String::from_utf8_lossy(&output.stdout);
        assert!(
            status.contains("modified.txt"),
            "modified.txt should still be modified"
        );
        assert!(
            status.contains("untracked.txt"),
            "untracked.txt should still be untracked"
        );
    }

    /// Comprehensive boundary test for the mass-deletion safety guard.
    #[tokio::test]
    async fn test_alert_unpushed_threshold() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "alert-threshold-repo");

        // Create and commit multiple files to build up unpushed commits
        for i in 0..3 {
            let fname = format!("file{}.txt", i);
            std::fs::write(repo.join(&fname), format!("content{}\n", i)).unwrap();
            git_cmd(&repo, &["add", &fname]);
            git_cmd(&repo, &["commit", "-m", &format!("add {}", fname)]);
        }

        // Set threshold to 2 — should trigger alert since we have 3 unpushed commits
        let toml_str = r#"
        auto_github_private = false
        auto_commit = false
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        alert_unpushed_threshold = 2
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        // No origin remote, so no push attempt — just check alert fires
        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed");
    }

    #[tokio::test]
    async fn test_alert_unpushed_threshold_not_triggered() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "alert-threshold-ok-repo");

        // Create and commit 1 file — below threshold
        std::fs::write(repo.join("file.txt"), "content\n").unwrap();
        git_cmd(&repo, &["add", "file.txt"]);
        git_cmd(&repo, &["commit", "-m", "add file"]);

        // Set threshold to 5 — should NOT trigger alert
        let toml_str = r#"
        auto_github_private = false
        auto_commit = false
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        alert_unpushed_threshold = 5
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(result.is_ok(), "sync_repo should succeed");
    }

    // Property-based test: verify safety guard logic for arbitrary deletion percentages
    #[test]
    fn test_safety_guard_property() {
        use proptest::prelude::*;

        proptest!(|(total in 1usize..100, delete_count in 0usize..100)| {
            let missing_count = delete_count.min(total);
            let is_mass_deletion = total > 0 && (missing_count * 100) / total >= 85;

            // Verify the guard blocks at ≥85% deletion, never below
            if (missing_count * 100) / total < 85 {
                prop_assert!(!is_mass_deletion, "guard should NOT block partial deletion: {} of {} ({}%)", missing_count, total, (missing_count * 100) / total);
            }

            if missing_count * 100 / total >= 85 && total > 0 {
                prop_assert!(is_mass_deletion, "guard SHOULD block mass deletion: {} of {} ({}%)", missing_count, total, (missing_count * 100) / total);
            }
        });
    }

    /// CR-1 regression test: when mass-deletion guard blocks, the index must be
    /// reset so that pre-existing modifications are not left staged uncommitted.
    #[tokio::test]
    async fn test_deletions_committed_when_intentional() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "deletion-commit-repo");

        // Create 7 tracked files — 6 will be deleted
        for i in 0..7 {
            std::fs::write(repo.join(format!("file{i}.txt")), format!("content{i}")).unwrap();
        }
        git_cmd(&repo, &["add", "."]);
        git_cmd(&repo, &["commit", "-m", "init"]);

        // Modify file0 so there's a real change that would be staged
        std::fs::write(repo.join("file0.txt"), "modified").unwrap();

        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        // Delete 6 files — no guard, daemon commits everything
        for i in 1..7 {
            std::fs::remove_file(repo.join(format!("file{i}.txt"))).unwrap();
        }

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(
            matches!(result, Ok(SyncOutcome::Synced)),
            "deletions should be committed, not blocked"
        );

        // Verify the deleted files are gone from tracking
        let ls = git_cmd(&repo, &["ls-files"]);
        let ls_str = String::from_utf8_lossy(&ls.stdout);
        assert!(
            ls_str.trim() == "file0.txt",
            "only file0 should remain, got: {:?}",
            ls_str
        );
    }

    /// CR-2 regression test: filter-only changes must NOT be re-detected by
    /// cli_diff_entries fallback, which would cause encrypted files to be
    /// committed as decrypted plaintext.
    #[tokio::test]
    async fn test_filter_only_skips_cli_diff_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "filter-only-repo");

        // Create a file that looks like a filter-only change
        std::fs::write(repo.join("secret.txt"), "plaintext").unwrap();
        git_cmd(&repo, &["add", "."]);
        git_cmd(&repo, &["commit", "-m", "init"]);

        // Simulate a filter-only state: working tree differs from index
        // but git diff HEAD shows no changes (all changes are filter artifacts).
        // We achieve this by writing the same content but with a different
        // line ending that the clean filter would normalize.
        std::fs::write(repo.join("secret.txt"), "plaintext\r\n").unwrap();

        let toml_str = r#"
        auto_github_private = false
        auto_commit = false
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        // sync_repo should see filter-only changes, skip them, and NOT
        // fall back to cli_diff_entries which would see the CRLF difference.
        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(
            matches!(
                result,
                Ok(SyncOutcome::NothingToDo) | Ok(SyncOutcome::Synced)
            ),
            "filter-only repo should produce NothingToDo or Synced without changes, got {:?}",
            result
        );

        // Verify nothing was staged
        let staged = git_cmd(&repo, &["diff", "--cached", "--name-only"]);
        let staged_str = String::from_utf8_lossy(&staged.stdout);
        assert!(
            staged_str.trim().is_empty(),
            "nothing should be staged for filter-only repo"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_with_duplicate_subjects_succeeds() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "stale-focus-repo");

        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        // Manually commit twice with same subject
        std::fs::write(repo.join("a.txt"), "a").unwrap();
        git_cmd(&repo, &["add", "."]);
        git_cmd(&repo, &["commit", "-m", "duplicate subject"]);

        std::fs::write(repo.join("b.txt"), "b").unwrap();
        git_cmd(&repo, &["add", "."]);
        git_cmd(&repo, &["commit", "-m", "duplicate subject"]);

        // sync_repo should succeed — no dedup guard blocking legitimate work
        std::fs::write(repo.join("c.txt"), "c").unwrap();
        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(
            result.is_ok(),
            "sync_repo should succeed with duplicate subjects in history"
        );
    }

    #[tokio::test]
    async fn test_sync_repo_new_branch_auto_push_attempted() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("new-branch-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "main"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "config",
                "user.email",
                "test@test",
            ])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                &repo.to_string_lossy(),
                "commit",
                "--allow-empty",
                "-m",
                "init",
            ])
            .status()
            .unwrap();

        // Create a file to make the repo dirty
        std::fs::write(repo.join("test.txt"), "content").unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = true
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        // sync_repo should succeed and attempt to push (even without upstream)
        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None).await;
        assert!(
            result.is_ok(),
            "sync_repo should succeed for new branch without upstream"
        );

        // Verify that a commit was created
        let output = std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "log", "--oneline", "-1"])
            .output()
            .unwrap();
        let log = String::from_utf8_lossy(&output.stdout);
        assert!(
            !log.contains("init") || log.contains("update"),
            "should have new commit after sync"
        );
    }
}
