use anyhow::Result;
use dracon_git::{build_commit_message, extract_intent, GitService};
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use crate::exclude::{can_restore_entry, handle_large_untracked, is_large_untracked, remove_tracked_excluded_paths, should_stage_entry};
use crate::git::{
    cli_diff_entries, detect_large_blobs_ahead, git_name_status_entries, has_origin_remote,
    has_tracking_upstream, is_cherry_pick_in_progress, is_merge_in_progress,
    is_rebase_in_progress, prune_other_default_branch, restore_paths, run_git_with_timeout, staged_paths,
    unstage_excluded_paths, unstage_oversized_paths,
};
use crate::policy::{debug_enabled, load_repo_override, SyncPolicy};
use crate::report::{build_commit_context, detect_report_signals, push_large_blob_threshold_bytes};

pub(crate) async fn sync_repo(
    repo: &Path,
    policy: &SyncPolicy,
    excluded_dir_names: &BTreeSet<String>,
    idle_seconds: u64,
) -> Result<bool> {
    let svc = GitService::new(repo)?;
    if !svc.is_git_repo().await? {
        if debug_enabled() {
            eprintln!("🐛 {} is not recognized as git repo", repo.display());
        }
        return Ok(false);
    }

    // Bail out early if repo is in a conflict state - manual intervention required
    if is_rebase_in_progress(repo) {
        eprintln!("⚠️ {} has rebase in progress, skipping (manual intervention required)", repo.display());
        return Ok(false);
    }
    if is_merge_in_progress(repo) {
        eprintln!("⚠️ {} has merge in progress, skipping (manual intervention required)", repo.display());
        return Ok(false);
    }
    if is_cherry_pick_in_progress(repo) {
        eprintln!("⚠️ {} has cherry-pick in progress, skipping (manual intervention required)", repo.display());
        return Ok(false);
    }

    let has_origin = has_origin_remote(repo);
    let has_origin = if !has_origin && policy.auto_github_private {
        if let Some(url) = crate::report::create_github_private_remote(repo, &policy.auto_github_private_account) {
            println!("🔗 created remote for {}: {}", repo.display(), url);
            true
        } else {
            eprintln!("⚠️ failed to create GitHub remote for {}", repo.display());
            false
        }
    } else {
        has_origin
    };
    let has_upstream = has_tracking_upstream(repo);
    let blob_threshold = push_large_blob_threshold_bytes(policy);
    let initial_status = svc.get_status().await?;

    // Optional per-repo overrides (untracked local settings).
    // Path: `<repo>/.dracon/dracon-sync.toml`
    let repo_override = load_repo_override(repo);
    let auto_bump_versions = repo_override
        .auto_bump_versions
        .unwrap_or(policy.auto_bump_versions);

    if policy.auto_pull && has_origin && has_upstream && initial_status.behind > 0 && initial_status.is_clean {
        match tokio::time::timeout(
            Duration::from_secs(policy.pull_op_timeout_secs),
            svc.pull_rebase(),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(dracon_git::error::GitError::MergeConflict)) => {
                eprintln!("⚠️ pull/rebase conflict in {} (manual intervention required)", repo.display());
                return Ok(false);
            }
            Ok(Err(e)) => {
                eprintln!("⚠️ pull/rebase failed for {}: {} - aborting sync pass", repo.display(), e);
                return Ok(false);
            }
            Err(_) => {
                eprintln!(
                    "⚠️ pull/rebase timeout for {} after {}s - aborting sync pass",
                    repo.display(),
                    policy.pull_op_timeout_secs
                );
                return Ok(false);
            }
        }
    } else if policy.auto_pull && has_origin && has_upstream && initial_status.behind == 0 {
        if debug_enabled() {
            eprintln!(
                "🐛 skip pull/rebase for {} (branch not behind upstream)",
                repo.display()
            );
        }
    } else if policy.auto_pull && has_origin && has_upstream && !initial_status.is_clean {
        if debug_enabled() {
            eprintln!(
                "🐛 skip pull/rebase for {} (dirty repo, commit first)",
                repo.display()
            );
        }
    } else if policy.auto_pull && !has_origin {
        eprintln!(
            "ℹ️ skip pull/rebase for {} (no origin remote)",
            repo.display()
        );
    } else if policy.auto_pull && has_origin && !has_upstream {
        eprintln!(
            "ℹ️ skip pull/rebase for {} (no tracking upstream on current branch)",
            repo.display()
        );
    }

    let unstaged = unstage_excluded_paths(repo, excluded_dir_names).await?;
    if unstaged > 0 {
        eprintln!(
            "🧹 removed {} staged excluded paths in {}",
            unstaged,
            repo.display()
        );
    }
    let unstaged_oversized = unstage_oversized_paths(repo, policy.max_stage_file_bytes).await?;
    if unstaged_oversized > 0 {
        eprintln!(
            "🧹 removed {} oversized staged paths in {}",
            unstaged_oversized,
            repo.display()
        );
    }

    // Remove any tracked files that live inside excluded directories
    // (e.g. build artifacts that were accidentally committed). This also
    // adds the directory pattern to .gitignore so it won't be re-tracked.
    if let Some(removed_dirs) = remove_tracked_excluded_paths(repo, excluded_dir_names)? {
        if !removed_dirs.is_empty() {
            eprintln!(
                "🧹 removed {} tracked excluded dir(s) from {}: {:?}",
                removed_dirs.len(),
                repo.display(),
                removed_dirs
            );
        }
    }

    let mut status = svc.get_status().await?;
    let mut entries = svc.get_diff_entries().await?;

    // Filter out entries that only differ due to clean/smudge filters.
    // `git diff HEAD` applies clean filters and correctly ignores filter-only changes.
    {
        let diff_output = crate::git::git_diff_head_files(repo).await;
        if diff_output.is_empty() && !entries.is_empty() {
            // git diff HEAD returned nothing. Only clear if ALL entries are
            // Modified (filter-only). Untracked/Added files don't appear in
            // git diff HEAD, so they should still be processed.
            let has_non_modified = entries.iter().any(|e| {
                !matches!(e.status, dracon_git::types::FileStatus::Modified)
            });
            if !has_non_modified {
                entries.clear();
                status.is_clean = true;
            }
        } else {
            entries.retain(|e| {
                if !matches!(e.status, dracon_git::types::FileStatus::Modified) {
                    return true;
                }
                diff_output.contains(&e.path.to_string_lossy().to_string())
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
    if entries.is_empty() {
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

    if !status.is_clean && policy.auto_commit {
        let entries_len = entries.len();
        // Partition into stage/restore/ignore. Gitlinks with unchanged pointers
        // should be ignored entirely (they appear dirty but can't be staged or restored).
        let (to_stage, to_restore): (Vec<_>, Vec<_>) = entries
            .into_iter()
            .filter(|e| {
                // Skip gitlink entries with unchanged pointers entirely
                // Use repo.join() because e.path is relative to repo, not CWD
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
            let filtered_entries = to_stage;
            let stage_paths: Vec<String> = filtered_entries
                .iter()
                .map(|e| e.path.to_string_lossy().to_string())
                .collect();

            let (existing, missing): (Vec<_>, Vec<_>) = stage_paths
                .into_iter()
                .partition(|p| repo.join(p).exists());

            if !existing.is_empty() {
                let mut add_args = vec!["add", "-A", "-f", "--"];
                for p in &existing {
                    add_args.push(p);
                }
                if let Err(e) = run_git_with_timeout(repo, &add_args, 30, "add").await {
                    eprintln!("⚠️ {} git add failed for {} paths: {:?}", repo.display(), existing.len(), existing);
                    return Err(e);
                }
            }

            if !missing.is_empty() {
                // SAFETY: If ALL files in the index are missing, this is likely a mistake
                // or destructive operation. Do NOT stage mass deletions without warning.
                // Get total tracked files count
                let total_tracked: usize = std::process::Command::new("git")
                    .args(["ls-files", "--count"])
                    .current_dir(repo)
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
                    .unwrap_or(0);

                let missing_count = missing.len();
                let is_mass_deletion = total_tracked > 0 && missing_count >= total_tracked;

                if is_mass_deletion {
                    eprintln!("⚠️ SAFETY: {} files missing from working tree ({}% of {} tracked)", missing_count, (missing_count * 100) / total_tracked, total_tracked);
                    eprintln!("⚠️ Refusing to stage mass deletion - this looks like a mistake or destructive operation");
                    eprintln!("⚠️ If you really want to delete all files, do: git add -A && git commit -m 'delete all'");
                    // Do NOT stage the deletions - let the user decide
                    return Ok(true);
                }

                let mut rm_args = vec!["rm", "--ignore-unmatch", "--"];
                for p in &missing {
                    rm_args.push(p);
                }
                if let Err(e) = run_git_with_timeout(repo, &rm_args, 30, "rm").await {
                    eprintln!("⚠️ {} git rm failed for {} paths: {:?}", repo.display(), missing.len(), missing);
                    return Err(e);
                }
            }

            // Build the payload from what we're actually going to commit (cached diff)
            let staged = git_name_status_entries(repo, &["diff", "--cached", "--name-status"]).await?;
            let committed_entries: Vec<dracon_git::types::DiffFile> = staged
                .into_iter()
                .map(|(path, status)| dracon_git::types::DiffFile { path, status })
                .collect();

            // Scribe: update project-state.md via AI BEFORE building commit context
            // so the fresh state is included in the commit body
            let staged_diff_names = committed_entries.iter()
                .map(|e| format!("{:?}: {}", e.status, e.path.display()))
                .collect::<Vec<_>>()
                .join("\n");

            // Get actual diff content for the scribe (stat + limited patch)
            let staged_diff_content: Option<String> = {
                let stat_out = std::process::Command::new("git")
                    .args(["diff", "--cached", "--stat"])
                    .current_dir(repo)
                    .output();
                match stat_out {
                    Ok(o) if o.status.success() => {
                        let stat = String::from_utf8_lossy(&o.stdout).to_string();
                        if stat.is_empty() {
                            None
                        } else {
                            let patch_out = std::process::Command::new("git")
                                .args(["diff", "--cached", "--unified=3", "--"])
                                .current_dir(repo)
                                .output();
                            let patch_text = match patch_out {
                                Ok(o) if o.status.success() => {
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
            };

            if cfg!(feature = "scribe") {
                #[cfg(feature = "scribe")]
                if let Err(e) = crate::scribe::update_project_state_from_ai(repo, &staged_diff_names, staged_diff_content).await {
                    eprintln!("📝 scribe failed for {}: {}", repo.display(), e);
                }
            }

            // Stage project-state.md if scribe updated it
            // Use -f (force) because project-state.md is in .dracon/ which is
            // typically gitignored, so we need to override that.
            if repo.join(".dracon/project-state.md").exists() {
                if let Err(e) = run_git_with_timeout(repo, &["add", "-f", ".dracon/project-state.md"], 10, "add-project-state").await {
                    eprintln!("⚠️ failed to stage project-state: {}", e);
                }
            }

            // Version bumper: deterministic patch-only (fallback when ai-bumper not enabled)
            if auto_bump_versions && cfg!(feature = "scribe") {
                #[cfg(feature = "scribe")]
                {
                    use crate::bump::{deterministic_decide_bump_level, bump_semver_patch, read_current_version, BumpLevel};
                    
                    let staged_diff = committed_entries.iter()
                        .map(|e| format!("{:?}: {}", e.status, e.path.display()))
                        .collect::<Vec<_>>()
                        .join("\n");
                    
                    if let Some(current_ver) = read_current_version(repo) {
                        let level = deterministic_decide_bump_level(&staged_diff);
                        if level != BumpLevel::None {
                            eprintln!("📦 bump: {} -> patch", current_ver);
                            if let Some(new_ver) = bump_semver_patch(&current_ver) {
                                let bumped = crate::bump::apply_version_bump_to_repo(repo, &current_ver, &new_ver);
                                if bumped {
                                    for file in &["Cargo.toml", "package.json", "VERSION", "Cargo.lock"] {
                                        if repo.join(file).exists() {
                                            if let Err(e) = run_git_with_timeout(repo, &["add", file], 30, "add").await {
                                                eprintln!("⚠️ failed to stage {}: {}", file, e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // AI version bumper: decides IF and what level to bump (when ai-bumper feature enabled)
            if auto_bump_versions && cfg!(feature = "ai-bumper") {
                #[cfg(feature = "ai-bumper")]
                {
                    use crate::bump::{ai_decide_bump_level, bump_semver_major, bump_semver_minor, bump_semver_patch, read_current_version, BumpLevel};
                    
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
                            let new_ver = match level {
                                BumpLevel::Major => bump_semver_major(&current_ver),
                                BumpLevel::Minor => bump_semver_minor(&current_ver),
                                BumpLevel::Patch => bump_semver_patch(&current_ver),
                                BumpLevel::None => None,
                            };
                            
                            if let Some(new_ver) = new_ver {
                                let bumped = crate::bump::apply_version_bump_to_repo(repo, &current_ver, &new_ver);
                                if bumped {
                                    for file in &["Cargo.toml", "package.json", "VERSION", "Cargo.lock"] {
                                        if repo.join(file).exists() {
                                            if let Err(e) = run_git_with_timeout(repo, &["add", file], 30, "add").await {
                                                eprintln!("⚠️ failed to stage {}: {}", file, e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Re-get staged entries after potential version bump
            let staged = git_name_status_entries(repo, &["diff", "--cached", "--name-status"]).await?;
            let committed_entries: Vec<dracon_git::types::DiffFile> = staged
                .into_iter()
                .map(|(path, status)| dracon_git::types::DiffFile { path, status })
                .collect();

            // If nothing is staged (all changes were filter-only, e.g. smudge filter
            // decrypting encrypted files), skip commit entirely. The working tree will
            // still appear "dirty" due to the smudge filter, which is harmless.
            if committed_entries.is_empty() {
                if let Err(e) = run_git_with_timeout(repo, &["reset", "HEAD", "--"], 10, "reset").await {
                    return Err(anyhow::anyhow!("sync_repo: failed to reset HEAD after filter-only commit: {}", e));
                }
                if debug_enabled() {
                    eprintln!("🐛 {} skipped commit: all changes were filter-only (smudge/clean)", repo.display());
                }
                return Ok(true);
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

            // Stable identity subject with rich JSON body.
            let msg = build_commit_message(&ctx);

            svc.commit(&msg).await?;
            eprintln!("📝 committed {} file(s) in {}", committed_entries.len(), repo.display());

            // Forbid creation of the "other" default branch (main vs master).
            // If someone or something created the non-canonical branch, delete it.
            prune_other_default_branch(repo);

            // Restore any excluded modified paths that weren't committed
            // Skip gitlink entries (dirty submodules can't be restored this way)
            let restorable: Vec<_> = to_restore.iter()
                .filter(|e| can_restore_entry(repo, e))
                .filter(|e| !repo.join(&e.path).is_dir() || !crate::exclude::is_gitlink_unchanged(repo, &e.path))
                .collect();

            handle_large_untracked(repo, &to_restore, policy)?;

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

            if policy.auto_push && has_origin {
                let ahead_large = match detect_large_blobs_ahead(repo, blob_threshold) {
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
                match run_git_with_timeout(
                    repo,
                    &["push", "origin", "HEAD"],
                    policy.push_op_timeout_secs,
                    "push",
                )
                .await
                {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("⚠️ push failed for {}: {}", repo.display(), e);
                        return Ok(false);
                    }
                }
            } else if policy.auto_push && !has_origin {
                eprintln!("ℹ️ skip push for {} (no origin remote)", repo.display());
            }
            return Ok(true);
        }
        // All changes were filtered out (excluded dirs, oversized files, etc.)
        // Restore modified files to avoid perpetual dirty state. Untracked files can't be restored.
        // Skip gitlink entries (dirty submodules can't be restored this way)
        let restorable: Vec<_> = to_restore.iter()
            .filter(|e| can_restore_entry(repo, e))
            .filter(|e| !repo.join(&e.path).is_dir() || !crate::exclude::is_gitlink_unchanged(repo, &e.path))
            .collect();
        let gitignore_updated = handle_large_untracked(repo, &to_restore, policy)?;

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
                "🧹 restoring {} excluded path(s) in {} (all changes filtered)",
                excluded_paths.len(),
                repo.display()
            );
            restore_paths(repo, &excluded_paths).await?;
            return Ok(true);
        }

        // If we updated .gitignore, commit it so the repo becomes clean
        if gitignore_updated && policy.auto_commit {
            let gitignore_path = ".gitignore";
            match run_git_with_timeout(repo, &["add", gitignore_path], 30, "add").await {
                Ok(()) => {
                    // Check if there's anything staged now
                    if let Ok(staged) = staged_paths(repo).await {
                        if !staged.is_empty() {
                            let msg = format!("[{}] update .gitignore",
                                extract_intent(repo, &[], Some(&status.branch)).intent);
                            match svc.commit(&msg).await {
                                Ok(()) => {
                                    eprintln!("📝 committed .gitignore update in {}", repo.display());
                                    if policy.auto_push && has_origin {
                                    match run_git_with_timeout(
                                        repo,
                                        &["push", "origin", "HEAD"],
                                        policy.push_op_timeout_secs,
                                        "push",
                                    )
                                    .await
                                    {
                                        Ok(()) => {}
                                        Err(e) => {
                                            eprintln!("⚠️ push failed for {}: {}", repo.display(), e);
                                            return Ok(false);
                                        }
                                    }
                                }
                                return Ok(true);
                                }
                                Err(e) => eprintln!("⚠️ failed to commit .gitignore in {}: {}", repo.display(), e),
                            }
                        }
                    }
                }
                Err(e) => eprintln!("⚠️ failed to stage .gitignore in {}: {}", repo.display(), e),
            }
        }

        // Dirty repo with entries but none passed filters and none restorable
        if entries_len > 0 && !gitignore_updated {
            eprintln!(
                "ℹ️ {} has {} dirty entries but none restorable (all untracked or excluded)",
                repo.display(),
                entries_len
            );
        }
    }

    // Re-fetch status for push decision (may have changed after pull/commit)
    let current_status = svc.get_status().await?;
    if policy.auto_push && current_status.ahead > 0 && has_origin {
        let ahead_large = match detect_large_blobs_ahead(repo, blob_threshold) {
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
        match run_git_with_timeout(
            repo,
            &["push", "origin", "HEAD"],
            policy.push_op_timeout_secs,
            "push",
        )
        .await
        {
            Ok(()) => {}
            Err(e) => {
                eprintln!("⚠️ push skipped for {}: {}", repo.display(), e);
                return Ok(false);
            }
        }
    } else if policy.auto_push && current_status.ahead > 0 && !has_origin {
        eprintln!("ℹ️ skip push for {} (no origin remote)", repo.display());
    }

    Ok(false)
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
        assert!(result.is_ok(), "sync_repo should handle missing gh gracefully: {:?}", result);

        assert!(
            !has_origin_remote(&repo),
            "no remote should exist when gh is unavailable"
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
        assert!(result.is_ok(), "sync_repo should succeed even during rebase");
        assert!(!result.unwrap(), "rebase should cause early return (nothing synced)");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
        assert!(result.is_ok(), "sync_repo should succeed even during merge");
        assert!(!result.unwrap(), "merge should cause early return (nothing synced)");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
        assert!(result.is_ok(), "sync_repo should succeed even during cherry-pick");
        assert!(!result.unwrap(), "cherry-pick should cause early return (nothing synced)");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
        assert!(result.is_ok(), "sync_repo should succeed: {:?}", result);
        assert!(result.unwrap(), "dirty repo with auto_commit should sync");

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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(!result.unwrap(), "clean repo should return false (nothing to sync)");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
        assert!(result.is_ok(), "sync_repo should succeed: {:?}", result);
        assert!(result.unwrap(), "untracked file should be staged and committed");

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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
        assert!(result.is_ok(), "sync_repo should succeed");
        assert!(!result.unwrap(), "not behind should return false (nothing to pull)");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
        assert!(result.is_ok(), "sync_repo should succeed with dirty repo");
        assert!(!result.unwrap(), "dirty repo should skip pull and return false");
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
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

        let result = sync_repo(&repo, &policy, &BTreeSet::new(), 0).await;
        assert!(result.is_ok(), "sync_repo should succeed without upstream");
    }
}
