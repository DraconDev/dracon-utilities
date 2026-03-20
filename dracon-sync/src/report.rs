use anyhow::{Context, Result};
use dracon_common::{ansi, emit_event, DraconEvent, EventSeverity};
use dracon_git::{
    build_commit_message,
    types::{DiffFile, FileStatus, RepoStatus},
    CommitContext, extract_intent, GitService,
};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::Duration;

use crate::bump::{bump_patch_version_in_repo, bump_node_package_version_in_repo, bump_version_file_in_repo};
use crate::exclude::{
    excluded_dir_names_set, should_stage_entry, is_excluded_change_path,
    has_sync_relevant_dirty_entries, is_large_untracked, can_restore_entry,
    append_to_gitignore, handle_large_untracked, is_excluded_file,
    is_excluded_dir_name,
};
use crate::git::{
    has_origin_remote, has_tracking_upstream, discover_git_repos, origin_url,
    strip_url_credentials, push_with_retries, rewrite_ahead_paths, current_branch,
    remote_branch_exists, set_upstream_to_branch, detect_large_blobs_ahead,
    top_level_dir, repo_diff_entries, run_git_with_timeout, run_git_capture_output,
    unstage_excluded_paths, unstage_oversized_paths, staged_paths,
};
use crate::policy::{
    SyncPolicy, resolve_policy_path, freeze_reason, debug_enabled,
    DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES, tokio_git_command,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepoFilter {
    All,
    Concern,
    Warn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConcernRepairFilter {
    All,
    StuckPush,
    StuckPull,
}

#[derive(Debug, Serialize)]
pub(crate) struct RepoReportRow {
    repo: String,
    state_flags: Vec<String>,
    branch: String,
    modified: usize,
    staged: usize,
    ahead: usize,
    behind: usize,
    last_hash: String,
    last_author: String,
    last_when: String,
    last_msg: String,
    last_unix: i64,
    concern: bool,
    warn: bool,
    hint: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RepoReportJson {
    policy: String,
    filter: String,
    repos: usize,
    ok: usize,
    warn: usize,
    concern: usize,
    failures: usize,
    rows: Vec<RepoReportRow>,
}

#[derive(Debug, Serialize)]
pub(crate) struct StatusJson {
    pub(crate) policy: String,
    pub(crate) roots: Vec<String>,
    pub(crate) repos_discovered: usize,
    pub(crate) pulse_interval_secs: u64,
    pub(crate) inactivity_push_delay_secs: u64,
    pub(crate) freeze: String,
    pub(crate) auto_commit: bool,
    pub(crate) auto_pull: bool,
    pub(crate) auto_push: bool,
    pub(crate) auto_bump_versions: bool,
    pub(crate) auto_repair_concerns: bool,
    pub(crate) auto_repair_warns: bool,
    pub(crate) auto_rewrite_large_blobs: bool,
    pub(crate) max_stage_file_bytes: u64,
    pub(crate) push_blob_threshold_bytes: u64,
    pub(crate) exclude_dirs: Vec<String>,
    pub(crate) exclude_file_patterns: Vec<String>,
    pub(crate) pull_op_timeout_secs: u64,
    pub(crate) push_op_timeout_secs: u64,
    pub(crate) repo_sync_timeout_secs: u64,
    pub(crate) push_retries: u32,
    pub(crate) repair_cooldown_secs: u64,
    pub(crate) incident_ledger_max_lines: usize,
    pub(crate) incident_ledger_max_age_days: u64,
    pub(crate) system_repo: String,
    pub(crate) backup_policy: String,
    pub(crate) backup_dir: String,
    pub(crate) extra_remotes: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct RepairJson {
    policy: String,
    scope: String,
    mode: String,
    found: usize,
    planned: usize,
    attempted: usize,
    succeeded: usize,
    resolved_now: usize,
    manual_only: usize,
    ledger: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct RepairSummary {
    pub(crate) found: usize,
    pub(crate) planned: usize,
    pub(crate) attempted: usize,
    pub(crate) succeeded: usize,
    pub(crate) resolved_now: usize,
    pub(crate) manual_only: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct IncidentRecord {
    ts_unix: u64,
    scope: String,
    repo: String,
    reason: String,
    action: String,
    backup_branch: Option<String>,
    result: String,
    details: Option<String>,
}

impl IncidentRecord {
    pub(crate) fn new(scope: &str, repo: &str, reason: &str, action: &str, result: &str) -> Self {
        Self {
            ts_unix: timestamp_secs(),
            scope: scope.to_string(),
            repo: repo.to_string(),
            reason: reason.to_string(),
            action: action.to_string(),
            backup_branch: None,
            result: result.to_string(),
            details: None,
        }
    }

    pub(crate) fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    pub(crate) fn with_backup_branch(mut self, branch: &str) -> Self {
        self.backup_branch = Some(branch.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ReportSignal {
    ActiveBoardChanged,
    IndexChanged,
    BlueprintCreated,
    BlueprintModified,
}

pub(crate) fn detect_report_signals(
    _repo: &Path,
    changed_files: &[DiffFile],
) -> Vec<ReportSignal> {
    let mut signals = Vec::new();
    
    for file in changed_files {
        let path_str = file.path.to_string_lossy();
        
        if path_str == "plan/ACTIVE_BOARD.md" || path_str.ends_with("/ACTIVE_BOARD.md") {
            signals.push(ReportSignal::ActiveBoardChanged);
        }
        
        if path_str == "plan/index.md" || path_str.ends_with("/index.md") {
            signals.push(ReportSignal::IndexChanged);
        }
        
        if path_str.contains("blueprint-") && path_str.ends_with(".md") {
            if file.status == dracon_git::types::FileStatus::Added {
                signals.push(ReportSignal::BlueprintCreated);
            } else {
                signals.push(ReportSignal::BlueprintModified);
            }
        }
    }
    
    signals
}

pub(crate) fn read_project_focus(repo: &Path) -> Option<String> {
    let state_path = repo.join(".dracon/project-state.md");
    let content = std::fs::read_to_string(&state_path).ok()?;
    
    // Return full project-state.md content for rich commit bodies
    // This gives AI reading git history full context (Completed, In Progress, Open Issues)
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn build_commit_context(
    repo: &Path,
    status: &RepoStatus,
    entries: &[DiffFile],
    is_checkpoint: bool,
    idle_seconds: u64,
) -> CommitContext {
    let changed_paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
    let intent_info = extract_intent(repo, &changed_paths, Some(&status.branch));
    
    let refs = intent_info.blueprint.as_ref().map(|p| {
        let rel = p.strip_prefix(repo).unwrap_or(p);
        rel.to_string_lossy().to_string()
    });
    
    // Read project state for commit body (scribe)
    let description = read_project_focus(repo);
    
    CommitContext {
        intent: intent_info.intent,
        track: intent_info.track,
        is_checkpoint,
        files: entries.to_vec(),
        task_progress: intent_info.task_progress,
        refs,
        idle_seconds,
        category: None,
        scope: None,
        severity: None,
        description,
        semantic_summary: None,
    }
}

pub(crate) fn timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(crate) fn incident_ledger_path(_policy_path: &Path) -> PathBuf {
    // IMPORTANT: Keep this ledger OUT of git repositories by default.
    // The policy file typically lives inside the system repo; writing next to it
    // causes perpetual DIRTY state and churn.
    if let Ok(custom) = std::env::var("DRACON_SYNC_LEDGER") {
        let p = PathBuf::from(custom);
        if !p.as_os_str().is_empty() {
            return p;
        }
    }

    if let Some(home) = dirs::home_dir() {
        return home.join(".dracon").join("dracon-sync-incidents.jsonl");
    }

    PathBuf::from("/tmp/dracon-sync-incidents.jsonl")
}

pub(crate) fn append_incident_record(policy_path: &Path, record: &IncidentRecord) {
    fn enforce_retention(path: &Path, policy: &SyncPolicy) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let now = timestamp_secs();
        let age_cutoff = now.saturating_sub(policy.incident_ledger_max_age_days.saturating_mul(86_400));

        let mut kept: Vec<String> = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let keep_by_age = serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|v| v.get("ts_unix").and_then(|t| t.as_u64()))
                .map(|ts| ts >= age_cutoff)
                .unwrap_or(true);
            if keep_by_age {
                kept.push(line.to_string());
            }
        }
        if kept.len() > policy.incident_ledger_max_lines {
            let drop_n = kept.len() - policy.incident_ledger_max_lines;
            kept.drain(0..drop_n);
        }
        let mut out = String::new();
        for line in kept {
            out.push_str(&line);
            out.push('\n');
        }
        std::fs::write(path, &out)?;

        Ok(())
    }
    // ── append logic ──
    let path = incident_ledger_path(policy_path);
    let line = match serde_json::to_string(record) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("⚠️ incident serialize failed: {}", e);
            return;
        }
    };
    let parent = path.parent().map(Path::to_path_buf);
    if let Some(dir) = parent {
        let _ = std::fs::create_dir_all(dir);
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut file) => {
            use std::io::Write;
            if let Err(e) = writeln!(file, "{}", line) {
                eprintln!("⚠️ incident write failed ({}): {}", path.display(), e);
            } else if let Ok(policy) = SyncPolicy::load(policy_path) {
                if let Err(e) = enforce_retention(&path, &policy) {
                    eprintln!("⚠️ incident retention failed ({}): {}", path.display(), e);
                }
            }
        }
        Err(e) => eprintln!("⚠️ incident open failed ({}): {}", path.display(), e),
    }
}

pub(crate) fn has_permanent_failure(repo: &Path) -> bool {
    let ledger_path = incident_ledger_path(Path::new("/dummy"));
    if !ledger_path.exists() {
        return false;
    }
    let repo_display = repo.display().to_string();
    if let Ok(content) = std::fs::read_to_string(&ledger_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                let repo_match = v.get("repo")
                    .and_then(|r| r.as_str())
                    .map(|r| r == repo_display)
                    .unwrap_or(false);
                let result_perm = v.get("result")
                    .and_then(|r| r.as_str())
                    .map(|r| r == "permanent_fail")
                    .unwrap_or(false);
                if repo_match && result_perm {
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn repo_state_flags(
    status: &dracon_git::types::RepoStatus,
    has_origin: bool,
    has_upstream: bool,
) -> Vec<String> {
    let mut flags = Vec::new();
    if !status.is_clean {
        flags.push("DIRTY".to_string());
    }
    if status.ahead > 0 {
        flags.push(format!("AHEAD:{}", status.ahead));
    }
    if status.behind > 0 {
        flags.push(format!("BEHIND:{}", status.behind));
    }
    if !has_origin {
        flags.push("NO_ORIGIN".to_string());
    }
    if has_origin && !has_upstream {
        flags.push("NO_UPSTREAM".to_string());
    }
    if status.ahead > 0 && has_origin && has_upstream {
        flags.push("STUCK_PUSH".to_string());
    }
    if status.behind > 0 && has_origin && has_upstream {
        flags.push("STUCK_PULL".to_string());
    }
    if flags.is_empty() {
        flags.push("OK".to_string());
    }
    flags
}

pub(crate) fn repo_is_concern(status: &dracon_git::types::RepoStatus, has_origin: bool, has_upstream: bool) -> bool {
    status.ahead > 0 || status.behind > 0 || !has_origin || (has_origin && !has_upstream)
}

pub(crate) fn repo_is_warn(status: &dracon_git::types::RepoStatus, has_origin: bool, has_upstream: bool) -> bool {
    !repo_is_concern(status, has_origin, has_upstream) && !status.is_clean
}

pub(crate) fn repo_hint(flags: &[String], warn: bool, concern: bool) -> String {
    if flags.iter().any(|f| f == "NO_ORIGIN") {
        return "set origin remote".to_string();
    }
    if flags.iter().any(|f| f == "NO_UPSTREAM") {
        return "run repair-concerns --apply (set upstream)".to_string();
    }
    if flags.iter().any(|f| f.starts_with("AHEAD:")) {
        return "run repair-concerns --apply (push or rewrite)".to_string();
    }
    if flags.iter().any(|f| f.starts_with("BEHIND:")) {
        return "run repair-concerns --apply (pull/rebase)".to_string();
    }
    if warn {
        return "run repair-warns --apply".to_string();
    }
    if concern {
        return "run repair-concerns --apply".to_string();
    }
    "healthy".to_string()
}

pub(crate) fn push_large_blob_threshold_bytes(policy: &SyncPolicy) -> u64 {
    policy
        .max_stage_file_bytes
        .min(policy.max_push_blob_bytes)
        .min(DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES)
}

pub(crate) fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let shortened: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", shortened)
}

pub(crate) async fn git_log_field(repo: &Path, format: &str) -> Option<String> {
    let output = tokio_git_command()
        .args(["log", "-1", &format!("--pretty=format:{}", format)])
        .current_dir(repo)
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub(crate) async fn git_log_unix_timestamp(repo: &Path) -> Option<i64> {
    git_log_field(repo, "%ct")
        .await
        .and_then(|s| s.parse::<i64>().ok())
}

pub(crate) async fn run_repos_report(policy_path: &Path, filter: RepoFilter, json: bool) -> Result<()> {
    let policy = SyncPolicy::load(policy_path)?;
    let roots = policy.watch_root_paths();
    let excluded_dir_names = excluded_dir_names_set(&policy);
    let repos = discover_git_repos(&roots, &excluded_dir_names);
    let mut rows: Vec<RepoReportRow> = Vec::new();
    let mut init_or_status_failures = 0usize;

    for repo in repos {
        let svc = match GitService::new(&repo) {
            Ok(svc) => svc,
            Err(e) => {
                init_or_status_failures += 1;
                println!(
                    "{} {} | init_failed: {}",
                    ansi("31", "❌"),
                    repo.display(),
                    e
                );
                continue;
            }
        };

        let status = match svc.get_status().await {
            Ok(status) => status,
            Err(e) => {
                init_or_status_failures += 1;
                println!(
                    "{} {} | status_failed: {}",
                    ansi("31", "❌"),
                    repo.display(),
                    e
                );
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
        let effective_status = dracon_git::types::RepoStatus {
            is_clean: !effective_dirty,
            modified_files: if effective_dirty { status.modified_files } else { 0 },
            ..status.clone()
        };

        let has_origin = has_origin_remote(&repo);
        let has_upstream = has_tracking_upstream(&repo);

        let flags = repo_state_flags(&effective_status, has_origin, has_upstream);

        let last_hash = status
            .last_commit_hash
            .as_deref()
            .map(|h| truncate(h, 12))
            .unwrap_or_else(|| "-".to_string());
        let last_msg = status
            .last_commit_msg
            .as_deref()
            .map(|m| truncate(m, 72))
            .unwrap_or_else(|| "-".to_string());
        let last_author = git_log_field(&repo, "%an")
            .await
            .unwrap_or_else(|| "-".to_string());
        let last_when = git_log_field(&repo, "%ar")
            .await
            .unwrap_or_else(|| "-".to_string());
        let last_unix = git_log_unix_timestamp(&repo).await.unwrap_or(0);

        let concern = repo_is_concern(&effective_status, has_origin, has_upstream);
        let warn = repo_is_warn(&effective_status, has_origin, has_upstream);
        let hint = repo_hint(&flags, warn, concern);

        rows.push(RepoReportRow {
            repo: repo.display().to_string(),
            state_flags: flags,
            branch: effective_status.branch,
            modified: effective_status.modified_files,
            staged: effective_status.staged_files,
            ahead: effective_status.ahead,
            behind: effective_status.behind,
            last_hash,
            last_author,
            last_when,
            last_msg,
            last_unix,
            concern,
            warn,
            hint,
        });
    }

    rows.sort_by(|a, b| b.last_unix.cmp(&a.last_unix));

    let concern_count_all = rows.iter().filter(|r| r.concern).count();
    let warn_count_all = rows.iter().filter(|r| r.warn).count();
    let ok_count_all = rows
        .len()
        .saturating_sub(concern_count_all + warn_count_all);
    match filter {
        RepoFilter::All => {}
        RepoFilter::Concern => rows.retain(|r| r.concern),
        RepoFilter::Warn => rows.retain(|r| r.warn),
    }

    let concern_count = rows.iter().filter(|r| r.concern).count();
    let warn_count = rows.iter().filter(|r| r.warn).count();
    let ok_count = rows.len().saturating_sub(concern_count + warn_count);
    let filter_text = match filter {
        RepoFilter::All => "all",
        RepoFilter::Concern => "only_concern",
        RepoFilter::Warn => "only_warn",
    };

    if json {
        let payload = RepoReportJson {
            policy: policy_path.display().to_string(),
            filter: filter_text.to_string(),
            repos: rows.len(),
            ok: ok_count,
            warn: warn_count,
            concern: concern_count,
            failures: init_or_status_failures,
            rows,
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!("📜 POLICY: {}", policy_path.display());
    match filter {
        RepoFilter::All => {}
        RepoFilter::Concern => {
            println!(
                "📊 FILTER: only concern repos (showing {} of {})",
                rows.len(),
                concern_count_all
            );
        }
        RepoFilter::Warn => {
            println!(
                "📊 FILTER: only warn repos (showing {} of {})",
                rows.len(),
                warn_count_all
            );
        }
    }
    println!(
        "📦 REPOS: {}  {} {}  {} {}  {} {}  ❌ {}{}",
        rows.len(),
        ansi("32", "OK"),
        ok_count,
        ansi("33", "WARN"),
        warn_count,
        ansi("31", "CONCERN"),
        concern_count,
        init_or_status_failures,
        match filter {
            RepoFilter::All => String::new(),
            RepoFilter::Concern | RepoFilter::Warn => format!(
                "  (all: OK {} WARN {} CONCERN {})",
                ok_count_all, warn_count_all, concern_count_all
            ),
        }
    );
    println!("🕒 SORT: last modified (newest first)");
    println!();

    for (idx, row) in rows.iter().enumerate() {
        let severity = if row.concern {
            ansi("31", "CONCERN")
        } else if row.warn {
            ansi("33", "WARN")
        } else {
            ansi("32", "OK")
        };

        println!("{}. [{}] {}", idx + 1, severity, row.repo);
        println!(
            "   updated={} branch={} state={} modified={} staged={} ahead={} behind={}",
            row.last_when,
            row.branch,
            row.state_flags.join(","),
            row.modified,
            row.staged,
            row.ahead,
            row.behind
        );
        println!(
            "   last={} by {} {}",
            row.last_hash, row.last_author, row.last_msg
        );
        println!("   hint={}", row.hint);
        println!();
    }

    Ok(())
}

pub(crate) async fn run_repair_concerns(
    policy_path: &Path,
    apply: bool,
    only_repo: Option<PathBuf>,
    push_timeout_override: Option<u64>,
    push_retries: u32,
    rewrite_large_any: bool,
    filter: ConcernRepairFilter,
    json: bool,
) -> Result<RepairSummary> {
    let human = !json;
    macro_rules! out {
        ($($arg:tt)*) => {{
            if human {
                println!($($arg)*);
            }
        }};
    }

    let policy = SyncPolicy::load(policy_path)?;
    let roots = policy.watch_root_paths();
    let excluded_dir_names = excluded_dir_names_set(&policy);
    let mut repos = discover_git_repos(&roots, &excluded_dir_names);
    if let Some(target_repo) = only_repo {
        repos.retain(|r| r == &target_repo);
        if repos.is_empty() {
            out!(
                "⚠️ target repo not discovered in policy roots: {}",
                target_repo.display()
            );
            return Ok(RepairSummary::default());
        }
    }
    let push_timeout_secs = push_timeout_override
        .unwrap_or(policy.push_op_timeout_secs)
        .max(10);
    let push_retries = push_retries.max(1);
    let blob_threshold = push_large_blob_threshold_bytes(&policy);

    let mut concerns = 0usize;
    let mut attempted_ops = 0usize;
    let mut succeeded_ops = 0usize;
    let mut manual_only = 0usize;
    let mut resolved = 0usize;

    out!("📜 POLICY: {}", policy_path.display());
    out!(
        "🛠️ MODE: {}",
        if apply {
            "APPLY (mutating)"
        } else {
            "DRY-RUN (no changes)"
        }
    );
    out!(
        "⚙️ PUSH: timeout={}s retries={}",
        push_timeout_secs, push_retries
    );

    for repo in repos {
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

        let has_origin = has_origin_remote(&repo);
        let mut has_upstream = has_tracking_upstream(&repo);
        let is_concern = repo_is_concern(&status, has_origin, has_upstream);
        if !is_concern {
            continue;
        }
        let stuck_push = status.ahead > 0 && has_origin && has_upstream;
        let stuck_pull = status.behind > 0 && has_origin && has_upstream;
        if matches!(filter, ConcernRepairFilter::StuckPush) && !stuck_push {
            continue;
        }
        if matches!(filter, ConcernRepairFilter::StuckPull) && !stuck_pull {
            continue;
        }
        concerns += 1;
        let flags = repo_state_flags(&status, has_origin, has_upstream);
        let reason = flags.join(",");

        out!(
            "\n🔎 {}  state: ahead={} behind={} clean={} origin={} upstream={}",
            repo.display(),
            status.ahead,
            status.behind,
            status.is_clean,
            has_origin,
            has_upstream
        );

        if !has_origin {
            manual_only += 1;
            out!("   manual: NO_ORIGIN (configure remote before sync can repair)");
            append_incident_record(
                policy_path,
                &IncidentRecord {
                    ts_unix: timestamp_secs(),
                    scope: "concern".to_string(),
                    repo: repo.display().to_string(),
                    reason: reason.clone(),
                    action: "manual_no_origin".to_string(),
                    backup_branch: None,
                    result: "manual".to_string(),
                    details: Some("configure origin remote".to_string()),
                },
            );
            continue;
        }

        if !has_upstream {
            attempted_ops += 1;
            out!("   plan: set upstream via `git push -u origin HEAD`");
            if apply {
                match run_git_with_timeout(
                    &repo,
                    &["push", "-u", "origin", "HEAD"],
                    push_timeout_secs,
                    "push -u",
                )
                .await
                {
                    Ok(()) => {
                        succeeded_ops += 1;
                        has_upstream = true;
                        out!("   ok: upstream configured");
                        append_incident_record(
                            policy_path,
                            &IncidentRecord {
                                ts_unix: timestamp_secs(),
                                scope: "concern".to_string(),
                                repo: repo.display().to_string(),
                                reason: reason.clone(),
                                action: "set_upstream_push_u".to_string(),
                                backup_branch: None,
                                result: "ok".to_string(),
                                details: None,
                            },
                        );
                    }
                    Err(e) => {
                        out!("   fail: upstream configure failed: {}", e);
                        append_incident_record(
                            policy_path,
                            &IncidentRecord {
                                ts_unix: timestamp_secs(),
                                scope: "concern".to_string(),
                                repo: repo.display().to_string(),
                                reason: reason.clone(),
                                action: "set_upstream_push_u".to_string(),
                                backup_branch: None,
                                result: "fail".to_string(),
                                details: Some(e.to_string()),
                            },
                        );
                        continue;
                    }
                }
            }
        }

        if status.behind > 0 && has_upstream {
            attempted_ops += 1;
            out!("   plan: pull --rebase --autostash");
            if apply {
                match run_git_with_timeout(
                    &repo,
                    &["pull", "--rebase", "--autostash"],
                    policy.pull_op_timeout_secs,
                    "pull/rebase",
                )
                .await
                {
                    Ok(()) => {
                        succeeded_ops += 1;
                        out!("   ok: pulled");
                        append_incident_record(
                            policy_path,
                            &IncidentRecord {
                                ts_unix: timestamp_secs(),
                                scope: "concern".to_string(),
                                repo: repo.display().to_string(),
                                reason: reason.clone(),
                                action: "pull_rebase_autostash".to_string(),
                                backup_branch: None,
                                result: "ok".to_string(),
                                details: None,
                            },
                        );
                    }
                    Err(e) => {
                        out!("   fail: pull failed: {}", e);
                        append_incident_record(
                            policy_path,
                            &IncidentRecord {
                                ts_unix: timestamp_secs(),
                                scope: "concern".to_string(),
                                repo: repo.display().to_string(),
                                reason: reason.clone(),
                                action: "pull_rebase_autostash".to_string(),
                                backup_branch: None,
                                result: "fail".to_string(),
                                details: Some(e.to_string()),
                            },
                        );
                    }
                }
            }
        }

        if status.ahead > 0 && has_upstream {
            attempted_ops += 1;
            out!("   plan: push origin HEAD");
            if apply {
                let mut push_ok = false;
                match push_with_retries(&repo, push_timeout_secs, push_retries, "push").await {
                    Ok(()) => {
                        succeeded_ops += 1;
                        push_ok = true;
                        out!("   ok: pushed");
                        append_incident_record(
                            policy_path,
                            &IncidentRecord {
                                ts_unix: timestamp_secs(),
                                scope: "concern".to_string(),
                                repo: repo.display().to_string(),
                                reason: reason.clone(),
                                action: "push_origin_head".to_string(),
                                backup_branch: None,
                                result: "ok".to_string(),
                                details: None,
                            },
                        );
                    }
                    Err(e) => {
                        out!("   fail: push failed: {}", e);

                        let err_str = e.to_string().to_lowercase();
                        
                        // Check if push failed because remote doesn't exist or is unreachable
                        // In this case, auto-create a private bare repo as the remote
                        let no_remote = err_str.contains("no such remote")
                            || err_str.contains("remote does not exist")
                            || err_str.contains("repository not found")
                            || err_str.contains("could not resolve host")
                            || (err_str.contains("exit status: 128") && err_str.contains("fatal:"));

                        if no_remote {
                            // Try to create a private bare repo and use it as origin
                            out!("   info: no remote detected, creating private bare repo");
                            if let Some(private_remote) = create_private_remote(&repo) {
                                out!("   info: created private remote: {}", private_remote);
                                // Retry push with new remote
                                match push_with_retries(&repo, push_timeout_secs, push_retries, "push").await {
                                    Ok(()) => {
                                        succeeded_ops += 1;
                                        push_ok = true;
                                        out!("   ok: pushed to private remote");
                                        append_incident_record(
                                            policy_path,
                                            &IncidentRecord {
                                                ts_unix: timestamp_secs(),
                                                scope: "concern".to_string(),
                                                repo: repo.display().to_string(),
                                                reason: reason.clone(),
                                                action: "push_origin_head".to_string(),
                                                backup_branch: None,
                                                result: "ok".to_string(),
                                                details: Some(format!("pushed to private remote: {}", private_remote)),
                                            },
                                        );
                                    }
                                    Err(e2) => {
                                        out!("   fail: push to private remote also failed: {}", e2);
                                        append_incident_record(
                                            policy_path,
                                            &IncidentRecord {
                                                ts_unix: timestamp_secs(),
                                                scope: "concern".to_string(),
                                                repo: repo.display().to_string(),
                                                reason: reason.clone(),
                                                action: "push_origin_head".to_string(),
                                                backup_branch: None,
                                                result: "fail".to_string(),
                                                details: Some(e2.to_string()),
                                            },
                                        );
                                    }
                                }
                            } else {
                                out!("   fail: could not create private remote");
                                append_incident_record(
                                    policy_path,
                                    &IncidentRecord {
                                        ts_unix: timestamp_secs(),
                                        scope: "concern".to_string(),
                                        repo: repo.display().to_string(),
                                        reason: reason.clone(),
                                        action: "push_origin_head".to_string(),
                                        backup_branch: None,
                                        result: "fail".to_string(),
                                        details: Some(e.to_string()),
                                    },
                                );
                            }
                            // Skip the large blob and rewrite logic since we handled (or failed) the push
                            continue;
                        }

                        // For permission denied or other errors on existing remote, 
                        // just record failure and continue - no permanent marking
                        // These will retry on next cycle naturally
                        append_incident_record(
                            policy_path,
                            &IncidentRecord {
                                ts_unix: timestamp_secs(),
                                scope: "concern".to_string(),
                                repo: repo.display().to_string(),
                                reason: reason.clone(),
                                action: "push_origin_head".to_string(),
                                backup_branch: None,
                                result: "fail".to_string(),
                                details: Some(e.to_string()),
                            },
                        );
                        // Skip to end of push handling - don't try large blob detection for permission errors
                        continue;
                    }
                            .unwrap_or_default();
                        if !large.is_empty() {
                            out!(
                                "   detect: large blobs in ahead range ({} entries)",
                                large.len()
                            );
                            let mut dirs = BTreeSet::new();
                            for (_, path) in &large {
                                if let Some(dir) = top_level_dir(path) {
                                    if is_excluded_dir_name(&dir, &excluded_dir_names) {
                                        dirs.insert(dir);
                                    }
                                }
                            }
                            let dirs: Vec<String> = dirs.into_iter().collect();
                            let rewrite_paths: Vec<String> = if !dirs.is_empty() {
                                dirs
                            } else if rewrite_large_any {
                                let mut unique = BTreeSet::new();
                                for (_, p) in &large {
                                    unique.insert(p.clone());
                                }
                                unique.into_iter().collect()
                            } else {
                                Vec::new()
                            };

                            if rewrite_paths.is_empty() {
                                out!("   manual: large blobs found but not in excluded dirs");
                                append_incident_record(
                                    policy_path,
                                    &IncidentRecord {
                                        ts_unix: timestamp_secs(),
                                        scope: "concern".to_string(),
                                        repo: repo.display().to_string(),
                                        reason: reason.clone(),
                                        action: "large_blob_detected".to_string(),
                                        backup_branch: None,
                                        result: "manual".to_string(),
                                        details: Some(format!(
                                            "threshold={} entries={} rewrite_allowed=false",
                                            blob_threshold,
                                            large.len()
                                        )),
                                    },
                                );
                            } else {
                                out!(
                                    "   plan: rewrite ahead history removing paths {:?}",
                                    rewrite_paths
                                );
                                match rewrite_ahead_paths(
                                    &repo,
                                    &rewrite_paths,
                                    "backup/pre-sync-largeblob-fix",
                                ) {
                                    Ok(Some(backup_branch)) => {
                                        let backup_branch_for_log = backup_branch.clone();
                                        out!(
                                            "   ok: rewrite complete (backup branch: {})",
                                            backup_branch
                                        );
                                        match push_with_retries(
                                            &repo,
                                            push_timeout_secs,
                                            push_retries,
                                            "push-after-rewrite",
                                        )
                                        .await
                                        {
                                            Ok(()) => {
                                                succeeded_ops += 1;
                                                push_ok = true;
                                                out!("   ok: pushed after rewrite");
                                                append_incident_record(
                                                    policy_path,
                                                    &IncidentRecord {
                                                        ts_unix: timestamp_secs(),
                                                        scope: "concern".to_string(),
                                                        repo: repo.display().to_string(),
                                                        reason: reason.clone(),
                                                        action: "rewrite_then_push".to_string(),
                                                        backup_branch: Some(backup_branch_for_log),
                                                        result: "ok".to_string(),
                                                        details: Some(format!(
                                                            "paths={:?}",
                                                            rewrite_paths
                                                        )),
                                                    },
                                                );
                                            }
                                            Err(e2) => {
                                                out!(
                                                    "   fail: push after rewrite failed: {}",
                                                    e2
                                                );
                                                append_incident_record(
                                                    policy_path,
                                                    &IncidentRecord {
                                                        ts_unix: timestamp_secs(),
                                                        scope: "concern".to_string(),
                                                        repo: repo.display().to_string(),
                                                        reason: reason.clone(),
                                                        action: "rewrite_then_push".to_string(),
                                                        backup_branch: Some(backup_branch),
                                                        result: "fail".to_string(),
                                                        details: Some(e2.to_string()),
                                                    },
                                                );
                                            }
                                        }
                                    }
                                    Ok(None) => {}
                                    Err(rewrite_err) => {
                                        out!("   fail: rewrite failed: {}", rewrite_err);
                                        append_incident_record(
                                            policy_path,
                                            &IncidentRecord {
                                                ts_unix: timestamp_secs(),
                                                scope: "concern".to_string(),
                                                repo: repo.display().to_string(),
                                                reason: reason.clone(),
                                                action: "rewrite_large_blob".to_string(),
                                                backup_branch: None,
                                                result: "fail".to_string(),
                                                details: Some(rewrite_err.to_string()),
                                            },
                                        );
                                    }
                                }
                            }
                        } else {
                            let branch = current_branch(&repo).unwrap_or_default();
                            let dry_run = run_git_capture_output(
                                &repo,
                                &["push", "--dry-run", "origin", "HEAD"],
                                "push --dry-run",
                            )
                            .unwrap_or_default();
                            let looks_branch_mismatch =
                                dry_run.to_ascii_lowercase().contains("up-to-date");
                            if looks_branch_mismatch
                                && !branch.is_empty()
                                && remote_branch_exists(&repo, &branch)
                                && has_tracking_upstream(&repo)
                            {
                                out!(
                                    "   plan: align upstream to origin/{} (possible branch mismatch)",
                                    branch
                                );
                                match set_upstream_to_branch(&repo, &branch) {
                                    Ok(()) => {
                                        out!("   ok: upstream realigned");
                                        match push_with_retries(
                                            &repo,
                                            push_timeout_secs,
                                            push_retries,
                                            "push-after-upstream-align",
                                        )
                                        .await
                                        {
                                            Ok(()) => {
                                                succeeded_ops += 1;
                                                push_ok = true;
                                                out!("   ok: pushed after upstream align");
                                                append_incident_record(
                                                    policy_path,
                                                    &IncidentRecord {
                                                        ts_unix: timestamp_secs(),
                                                        scope: "concern".to_string(),
                                                        repo: repo.display().to_string(),
                                                        reason: reason.clone(),
                                                        action: "realign_upstream_then_push".to_string(),
                                                        backup_branch: None,
                                                        result: "ok".to_string(),
                                                        details: Some(format!(
                                                            "branch={}",
                                                            branch
                                                        )),
                                                    },
                                                );
                                            }
                                            Err(e2) => {
                                                out!(
                                                    "   fail: push after upstream align failed: {}",
                                                    e2
                                                );
                                                append_incident_record(
                                                    policy_path,
                                                    &IncidentRecord {
                                                        ts_unix: timestamp_secs(),
                                                        scope: "concern".to_string(),
                                                        repo: repo.display().to_string(),
                                                        reason: reason.clone(),
                                                        action: "realign_upstream_then_push".to_string(),
                                                        backup_branch: None,
                                                        result: "fail".to_string(),
                                                        details: Some(e2.to_string()),
                                                    },
                                                );
                                            }
                                        }
                                    }
                                    Err(set_err) => {
                                        out!("   fail: upstream align failed: {}", set_err)
                                    }
                                }
                            }
                        }
                    }
                }
                if !push_ok {
                    append_incident_record(
                        policy_path,
                        &IncidentRecord {
                            ts_unix: timestamp_secs(),
                            scope: "concern".to_string(),
                            repo: repo.display().to_string(),
                            reason: reason.clone(),
                            action: "push_origin_head".to_string(),
                            backup_branch: None,
                            result: "fail".to_string(),
                            details: Some("push did not clear concern".to_string()),
                        },
                    );
                }
                if push_ok {
                    if let Ok(next_after_push) = svc.get_status().await {
                        if next_after_push.ahead > 0 {
                            let branch = current_branch(&repo).unwrap_or_default();
                            if !branch.is_empty() && remote_branch_exists(&repo, &branch) {
                                out!(
                                    "   plan: realign upstream to origin/{} (ahead still > 0 after push)",
                                    branch
                                );
                                match set_upstream_to_branch(&repo, &branch) {
                                    Ok(()) => out!("   ok: upstream realigned"),
                                    Err(e) => out!("   fail: upstream realign failed: {}", e),
                                }
                            }
                        }
                    }
                }
            }
        }

        if apply {
            if let Ok(next) = svc.get_status().await {
                let still_concern = next.ahead > 0
                    || next.behind > 0
                    || !has_origin_remote(&repo)
                    || (has_origin_remote(&repo) && !has_tracking_upstream(&repo));
                if !still_concern {
                    resolved += 1;
                    out!("   resolved: concern cleared");
                    append_incident_record(
                        policy_path,
                        &IncidentRecord {
                            ts_unix: timestamp_secs(),
                            scope: "concern".to_string(),
                            repo: repo.display().to_string(),
                            reason,
                            action: "verify_resolved".to_string(),
                            backup_branch: None,
                            result: "ok".to_string(),
                            details: None,
                        },
                    );
                } else {
                    out!(
                        "   remaining: ahead={} behind={} origin={} upstream={}",
                        next.ahead,
                        next.behind,
                        has_origin_remote(&repo),
                        has_tracking_upstream(&repo)
                    );
                    append_incident_record(
                        policy_path,
                        &IncidentRecord {
                            ts_unix: timestamp_secs(),
                            scope: "concern".to_string(),
                            repo: repo.display().to_string(),
                            reason,
                            action: "verify_resolved".to_string(),
                            backup_branch: None,
                            result: "remaining".to_string(),
                            details: Some(format!(
                                "ahead={} behind={}",
                                next.ahead, next.behind
                            )),
                        },
                    );
                }
            }
        }
    }

    let summary = RepairSummary {
        found: concerns,
        planned: attempted_ops,
        attempted: if apply { attempted_ops } else { 0 },
        succeeded: succeeded_ops,
        resolved_now: if apply { resolved } else { 0 },
        manual_only,
    };
    if json {
        let payload = RepairJson {
            policy: policy_path.display().to_string(),
            scope: "concern".to_string(),
            mode: if apply { "apply".to_string() } else { "dry_run".to_string() },
            found: summary.found,
            planned: summary.planned,
            attempted: summary.attempted,
            succeeded: summary.succeeded,
            resolved_now: summary.resolved_now,
            manual_only: summary.manual_only,
            ledger: incident_ledger_path(policy_path).display().to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("\n✅ concern management summary");
        println!("   concerns_found: {}", summary.found);
        println!("   operations_planned: {}", summary.planned);
        println!("   operations_succeeded: {}", summary.succeeded);
        println!("   manual_only: {}", summary.manual_only);
        if apply {
            println!("   concerns_resolved_now: {}", summary.resolved_now);
        } else {
            println!("   dry_run: true (rerun with --apply to execute)");
        }
        println!("   ledger: {}", incident_ledger_path(policy_path).display());
    }

    Ok(summary)
}

pub(crate) async fn run_repair_warns(
    policy_path: &Path,
    apply: bool,
    only_repo: Option<PathBuf>,
    json: bool,
) -> Result<RepairSummary> {
    let human = !json;
    macro_rules! out {
        ($($arg:tt)*) => {{
            if human {
                println!($($arg)*);
            }
        }};
    }

    let policy = SyncPolicy::load(policy_path)?;
    let roots = policy.watch_root_paths();
    let excluded_dir_names = excluded_dir_names_set(&policy);
    let mut repos = discover_git_repos(&roots, &excluded_dir_names);
    if let Some(target_repo) = only_repo {
        repos.retain(|r| r == &target_repo);
        if repos.is_empty() {
            out!(
                "⚠️ target repo not discovered in policy roots: {}",
                target_repo.display()
            );
            return Ok(RepairSummary::default());
        }
    }

    let mut warns = 0usize;
    let mut attempted = 0usize;
    let mut succeeded = 0usize;

    out!("📜 POLICY: {}", policy_path.display());
    out!(
        "🧹 WARN MODE: {}",
        if apply {
            "APPLY (mutating)"
        } else {
            "DRY-RUN (no changes)"
        }
    );

    for repo in repos {
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
        let has_origin = has_origin_remote(&repo);
        let has_upstream = has_tracking_upstream(&repo);
        let effective_status = dracon_git::types::RepoStatus {
            is_clean: !effective_dirty,
            modified_files: if effective_dirty { status.modified_files } else { 0 },
            ..status.clone()
        };
        if !repo_is_warn(&effective_status, has_origin, has_upstream) {
            continue;
        }
        warns += 1;
        let flags = repo_state_flags(&effective_status, has_origin, has_upstream);
        let reason = flags.join(",");
        out!(
            "\n🟡 {}  state={} modified={} staged={}",
            repo.display(),
            reason,
            effective_status.modified_files,
            effective_status.staged_files
        );
        out!("   plan: run normal sync triage (stage/commit/push)");
        if !apply {
            append_incident_record(
                policy_path,
                &IncidentRecord {
                    ts_unix: timestamp_secs(),
                    scope: "warn".to_string(),
                    repo: repo.display().to_string(),
                    reason,
                    action: "dry_run_sync_triage".to_string(),
                    backup_branch: None,
                    result: "planned".to_string(),
                    details: None,
                },
            );
            continue;
        }

        attempted += 1;
        match tokio::time::timeout(
            Duration::from_secs(policy.repo_sync_timeout_secs),
            crate::sync::sync_repo(&repo, &policy, &excluded_dir_names, 0),
        )
        .await
        {
            Err(_) => {
                out!(
                    "   fail: sync timeout after {}s",
                    policy.repo_sync_timeout_secs
                );
                append_incident_record(
                    policy_path,
                    &IncidentRecord {
                        ts_unix: timestamp_secs(),
                        scope: "warn".to_string(),
                        repo: repo.display().to_string(),
                        reason,
                        action: "sync_triage".to_string(),
                        backup_branch: None,
                        result: "fail".to_string(),
                        details: Some(format!(
                            "timeout={}s",
                            policy.repo_sync_timeout_secs
                        )),
                    },
                );
            }
            Ok(Ok(changed)) => {
                succeeded += 1;
                out!("   ok: triage complete changed={}", changed);
                append_incident_record(
                    policy_path,
                    &IncidentRecord {
                        ts_unix: timestamp_secs(),
                        scope: "warn".to_string(),
                        repo: repo.display().to_string(),
                        reason,
                        action: "sync_triage".to_string(),
                        backup_branch: None,
                        result: "ok".to_string(),
                        details: Some(format!("changed={}", changed)),
                    },
                );
            }
            Ok(Err(e)) => {
                out!("   fail: sync triage failed: {}", e);
                append_incident_record(
                    policy_path,
                    &IncidentRecord {
                        ts_unix: timestamp_secs(),
                        scope: "warn".to_string(),
                        repo: repo.display().to_string(),
                        reason,
                        action: "sync_triage".to_string(),
                        backup_branch: None,
                        result: "fail".to_string(),
                        details: Some(e.to_string()),
                    },
                );
            }
        }
    }

    let summary = RepairSummary {
        found: warns,
        planned: warns,
        attempted,
        succeeded,
        resolved_now: 0,
        manual_only: 0,
    };
    if json {
        let payload = RepairJson {
            policy: policy_path.display().to_string(),
            scope: "warn".to_string(),
            mode: if apply { "apply".to_string() } else { "dry_run".to_string() },
            found: summary.found,
            planned: summary.planned,
            attempted: summary.attempted,
            succeeded: summary.succeeded,
            resolved_now: summary.resolved_now,
            manual_only: summary.manual_only,
            ledger: incident_ledger_path(policy_path).display().to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("\n✅ warn management summary");
        println!("   warns_found: {}", summary.found);
        println!("   operations_planned: {}", summary.planned);
        println!("   operations_attempted: {}", summary.attempted);
        println!("   operations_succeeded: {}", summary.succeeded);
        if !apply {
            println!("   dry_run: true (rerun with --apply to execute)");
        }
        println!("   ledger: {}", incident_ledger_path(policy_path).display());
    }
    Ok(summary)
}

fn create_private_remote(repo: &Path) -> Option<String> {
    let repo_name = repo.file_name()?.to_str()?.to_string();
    let private_remotes_dir = dirs::home_dir()?.join("dracon/private-remotes");
    let bare_repo_path = private_remotes_dir.join(format!("{}.git", repo_name));
    
    if !private_remotes_dir.exists() {
        std::fs::create_dir_all(&private_remotes_dir).ok()?;
    }
    
    let mut final_path = bare_repo_path.clone();
    let mut counter = 1;
    while final_path.exists() {
        final_path = private_remotes_dir.join(format!("{}-{}.git", repo_name, counter));
        counter += 1;
    }
    
    let bare_name = final_path.file_name()?.to_str()?;
    
    let output = std::process::Command::new("git")
        .args(["init", "--bare", bare_name])
        .current_dir(&private_remotes_dir)
        .output()
        .ok()?;
    
    if !output.status.success() {
        std::fs::create_dir_all(&final_path).ok()?;
        let output = std::process::Command::new("git")
            .args(["init", "--bare"])
            .current_dir(&final_path)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
    }
    
    let remote_url = format!("file://{}", final_path.display());
    
    let add_remote_result = std::process::Command::new("git")
        .args(["remote", "add", "origin", &remote_url])
        .current_dir(repo)
        .output();
    
    if add_remote_result.is_err() {
        let _ = std::process::Command::new("git")
            .args(["remote", "set-url", "origin", &remote_url])
            .current_dir(repo)
            .output();
    }
    
    Some(remote_url)
}
