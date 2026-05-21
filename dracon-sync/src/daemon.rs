use anyhow::Result;
use dracon_git::GitService;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::signal::unix::SignalKind;
use tokio::time::sleep;

pub(crate) static VERBOSITY: AtomicU8 = AtomicU8::new(0);

/// Conditional eprintln based on verbosity level.
#[macro_export]
macro_rules! veprintln {
    ($lvl:expr, $($arg:tt)*) => {
        if $lvl <= VERBOSITY.load(Ordering::SeqCst) {
            eprintln!($($arg)*);
            use std::io::Write;
            let _ = std::io::stderr().flush();
        }
    };
}

use crate::exclude::{excluded_dir_names_set, has_sync_relevant_dirty_entries};
use crate::git::{
    discover_git_repos, git_diff_head_files, has_both_main_and_master, has_origin_remote,
    has_tracking_upstream, is_repo_ready, repair_broken_tracking, repo_diff_entries,
};
use crate::policy::{debug_enabled, freeze_reason, timestamp_secs, SyncPolicy};
use crate::report::{run_repair_concerns, run_repair_warns, ConcernRepairFilter};
use crate::sync::{sync_repo, SyncOutcome};

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
        assert!(path
            .to_string_lossy()
            .contains("dracon-sync-stuck-push-repos.json"));
    }

    #[test]
    fn test_load_stuck_push_repos_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let _guard = crate::test_helpers::EnvRestorer::new(
            "DRACON_SYNC_STATE_DIR",
            temp_dir.path().to_string_lossy().as_ref(),
        );
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

    #[test]
    fn test_skips_nonexistent_repo() {
        // If a repo is deleted between discovery and processing, the daemon
        // should skip it gracefully rather than panicking or erroring.
        use crate::git::discover_git_repos;
        use crate::policy::SyncPolicy;

        let policy = SyncPolicy::default();
        let excluded = crate::exclude::excluded_dir_names_set(&policy);

        // Nonexistent repo should not be discovered
        let repos = discover_git_repos(&[PathBuf::from("/nonexistent/path")], &excluded, &[], None);
        assert!(repos.is_empty(), "should not discover nonexistent paths");
    }

    #[test]
    fn test_is_repo_ready_nonexistent_path() {
        // is_repo_ready should return false for a repo path that doesn't exist
        assert!(!is_repo_ready(Path::new("/nonexistent/repo")));
    }

    #[test]
    fn test_policy_clone_at_repo_iteration() {
        // Verifies that a cloned SyncPolicy is an independent snapshot:
        // each repo iteration should clone the policy to avoid race conditions
        // from mid-cycle policy reloads (e.g., SIGHUP).
        use crate::policy::SyncPolicy;

        let policy = SyncPolicy::default();
        let cloned = policy.clone();

        // Debug format should match — same field values
        assert_eq!(format!("{:?}", policy), format!("{:?}", cloned));

        // Verify key fields are carried over
        assert_eq!(policy.auto_commit, cloned.auto_commit);
        assert_eq!(policy.auto_pull, cloned.auto_pull);
        assert_eq!(policy.auto_push, cloned.auto_push);
        assert_eq!(policy.pulse_interval_secs, cloned.pulse_interval_secs);
        assert_eq!(policy.push_retries, cloned.push_retries);
        assert_eq!(policy.max_stage_file_bytes, cloned.max_stage_file_bytes);
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
        .map(|(p, t)| StuckRepoEntry {
            path: p.clone(),
            stuck_since: *t,
        })
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
        eprintln!(
            "⚠️ failed writing stuck repos tmp ({}): {}",
            tmp_path.display(),
            e
        );
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
        eprintln!("🔓 unstuck: {}", repo.display());
        true
    } else {
        eprintln!("ℹ️ {} not in stuck repos", repo.display());
        false
    }
}

