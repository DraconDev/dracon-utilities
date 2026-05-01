use anyhow::Result;
use dracon_git::{
    types::{DiffFile, RepoStatus},
    CommitContext, extract_intent, GitService,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tokio::time::Duration;

fn send_sync_conflict_notification(repo_path: &Path, reason: &str, details: &str) {
    let repo_name = repo_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| repo_path.display().to_string());
    
    let title = "Dracon Sync: Manual Action Required";
    let body = format!(
        "Repository '{}' needs manual resolution.\nReason: {}\nDetails: {}",
        repo_name, reason, details
    );
    
    if let Err(e) = notify_rust::Notification::new()
        .summary(title)
        .body(&body)
        .show() 
    {
        eprintln!("⚠️ failed to send desktop notification: {}", e);
    }
}

use crate::exclude::{
    excluded_dir_names_set,
    has_sync_relevant_dirty_entries,
    is_excluded_dir_name,
};
use crate::git::{
    has_origin_remote, has_tracking_upstream, discover_git_repos, push_with_retries, rewrite_ahead_paths, current_branch,
    remote_branch_exists, set_upstream_to_branch, detect_large_blobs_ahead,
    top_level_dir, repo_diff_entries, run_git_with_timeout, run_git_capture_output,
};
use crate::policy::{
    SyncPolicy,
    DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES, tokio_git_command, timestamp_secs,
};

fn ansi(color: &str, text: &str) -> String {
    let codes = match color {
        "31" => "31",
        "32" => "32",
        "33" => "33",
        "34" => "34",
        "35" => "35",
        "36" => "36",
        "37" => "37",
        "1" => "1",
        _ => "0",
    };
    format!("\x1b[{}m{}\x1b[0m", codes, text)
}

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

#[derive(Debug, Serialize, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
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

    // Extract category/scope from scribe's "Current Focus" line for better commit messages
    let (scribe_category, scribe_scope) = description
        .as_ref()
        .and_then(|d| extract_category_scope_from_focus(d))
        .unwrap_or((String::new(), String::new()));

    CommitContext {
        intent: intent_info.intent,
        track: intent_info.track,
        is_checkpoint,
        files: entries.to_vec(),
        task_progress: intent_info.task_progress,
        refs,
        idle_seconds,
        category: if scribe_category.is_empty() { None } else { Some(scribe_category) },
        scope: if scribe_scope.is_empty() { None } else { Some(scribe_scope) },
        severity: None,
        description,
        semantic_summary: None,
    }
}

fn extract_category_scope_from_focus(content: &str) -> Option<(String, String)> {
    // Extract "Current Focus" line from project-state.md content
    let focus_line = content
        .lines()
        .skip_while(|l| !l.starts_with("## Current Focus"))
        .nth(1)?
        .trim();

    // Handle scribe format: "prefix(category): focus" where prefix might be "updated", "added", etc.
    // e.g., "docs(security): updated session cleanup" or "fix(auth): added JWT validation"
    if let Some(paren_start) = focus_line.find('(') {
        if let Some(paren_end) = focus_line[paren_start..].find(')') {
            let cat = &focus_line[paren_start+1..paren_start+paren_end];
            if !cat.is_empty() && cat.len() <= 20 {
                // Valid category in parentheses - extract focus after the closing paren
                let focus_start = paren_start + paren_end + 1;
                if focus_start < focus_line.len() {
                    let after_cat = focus_line[focus_start..].trim_start_matches([' ', ':', '-']);
                    return Some((cat.to_string(), extract_scope_from_focus(after_cat)));
                }
            }
        }
    }

    // No valid format - derive from entire line
    let focus_lower = focus_line.to_lowercase();

    let category = if focus_lower.contains("fix") || focus_lower.contains("bug") || focus_lower.contains("error") || focus_lower.contains("issue") || focus_lower.contains("patch") {
        "fix".to_string()
    } else if focus_lower.contains("add") || focus_lower.contains("new") || focus_lower.contains("implement") || focus_lower.contains("create") || focus_lower.contains("support for") {
        "feat".to_string()
    } else if focus_lower.contains("remove") || focus_lower.contains("delete") || focus_lower.contains("clean up") || focus_lower.contains("refactor") {
        "refactor".to_string()
    } else if focus_lower.contains("security") || focus_lower.contains("encrypt") || focus_lower.contains("protect") || focus_lower.contains("auth") {
        "security".to_string()
    } else if focus_lower.contains("docs") || focus_lower.contains("documentation") || focus_lower.contains("readme") || focus_lower.contains("comment") {
        "docs".to_string()
    } else if focus_lower.contains("test") || focus_lower.contains("testing") || focus_lower.contains("verify") {
        "test".to_string()
    } else {
        return None;
    };

    Some((category, extract_scope_from_focus(focus_line)))
}

