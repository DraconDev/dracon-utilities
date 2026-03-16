use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dracon_git::{
    build_commit_message, CommitContext,
    extract_intent, GitService,
};
use dracon_git::types::{DiffFile, RepoStatus};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::process::Command as TokioCommand;
use tokio::time::{sleep, Duration};

#[derive(Parser, Debug)]
#[command(name = "dracon-sync")]
#[command(about = "Dracon sync runtime")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Show resolved policy path and sync scope.
    Status {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// One-off report across discovered repositories.
    Repos {
        /// Show only concern repos.
        #[arg(long)]
        only_concern: bool,
        /// Show only warn repos.
        #[arg(long, conflicts_with = "only_concern")]
        only_warn: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Repair concern repos (dry-run by default; use --apply to execute).
    RepairConcerns {
        /// Execute git operations to repair concerns.
        #[arg(long)]
        apply: bool,
        /// Only repair this repository path.
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Override push timeout seconds for this run.
        #[arg(long)]
        push_timeout_secs: Option<u64>,
        /// Retry count for push operations.
        #[arg(long, default_value_t = 3)]
        push_retries: u32,
        /// Allow rewrite of large blobs even when paths are outside excluded dirs.
        #[arg(long)]
        rewrite_large_any: bool,
        /// Only repair stuck push concerns.
        #[arg(long, conflicts_with = "only_stuck_pull")]
        only_stuck_push: bool,
        /// Only repair stuck pull concerns.
        #[arg(long, conflicts_with = "only_stuck_push")]
        only_stuck_pull: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Repair warn repos (dirty-only triage; dry-run by default).
    RepairWarns {
        /// Execute git operations to repair warns.
        #[arg(long)]
        apply: bool,
        /// Only repair this repository path.
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Run one sync pass.
    Once,
    /// Run continuous sync loop.
    Daemon,
    /// Sync a specific repository now.
    SyncNow { repo: PathBuf },
    /// Open sync policy in the system editor.
    EditConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct SyncPolicy {
    #[serde(default)]
    system_repo: String,
    #[serde(default = "default_pulse_interval")]
    pulse_interval_secs: u64,
    #[serde(default = "default_inactivity_push_delay_secs")]
    inactivity_push_delay_secs: u64,
    #[serde(default = "default_true")]
    auto_commit: bool,
    /// If true, bump patch versions before an auto-commit (best-effort).
    /// Applies to common files when present at repo root:
    /// - Rust: `Cargo.toml` (and keep `Cargo.lock` aligned for root package)
    /// - Node/TS: `package.json` (and align `package-lock.json` root `version` when applicable)
    /// - Generic: `VERSION`
    #[serde(default = "default_true")]
    auto_bump_versions: bool,
    #[serde(default = "default_true")]
    auto_pull: bool,
    #[serde(default = "default_true")]
    auto_push: bool,
    #[serde(default)]
    backup_policy: String,
    #[serde(default)]
    backup_dir: String,
    #[serde(default)]
    watch_roots: Vec<String>,
    #[serde(default)]
    extra_remotes: HashMap<String, String>,
    #[serde(default = "default_exclude_dir_names")]
    exclude_dir_names: Vec<String>,
    #[serde(default = "default_max_stage_file_bytes")]
    max_stage_file_bytes: u64,
    #[serde(default = "default_pull_op_timeout_secs")]
    pull_op_timeout_secs: u64,
    #[serde(default = "default_push_op_timeout_secs")]
    push_op_timeout_secs: u64,
    #[serde(default = "default_repo_sync_timeout_secs")]
    repo_sync_timeout_secs: u64,
    #[serde(default = "default_true")]
    auto_repair_concerns: bool,
    #[serde(default = "default_true")]
    auto_repair_warns: bool,
    #[serde(default)]
    auto_rewrite_large_blobs: bool,
    #[serde(default = "default_push_retries")]
    push_retries: u32,
    #[serde(default = "default_repair_cooldown_secs")]
    repair_cooldown_secs: u64,
    #[serde(default = "default_max_push_blob_bytes")]
    max_push_blob_bytes: u64,
    #[serde(default = "default_incident_ledger_max_lines")]
    incident_ledger_max_lines: usize,
    #[serde(default = "default_incident_ledger_max_age_days")]
    incident_ledger_max_age_days: u64,
    #[serde(default = "default_exclude_file_patterns")]
    exclude_file_patterns: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_pulse_interval() -> u64 {
    1
}

fn default_inactivity_push_delay_secs() -> u64 {
    5
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RepoPolicyOverride {
    /// Optional per-repo override for `auto_bump_versions`.
    auto_bump_versions: Option<bool>,
}

fn load_repo_override(repo: &Path) -> RepoPolicyOverride {
    let path = repo.join(".dracon").join("dracon-sync.toml");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return RepoPolicyOverride::default();
    };
    toml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("⚠️ failed to parse repo override {}: {}", path.display(), e);
        RepoPolicyOverride::default()
    })
}

fn bump_semver_patch(ver: &str) -> Option<String> {
    let parts: Vec<&str> = ver.split('.').collect();
    if parts.len() < 3 {
        return None;
    }
    if !parts[0].chars().all(|c| c.is_ascii_digit())
        || !parts[1].chars().all(|c| c.is_ascii_digit())
        || !parts[2].chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let major: u64 = parts[0].parse().ok()?;
    let minor: u64 = parts[1].parse().ok()?;
    let patch: u64 = parts[2].parse().ok()?;
    Some(format!("{}.{}.{}", major, minor, patch + 1))
}

fn bump_first_json_string_field(content: &str, key: &str) -> Option<(String, String, String)> {
    // Tiny, formatting-preserving bump helper:
    // finds the first `"key": "x.y.z"` occurrence and bumps patch.
    let needle = format!("\"{}\"", key);
    let mut start = 0usize;
    while let Some(idx) = content[start..].find(&needle) {
        let key_pos = start + idx;
        let after_key = key_pos + needle.len();
        let rest = &content[after_key..];
        let colon_rel = rest.find(':')?;
        let after_colon = after_key + colon_rel + 1;
        let rest2 = &content[after_colon..];
        let q1_rel = rest2.find('"')?;
        let q1 = after_colon + q1_rel + 1;
        let rest3 = &content[q1..];
        let q2_rel = rest3.find('"')?;
        let q2 = q1 + q2_rel;
        let old_ver = &content[q1..q2];
        if let Some(new_ver) = bump_semver_patch(old_ver) {
            let mut out = String::with_capacity(content.len());
            out.push_str(&content[..q1]);
            out.push_str(&new_ver);
            out.push_str(&content[q2..]);
            return Some((out, old_ver.to_string(), new_ver));
        }
        start = after_key;
    }
    None
}

fn set_first_json_string_field_to_value(
    content: &str,
    key: &str,
    expected_old: &str,
    new_value: &str,
) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let mut start = 0usize;
    while let Some(idx) = content[start..].find(&needle) {
        let key_pos = start + idx;
        let after_key = key_pos + needle.len();
        let rest = &content[after_key..];
        let colon_rel = rest.find(':')?;
        let after_colon = after_key + colon_rel + 1;
        let rest2 = &content[after_colon..];
        let q1_rel = rest2.find('"')?;
        let q1 = after_colon + q1_rel + 1;
        let rest3 = &content[q1..];
        let q2_rel = rest3.find('"')?;
        let q2 = q1 + q2_rel;
        let old = &content[q1..q2];
        if old == expected_old {
            let mut out = String::with_capacity(content.len());
            out.push_str(&content[..q1]);
            out.push_str(new_value);
            out.push_str(&content[q2..]);
            return Some(out);
        }
        start = after_key;
    }
    None
}

#[derive(Debug, Clone, Copy)]
struct BumpOutcome {
    bumped_cargo_toml: bool,
    updated_cargo_lock: bool,
    bumped_workspace_package: bool,
}

fn bump_patch_version_in_repo(repo: &Path) -> Result<BumpOutcome> {
    fn bump_in_section(content: &str, target_section: &str) -> Option<(String, String)> {
        let mut out = String::with_capacity(content.len() + 16);
        let mut section = String::new();
        let mut changed = false;
        let mut new_version = String::new();

        for raw in content.split_inclusive('\n') {
            let line = raw.trim_end_matches('\n');
            let newline = if raw.ends_with('\n') { "\n" } else { "" };
            let trimmed = line.trim();

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section = trimmed
                    .trim_matches(&['[', ']'][..])
                    .trim()
                    .to_string();
                out.push_str(line);
                out.push_str(newline);
                continue;
            }

            if !changed && section == target_section {
                // Match `version = "x.y.z"` only inside the target section.
                if let Some(rest) = trimmed.strip_prefix("version") {
                    let rest = rest.trim_start();
                    if let Some(rest) = rest.strip_prefix('=') {
                        let rest = rest.trim_start();
                        if let Some((_, after_q1)) = rest.split_once('"') {
                            if let Some((ver, after_q2)) = after_q1.split_once('"') {
                                let parts: Vec<&str> = ver.split('.').collect();
                                if parts.len() >= 3
                                    && parts[0].chars().all(|c| c.is_ascii_digit())
                                    && parts[1].chars().all(|c| c.is_ascii_digit())
                                    && parts[2].chars().all(|c| c.is_ascii_digit())
                                {
                                    let major: u64 = parts[0].parse().ok()?;
                                    let minor: u64 = parts[1].parse().ok()?;
                                    let patch: u64 = parts[2].parse().ok()?;
                                    new_version = format!("{}.{}.{}", major, minor, patch + 1);

                                    // Reconstruct preserving indentation and any trailing comment.
                                    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
                                    out.push_str(&indent);
                                    out.push_str("version = \"");
                                    out.push_str(&new_version);
                                    out.push('"');
                                    out.push_str(after_q2);
                                    out.push_str(newline);
                                    changed = true;
                                    continue;
                                }
                            }
                        }
                    }
                }
            }

            out.push_str(line);
            out.push_str(newline);
        }

        if changed {
            Some((out, new_version))
        } else {
            None
        }
    }

    fn find_package_name(content: &str, target_section: &str) -> Option<String> {
        let mut section = String::new();
        for raw in content.split_inclusive('\n') {
            let line = raw.trim_end_matches('\n');
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section = trimmed
                    .trim_matches(&['[', ']'][..])
                    .trim()
                    .to_string();
                continue;
            }
            if section != target_section {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("name") {
                let rest = rest.trim_start();
                if let Some(rest) = rest.strip_prefix('=') {
                    let value = rest.trim();
                    if let Some(s) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                        return Some(s.to_string());
                    }
                }
            }
        }
        None
    }

    fn update_cargo_lock_package_version(
        repo: &Path,
        package_name: &str,
        new_version: &str,
    ) -> Result<bool> {
        let lock_path = repo.join("Cargo.lock");
        let Ok(content) = std::fs::read_to_string(&lock_path) else {
            return Ok(false);
        };

        let mut out = String::with_capacity(content.len());
        let mut in_pkg = false;
        let mut name_matches = false;
        let mut changed = false;

        for raw in content.split_inclusive('\n') {
            let line = raw.trim_end_matches('\n');
            let newline = if raw.ends_with('\n') { "\n" } else { "" };
            let trimmed = line.trim();

            if trimmed == "[[package]]" {
                in_pkg = true;
                name_matches = false;
                out.push_str(line);
                out.push_str(newline);
                continue;
            }

            if in_pkg {
                if let Some(rest) = trimmed.strip_prefix("name") {
                    let rest = rest.trim_start();
                    if let Some(rest) = rest.strip_prefix('=') {
                        let value = rest.trim();
                        if let Some(s) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                            name_matches = s == package_name;
                        }
                    }
                    out.push_str(line);
                    out.push_str(newline);
                    continue;
                }
            }

            if in_pkg && name_matches && trimmed.starts_with("version") {
                let rest = trimmed["version".len()..].trim_start();
                if rest.starts_with('=') {
                    let replacement = format!("version = \"{}\"", new_version);
                    if trimmed != replacement {
                        changed = true;
                        out.push_str(&replacement);
                        out.push_str(newline);
                        continue;
                    }
                }
            }

            out.push_str(line);
            out.push_str(newline);
        }

        if changed {
            std::fs::write(&lock_path, out)
                .with_context(|| format!("failed writing {}", lock_path.display()))?;
        }
        Ok(changed)
    }

    let cargo = repo.join("Cargo.toml");
    let Ok(content) = std::fs::read_to_string(&cargo) else {
        return Ok(BumpOutcome {
            bumped_cargo_toml: false,
            updated_cargo_lock: false,
            bumped_workspace_package: false,
        });
    };

    // Prefer workspace versioning when present.
    let (next, new_ver, bumped_section) = if let Some((next, v)) = bump_in_section(&content, "workspace.package") {
        (next, v, "workspace.package")
    } else if let Some((next, v)) = bump_in_section(&content, "package") {
        (next, v, "package")
    } else {
        return Ok(BumpOutcome {
            bumped_cargo_toml: false,
            updated_cargo_lock: false,
            bumped_workspace_package: false,
        });
    };

    if next == content {
        return Ok(BumpOutcome {
            bumped_cargo_toml: false,
            updated_cargo_lock: false,
            bumped_workspace_package: false,
        });
    }

    std::fs::write(&cargo, next).with_context(|| format!("failed writing {}", cargo.display()))?;

    // Keep Cargo.lock consistent for single-package repos: if we can find the package name in
    // the same bumped section (typically [package]), update the matching lock entry's version.
    let mut updated_cargo_lock = false;
    if bumped_section == "package" && !new_ver.is_empty() {
        if let Some(name) = find_package_name(&content, "package") {
            match update_cargo_lock_package_version(repo, &name, &new_ver) {
                Ok(changed) => updated_cargo_lock = changed,
                Err(e) => eprintln!("⚠️ failed to update Cargo.lock for {}: {}", repo.display(), e),
            }
        }
    }

    Ok(BumpOutcome {
        bumped_cargo_toml: true,
        updated_cargo_lock,
        bumped_workspace_package: bumped_section == "workspace.package",
    })
}

