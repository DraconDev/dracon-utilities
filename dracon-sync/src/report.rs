use anyhow::Result;
use dracon_git::GitService;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tokio::time::Duration;

pub(crate) fn send_sync_conflict_notification(repo_path: &Path, reason: &str, details: &str) {
    let repo_name = repo_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| repo_path.display().to_string());

    let title = "Dracon Sync: Manual Action Required";
    let body = format!(
        "Repository '{}' needs manual resolution.\nReason: {}\nDetails: {}",
        repo_name, reason, details
    );

    // Spawn in background to avoid blocking the daemon loop
    tokio::spawn(async move {
        if let Err(e) = notify_rust::Notification::new()
            .summary(title)
            .body(&body)
            .show()
        {
            eprintln!("⚠️ failed to send desktop notification: {}", e);
        }
    });
}

use crate::exclude::{
    excluded_dir_names_set, has_sync_relevant_dirty_entries, is_excluded_dir_name,
};
use crate::git::{
    current_branch, detect_large_blobs_ahead, discover_git_repos, has_origin_remote,
    has_tracking_upstream, push_with_retries, remote_branch_exists, repo_diff_entries,
    rewrite_ahead_paths, run_git_capture_output, run_git_with_timeout, set_upstream_to_branch,
    top_level_dir,
};
use crate::git::multi_remote::push_mirror_remotes;
use crate::policy::{
    timestamp_secs, tokio_git_command, SyncPolicy, DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES,
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

fn shorten_when(s: &str) -> String {
    let s = s.trim();

    // Parse "N minutes ago" and convert to hours+minutes if >= 60
    if let Some(rest) = s.strip_suffix(" minutes ago") {
        if let Ok(mins) = rest.parse::<u64>() {
            if mins >= 60 {
                let h = mins / 60;
                let m = mins % 60;
                if m == 0 {
                    return format!("{}h", h);
                }
                return format!("{}h {}m", h, m);
            }
            return format!("{}m", mins);
        }
    }
    if let Some(rest) = s.strip_suffix(" minute ago") {
        if let Ok(mins) = rest.parse::<u64>() {
            return format!("{}m", mins);
        }
    }

    // Convert seconds to minutes if >= 60
    if let Some(rest) = s.strip_suffix(" seconds ago") {
        if let Ok(secs) = rest.parse::<u64>() {
            if secs >= 60 {
                let m = secs / 60;
                let s_remainder = secs % 60;
                if s_remainder == 0 {
                    return format!("{}m", m);
                }
                return format!("{}m {}s", m, s_remainder);
            }
            return format!("{}s", secs);
        }
    }
    if let Some(rest) = s.strip_suffix(" second ago") {
        if let Ok(secs) = rest.parse::<u64>() {
            return format!("{}s", secs);
        }
    }

    // Convert hours to days when >= 24
    if let Some(rest) = s.strip_suffix(" hours ago") {
        if let Ok(hrs) = rest.parse::<u64>() {
            if hrs >= 24 {
                let d = hrs / 24;
                let h = hrs % 24;
                if h == 0 {
                    return format!("{}d", d);
                }
                return format!("{}d {}h", d, h);
            }
            return format!("{}h", hrs);
        }
    }
    if let Some(rest) = s.strip_suffix(" hour ago") {
        if let Ok(hrs) = rest.parse::<u64>() {
            return format!("{}h", hrs);
        }
    }

    // Convert days to weeks when >= 7
    if let Some(rest) = s.strip_suffix(" days ago") {
        if let Ok(days) = rest.parse::<u64>() {
            if days >= 7 {
                let w = days / 7;
                let d = days % 7;
                if d == 0 {
                    return format!("{}w", w);
                }
                return format!("{}w {}d", w, d);
            }
            return format!("{}d", days);
        }
    }
    if let Some(rest) = s.strip_suffix(" day ago") {
        if let Ok(days) = rest.parse::<u64>() {
            return format!("{}d", days);
        }
    }

    // Convert months to years when >= 12
    if let Some(rest) = s.strip_suffix(" months ago") {
        if let Ok(months) = rest.parse::<u64>() {
            if months >= 12 {
                let y = months / 12;
                let mo = months % 12;
                if mo == 0 {
                    return format!("{}y", y);
                }
                return format!("{}y {}mo", y, mo);
            }
            return format!("{}mo", months);
        }
    }
    if let Some(rest) = s.strip_suffix(" month ago") {
        if let Ok(months) = rest.parse::<u64>() {
            return format!("{}mo", months);
        }
    }

    // Weeks and years stay as-is (w, y)
    s.replace(" weeks ago", "w")
        .replace(" week ago", "w")
        .replace(" years ago", "y")
        .replace(" year ago", "y")
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
    untracked: usize,
    ahead: usize,
    behind: usize,
    last_hash: String,
    last_author: String,
    last_when: String,
    last_msg: String,
    last_unix: i64,
    last_push: String,
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
pub(crate) struct RemoteStatus {
    pub(crate) name: String,
    pub(crate) auth_type: String,
    pub(crate) auto_create: bool,
    pub(crate) priority: u32,
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
    pub(crate) remotes: usize,
    pub(crate) remote_configs: Vec<RemoteStatus>,
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

impl IncidentRecord {
    pub(crate) fn new(
        ts_unix: u64,
        scope: impl Into<String>,
        repo: impl Into<String>,
        reason: impl Into<String>,
        action: impl Into<String>,
        backup_branch: Option<String>,
        result: impl Into<String>,
        details: Option<String>,
    ) -> Self {
        Self {
            ts_unix,
            scope: scope.into(),
            repo: repo.into(),
            reason: reason.into(),
            action: action.into(),
            backup_branch,
            result: result.into(),
            details,
        }
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
        return home
            .join(".local")
            .join("state")
            .join("dracon")
            .join("dracon-sync-incidents.jsonl");
    }

    PathBuf::from("/tmp/dracon-sync-incidents.jsonl")
}

/// Enforce incident ledger retention at any time.
/// Removes entries older than max_age_days and truncates to max_lines.
/// Returns the number of pruned entries (or 0 if nothing was removed).
pub(crate) fn enforce_retention(path: &Path, policy: &SyncPolicy) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let meta = std::fs::metadata(path)?;
    if meta.len() > 100 * 1024 * 1024 {
        eprintln!(
            "⚠️ incident ledger is {}MB (>100MB), truncating to last {} lines",
            meta.len() / (1024 * 1024),
            policy.incident_ledger_max_lines,
        );
        let content = std::fs::read_to_string(path)?;
        let lines: Vec<&str> = content
            .lines()
            .rev()
            .take(policy.incident_ledger_max_lines)
            .collect();
        let out = lines.iter().rev().copied().collect::<Vec<_>>().join("\n") + "\n";
        std::fs::write(path, &out)?;
        return Ok(lines.len());
    }
    let content = std::fs::read_to_string(path)?;
    let original_count = content.lines().count();
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
    let out = kept.join("\n") + "\n";
    std::fs::write(path, &out)?;

    let removed = original_count.saturating_sub(kept.len());
    Ok(removed)
}

pub(crate) fn append_incident_record(policy_path: &Path, record: &IncidentRecord) {
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
            }
        }
        Err(e) => eprintln!("⚠️ incident open failed ({}): {}", path.display(), e),
    }
    // ── lazy retention: only check when file has likely grown past max ──
    if path.exists() {
        if let Ok(metadata) = std::fs::metadata(&path) {
            // rough estimate: ~200 bytes per JSON line
            let approx_lines = metadata.len() as usize / 200;
            let policy = SyncPolicy::load(policy_path).ok();
            if let Some(ref p) = policy {
                if approx_lines >= p.incident_ledger_max_lines {
                    if let Err(e) = enforce_retention(&path, p).map(|_| ()) {
                        eprintln!("⚠️ incident retention failed ({}): {}", path.display(), e);
                    }
                }
            }
        }
    }
}