fn extract_scope_from_focus(focus: &str) -> String {
    // Skip common action words at the start
    let action_words = ["updated", "added", "created", "fixed", "implemented",
                        "removed", "deleted", "refactored", "improved", "changed",
                        "enhanced", "refined", "cleaned", "cleaned up"];
    let mut focus_trimmed = focus;
    for action in &action_words {
        if focus_trimmed.to_lowercase().starts_with(action) {
            if let Some(rest) = focus_trimmed[action.len()..].trim_start().strip_prefix('-').or_else(|| Some(focus_trimmed[action.len()..].trim_start())) {
                focus_trimmed = rest;
            }
            break;
        }
    }

    // Take only 1-2 meaningful words for scope
    let scope = focus_trimmed
        .split_whitespace()
        .filter(|w| !w.chars().all(|c| c == '.' || c == ',' || c == ')'))
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', ',', ')'])
        .to_lowercase();

    // Allow 2-word scopes up to ~25 chars (e.g., "comprehensive test" = 20)
    if scope.is_empty() || scope.len() > 25 {
        focus_trimmed.split_whitespace().take(2).collect::<Vec<_>>().join(" ").to_lowercase()
    } else {
        scope
    }
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
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("⚠️ failed to create incident ledger dir: {}", e);
        }
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
    status.ahead > 0 || status.behind > 0 || !has_origin || !has_upstream
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
    let repos = discover_git_repos(&roots, &excluded_dir_names, &policy.exclude_repos, Some(&policy.system_repo));
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

    rows.sort_by_key(|a| a.last_unix);

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

