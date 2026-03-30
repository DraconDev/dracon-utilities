use anyhow::Result;
use dracon_git::GitService;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::time::sleep;

use crate::policy::{SyncPolicy, freeze_reason};
use crate::exclude::{excluded_dir_names_set, has_sync_relevant_dirty_entries};
use crate::git::{discover_git_repos, repo_diff_entries, has_origin_remote, has_tracking_upstream};
use crate::report::{ConcernRepairFilter, RepairSummary, run_repair_concerns, run_repair_warns};
use crate::sync::sync_repo;

pub(crate) async fn run_once(policy_path: &Path) -> Result<()> {
    if let Some(reason) = freeze_reason(policy_path) {
        println!("⏸️ sync frozen ({})", reason);
        return Ok(());
    }

    let policy = SyncPolicy::load(policy_path)?;
    let roots = policy.watch_root_paths();
    let excluded_dir_names = excluded_dir_names_set(&policy);
    let repos = discover_git_repos(&roots, &excluded_dir_names);

    let mut changed = 0usize;
    for repo in repos {
        match tokio::time::timeout(
            Duration::from_secs(policy.repo_sync_timeout_secs),
            sync_repo(&repo, &policy, &excluded_dir_names, 0),
        )
        .await
        {
            Err(_) => {
                eprintln!(
                    "⚠️ repo sync timeout for {} after {}s",
                    repo.display(),
                    policy.repo_sync_timeout_secs
                );
            }
            Ok(Ok(true)) => {
                changed += 1;
                println!("🔁 synced {}", repo.display());
            }
            Ok(Ok(false)) => {}
            Ok(Err(e)) => {
                eprintln!("⚠️ sync failed for {}: {}", repo.display(), e);
            }
        }
    }

    println!("✅ sync pass complete (repos changed: {})", changed);
    if policy.auto_repair_concerns {
        if let Err(e) = run_repair_concerns(
            policy_path,
            true,
            None,
            Some(policy.push_op_timeout_secs),
            policy.push_retries,
            policy.auto_rewrite_large_blobs,
            ConcernRepairFilter::All,
            false,
        )
        .await
        {
            eprintln!("⚠️ auto-repair concerns failed: {}", e);
        }
    }
    if policy.auto_repair_warns {
        if let Err(e) = run_repair_warns(policy_path, true, None, false).await {
            eprintln!("⚠️ auto-repair warns failed: {}", e);
        }
    }
    Ok(())
}