pub(crate) fn list_stuck_repos() {
    let repos = load_stuck_push_repos();
    if repos.is_empty() {
        eprintln!("✅ no stuck repos");
        return;
    }
    eprintln!("🔒 stuck repos (expire after 24h):");
    let now = timestamp_secs();
    for (path, since) in repos {
        let age_hrs = (now.saturating_sub(since)) / 3600;
        eprintln!("   {} ({}h ago)", path.display(), age_hrs);
    }
}

pub(crate) fn is_repo_stuck(repo: &Path) -> bool {
    load_stuck_push_repos().contains_key(repo)
}

/// Run startup cleanup: prune stale state from previous runs.
/// Called by both `run_once` (for one-shot sync) and `run_daemon` (on startup).
/// Returns the number of stale index.lock files removed.
pub(crate) async fn run_startup_cleanup(policy_path: &Path) -> (BTreeSet<PathBuf>, u64) {
    eprintln!("🧹 startup: running cleanup...");
    let policy = match SyncPolicy::load(policy_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("⚠️ failed loading policy for startup cleanup: {}", e);
            SyncPolicy::default()
        }
    };
    let roots = policy.watch_root_paths();
    let excluded_dir_names = excluded_dir_names_set(&policy);
    let discovered = discover_git_repos(
        &roots,
        &excluded_dir_names,
        &policy.exclude_repos,
        Some(&policy.system_repo),
    );
    let repo_set: BTreeSet<PathBuf> = discovered.iter().cloned().collect();

    // Prune stuck repos no longer on disk
    let mut stuck_push_repos = load_stuck_push_repos();
    let before = stuck_push_repos.len();
    stuck_push_repos.retain(|repo, _| repo_set.contains(repo));
    if stuck_push_repos.len() != before {
        save_stuck_push_repos(&stuck_push_repos);
        eprintln!(
            "🧹 startup: pruned {} stale stuck repos",
            before - stuck_push_repos.len()
        );
    }

    // Enforce incident ledger retention now
    if let Err(e) = crate::report::enforce_retention_at_startup(policy_path, &policy) {
        eprintln!("⚠️ startup: incident ledger cleanup failed: {}", e);
    }

    // Prune visibility cache for deleted repos
    if let Err(e) = crate::visibility::prune_stale_visibility_cache(&repo_set) {
        eprintln!("⚠️ startup: visibility cache cleanup failed: {}", e);
    }

    // Repair broken upstream tracking references (e.g. origin/master: gone)
    let discovered_refs: Vec<PathBuf> = repo_set.iter().cloned().collect();
    let fixed = repair_broken_tracking(&discovered_refs);
    if fixed > 0 {
        eprintln!(
            "🧹 startup: repaired {} broken upstream tracking refs",
            fixed
        );
    }

    // Remove stale .git/index.lock files from crashed git processes.
    // A lock file with no holding process prevents all git operations.
    let mut locks_removed = 0u64;
    for repo in &repo_set {
        let lock = repo.join(".git/index.lock");
        if lock.exists() {
            eprintln!("🧹 startup: found index.lock in {} (checking fuser...)", repo.display());
            let in_use = std::process::Command::new("fuser")
                .arg(&lock)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !in_use {
                if let Err(e) = std::fs::remove_file(&lock) {
                    eprintln!("⚠️ startup: failed to remove {}: {}", lock.display(), e);
                } else {
                    locks_removed += 1;
                }
            }
        }
    }
    if locks_removed > 0 {
        eprintln!(
            "🧹 startup: removed {} stale .git/index.lock files",
            locks_removed
        );
    }

    (repo_set, locks_removed)
}