/// Enforce incident ledger retention at daemon startup.
/// Delegates to the shared [`enforce_retention`] function.
pub(crate) fn enforce_retention_at_startup(policy_path: &Path, policy: &SyncPolicy) -> Result<()> {
    let path = incident_ledger_path(policy_path);
    let removed = enforce_retention(&path, policy)?;
    if removed > 0 {
        eprintln!(
            "🧹 startup: pruned {} stale incident entries (remaining after reload)",
            removed,
        );
    }
    Ok(())
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

pub(crate) fn repo_is_concern(
    status: &dracon_git::types::RepoStatus,
    has_origin: bool,
    has_upstream: bool,
) -> bool {
    status.ahead > 0 || status.behind > 0 || !has_origin || !has_upstream
}

#[cfg(test)]
pub(crate) fn repo_is_warn(
    status: &dracon_git::types::RepoStatus,
    has_origin: bool,
    has_upstream: bool,
) -> bool {
    // WARN: has TRACKED modifications or staged changes, but not a concern.
    // Untracked files (build artifacts) do NOT count as warn.
    !repo_is_concern(status, has_origin, has_upstream)
        && (status.modified_files > 0 || status.staged_files > 0)
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
        return "run repair-concerns --apply (pull/merge)".to_string();
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

pub(crate) async fn run_repos_report(
    policy_path: &Path,
    filter: RepoFilter,
    json: bool,
    sort: &str,
    filter_name: Option<&str>,
    full_path: bool,
) -> Result<()> {
    let policy = SyncPolicy::load(policy_path)?;
    let roots = policy.watch_root_paths();
    let excluded_dir_names = excluded_dir_names_set(&policy);
    let repos = discover_git_repos(
        &roots,
        &excluded_dir_names,
        &policy.exclude_repos,
        Some(&policy.system_repo),
    );
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
            modified_files: status.modified_files,
            staged_files: status.staged_files,
            ..status.clone()
        };

        let has_origin = has_origin_remote(&repo);
        let has_upstream = has_tracking_upstream(&repo);

        // Classification: a repo is WARN if it has TRACKED modifications or
        // staged changes. Untracked files (e.g., target/, node_modules/) are
        // NOT counted — they are build artifacts that shouldn't trigger
        // WARN. A repo with only untracked build artifacts is OK.
        let real_is_dirty = status.modified_files > 0 || status.staged_files > 0;
        let concern = repo_is_concern(&effective_status, has_origin, has_upstream);
        let warn = !concern && real_is_dirty;

        // Flags still use effective_status for ahead/behind/origin detection
        let flags = repo_state_flags(&effective_status, has_origin, has_upstream);
        let hint = repo_hint(&flags, warn, concern);

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
        // Get last push time from reflog — scan all remote branches and pick the most recent
        let last_push = {
            use std::process::Command;
            let repo_str = repo.to_str().unwrap_or("").to_string();
            let current_branch = effective_status.branch.clone();
            // Collect candidate refs: origin/<current>, plus all origin/* branches
            let mut candidates: Vec<String> = vec![format!("origin/{}", current_branch)];
            // List all origin/* branches
            if let Ok(br_out) = Command::new("git")
                .args(["-C", &repo_str, "for-each-ref", "--format=%(refname:short)", "refs/remotes/origin/"])
                .output()
            {
                if let Ok(s) = String::from_utf8(br_out.stdout) {
                    for line in s.lines() {
                        let t = line.trim().to_string();
                        if !t.is_empty() && !candidates.contains(&t) {
                            candidates.push(t);
                        }
                    }
                }
            }
            let mut best: Option<String> = None;
            let mut best_unix: i64 = 0;
            for cand in &candidates {
                let out = Command::new("git")
                    .args([
                        "-C", &repo_str,
                        "reflog", "show", cand,
                        "--format=%ct %cr", "-1",
                    ])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok());
                if let Some(s) = out {
                    if let Some(line) = s.lines().next() {
                        let mut parts = line.splitn(2, ' ');
                        if let (Some(ts), Some(rel)) = (parts.next(), parts.next()) {
                            if let Ok(u) = ts.parse::<i64>() {
                                if u > best_unix {
                                    best_unix = u;
                                    best = Some(rel.trim().to_string());
                                }
                            }
                        }
                    }
                }
            }
            best.filter(|s| !s.is_empty()).unwrap_or_else(|| "-".to_string())
        };

        rows.push(RepoReportRow {
            repo: repo.display().to_string(),
            state_flags: flags,
            branch: effective_status.branch,
            modified: effective_status.modified_files,
            staged: effective_status.staged_files,
            untracked: effective_status.untracked_files,
            ahead: effective_status.ahead,
            behind: effective_status.behind,
            last_hash,
            last_author,
            last_when,
            last_msg,
            last_unix,
            last_push,
            concern,
            warn,
            hint,
        });
    }

    match sort {
        "name" => rows.sort_by(|a, b| a.repo.cmp(&b.repo)),
        "modified" => rows.sort_by_key(|b| std::cmp::Reverse(b.modified)),
        "ahead" => rows.sort_by_key(|b| std::cmp::Reverse(b.ahead)),
        "behind" => rows.sort_by_key(|b| std::cmp::Reverse(b.behind)),
        _ => rows.sort_by_key(|a| std::cmp::Reverse(a.last_unix)),
    }

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

    if let Some(pattern) = filter_name {
        let pat = pattern.to_lowercase();
        rows.retain(|r| {
            let name = std::path::Path::new(&r.repo)
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            name.contains(&pat)
        });
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

    println!("📜 Policy: {}", policy_path.display());
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
    let sort_label = match sort {
        "name" => "name",
        "modified" => "modified files",
        "ahead" => "ahead count",
        "behind" => "behind count",
        _ => "last modified (newest first)",
    };
    println!("🕒 SORT: {sort_label}");
    println!();

    use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, Table};

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec![
        Cell::new("#"),
        Cell::new("STATUS"),
        Cell::new("REPO"),
        Cell::new("BRANCH"),
        Cell::new("MODIFIED"),
        Cell::new("STAGED"),
        Cell::new("UNTRACKED"),
        Cell::new("AHEAD"),
        Cell::new("BEHIND"),
        Cell::new("LAST COMMIT"),
        Cell::new("LAST PUSH"),
        Cell::new("AUTHOR"),
    ]);

    for (idx, row) in rows.iter().enumerate() {
        let status_text = if row.concern {
            "CONCERN".to_string()
        } else if row.warn {
            "WARN".to_string()
        } else {
            "OK".to_string()
        };
        let status_color = if row.concern {
            Color::Red
        } else if row.warn {
            Color::Yellow
        } else {
            Color::Green
        };

        let repo_name = if full_path {
            row.repo.clone()
        } else {
            std::path::Path::new(&row.repo)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| row.repo.clone())
        };

        table.add_row(vec![
            Cell::new(idx + 1),
            Cell::new(status_text).fg(status_color),
            Cell::new(repo_name),
            Cell::new(&row.branch),
            Cell::new(row.modified),
            Cell::new(row.staged),
            Cell::new(row.untracked),
            Cell::new(row.ahead),
            Cell::new(row.behind),
            Cell::new(shorten_when(&row.last_when)),
            Cell::new(&row.last_push),
            Cell::new(&row.last_author),
        ]);
    }

    println!("{table}");
    println!();
    println!(
        "{} repos: {} {}  {} {}  {} {}  ❌ {}{}",
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

    Ok(())
}

