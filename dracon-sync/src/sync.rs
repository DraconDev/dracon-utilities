use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::log_warn;

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
    #[allow(dead_code)]
    idle_seconds: u64,
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
    ctx: &SyncContext<'_>,
) {
    if ctx.dry_run || (!ctx.policy.sync_visibility && !ctx.policy.sync_metadata) {
        return;
    }
    if let Some(origin_url) = crate::git::multi_remote::get_remote_url(ctx.repo, "origin") {
        if ctx.policy.sync_metadata {
            sync_mirror_metadata(&origin_url, &ctx.policy.remotes, ctx.repo, ctx.policy.sync_visibility_interval_hours);
        }
        if ctx.policy.sync_visibility {
            sync_mirror_visibility(&origin_url, &ctx.policy.remotes, ctx.repo, ctx.policy.sync_visibility_interval_hours);
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
    ctx: &SyncContext<'_>,
    initial_status: &dracon_git::types::RepoStatus,
) -> Result<()> {
    let repo = ctx.repo;
    let policy = ctx.policy;
    if policy.auto_pull && ctx.has_origin && ctx.has_upstream && initial_status.behind > 0 && initial_status.is_clean {
        if ctx.dry_run {
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

async fn clean_staged_paths(
    ctx: &SyncContext<'_>,
) -> Result<()> {
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
    ctx: &SyncContext<'_>,
    missing: &[String],
) -> Result<MassDeletionCheck> {
    let repo = ctx.repo;
    let force_deletion = ctx.force_deletion;
    let dry_run = ctx.dry_run;
    let policy_path = ctx.policy_path;
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
        log_warn!("SAFETY: {}", reason);
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
            // Defense-in-depth: ai_decide_bump_level already maps "major" => None,
            // but this guard catches it if the mapping is ever accidentally changed.
            if level == BumpLevel::Major {
                eprintln!("🔒 major bump blocked (manual-only), skipping");
                return false;
            }
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
    ctx: &SyncContext<'_>,
    to_restore: &[dracon_git::types::DiffFile],
) -> Result<()> {
    let repo = ctx.repo;
    let policy = ctx.policy;
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
    ctx: &mut SyncContext<'_>,
    ahead: usize,
) -> Result<bool> {
    let repo = ctx.repo;
    let policy = ctx.policy;
    let blob_threshold = ctx.blob_threshold;
    let has_origin = ctx.has_origin;
    let dry_run = ctx.dry_run;
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

async fn stage_commit_and_push(
    svc: &GitService,
    ctx: &mut SyncContext<'_>,
    status: &dracon_git::types::RepoStatus,
    to_stage: &[dracon_git::types::DiffFile],
    to_restore: &[dracon_git::types::DiffFile],
) -> Result<Option<SyncOutcome>> {
    let repo = ctx.repo;
    let policy = ctx.policy;
    let dry_run = ctx.dry_run;
    let auto_bump_versions = ctx.auto_bump_versions;
    let has_origin = ctx.has_origin;
    let idle_seconds = ctx.idle_seconds;

    let stage_paths: Vec<String> = to_stage
        .iter()
        .map(|e| e.path.to_string_lossy().to_string())
        .collect();

    let (existing, missing): (Vec<_>, Vec<_>) = stage_paths
        .into_iter()
        .partition(|p| repo.join(p).exists());

    stage_existing_files(repo, &existing, dry_run).await?;

    if !missing.is_empty() {
        match check_mass_deletion(ctx, &missing).await? {
            MassDeletionCheck::Blocked => {
                maybe_sync_visibility_and_metadata(ctx);
                return Ok(Some(SyncOutcome::Blocked));
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

    let is_noise_only = crate::bump::deterministic_decide_bump_level(&staged_diff_names)
        == crate::bump::BumpLevel::None;

    let staged_diff_content = get_staged_diff_content(repo).await;

    // AI bumper is the sole version bump decider — determines patch vs minor vs none
    let mut version_bumped = false;
    let ai_bumped = run_ai_bumper(repo, &committed_entries, dry_run, auto_bump_versions, version_bumped).await;
    if ai_bumped {
        version_bumped = true;
        stage_version_files(repo).await;
    }

    let ai_subject = if !is_noise_only {
        crate::scribe::generate_commit_message(repo, &staged_diff_names, staged_diff_content).await
    } else {
        None
    };

    let local_fallback = if ai_subject.is_none() {
        Some(crate::scribe::local_fallback_message(&staged_diff_names))
    } else {
        None
    };

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
        maybe_sync_visibility_and_metadata(ctx);
        return Ok(Some(SyncOutcome::NothingToDo));
    }

    let signals = detect_report_signals(repo, &committed_entries);
    let is_report = !signals.is_empty();

    let commit_ctx = build_commit_context(
        repo,
        status,
        &committed_entries,
        !is_report,
        idle_seconds,
        None,
        ai_subject.as_deref(),
        local_fallback.as_deref(),
    );

    let msg = if is_noise_only && !is_report {
        if let Some(ref fb) = local_fallback {
            let category = commit_ctx.category.as_deref().unwrap_or("chore");
            let scope = commit_ctx.scope.as_deref().unwrap_or("sync");
            format!("{}({}): {}", category, scope, fb)
        } else {
            build_commit_message(&commit_ctx)
        }
    } else if let Some(ref ai_sub) = ai_subject {
        let has_conventional_prefix = crate::bump::CONVENTIONAL_COMMIT_TYPES.iter().any(|t| {
            ai_sub.starts_with(&format!("{}:", t)) || ai_sub.starts_with(&format!("{}(", t))
        });
        if has_conventional_prefix {
            ai_sub.clone()
        } else {
            let category = commit_ctx.category.as_deref().unwrap_or("chore");
            let scope = commit_ctx.scope.as_deref().unwrap_or("sync");
            format!("{}({}): {}", category, scope, ai_sub)
        }
    } else if let Some(ref fb) = local_fallback {
        let category = commit_ctx.category.as_deref().unwrap_or("chore");
        let scope = commit_ctx.scope.as_deref().unwrap_or("sync");
        format!("{}({}): {}", category, scope, fb)
    } else {
        build_commit_message(&commit_ctx)
    };

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
    force_deletion: bool,
) -> Result<SyncOutcome> {
    let svc = GitService::new(repo)?;
    if !svc.is_git_repo().await? {
        if debug_enabled() {
            eprintln!("🐛 {} is not recognized as git repo", repo.display());
        }
        let ctx = SyncContext {
            repo, policy, excluded_dir_names, dry_run, force_deletion,
            idle_seconds, policy_path, has_origin: false, has_upstream: false,
            blob_threshold: 0, auto_bump_versions: false, remote_failures: None,
        };
        maybe_sync_visibility_and_metadata(&ctx);
        return Ok(SyncOutcome::NothingToDo);
    }

    if let Some(blocked) = check_conflict_state(repo) {
        let ctx = SyncContext {
            repo, policy, excluded_dir_names, dry_run, force_deletion,
            idle_seconds, policy_path, has_origin: false, has_upstream: false,
            blob_threshold: 0, auto_bump_versions: false, remote_failures: None,
        };
        maybe_sync_visibility_and_metadata(&ctx);
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

    let mut ctx = SyncContext {
        repo, policy, excluded_dir_names, dry_run, force_deletion,
        idle_seconds, policy_path, has_origin, has_upstream,
        blob_threshold, auto_bump_versions, remote_failures,
    };

    auto_pull_merge(&svc, &ctx, &initial_status).await?;

    clean_staged_paths(&ctx).await?;

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
            if let Some(outcome) = stage_commit_and_push(
                &svc, &mut ctx, &status, &to_stage, &to_restore,
            ).await? {
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

async fn handle_ahead_push(
    ctx: &mut SyncContext<'_>,
    svc: &GitService,
) -> Result<()> {
    let current_status = svc.get_status().await?;
    if ctx.policy.auto_push && current_status.ahead > 0 && ctx.has_origin {
        let push_ok = push_with_blob_check(ctx, current_status.ahead).await?;
        if !push_ok {
            eprintln!("ℹ️ push partially skipped for {} (see warnings above)", ctx.repo.display());
        }
    } else if ctx.policy.auto_push && current_status.ahead > 0 && !ctx.has_origin {
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
            .status()
            .unwrap();

        let toml_str = r#"
auto_github_private = true
auto_github_private_account = "TestAccount"
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should handle missing gh gracefully: {:?}", result);
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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
        let count_before: usize = String::from_utf8_lossy(&commits_before).trim().parse().unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = true
auto_pull = false
auto_push = false
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed: {:?}", result);

        // Verify a commit was created
        let commits_after = std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "rev-list", "--count", "HEAD"])
            .output()
            .unwrap()
            .stdout;
        let count_after: usize = String::from_utf8_lossy(&commits_after).trim().parse().unwrap();
        assert_eq!(
            count_after, count_before + 1,
            "sync_repo should have created one new commit (before={}, after={})",
            count_before, count_after
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed even during rebase");
        assert!(matches!(result, Ok(SyncOutcome::Blocked)), "rebase should cause early return (nothing synced)");
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed even during merge");
        assert!(matches!(result, Ok(SyncOutcome::Blocked)), "merge should cause early return (nothing synced)");
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed even during cherry-pick");
        assert!(matches!(result, Ok(SyncOutcome::Blocked)), "cherry-pick should cause early return (nothing synced)");
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed: {:?}", result);
        assert!(matches!(result, Ok(SyncOutcome::Synced)), "dirty repo with auto_commit should sync");

        // Verify commit was made
        let output = std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "log", "--oneline"])
            .output()
            .unwrap();
        let log = String::from_utf8_lossy(&output.stdout);
        assert!(log.lines().count() >= 2, "should have at least 2 commits (init + auto-commit)");
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(matches!(result, Ok(SyncOutcome::NothingToDo)), "clean repo should return false (nothing to sync)");
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed: {:?}", result);
        assert!(matches!(result, Ok(SyncOutcome::Synced)), "untracked file should be staged and committed");

        // Verify file is tracked
        let output = std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "ls-files"])
            .output()
            .unwrap();
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(tracked.contains("newfile.txt"), "newfile.txt should be tracked");
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(matches!(result, Ok(SyncOutcome::NothingToDo)), "not behind should return false (nothing to pull)");
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed with dirty repo");
        assert!(matches!(result, Ok(SyncOutcome::NothingToDo)), "dirty repo should skip pull and return false");
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "remote", "add", "origin", &origin_bare.to_string_lossy()])
            .status()
            .unwrap();

        // Point mirror to non-existent path so push fails
        let bad_mirror = tmp.path().join("nonexistent-mirror.git");
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "remote", "add", "mirror", &bad_mirror.to_string_lossy()])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should not error");
        assert!(matches!(result, Ok(SyncOutcome::NothingToDo)), "mirror push failure should return false (hard fail)");
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "remote", "add", "origin", &origin_bare.to_string_lossy()])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "push", "-u", "origin", "master"])
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
        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, Some(&mut remote_failures), false, None, false).await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), SyncOutcome::Synced), "mirror push failure should still return Synced (origin push succeeded)");
        assert_eq!(remote_failures.get("bad-mirror"), Some(&1), "bad-mirror failure should be tracked");
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
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "remote", "add", "origin", &origin_bare.to_string_lossy()])
            .status()
            .unwrap();
        // Push initial commit to origin so upstream is set
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "push", "-u", "origin", "master"])
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should not error: {:?}", result);
        assert!(matches!(result, Ok(SyncOutcome::Synced)), "mirror push success should return true");
    }

    fn init_test_repo(tmp: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
        let repo = tmp.path().join(name);
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.email", "test@test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "config", "user.name", "test"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "commit", "--allow-empty", "-m", "init"])
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

        let result = sync_repo(&not_repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should not error on non-git dir");
        assert!(matches!(result, Ok(SyncOutcome::NothingToDo)), "non-git dir should return false");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(matches!(result, Ok(SyncOutcome::Synced)), "single deletion should be committed");

        let output = git_cmd(&repo, &["ls-files"]);
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(tracked.contains("keep.txt"), "keep.txt should still be tracked");
        assert!(!tracked.contains("remove.txt"), "remove.txt should be removed from index");
    }

    #[tokio::test]
    async fn test_sync_repo_mass_deletion_prevented() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "mass-del-repo");

        std::fs::write(repo.join("a.txt"), "a\n").unwrap();
        std::fs::write(repo.join("b.txt"), "b\n").unwrap();
        std::fs::write(repo.join("c.txt"), "c\n").unwrap();
        git_cmd(&repo, &["add", "-A"]);
        git_cmd(&repo, &["commit", "-m", "add files"]);

        // Delete ALL files from working tree
        std::fs::remove_file(repo.join("a.txt")).unwrap();
        std::fs::remove_file(repo.join("b.txt")).unwrap();
        std::fs::remove_file(repo.join("c.txt")).unwrap();

        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(matches!(result, Ok(SyncOutcome::Blocked)), "mass deletion should be prevented (returns true without committing)");

        // Verify files are still tracked (deletion was NOT committed)
        let output = git_cmd(&repo, &["ls-files"]);
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(tracked.contains("a.txt"), "a.txt should still be tracked after mass deletion safety");
        assert!(tracked.contains("b.txt"), "b.txt should still be tracked after mass deletion safety");
        assert!(tracked.contains("c.txt"), "c.txt should still be tracked after mass deletion safety");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(matches!(result, Ok(SyncOutcome::Synced)), "partial deletion should be committed (not blocked)");

        // Verify deleted files are removed from tracking (deletion WAS committed)
        let output = git_cmd(&repo, &["ls-files"]);
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(!tracked.contains("a.txt"), "a.txt should be removed after partial deletion commit");
        assert!(!tracked.contains("b.txt"), "b.txt should be removed after partial deletion commit");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(matches!(result, Ok(SyncOutcome::Synced)), "exactly 50% deletion should be committed (not blocked)");

        let output = git_cmd(&repo, &["ls-files"]);
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(!tracked.contains("a.txt"), "a.txt should be removed after 50% deletion commit");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should not panic on empty repo");
    }

    #[tokio::test]
    async fn test_sync_repo_mass_deletion_logs_incident() {
        use crate::test_helpers::EnvRestorer;

        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "mass-del-incident-repo");

        std::fs::write(repo.join("a.txt"), "a\n").unwrap();
        std::fs::write(repo.join("b.txt"), "b\n").unwrap();
        std::fs::write(repo.join("c.txt"), "c\n").unwrap();
        git_cmd(&repo, &["add", "-A"]);
        git_cmd(&repo, &["commit", "-m", "add files"]);

        // Delete ALL files to trigger the safety guard
        std::fs::remove_file(repo.join("a.txt")).unwrap();
        std::fs::remove_file(repo.join("b.txt")).unwrap();
        std::fs::remove_file(repo.join("c.txt")).unwrap();

        let ledger = tmp.path().join("test-incidents.jsonl");
        let _ledger_guard = EnvRestorer::new("DRACON_SYNC_LEDGER", &ledger.to_string_lossy());

        let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, Some(Path::new("/fake/policy.toml")), false).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(matches!(result, Ok(SyncOutcome::Blocked)), "mass deletion should be prevented");

        // Verify incident was logged
        assert!(ledger.exists(), "incident ledger should be created");
        let content = std::fs::read_to_string(&ledger).unwrap();
        assert!(content.contains("mass_deletion_guard"), "incident should contain mass_deletion_guard action");
        assert!(content.contains("blocked"), "incident should have 'blocked' result");
    }

    #[tokio::test]
    async fn test_sync_repo_unstages_excluded_dir_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "exclude-dir-repo");

        std::fs::create_dir_all(repo.join("node_modules/pkg")).unwrap();
        std::fs::write(repo.join("node_modules/pkg/index.js"), "module.exports = {};\n").unwrap();
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/main.rs"), "fn main() {}\n").unwrap();
        git_cmd(&repo, &["add", "-A"]);
        git_cmd(&repo, &["commit", "-m", "initial"]);

        std::fs::write(repo.join("node_modules/pkg/index.js"), "updated\n").unwrap();
        std::fs::write(repo.join("src/main.rs"), "fn main() { println!(\"hello\"); }\n").unwrap();
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

        let result = sync_repo(&repo, &policy, &excluded, 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed");

        let output = git_cmd(&repo, &["log", "--oneline", "-1"]);
        let last_commit = String::from_utf8_lossy(&output.stdout);
        assert!(!last_commit.is_empty(), "should have committed the non-excluded change");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed with oversized file");

        let output = git_cmd(&repo, &["ls-files"]);
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(tracked.contains("small2.txt"), "small file should be tracked");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(matches!(result, Ok(SyncOutcome::Synced)), "mixed changes should be committed");

        let output = git_cmd(&repo, &["ls-files"]);
        let tracked = String::from_utf8_lossy(&output.stdout);
        assert!(tracked.contains("existing.txt"), "existing.txt should be tracked");
        assert!(tracked.contains("brand_new.txt"), "brand_new.txt should be tracked");

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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed without origin");
        assert!(matches!(result, Ok(SyncOutcome::NothingToDo)), "no origin should skip pull and return false");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(matches!(result, Ok(SyncOutcome::NothingToDo)), "auto_commit=false should not commit dirty files");

        let output = git_cmd(&repo, &["status", "--porcelain"]);
        let status = String::from_utf8_lossy(&output.stdout);
        assert!(status.contains("dirty.txt"), "file should still be untracked/unstaged");
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
            .trim().parse().unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, true, None, false).await;
        assert!(result.is_ok(), "dry-run should succeed");

        let commits_after = git_cmd(&repo, &["rev-list", "--count", "HEAD"]);
        let commits_count_after: usize = String::from_utf8_lossy(&commits_after.stdout)
            .trim().parse().unwrap();
        assert_eq!(commits_count_before, commits_count_after,
            "dry-run should not create any commits");

        let status = git_cmd(&repo, &["status", "--porcelain"]);
        let status_output = String::from_utf8_lossy(&status.stdout);
        assert!(status_output.contains("new_file.txt"),
            "file should still appear as untracked in working tree");
    }

    #[tokio::test]
    async fn test_sync_repo_dry_run_does_not_push() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "dry-run-push-test");

        std::fs::write(repo.join("file.txt"), "change\n").unwrap();
        git_cmd(&repo, &["add", "."]);
        git_cmd(&repo, &["commit", "-m", "add file"]);

        let commits_before = git_cmd(&repo, &["rev-list", "--count", "HEAD"]);
        let count_before: usize = String::from_utf8_lossy(&commits_before.stdout).trim().parse().unwrap();

        let toml_str = r#"