#[derive(Debug, Clone, Copy)]
struct SimpleBumpOutcome {
    bumped: bool,
    updated_lock: bool,
}

fn bump_node_package_version_in_repo(repo: &Path) -> Result<SimpleBumpOutcome> {
    let pkg = repo.join("package.json");
    let Ok(content) = std::fs::read_to_string(&pkg) else {
        return Ok(SimpleBumpOutcome {
            bumped: false,
            updated_lock: false,
        });
    };
    let Some((next, old_ver, new_ver)) = bump_first_json_string_field(&content, "version") else {
        return Ok(SimpleBumpOutcome {
            bumped: false,
            updated_lock: false,
        });
    };
    if next != content {
        std::fs::write(&pkg, next).with_context(|| format!("failed writing {}", pkg.display()))?;
    }

    // Best-effort: keep package-lock.json root version aligned if it matches the old version.
    let mut updated_lock = false;
    let lock = repo.join("package-lock.json");
    if let Ok(lock_content) = std::fs::read_to_string(&lock) {
        if let Some(lock_next) =
            set_first_json_string_field_to_value(&lock_content, "version", &old_ver, &new_ver)
        {
            if lock_next != lock_content {
                std::fs::write(&lock, lock_next)
                    .with_context(|| format!("failed writing {}", lock.display()))?;
                updated_lock = true;
            }
        }
    }

    Ok(SimpleBumpOutcome {
        bumped: true,
        updated_lock,
    })
}

fn bump_version_file_in_repo(repo: &Path) -> Result<bool> {
    let p = repo.join("VERSION");
    let Ok(content) = std::fs::read_to_string(&p) else {
        return Ok(false);
    };
    let raw = content.trim();
    let Some(new_ver) = bump_semver_patch(raw) else {
        return Ok(false);
    };
    let next = format!("{}\n", new_ver);
    if next != content {
        std::fs::write(&p, next).with_context(|| format!("failed writing {}", p.display()))?;
        return Ok(true);
    }
    Ok(false)
}

async fn restore_paths(repo: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }

    // Prefer `git restore` (newer git). Fallback to `reset` + `checkout`.
    let mut args: Vec<String> = Vec::new();
    args.push("restore".to_string());
    args.push("--staged".to_string());
    args.push("--worktree".to_string());
    args.push("--".to_string());
    args.extend(paths.iter().cloned());
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    if run_git_with_timeout(repo, &args_ref, 30, "restore").await.is_ok() {
        return Ok(());
    }

    let mut reset: Vec<String> = Vec::new();
    reset.push("reset".to_string());
    reset.push("HEAD".to_string());
    reset.push("--".to_string());
    reset.extend(paths.iter().cloned());
    let reset_ref: Vec<&str> = reset.iter().map(|s| s.as_str()).collect();
    let _ = run_git_with_timeout(repo, &reset_ref, 30, "reset").await;

    let mut checkout: Vec<String> = Vec::new();
    checkout.push("checkout".to_string());
    checkout.push("--".to_string());
    checkout.extend(paths.iter().cloned());
    let checkout_ref: Vec<&str> = checkout.iter().map(|s| s.as_str()).collect();
    run_git_with_timeout(repo, &checkout_ref, 30, "checkout").await
}