pub(crate) async fn run_once(policy_path: &Path) -> Result<()> {
    if let Some(reason) = freeze_reason(policy_path) {
        eprintln!("⏸️ sync frozen ({})", reason);
        return Ok(());
    }

    // Clean up stale state from previous runs (including index.lock files)
    let (repo_set, _) = run_startup_cleanup(policy_path).await;

    let policy = SyncPolicy::load(policy_path)?;
    let excluded_dir_names = excluded_dir_names_set(&policy);

    let mut changed = 0usize;
    for repo in &repo_set {
        // Guard against repo-discovery race
        if !repo.exists() {
            eprintln!(
                "⚠️ {} repo path vanished between discovery and sync, skipping",
                repo.display()
            );
            continue;
        }
        match tokio::time::timeout(
            Duration::from_secs(policy.repo_sync_timeout_secs),
            sync_repo(
                &repo,
                &policy,
                &excluded_dir_names,
                0,
                None,
                false,
                Some(policy_path),
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
            }
            Ok(Ok(SyncOutcome::Synced)) => {
                changed += 1;
                println!("🔁 synced {}", repo.display());
            }
            Ok(Ok(SyncOutcome::NothingToDo)) | Ok(Ok(SyncOutcome::Blocked)) => {}
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

pub(crate) async fn run_daemon(
    policy_path: PathBuf,
    override_interval_secs: Option<u64>,
) -> Result<()> {
    // Note: Rust's stdio buffers are separate from C's FILE* buffers.
    // When running under systemd (socket-based journal capture), Rust defaults
    // to block buffering. We can't use setvbuf on Rust's handles, so instead
    // we flush stderr at strategic points in the daemon loop (see flush calls below).
    eprintln!("🔄 dracon-sync daemon started");
    #[derive(Debug, Clone)]
    struct RepoActivity {
        fingerprint: String,
        changed_at: Instant,
        /// When the repo first became dirty in this cycle.
        /// Unlike changed_at, this doesn't reset on fingerprint changes.
        dirty_since: Option<Instant>,
        /// When the repo first became ahead of origin (unpushed commits).
        ahead_since: Option<Instant>,
        /// When the repo first became behind origin (unpulled commits).
        behind_since: Option<Instant>,
        /// Which mirrors have failed consecutively (name → consecutive fail count).
        mirror_consecutive_fails: HashMap<String, usize>,
        failure_count: usize,
        remote_failures: HashMap<String, usize>,
    }

    let mut activity: HashMap<PathBuf, RepoActivity> = HashMap::new();
    let mut pending_repos: HashMap<PathBuf, Instant> = HashMap::new();
    let mut initial_repos: HashSet<PathBuf>; // populated after first scan
    let mut repair_cooldowns: HashMap<PathBuf, Instant> = HashMap::new();
    let mut filter_cooldowns: HashMap<PathBuf, Instant> = HashMap::new();
    let mut stuck_push_repos = load_stuck_push_repos();
    let mut remote_notify_cooldowns: HashMap<String, Instant> = HashMap::new();
    let mut cycle_count: u64 = 0;

    // ── Startup cleanup: prune stale state from previous runs ──
    let (repo_set, _) = run_startup_cleanup(&policy_path).await;
    initial_repos = repo_set.iter().cloned().collect();
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

    while !shutdown.load(Ordering::SeqCst) {        if reload.load(Ordering::SeqCst) {
            reload.store(false, Ordering::SeqCst);
            match SyncPolicy::load(&policy_path) {
                Ok(p) => {
                    veprintln!(
                        2,
                        "sync: policy reloaded on SIGHUP (watch_root={} repos, excluded={})",
                        p.watch_root_paths().len(),
                        p.exclude_repos.len()
                    );
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
        let scan_interval = override_interval_secs
            .unwrap_or(policy.pulse_interval_secs)
            .max(1);
        let inactivity_delay = Duration::from_secs(policy.inactivity_push_delay_secs.max(1));
        let roots = policy.watch_root_paths();
        let excluded_dir_names = excluded_dir_names_set(&policy);
        let repos = discover_git_repos(
            &roots,
            &excluded_dir_names,
            &policy.exclude_repos,
            Some(&policy.system_repo),
        );
        let repo_set: BTreeSet<PathBuf> = repos.iter().cloned().collect();
        activity.retain(|repo, _| {
            let keep = repo_set.contains(repo);
            if !keep {
                initial_repos.remove(repo);
            }
            keep
        });
        pending_repos.retain(|repo, _| repo_set.contains(repo));
        repair_cooldowns.retain(|repo, _| repo_set.contains(repo));
        filter_cooldowns.retain(|repo, _| repo_set.contains(repo));
        stuck_push_repos.retain(|repo, _| repo_set.contains(repo));

        // Periodic broken tracking repair (every ~5 min at 1s interval)
        cycle_count += 1;
        if cycle_count.is_multiple_of(300) {
            let repo_refs: Vec<PathBuf> = repo_set.iter().cloned().collect();
            repair_broken_tracking(&repo_refs);
        }

        if let Some(reason) = freeze_reason(&policy_path) {
            eprintln!("⏸️ sync daemon paused ({})", reason);
            sleep(Duration::from_secs(scan_interval)).await;
            continue;
        }

        for repo in repos {
            // Clone policy at each repo iteration for a consistent snapshot.
            // If the policy is reloaded mid-cycle (SIGHUP), this repo still
            // operates on the policy version it was started with.
            let policy = policy.clone();

            // Guard against repo-discovery race: if a repo was deleted between
            // discovery and processing, skip it and clean up tracking.
            if !repo.exists() {
                if debug_enabled() {
                    eprintln!("⏳ {} repo path vanished, skipping", repo.display());
                }
                activity.remove(&repo);
                initial_repos.remove(&repo);
                continue;
            }

            let now = Instant::now();
            if !is_repo_ready(&repo) {
                if debug_enabled() {
                    eprintln!(
                        "⏳ {} not ready (mid-clone or empty repo), skipping",
                        repo.display()
                    );
                }
                continue;
            }
            // Skip repos mid-checkout (clone's checkout phase holds index.lock).
            // Without this guard, the daemon can interfere with git checkout by
            // creating files (standard_files, project-state.md, etc.) that later
            // cause "Untracked working tree file would be overwritten by merge"
            // errors when git's own checkout tries to write them.
            let lock = repo.join(".git").join("index.lock");
            if lock.exists() {
                if debug_enabled() {
                    eprintln!(
                        "⏳ {} has index.lock (mid-checkout), skipping",
                        repo.display()
                    );
                }
                continue;
            }
            // Grace period for newly discovered repos: skip git operations
            // for the first 15s to avoid interfering with in-progress clones.
            // During git clone, HEAD resolves after fetch but checkout may
            // still be in progress — running git status or writing standard
            // files here can create working-tree files that conflict with
            // git's own checkout, causing "Untracked working tree file would
            // be overwritten by merge" errors.
            //
            // Only applies to repos discovered AFTER the first scan cycle.
            // Repos present at daemon startup are assumed to be stable
            // (already checked out) and are processed immediately.
            if !initial_repos.contains(&repo) && cycle_count > 0 {
                const PENDING_GRACE_SECS: Duration = Duration::from_secs(15);
                if let Some(&entry_time) = pending_repos.get(&repo) {
                    if Instant::now().duration_since(entry_time) < PENDING_GRACE_SECS {
                        continue;
                    }
                    pending_repos.remove(&repo);
                } else {
                    // First time seeing this repo after startup: enter grace period
                    pending_repos.insert(repo.clone(), Instant::now());
                    if debug_enabled() {
                        eprintln!(
                            "⏳ {} new repo, entering 15s grace period",
                            repo.display()
                        );
                    }
                    continue;
                }
            }

            // Skip repos that are stuck on push, but retry them every 5 minutes
            // to see if the issue resolved (e.g., remote was recreated, permissions fixed, etc.)
            if let Some(stuck_since) = stuck_push_repos.get(&repo).copied() {
                let stuck_age_secs = timestamp_secs().saturating_sub(stuck_since);
                if stuck_age_secs < 300 {
                    // Less than 5 minutes since stuck was recorded - skip
                    continue;
                }
                // 5+ minutes since stuck was recorded - retry once
                eprintln!(
                    "🔄 {} was stuck, retrying push after {}s",
                    repo.display(),
                    stuck_age_secs
                );
                stuck_push_repos.remove(&repo);
                save_stuck_push_repos(&stuck_push_repos);
            }
            if has_both_main_and_master(&repo) {
                eprintln!(
                    "🔧 {} has both main+master, consolidating to main",
                    repo.display()
                );
                if let Err(e) = crate::git::consolidate_to_main(&repo).await {
                    eprintln!("⚠️ failed to consolidate {} to main: {}", repo.display(), e);
                    continue;
                }
            } else if crate::git::has_only_master_branch(&repo) {
                eprintln!(
                    "🔧 {} has only 'master', renaming to 'main'",
                    repo.display()
                );
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
            let mut status = match svc.get_status().await {
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
            let (effective_dirty, _entries) = if status.is_clean
                && status.ahead == 0
                && status.behind == 0
            {
                // Clean and synced — skip all expensive git calls
                let has_remote_issues = !has_origin || !has_upstream;
                if !has_remote_issues {
                    activity.remove(&repo);
                    initial_repos.remove(&repo);
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
                    initial_repos.remove(&repo);
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
                    let has_non_modified = raw_entries
                        .iter()
                        .any(|e| !matches!(e.status, dracon_git::types::FileStatus::Modified));
                    if has_non_modified {
                        raw_entries
                            .into_iter()
                            .filter(|e| {
                                !matches!(e.status, dracon_git::types::FileStatus::Modified)
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    raw_entries
                        .into_iter()
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
                    dirty || status.ahead > 0 || status.behind > 0 || !has_origin || !has_upstream;
                if !has_local_or_pending_work {
                    activity.remove(&repo);
                    initial_repos.remove(&repo);
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
                        dirty_since: if effective_dirty { Some(now) } else { None },
                        ahead_since: if status.ahead > 0 { Some(now) } else { None },
                        behind_since: if status.behind > 0 { Some(now) } else { None },
                        mirror_consecutive_fails: HashMap::new(),
                        failure_count: 0,
                        remote_failures: HashMap::new(),
                    },
                );
                continue;
            };
            // Track when the repo first became dirty in this activity window.
            // This persists across fingerprint changes so that actively-edited
            // repos still get synced after a maximum delay (30s).
            if effective_dirty && entry.dirty_since.is_none() {
                entry.dirty_since = Some(now);
            } else if !effective_dirty {
                entry.dirty_since = None;
            }
            // Track ahead/behind state transitions for sustained-state notifications
            if status.ahead > 0 && entry.ahead_since.is_none() {
                entry.ahead_since = Some(now);
            } else if status.ahead == 0 {
                entry.ahead_since = None;
            }
            if status.behind > 0 && entry.behind_since.is_none() {
                entry.behind_since = Some(now);
            } else if status.behind == 0 {
                entry.behind_since = None;
            }
            if entry.fingerprint != fingerprint {
                entry.fingerprint = fingerprint;
                entry.changed_at = now;
                entry.failure_count = 0;
                // Don't skip if the repo has been dirty for > 5s —
                // sync it regardless of fingerprint changes.
                const MAX_DIRTY_DELAY: Duration = Duration::from_secs(5);
                let dirty_long_enough = entry
                    .dirty_since
                    .is_some_and(|since| now.duration_since(since) >= MAX_DIRTY_DELAY);
                if !dirty_long_enough {
                    continue;
                }
            }
            if now.duration_since(entry.changed_at) < inactivity_delay {
                // Same check for the stable-fingerprint case:
                // allow sync if dirty for > 5s even if fingerprint is stable.
                const MAX_DIRTY_DELAY: Duration = Duration::from_secs(5);
                let dirty_long_enough = entry
                    .dirty_since
                    .is_some_and(|since| now.duration_since(since) >= MAX_DIRTY_DELAY);
                if !dirty_long_enough {
                    continue;
                }
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
                    // Rate-limit: notify at most once per repo per 30 min
                    let notify_key = format!("timeout-{}", repo.display());
                    if let std::collections::hash_map::Entry::Vacant(e) =
                        remote_notify_cooldowns.entry(notify_key)
                    {
                        crate::report::send_sync_conflict_notification(
                            &repo,
                            "Sync Timeout",
                            &format!("exceeded {}s limit", policy.repo_sync_timeout_secs),
                        );
                        e.insert(Instant::now() + Duration::from_secs(1800));
                    }
                    false
                }
                Ok(Ok(crate::sync::SyncOutcome::Synced)) => {
                    eprintln!("🔁 synced {}", repo.display());
                    // Flush so journald captures sync activity in real-time
                    let _ = std::io::stderr().flush();
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
                        eprintln!(
                            "🐛 {} blocked (guard or manual intervention)",
                            repo.display()
                        );
                    }
                    false
                }
                Ok(Err(e)) => {
                    eprintln!("⚠️ sync failed for {}: {}", repo.display(), e);
                    let err_str = e.to_string();
                    if err_str.contains("push") || err_str.contains("remote") {
                        // Rate-limit: notify at most once per repo per 30 min
                        let notify_key = format!("pushfail-{}", repo.display());
                        if let std::collections::hash_map::Entry::Vacant(e) =
                            remote_notify_cooldowns.entry(notify_key)
                        {
                            crate::report::send_sync_conflict_notification(
                                &repo,
                                "Push Failed",
                                &err_str,
                            );
                            e.insert(Instant::now() + Duration::from_secs(1800));
                        }
                    }
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
                        if summary.found > 0 && summary.resolved_now == 0 && summary.succeeded == 0
                        {
                            should_cooldown = true;
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "⚠️ auto-repair concerns failed for {}: {}",
                            repo.display(),
                            e
                        );
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
                // Notify if this repo was previously stuck
                if stuck_push_repos.remove(&repo).is_some() {
                    save_stuck_push_repos(&stuck_push_repos);
                    crate::report::send_sync_conflict_notification(
                        &repo,
                        "Unstuck",
                        "push succeeded after being stuck",
                    );
                }
                entry.failure_count = 0;
                entry.remote_failures.clear();
                // Mirror pushes succeeded — reset consecutive fail counters
                for count in entry.mirror_consecutive_fails.values_mut() {
                    *count = 0;
                }
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
                        eprintln!(
                            "🐛 {} filter-only dirty, cooldown {}s",
                            repo.display(),
                            cooldown_secs
                        );
                    }
                }
                activity.remove(&repo);
                initial_repos.remove(&repo);
            } else {
                entry.failure_count += 1;

                // Re-fetch status after sync attempt before making stuck decisions.
                // sync_repo can resolve divergence (pull, merge, etc.), so stale
                // pre-sync status would produce false stuck markings.
                status = match svc.get_status().await {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("⚠️ {} post-sync status failed: {}", repo.display(), e);
                        status
                    }
                };

                // Check if ALL configured remotes are failing — desktop notification
                if !entry.remote_failures.is_empty() {
                    let all_failed = policy
                        .remotes
                        .iter()
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
                        if let std::collections::hash_map::Entry::Vacant(e) =
                            remote_notify_cooldowns.entry(notify_key)
                        {
                            let failed_list: Vec<_> =
                                entry.remote_failures.keys().cloned().collect();
                            let msg = format!(
                                "All remotes failing: {}. Failures: {:?}",
                                failed_list.join(", "),
                                entry.remote_failures
                            );
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
                if is_diverged || (!effective_dirty && status.ahead > 0 && entry.failure_count >= 3)
                {
                    let reason = if is_diverged {
                        format!(
                            "(diverged: ahead={}, behind={})",
                            status.ahead, status.behind
                        )
                    } else {
                        format!("(ahead={}, clean)", status.ahead)
                    };
                    eprintln!(
                        "🔒 {} permanently stuck on push {} skipping",
                        repo.display(),
                        reason
                    );
                    crate::report::send_sync_conflict_notification(
                        &repo,
                        "Stuck on Push",
                        &reason,
                    );
                    stuck_push_repos.insert(repo.clone(), timestamp_secs());
                    save_stuck_push_repos(&stuck_push_repos);
                    activity.remove(&repo);
                    initial_repos.remove(&repo);
                }
            }

            // Update mirror consecutive-fail tracking from remote_failures.
            // If a mirror has failures this cycle, increment its counter;
            // if it has no failures (not in remote_failures), reset to 0.
            if let Some(entry) = activity.get_mut(&repo) {
                for remote in &policy.remotes {
                    let count = entry.mirror_consecutive_fails
                        .entry(remote.name.clone())
                        .or_insert(0);
                    if entry.remote_failures.contains_key(&remote.name) {
                        *count += 1;
                    } else {
                        *count = 0;
                    }
                }
            }
        }

        // Flush stderr after each full scan cycle so journald captures
        // all output from this cycle. Rust's block buffering under systemd
        // can delay output for minutes without explicit flushes.
        let _ = std::io::stderr().flush();
        let _ = std::io::stdout().flush();

        // === Sustained-state notifications ===
        // Check for repos that have been in a concerning state for too long.
        // These fire once per repo per sustained incident, rate-limited to 30 min.
        let notification_now = Instant::now();
        const STUCK_AHEAD_THRESHOLD: Duration = Duration::from_secs(600);  // 10 min
        const STUCK_BEHIND_THRESHOLD: Duration = Duration::from_secs(1800); // 30 min
        const MIRROR_DEGRADED_THRESHOLD: usize = 3; // 3 consecutive fails

        for (repo, entry) in &activity {
            // Repo stuck ahead (unpushed commits piling up)
            if let Some(since) = entry.ahead_since {
                if notification_now.duration_since(since) >= STUCK_AHEAD_THRESHOLD {
                    let notify_key = format!("stuck-ahead-{}", repo.display());
                    if let std::collections::hash_map::Entry::Vacant(e) =
                        remote_notify_cooldowns.entry(notify_key.clone())
                    {
                        crate::report::send_sync_conflict_notification(
                            repo,
                            "Stuck Ahead (Unpushed)",
                            "commits not reaching origin for >10 min — push may be failing",
                        );
                        e.insert(Instant::now() + Duration::from_secs(1800));
                    }
                }
            }

            // Repo stuck behind (unpulled upstream changes)
            if let Some(since) = entry.behind_since {
                if notification_now.duration_since(since) >= STUCK_BEHIND_THRESHOLD {
                    let notify_key = format!("stuck-behind-{}", repo.display());
                    if let std::collections::hash_map::Entry::Vacant(e) =
                        remote_notify_cooldowns.entry(notify_key.clone())
                    {
                        crate::report::send_sync_conflict_notification(
                            repo,
                            "Stuck Behind (Unpulled)",
                            "upstream has unmerged changes for >30 min — pull may be failing",
                        );
                        e.insert(Instant::now() + Duration::from_secs(1800));
                    }
                }
            }

            // Mirror degraded (one mirror consistently failing)
            for (mirror_name, fail_count) in &entry.mirror_consecutive_fails {
                if *fail_count >= MIRROR_DEGRADED_THRESHOLD {
                    let notify_key = format!("mirror-{}-{}", repo.display(), mirror_name);
                    if let std::collections::hash_map::Entry::Vacant(e) =
                        remote_notify_cooldowns.entry(notify_key.clone())
                    {
                        crate::report::send_sync_conflict_notification(
                            repo,
                            &format!("Mirror Degraded: {}", mirror_name),
                            &format!("{} consecutive push failures — mirror may be unreachable", fail_count),
                        );
                        e.insert(Instant::now() + Duration::from_secs(1800));
                    }
                }
            }
        }

        sleep(Duration::from_secs(scan_interval)).await;
    }
    Ok(())
}
