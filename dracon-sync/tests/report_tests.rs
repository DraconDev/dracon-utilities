use dracon_git::types::RepoStatus;
use dracon_sync::report::{
    push_large_blob_threshold_bytes, repo_hint, repo_is_concern, repo_is_warn, repo_state_flags,
    truncate, SyncPolicy,
};

fn make_status(
    is_clean: bool,
    ahead: u32,
    behind: u32,
    modified: usize,
    staged: usize,
) -> RepoStatus {
    RepoStatus {
        is_clean,
        ahead,
        behind,
        modified_files: modified,
        staged_files: staged,
        last_commit_hash: None,
        last_commit_msg: None,
    }
}

#[test]
fn test_truncate_short_string() {
    assert_eq!(truncate("hello", 10), "hello");
}

#[test]
fn test_truncate_exact_length() {
    assert_eq!(truncate("hello", 5), "hello");
}

#[test]
fn test_truncate_long_string() {
    assert_eq!(truncate("hello world", 8), "hello wo…");
}

#[test]
fn test_truncate_empty_string() {
    assert_eq!(truncate("", 10), "");
}

#[test]
fn test_truncate_unicode() {
    let s = "hello 世界";
    let result = truncate(s, 8);
    assert!(result.ends_with('…'));
    assert_eq!(result.chars().count(), 8);
}

#[test]
fn test_repo_state_flags_clean_with_origin() {
    let status = make_status(true, 0, 0, 0, 0);
    let flags = repo_state_flags(&status, true, true);
    assert_eq!(flags, vec!["OK"]);
}

#[test]
fn test_repo_state_flags_dirty() {
    let status = make_status(false, 0, 0, 2, 0);
    let flags = repo_state_flags(&status, true, true);
    assert!(flags.contains(&"DIRTY".to_string()));
}

#[test]
fn test_repo_state_flags_ahead() {
    let status = make_status(false, 3, 0, 0, 0);
    let flags = repo_state_flags(&status, true, true);
    assert!(flags.iter().any(|f| f.starts_with("AHEAD:")));
}

#[test]
fn test_repo_state_flags_behind() {
    let status = make_status(false, 0, 5, 0, 0);
    let flags = repo_state_flags(&status, true, true);
    assert!(flags.iter().any(|f| f.starts_with("BEHIND:")));
}

#[test]
fn test_repo_state_flags_no_origin() {
    let status = make_status(true, 0, 0, 0, 0);
    let flags = repo_state_flags(&status, false, false);
    assert!(flags.contains(&"NO_ORIGIN".to_string()));
}

#[test]
fn test_repo_state_flags_no_upstream() {
    let status = make_status(true, 0, 0, 0, 0);
    let flags = repo_state_flags(&status, true, false);
    assert!(flags.contains(&"NO_UPSTREAM".to_string()));
}

#[test]
fn test_repo_state_flags_stuck_push() {
    let status = make_status(false, 3, 0, 0, 0);
    let flags = repo_state_flags(&status, true, true);
    assert!(flags.contains(&"STUCK_PUSH".to_string()));
}

#[test]
fn test_repo_state_flags_stuck_pull() {
    let status = make_status(false, 0, 2, 0, 0);
    let flags = repo_state_flags(&status, true, true);
    assert!(flags.contains(&"STUCK_PULL".to_string()));
}

#[test]
fn test_repo_state_flags_multiple() {
    let status = make_status(false, 3, 2, 0, 0);
    let flags = repo_state_flags(&status, true, true);
    assert!(flags.contains(&"DIRTY".to_string()));
    assert!(flags.iter().any(|f| f.starts_with("AHEAD:")));
    assert!(flags.iter().any(|f| f.starts_with("BEHIND:")));
}

#[test]
fn test_repo_is_concern_no_origin() {
    let status = make_status(true, 0, 0, 0, 0);
    assert!(repo_is_concern(&status, false, false));
}

#[test]
fn test_repo_is_concern_ahead() {
    let status = make_status(false, 5, 0, 0, 0);
    assert!(repo_is_concern(&status, true, true));
}

#[test]
fn test_repo_is_concern_behind() {
    let status = make_status(false, 0, 3, 0, 0);
    assert!(repo_is_concern(&status, true, true));
}

