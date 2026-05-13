use anyhow::Result;
use dracon_git::GitService;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::signal::unix::SignalKind;
use tokio::time::sleep;

pub(crate) static VERBOSITY: AtomicU8 = AtomicU8::new(0);

#[macro_export]
macro_rules! veprintln {
    ($lvl:expr, $($arg:tt)*) => {
        if $lvl <= VERBOSITY.load(Ordering::SeqCst) {
            eprintln!($($arg)*);
        }
    };
}

use crate::policy::{SyncPolicy, freeze_reason, debug_enabled, timestamp_secs};
use crate::exclude::{excluded_dir_names_set, has_sync_relevant_dirty_entries};
use crate::git::{discover_git_repos, repo_diff_entries, has_origin_remote, has_tracking_upstream, has_both_main_and_master, git_diff_head_files};
use crate::report::{ConcernRepairFilter, run_repair_concerns, run_repair_warns};
use crate::sync::sync_repo;

const STUCK_REPO_EXPIRY_SECS: u64 = 24 * 60 * 60; // 24 hours

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StuckRepoEntry {
    path: PathBuf,
    stuck_since: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stuck_repo_entry_serialization() {
        let entry = StuckRepoEntry {
            path: PathBuf::from("/test/repo"),
            stuck_since: 1000,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"/test/repo\""));
        assert!(json.contains("1000"));
    }

    #[test]
    fn test_stuck_repo_entry_deserialization() {
        let json = r#"{"path":"/test/repo","stuck_since":1000}"#;
        let entry: StuckRepoEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.path, PathBuf::from("/test/repo"));
        assert_eq!(entry.stuck_since, 1000);
    }

    #[test]
    fn test_stuck_repo_expiry_constant() {
        assert_eq!(STUCK_REPO_EXPIRY_SECS, 24 * 60 * 60);
    }

    #[test]
    fn test_stuck_repo_expiry_one_day() {
        assert_eq!(STUCK_REPO_EXPIRY_SECS, 86400);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_stuck_repo_expiry_not_zero() {
        assert!(STUCK_REPO_EXPIRY_SECS > 0);
    }

    #[test]
    fn test_stuck_repo_entry_debug() {
        let entry = StuckRepoEntry {
            path: PathBuf::from("/test/repo"),
            stuck_since: 1000,
        };
        let debug = format!("{:?}", entry);
        assert!(debug.contains("/test/repo"));
        assert!(debug.contains("1000"));
    }

    #[test]
    fn test_stuck_repo_entry_clone() {
        let entry = StuckRepoEntry {
            path: PathBuf::from("/test/repo"),
            stuck_since: 1000,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.path, entry.path);
        assert_eq!(cloned.stuck_since, entry.stuck_since);
    }

    #[test]
    fn test_stuck_repo_entry_equality() {
        let entry1 = StuckRepoEntry {
            path: PathBuf::from("/test/repo"),
            stuck_since: 1000,
        };
        let entry2 = StuckRepoEntry {
            path: PathBuf::from("/test/repo"),
            stuck_since: 1000,
        };
        let entry3 = StuckRepoEntry {
            path: PathBuf::from("/other/repo"),
            stuck_since: 1000,
        };
        assert_eq!(entry1.path, entry2.path);
        assert_ne!(entry1.path, entry3.path);
    }

    #[test]
    fn test_stuck_repo_entry_path_stored_correctly() {
        let path = PathBuf::from("/home/user/code/my-project");
        let entry = StuckRepoEntry {
            path: path.clone(),
            stuck_since: 12345,
        };
        assert_eq!(entry.path, path);
        assert_eq!(entry.path.to_string_lossy(), "/home/user/code/my-project");
    }

    #[test]
    fn test_stuck_repo_entry_timestamp_ordering() {
        let old = StuckRepoEntry {
            path: PathBuf::from("/old"),
            stuck_since: 1000,
        };
        let new = StuckRepoEntry {
            path: PathBuf::from("/new"),
            stuck_since: 2000,
        };
        assert!(old.stuck_since < new.stuck_since);
    }
}

#[cfg(test)]
mod daemon_tests {
    use super::*;

    #[test]
    fn test_stuck_repos_path_format() {
        let path = stuck_repos_path();
        assert!(path.to_string_lossy().contains(".local"));
        assert!(path.to_string_lossy().contains("dracon-sync-stuck-push-repos.json"));
    }

