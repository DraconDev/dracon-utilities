use anyhow::Result;
use dracon_common::{emit_event, DraconEvent, EventSeverity};
use dracon_git::{build_commit_message, extract_intent, CommitContext, GitService};
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use crate::bump::{bump_node_package_version_in_repo, bump_patch_version_in_repo, bump_version_file_in_repo};
use crate::exclude::{can_restore_entry, handle_large_untracked, is_large_untracked, should_stage_entry};
use crate::git::{
    cli_diff_entries, detect_large_blobs_ahead, git_name_status_entries, has_origin_remote,
    has_tracking_upstream, is_cherry_pick_in_progress, is_merge_in_progress,
    is_rebase_in_progress, run_cmd_with_timeout, run_git_with_timeout, staged_paths,
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
                emit_event(&DraconEvent::new("sync", EventSeverity::Error, format!("pull/{}", repo.display()), "merge conflict - manual intervention required"));
                return Ok(false);
            }
            Ok(Err(e)) => {
                eprintln!("⚠️ pull/rebase failed for {}: {} - aborting sync pass", repo.display(), e);
                emit_event(&DraconEvent::new("sync", EventSeverity::Warn, format!("pull/{}", repo.display()), format!("failed: {e}")));
                return Ok(false);
            }
            Err(_) => {
                eprintln!(
                    "⚠️ pull/rebase timeout for {} after {}s - aborting sync pass",
                    repo.display(),
                    policy.pull_op_timeout_secs
                );
                emit_event(&DraconEvent::new("sync", EventSeverity::Warn, format!("pull/{}", repo.display()), format!("timeout after {}s", policy.pull_op_timeout_secs)));
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

    let mut status = svc.get_status().await?;
    let mut entries = svc.get_diff_entries().await?;
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
        let (to_stage, to_restore): (Vec<_>, Vec<_>) = entries
            .into_iter()
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

            svc.add_paths(&stage_paths).await?;

            // Optional: bump patch versions, then stage any files we touched (best-effort).
            if auto_bump_versions {
                let outcome = bump_patch_version_in_repo(repo)?;
                if outcome.bumped_cargo_toml {
                    let _ = run_git_with_timeout(repo, &["add", "Cargo.toml"], 30, "add").await;
                }
                if outcome.updated_cargo_lock {
                    let _ = run_git_with_timeout(repo, &["add", "Cargo.lock"], 30, "add").await;
                }
                if outcome.bumped_workspace_package && repo.join("Cargo.lock").exists() {
                    // Workspace version bumps will cause Cargo.lock churn until it's regenerated.
                    // Do it immediately so we never end up with a follow-up Cargo.lock-only commit.
                    match run_cmd_with_timeout(
                        repo,
                        "cargo",
                        &["generate-lockfile"],
                        180,
                        "generate-lockfile",
                    )
                    .await
                    {
                        Ok(()) => {
                            let _ =
                                run_git_with_timeout(repo, &["add", "Cargo.lock"], 30, "add").await;
                        }
                        Err(e) => {
                            eprintln!(
                                "⚠️ {}: failed to refresh Cargo.lock after workspace version bump: {}",
                                repo.display(),
                                e
                            );
                        }
                    }
                }

                // Node/TS: package.json (+ optional package-lock.json alignment).
                let outcome = bump_node_package_version_in_repo(repo)?;
                if outcome.bumped {
                    let _ = run_git_with_timeout(repo, &["add", "package.json"], 30, "add").await;
                }
                if outcome.updated_lock {
                    let _ =
                        run_git_with_timeout(repo, &["add", "package-lock.json"], 30, "add").await;
                }

                // Generic: VERSION file.
                if bump_version_file_in_repo(repo)? {
                    let _ = run_git_with_timeout(repo, &["add", "VERSION"], 30, "add").await;
                }
            }

            // Build the payload from what we're actually going to commit (cached diff),
            // so version bumps don't silently add files not reflected in the JSON.
            let staged = git_name_status_entries(repo, &["diff", "--cached", "--name-status"]).await?;
            let committed_entries: Vec<dracon_git::types::DiffFile> = staged
                .into_iter()
                .map(|(path, status)| dracon_git::types::DiffFile { path, status })
                .collect();

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
            emit_event(&DraconEvent::new("sync", EventSeverity::Info, format!("commit/{}", repo.display()), format!("committed {} file(s)", committed_entries.len())));

            // Scribe: update project-state.md via AI (if configured)
            if cfg!(feature = "scribe") {
                #[cfg(feature = "scribe")]
                if let Err(e) = crate::scribe::update_project_state_from_ai(repo).await {
                    eprintln!("📝 scribe failed for {}: {}", repo.display(), e);
                }
            }

            // Restore any excluded modified paths that weren't committed
            let restorable: Vec<_> = to_restore.iter().filter(|e| can_restore_entry(e)).collect();

            handle_large_untracked(repo, &to_restore, policy)?;

            let other_untracked: Vec<_> = to_restore
                .iter()
                .filter(|e| !can_restore_entry(e) && !is_large_untracked(e, repo, policy.max_stage_file_bytes))
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
                        eprintln!("⚠️ push skipped for {}: {}", repo.display(), e);
                        emit_event(&DraconEvent::new("sync", EventSeverity::Warn, format!("push/{}", repo.display()), format!("failed: {e}")));
                    }
                }
            } else if policy.auto_push && !has_origin {
                eprintln!("ℹ️ skip push for {} (no origin remote)", repo.display());
            }
            return Ok(true);
        }
        // All changes were filtered out (excluded dirs, oversized files, etc.)
        // Restore modified files to avoid perpetual dirty state. Untracked files can't be restored.
        let restorable: Vec<_> = to_restore.iter().filter(|e| can_restore_entry(e)).collect();
        let gitignore_updated = handle_large_untracked(repo, &to_restore, policy)?;

        let other_untracked: Vec<_> = to_restore
            .iter()
            .filter(|e| !can_restore_entry(e) && !is_large_untracked(e, repo, policy.max_stage_file_bytes))
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
                                        let _ = run_git_with_timeout(
                                            repo,
                                            &["push", "origin", "HEAD"],
                                            policy.push_op_timeout_secs,
                                            "push",
                                        )
                                        .await;
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
            Err(e) => eprintln!("⚠️ push skipped for {}: {}", repo.display(), e),
        }
    } else if policy.auto_push && current_status.ahead > 0 && !has_origin {
        eprintln!("ℹ️ skip push for {} (no origin remote)", repo.display());
    }

    Ok(false)
}