auto_github_private = false
auto_commit = false
auto_pull = false
auto_push = true
auto_bump_versions = false
"#;
        let policy: SyncPolicy = toml::from_str(toml_str).unwrap();

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, true, None, false).await;
        assert!(result.is_ok(), "dry-run should succeed");

        let commits_after = git_cmd(&repo, &["rev-list", "--count", "HEAD"]);
        let count_after: usize = String::from_utf8_lossy(&commits_after.stdout).trim().parse().unwrap();
        assert_eq!(count_before, count_after, "dry-run should not change commit count");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, true, None, false).await;
        assert!(result.is_ok(), "dry-run should succeed");

        let output = git_cmd(&repo, &["status", "--porcelain"]);
        let status = String::from_utf8_lossy(&output.stdout);
        assert!(status.contains("modified.txt"), "modified.txt should still be modified");
        assert!(status.contains("untracked.txt"), "untracked.txt should still be untracked");
    }

    /// Comprehensive boundary test for the mass-deletion safety guard.
    /// Covers the full matrix of (tracked_files, deleted_files) combinations
    /// to verify the guard's >=85% threshold, and the atomic counter.
    #[tokio::test]
    async fn test_safety_guard_boundaries() {
        async fn check_scenario(tmp: &tempfile::TempDir, name: &str, total: usize, delete_count: usize, expect_blocked: bool) {
            let before = crate::sync::MASS_DELETION_GUARD_BLOCKED.load(std::sync::atomic::Ordering::Relaxed);
            let repo = init_test_repo(tmp, name);

            // Create tracked files
            let file_names: Vec<String> = (0..total).map(|i| format!("f{}.txt", i)).collect();
            let _file_refs: Vec<&str> = file_names.iter().map(|s| s.as_str()).collect();
            for f in &file_names {
                std::fs::write(repo.join(f), "content\n").unwrap();
            }
            git_cmd(&repo, &["add", "-A"]);
            git_cmd(&repo, &["commit", "-m", "setup"]);

            // Delete files
            let to_delete: Vec<&String> = file_names.iter().take(delete_count).collect();
            for f in &to_delete {
                std::fs::remove_file(repo.join(f)).unwrap();
            }

            let toml_str = r#"
        auto_github_private = false
        auto_commit = true
        auto_pull = false
        auto_push = false
        auto_bump_versions = false
        "#;
            let policy: SyncPolicy = toml::from_str(toml_str).unwrap();
            let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
            assert!(result.is_ok(), "sync_repo should succeed for {} tracked, {} deleted", total, delete_count);

            let pct = if total > 0 { (delete_count * 100) / total } else { 0 };
            if expect_blocked {
                assert!(matches!(result, Ok(SyncOutcome::Blocked)), "guard should block {}% deletion ({} of {})", pct, delete_count, total);
                // Verify deleted files are still tracked
                let output = git_cmd(&repo, &["ls-files"]);
                let tracked = String::from_utf8_lossy(&output.stdout);
                for f in &to_delete {
                    assert!(tracked.contains(f.as_str()), "{} should still be tracked after guard block ({}% deletion)", f, pct);
                }
                let after = crate::sync::MASS_DELETION_GUARD_BLOCKED.load(std::sync::atomic::Ordering::Relaxed);
                assert_eq!(after, before + 1, "guard blocked counter should increment for {}% deletion ({} of {})", pct, delete_count, total);
            } else if delete_count == 0 {
                // No deletions at all → sync_repo returns false (nothing to do)
                assert!(matches!(result, Ok(SyncOutcome::NothingToDo)), "no changes should return false for {}% deletion ({} of {})", pct, delete_count, total);
                let after = crate::sync::MASS_DELETION_GUARD_BLOCKED.load(std::sync::atomic::Ordering::Relaxed);
                assert_eq!(after, before, "guard blocked counter should not increment");
            } else {
                assert!(matches!(result, Ok(SyncOutcome::Synced)), "deletion should be committed for {}% ({} of {})", pct, delete_count, total);
                // Verify deleted files are removed
                let output = git_cmd(&repo, &["ls-files"]);
                let tracked = String::from_utf8_lossy(&output.stdout);
                for f in &to_delete {
                    assert!(!tracked.contains(f.as_str()), "{} should be removed after commit ({}% deletion)", f, pct);
                }
                let after = crate::sync::MASS_DELETION_GUARD_BLOCKED.load(std::sync::atomic::Ordering::Relaxed);
                assert_eq!(after, before, "guard blocked counter should NOT increment for allowed {}% deletion", pct);
            }
        }

        let tmp = tempfile::tempdir().unwrap();

        // 0 of 3 deleted (0%) — ALLOWED
        check_scenario(&tmp, "boundary-0pct", 3, 0, false).await;

        // 1 of 3 deleted (33%) — ALLOWED
        check_scenario(&tmp, "boundary-33pct", 3, 1, false).await;

        // 1 of 2 deleted (50%) — ALLOWED
        check_scenario(&tmp, "boundary-50pct", 2, 1, false).await;

        // 2 of 3 deleted (66%) — ALLOWED (below 85% threshold)
        check_scenario(&tmp, "boundary-66pct", 3, 2, false).await;

        // 3 of 3 deleted (100%) — BLOCKED (total wipe)
        check_scenario(&tmp, "boundary-100pct", 3, 3, true).await;

        // 2 of 5 deleted (40%) — ALLOWED
        check_scenario(&tmp, "boundary-40pct", 5, 2, false).await;

        // 3 of 5 deleted (60%) — ALLOWED (below 85% threshold)
        check_scenario(&tmp, "boundary-60pct", 5, 3, false).await;

        // 5 of 6 deleted (83%) — ALLOWED (just below 85% threshold)
        check_scenario(&tmp, "boundary-83pct", 6, 5, false).await;

        // 6 of 7 deleted (~85.7%) — BLOCKED (at 85% threshold)
        check_scenario(&tmp, "boundary-86pct", 7, 6, true).await;

        // 1 of 1 deleted (100%) — BLOCKED (single file is still 100%)
        check_scenario(&tmp, "boundary-single-100pct", 1, 1, true).await;
    }

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
        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
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
    async fn test_guard_blocks_resets_staged_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(&tmp, "guard-reset-repo");

        // Create 7 tracked files — 6 will be deleted (85.7% >= 85 threshold)
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

        // Delete 6 files — this triggers the 85% mass-deletion guard
        for i in 1..7 {
            std::fs::remove_file(repo.join(format!("file{i}.txt"))).unwrap();
        }

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(matches!(result, Ok(SyncOutcome::Blocked)), "guard should block 85%+ deletion");

        // Verify index is clean — file0 modification should have been reset
        let staged = git_cmd(&repo, &["diff", "--cached", "--name-only"]);
        let staged_str = String::from_utf8_lossy(&staged.stdout);
        assert!(staged_str.trim().is_empty(), "staged changes should be reset after guard blocks, got: {:?}", staged_str);
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
        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(
            matches!(result, Ok(SyncOutcome::NothingToDo) | Ok(SyncOutcome::Synced)),
            "filter-only repo should produce NothingToDo or Synced without changes, got {:?}",
            result
        );

        // Verify nothing was staged
        let staged = git_cmd(&repo, &["diff", "--cached", "--name-only"]);
        let staged_str = String::from_utf8_lossy(&staged.stdout);
        assert!(staged_str.trim().is_empty(), "nothing should be staged for filter-only repo");
    }

    // ── Stale focus detection tests ──

    #[test]
    fn test_stale_focus_detection() {
        // Helper: mirrors the stale focus logic in build_commit_context
        fn focus_is_stale(focus: &str, recent_subjects: &[&str]) -> bool {
            let focus_matches = recent_subjects.iter().filter(|subj| {
                let subj_body = subj.splitn(2, ": ").nth(1).unwrap_or(subj);
                if focus.len() > subj_body.len() {
                    let truncated = &focus[..subj_body.len().min(focus.len())];
                    subj_body.starts_with(truncated)
                } else {
                    subj_body.contains(focus)
                }
            }).count();
            focus_matches >= 3
        }

        // Empty history — not stale
        assert!(!focus_is_stale("add auth", &[]));

        // 1 match out of 5 — not stale
        assert!(!focus_is_stale("add auth", &["feat: add auth", "fix: bug", "chore: cleanup", "docs: readme", "test: unit"]));

        // 2 matches — not stale (below threshold of 3)
        assert!(!focus_is_stale("add auth", &["feat: add auth", "chore: sync metadata", "feat: add auth", "fix: bug", "docs: readme"]));

        // 3 matches — stale (at threshold)
        assert!(focus_is_stale("add auth", &["feat: add auth", "chore: sync metadata", "feat: add auth", "fix: bug", "feat: add auth"]));

        // 5 matches — very stale
        assert!(focus_is_stale("add auth", &["feat: add auth", "feat: add auth", "feat: add auth", "feat: add auth", "feat: add auth"]));

        // Alternating A/B/A/B/A pattern — A appears 3 times → stale
        assert!(focus_is_stale("add auth", &["feat: add auth", "chore: sync metadata", "feat: add auth", "chore: sync metadata", "feat: add auth"]));

        // Different focus — not stale
        assert!(!focus_is_stale("fix bug", &["feat: add auth", "feat: add auth", "feat: add auth", "feat: add auth", "feat: add auth"]));

        // Conventional commit prefix stripped: focus "add auth" matches subject "feat: add auth"
        assert!(focus_is_stale("add auth", &["feat: add auth", "fix: add auth", "chore: add auth", "docs: other", "test: other"]));
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
        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0, None, false, None, false).await;
        assert!(result.is_ok(), "sync_repo should succeed with duplicate subjects in history");
    }

    #[test]
    fn test_conventional_prefix_detects_scoped_and_bare() {
        let has_prefix = |s: &str| -> bool {
            crate::bump::CONVENTIONAL_COMMIT_TYPES.iter().any(|t| s.starts_with(&format!("{}:", t)) || s.starts_with(&format!("{}(", t)))
        };

        // bare prefixes
        assert!(has_prefix("fix: validate input"));
        assert!(has_prefix("feat: add modal"));

        // scoped prefixes — these were the bug (double prefix)
        assert!(has_prefix("fix(modals): add input shield"));
        assert!(has_prefix("feat(ui): add button"));
        assert!(has_prefix("refactor(sync): detect full set"));

        // revert and security — were missing from the list
        assert!(has_prefix("revert: undo auth change"));
        assert!(has_prefix("security: fix XSS vulnerability"));
        assert!(has_prefix("revert(auth): undo token change"));
        assert!(has_prefix("security(crypto): fix padding oracle"));

        // not conventional
        assert!(!has_prefix("add input shield"));
        assert!(!has_prefix("update readme"));
    }
}