fn default_exclude_dir_names() -> Vec<String> {
    [
        "target",
        "node_modules",
        ".cache",
        ".direnv",
        ".venv",
        "dist",
        "build",
        "archives",
        ".tmp-*",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_exclude_file_patterns() -> Vec<String> {
    [
        "*.log",
        "nohup.out",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_max_stage_file_bytes() -> u64 {
    100 * 1024 * 1024
}

fn default_pull_op_timeout_secs() -> u64 {
    30
}

fn default_push_op_timeout_secs() -> u64 {
    300
}

fn default_repo_sync_timeout_secs() -> u64 {
    420
}

fn default_push_retries() -> u32 {
    3
}

fn default_repair_cooldown_secs() -> u64 {
    60
}

fn default_max_push_blob_bytes() -> u64 {
    DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES
}

fn default_incident_ledger_max_lines() -> usize {
    10_000
}

fn default_incident_ledger_max_age_days() -> u64 {
    30
}

const DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepoFilter {
    All,
    Concern,
    Warn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConcernRepairFilter {
    All,
    StuckPush,
    StuckPull,
}

#[derive(Debug, Serialize)]
struct RepoReportRow {
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
struct RepoReportJson {
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
struct StatusJson {
    policy: String,
    roots: Vec<String>,
    repos_discovered: usize,
    pulse_interval_secs: u64,
    inactivity_push_delay_secs: u64,
    freeze: String,
    auto_commit: bool,
    auto_pull: bool,
    auto_push: bool,
    auto_bump_versions: bool,
    auto_repair_concerns: bool,
    auto_repair_warns: bool,
    auto_rewrite_large_blobs: bool,
    max_stage_file_bytes: u64,
    push_blob_threshold_bytes: u64,
    exclude_dirs: Vec<String>,
    exclude_file_patterns: Vec<String>,
    pull_op_timeout_secs: u64,
    push_op_timeout_secs: u64,
    repo_sync_timeout_secs: u64,
    push_retries: u32,
    repair_cooldown_secs: u64,
    incident_ledger_max_lines: usize,
    incident_ledger_max_age_days: u64,
    system_repo: String,
    backup_policy: String,
    backup_dir: String,
    extra_remotes: usize,
}

#[derive(Debug, Serialize)]
struct RepairJson {
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
struct RepairSummary {
    found: usize,
    planned: usize,
    attempted: usize,
    succeeded: usize,
    resolved_now: usize,
    manual_only: usize,
}

#[derive(Debug, Serialize)]
struct IncidentRecord {
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
    fn new(scope: &str, repo: &str, reason: &str, action: &str, result: &str) -> Self {
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

    fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    fn with_backup_branch(mut self, branch: &str) -> Self {
        self.backup_branch = Some(branch.to_string());
        self
    }
}

impl SyncPolicy {
    fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read policy {}", path.display()))?;
        let mut policy: Self = toml::from_str(&content)
            .with_context(|| format!("failed to parse policy {}", path.display()))?;
        if policy.exclude_dir_names.is_empty() {
            policy.exclude_dir_names = default_exclude_dir_names();
        }
        if policy.max_stage_file_bytes == 0 {
            policy.max_stage_file_bytes = default_max_stage_file_bytes();
        }
        if policy.pull_op_timeout_secs == 0 {
            policy.pull_op_timeout_secs = default_pull_op_timeout_secs();
        }
        if policy.push_op_timeout_secs == 0 {
            policy.push_op_timeout_secs = default_push_op_timeout_secs();
        }
        if policy.repo_sync_timeout_secs == 0 {
            policy.repo_sync_timeout_secs = default_repo_sync_timeout_secs();
        }
        if policy.push_retries == 0 {
            policy.push_retries = default_push_retries();
        }
        if policy.inactivity_push_delay_secs == 0 {
            policy.inactivity_push_delay_secs = default_inactivity_push_delay_secs();
        }
        if policy.repair_cooldown_secs == 0 {
            policy.repair_cooldown_secs = default_repair_cooldown_secs();
        }
        if policy.max_push_blob_bytes == 0 {
            policy.max_push_blob_bytes = default_max_push_blob_bytes();
        }
        if policy.incident_ledger_max_lines == 0 {
            policy.incident_ledger_max_lines = default_incident_ledger_max_lines();
        }
        if policy.incident_ledger_max_age_days == 0 {
            policy.incident_ledger_max_age_days = default_incident_ledger_max_age_days();
        }
        if policy.pull_op_timeout_secs < 5 {
            eprintln!("⚠️ pull_op_timeout_secs {} below minimum 5s, adjusting", policy.pull_op_timeout_secs);
            policy.pull_op_timeout_secs = 5;
        }
        if policy.push_op_timeout_secs < 10 {
            eprintln!("⚠️ push_op_timeout_secs {} below minimum 10s, adjusting", policy.push_op_timeout_secs);
            policy.push_op_timeout_secs = 10;
        }
        policy.max_push_blob_bytes = policy
            .max_push_blob_bytes
            .min(DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES)
            .max(1);
        policy.repo_sync_timeout_secs = policy.repo_sync_timeout_secs.max(
            policy
                .push_op_timeout_secs
                .saturating_add(30)
                .max(policy.pull_op_timeout_secs.saturating_add(30)),
        );
        Ok(policy)
    }

    fn watch_root_paths(&self) -> Vec<PathBuf> {
        self.watch_roots
            .iter()
            .map(PathBuf::from)
            .filter(|p| {
                if !p.exists() {
                    eprintln!("⚠️ watch root {} does not exist, skipping", p.display());
                    false
                } else {
                    true
                }
            })
            .collect()
    }
}

fn resolve_policy_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("home not found")?;
    dracon_common::resolve_policy_path(
        &["DRACON_SYNC_POLICY"],
        &[
            home.join(".dracon/utilities/sync/dracon-sync.toml"),
            home.join(".dracon/utilities/sync/config.toml"),
            home.join(".dracon/git/dracon-git.toml"),
        ],
        "sync policy not found",
    )
}

fn normalized_dir_name(value: &str) -> String {
    value.trim_matches('/').to_ascii_lowercase()
}

fn excluded_dir_names_set(policy: &SyncPolicy) -> BTreeSet<String> {
    policy
        .exclude_dir_names
        .iter()
        .map(|d| normalized_dir_name(d))
        .filter(|d| !d.is_empty())
        .collect()
}

fn is_excluded_dir_name(name: &str, excluded_dir_names: &BTreeSet<String>) -> bool {
    let normalized = normalized_dir_name(name);
    for pattern in excluded_dir_names {
        if *pattern == normalized {
            return true;
        }
        if pattern.ends_with('-') && pattern.starts_with('.') && normalized.starts_with(&pattern[..pattern.len()-1]) {
            return true;
        }
        if pattern.ends_with('*') && normalized.starts_with(&pattern[..pattern.len()-1]) {
            return true;
        }
    }
    false
}

fn is_excluded_change_path(path: &Path, excluded_dir_names: &BTreeSet<String>) -> bool {
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .any(|name| is_excluded_dir_name(name, excluded_dir_names))
}

fn matches_file_pattern(file_name: &str, pattern: &str) -> bool {
    if pattern == file_name {
        return true;
    }
    if pattern.starts_with("*.") {
        let ext = &pattern[1..];
        if file_name.ends_with(ext) {
            return true;
        }
    }
    if pattern.ends_with(".*") {
        let prefix = &pattern[..pattern.len() - 1];
        if file_name.starts_with(prefix) {
            return true;
        }
    }
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let (prefix, suffix) = (parts[0], parts[1]);
            if file_name.starts_with(prefix) && file_name.ends_with(suffix) {
                return true;
            }
        }
    }
    false
}

fn is_excluded_file(file_path: &Path, excluded_patterns: &[String]) -> bool {
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    for pattern in excluded_patterns {
        if matches_file_pattern(file_name, pattern) {
            return true;
        }
    }
    false
}
fn should_stage_entry(
    repo: &Path,
    entry: &dracon_git::types::DiffFile,
    excluded_dir_names: &BTreeSet<String>,
    excluded_file_patterns: &[String],
    max_stage_file_bytes: u64,
) -> bool {
    if matches!(entry.status, dracon_git::types::FileStatus::Deleted) {
        return true;
    }

    if is_excluded_change_path(&entry.path, excluded_dir_names) {
        return false;
    }

    if is_excluded_file(&entry.path, excluded_file_patterns) {
        return false;
    }

    // Submodules and directory type changes
    if matches!(entry.status, dracon_git::types::FileStatus::TypeChange) {
        return true;
    }

    let full_path = repo.join(&entry.path);
    match std::fs::metadata(&full_path) {
        Ok(meta) if meta.is_file() => {
            if meta.len() > max_stage_file_bytes {
                eprintln!(
                    "ℹ️ skip large file {} ({} bytes > {} bytes)",
                    full_path.display(),
                    meta.len(),
                    max_stage_file_bytes
                );
                return false;
            }
            true
        }
        Ok(meta) if meta.is_dir() => {
            // This is likely a submodule.
            true
        }
        Ok(_) => true,
        Err(_) => {
            // If it's a deleted file, metadata will fail, but we handled Deleted at the top.
            // For other cases (broken symlinks, etc), default to staging if not excluded.
            true
        }
    }
}

fn can_restore_entry(entry: &dracon_git::types::DiffFile) -> bool {
    use dracon_git::types::FileStatus;
    matches!(entry.status, FileStatus::Modified | FileStatus::TypeChange | FileStatus::Renamed)
}

fn is_large_untracked(entry: &dracon_git::types::DiffFile, repo: &Path, threshold: u64) -> bool {
    use dracon_git::types::FileStatus;
    if entry.status != FileStatus::Added {
        return false;
    }
    let full_path = repo.join(&entry.path);
    match std::fs::metadata(&full_path) {
        Ok(meta) if meta.is_file() => meta.len() > threshold,
        _ => false,
    }
}

fn append_to_gitignore(repo: &Path, patterns: &[String]) -> Result<()> {
    let gitignore = repo.join(".gitignore");
    let current = std::fs::read_to_string(&gitignore).unwrap_or_default();
    
    let mut lines: Vec<String> = current.lines().map(String::from).collect();
    let mut added = Vec::new();
    
    for pattern in patterns {
        let pattern_line = pattern.trim();
        if pattern_line.is_empty() || lines.iter().any(|l| l.trim() == pattern_line) {
            continue;
        }
        added.push(pattern_line.to_string());
    }
    
    if added.is_empty() {
        return Ok(());
    }
    
    // Check if there's a warden-managed block
    let block_begin_idx = lines.iter().position(|l| l.contains("--- BEGIN DRACON MANAGED BLOCK ---"));
    let block_end_idx = lines.iter().position(|l| l.contains("--- END DRACON MANAGED BLOCK ---"));
    
    if let (Some(begin_idx), Some(end_idx)) = (block_begin_idx, block_end_idx) {
        // Warden manages this .gitignore - insert patterns INSIDE the managed block
        // (before the END marker) so warden will preserve them
        let insert_at = end_idx;
        
        // Check if we already have a large files section inside the managed block
        let has_large_files_section = lines[begin_idx..end_idx]
            .iter()
            .any(|l| l.contains("# Large files (auto-added by dracon-sync)"));
        
        let mut to_insert = Vec::new();
        if !has_large_files_section {
            to_insert.push("# Large files (auto-added by dracon-sync)".to_string());
        }
        for pattern in &added {
            to_insert.push(pattern.clone());
        }
        
        // Insert before the END marker
        for (i, line) in to_insert.into_iter().enumerate() {
            lines.insert(insert_at + i, line);
        }
        
        let new_content = lines.join("\n");
        std::fs::write(&gitignore, new_content)?;
        
        eprintln!(
            "📝 added {} large file pattern(s) to .gitignore in {} (inside warden managed block)",
            added.len(),
            repo.display()
        );
        
        return Ok(());
    }
    
    // No warden block - we can safely append
    // Check if we already have a large files section
    let has_large_files_section = lines.iter().any(|l| l.contains("# Large files (auto-added by dracon-sync)"));
    
    // Build the new lines to append
    let mut to_append = Vec::new();
    if !has_large_files_section {
        to_append.push(String::new()); // blank line
        to_append.push("# Large files (auto-added by dracon-sync)".to_string());
    }
    for pattern in added {
        to_append.push(pattern);
    }
    
    // Append to the end
    lines.extend(to_append);
    
    let new_content = lines.join("\n");
    std::fs::write(&gitignore, new_content)?;
    
    Ok(())
}

/// Handle large untracked files by adding them to .gitignore.
/// Returns true if .gitignore was updated.
fn handle_large_untracked(
    repo: &Path,
    to_restore: &[dracon_git::types::DiffFile],
    policy: &SyncPolicy,
) -> Result<bool> {
    let large_untracked: Vec<_> = to_restore
        .iter()
        .filter(|e| is_large_untracked(e, repo, policy.max_stage_file_bytes))
        .collect();

    if large_untracked.is_empty() {
        return Ok(false);
    }

    let patterns: Vec<String> = large_untracked
        .iter()
        .map(|e| e.path.to_string_lossy().to_string())
        .collect();
    eprintln!(
        "📝 {} has {} large untracked file(s) > {} bytes - adding to .gitignore",
        repo.display(),
        patterns.len(),
        policy.max_stage_file_bytes
    );
    append_to_gitignore(repo, &patterns)?;
    Ok(true)
}

fn env_freeze_enabled() -> bool {
    matches!(
        std::env::var("DRACON_SYNC_FREEZE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn debug_enabled() -> bool {
    matches!(
        std::env::var("DRACON_SYNC_DEBUG")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn freeze_marker_paths(_policy_path: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    // Freeze markers are intentionally kept out of git-tracked repos to avoid accidental
    // perpetual DIRTY states and surprise "sync frozen" incidents.
    //
    // Canonical locations:
    // - ~/.dracon/dracon-sync.freeze
    // - ~/.dracon/freeze/dracon-sync
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".dracon").join("dracon-sync.freeze"));
        paths.push(home.join(".dracon").join("freeze").join("dracon-sync"));
    }
    paths
}

fn freeze_reason(policy_path: &Path) -> Option<String> {
    if env_freeze_enabled() {
        return Some("env DRACON_SYNC_FREEZE".to_string());
    }

    for marker in freeze_marker_paths(policy_path) {
        if marker.exists() {
            return Some(format!("marker {}", marker.display()));
        }
    }

    None
}

fn git_binary() -> &'static Path {
    static GIT_BIN: OnceLock<PathBuf> = OnceLock::new();
    GIT_BIN
        .get_or_init(|| {
            if let Ok(custom) = std::env::var("DRACON_SYNC_GIT_BIN") {
                let trimmed = custom.trim();
                if !trimmed.is_empty() {
                    return PathBuf::from(trimmed);
                }
            }

            for candidate in ["/run/current-system/sw/bin/git", "/usr/bin/git", "/bin/git"] {
                let path = PathBuf::from(candidate);
                if path.exists() {
                    return path;
                }
            }

            PathBuf::from("git")
        })
        .as_path()
}

fn std_git_command() -> StdCommand {
    StdCommand::new(git_binary())
}

fn tokio_git_command() -> TokioCommand {
    TokioCommand::new(git_binary())
}

fn acquire_daemon_lock() -> Result<File> {
    dracon_common::acquire_daemon_lock("dracon-sync")
}

#[derive(Debug, Clone)]
enum ReportSignal {
    ActiveBoardChanged,
    IndexChanged,
    BlueprintCreated,
    BlueprintModified,
}

fn detect_report_signals(
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

fn read_project_focus(repo: &Path) -> Option<String> {
    let state_path = repo.join(".dracon/project-state.md");
    let content = std::fs::read_to_string(&state_path).ok()?;
    
    let mut in_focus = false;
    let mut lines = Vec::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Enter focus section
        if trimmed.starts_with("## ") && trimmed.to_lowercase().contains("current focus") {
            in_focus = true;
            continue;
        }
        
        // Exit on next section
        if in_focus && trimmed.starts_with("## ") {
            break;
        }
        
        // Collect non-empty lines in focus section
        if in_focus && !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
    }
    
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn build_commit_context(
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

fn discover_git_repos(roots: &[PathBuf], excluded_dir_names: &BTreeSet<String>) -> Vec<PathBuf> {
    dracon_common::discover_git_repos(roots, excluded_dir_names)
}

fn has_origin_remote(repo: &Path) -> bool {
    std_git_command()
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_tracking_upstream(repo: &Path) -> bool {
    std_git_command()
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_rebase_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("rebase-merge").exists()
        || repo.join(".git").join("rebase-apply").exists()
}

fn is_merge_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("MERGE_HEAD").exists()
}

fn is_cherry_pick_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("CHERRY_PICK_HEAD").exists()
}

async fn kill_descendants(pid: u32) {
    let pid_s = pid.to_string();
    let _ = TokioCommand::new("pkill")
        .args(["-TERM", "-P", &pid_s])
        .output()
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = TokioCommand::new("pkill")
        .args(["-KILL", "-P", &pid_s])
        .output()
        .await;
}

async fn run_git_with_timeout(
    repo: &Path,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
) -> Result<()> {
    let mut child = tokio_git_command()
        .args(args)
        .current_dir(repo)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn git {} in {}", op_label, repo.display()))?;

    let pid = child.id();
    match tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait()).await {
        Ok(Ok(status)) => {
            if status.success() {
                return Ok(());
            }
            return Err(anyhow::anyhow!(
                "git {} failed in {} with status {}",
                op_label,
                repo.display(),
                status
            ));
        }
        Ok(Err(e)) => {
            return Err(anyhow::anyhow!(
                "git {} failed in {}: {}",
                op_label,
                repo.display(),
                e
            ));
        }
        Err(_) => {
            if let Some(pid) = pid {
                kill_descendants(pid).await;
            }
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(anyhow::anyhow!(
                "git {} timeout in {} after {}s",
                op_label,
                repo.display(),
                timeout_secs
            ));
        }
    }
}

async fn run_git_with_timeout_env(
    repo: &Path,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
    env: &[(&str, &str)],
) -> Result<()> {
    let mut cmd = tokio_git_command();
    cmd.args(args)
        .current_dir(repo)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn git {} in {}", op_label, repo.display()))?;

    let pid = child.id();
    match tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait()).await {
        Ok(Ok(status)) => {
            if status.success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "git {} failed in {} with status {}",
                    op_label,
                    repo.display(),
                    status
                ))
            }
        }
        Ok(Err(e)) => Err(anyhow::anyhow!(
            "git {} failed in {}: {}",
            op_label,
            repo.display(),
            e
        )),
        Err(_) => {
            if let Some(pid) = pid {
                kill_descendants(pid).await;
            }
            let _ = child.start_kill();
            let _ = child.wait().await;
            Err(anyhow::anyhow!(
                "git {} timeout in {} after {}s",
                op_label,
                repo.display(),
                timeout_secs
            ))
        }
    }
}

async fn run_cmd_with_timeout(
    repo: &Path,
    program: &str,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
) -> Result<()> {
    let mut child = TokioCommand::new(program)
        .args(args)
        .current_dir(repo)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "failed to spawn {} {} in {}",
                program,
                op_label,
                repo.display()
            )
        })?;

    let pid = child.id();
    match tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait()).await {
        Ok(Ok(status)) => {
            if status.success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "{} {} failed in {} with status {}",
                    program,
                    op_label,
                    repo.display(),
                    status
                ))
            }
        }
        Ok(Err(e)) => Err(anyhow::anyhow!(
            "{} {} failed in {}: {}",
            program,
            op_label,
            repo.display(),
            e
        )),
        Err(_) => {
            if let Some(pid) = pid {
                kill_descendants(pid).await;
            }
            let _ = child.start_kill();
            let _ = child.wait().await;
            Err(anyhow::anyhow!(
                "{} {} timeout in {} after {}s",
                program,
                op_label,
                repo.display(),
                timeout_secs
            ))
        }
    }
}