    #[test]
    fn test_load_stuck_push_repos_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let _guard = crate::test_helpers::EnvRestorer::new("DRACON_SYNC_STATE_DIR", temp_dir.path().to_string_lossy().as_ref());
        let repos = load_stuck_push_repos();
        assert!(repos.is_empty());
    }

    #[test]
    fn test_unstuck_repo_nonexistent() {
        let result = unstuck_repo(Path::new("/nonexistent/path"));
        assert!(!result);
    }

    #[test]
    fn test_list_stuck_repos_empty() {
        list_stuck_repos();
    }

    #[test]
    fn test_is_repo_stuck_false() {
        assert!(!is_repo_stuck(Path::new("/nonexistent/path")));
    }

    #[test]
    fn test_stuck_repos_path_home() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let expected_base = home.join(".local").join("state").join("dracon");
        let path = stuck_repos_path();
        assert!(path.starts_with(expected_base));
    }
}

fn stuck_repos_path() -> PathBuf {
    if let Ok(state_dir) = std::env::var("DRACON_SYNC_STATE_DIR") {
        if !state_dir.is_empty() {
            return PathBuf::from(state_dir).join("dracon-sync-stuck-push-repos.json");
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local")
        .join("state")
        .join("dracon")
        .join("dracon-sync-stuck-push-repos.json")
}

fn load_stuck_push_repos() -> HashMap<PathBuf, u64> {
    let path = stuck_repos_path();
    if !path.exists() {
        return HashMap::new();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("⚠️ failed reading stuck repos ({}): {}", path.display(), e);
            return HashMap::new();
        }
    };
    let entries: Vec<StuckRepoEntry> = serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("⚠️ failed parsing stuck repos ({}): {}", path.display(), e);
        Vec::new()
    });
    let now = timestamp_secs();
    let cutoff = now.saturating_sub(STUCK_REPO_EXPIRY_SECS);
    entries
        .into_iter()
        .filter(|e| e.stuck_since > cutoff)
        .map(|e| (e.path, e.stuck_since))
        .collect()
}

fn save_stuck_push_repos(repos: &HashMap<PathBuf, u64>) {
    let path = stuck_repos_path();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("⚠️ failed creating stuck repos dir: {}", e);
                return;
            }
        }
    }
    let entries: Vec<StuckRepoEntry> = repos
        .iter()
        .map(|(p, t)| StuckRepoEntry { path: p.clone(), stuck_since: *t })
        .collect();
    let content = serde_json::to_string_pretty(&entries).unwrap_or_else(|e| {
        eprintln!("⚠️ failed serializing stuck repos: {}", e);
        String::new()
    });
    if content.is_empty() {
        return;
    }
    let tmp_path = path.with_extension("tmp");
    if let Err(e) = std::fs::write(&tmp_path, &content) {
        eprintln!("⚠️ failed writing stuck repos tmp ({}): {}", tmp_path.display(), e);
        let _ = std::fs::remove_file(&tmp_path);
        return;
    }
    if let Err(e) = std::fs::rename(&tmp_path, &path) {
        eprintln!("⚠️ failed renaming stuck repos ({}): {}", path.display(), e);
        let _ = std::fs::remove_file(&tmp_path);
    }
}

pub(crate) fn unstuck_repo(repo: &Path) -> bool {
    let path = stuck_repos_path();
    if !path.exists() {
        return false;
    }
    let mut repos = load_stuck_push_repos();
    if repos.remove(repo).is_some() {
        save_stuck_push_repos(&repos);
        println!("🔓 unstuck: {}", repo.display());
        true
    } else {
        println!("ℹ️ {} not in stuck repos", repo.display());
        false
    }
}

pub(crate) fn list_stuck_repos() {
    let repos = load_stuck_push_repos();
    if repos.is_empty() {
        println!("✅ no stuck repos");
        return;
    }
    println!("🔒 stuck repos (expire after 24h):");
    let now = timestamp_secs();
    for (path, since) in repos {
        let age_hrs = (now.saturating_sub(since)) / 3600;
        println!("   {} ({}h ago)", path.display(), age_hrs);
    }
}

pub(crate) fn is_repo_stuck(repo: &Path) -> bool {
    load_stuck_push_repos().contains_key(repo)
}