#[allow(clippy::too_many_arguments)]
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
    let repos = if let Some(target_repo) = &only_repo {
        vec![target_repo.clone()]
    } else {
        discover_git_repos(&roots, &excluded_dir_names, &policy.exclude_repos, Some(&policy.system_repo))
    };
    if repos.is_empty() {
        if let Some(target_repo) = &only_repo {
            out!(
                "⚠️ target repo not discovered in policy roots: {}",
                target_repo.display()
            );
        }
        return Ok(RepairSummary::default());
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

        let mut has_origin = has_origin_remote(&repo);
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
            attempted_ops += 1;
            if apply {
                let private_remote = if policy.auto_github_private {
                    out!("   plan: create GitHub private repo as origin");
                    create_github_private_remote(&repo, &policy.auto_github_private_account)
                } else {
                    out!("   plan: create private bare repo as origin");
                    create_private_remote(&repo)
                };
                if let Some(private_remote) = private_remote {
                    succeeded_ops += 1;
                    has_origin = true;
                    has_upstream = true;
                    out!("   ok: created private remote: {}", private_remote);
                    append_incident_record(
                        policy_path,
                        &IncidentRecord {
                            ts_unix: timestamp_secs(),
                            scope: "concern".to_string(),
                            repo: repo.display().to_string(),
                            reason: reason.clone(),
                            action: "create_private_remote".to_string(),
                            backup_branch: None,
                            result: "ok".to_string(),
                            details: Some(format!("created private remote: {}", private_remote)),
                        },
                    );
                } else {
                    manual_only += 1;
                    out!("   fail: could not create private remote");
                    append_incident_record(
                        policy_path,
                        &IncidentRecord {
                            ts_unix: timestamp_secs(),
                            scope: "concern".to_string(),
                            repo: repo.display().to_string(),
                            reason: reason.clone(),
                            action: "create_private_remote".to_string(),
                            backup_branch: None,
                            result: "fail".to_string(),
                            details: Some("failed to create private remote".to_string()),
                        },
                    );
                }
            }
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
                #[allow(unused_assignments)]
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
                            || err_str.contains("does not appear to be a git repository")
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
                                        continue;
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
                                        continue;
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
                                continue;
                            }
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
                        // Don't continue here - let it fall through to large blob detection below
                        // (but without the manual_only marking)

                        let large = detect_large_blobs_ahead(&repo, blob_threshold)
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
                let has_origin = has_origin_remote(&repo);
                let has_upstream = has_tracking_upstream(&repo);
                let still_concern = next.ahead > 0
                    || next.behind > 0
                    || !has_origin
                    || !has_upstream;
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
                        has_origin,
                        has_upstream
                    );
                    // Only notify on true divergence (both ahead AND behind) - that's
                    // the only case where we have no automatic resolution.
                    // If just ahead > 0, we can push. If just behind > 0, we can pull.
                    if next.ahead > 0 && next.behind > 0 {
                        let details = format!("ahead={} behind={}", next.ahead, next.behind);
                        send_sync_conflict_notification(&repo, &reason, &details);
                    }
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
                            details: Some(format!("ahead={} behind={}", next.ahead, next.behind)),
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
    let repos = if let Some(target_repo) = &only_repo {
        vec![target_repo.clone()]
    } else {
        discover_git_repos(&roots, &excluded_dir_names, &policy.exclude_repos, Some(&policy.system_repo))
    };
    if repos.is_empty() {
        if let Some(target_repo) = &only_repo {
            out!(
                "⚠️ target repo not discovered in policy roots: {}",
                target_repo.display()
            );
        }
        return Ok(RepairSummary::default());
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

pub(crate) fn create_github_private_remote(repo: &Path, account: &str) -> Option<String> {
    let base_name = repo.file_name()?.to_str()?.to_string();
    
    let mut repo_name = base_name.clone();
    let mut counter = 1;
    
    loop {
        let output = std::process::Command::new("gh")
            .args(["repo", "create", &repo_name, "--private"])
            .current_dir(repo)
            .output()
            .ok()?;
        
        if output.status.success() {
            let remote_url = format!("git@github.com:{}/{}.git", account, repo_name);
            
            let add_result = std::process::Command::new("git")
                .args(["remote", "add", "origin", &remote_url])
                .current_dir(repo)
                .output();
            
            if let Err(e) = add_result {
                eprintln!(
                    "⚠️ failed to add origin for {}: {}",
                    repo.display(), e
                );
            }
            
            // Push to set upstream and populate the remote
            let push_result = std::process::Command::new("git")
                .args(["push", "-u", "origin", "HEAD"])
                .current_dir(repo)
                .output();
            
            if let Ok(push_output) = push_result {
                if !push_output.status.success() {
                    let stderr = String::from_utf8_lossy(&push_output.stderr);
                    eprintln!(
                        "⚠️ failed to push initial commit for {}: {}",
                        repo.display(), stderr
                    );
                }
            } else {
                eprintln!(
                    "⚠️ failed to push initial commit for {}: could not execute",
                    repo.display()
                );
            }
            
            return Some(remote_url);
        }
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Name already exists") && counter <= 100 {
            repo_name = format!("{}-{}", base_name, counter);
            counter += 1;
            continue;
        }
        
        eprintln!("⚠️ gh repo create failed for {}: {}", repo_name, stderr);
        return None;
    }
}