#[test]
fn test_repo_is_concern_no_upstream() {
    let status = make_status(true, 0, 0, 0, 0);
    assert!(repo_is_concern(&status, true, false));
}

#[test]
fn test_repo_is_concern_clean() {
    let status = make_status(true, 0, 0, 0, 0);
    assert!(!repo_is_concern(&status, true, true));
}

#[test]
fn test_repo_is_warn_dirty_but_healthy() {
    let status = make_status(false, 0, 0, 2, 0);
    assert!(!repo_is_warn(&status, true, true));
    assert!(!repo_is_concern(&status, true, true));
}

#[test]
fn test_repo_is_warn_dirty_with_origin() {
    let status = make_status(false, 0, 0, 2, 0);
    assert!(repo_is_warn(&status, true, true));
}

#[test]
fn test_repo_is_warn_no_origin() {
    let status = make_status(false, 0, 0, 2, 0);
    assert!(!repo_is_warn(&status, false, false));
}

#[test]
fn test_repo_hint_no_origin() {
    let hint = repo_hint(&["NO_ORIGIN"], false, false);
    assert_eq!(hint, "set origin remote");
}

#[test]
fn test_repo_hint_no_upstream() {
    let hint = repo_hint(&["NO_UPSTREAM"], false, false);
    assert_eq!(hint, "run repair-concerns --apply (set upstream)");
}

#[test]
fn test_repo_hint_ahead() {
    let hint = repo_hint(&["AHEAD:3"], false, false);
    assert_eq!(hint, "run repair-concerns --apply (push or rewrite)");
}

#[test]
fn test_repo_hint_behind() {
    let hint = repo_hint(&["BEHIND:2"], false, false);
    assert_eq!(hint, "run repair-concerns --apply (pull/rebase)");
}

#[test]
fn test_repo_hint_healthy() {
    let hint = repo_hint(&["OK"], false, false);
    assert_eq!(hint, "healthy");
}

#[test]
fn test_repo_hint_warn() {
    let hint = repo_hint(&["DIRTY"], true, false);
    assert_eq!(hint, "run repair-warns --apply");
}

#[test]
fn test_repo_hint_concern() {
    let hint = repo_hint(&["DIRTY"], false, true);
    assert_eq!(hint, "run repair-concerns --apply");
}

#[test]
fn test_push_large_blob_threshold_bytes() {
    let policy = SyncPolicy {
        max_stage_file_bytes: 100 * 1024 * 1024,
        max_push_blob_bytes: 50 * 1024 * 1024,
        ..Default::default()
    };
    let threshold = push_large_blob_threshold_bytes(&policy);
    assert_eq!(threshold, 50 * 1024 * 1024);
}

#[test]
fn test_push_large_blob_threshold_caps_at_blob_limit() {
    let policy = SyncPolicy {
        max_stage_file_bytes: 200 * 1024 * 1024,
        max_push_blob_bytes: 50 * 1024 * 1024,
        ..Default::default()
    };
    let threshold = push_large_blob_threshold_bytes(&policy);
    assert_eq!(threshold, 50 * 1024 * 1024);
}

impl Default for SyncPolicy {
    fn default() -> Self {
        SyncPolicy {
            system_repo: String::new(),
            pulse_interval_secs: 1,
            inactivity_push_delay_secs: 5,
            auto_commit: true,
            auto_bump_versions: true,
            auto_pull: true,
            auto_push: true,
            backup_policy: String::new(),
            backup_dir: String::new(),
            exclude_repos: vec![],
            exclude_dir_names: vec![],
            exclude_file_patterns: vec![],
            auto_repair_concerns: true,
            auto_repair_warns: true,
            auto_rewrite_large_blobs: true,
            watch_roots: vec![],
            extra_remotes: vec![],
            auto_github_private: false,
            auto_github_private_account: "DraconDev".to_string(),
            max_stage_file_bytes: 100 * 1024 * 1024,
            pull_op_timeout_secs: 30,
            push_op_timeout_secs: 300,
            repo_sync_timeout_secs: 420,
            push_retries: 3,
            repair_cooldown_secs: 60,
            max_push_blob_bytes: 100 * 1024 * 1024,
            incident_ledger_max_lines: 10_000,
            incident_ledger_max_age_days: 30,
        }
    }
}