pub(crate) async fn run_daemon(policy_path: PathBuf) -> Result<()> {
    #[derive(Debug, Clone)]
    struct RepoActivity {
        fingerprint: String,
        changed_at: Instant,
        failure_count: usize,
    }

    let mut activity: HashMap<PathBuf, RepoActivity> = HashMap::new();
    let mut repair_cooldowns: HashMap<PathBuf, Instant> = HashMap::new();

    loop {
        let policy = match SyncPolicy::load(&policy_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("⚠️ failed loading policy: {}", e);
                sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        let scan_interval = policy.pulse_interval_secs.max(1);
        let inactivity_delay = Duration::from_secs(policy.inactivity_push_delay_secs.max(1));
        let roots = policy.watch_root_paths();
        let excluded_dir_names = excluded_dir_names_set(&policy);
        let repos = discover_git_repos(&roots, &excluded_dir_names);
        let repo_set: BTreeSet<PathBuf> = repos.iter().cloned().collect();
        activity.retain(|repo, _| repo_set.contains(repo));
        repair_cooldowns.retain(|repo, _| repo_set.contains(repo));

        if let Some(reason) = freeze_reason(&policy_path) {
            println!("⏸️ sync daemon paused ({})", reason);
            sleep(Duration::from_secs(scan_interval)).await;
            continue;
        }

        for repo in repos {
            let now = Instant::now();
            if let Some(until) = repair_cooldowns.get(&repo).copied() {
                if now < until {
                    continue;
                }
                repair_cooldowns.remove(&repo);
            }
            let svc = match GitService::new(&repo) {
                Ok(svc) => svc,
                Err(e) => {
                    eprintln!("⚠️ {} init_failed: {}", repo.display(), e);
                    continue;
                }
            };
            let status = match svc.get_status().await {
                Ok(status) => status,
                Err(e) => {
                    eprintln!("⚠️ {} status_failed: {}", repo.display(), e);
                    continue;
                }
            };
            let entries = repo_diff_entries(&repo).await.unwrap_or_default();
            let effective_dirty = has_sync_relevant_dirty_entries(
                &repo,
                &entries,
                &excluded_dir_names,
                &policy.exclude_file_patterns,
                policy.max_stage_file_bytes,
            );
            let has_local_or_pending_work =
                effective_dirty || status.ahead > 0 || status.behind > 0 || !has_origin_remote(&repo);
            if !has_local_or_pending_work {
                activity.remove(&repo);
                continue;
            }

            let fingerprint = format!(
                "{}:{}:{}:{}:{}",
                status.branch,
                effective_dirty as u8,
                status.staged_files,
                status.ahead,
                status.behind
            );
            let Some(entry) = activity.get_mut(&repo) else {
                activity.insert(
                    repo.clone(),
                    RepoActivity {
                        fingerprint,
                        changed_at: now,
                        failure_count: 0,
                    },
                );
                continue;
            };
            if entry.fingerprint != fingerprint {
                entry.fingerprint = fingerprint;
                entry.changed_at = now;
                entry.failure_count = 0;
                continue;
            }
            if now.duration_since(entry.changed_at) < inactivity_delay {
                continue;
            }
            
            const MAX_FAILURES: usize = 5;
            if entry.failure_count >= MAX_FAILURES {
                if entry.failure_count == MAX_FAILURES {
                    eprintln!(
                        "⚠️ {} exceeded max failures ({}), skipping until resolved",
                        repo.display(),
                        MAX_FAILURES
                    );
                    entry.failure_count += 1;
                }
                continue;
            }

            let sync_success = match tokio::time::timeout(
                Duration::from_secs(policy.repo_sync_timeout_secs),
                sync_repo(
                    &repo,
                    &policy,
                    &excluded_dir_names,
                    now.duration_since(entry.changed_at).as_secs(),
                ),
            )
            .await
            {
                Err(_) => {
                    eprintln!(
                        "⚠️ repo sync timeout for {} after {}s",
                        repo.display(),
                        policy.repo_sync_timeout_secs
                    );
                    false
                }
                Ok(Ok(true)) => {
                    println!("🔁 synced {}", repo.display());
                    true
                }
                Ok(Ok(false)) => true,
                Ok(Err(e)) => {
                    eprintln!("⚠️ sync failed for {}: {}", repo.display(), e);
                    false
                }
            };

            let mut should_cooldown = false;
            if policy.auto_repair_concerns {
                match run_repair_concerns(
                    &policy_path,
                    true,
                    Some(repo.clone()),
                    Some(policy.push_op_timeout_secs),
                    policy.push_retries,
                    policy.auto_rewrite_large_blobs,
                    ConcernRepairFilter::All,
                    false,
                )
                .await
                {
                    Ok(summary) => {
                        if summary.found > 0 && summary.resolved_now == 0 && summary.succeeded == 0 {
                            should_cooldown = true;
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠️ auto-repair concerns failed for {}: {}", repo.display(), e);
                        should_cooldown = true;
                    }
                }
            }
            if policy.auto_repair_warns {
                match run_repair_warns(&policy_path, true, Some(repo.clone()), false).await {
                    Ok(summary) => {
                        if summary.found > 0 && summary.attempted > 0 && summary.succeeded == 0 {
                            should_cooldown = true;
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠️ auto-repair warns failed for {}: {}", repo.display(), e);
                        should_cooldown = true;
                    }
                }
            }
            if should_cooldown {
                repair_cooldowns.insert(
                    repo.clone(),
                    Instant::now() + Duration::from_secs(policy.repair_cooldown_secs.max(1)),
                );
            }

            if sync_success {
                entry.failure_count = 0;
                activity.remove(&repo);
            } else {
                entry.failure_count += 1;
            }
        }

        sleep(Duration::from_secs(scan_interval)).await;
    }
}