fn origin_url(repo: &Path) -> Option<String> {
    let out = std_git_command()
        .args(["remote", "get-url", "origin"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if url.is_empty() {
        None
    } else {
        Some(url)
    }
}

fn strip_url_credentials(url: &str) -> String {
    if let Some(stripped) = url.strip_prefix("https://") {
        if let Some(at_pos) = stripped.find('@') {
            return format!("https://{}", &stripped[at_pos + 1..]);
        }
    }
    url.to_string()
}

fn github_https_url(origin: &str) -> Option<String> {
    if let Some(rest) = origin.strip_prefix("git@github.com:") {
        return Some(format!("https://github.com/{}", rest));
    }
    if let Some(rest) = origin.strip_prefix("ssh://git@github.com/") {
        return Some(format!("https://github.com/{}", rest));
    }
    if origin.starts_with("https://github.com/") {
        return Some(strip_url_credentials(origin));
    }
    None
}

async fn push_with_transport_fallbacks(
    repo: &Path,
    timeout_secs: u64,
    op_label: &str,
) -> Result<()> {
    let ssh_hardening = "ssh -o ConnectTimeout=10 -o ConnectionAttempts=1 -o ServerAliveInterval=5 -o ServerAliveCountMax=2";
    match run_git_with_timeout_env(
        repo,
        &["push", "origin", "HEAD"],
        timeout_secs,
        &format!("{op_label}-ssh-hardened"),
        &[("GIT_SSH_COMMAND", ssh_hardening)],
    )
    .await
    {
        Ok(()) => return Ok(()),
        Err(e) => {
            let origin = origin_url(repo).unwrap_or_default();
            if let Some(https) = github_https_url(&origin) {
                let branch = current_branch(repo).unwrap_or_else(|| "master".to_string());
                let refspec = format!("HEAD:refs/heads/{branch}");
                run_git_with_timeout(
                    repo,
                    &["push", &https, &refspec],
                    timeout_secs,
                    &format!("{op_label}-https-fallback"),
                )
                .await
                .with_context(|| format!("ssh fallback failed first: {}", e))
            } else {
                Err(e)
            }
        }
    }
}

async fn push_with_retries(
    repo: &Path,
    timeout_secs: u64,
    retries: u32,
    op_label: &str,
) -> Result<()> {
    let attempts = retries.max(1);
    let mut last_err: Option<anyhow::Error> = None;
    let mut timeout_seen = false;
    for attempt in 1..=attempts {
        match run_git_with_timeout(repo, &["push", "origin", "HEAD"], timeout_secs, op_label).await
        {
            Ok(()) => return Ok(()),
            Err(e) => {
                let err_text = e.to_string();
                let is_timeout = err_text.contains("timeout");
                timeout_seen |= is_timeout;
                last_err = Some(e);
                if attempt < attempts && is_timeout {
                    let backoff = (attempt as u64).min(5);
                    eprintln!(
                        "⏱️ push retry {}/{} for {} after {}s",
                        attempt + 1,
                        attempts,
                        repo.display(),
                        backoff
                    );
                    sleep(Duration::from_secs(backoff)).await;
                    continue;
                }
                break;
            }
        }
    }
    if timeout_seen {
        if let Ok(()) = push_with_transport_fallbacks(repo, timeout_secs, op_label).await {
            return Ok(());
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("push failed")))
}

fn run_git_capture_output(repo: &Path, args: &[&str], op_label: &str) -> Result<String> {
    let output = std_git_command()
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed to run git {} in {}", op_label, repo.display()))?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(text)
}

async fn git_list_paths(repo: &Path, args: &[&str]) -> Result<Vec<PathBuf>> {
    let output = tokio_git_command()
        .args(args)
        .current_dir(repo)
        .output()
        .await
        .with_context(|| format!("failed to run git {:?} in {}", args, repo.display()))?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect())
}

fn parse_name_status_line(line: &str) -> Option<(PathBuf, dracon_git::types::FileStatus)> {
    let mut parts = line.split('\t');
    let status_raw = parts.next()?.trim();
    if status_raw.is_empty() {
        return None;
    }
    let status_char = status_raw.chars().next()?;
    let (path, status) = match status_char {
        'M' => (parts.next()?, dracon_git::types::FileStatus::Modified),
        'A' => (parts.next()?, dracon_git::types::FileStatus::Added),
        'D' => (parts.next()?, dracon_git::types::FileStatus::Deleted),
        'T' => (parts.next()?, dracon_git::types::FileStatus::TypeChange),
        'R' => {
            let _old = parts.next()?;
            let new = parts.next()?;
            (new, dracon_git::types::FileStatus::Renamed)
        }
        _ => return None,
    };
    Some((PathBuf::from(path.trim()), status))
}

async fn git_name_status_entries(
    repo: &Path,
    args: &[&str],
) -> Result<Vec<(PathBuf, dracon_git::types::FileStatus)>> {
    let output = tokio_git_command()
        .args(args)
        .current_dir(repo)
        .output()
        .await
        .with_context(|| format!("failed to run git {:?} in {}", args, repo.display()))?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter_map(parse_name_status_line)
        .collect::<Vec<_>>())
}