fn create_private_remote(repo: &Path) -> Option<String> {
    // NEVER overwrite an existing origin. Only create a local bare repo
    // for repos that genuinely have no remote configured.
    if has_origin_remote(repo) {
        eprintln!(
            "⚠️ refusing to create private remote for {} — origin already exists",
            repo.display()
        );
        return None;
    }

    let repo_name = repo.file_name()?.to_str()?.to_string();
    let private_remotes_dir = dirs::home_dir()?.join("dracon/private-remotes");
    
    if !private_remotes_dir.exists() {
        std::fs::create_dir_all(&private_remotes_dir).ok()?;
    }
    
    let bare_repo_path = private_remotes_dir.join(format!("{}.git", repo_name));
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
    
    let add_result = std::process::Command::new("git")
        .args(["remote", "add", "origin", &remote_url])
        .current_dir(repo)
        .output();
    
    if let Err(e) = add_result {
        eprintln!(
            "⚠️ failed to add origin for {}: {}",
            repo.display(), e
        );
    }
    
    Some(remote_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dracon_git::types::{DiffFile, FileStatus, RepoStatus};

    fn make_status(is_clean: bool, ahead: usize, behind: usize) -> RepoStatus {
        RepoStatus {
            branch: String::new(),
            is_clean,
            ahead,
            behind,
            modified_files: 0,
            staged_files: 0,
            last_commit_hash: None,
            last_commit_msg: None,
        }
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_shorter() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_longer() {
        assert_eq!(truncate("hello world", 5), "hell…");
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_truncate_unicode_truncation() {
        let s = "hello 世界 test";
        let result = truncate(s, 10);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_repo_state_flags_ok() {
        let status = make_status(true, 0, 0);
        let flags = repo_state_flags(&status, true, true);
        assert!(flags.contains(&"OK".to_string()));
    }

    #[test]
    fn test_repo_state_flags_dirty() {
        let mut status = make_status(false, 0, 0);
        status.modified_files = 2;
        let flags = repo_state_flags(&status, true, true);
        assert!(flags.contains(&"DIRTY".to_string()));
    }

    #[test]
    fn test_repo_state_flags_ahead() {
        let status = make_status(true, 3, 0);
        let flags = repo_state_flags(&status, true, true);
        assert!(flags.iter().any(|f| f.starts_with("AHEAD:")));
    }

    #[test]
    fn test_repo_state_flags_behind() {
        let status = make_status(true, 0, 2);
        let flags = repo_state_flags(&status, true, true);
        assert!(flags.iter().any(|f| f.starts_with("BEHIND:")));
    }

    #[test]
    fn test_repo_state_flags_no_origin() {
        let status = make_status(true, 0, 0);
        let flags = repo_state_flags(&status, false, false);
        assert!(flags.contains(&"NO_ORIGIN".to_string()));
    }

    #[test]
    fn test_repo_state_flags_no_upstream() {
        let status = make_status(true, 0, 0);
        let flags = repo_state_flags(&status, true, false);
        assert!(flags.contains(&"NO_UPSTREAM".to_string()));
    }

    #[test]
    fn test_repo_state_flags_stuck_push() {
        let status = make_status(false, 5, 0);
        let flags = repo_state_flags(&status, true, true);
        assert!(flags.contains(&"STUCK_PUSH".to_string()));
    }

    #[test]
    fn test_repo_state_flags_stuck_pull() {
        let status = make_status(false, 0, 3);
        let flags = repo_state_flags(&status, true, true);
        assert!(flags.contains(&"STUCK_PULL".to_string()));
    }

    #[test]
    fn test_repo_state_flags_multiple() {
        let status = make_status(false, 3, 2);
        let flags = repo_state_flags(&status, true, true);
        assert!(flags.contains(&"DIRTY".to_string()));
        assert!(flags.iter().any(|f| f.starts_with("AHEAD:")));
        assert!(flags.iter().any(|f| f.starts_with("BEHIND:")));
    }

    #[test]
    fn test_repo_is_concern_no_origin() {
        let status = make_status(true, 0, 0);
        assert!(repo_is_concern(&status, false, false));
    }

    #[test]
    fn test_repo_is_concern_no_upstream() {
        let status = make_status(true, 0, 0);
        assert!(repo_is_concern(&status, true, false));
    }

    #[test]
    fn test_repo_is_concern_ahead() {
        let status = make_status(false, 5, 0);
        assert!(repo_is_concern(&status, true, true));
    }

    #[test]
    fn test_repo_is_concern_behind() {
        let status = make_status(false, 0, 3);
        assert!(repo_is_concern(&status, true, true));
    }

    #[test]
    fn test_repo_is_concern_clean_healthy() {
        let status = make_status(true, 0, 0);
        assert!(!repo_is_concern(&status, true, true));
    }

    #[test]
    fn test_repo_is_warn_dirty() {
        let status = make_status(false, 0, 0);
        assert!(repo_is_warn(&status, true, true));
    }

    #[test]
    fn test_repo_is_warn_not_concern() {
        let status = make_status(false, 0, 0);
        assert!(!repo_is_warn(&status, false, false));
    }

    #[test]
    fn test_repo_hint_no_origin() {
        let hint = repo_hint(&["NO_ORIGIN".into()], false, false);
        assert_eq!(hint, "set origin remote");
    }

    #[test]
    fn test_repo_hint_no_upstream() {
        let hint = repo_hint(&["NO_UPSTREAM".into()], false, false);
        assert_eq!(hint, "run repair-concerns --apply (set upstream)");
    }

    #[test]
    fn test_repo_hint_ahead() {
        let hint = repo_hint(&["AHEAD:3".into()], false, false);
        assert_eq!(hint, "run repair-concerns --apply (push or rewrite)");
    }

    #[test]
    fn test_repo_hint_behind() {
        let hint = repo_hint(&["BEHIND:2".into()], false, false);
        assert_eq!(hint, "run repair-concerns --apply (pull/rebase)");
    }

    #[test]
    fn test_repo_hint_healthy() {
        let hint = repo_hint(&["OK".into()], false, false);
        assert_eq!(hint, "healthy");
    }

    #[test]
    fn test_repo_hint_warn() {
        let hint = repo_hint(&["DIRTY".into()], true, false);
        assert_eq!(hint, "run repair-warns --apply");
    }

    #[test]
    fn test_repo_hint_concern() {
        let hint = repo_hint(&["DIRTY".into()], false, true);
        assert_eq!(hint, "run repair-concerns --apply");
    }

    #[test]
    fn test_push_large_blob_threshold_bytes() {
        let policy = SyncPolicy {
            max_stage_file_bytes: 200 * 1024 * 1024,
            max_push_blob_bytes: 50 * 1024 * 1024,
            ..test_sync_policy()
        };
        let threshold = push_large_blob_threshold_bytes(&policy);
        assert_eq!(threshold, 50 * 1024 * 1024);
    }

    #[test]
    fn test_push_large_blob_threshold_caps_at_git_limit() {
        let policy = SyncPolicy {
            max_stage_file_bytes: 200 * 1024 * 1024,
            max_push_blob_bytes: 200 * 1024 * 1024,
            ..test_sync_policy()
        };
        let threshold = push_large_blob_threshold_bytes(&policy);
        assert_eq!(threshold, DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES);
    }

    #[test]
    fn test_detect_report_signals_active_board() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("plan/ACTIVE_BOARD.md"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::ActiveBoardChanged));
    }

    #[test]
    fn test_detect_report_signals_index() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("docs/index.md"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::IndexChanged));
    }

    #[test]
    fn test_detect_report_signals_blueprint_added() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("docs/blueprint-foo.md"),
                status: FileStatus::Added,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::BlueprintCreated));
    }

    #[test]
    fn test_detect_report_signals_blueprint_modified() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("docs/blueprint-bar.md"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::BlueprintModified));
    }

    #[test]
    fn test_timestamp_secs_returns_reasonable_value() {
        let ts = timestamp_secs();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(ts > 0);
        assert!(ts <= now + 1);
    }

    static LEDGER_ENV_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct VarGuard {
        var: String,
        original: Option<String>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }
    impl VarGuard {
        fn set_temp(var: &str, value: &str) -> Self {
            let lock = LEDGER_ENV_GUARD.lock().unwrap();
            let original = std::env::var(var).ok();
            if value.is_empty() {
                std::env::remove_var(var);
            } else {
                std::env::set_var(var, value);
            }
            Self { var: var.to_string(), original, _lock: lock }
        }
    }
    impl Drop for VarGuard {
        fn drop(&mut self) {
            if let Some(orig) = self.original.take() {
                std::env::set_var(&self.var, orig);
            } else {
                std::env::remove_var(&self.var);
            }
        }
    }

    #[test]
    fn test_incident_ledger_path_default() {
        let _guard = VarGuard::set_temp("DRACON_SYNC_LEDGER", "");
        let path = incident_ledger_path(std::path::Path::new("/fake/policy.toml"));
        assert!(path.to_string_lossy().contains("dracon-sync-incidents.jsonl"));
    }

    #[test]
    fn test_incident_ledger_path_custom_env() {
        let _guard = VarGuard::set_temp("DRACON_SYNC_LEDGER", "/custom/path/ledger.jsonl");
        let path = incident_ledger_path(std::path::Path::new("/fake/policy.toml"));
        let result = path.to_string_lossy();
        assert_eq!(result, "/custom/path/ledger.jsonl");
    }

    fn test_sync_policy() -> SyncPolicy {
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

    #[test]
    fn test_truncate_unicode_emoji() {
        let result = truncate("hello 👋 world", 10);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_report_signal_active_board_changed_exact_path() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("plan/ACTIVE_BOARD.md"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::ActiveBoardChanged));
    }

    #[test]
    fn test_report_signal_index_changed_nested() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("docs/plan/index.md"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::IndexChanged));
    }

    #[test]
    fn test_report_signal_blueprint_created() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("docs/blueprint-foo.md"),
                status: FileStatus::Added,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::BlueprintCreated));
    }

    #[test]
    fn test_report_signal_multiple() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("plan/ACTIVE_BOARD.md"),
                status: FileStatus::Modified,
            },
            DiffFile {
                path: std::path::PathBuf::from("docs/blueprint-bar.md"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::ActiveBoardChanged));
        assert!(signals.contains(&ReportSignal::BlueprintModified));
    }

    #[test]
    fn test_report_signal_empty() {
        let files: Vec<DiffFile> = vec![];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_report_signal_no_match() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("src/main.rs"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_report_signal_blueprint_added() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("docs/blueprint-new.md"),
                status: FileStatus::Added,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::BlueprintCreated));
    }

    #[test]
    fn test_report_signal_blueprint_modified_other_dir() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("project/docs/blueprint-foo.md"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::BlueprintModified));
    }

    #[test]
    fn test_report_signal_index_changed_exact() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("plan/index.md"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::IndexChanged));
    }

    #[test]
    fn test_report_signal_all_signals_together() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("plan/ACTIVE_BOARD.md"),
                status: FileStatus::Modified,
            },
            DiffFile {
                path: std::path::PathBuf::from("plan/index.md"),
                status: FileStatus::Modified,
            },
            DiffFile {
                path: std::path::PathBuf::from("docs/blueprint-new.md"),
                status: FileStatus::Added,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::ActiveBoardChanged));
        assert!(signals.contains(&ReportSignal::IndexChanged));
        assert!(signals.contains(&ReportSignal::BlueprintCreated));
        assert_eq!(signals.len(), 3);
    }

    #[test]
    fn test_repair_summary_default() {
        let summary = RepairSummary::default();
        assert_eq!(summary.found, 0);
        assert_eq!(summary.planned, 0);
        assert_eq!(summary.attempted, 0);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.resolved_now, 0);
        assert_eq!(summary.manual_only, 0);
    }

    #[test]
    fn test_repair_summary_debug() {
        let summary = RepairSummary {
            found: 1,
            planned: 2,
            attempted: 3,
            succeeded: 4,
            resolved_now: 5,
            manual_only: 6,
        };
        let debug = format!("{:?}", summary);
        assert!(debug.contains("found"));
    }

    #[test]
    fn test_report_signal_debug() {
        let signal = ReportSignal::ActiveBoardChanged;
        assert_eq!(format!("{:?}", signal), "ActiveBoardChanged");
    }

    #[test]
    fn test_report_signal_clone() {
        let signal1 = ReportSignal::BlueprintCreated;
        let signal2 = signal1.clone();
        assert_eq!(signal1, signal2);
    }

    #[test]
    fn test_report_signal_partial_eq() {
        assert_eq!(ReportSignal::ActiveBoardChanged, ReportSignal::ActiveBoardChanged);
        assert_ne!(ReportSignal::ActiveBoardChanged, ReportSignal::IndexChanged);
    }

    #[test]
    fn test_ansi_colors() {
        assert_eq!(ansi("31", "error"), "\x1b[31merror\x1b[0m");
        assert_eq!(ansi("32", "ok"), "\x1b[32mok\x1b[0m");
        assert_eq!(ansi("1", "bold"), "\x1b[1mbold\x1b[0m");
        assert_eq!(ansi("unknown", "default"), "\x1b[0mdefault\x1b[0m");
    }

    #[test]
    fn test_repo_filter_variants() {
        assert_eq!(format!("{:?}", RepoFilter::All), "All");
        assert_eq!(format!("{:?}", RepoFilter::Concern), "Concern");
        assert_eq!(format!("{:?}", RepoFilter::Warn), "Warn");
    }

    #[test]
    fn test_concern_repair_filter_variants() {
        assert_eq!(format!("{:?}", ConcernRepairFilter::All), "All");
        assert_eq!(format!("{:?}", ConcernRepairFilter::StuckPush), "StuckPush");
        assert_eq!(format!("{:?}", ConcernRepairFilter::StuckPull), "StuckPull");
    }

    #[test]
    fn test_incident_record_serialization() {
        let record = IncidentRecord {
            ts_unix: 1700000000,
            scope: "test".to_string(),
            repo: "/test/repo".to_string(),
            reason: "test reason".to_string(),
            action: "test action".to_string(),
            backup_branch: Some("backup".to_string()),
            result: "success".to_string(),
            details: Some("details".to_string()),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("1700000000"));
        assert!(json.contains("test reason"));
    }

    #[test]
    fn test_repo_report_row_structure() {
        let row = RepoReportRow {
            repo: "/test/repo".to_string(),
            state_flags: vec!["OK".to_string()],
            branch: "main".to_string(),
            modified: 0,
            staged: 0,
            ahead: 0,
            behind: 0,
            last_hash: "abc123".to_string(),
            last_author: "test".to_string(),
            last_when: "2024-01-01".to_string(),
            last_msg: "test commit".to_string(),
            last_unix: 1700000000,
            concern: false,
            warn: false,
            hint: "healthy".to_string(),
        };
        assert_eq!(row.repo, "/test/repo");
        assert_eq!(row.branch, "main");
        assert!(!row.concern);
    }

    #[test]
    fn test_repo_report_json_structure() {
        let json = RepoReportJson {
            policy: "default".to_string(),
            filter: "all".to_string(),
            repos: 1,
            ok: 1,
            warn: 0,
            concern: 0,
            failures: 0,
            rows: vec![],
        };
        assert_eq!(json.repos, 1);
        assert_eq!(json.ok, 1);
    }

    #[test]
    fn test_status_json_structure() {
        let status = StatusJson {
            policy: "default".to_string(),
            roots: vec!["~/code".to_string()],
            repos_discovered: 5,
            pulse_interval_secs: 30,
            inactivity_push_delay_secs: 300,
            freeze: "none".to_string(),
            auto_commit: true,
            auto_pull: true,
            auto_push: true,
            auto_bump_versions: true,
            auto_repair_concerns: true,
            auto_repair_warns: true,
            auto_rewrite_large_blobs: true,
            max_stage_file_bytes: 100 * 1024 * 1024,
            push_blob_threshold_bytes: 100 * 1024 * 1024,
            exclude_dirs: vec![],
            exclude_file_patterns: vec![],
            pull_op_timeout_secs: 30,
            push_op_timeout_secs: 300,
            repo_sync_timeout_secs: 420,
            push_retries: 3,
            repair_cooldown_secs: 60,
            incident_ledger_max_lines: 10000,
            incident_ledger_max_age_days: 30,
            system_repo: String::new(),
            backup_policy: String::new(),
            backup_dir: String::new(),
            extra_remotes: 0,
        };
        assert_eq!(status.repos_discovered, 5);
        assert!(status.auto_commit);
    }

    #[test]
    fn test_report_signal_blueprint_modified() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("docs/blueprint-foo.md"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::BlueprintModified));
    }

    #[test]
    fn test_report_signal_blueprint_modified_plan_dir() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("plan/blueprint-bar.md"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::BlueprintModified));
    }

    #[test]
    fn test_push_large_blob_threshold_min_limit() {
        let policy = SyncPolicy {
            max_stage_file_bytes: 10 * 1024 * 1024,
            max_push_blob_bytes: 5 * 1024 * 1024,
            ..test_sync_policy()
        };
        let threshold = push_large_blob_threshold_bytes(&policy);
        assert_eq!(threshold, 5 * 1024 * 1024);
    }

    #[test]
    fn test_truncate_three_chars() {
        let result = truncate("hello", 3);
        assert_eq!(result, "he…");
    }

    #[test]
    fn test_truncate_exact_length_no_ellipsis() {
        let result = truncate("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_empty_string() {
        let result = truncate("", 5);
        assert_eq!(result, "");
    }

    #[test]
    fn test_push_large_blob_threshold_bytes_custom() {
        let policy = SyncPolicy {
            max_push_blob_bytes: 50 * 1024 * 1024,
            ..test_sync_policy()
        };
        let threshold = push_large_blob_threshold_bytes(&policy);
        assert_eq!(threshold, 50 * 1024 * 1024);
    }

    #[test]
    fn test_push_large_blob_threshold_bytes_uses_min_of_all() {
        let policy = SyncPolicy {
            max_stage_file_bytes: 10 * 1024 * 1024,
            max_push_blob_bytes: 50 * 1024 * 1024,
            ..test_sync_policy()
        };
        let threshold = push_large_blob_threshold_bytes(&policy);
        assert_eq!(threshold, 10 * 1024 * 1024, "should use smaller of stage and push limit");
    }

    #[test]
    fn test_detect_report_signals_blueprint_deleted() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("docs/blueprint-foo.md"),
                status: FileStatus::Deleted,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::BlueprintModified));
    }

    #[test]
    fn test_detect_report_signals_index_nested_dir() {
        let files = vec![
            DiffFile {
                path: std::path::PathBuf::from("project/plan/index.md"),
                status: FileStatus::Modified,
            }
        ];
        let signals = detect_report_signals(std::path::Path::new("/fake"), &files);
        assert!(signals.contains(&ReportSignal::IndexChanged));
    }

    #[test]
    fn test_extract_category_scope_from_focus_with_parens_format() {
        let content = r#"# Project State

## Current Focus
docs(security): updated session cleanup
"#;
        let result = extract_category_scope_from_focus(content);
        assert!(result.is_some());
        let (cat, scope) = result.unwrap();
        assert_eq!(cat, "security");
        assert!(scope.contains("session") || scope.contains("cleanup"));
    }

    #[test]
    fn test_extract_category_scope_from_focus_fix_derivation() {
        let content = r#"# Project State

## Current Focus
fixed auth bug
"#;
        let result = extract_category_scope_from_focus(content);
        assert!(result.is_some());
        let (cat, _scope) = result.unwrap();
        assert_eq!(cat, "fix");
    }

    #[test]
    fn test_extract_category_scope_from_focus_add_derivation() {
        let content = r#"# Project State

## Current Focus
added JWT validation
"#;
        let result = extract_category_scope_from_focus(content);
        assert!(result.is_some());
        let (cat, _scope) = result.unwrap();
        assert_eq!(cat, "feat");
    }

    #[test]
    fn test_extract_category_scope_from_focus_no_valid_category_format() {
        let content = r#"# Project State

## Current Focus
some arbitrary text without clear intent
"#;
        let result = extract_category_scope_from_focus(content);
        assert!(result.is_some());
        let (cat, scope) = result.unwrap();
        assert_eq!(scope.trim(), "some arbitrary text without clear intent");
    }

    #[test]
    fn test_extract_category_scope_from_focus_no_current_focus_section() {
        let content = r#"# Project State

## Completed
- did stuff
"#;
        let result = extract_category_scope_from_focus(content);
        assert!(result.is_none(), "should return None when no Current Focus section");
    }

    #[test]
    fn test_extract_scope_from_focus_action_word_stripping() {
        assert_eq!(extract_scope_from_focus("updated auth flow"), "auth flow");
        assert_eq!(extract_scope_from_focus("added JWT support"), "JWT support");
        assert_eq!(extract_scope_from_focus("fixed critical bug"), "critical bug");
    }

    #[test]
    fn test_extract_scope_from_focus_takes_two_words() {
        let scope = extract_scope_from_focus("implemented new user authentication system");
        let words: Vec<_> = scope.split_whitespace().collect();
        assert!(words.len() <= 3, "scope should be 1-2 meaningful words, got: {}", scope);
    }

    #[test]
    fn test_extract_scope_from_focus_handles_punctuation() {
        let scope = extract_scope_from_focus("cleaned up, refactored.");
        assert!(!scope.ends_with(',') && !scope.ends_with('.'));
    }
}