pub(crate) async fn run_once(policy_path: &Path) -> Result<()> {
    if let Some(reason) = freeze_reason(policy_path) {
        println!("⏸️ sync frozen ({})", reason);
        return Ok(());
    }

    let policy = SyncPolicy::load(policy_path)?;
    let roots = policy.watch_root_paths();
    let excluded_dir_names = excluded_dir_names_set(&policy);
    let repos = discover_git_repos(&roots, &excluded_dir_names, &policy.exclude_repos, Some(&policy.system_repo));

    let mut changed = 0usize;
    for repo in repos {
        match tokio::time::timeout(
            Duration::from_secs(policy.repo_sync_timeout_secs),
            sync_repo(&repo, &policy, &excluded_dir_names, 0, None, false, Some(policy_path), false),
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

pub(crate) async fn run_daemon(policy_path: PathBuf, override_interval_secs: Option<u64>) -> Result<()> {
    #[derive(Debug, Clone)]
    struct RepoActivity {
        fingerprint: String,
        changed_at: Instant,
        failure_count: usize,
        remote_failures: HashMap<String, usize>,
    }

    let mut activity: HashMap<PathBuf, RepoActivity> = HashMap::new();
    let mut repair_cooldowns: HashMap<PathBuf, Instant> = HashMap::new();
    let mut filter_cooldowns: HashMap<PathBuf, Instant> = HashMap::new();
    let mut stuck_push_repos = load_stuck_push_repos();
    let mut remote_notify_cooldowns: HashMap<String, Instant> = HashMap::new();

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_sigterm = shutdown.clone();
    let shutdown_sigint = shutdown.clone();
    let reload = Arc::new(AtomicBool::new(false));
    let reload_sighup = reload.clone();

    tokio::spawn(async move {
        if let Ok(mut sig) = tokio::signal::unix::signal(SignalKind::terminate()) {
            sig.recv().await;
            veprintln!(1, "sync: received SIGTERM, shutting down gracefully...");
            shutdown_sigterm.store(true, Ordering::SeqCst);
        } else {
            eprintln!("sync: failed to set up SIGTERM handler");
        }
    });

    tokio::spawn(async move {
        if let Ok(mut sig) = tokio::signal::unix::signal(SignalKind::interrupt()) {
            sig.recv().await;
            veprintln!(1, "sync: received SIGINT, shutting down gracefully...");
            shutdown_sigint.store(true, Ordering::SeqCst);
        } else {
            eprintln!("sync: failed to set up SIGINT handler");
        }
    });

    tokio::spawn(async move {
        if let Ok(mut sig) = tokio::signal::unix::signal(SignalKind::hangup()) {
            while sig.recv().await.is_some() {
                veprintln!(1, "sync: received SIGHUP, will reload policy...");
                reload_sighup.store(true, Ordering::SeqCst);
            }
        } else {
            eprintln!("sync: failed to set up SIGHUP handler");
        }
    });

    while !shutdown.load(Ordering::SeqCst) {
        if reload.load(Ordering::SeqCst) {
            reload.store(false, Ordering::SeqCst);
            match SyncPolicy::load(&policy_path) {
                Ok(p) => {
                    veprintln!(2, "sync: policy reloaded on SIGHUP (watch_root={} repos, excluded={})",
                        p.watch_root_paths().len(), p.exclude_repos.len());
                    activity.clear();
                    repair_cooldowns.clear();
                    filter_cooldowns.clear();
                }
                Err(e) => eprintln!("sync: SIGHUP policy reload failed: {}", e),
            }
        }
        let policy = match SyncPolicy::load(&policy_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("⚠️ failed loading policy: {}", e);
                sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        let scan_interval = override_interval_secs.unwrap_or(policy.pulse_interval_secs).max(1);
        let inactivity_delay = Duration::from_secs(policy.inactivity_push_delay_secs.max(1));
        let roots = policy.watch_root_paths();
        let excluded_dir_names = excluded_dir_names_set(&policy);
        let repos = discover_git_repos(&roots, &excluded_dir_names, &policy.exclude_repos, Some(&policy.system_repo));
        let repo_set: BTreeSet<PathBuf> = repos.iter().cloned().collect();
        activity.retain(|repo, _| repo_set.contains(repo));
        repair_cooldowns.retain(|repo, _| repo_set.contains(repo));
        filter_cooldowns.retain(|repo, _| repo_set.contains(repo));
        stuck_push_repos.retain(|repo, _| repo_set.contains(repo));

        if let Some(reason) = freeze_reason(&policy_path) {
            println!("⏸️ sync daemon paused ({})", reason);
            sleep(Duration::from_secs(scan_interval)).await;
            continue;
        }

        for repo in repos {
            let now = Instant::now();
            // Skip repos that are stuck on push, but retry them every 5 minutes
            // to see if the issue resolved (e.g., remote was recreated, permissions fixed, etc.)
            if let Some(stuck_since) = stuck_push_repos.get(&repo).copied() {
                let stuck_age_secs = timestamp_secs().saturating_sub(stuck_since);
                if stuck_age_secs < 300 {
                    // Less than 5 minutes since stuck was recorded - skip
                    continue;
                }
                // 5+ minutes since stuck was recorded - retry once
                eprintln!("🔄 {} was stuck, retrying push after {}s", repo.display(), stuck_age_secs);
                stuck_push_repos.remove(&repo);
                save_stuck_push_repos(&stuck_push_repos);
            }
            if has_both_main_and_master(&repo) {
                eprintln!("🔧 {} has both main+master, consolidating to main", repo.display());
                if let Err(e) = crate::git::consolidate_to_main(&repo).await {
                    eprintln!("⚠️ failed to consolidate {} to main: {}", repo.display(), e);
                    continue;
                }
            } else if crate::git::has_only_master_branch(&repo) {
                eprintln!("🔧 {} has only 'master', renaming to 'main'", repo.display());
                if let Err(e) = crate::git::rename_master_to_main(&repo).await {
                    eprintln!("⚠️ failed to rename {} master→main: {}", repo.display(), e);
                    continue;
                }
            }
            if let Some(until) = repair_cooldowns.get(&repo).copied() {
                if now < until {
                    continue;
                }
                repair_cooldowns.remove(&repo);
            }
            if let Some(until) = filter_cooldowns.get(&repo).copied() {
                if now < until {
                    continue;
                }
                filter_cooldowns.remove(&repo);
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

            // Cache remote checks — used in both fast and slow paths
            let has_origin = has_origin_remote(&repo);
            let has_upstream = has_tracking_upstream(&repo);

            // Fast path: skip expensive git diff calls for clean, synced repos.
            // Only do detailed diff analysis when the repo actually has changes.
            let (effective_dirty, _entries) =
            if status.is_clean && status.ahead == 0 && status.behind == 0 {
                // Clean and synced — skip all expensive git calls
                let has_remote_issues = !has_origin || !has_upstream;
                if !has_remote_issues {
                    activity.remove(&repo);
                    continue;
                }
                // Remote issues but clean — check for dirty files that
                // has_sync_relevant_dirty_entries would detect (untracked in excluded
                // dirs, oversized files, etc.) before committing to dirty state.
                let entries = repo_diff_entries(&repo).await.unwrap_or_default();
                let dirty = has_sync_relevant_dirty_entries(
                    &repo,
                    &entries,
                    &excluded_dir_names,
                    &policy.exclude_file_patterns,
                    policy.max_stage_file_bytes,
                );
                if !dirty {
                    activity.remove(&repo);
                    continue;
                }
                (dirty, entries)
            } else {
                let raw_entries = repo_diff_entries(&repo).await.unwrap_or_default();
                // Filter out entries that only differ due to clean/smudge filters.
                // `git status` shows filter-processed files as modified, but `git diff HEAD`
                // correctly applies the clean filter and shows no diff for such files.
                // Note: untracked files don't appear in `git diff HEAD`, so they always pass.
                let diff_head_files = git_diff_head_files(&repo).await.unwrap_or_default();
                let filtered: Vec<_> = if diff_head_files.is_empty() && !raw_entries.is_empty() {
                    // git diff HEAD returned nothing. Only clear if ALL entries are Modified
                    // (filter-only). Untracked/Added files don't appear in git diff HEAD.
                    let has_non_modified = raw_entries.iter().any(|e| {
                        !matches!(e.status, dracon_git::types::FileStatus::Modified)
                    });
                    if has_non_modified {
                        raw_entries.into_iter()
                            .filter(|e| !matches!(e.status, dracon_git::types::FileStatus::Modified))
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    raw_entries.into_iter()
                        .filter(|e| {
                            // Always keep non-modified entries (added, deleted, etc.)
                            // For modified entries, only keep if git diff HEAD shows them
                            if !matches!(e.status, dracon_git::types::FileStatus::Modified) {
                                return true;
                            }
                            diff_head_files.contains(&e.path)
                        })
                        .collect()
                };
                let dirty = has_sync_relevant_dirty_entries(
                    &repo,
                    &filtered,
                    &excluded_dir_names,
                    &policy.exclude_file_patterns,
                    policy.max_stage_file_bytes,
                );
                let has_local_or_pending_work =
                    dirty || status.ahead > 0 || status.behind > 0
                    || !has_origin || !has_upstream;
                if !has_local_or_pending_work {
                    activity.remove(&repo);
                    continue;
                }
                (dirty, filtered)
            };

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
                        remote_failures: HashMap::new(),
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
            
            // MAX_FAILURES: per-cycle retry cap for transient errors.
            // Stuck repos (line ~505) trigger at failure_count >= 3 when repo is
            // clean + ahead > 0 — that's a permanent condition. MAX_FAILURES is
            // a higher bar for repos that might still be recoverable (dirty,
            // network issues, etc.).
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
                    Some(&mut entry.remote_failures),
                    false,
                    Some(&policy_path),
                    false,
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
                Ok(Ok(crate::sync::SyncOutcome::Synced)) => {
                    println!("🔁 synced {}", repo.display());
                    true
                }
                Ok(Ok(crate::sync::SyncOutcome::NothingToDo)) => {
                    if debug_enabled() {
                        eprintln!("🐛 {} nothing to commit", repo.display());
                    }
                    true
                }
                Ok(Ok(crate::sync::SyncOutcome::Blocked)) => {
                    if debug_enabled() {
                        eprintln!("🐛 {} blocked (guard or manual intervention)", repo.display());
                    }
                    false
                }
                Ok(Err(e)) => {
                    eprintln!("⚠️ sync failed for {}: {}", repo.display(), e);
                    false
                }
            };

            let mut should_cooldown = false;
            if policy.auto_repair_concerns && sync_success {
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
                entry.remote_failures.clear();
                // Re-check if repo is still dirty (filter-only changes persist).
                // If so, use a long cooldown instead of removing from activity
                // to prevent tight triage loops on phantom changes.
                let entries_after = repo_diff_entries(&repo).await.unwrap_or_default();
                let still_dirty = has_sync_relevant_dirty_entries(
                    &repo,
                    &entries_after,
                    &excluded_dir_names,
                    &policy.exclude_file_patterns,
                    policy.max_stage_file_bytes,
                );
                if still_dirty {
                    let cooldown_secs = policy.inactivity_push_delay_secs.max(5);
                    filter_cooldowns.insert(
                        repo.clone(),
                        Instant::now() + Duration::from_secs(cooldown_secs),
                    );
                    if debug_enabled() {
                        eprintln!("🐛 {} filter-only dirty, cooldown {}s", repo.display(), cooldown_secs);
                    }
                }
                activity.remove(&repo);
            } else {
                entry.failure_count += 1;

                // Check if ALL configured remotes are failing — desktop notification
                if !entry.remote_failures.is_empty() {
                    let all_failed = policy.remotes.iter()
                        .all(|r| entry.remote_failures.get(&r.name).copied().unwrap_or(0) > 0);
                    if all_failed {
                        let notify_key = format!("{}-all", repo.display());
                        let now = Instant::now();

                        // Check cooldown BEFORE firing notification
                        if let Some(cooldown_until) = remote_notify_cooldowns.get(&notify_key) {
                            if now < *cooldown_until {
                                // still in cooldown, skip notification entirely
                            } else {
                                // cooldown expired, fire and reset
                                remote_notify_cooldowns.remove(&notify_key);
                            }
                        }

                        // Fire only if not in cooldown (cooldown entry was removed above)
                        if let std::collections::hash_map::Entry::Vacant(e) = remote_notify_cooldowns.entry(notify_key) {
                            let failed_list: Vec<_> = entry.remote_failures.keys().cloned().collect();
                            let msg = format!("All remotes failing: {}. Failures: {:?}", failed_list.join(", "), entry.remote_failures);
                            crate::report::send_sync_conflict_notification(
                                &repo,
                                "All Remotes Failing",
                                &msg,
                            );
                            e.insert(now + Duration::from_secs(1800));
                        }
                    }
                }

                // If repo has divergence (ahead AND behind), push will always fail
                // regardless of dirty state - mark as stuck immediately.
                // This prevents the repo from blocking other syncs.
                let is_diverged = status.ahead > 0 && status.behind > 0;
                // If repo is clean but has ahead commits and push keeps failing,
                // it's permanently stuck (permission error, deleted remote, etc).
                // Skip it entirely to unblock other repos.
                if is_diverged || (!effective_dirty && status.ahead > 0 && entry.failure_count >= 3) {
                    let reason = if is_diverged {
                        format!("(diverged: ahead={}, behind={})", status.ahead, status.behind)
                    } else {
                        format!("(ahead={}, clean)", status.ahead)
                    };
                    eprintln!("🔒 {} permanently stuck on push {} skipping", repo.display(), reason);
                    stuck_push_repos.insert(repo.clone(), timestamp_secs());
                    save_stuck_push_repos(&stuck_push_repos);
                    activity.remove(&repo);
                }
            }
        }

        sleep(Duration::from_secs(scan_interval)).await;
    }
    Ok(())
}