fn log_incident(
    policy_path: &Path,
    scope: impl Into<String>,
    repo: impl Into<String>,
    reason: impl Into<String>,
    action: impl Into<String>,
    backup_branch: Option<String>,
    result: impl Into<String>,
    details: Option<String>,
) {
    let record = IncidentRecord::new(
        timestamp_secs(),
        scope,
        repo,
        reason,
        action,
        backup_branch,
        result,
        details,
    );
    append_incident_record(policy_path, &record);
}

struct RepairState {
    attempted_ops: usize,
    succeeded_ops: usize,
    manual_only: usize,
    has_origin: bool,
    has_upstream: bool,
    push_ok: bool,
}

async fn handle_no_origin(
    state: &mut RepairState,
    repo: &Path,
    apply: bool,
    human: bool,
    policy: &SyncPolicy,
    reason: &str,
    policy_path: &Path,
) -> bool {
    if state.has_origin {
        return false;
    }
    state.attempted_ops += 1;
    if apply {
        let private_remote = if policy.auto_github_private {
            if human {
                println!("   plan: create GitHub private repo as origin");
            }
            create_github_private_remote(repo, &policy.auto_github_private_account, true)
        } else {
            if human {
                println!("   plan: create private bare repo as origin");
            }
            create_private_remote(repo)
        };
        if let Some(private_remote) = private_remote {
            state.succeeded_ops += 1;
            state.has_origin = true;
            state.has_upstream = true;
            if human {
                println!("   ok: created private remote: {}", private_remote);
            }
            log_incident(
                policy_path,
                "concern",
                repo.display().to_string(),
                reason,
                "create_private_remote",
                None,
                "ok",
                Some(format!("created private remote: {}", private_remote)),
            );
        } else {
            state.manual_only += 1;
            if human {
                println!("   fail: could not create private remote");
            }
            log_incident(
                policy_path,
                "concern",
                repo.display().to_string(),
                reason,
                "create_private_remote",
                None,
                "fail",
                Some("failed to create private remote".to_string()),
            );
        }
    }
    true
}