fn fallback_status_rank(status: &dracon_git::types::FileStatus) -> u8 {
    match status {
        dracon_git::types::FileStatus::Deleted => 5,
        dracon_git::types::FileStatus::Renamed => 4,
        dracon_git::types::FileStatus::TypeChange => 3,
        dracon_git::types::FileStatus::Added => 2,
        dracon_git::types::FileStatus::Modified => 1,
        dracon_git::types::FileStatus::Unknown => 0,
    }
}

async fn cli_diff_entries(repo: &Path) -> Result<Vec<dracon_git::types::DiffFile>> {
    let mut entries: BTreeMap<PathBuf, dracon_git::types::FileStatus> = BTreeMap::new();

    for args in [
        &["diff", "--name-status"][..],
        &["diff", "--cached", "--name-status"][..],
    ] {
        for (path, status) in git_name_status_entries(repo, args).await? {
            let should_replace = entries
                .get(&path)
                .map(|old| fallback_status_rank(&status) >= fallback_status_rank(old))
                .unwrap_or(true);
            if should_replace {
                entries.insert(path, status);
            }
        }
    }

    for path in git_list_paths(repo, &["ls-files", "--others", "--exclude-standard"]).await? {
        let should_replace = entries
            .get(&path)
            .map(|old| fallback_status_rank(&dracon_git::types::FileStatus::Added) >= fallback_status_rank(old))
            .unwrap_or(true);
        if should_replace {
            entries.insert(path, dracon_git::types::FileStatus::Added);
        }
    }

    Ok(entries
        .into_iter()
        .map(|(path, status)| dracon_git::types::DiffFile {
            path,
            status,
        })
        .collect())
}

async fn repo_diff_entries(repo: &Path) -> Result<Vec<dracon_git::types::DiffFile>> {
    let svc = GitService::new(repo)?;
    let mut entries = svc.get_diff_entries().await?;
    if entries.is_empty() {
        let fallback_entries = cli_diff_entries(repo).await?;
        if !fallback_entries.is_empty() {
            entries = fallback_entries;
        }
    }
    Ok(entries)
}

fn has_sync_relevant_dirty_entries(
    repo: &Path,
    entries: &[dracon_git::types::DiffFile],
    excluded_dir_names: &BTreeSet<String>,
    excluded_file_patterns: &[String],
    max_stage_file_bytes: u64,
) -> bool {
    entries.iter().any(|entry| {
        should_stage_entry(
            repo,
            entry,
            excluded_dir_names,
            excluded_file_patterns,
            max_stage_file_bytes,
        ) || can_restore_entry(entry)
            || is_large_untracked(entry, repo, max_stage_file_bytes)
    })
}

async fn staged_paths(repo: &Path) -> Result<Vec<PathBuf>> {
    git_list_paths(repo, &["diff", "--cached", "--name-only"]).await
}

async fn unstage_excluded_paths(
    repo: &Path,
    excluded_dir_names: &BTreeSet<String>,
) -> Result<usize> {
    let staged = staged_paths(repo).await?;
    let mut removed = 0usize;
    for path in staged {
        if !is_excluded_change_path(&path, excluded_dir_names) {
            continue;
        }
        let status = tokio_git_command()
            .args(["reset", "-q", "HEAD", "--"])
            .arg(&path)
            .current_dir(repo)
            .status()
            .await
            .with_context(|| {
                format!("failed to unstage {} in {}", path.display(), repo.display())
            })?;
        if status.success() {
            removed += 1;
        }
    }
    Ok(removed)
}

async fn unstage_oversized_paths(repo: &Path, max_stage_file_bytes: u64) -> Result<usize> {
    let staged = staged_paths(repo).await?;
    let mut removed = 0usize;
    for path in staged {
        let full = repo.join(&path);
        let meta = match std::fs::metadata(&full) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() || meta.len() <= max_stage_file_bytes {
            continue;
        }
        let status = tokio_git_command()
            .args(["reset", "-q", "HEAD", "--"])
            .arg(&path)
            .current_dir(repo)
            .status()
            .await
            .with_context(|| {
                format!(
                    "failed to unstage oversized path {} in {}",
                    path.display(),
                    repo.display()
                )
            })?;
        if status.success() {
            removed += 1;
            eprintln!(
                "🧹 removed oversized staged path {} ({} bytes)",
                full.display(),
                meta.len()
            );
        }
    }
    Ok(removed)
}

fn current_branch(repo: &Path) -> Option<String> {
    std_git_command()
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
}

fn remote_branch_exists(repo: &Path, branch: &str) -> bool {
    std_git_command()
        .args(["show-ref", "--verify", "--quiet"])
        .arg(format!("refs/remotes/origin/{branch}"))
        .current_dir(repo)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn set_upstream_to_branch(repo: &Path, branch: &str) -> Result<()> {
    let target = format!("origin/{branch}");
    let status = std_git_command()
        .args(["branch", "--set-upstream-to"])
        .arg(&target)
        .arg(branch)
        .current_dir(repo)
        .status()
        .with_context(|| format!("failed to set upstream for {}", repo.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "set-upstream failed for {} -> {}",
            repo.display(),
            target
        ))
    }
}

fn detect_large_blobs_ahead(repo: &Path, min_bytes: u64) -> Result<Vec<(u64, String)>> {
    // Step 1: Get object IDs from commits ahead of upstream
    let rev_list = std_git_command()
        .args(["rev-list", "--objects", "@{u}..HEAD"])
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed rev-list in {}", repo.display()))?;
    if !rev_list.status.success() {
        return Ok(Vec::new());
    }

    // Step 2: Batch-check object types and sizes (no shell involved)
    let mut cat_file = std_git_command()
        .args(["cat-file", "--batch-check=%(objectname) %(objecttype) %(objectsize) %(rest)"])
        .current_dir(repo)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed cat-file in {}", repo.display()))?;

    if let Some(mut stdin) = cat_file.stdin.take() {
        use std::io::Write;
        stdin.write_all(&rev_list.stdout)?;
    }
    let output = cat_file.wait_with_output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    // Step 3: Filter blobs > min_bytes in Rust (no shell, no awk)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut out: Vec<(u64, String)> = stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let _oid = parts.next()?;
            let obj_type = parts.next()?;
            let size_str = parts.next()?;
            let path = parts.next()?;
            if obj_type == "blob" {
                let size = size_str.parse::<u64>().ok()?;
                if size > min_bytes {
                    return Some((size, path.to_string()));
                }
            }
            None
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(out)
}

fn top_level_dir(path: &str) -> Option<String> {
    path.split('/').next().map(|s| s.to_string())
}

