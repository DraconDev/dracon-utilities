use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::Result;
use dracon_git::{build_commit_message, GitService};

pub(crate) static MASS_DELETION_GUARD_BLOCKED: AtomicU64 = AtomicU64::new(0);

use crate::exclude::{can_restore_entry, handle_large_untracked, is_large_untracked, remove_tracked_excluded_paths, should_stage_entry};
use crate::git::{
    cli_diff_entries, detect_large_blobs_ahead, git_name_status_entries, has_origin_remote,
    has_tracking_upstream, is_cherry_pick_in_progress, is_merge_in_progress,
    is_rebase_in_progress, prune_other_default_branch, push_with_retries,
    restore_paths, run_git_with_timeout,
    unstage_excluded_paths, unstage_oversized_paths,
};
use crate::git::multi_remote::{
    push_mirror_remotes,
};
use crate::policy::{debug_enabled, load_repo_override, SyncPolicy};
use crate::report::{append_incident_record, build_commit_context, detect_report_signals, IncidentRecord, push_large_blob_threshold_bytes};
use crate::git::{origin_url};
use crate::visibility::{get_github_visibility, parse_github_owner_repo, sync_mirror_visibility, sync_mirror_metadata};

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
    force_deletion: bool,
    idle_seconds: u64,
    policy_path: Option<&'a Path>,
    has_origin: bool,
    has_upstream: bool,
    blob_threshold: u64,
    auto_bump_versions: bool,
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
        &["Cargo.toml", "package.json", "pyproject.toml", "pubspec.yaml", "version.txt", "VERSION"][..]
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
                "Cargo.toml" => content.lines()
                    .map(|l| l.trim())
                    .find(|l| l.starts_with("version") && !l.starts_with("version_prefix"))
                    .and_then(|l| l.split('=').nth(1))
                    .map(|v| v.trim().trim_matches('"').trim())
                    .filter(|v| !v.is_empty() && !v.starts_with("workspace"))
                    .map(|v| v.to_string()),
                "package.json" => content.lines()
                    .map(|l| l.trim())
                    .find(|l| l.starts_with("\"version\""))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|v| v.trim().trim_matches('"').trim_matches(',').trim())
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                "pyproject.toml" => content.lines()
                    .map(|l| l.trim())
                    .find(|l| l.starts_with("version") && !l.starts_with("version_prefix"))
                    .and_then(|l| l.split('=').nth(1))
                    .map(|v| v.trim().trim_matches('"').trim_matches(',').trim())
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                "pubspec.yaml" => content.lines()
                    .map(|l| l.trim())
                    .find(|l| l.starts_with("version:"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|v| v.trim().split('+').next().unwrap_or("").trim())
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string()),
                "version.txt" | "VERSION" => {
                    let v = content.trim();
                    if !v.is_empty() && v.contains('.') { Some(v.to_string()) } else { None }
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

fn maybe_sync_visibility_and_metadata(
    repo: &Path,
    policy: &SyncPolicy,
    dry_run: bool,
) {
    if dry_run || (!policy.sync_visibility && !policy.sync_metadata) {
        return;
    }
    if let Some(origin_url) = crate::git::multi_remote::get_remote_url(repo, "origin") {
        if policy.sync_metadata {
            sync_mirror_metadata(&origin_url, &policy.remotes, repo, policy.sync_visibility_interval_hours);
        }
        if policy.sync_visibility {
            sync_mirror_visibility(&origin_url, &policy.remotes, repo, policy.sync_visibility_interval_hours);
        }
    }
}

fn check_conflict_state(repo: &Path) -> Option<SyncOutcome> {
    if is_rebase_in_progress(repo) {
        eprintln!("⚠️ {} has rebase in progress, skipping (manual intervention required)", repo.display());
        return Some(SyncOutcome::Blocked);
    }
    if is_merge_in_progress(repo) {
        eprintln!("⚠️ {} has merge in progress, skipping (manual intervention required)", repo.display());
        return Some(SyncOutcome::Blocked);
    }
    if is_cherry_pick_in_progress(repo) {
        eprintln!("⚠️ {} has cherry-pick in progress, skipping (manual intervention required)", repo.display());
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
        if let Some(url) = crate::report::create_github_private_remote(repo, &policy.auto_github_private_account, private) {
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
    repo: &Path,
    policy: &SyncPolicy,
    has_origin: bool,
    has_upstream: bool,
    initial_status: &dracon_git::types::RepoStatus,
    dry_run: bool,
) -> Result<()> {
    if policy.auto_pull && has_origin && has_upstream && initial_status.behind > 0 && initial_status.is_clean {
        if dry_run {
            println!("🔽 Would pull/merge {} commit(s) from upstream in {}", initial_status.behind, repo.display());
        } else {
            match tokio::time::timeout(
                Duration::from_secs(policy.pull_op_timeout_secs),
                svc.pull_merge(),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(dracon_git::error::GitError::MergeConflict)) => {
                    eprintln!("⚠️ pull/merge conflict in {} (manual intervention required)", repo.display());
                    return Err(anyhow::anyhow!("pull/merge conflict"));
                }
                Ok(Err(e)) => {
                    eprintln!("⚠️ pull/merge failed for {}: {} - aborting sync pass", repo.display(), e);
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
    } else if policy.auto_pull && has_origin && has_upstream && initial_status.behind == 0 {
        if debug_enabled() {
            eprintln!(
                "🐛 skip pull/merge for {} (branch not behind upstream)",
                repo.display()
            );
        }
    } else if policy.auto_pull && has_origin && has_upstream && !initial_status.is_clean {
        if debug_enabled() {
            eprintln!(
                "🐛 skip pull/merge for {} (dirty repo, commit first)",
                repo.display()
            );
        }
    } else if policy.auto_pull && !has_origin {
        eprintln!(
            "ℹ️ skip pull/merge for {} (no origin remote)",
            repo.display()
        );
    } else if policy.auto_pull && has_origin && !has_upstream {
        eprintln!(
            "ℹ️ skip pull/merge for {} (no tracking upstream on current branch)",
            repo.display()
        );
    }
    Ok(())
}

async fn clean_staged_paths(
    repo: &Path,
    policy: &SyncPolicy,
    excluded_dir_names: &BTreeSet<String>,
    dry_run: bool,
) -> Result<()> {
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
    filter_only_cleared: bool,
}

async fn compute_diff_entries(
    svc: &GitService,
    repo: &Path,
) -> Result<DiffResult> {
    let mut status = svc.get_status().await?;
    let mut entries = svc.get_diff_entries().await?;
    let mut filter_only_cleared = false;

    {
        let diff_output = crate::git::git_diff_head_files(repo).await.unwrap_or_default();
        if diff_output.is_empty() && !entries.is_empty() {
            let has_non_modified = entries.iter().any(|e| {
                !matches!(e.status, dracon_git::types::FileStatus::Modified)
            });
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
            entries = fallback_entries;
            if debug_enabled() {
                eprintln!(
                    "🐛 {} fallback entries(cli)={} => forcing dirty",
                    repo.display(),
                    status.modified_files
                );
            }
        }
    }

    Ok(DiffResult { status, entries, filter_only_cleared })
}

enum MassDeletionCheck {
    Ok,
    Blocked,
}

async fn check_mass_deletion(
    repo: &Path,
    missing: &[String],
    force_deletion: bool,
    dry_run: bool,
    policy_path: Option<&Path>,
) -> Result<MassDeletionCheck> {
    if force_deletion {
        eprintln!("⚠️ --force: bypassing mass-deletion safety guard for {} ({} files)", repo.display(), missing.len());
        return Ok(MassDeletionCheck::Ok);
    }

    let total_tracked: usize = {
        let repo = repo.to_path_buf();
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("git")
                .args(["ls-files"])
                .current_dir(&repo)
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
                .unwrap_or(0)
        })
        .await
        .unwrap_or(0)
    };

    let missing_count = missing.len();
    const MASS_DELETION_THRESHOLD_PCT: usize = 85;
    let is_mass_deletion = total_tracked > 0
        && (missing_count * 100) / total_tracked >= MASS_DELETION_THRESHOLD_PCT;

    if is_mass_deletion {
        let pct = (missing_count * 100) / total_tracked;
        let reason = format!("{} files missing from working tree ({}% of {} tracked)", missing_count, pct, total_tracked);
        eprintln!("⚠️ SAFETY: {}", reason);
        eprintln!("⚠️ Refusing to stage mass deletion - this looks like a mistake or destructive operation");
        eprintln!("⚠️ If you really want to delete these files, do: git add -A && git commit -m 'delete files'");
        MASS_DELETION_GUARD_BLOCKED.fetch_add(1, Ordering::Relaxed);
        if let Some(path) = policy_path {
            append_incident_record(
                path,
                &IncidentRecord::new(
                    crate::policy::timestamp_secs(),
                    "safety",
                    repo.display().to_string(),
                    reason.clone(),
                    "mass_deletion_guard",
                    None,
                    "blocked",
                    Some(format!("total_tracked={} missing_count={}", total_tracked, missing_count)),
                ),
            );
        }
        if !dry_run {
            let _ = run_git_with_timeout(repo, &["reset", "HEAD"], 10, "reset-after-guard").await;
        }
        return Ok(MassDeletionCheck::Blocked);
    }

    Ok(MassDeletionCheck::Ok)
}

async fn stage_existing_files(
    repo: &Path,
    existing: &[String],
    dry_run: bool,
) -> Result<()> {
    if existing.is_empty() {
        return Ok(());
    }
    if dry_run {
        println!("📝 Would stage {} file(s) in {}: {:?}", existing.len(), repo.display(), &existing[..existing.len().min(5)]);
        if existing.len() > 5 {
            println!("  ... and {} more", existing.len() - 5);
        }
    } else {
        let mut add_args = vec!["add", "-A", "-f", "--"];
        for p in existing {
            add_args.push(p);
        }
        if let Err(e) = run_git_with_timeout(repo, &add_args, 30, "add").await {
            eprintln!("⚠️ {} git add failed for {} paths: {:?}", repo.display(), existing.len(), existing);
            return Err(e);
        }
    }
    Ok(())
}

async fn git_rm_missing(
    repo: &Path,
    missing: &[String],
    dry_run: bool,
) -> Result<()> {
    if missing.is_empty() {
        return Ok(());
    }
    let mut rm_args = vec!["rm", "--ignore-unmatch", "--"];
    for p in missing {
        rm_args.push(p);
    }
    if dry_run {
        println!("🗑️  Would delete (git rm) {} file(s) from {}: {:?}", missing.len(), repo.display(), &missing[..missing.len().min(5)]);
        if missing.len() > 5 {
            println!("  ... and {} more", missing.len() - 5);
        }
    } else if let Err(e) = run_git_with_timeout(repo, &rm_args, 30, "rm").await {
        eprintln!("⚠️ {} git rm failed for {} paths: {:?}", repo.display(), missing.len(), missing);
        return Err(e);
    }
    Ok(())
}

async fn get_staged_diff_content(repo: &Path) -> Option<String> {
    let repo_stat = repo.to_path_buf();
    let repo_patch = repo.to_path_buf();
    let stat_result = tokio::task::spawn_blocking(move || {
        std::process::Command::new("git")
            .args(["diff", "--cached", "--stat"])
            .current_dir(&repo_stat)
            .output()
    })
    .await;
    match stat_result {
        Ok(Ok(o)) if o.status.success() => {
            let stat = String::from_utf8_lossy(&o.stdout).to_string();
            if stat.is_empty() {
                None
            } else {
                let patch_result = tokio::task::spawn_blocking(move || {
                    std::process::Command::new("git")
                        .args(["diff", "--cached", "--unified=3", "--"])
                        .current_dir(&repo_patch)
                        .output()
                })
                .await;
                let patch_text = match patch_result {
                    Ok(Ok(o)) if o.status.success() => {
                        let patch = String::from_utf8_lossy(&o.stdout).to_string();
                        if patch.lines().count() > 200 {
                            patch.lines().take(200).collect::<Vec<_>>().join("\n") + "\n... (truncated)"
                        } else {
                            patch
                        }
                    }
                    _ => String::new(),
                };
                Some(format!("{}\n\n{}", stat, patch_text))
            }
        }
        _ => None,
    }
}

async fn run_deterministic_bumper(
    repo: &Path,
    committed_entries: &[dracon_git::types::DiffFile],
    dry_run: bool,
    auto_bump_versions: bool,
) -> bool {
    if dry_run || !auto_bump_versions || !cfg!(feature = "scribe") {
        return false;
    }
    #[cfg(feature = "scribe")]
    {
        use crate::bump::{deterministic_decide_bump_level, bump_semver, read_current_version, BumpLevel};

        let staged_diff = committed_entries.iter()
            .map(|e| format!("{:?}: {}", e.status, e.path.display()))
            .collect::<Vec<_>>()
            .join("\n");

        if let Some(current_ver) = read_current_version(repo) {
            let level = deterministic_decide_bump_level(&staged_diff);
            if level != BumpLevel::None {
                eprintln!("📦 bump: {} -> patch", current_ver);
                if let Some(new_ver) = bump_semver(&current_ver, BumpLevel::Patch) {
                    let bumped = crate::bump::apply_version_bump_to_repo(repo, &current_ver, &new_ver);
                    if bumped {
                        return true;
                    }
                }
            }
        }
    }
    let _ = (repo, committed_entries, auto_bump_versions);
    false
}

async fn stage_version_files(repo: &Path) {
    for file in crate::bump::VERSION_FILES {
        if repo.join(file).exists() {
            if let Err(e) = run_git_with_timeout(repo, &["add", file], 30, "add").await {
                eprintln!("⚠️ failed to stage {}: {}", file, e);
            }
        }
    }
}

async fn run_ai_bumper(
    repo: &Path,
    committed_entries: &[dracon_git::types::DiffFile],
    dry_run: bool,
    auto_bump_versions: bool,
    version_bumped: bool,
) -> bool {
    if dry_run || !auto_bump_versions || version_bumped || !cfg!(feature = "ai-bumper") {
        return false;
    }
    #[cfg(feature = "ai-bumper")]
    {
        use crate::bump::{ai_decide_bump_level, bump_semver, read_current_version, BumpLevel};

        let staged_diff = committed_entries.iter()
            .map(|e| format!("{:?}: {}", e.status, e.path.display()))
            .collect::<Vec<_>>()
            .join("\n");
        let project_state = std::fs::read_to_string(repo.join(".dracon/project-state.md"))
            .unwrap_or_default();

        if let Some(current_ver) = read_current_version(repo) {
            let level = ai_decide_bump_level(repo, &current_ver, &staged_diff, &project_state).await;
            if level != BumpLevel::None {
                eprintln!("🤖 ai-bump: {} -> {}", current_ver, level.as_str());
                let new_ver = bump_semver(&current_ver, level);

                if let Some(new_ver) = new_ver {
                    let bumped = crate::bump::apply_version_bump_to_repo(repo, &current_ver, &new_ver);
                    if bumped {
                        return true;
                    }
                }
            }
        }
    }
    let _ = (repo, committed_entries, auto_bump_versions, version_bumped);
    false
}

async fn scribe_update(
    repo: &Path,
    staged_diff_names: &str,
    staged_diff_content: Option<String>,
    dry_run: bool,
) {
    if !dry_run && cfg!(feature = "scribe") {
        #[cfg(feature = "scribe")]
        if let Err(e) = crate::scribe::update_project_state_from_ai(repo, staged_diff_names, staged_diff_content).await {
            eprintln!("📝 scribe failed for {}: {}", repo.display(), e);
        }
    }
    let _ = (repo, staged_diff_names, staged_diff_content);
}

async fn stage_project_state(repo: &Path) {
    if repo.join(".dracon/project-state.md").exists() {
        if let Err(e) = run_git_with_timeout(repo, &["add", "-f", ".dracon/project-state.md"], 10, "add-project-state").await {
            eprintln!("⚠️ failed to stage project-state: {}", e);
        }
    }
}

async fn post_commit_pull(
    svc: &GitService,
    repo: &Path,
    policy: &SyncPolicy,
) {
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
        ).await {
            Ok(Ok(())) => {
                eprintln!("✅ post-commit pull succeeded for {}", repo.display());
            }
            Ok(Err(dracon_git::error::GitError::MergeConflict)) => {
                eprintln!("⚠️ post-commit pull conflict in {} (manual intervention required)", repo.display());
            }
            Ok(Err(e)) => {
                eprintln!("⚠️ post-commit pull failed for {}: {} - will still attempt push", repo.display(), e);
            }
            Err(_) => {
                eprintln!("⚠️ post-commit pull timeout for {} after {}s - will still attempt push", repo.display(), policy.pull_op_timeout_secs);
            }
        }
    }
}

async fn restore_excluded_paths(
    repo: &Path,
    to_restore: &[dracon_git::types::DiffFile],
    policy: &SyncPolicy,
) -> Result<()> {
    let restorable: Vec<_> = to_restore.iter()
        .filter(|e| can_restore_entry(repo, e))
        .filter(|e| !repo.join(&e.path).is_dir() || !crate::exclude::is_gitlink_unchanged(repo, &e.path))
        .collect();

    handle_large_untracked(repo, to_restore, policy)?;

    let other_untracked: Vec<_> = to_restore
        .iter()
        .filter(|e| !can_restore_entry(repo, e) && !is_large_untracked(e, repo, policy.max_stage_file_bytes))
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

async fn run_release_pipeline_if_bumped(
    repo: &Path,
    policy: &SyncPolicy,
    version_bumped: bool,
) {
    if !version_bumped {
        return;
    }
    if let Some((old_ver, new_ver, level)) = get_bump_info(repo).await {
        let repo_override = crate::policy::load_repo_override(repo);
        let repo_auto_tag = repo_override.auto_tag.unwrap_or(policy.auto_tag);
        let repo_auto_release = repo_override.auto_release.unwrap_or(policy.auto_release);
        let repo_publish_targets = repo_override.auto_publish;
        let steps = crate::release::run_release_pipeline(
            repo, &old_ver, &new_ver, level.as_str(), policy,
            repo_auto_tag, repo_auto_release, &repo_publish_targets,
        ).await;
        for step in &steps {
            match step {
                crate::release::ReleaseStep::TagCreated(tag) => eprintln!("🏷️  {tag}"),
                crate::release::ReleaseStep::GitHubReleaseCreated(tag) => eprintln!("🚀 {tag}"),
                crate::release::ReleaseStep::Published { registry, version } => eprintln!("📦 published to {registry} v{version}"),
                crate::release::ReleaseStep::Skipped(reason) => { if debug_enabled() { eprintln!("🐛 release skipped: {reason}"); } }
                crate::release::ReleaseStep::Failed { step: s, error } => eprintln!("⚠️ release failed: {s} — {error}"),
            }
        }
    }
}

async fn push_with_blob_check(
    repo: &Path,
    policy: &SyncPolicy,
    blob_threshold: u64,
    has_origin: bool,
    ahead: usize,
    mut remote_failures: Option<&mut HashMap<String, usize>>,
    dry_run: bool,
) -> Result<bool> {
    if !policy.auto_push || !has_origin || ahead == 0 {
        return Ok(true);
    }

    let ahead_large = match detect_large_blobs_ahead(repo, blob_threshold).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("⚠️ large blob detection failed for {}: {} - skipping push", repo.display(), e);
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
            eprintln!("⚠️ push failed for {}: {}", repo.display(), e);
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
            policy.remotes.iter().map(|r| (r.name.clone(), Ok(()))).collect()
        } else {
            push_mirror_remotes(
                repo,
                &policy.remotes,
                policy.push_op_timeout_secs,
                policy.push_retries,
                private,
            ).await
        };
        let all_ok = push_results.iter().all(|(_, r)| r.is_ok());
        if !all_ok {
            for (name, result) in &push_results {
                if let Err(e) = result {
                    eprintln!("⚠️ push to {} failed for {}: {}", name, repo.display(), e);
                    if let Some(ref url) = policy.webhook_url {
                        notify_webhook_failure(url, repo, name, &e.to_string());
                    }
                    if let Some(ref mut rf) = remote_failures {
                        *rf.entry(name.clone()).or_insert(0) += 1;
                    }
                }
            }
            return Ok(false);
        } else if let Some(ref mut rf) = remote_failures {
            for name in policy.remotes.iter().map(|r| r.name.clone()) {
                rf.remove(&name);
            }
        }
    }

    Ok(true)
}