async fn handle_no_upstream(
    state: &mut RepairState,
    repo: &Path,
    apply: bool,
    human: bool,
    push_timeout_secs: u64,
    _push_retries: u32,
    reason: &str,
    policy_path: &Path,
) -> bool {
    if state.has_upstream {
        return false;
    }
    state.attempted_ops += 1;
    if human {
        println!("   plan: set upstream via `git push -u origin HEAD`");
    }
    if apply {
        match run_git_with_timeout(
            repo,
            &["push", "-u", "origin", "HEAD"],
            push_timeout_secs,
            "push -u",
        )
        .await
        {
            Ok(()) => {
                state.succeeded_ops += 1;
                state.has_upstream = true;
                if human {
                    println!("   ok: upstream configured");
                }
                log_incident(
                    policy_path,
                    "concern",
                    repo.display().to_string(),
                    reason,
                    "set_upstream_push_u",
                    None,
                    "ok",
                    None,
                );
            }
            Err(e) => {
                if human {
                    println!("   fail: upstream configure failed: {}", e);
                }
                log_incident(
                    policy_path,
                    "concern",
                    repo.display().to_string(),
                    reason,
                    "set_upstream_push_u",
                    None,
                    "fail",
                    Some(e.to_string()),
                );
                return true;
            }
        }
    }
    false
}