fn timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn incident_ledger_path(_policy_path: &Path) -> PathBuf {
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

fn append_incident_record(policy_path: &Path, record: &IncidentRecord) {
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

// ─── AI Scribe (feature-gated) ──────────────────────────────────────────
// Integrated into sync flow: called after each commit to update project-state.md.
// Uses reqwest directly — no dracon-ai binary, no routing runtime.

#[cfg(feature = "scribe")]
async fn update_project_state_from_ai(repo: &Path) -> anyhow::Result<()> {
    use anyhow::Context;
    use std::process::Command as StdCommand;

    // Collect git context
    let git_log = StdCommand::new("git")
        .args(["log", "--format=%s", "-20"])
        .current_dir(repo)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let git_files = StdCommand::new("git")
        .args(["log", "--oneline", "--name-only", "-10"])
        .current_dir(repo)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let blueprint = dracon_git::read_blueprint_content(repo);

    // Resolve AI provider from config
    let resolved = ai_runtime_config::resolve_ai_runtime_config();
    let provider = match resolved.openai_providers.iter()
        .find(|p| !p.api_keys.is_empty() && !p.api_keys[0].is_empty())
    {
        Some(p) => p,
        None => anyhow::bail!("no AI provider configured"),
    };

    let prompt = format!(
        "You are a scribe. Analyze git history and write a concise project-state.md.\n\n\
         ## Recent Git Log\n{}\n\n## File Changes\n{}\n\n## Blueprint\n{}\n\n\
         Write EXACTLY this format:\n\
         # Project State\n\n## Current Focus\n{{one line}}\n\n\
         ## Completed\n- [x] {{done}}\n\n## In Progress\n- [ ] {{active}}\n\n\
         ## Open Issues\n- {{blockers}}\n\n\
         Be factual. Infer from evidence.",
        git_log, git_files, blueprint
    );

    let body = serde_json::json!({
        "model": &provider.payload_model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 1000,
        "temperature": 0.3,
    });

    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", provider.endpoint.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .header(
            &provider.auth_header_name,
            format!("{}{}", provider.auth_header_prefix, provider.api_keys[0]),
        )
        .json(&body)
        .send()
        .await
        .with_context(|| "AI scribe request")?;

    if !resp.status().is_success() {
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            anyhow::bail!("auth failed (check ~/.dracon/ai/secrets/)");
        }
        anyhow::bail!("AI returned {}", resp.status());
    }

    let json: serde_json::Value = resp.json().await?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("AI response missing content"))?;

    let markdown = if let Some(start) = content.find("# Project State") {
        content[start..].trim()
    } else {
        content.trim()
    };

    let state_path = repo.join(".dracon/project-state.md");
    std::fs::create_dir_all(repo.join(".dracon"))?;
    std::fs::write(&state_path, markdown)
        .with_context(|| format!("writing {}", state_path.display()))?;
    eprintln!("📝 scribe: updated {}", state_path.display());

    Ok(())
}