pub(crate) async fn sync_repo(
    repo: &Path,
    policy: &SyncPolicy,
    excluded_dir_names: &BTreeSet<String>,
    idle_seconds: u64,
    remote_failures: Option<&mut HashMap<String, usize>>,
    dry_run: bool,
    policy_path: Option<&Path>,
    force_deletion: bool,
) -> Result<SyncOutcome> {
    let svc = GitService::new(repo)?;
    if !svc.is_git_repo().await? {
        if debug_enabled() {
            eprintln!("🐛 {} is not recognized as git repo", repo.display());
        }
        maybe_sync_visibility_and_metadata(repo, policy, dry_run);
        return Ok(SyncOutcome::NothingToDo);
    }

    if let Some(blocked) = check_conflict_state(repo) {
        maybe_sync_visibility_and_metadata(repo, policy, dry_run);
        return Ok(blocked);
    }

    let has_origin = ensure_origin_remote(repo, policy);
    let has_upstream = has_tracking_upstream(repo);
    let blob_threshold = push_large_blob_threshold_bytes(policy);
    let initial_status = svc.get_status().await?;

    let repo_override = load_repo_override(repo);
    let auto_bump_versions = repo_override
        .auto_bump_versions
        .unwrap_or(policy.auto_bump_versions);

    auto_pull_merge(&svc, repo, policy, has_origin, has_upstream, &initial_status, dry_run).await?;

    clean_staged_paths(repo, policy, excluded_dir_names, dry_run).await?;

    let DiffResult { status, entries, filter_only_cleared: _ } = compute_diff_entries(&svc, repo).await?;

    if !status.is_clean && policy.auto_commit {
        let (to_stage, to_restore): (Vec<_>, Vec<_>) = entries
            .into_iter()
            .filter(|e| {
                if repo.join(&e.path).is_dir() && crate::exclude::is_gitlink_unchanged(repo, &e.path) {
                    return false;
                }
                true
            })
            .partition(|e| {
                should_stage_entry(repo, e, excluded_dir_names, &policy.exclude_file_patterns, policy.max_stage_file_bytes)
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
            let stage_paths: Vec<String> = to_stage
                .iter()
                .map(|e| e.path.to_string_lossy().to_string())
                .collect();

            let (existing, missing): (Vec<_>, Vec<_>) = stage_paths
                .into_iter()
                .partition(|p| repo.join(p).exists());

            stage_existing_files(repo, &existing, dry_run).await?;

            if !missing.is_empty() {
                match check_mass_deletion(repo, &missing, force_deletion, dry_run, policy_path).await? {
                    MassDeletionCheck::Blocked => {
                        maybe_sync_visibility_and_metadata(repo, policy, dry_run);
                        return Ok(SyncOutcome::Blocked);
                    }
                    MassDeletionCheck::Ok => {}
                }

                git_rm_missing(repo, &missing, dry_run).await?;
            }

            let staged = git_name_status_entries(repo, &["diff", "--cached", "--name-status"]).await?;
            let committed_entries: Vec<dracon_git::types::DiffFile> = staged
                .into_iter()
                .map(|(path, status)| dracon_git::types::DiffFile { path, status })
                .collect();

            let staged_diff_names = committed_entries.iter()
                .map(|e| format!("{:?}: {}", e.status, e.path.display()))
                .collect::<Vec<_>>()
                .join("\n");

            let staged_diff_content = get_staged_diff_content(repo).await;

            let mut version_bumped = run_deterministic_bumper(repo, &committed_entries, dry_run, auto_bump_versions).await;
            if version_bumped {
                stage_version_files(repo).await;
            }

            let ai_bumped = run_ai_bumper(repo, &committed_entries, dry_run, auto_bump_versions, version_bumped).await;
            if ai_bumped {
                version_bumped = true;
                stage_version_files(repo).await;
            }

            scribe_update(repo, &staged_diff_names, staged_diff_content, dry_run).await;

            stage_project_state(repo).await;

            let staged = git_name_status_entries(repo, &["diff", "--cached", "--name-status"]).await?;
            let committed_entries: Vec<dracon_git::types::DiffFile> = staged
                .into_iter()
                .map(|(path, status)| dracon_git::types::DiffFile { path, status })
                .collect();

            if committed_entries.is_empty() {
                if let Err(e) = run_git_with_timeout(repo, &["reset", "HEAD", "--"], 10, "reset").await {
                    return Err(anyhow::anyhow!("sync_repo: failed to reset HEAD after filter-only commit: {}", e));
                }
                if debug_enabled() {
                    eprintln!("🐛 {} skipped commit: all changes were filter-only (smudge/clean)", repo.display());
                }
                maybe_sync_visibility_and_metadata(repo, policy, dry_run);
                return Ok(SyncOutcome::NothingToDo);
            }

            let signals = detect_report_signals(repo, &committed_entries);
            let is_report = !signals.is_empty();

            let ctx = build_commit_context(
                repo,
                &status,
                &committed_entries,
                !is_report,
                idle_seconds,
            );

            let msg = build_commit_message(&ctx);

            if dry_run {
                println!("📝 Would commit {} file(s) in {}:", committed_entries.len(), repo.display());
                for entry in committed_entries.iter().take(10) {
                    println!("  {:?}: {}", entry.status, entry.path.display());
                }
                if committed_entries.len() > 10 {
                    println!("  ... and {} more", committed_entries.len() - 10);
                }
                println!("  message: {}", msg.lines().next().unwrap_or("(empty)"));
            } else {
                svc.commit(&msg).await?;
                eprintln!("📝 committed {} file(s) in {}", committed_entries.len(), repo.display());
            }

            prune_other_default_branch(repo).await;

            post_commit_pull(&svc, repo, policy).await;

            let alert_status = svc.get_status().await?;
            if alert_status.ahead > policy.alert_unpushed_threshold {
                eprintln!(
                    "🚨 ALERT: {} has {} unpushed commits (threshold: {}). Something may be wrong with push.",
                    repo.display(),
                    alert_status.ahead,
                    policy.alert_unpushed_threshold
                );
            }

            restore_excluded_paths(repo, &to_restore, policy).await?;

            if policy.auto_push && has_origin {
                let push_ok = push_with_blob_check(repo, policy, blob_threshold, has_origin, 1, remote_failures, dry_run).await?;
                if !push_ok {
                    eprintln!("⚠️ some mirror pushes failed for {}", repo.display());
                }
            }

            run_release_pipeline_if_bumped(repo, policy, version_bumped).await;
        } else if policy.auto_push && !has_origin {
            eprintln!("ℹ️ skip push for {} (no origin remote)", repo.display());
        }

        return Ok(SyncOutcome::Synced);
    }

    maybe_sync_visibility_and_metadata(repo, policy, dry_run);

    let current_status = svc.get_status().await?;
    if policy.auto_push && current_status.ahead > 0 && has_origin {
        let push_ok = push_with_blob_check(repo, policy, blob_threshold, has_origin, current_status.ahead, remote_failures, dry_run).await?;
        if !push_ok {
            eprintln!("ℹ️ push partially skipped for {} (see warnings above)", repo.display());
        }
    } else if policy.auto_push && current_status.ahead > 0 && !has_origin {
        eprintln!("ℹ️ skip push for {} (no origin remote)", repo.display());
    }

    maybe_sync_visibility_and_metadata(repo, policy, dry_run);
    Ok(SyncOutcome::NothingToDo)
}