async fn handle_behind(
    state: &mut RepairState,
    repo: &Path,
    apply: bool,
    human: bool,
    pull_timeout_secs: u64,
    reason: &str,
    policy_path: &Path,
) -> bool {
    state.attempted_ops += 1;
    if human {
        println!("   plan: pull --no-rebase (merge)");
    }
    if apply {
        match run_git_with_timeout(
            repo,
            &["pull", "--no-rebase"],
            pull_timeout_secs,
            "pull/merge",
        )
        .await
        {
            Ok(()) => {
                state.succeeded_ops += 1;
                if human {
                    println!("   ok: pulled");
                }
                log_incident(
                    policy_path,
                    "concern",
                    repo.display().to_string(),
                    reason,
                    "pull_merge",
                    None,
                    "ok",
                    None,
                );
            }
            Err(e) => {
                if human {
                    println!("   fail: pull failed: {}", e);
                }
                log_incident(
                    policy_path,
                    "concern",
                    repo.display().to_string(),
                    reason,
                    "pull_merge",
                    None,
                    "fail",
                    Some(e.to_string()),
                );
            }
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
async fn handle_ahead(
    state: &mut RepairState,
    repo: &Path,
    apply: bool,
    human: bool,
    push_timeout_secs: u64,
    push_retries: u32,
    blob_threshold: u64,
    rewrite_large_any: bool,
    excluded_dir_names: &std::collections::BTreeSet<String>,
    reason: &str,
    policy_path: &Path,
    svc: &GitService,
) -> bool {
    state.attempted_ops += 1;
    if human {
        println!("   plan: push origin HEAD");
    }
    state.push_ok = false;
    if !apply {
        return false;
    }
    #[allow(unused_assignments)]
    match push_with_retries(repo, push_timeout_secs, push_retries, "push").await {
        Ok(()) => {
            state.succeeded_ops += 1;
            state.push_ok = true;
            if human {
                println!("   ok: pushed");
            }
            log_incident(
                policy_path,
                "concern",
                repo.display().to_string(),
                reason,
                "push_origin_head",
                None,
                "ok",
                None,
            );
            // Also push to mirror remotes (codeberg, gitlab, etc.)
            if let Ok(policy) = SyncPolicy::load(policy_path) {
                if !policy.remotes.is_empty() {
                    let mirror_results = push_mirror_remotes(
                        repo,
                        &policy.remotes,
                        push_timeout_secs,
                        push_retries,
                        true,
                    ).await;
                    for (name, result) in &mirror_results {
                        if let Err(e) = result {
                            if human {
                                println!("   warn: mirror push to {} failed: {}", name, e);
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            if human {
                println!("   fail: push failed: {}", e);
            }

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
                if human {
                    println!("   info: no remote detected, creating private bare repo");
                }
                if let Some(private_remote) = create_private_remote(repo) {
                    if human {
                        println!("   info: created private remote: {}", private_remote);
                    }
                    // Retry push with new remote
                    match push_with_retries(repo, push_timeout_secs, push_retries, "push").await {
                        Ok(()) => {
                            state.succeeded_ops += 1;
                            state.push_ok = true;
                            if human {
                                println!("   ok: pushed to private remote");
                            }
                            log_incident(
                                policy_path,
                                "concern",
                                repo.display().to_string(),
                                reason,
                                "push_origin_head",
                                None,
                                "ok",
                                Some(format!("pushed to private remote: {}", private_remote)),
                            );
                            return true;
                        }
                        Err(e2) => {
                            if human {
                                println!("   fail: push to private remote also failed: {}", e2);
                            }
                            log_incident(
                                policy_path,
                                "concern",
                                repo.display().to_string(),
                                reason,
                                "push_origin_head",
                                None,
                                "fail",
                                Some(e2.to_string()),
                            );
                            return true;
                        }
                    }
                } else {
                    if human {
                        println!("   fail: could not create private remote");
                    }
                    log_incident(
                        policy_path,
                        "concern",
                        repo.display().to_string(),
                        reason,
                        "push_origin_head",
                        None,
                        "fail",
                        Some(e.to_string()),
                    );
                    return true;
                }
            }

            // For permission denied or other errors on existing remote,
            // just record failure and continue - no permanent marking
            // These will retry on next cycle naturally
            log_incident(
                policy_path,
                "concern",
                repo.display().to_string(),
                reason,
                "push_origin_head",
                None,
                "fail",
                Some(e.to_string()),
            );
            // Don't continue here - let it fall through to large blob detection below
            // (but without the manual_only marking)

            let large = detect_large_blobs_ahead(repo, blob_threshold)
                .await
                .unwrap_or_default();
            if !large.is_empty() {
                if human {
                    println!(
                        "   detect: large blobs in ahead range ({} entries)",
                        large.len()
                    );
                }
                let mut dirs = BTreeSet::new();
                for (_, path) in &large {
                    if let Some(dir) = top_level_dir(path) {
                        if is_excluded_dir_name(&dir, excluded_dir_names) {
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
                    if human {
                        println!("   manual: large blobs found but not in excluded dirs");
                    }
                    log_incident(
                        policy_path,
                        "concern",
                        repo.display().to_string(),
                        reason,
                        "large_blob_detected",
                        None,
                        "manual",
                        Some(format!(
                            "threshold={} entries={} rewrite_allowed=false",
                            blob_threshold,
                            large.len()
                        )),
                    );
                } else {
                    if human {
                        println!(
                            "   plan: rewrite ahead history removing paths {:?}",
                            rewrite_paths
                        );
                    }
                    match rewrite_ahead_paths(repo, &rewrite_paths, "backup/pre-sync-largeblob-fix")
                    {
                        Ok(Some(backup_branch)) => {
                            let backup_branch_for_log = backup_branch.clone();
                            if human {
                                println!(
                                    "   ok: rewrite complete (backup branch: {})",
                                    backup_branch
                                );
                            }
                            match push_with_retries(
                                repo,
                                push_timeout_secs,
                                push_retries,
                                "push-after-rewrite",
                            )
                            .await
                            {
                                Ok(()) => {
                                    state.succeeded_ops += 1;
                                    state.push_ok = true;
                                    if human {
                                        println!("   ok: pushed after rewrite");
                                    }
                                    log_incident(
                                        policy_path,
                                        "concern",
                                        repo.display().to_string(),
                                        reason,
                                        "rewrite_then_push",
                                        Some(backup_branch_for_log),
                                        "ok",
                                        Some(format!("paths={:?}", rewrite_paths)),
                                    );
                                    // Also push to mirror remotes
                                    if let Ok(policy) = SyncPolicy::load(policy_path) {
                                        if !policy.remotes.is_empty() {
                                            push_mirror_remotes(repo, &policy.remotes, push_timeout_secs, push_retries, true).await;
                                        }
                                    }
                                }
                                Err(e2) => {
                                    if human {
                                        println!("   fail: push after rewrite failed: {}", e2);
                                    }
                                    log_incident(
                                        policy_path,
                                        "concern",
                                        repo.display().to_string(),
                                        reason,
                                        "rewrite_then_push",
                                        Some(backup_branch),
                                        "fail",
                                        Some(e2.to_string()),
                                    );
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(rewrite_err) => {
                            if human {
                                println!("   fail: rewrite failed: {}", rewrite_err);
                            }
                            log_incident(
                                policy_path,
                                "concern",
                                repo.display().to_string(),
                                reason,
                                "rewrite_large_blob",
                                None,
                                "fail",
                                Some(rewrite_err.to_string()),
                            );
                        }
                    }
                }
            } else {
                let branch = current_branch(repo).unwrap_or_default();
                let dry_run = run_git_capture_output(
                    repo,
                    &["push", "--dry-run", "origin", "HEAD"],
                    "push --dry-run",
                )
                .unwrap_or_default();
                let looks_branch_mismatch = dry_run.to_ascii_lowercase().contains("up-to-date");
                if looks_branch_mismatch
                    && !branch.is_empty()
                    && remote_branch_exists(repo, &branch)
                    && has_tracking_upstream(repo)
                {
                    if human {
                        println!(
                            "   plan: align upstream to origin/{} (possible branch mismatch)",
                            branch
                        );
                    }
                    match set_upstream_to_branch(repo, &branch) {
                        Ok(()) => {
                            if human {
                                println!("   ok: upstream realigned");
                            }
                            match push_with_retries(
                                repo,
                                push_timeout_secs,
                                push_retries,
                                "push-after-upstream-align",
                            )
                            .await
                            {
                                Ok(()) => {
                                    state.succeeded_ops += 1;
                                    state.push_ok = true;
                                    if human {
                                        println!("   ok: pushed after upstream align");
                                    }
                                    log_incident(
                                        policy_path,
                                        "concern",
                                        repo.display().to_string(),
                                        reason,
                                        "realign_upstream_then_push",
                                        None,
                                        "ok",
                                        Some(format!("branch={}", branch)),
                                    );
                                    // Also push to mirror remotes
                                    if let Ok(policy) = SyncPolicy::load(policy_path) {
                                        if !policy.remotes.is_empty() {
                                            push_mirror_remotes(repo, &policy.remotes, push_timeout_secs, push_retries, true).await;
                                        }
                                    }
                                }
                                Err(e2) => {
                                    if human {
                                        println!(
                                            "   fail: push after upstream align failed: {}",
                                            e2
                                        );
                                    }
                                    log_incident(
                                        policy_path,
                                        "concern",
                                        repo.display().to_string(),
                                        reason,
                                        "realign_upstream_then_push",
                                        None,
                                        "fail",
                                        Some(e2.to_string()),
                                    );
                                }
                            }
                        }
                        Err(set_err) => {
                            if human {
                                println!("   fail: upstream align failed: {}", set_err);
                            }
                        }
                    }
                }
            }
        }
    }
    if !state.push_ok {
        log_incident(
            policy_path,
            "concern",
            repo.display().to_string(),
            reason,
            "push_origin_head",
            None,
            "fail",
            Some("push did not clear concern".to_string()),
        );
    }
    if state.push_ok {
        if let Ok(next_after_push) = svc.get_status().await {
            if next_after_push.ahead > 0 {
                let branch = current_branch(repo).unwrap_or_default();
                if !branch.is_empty() && remote_branch_exists(repo, &branch) {
                    if human {
                        println!(
                            "   plan: realign upstream to origin/{} (ahead still > 0 after push)",
                            branch
                        );
                    }
                    match set_upstream_to_branch(repo, &branch) {
                        Ok(()) => {
                            if human {
                                println!("   ok: upstream realigned");
                            }
                        }
                        Err(e) => {
                            if human {
                                println!("   fail: upstream realign failed: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

async fn verify_resolution(
    repo: &Path,
    apply: bool,
    human: bool,
    resolved: &mut usize,
    reason: &str,
    policy_path: &Path,
    svc: &GitService,
) {
    if !apply {
        return;
    }
    if let Ok(next) = svc.get_status().await {
        let has_origin = has_origin_remote(repo);
        let has_upstream = has_tracking_upstream(repo);
        let still_concern = next.ahead > 0 || next.behind > 0 || !has_origin || !has_upstream;
        if !still_concern {
            *resolved += 1;
            if human {
                println!("   resolved: concern cleared");
            }
            log_incident(
                policy_path,
                "concern",
                repo.display().to_string(),
                reason,
                "verify_resolved",
                None,
                "ok",
                None,
            );
        } else {
            if human {
                println!(
                    "   remaining: ahead={} behind={} origin={} upstream={}",
                    next.ahead, next.behind, has_origin, has_upstream
                );
            }
            // Only notify on true divergence (both ahead AND behind) - that's
            // the only case where we have no automatic resolution.
            // If just ahead > 0, we can push. If just behind > 0, we can pull.
            if next.ahead > 0 && next.behind > 0 {
                let details = format!("ahead={} behind={}", next.ahead, next.behind);
                send_sync_conflict_notification(repo, reason, &details);
            }
            log_incident(
                policy_path,
                "concern",
                repo.display().to_string(),
                reason,
                "verify_resolved",
                None,
                "remaining",
                Some(format!("ahead={} behind={}", next.ahead, next.behind)),
            );
        }
    }
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
        discover_git_repos(
            &roots,
            &excluded_dir_names,
            &policy.exclude_repos,
            Some(&policy.system_repo),
        )
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
    let mut state = RepairState {
        attempted_ops: 0,
        succeeded_ops: 0,
        manual_only: 0,
        has_origin: false,
        has_upstream: false,
        push_ok: false,
    };
    let mut resolved = 0usize;

    for repo in repos {
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

        state.has_origin = has_origin_remote(&repo);
        state.has_upstream = has_tracking_upstream(&repo);
        let is_concern = repo_is_concern(&status, state.has_origin, state.has_upstream);
        if !is_concern {
            continue;
        }
        let stuck_push = status.ahead > 0 && state.has_origin && state.has_upstream;
        let stuck_pull = status.behind > 0 && state.has_origin && state.has_upstream;
        if matches!(filter, ConcernRepairFilter::StuckPush) && !stuck_push {
            continue;
        }
        if matches!(filter, ConcernRepairFilter::StuckPull) && !stuck_pull {
            continue;
        }
        concerns += 1;
        let flags = repo_state_flags(&status, state.has_origin, state.has_upstream);
        let reason = flags.join(",");

        out!(
            "\n🔎 {}  state: ahead={} behind={} clean={} origin={} upstream={}",
            repo.display(),
            status.ahead,
            status.behind,
            status.is_clean,
            state.has_origin,
            state.has_upstream
        );

        if handle_no_origin(
            &mut state,
            &repo,
            apply,
            human,
            &policy,
            &reason,
            policy_path,
        )
        .await
        {
            continue;
        }

        if handle_no_upstream(
            &mut state,
            &repo,
            apply,
            human,
            push_timeout_secs,
            push_retries,
            &reason,
            policy_path,
        )
        .await
        {
            continue;
        }

        #[allow(clippy::collapsible_if)]
        if status.behind > 0 && state.has_upstream {
            if handle_behind(
                &mut state,
                &repo,
                apply,
                human,
                policy.pull_op_timeout_secs,
                &reason,
                policy_path,
            )
            .await
            {
                continue;
            }
            // Re-fetch status after pull — the repo state may have changed
            // (e.g. diverged repo is now just ahead after merge).
            if let Ok(new_status) = svc.get_status().await {
                status = new_status;
                state.has_upstream = has_tracking_upstream(&repo);
            }
        }

        #[allow(clippy::collapsible_if)]
        if status.ahead > 0 && state.has_upstream {
            if handle_ahead(
                &mut state,
                &repo,
                apply,
                human,
                push_timeout_secs,
                push_retries,
                blob_threshold,
                rewrite_large_any,
                &excluded_dir_names,
                &reason,
                policy_path,
                &svc,
            )
            .await
            {
                continue;
            }
        }

        verify_resolution(
            &repo,
            apply,
            human,
            &mut resolved,
            &reason,
            policy_path,
            &svc,
        )
        .await;
    }

    let summary = RepairSummary {
        found: concerns,
        planned: state.attempted_ops,
        attempted: if apply { state.attempted_ops } else { 0 },
        succeeded: state.succeeded_ops,
        resolved_now: if apply { resolved } else { 0 },
        manual_only: state.manual_only,
    };
    if json {
        let payload = RepairJson {
            policy: policy_path.display().to_string(),
            scope: "concern".to_string(),
            mode: if apply {
                "apply".to_string()
            } else {
                "dry_run".to_string()
            },
            found: summary.found,
            planned: summary.planned,
            attempted: summary.attempted,
            succeeded: summary.succeeded,
            resolved_now: summary.resolved_now,
            manual_only: summary.manual_only,
            ledger: incident_ledger_path(policy_path).display().to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if summary.found > 0 {
        println!("\n✅ Concern management summary");
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
        discover_git_repos(
            &roots,
            &excluded_dir_names,
            &policy.exclude_repos,
            Some(&policy.system_repo),
        )
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
            modified_files: status.modified_files,
            staged_files: status.staged_files,
            ..status.clone()
        };
        // Use real dirty state for classification — a repo with TRACKED
        // modified files is WARN even if the daemon wouldn't auto-commit them.
        // Untracked files (build artifacts) do NOT count as dirty.
        let real_is_dirty = status.modified_files > 0 || status.staged_files > 0;
        if !real_is_dirty {
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
            crate::sync::sync_repo(
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
                        details: Some(format!("timeout={}s", policy.repo_sync_timeout_secs)),
                    },
                );
            }
            Ok(Ok(outcome)) => {
                succeeded += 1;
                out!("   ok: triage complete changed={}", outcome.has_changes());
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
                        details: Some(format!("changed={}", outcome.has_changes())),
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
            mode: if apply {
                "apply".to_string()
            } else {
                "dry_run".to_string()
            },
            found: summary.found,
            planned: summary.planned,
            attempted: summary.attempted,
            succeeded: summary.succeeded,
            resolved_now: summary.resolved_now,
            manual_only: summary.manual_only,
            ledger: incident_ledger_path(policy_path).display().to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if summary.found > 0 {
        println!("\n✅ Warn management summary");
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

pub(crate) fn create_github_private_remote(
    repo: &Path,
    account: &str,
    private: bool,
) -> Option<String> {
    let repo_name = repo.file_name()?.to_str()?.to_string();

    // FIRST REPO CREATE ATTEMPT:
    // Try to create a private GitHub repo matching the local directory name.
    // If it already exists, we reuse it below — we NEVER append a suffix.
    //   ⚠️  HISTORY: A previous version had a loop that appended -1, -2, -N to
    //   repo names when the base name was taken. This created 15+ orphan repos
    //   (dracon-demons-1..-9, browser-extensions-shared-1..-6).
    //   The suffix approach is DANGEROUS because:
    //   1. Every daemon cycle creates a new orphan repo
    //   2. GitHub counts orphan repos against quotas
    //   3. No cleanup mechanism existed
    //   NEVER reintroduce a suffix loop here or in any repo creation function.
    let mut cmd = std::process::Command::new("gh");
    cmd.args(["repo", "create", &repo_name]);
    if private {
        cmd.arg("--private");
    } else {
        cmd.arg("--public");
    }
    let output = cmd.current_dir(repo).output().ok()?;

    if output.status.success() {
        let remote_url = format!("https://github.com/{}/{}.git", account, repo_name);

        let add_result = std::process::Command::new("git")
            .args(["remote", "add", "origin", &remote_url])
            .current_dir(repo)
            .output();

        if let Err(e) = add_result {
            eprintln!("⚠️ failed to add origin for {}: {}", repo.display(), e);
        }

        let mut current_branch =
            crate::git::current_branch(repo).unwrap_or_else(|| "main".to_string());

        if current_branch == "master" {
            if let Err(e) = std::process::Command::new("git")
                .args(["branch", "-m", "master", "main"])
                .current_dir(repo)
                .output()
            {
                eprintln!(
                    "⚠️ failed to rename master to main in {}: {}",
                    repo.display(),
                    e
                );
            } else {
                current_branch = "main".to_string();
            }
        }

        let push_result = std::process::Command::new("git")
            .args([
                "push",
                "-u",
                "origin",
                &format!("HEAD:refs/heads/{}", current_branch),
            ])
            .current_dir(repo)
            .output();

        if let Ok(push_output) = push_result {
            if !push_output.status.success() {
                let stderr = String::from_utf8_lossy(&push_output.stderr);
                eprintln!(
                    "⚠️ failed to push initial commit for {}: {}",
                    repo.display(),
                    stderr
                );
            }
        } else {
            eprintln!(
                "⚠️ failed to push initial commit for {}: could not execute",
                repo.display()
            );
        }

        if !crate::git::has_tracking_upstream(repo) {
            let _ = std::process::Command::new("git")
                .args(["branch", "--set-upstream-to=origin/main", &current_branch])
                .current_dir(repo)
                .output();
        }

        return Some(remote_url);
    }

    // Repo already exists — reuse it instead of creating a new one with a suffix
    let remote_url = format!("https://github.com/{}/{}.git", account, repo_name);

    // Check if origin already exists locally before adding
    let has_origin = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_origin {
        let add_result = std::process::Command::new("git")
            .args(["remote", "add", "origin", &remote_url])
            .current_dir(repo)
            .output();

        if let Err(e) = add_result {
            eprintln!("⚠️ failed to add origin for {}: {}", repo.display(), e);
        }
    }

    let mut current_branch = crate::git::current_branch(repo).unwrap_or_else(|| "main".to_string());

    if current_branch == "master" {
        if let Err(e) = std::process::Command::new("git")
            .args(["branch", "-m", "master", "main"])
            .current_dir(repo)
            .output()
        {
            eprintln!(
                "⚠️ failed to rename master to main in {}: {}",
                repo.display(),
                e
            );
        } else {
            current_branch = "main".to_string();
        }
    }

    let _ = std::process::Command::new("git")
        .args([
            "push",
            "-u",
            "origin",
            &format!("HEAD:refs/heads/{}", current_branch),
        ])
        .current_dir(repo)
        .output();

    if !crate::git::has_tracking_upstream(repo) {
        let _ = std::process::Command::new("git")
            .args(["branch", "--set-upstream-to=origin/main", &current_branch])
            .current_dir(repo)
            .output();
    }

    Some(remote_url)
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
    let private_remotes_dir = dirs::data_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("dracon/private-remotes");

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
        eprintln!("⚠️ failed to add origin for {}: {}", repo.display(), e);
    }

    Some(remote_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::EnvRestorer;
    use dracon_git::types::RepoStatus;
    use std::os::unix::fs::PermissionsExt;

    fn make_status(is_clean: bool, ahead: usize, behind: usize) -> RepoStatus {
        RepoStatus {
            branch: String::new(),
            is_clean,
            ahead,
            behind,
            modified_files: if is_clean { 0 } else { 1 },
            untracked_files: 0,
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
        assert_eq!(hint, "run repair-concerns --apply (pull/merge)");
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
            Self {
                var: var.to_string(),
                original,
                _lock: lock,
            }
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
        assert!(path
            .to_string_lossy()
            .contains("dracon-sync-incidents.jsonl"));
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
            remotes: vec![],
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
            webhook_url: None,
            alert_unpushed_threshold: 10,
            sync_visibility: false,
            sync_visibility_interval_hours: 24,
            sync_metadata: false,
            auto_tag: true,
            auto_release: false,
            auto_publish: false,
            publish_targets: vec![],
            nix_auto_update: false,
            standard_files: vec![],
            standard_files_auto: true,
        }
    }

    #[test]
    fn test_truncate_unicode_emoji() {
        let result = truncate("hello 👋 world", 10);
        assert!(result.ends_with('…'));
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
            untracked: 0,
            ahead: 0,
            behind: 0,
            last_hash: "abc123".to_string(),
            last_author: "test".to_string(),
            last_when: "2024-01-01".to_string(),
            last_msg: "test commit".to_string(),
            last_unix: 1700000000,
            last_push: "5m ago".to_string(),
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
            remotes: 0,
            remote_configs: vec![],
        };
        assert_eq!(status.repos_discovered, 5);
        assert!(status.auto_commit);
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
        assert_eq!(
            threshold,
            10 * 1024 * 1024,
            "should use smaller of stage and push limit"
        );
    }

    #[test]
    fn test_create_github_private_remote_success() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("my-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");

        let gh_mock = tmp.path().join("gh");
        std::fs::write(&gh_mock, "#!/bin/sh\necho \"mock gh called\" >&2\nexit 0\n")
            .expect("write gh mock");
        std::fs::set_permissions(&gh_mock, std::fs::Permissions::from_mode(0o755))
            .expect("chmod gh");
        let _lock = crate::git::acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new(
            "PATH",
            &format!("{}:{}", tmp.path().to_string_lossy(), orig_path),
        );

        let result = create_github_private_remote(&repo, "testaccount", true);

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            "https://github.com/testaccount/my-repo.git"
        );
    }

    #[test]
    fn test_create_github_private_remote_already_exists_reuses_without_suffix() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("dracon-demons");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");

        let gh_mock = tmp.path().join("gh");
        std::fs::write(
            &gh_mock,
            "#!/bin/sh\necho ' Name already exists' >&2\nexit 1\n",
        )
        .expect("write gh mock");
        std::fs::set_permissions(&gh_mock, std::fs::Permissions::from_mode(0o755))
            .expect("chmod gh");
        let _lock = crate::git::acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new(
            "PATH",
            &format!("{}:{}", tmp.path().to_string_lossy(), orig_path),
        );

        let result = create_github_private_remote(&repo, "testaccount", true);

        assert!(result.is_some());
        let url = result.unwrap();
        assert!(!url.contains("-1"), "should NOT contain suffix -1: {}", url);
        assert!(!url.contains("-2"), "should NOT contain suffix -2: {}", url);
        assert_eq!(url, "https://github.com/testaccount/dracon-demons.git");
    }

    #[test]
    fn test_create_github_private_remote_origin_already_exists_does_not_add_duplicate() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("existing-remote-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");
        std::process::Command::new("git")
            .args(["remote", "add", "origin", "git@github.com:old/old.git"])
            .current_dir(&repo)
            .status()
            .expect("git remote add");

        let gh_mock = tmp.path().join("gh");
        std::fs::write(&gh_mock, "#!/bin/sh\nexit 1\n").expect("write gh mock");
        std::fs::set_permissions(&gh_mock, std::fs::Permissions::from_mode(0o755))
            .expect("chmod gh");
        let _lock = crate::git::acquire_path_lock();
        let orig_path = std::env::var("PATH").unwrap_or_default();
        let _guard = EnvRestorer::new(
            "PATH",
            &format!("{}:{}", tmp.path().to_string_lossy(), orig_path),
        );

        let result = create_github_private_remote(&repo, "testaccount", true);

        assert!(result.is_some());
        let remotes = crate::git::multi_remote::list_remotes(&repo);
        assert_eq!(remotes.len(), 1, "should not add duplicate origin");
        assert_eq!(remotes[0], "origin");
    }

    #[test]
    fn test_create_github_private_remote_no_gh_installed_returns_none() {
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let repo = tmp.path().join("no-gh-repo");
        std::process::Command::new("git")
            .args(["init", "-q", "-b", "master"])
            .arg(&repo)
            .status()
            .expect("git init");

        let git_dir = std::path::Path::new("/run/current-system/sw/bin");
        let _lock = crate::git::acquire_path_lock();
        let _guard = EnvRestorer::new(
            "PATH",
            &format!(
                "{}:{}",
                tmp.path().to_string_lossy(),
                git_dir.to_string_lossy()
            ),
        );

        let result = create_github_private_remote(&repo, "testaccount", true);

        assert!(result.is_none());
    }

    #[test]
    fn test_shorten_when() {
        assert_eq!(shorten_when("5 seconds ago"), "5s");
        assert_eq!(shorten_when("29 minutes ago"), "29m");
        assert_eq!(shorten_when("74 minutes ago"), "1h14m");
        assert_eq!(shorten_when("60 minutes ago"), "1h");
        assert_eq!(shorten_when("119 minutes ago"), "1h59m");
        assert_eq!(shorten_when("3 hours ago"), "3h");
        assert_eq!(shorten_when("2 days ago"), "2d");
        assert_eq!(shorten_when("6 weeks ago"), "6w");
        assert_eq!(shorten_when("just now"), "just now");
        assert_eq!(shorten_when("unknown"), "unknown");
    }
}