fn repo_state_flags(
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

fn repo_is_concern(status: &dracon_git::types::RepoStatus, has_origin: bool, has_upstream: bool) -> bool {
    status.ahead > 0 || status.behind > 0 || !has_origin || (has_origin && !has_upstream)
}

fn repo_is_warn(status: &dracon_git::types::RepoStatus, has_origin: bool, has_upstream: bool) -> bool {
    !repo_is_concern(status, has_origin, has_upstream) && !status.is_clean
}

fn repo_hint(flags: &[String], warn: bool, concern: bool) -> String {
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

fn push_large_blob_threshold_bytes(policy: &SyncPolicy) -> u64 {
    policy
        .max_stage_file_bytes
        .min(policy.max_push_blob_bytes)
        .min(DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES)
}

fn rewrite_ahead_paths(
    repo: &Path,
    paths_to_remove: &[String],
    backup_prefix: &str,
) -> Result<Option<String>> {
    if paths_to_remove.is_empty() {
        return Ok(None);
    }
    let backup_branch = format!("{backup_prefix}-{}", timestamp_secs());
    let create_backup = std_git_command()
        .args(["branch", &backup_branch])
        .current_dir(repo)
        .status()
        .with_context(|| format!("failed backup branch in {}", repo.display()))?;
    if !create_backup.success() {
        return Err(anyhow::anyhow!(
            "failed to create backup branch {} in {}",
            backup_branch,
            repo.display()
        ));
    }

    // Try git-filter-repo first (preferred, faster, actively maintained)
    let filter_repo_available = std_git_command()
        .args(["filter-repo", "--version"])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if filter_repo_available {
        let mut args: Vec<String> = vec![
            "filter-repo".to_string(),
            "--invert-paths".to_string(),
            "--force".to_string(),
        ];
        for path in paths_to_remove {
            args.push("--path".to_string());
            args.push(path.clone());
        }
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let rewrite = std_git_command()
            .args(&args_ref)
            .current_dir(repo)
            .status()
            .with_context(|| format!("failed filter-repo in {}", repo.display()))?;
        if !rewrite.success() {
            return Err(anyhow::anyhow!(
                "filter-repo failed in {} (backup: {})",
                repo.display(),
                backup_branch
            ));
        }
        return Ok(Some(backup_branch));
    }

    // Fallback to deprecated git filter-branch
    eprintln!(
        "⚠️ git-filter-repo not found, using deprecated filter-branch. Install git-filter-repo for better performance."
    );
    let mut index_filter = String::from("git rm -r --cached --ignore-unmatch");
    for path in paths_to_remove {
        index_filter.push_str(" '");
        index_filter.push_str(&path.replace('\'', "'\\''"));
        index_filter.push('\'');
    }

    let rewrite = std_git_command()
        .args([
            "filter-branch",
            "--force",
            "--index-filter",
            &index_filter,
            "--prune-empty",
            "@{u}..HEAD",
        ])
        .current_dir(repo)
        .status()
        .with_context(|| format!("failed history rewrite in {}", repo.display()))?;
    if !rewrite.success() {
        return Err(anyhow::anyhow!(
            "history rewrite failed in {} (backup: {})",
            repo.display(),
            backup_branch
        ));
    }

    Ok(Some(backup_branch))
}

async fn sync_repo(
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
            
            // Scribe: update project-state.md via AI (if configured)
            if cfg!(feature = "scribe") {
                #[cfg(feature = "scribe")]
                let _ = update_project_state_from_ai(repo).await;
            }
            
            // Restore any excluded modified paths that weren't committed
            let restorable: Vec<_> = to_restore.iter().filter(|e| can_restore_entry(e)).collect();
            let large_untracked: Vec<_> = to_restore
                .iter()
                .filter(|e| is_large_untracked(e, repo, policy.max_stage_file_bytes))
                .collect();
            
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
                    Err(e) => eprintln!("⚠️ push skipped for {}: {}", repo.display(), e),
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

async fn run_once(policy_path: &Path) -> Result<()> {
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
            Ok(Err(e)) => eprintln!("⚠️ sync failed for {}: {}", repo.display(), e),
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

async fn run_daemon(policy_path: PathBuf) -> Result<()> {
    let _lock = acquire_daemon_lock()?;
    
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
                effective_dirty || status.ahead > 0 || status.behind > 0;
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

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let shortened: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", shortened)
}

fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
}

fn paint(value: &str, code: &str) -> String {
    if colors_enabled() {
        format!("\x1b[{}m{}\x1b[0m", code, value)
    } else {
        value.to_string()
    }
}

async fn git_log_field(repo: &Path, format: &str) -> Option<String> {
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

async fn git_log_unix_timestamp(repo: &Path) -> Option<i64> {
    git_log_field(repo, "%ct")
        .await
        .and_then(|s| s.parse::<i64>().ok())
}

async fn run_repos_report(policy_path: &Path, filter: RepoFilter, json: bool) -> Result<()> {
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
                    paint("❌", "31"),
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
                    paint("❌", "31"),
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
        paint("OK", "32"),
        ok_count,
        paint("WARN", "33"),
        warn_count,
        paint("CONCERN", "31"),
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
            paint("CONCERN", "31")
        } else if row.warn {
            paint("WARN", "33")
        } else {
            paint("OK", "32")
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

async fn run_repair_concerns(
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

async fn run_repair_warns(
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
            sync_repo(&repo, &policy, &excluded_dir_names, 0),
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

fn open_policy_in_editor(policy_path: &Path) -> Result<()> {
    let mut editors = Vec::new();
    if let Ok(visual) = std::env::var("VISUAL") {
        if !visual.trim().is_empty() {
            editors.push(visual);
        }
    }
    if let Ok(editor) = std::env::var("EDITOR") {
        if !editor.trim().is_empty() {
            editors.push(editor);
        }
    }
    for fallback in ["nvim", "vim", "nano", "vi"] {
        editors.push(fallback.to_string());
    }

    for editor in editors {
        match StdCommand::new(editor.trim()).arg(policy_path).status() {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => {
                return Err(anyhow::anyhow!(
                    "editor exited non-zero ({}). policy: {}",
                    status,
                    policy_path.display()
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "failed to launch editor '{}' for {}: {}",
                    editor,
                    policy_path.display(),
                    e
                ));
            }
        }
    }

    Err(anyhow::anyhow!(
        "no editor available. set VISUAL or EDITOR to open {}",
        policy_path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let unique = format!(
                "{}_{}_{}",
                prefix,
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time")
                    .as_nanos()
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

fn test_policy() -> SyncPolicy {
        SyncPolicy {
            system_repo: String::new(),
            pulse_interval_secs: 5,
            inactivity_push_delay_secs: 3,
            auto_commit: true,
            auto_pull: true,
            auto_push: true,
            auto_bump_versions: true,
            backup_policy: String::new(),
            backup_dir: String::new(),
            watch_roots: vec![],
            extra_remotes: HashMap::new(),
            exclude_dir_names: vec!["target".into(), "node_modules".into()],
            exclude_file_patterns: vec!["events.jsonl".into()],
            max_stage_file_bytes: 1024,
            pull_op_timeout_secs: 10,
            push_op_timeout_secs: 10,
            repo_sync_timeout_secs: 40,
            auto_repair_concerns: true,
            auto_repair_warns: true,
            auto_rewrite_large_blobs: false,
            push_retries: 2,
            repair_cooldown_secs: 60,
            max_push_blob_bytes: DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES,
            incident_ledger_max_lines: 1000,
            incident_ledger_max_age_days: 30,
        }
    }

    fn mk_status(
        is_clean: bool,
        ahead: usize,
        behind: usize,
        modified_files: usize,
        staged_files: usize,
    ) -> dracon_git::types::RepoStatus {
        dracon_git::types::RepoStatus {
            branch: "master".to_string(),
            ahead,
            behind,
            modified_files,
            staged_files,
            is_clean,
            last_commit_msg: None,
            last_commit_hash: None,
        }
    }

    #[test]
    fn defaults_are_stable() {
        assert!(default_true());
        assert_eq!(default_pulse_interval(), 1);
        assert_eq!(default_inactivity_push_delay_secs(), 5);
        assert!(default_exclude_dir_names().contains(&"target".to_string()));
        assert_eq!(default_max_stage_file_bytes(), 100 * 1024 * 1024);
        assert_eq!(default_pull_op_timeout_secs(), 30);
        assert_eq!(default_push_op_timeout_secs(), 300);
        assert_eq!(default_repo_sync_timeout_secs(), 420);
        assert_eq!(default_repair_cooldown_secs(), 60);
        assert_eq!(default_max_push_blob_bytes(), DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES);
        assert_eq!(default_incident_ledger_max_lines(), 10_000);
        assert_eq!(default_incident_ledger_max_age_days(), 30);
    }

    #[test]
    fn normalized_dir_name_handles_wrapping_and_case() {
        assert_eq!(normalized_dir_name("/Target/"), "target");
        assert_eq!(normalized_dir_name("Node_Modules"), "node_modules");
        assert_eq!(normalized_dir_name(""), "");
    }

    #[test]
    fn excluded_checks_work() {
        let mut p = test_policy();
        p.exclude_dir_names = vec!["Target".into(), "build".into()];
        let set = excluded_dir_names_set(&p);
        assert!(is_excluded_dir_name("target", &set));
        assert!(is_excluded_dir_name("TARGET", &set));
        assert!(is_excluded_change_path(Path::new("a/Build/x.txt"), &set));
        assert!(!is_excluded_change_path(Path::new("a/src/x.txt"), &set));
    }

    #[test]
    fn excluded_pattern_matching_works() {
        let mut p = test_policy();
        p.exclude_dir_names = vec![".tmp-*".into(), "cache-*".into()];
        let set = excluded_dir_names_set(&p);
        assert!(is_excluded_dir_name(".tmp-dracon-code-patch-123", &set));
        assert!(is_excluded_dir_name(".tmp-anything", &set));
        assert!(!is_excluded_dir_name(".tmp", &set));
        assert!(is_excluded_dir_name("cache-redis", &set));
        assert!(is_excluded_dir_name("cache-local", &set));
        assert!(!is_excluded_dir_name("cache", &set));
    }

    #[test]
    fn should_stage_entry_respects_rules() {
        let td = TempDir::new("sync_should_stage");
        let repo = td.path().join("repo");
        std::fs::create_dir_all(&repo).expect("repo");
        let excluded = BTreeSet::from(["target".to_string()]);
        let no_file_patterns: Vec<String> = Vec::new();

        let deleted = dracon_git::types::DiffFile {
            path: PathBuf::from("target/missing.bin"),
            status: dracon_git::types::FileStatus::Deleted,
        };
        assert!(should_stage_entry(&repo, &deleted, &excluded, &no_file_patterns, 10));

        let excluded_file = dracon_git::types::DiffFile {
            path: PathBuf::from("target/file.bin"),
            status: dracon_git::types::FileStatus::Modified,
        };
        assert!(!should_stage_entry(&repo, &excluded_file, &excluded, &no_file_patterns, 10));

        let big_path = repo.join("big.bin");
        std::fs::write(&big_path, vec![1u8; 64]).expect("write big");
        let big = dracon_git::types::DiffFile {
            path: PathBuf::from("big.bin"),
            status: dracon_git::types::FileStatus::Modified,
        };
        assert!(!should_stage_entry(&repo, &big, &BTreeSet::new(), &no_file_patterns, 16));

        let missing = dracon_git::types::DiffFile {
            path: PathBuf::from("gone.bin"),
            status: dracon_git::types::FileStatus::Modified,
        };
        assert!(should_stage_entry(&repo, &missing, &BTreeSet::new(), &no_file_patterns, 16));
    }

    #[test]
    fn should_stage_entry_excludes_file_patterns() {
        let td = TempDir::new("sync_should_stage_patterns");
        let repo = td.path().join("repo");
        std::fs::create_dir_all(&repo).expect("repo");
        let no_excluded_dirs: BTreeSet<String> = BTreeSet::new();
        let excluded_patterns = vec!["events.jsonl".to_string(), "*.log".to_string()];

        let events_file = dracon_git::types::DiffFile {
            path: PathBuf::from("plan/events.jsonl"),
            status: dracon_git::types::FileStatus::Modified,
        };
        assert!(!should_stage_entry(&repo, &events_file, &no_excluded_dirs, &excluded_patterns, 10));

        let log_file = dracon_git::types::DiffFile {
            path: PathBuf::from("debug.log"),
            status: dracon_git::types::FileStatus::Modified,
        };
        assert!(!should_stage_entry(&repo, &log_file, &no_excluded_dirs, &excluded_patterns, 10));

        let normal_file = dracon_git::types::DiffFile {
            path: PathBuf::from("src/main.rs"),
            status: dracon_git::types::FileStatus::Modified,
        };
        assert!(should_stage_entry(&repo, &normal_file, &no_excluded_dirs, &excluded_patterns, 10));
    }

    #[test]
    fn parse_name_status_line_maps_deleted_and_rename() {
        let deleted = parse_name_status_line("D\tplans/old.md").expect("deleted parsed");
        assert_eq!(deleted.0, PathBuf::from("plans/old.md"));
        assert!(matches!(
            deleted.1,
            dracon_git::types::FileStatus::Deleted
        ));

        let renamed =
            parse_name_status_line("R100\tplans/old.md\tplans/new.md").expect("rename parsed");
        assert_eq!(renamed.0, PathBuf::from("plans/new.md"));
        assert!(matches!(
            renamed.1,
            dracon_git::types::FileStatus::Renamed
        ));
    }

    #[test]
    fn fallback_status_rank_prefers_deleted() {
        use dracon_git::types::FileStatus;
        assert!(
            fallback_status_rank(&FileStatus::Deleted) > fallback_status_rank(&FileStatus::Added)
        );
        assert!(
            fallback_status_rank(&FileStatus::Deleted)
                > fallback_status_rank(&FileStatus::Modified)
        );
    }

    #[test]
    fn freeze_helpers_work() {
        let _guard = env_lock().lock().expect("lock");
        std::env::remove_var("DRACON_SYNC_FREEZE");

        let td = TempDir::new("sync_freeze");
        let policy = td.path().join("dracon-sync.toml");
        std::fs::write(&policy, "").expect("policy");

        let markers = freeze_marker_paths(&policy);
        assert_eq!(markers.len(), 2);
        let as_text = markers
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>();
        assert!(as_text.iter().any(|s| s.ends_with(".dracon/dracon-sync.freeze")));
        assert!(as_text.iter().any(|s| s.ends_with(".dracon/freeze/dracon-sync")));

        // Baseline can be frozen depending on the developer machine (marker files may exist).
        let baseline = freeze_reason(&policy);

        std::env::set_var("DRACON_SYNC_FREEZE", "1");
        assert_eq!(
            freeze_reason(&policy).as_deref(),
            Some("env DRACON_SYNC_FREEZE")
        );
        std::env::remove_var("DRACON_SYNC_FREEZE");
        assert_eq!(freeze_reason(&policy), baseline);
    }

    #[test]
    fn truncate_and_paint_behave() {
        assert_eq!(truncate("short", 10), "short");
        assert!(truncate("very long value", 8).ends_with('…'));
        let _guard = env_lock().lock().expect("lock");
        std::env::set_var("NO_COLOR", "1");
        assert_eq!(paint("x", "31"), "x");
        std::env::remove_var("NO_COLOR");
    }

    #[test]
    fn discover_git_repos_finds_and_excludes() {
        let td = TempDir::new("sync_discover");
        let root = td.path().join("root");
        std::fs::create_dir_all(root.join("repo-a/.git")).expect("repo-a");
        std::fs::create_dir_all(root.join("target/repo-b/.git")).expect("repo-b");
        std::fs::create_dir_all(root.join("nested/repo-c/.git")).expect("repo-c");
        let excluded = BTreeSet::from(["target".to_string()]);

        let repos = discover_git_repos(&[root], &excluded);
        let as_text = repos
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>();
        assert!(as_text.iter().any(|s| s.contains("repo-a")));
        assert!(as_text.iter().any(|s| s.contains("repo-c")));
        assert!(!as_text.iter().any(|s| s.contains("repo-b")));
    }

    #[test]
    fn normalization_scenarios_repeated() {
        for i in 0..240usize {
            let input = if i % 3 == 0 {
                format!("/TaRgEt/{i}/")
            } else if i % 3 == 1 {
                format!("NODE_MODULES/{i}")
            } else {
                format!("build/{i}")
            };
            let out = normalized_dir_name(&input);
            assert_eq!(out, out.to_ascii_lowercase());
            assert!(!out.starts_with('/'));
            assert!(!out.ends_with('/'));
        }
    }

    #[test]
    fn repo_state_classification_paths() {
        let clean = mk_status(true, 0, 0, 0, 0);
        assert!(!repo_is_concern(&clean, true, true));
        assert!(!repo_is_warn(&clean, true, true));

        let dirty = mk_status(false, 0, 0, 3, 1);
        assert!(!repo_is_concern(&dirty, true, true));
        assert!(repo_is_warn(&dirty, true, true));

        let ahead = mk_status(true, 2, 0, 0, 0);
        assert!(repo_is_concern(&ahead, true, true));
        assert!(!repo_is_warn(&ahead, true, true));
    }

    #[test]
    fn repo_state_flags_and_hint_are_consistent() {
        let st = mk_status(false, 7, 0, 2, 1);
        let flags = repo_state_flags(&st, true, true);
        assert!(flags.iter().any(|f| f == "DIRTY"));
        assert!(flags.iter().any(|f| f == "AHEAD:7"));
        assert!(flags.iter().any(|f| f == "STUCK_PUSH"));
        let hint = repo_hint(&flags, false, true);
        assert!(hint.contains("repair-concerns"));
    }

    #[test]
    fn push_blob_threshold_is_guardrailed() {
        let mut p = test_policy();
        p.max_stage_file_bytes = 200 * 1024 * 1024;
        p.max_push_blob_bytes = 80 * 1024 * 1024;
        assert_eq!(push_large_blob_threshold_bytes(&p), 80 * 1024 * 1024);
        p.max_push_blob_bytes = 200 * 1024 * 1024;
        assert_eq!(
            push_large_blob_threshold_bytes(&p),
            DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES
        );
        p.max_stage_file_bytes = 50 * 1024 * 1024;
        assert_eq!(push_large_blob_threshold_bytes(&p), 50 * 1024 * 1024);
    }

    #[test]
    fn incident_ledger_write_roundtrip() {
        let td = TempDir::new("sync_ledger");
        let policy = td.path().join("dracon-sync.toml");
        std::fs::write(&policy, "watch_roots=[]").expect("policy");
        let record = IncidentRecord {
            ts_unix: timestamp_secs(),
            scope: "concern".to_string(),
            repo: "/tmp/repo".to_string(),
            reason: "AHEAD:1".to_string(),
            action: "push_origin_head".to_string(),
            backup_branch: None,
            result: "ok".to_string(),
            details: Some("d".to_string()),
        };
        append_incident_record(&policy, &record);
        let ledger = incident_ledger_path(&policy);
        let body = std::fs::read_to_string(&ledger).expect("ledger");
        assert!(!body.trim().is_empty());
        let last = body.lines().last().expect("line");
        let parsed: Value = serde_json::from_str(last).expect("json");
        assert_eq!(parsed["scope"], "concern");
        assert_eq!(parsed["result"], "ok");
    }

    /// Regression test: deleted files must ALWAYS be staged (go to to_stage),
    /// never filtered out (to_restore). If deleted files land in to_restore,
    /// the post-commit restore logic would restore them from HEAD.
    #[test]
    fn test_deleted_files_always_staged() {
        use dracon_git::types::{DiffFile, FileStatus};
        use std::collections::BTreeSet;

        let repo = TempDir::new("deleted_stage");

        let entry = DiffFile {
            path: PathBuf::from("some/deleted/dir/file.rs"),
            status: FileStatus::Deleted,
        };

        let excluded = BTreeSet::new();
        let patterns: Vec<String> = vec![];

        let result = should_stage_entry(
            repo.path(),
            &entry,
            &excluded,
            &patterns,
            10_000_000,
        );

        assert!(
            result,
            "REGRESSION: Deleted file was NOT staged! \
             This means the post-commit restore would restore it from HEAD. \
             Deleted files must always go to to_stage."
        );

        // Also verify can_restore_entry returns false for Deleted
        assert!(
            !can_restore_entry(&entry),
            "can_restore_entry must return false for Deleted files"
        );
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // If output is piped (e.g. `dracon-sync repos | head`), stdout can become a broken pipe.
    // Rust's default printing panics on write errors; convert that specific panic into a clean exit.
    let default_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();
        if msg.contains("Broken pipe") {
            std::process::exit(0);
        }
        default_panic_hook(info);
    }));

    let cli = Cli::parse();
    let policy_path = resolve_policy_path()?;

    match cli.cmd {
        Command::Status { json } => {
            let policy = SyncPolicy::load(&policy_path)?;
            let roots = policy.watch_root_paths();
            let excluded_dir_names = excluded_dir_names_set(&policy);
            let repos = discover_git_repos(&roots, &excluded_dir_names);
            let freeze = freeze_reason(&policy_path);
            if json {
                let payload = StatusJson {
                    policy: policy_path.display().to_string(),
                    roots: roots.iter().map(|p| p.display().to_string()).collect(),
                    repos_discovered: repos.len(),
                    pulse_interval_secs: policy.pulse_interval_secs,
                    inactivity_push_delay_secs: policy.inactivity_push_delay_secs,
                    freeze: freeze
                        .map(|r| format!("ON ({})", r))
                        .unwrap_or_else(|| "OFF".to_string()),
                    auto_commit: policy.auto_commit,
                    auto_pull: policy.auto_pull,
                    auto_push: policy.auto_push,
                    auto_bump_versions: policy.auto_bump_versions,
                    auto_repair_concerns: policy.auto_repair_concerns,
                    auto_repair_warns: policy.auto_repair_warns,
                    auto_rewrite_large_blobs: policy.auto_rewrite_large_blobs,
                    max_stage_file_bytes: policy.max_stage_file_bytes,
                    push_blob_threshold_bytes: push_large_blob_threshold_bytes(&policy),
                    exclude_dirs: policy.exclude_dir_names.clone(),
                    exclude_file_patterns: policy.exclude_file_patterns.clone(),
                    pull_op_timeout_secs: policy.pull_op_timeout_secs,
                    push_op_timeout_secs: policy.push_op_timeout_secs,
                    repo_sync_timeout_secs: policy.repo_sync_timeout_secs,
                    push_retries: policy.push_retries,
                    repair_cooldown_secs: policy.repair_cooldown_secs,
                    incident_ledger_max_lines: policy.incident_ledger_max_lines,
                    incident_ledger_max_age_days: policy.incident_ledger_max_age_days,
                    system_repo: policy.system_repo.clone(),
                    backup_policy: policy.backup_policy.clone(),
                    backup_dir: policy.backup_dir.clone(),
                    extra_remotes: policy.extra_remotes.len(),
                };
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("📜 POLICY: {}", policy_path.display());
                println!("🔁 ROOTS: {:?}", roots);
                println!("📦 REPOS_DISCOVERED: {}", repos.len());
                println!("⏱️ PULSE: {}s", policy.pulse_interval_secs);
                println!(
                    "⏳ INACTIVITY_PUSH_DELAY: {}s",
                    policy.inactivity_push_delay_secs
                );
                println!(
                    "⏸️ FREEZE: {}",
                    freeze
                        .map(|r| format!("ON ({})", r))
                        .unwrap_or_else(|| "OFF".to_string())
                );
                println!(
                    "⚙️ FLAGS: auto_commit={} auto_pull={} auto_push={} auto_bump_versions={} auto_repair_concerns={} auto_repair_warns={} auto_rewrite_large_blobs={}",
                    policy.auto_commit,
                    policy.auto_pull,
                    policy.auto_push,
                    policy.auto_bump_versions,
                    policy.auto_repair_concerns,
                    policy.auto_repair_warns,
                    policy.auto_rewrite_large_blobs
                );
                println!("📏 MAX_STAGE_FILE_BYTES: {}", policy.max_stage_file_bytes);
                println!(
                    "🧱 PUSH_BLOB_THRESHOLD_BYTES: {}",
                    push_large_blob_threshold_bytes(&policy)
                );
                println!("🚫 EXCLUDE_DIRS: {:?}", policy.exclude_dir_names);
                println!("🚫 EXCLUDE_FILE_PATTERNS: {:?}", policy.exclude_file_patterns);
                println!(
                    "⏱️ TIMEOUTS: pull={}s push={}s repo={}s retries={}",
                    policy.pull_op_timeout_secs,
                    policy.push_op_timeout_secs,
                    policy.repo_sync_timeout_secs,
                    policy.push_retries
                );
                println!(
                    "🧯 REPAIR: cooldown={}s ledger_max_lines={} ledger_max_age_days={}",
                    policy.repair_cooldown_secs,
                    policy.incident_ledger_max_lines,
                    policy.incident_ledger_max_age_days
                );
                if !policy.system_repo.is_empty() {
                    println!("🏛️ SYSTEM_REPO: {}", policy.system_repo);
                }
                if !policy.backup_policy.is_empty() || !policy.backup_dir.is_empty() {
                    println!(
                        "🧰 BACKUP: policy={} dir={}",
                        policy.backup_policy, policy.backup_dir
                    );
                }
                println!("🌐 EXTRA_REMOTES: {}", policy.extra_remotes.len());
            }
        }
        Command::Repos {
            only_concern,
            only_warn,
            json,
        } => {
            let filter = if only_concern {
                RepoFilter::Concern
            } else if only_warn {
                RepoFilter::Warn
            } else {
                RepoFilter::All
            };
            run_repos_report(&policy_path, filter, json).await?;
        }
        Command::RepairConcerns {
            apply,
            repo,
            push_timeout_secs,
            push_retries,
            rewrite_large_any,
            only_stuck_push,
            only_stuck_pull,
            json,
        } => {
            let filter = if only_stuck_push {
                ConcernRepairFilter::StuckPush
            } else if only_stuck_pull {
                ConcernRepairFilter::StuckPull
            } else {
                ConcernRepairFilter::All
            };
            run_repair_concerns(
                &policy_path,
                apply,
                repo,
                push_timeout_secs,
                push_retries,
                rewrite_large_any,
                filter,
                json,
            )
            .await?;
        }
        Command::RepairWarns { apply, repo, json } => {
            run_repair_warns(&policy_path, apply, repo, json).await?;
        }
        Command::Once => {
            run_once(&policy_path).await?;
        }
        Command::Daemon => {
            run_daemon(policy_path).await?;
        }
        Command::SyncNow { repo } => {
            if let Some(reason) = freeze_reason(&policy_path) {
                println!("⏸️ sync frozen ({})", reason);
                return Ok(());
            }
            let policy = SyncPolicy::load(&policy_path)?;
            let excluded_dir_names = excluded_dir_names_set(&policy);
            if sync_repo(&repo, &policy, &excluded_dir_names, 0).await? {
                println!("🔁 synced {}", repo.display());
            } else {
                println!("✅ no sync changes {}", repo.display());
            }
        }
        Command::EditConfig => {
            open_policy_in_editor(&policy_path)?;
        }
    }

    Ok(())
}
