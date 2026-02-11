use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use tokio::time::sleep;

use dracon_system_lib::analyze_workspace_storage;

#[derive(Parser, Debug)]
#[command(name = "dracon-system")]
#[command(about = "Deterministic system utility (no AI)")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show core path and service status.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Run deterministic diagnostics for canonical dracon setup.
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        strict: bool,
    },
    /// Analyze storage hotspots and optionally clean safe build/cache dirs.
    Storage {
        root: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        cleanup: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        allow_tracked: bool,
        #[arg(long)]
        min_size_mb: Option<u64>,
        #[arg(long)]
        kinds: Option<String>,
    },
    /// Manage deterministic symlink ownership for system setup.
    Link {
        #[command(subcommand)]
        cmd: LinkCommands,
    },
    /// Guard runtime: monitor disk/process pressure and notify/mitigate.
    Guard {
        #[command(subcommand)]
        cmd: GuardCommands,
    },
}

#[derive(Subcommand, Debug)]
enum LinkCommands {
    /// Show link reconciliation status from policy.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Diagnose link drift and invalid targets.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Apply link policy by creating/fixing symlinks.
    Apply {
        #[arg(long)]
        json: bool,
        /// Replace non-symlink paths at link locations (backs up existing content first).
        #[arg(long)]
        force_replace: bool,
    },
}

#[derive(Subcommand, Debug)]
enum GuardCommands {
    /// Run one guard evaluation pass.
    Once {
        #[arg(long)]
        json: bool,
    },
    /// Run continuous guard loop.
    Daemon,
}

#[derive(Debug, Serialize)]
struct StatusReport {
    system_root: String,
    nixos_root: String,
    sync_policy: String,
    system_policy: String,
    sync_service_active: bool,
    warden_service_active: bool,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    system_root_exists: bool,
    nixos_root_exists: bool,
    canonical_libs_exists: bool,
    canonical_utils_exists: bool,
    sync_policy_exists: bool,
    legacy_config_dracon_exists: bool,
    sync_service_active: bool,
    warden_service_active: bool,
}

#[derive(Debug, Clone)]
struct CleanupConfig {
    apply: bool,
    allow_tracked: bool,
    min_size_mb: u64,
    kinds: HashSet<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct SystemPolicy {
    #[serde(default)]
    storage: StoragePolicy,
    #[serde(default)]
    links: LinkPolicy,
    #[serde(default)]
    guard: GuardPolicy,
}

#[derive(Debug, Clone, Deserialize)]
struct StoragePolicy {
    #[serde(default)]
    default_root: String,
    #[serde(default = "default_min_size_mb")]
    min_size_mb: u64,
    #[serde(default = "default_kinds")]
    kinds: String,
}

impl Default for StoragePolicy {
    fn default() -> Self {
        Self {
            default_root: String::new(),
            min_size_mb: default_min_size_mb(),
            kinds: default_kinds(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct LinkPolicy {
    #[serde(default)]
    entries: Vec<LinkEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct LinkEntry {
    link: String,
    target: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GuardPolicy {
    #[serde(default = "default_guard_enabled")]
    enabled: bool,
    #[serde(default = "default_guard_interval_secs")]
    interval_secs: u64,
    #[serde(default = "default_disk_warn_percent")]
    disk_warn_percent: u8,
    #[serde(default = "default_disk_action_percent")]
    disk_action_percent: u8,
    #[serde(default = "default_disk_critical_percent")]
    disk_critical_percent: u8,
    #[serde(default = "default_true")]
    freeze_sync_at_action: bool,
    #[serde(default = "default_sync_freeze_marker")]
    sync_freeze_marker: String,
    #[serde(default = "default_unfreeze_below_percent")]
    unfreeze_below_percent: u8,
    #[serde(default = "default_process_cpu_percent")]
    process_cpu_percent: f32,
    #[serde(default = "default_process_rss_mb")]
    process_rss_mb: u64,
    #[serde(default = "default_process_sustain_secs")]
    process_sustain_secs: u64,
    #[serde(default = "default_process_exempt_names")]
    process_exempt_names: String,
    #[serde(default = "default_true")]
    notify: bool,
    #[serde(default = "default_notify_command")]
    notify_command: String,
    #[serde(default = "default_notify_cooldown_secs")]
    notify_cooldown_secs: u64,
    #[serde(default)]
    auto_renice: bool,
    #[serde(default = "default_renice_value")]
    renice_value: i32,
}

impl Default for GuardPolicy {
    fn default() -> Self {
        Self {
            enabled: default_guard_enabled(),
            interval_secs: default_guard_interval_secs(),
            disk_warn_percent: default_disk_warn_percent(),
            disk_action_percent: default_disk_action_percent(),
            disk_critical_percent: default_disk_critical_percent(),
            freeze_sync_at_action: default_true(),
            sync_freeze_marker: default_sync_freeze_marker(),
            unfreeze_below_percent: default_unfreeze_below_percent(),
            process_cpu_percent: default_process_cpu_percent(),
            process_rss_mb: default_process_rss_mb(),
            process_sustain_secs: default_process_sustain_secs(),
            process_exempt_names: default_process_exempt_names(),
            notify: default_true(),
            notify_command: default_notify_command(),
            notify_cooldown_secs: default_notify_cooldown_secs(),
            auto_renice: false,
            renice_value: default_renice_value(),
        }
    }
}

#[derive(Debug, Serialize)]
struct GuardProcessAlert {
    pid: i32,
    command: String,
    cpu_percent: f32,
    rss_mb: u64,
    sustained_secs: u64,
    action: String,
}

#[derive(Debug, Serialize)]
struct GuardReport {
    enabled: bool,
    disk_use_percent: u8,
    disk_state: String,
    sync_frozen: bool,
    alerts: Vec<GuardProcessAlert>,
}

fn default_min_size_mb() -> u64 {
    512
}

fn default_kinds() -> String {
    "rust-build,node-deps,build-output,cache".to_string()
}

fn default_true() -> bool {
    true
}

fn default_guard_enabled() -> bool {
    true
}

fn default_guard_interval_secs() -> u64 {
    30
}

fn default_disk_warn_percent() -> u8 {
    85
}

fn default_disk_action_percent() -> u8 {
    92
}

fn default_disk_critical_percent() -> u8 {
    97
}

fn default_sync_freeze_marker() -> String {
    "/home/dracon/dracon/utilities/sync/.freeze".to_string()
}

fn default_unfreeze_below_percent() -> u8 {
    88
}

fn default_process_cpu_percent() -> f32 {
    180.0
}

fn default_process_rss_mb() -> u64 {
    4096
}

fn default_process_sustain_secs() -> u64 {
    30
}

fn default_process_exempt_names() -> String {
    "systemd,dbus-daemon,Xorg,kwin_wayland,plasmashell".to_string()
}

fn default_notify_command() -> String {
    "notify-send".to_string()
}

fn default_notify_cooldown_secs() -> u64 {
    120
}

fn default_renice_value() -> i32 {
    10
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut idx = 0usize;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    format!("{value:.1} {}", UNITS[idx])
}

fn canonical_system_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join("dracon")
}

fn expand_tilde(raw: &str) -> PathBuf {
    if raw == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        return dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/home"))
            .join(rest);
    }
    PathBuf::from(raw)
}

fn parse_kinds(csv: &str) -> HashSet<String> {
    csv.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn parse_names(csv: &str) -> HashSet<String> {
    csv.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone)]
struct ProcSample {
    pid: i32,
    cpu_percent: f32,
    rss_mb: u64,
    command: String,
}

#[derive(Debug, Default)]
struct GuardRuntimeState {
    heavy_since: HashMap<i32, Instant>,
    notify_cooldowns: HashMap<String, Instant>,
    last_disk_state: String,
}

fn parse_df_use_percent(output: &str) -> Option<u8> {
    output
        .lines()
        .nth(1)
        .and_then(|line| line.split_whitespace().nth(4))
        .and_then(|v| v.trim_end_matches('%').parse::<u8>().ok())
}

fn parse_ps_output(output: &str) -> Vec<ProcSample> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid = parts.next()?.parse::<i32>().ok()?;
            let cpu_percent = parts.next()?.parse::<f32>().ok()?;
            let rss_kb = parts.next()?.parse::<u64>().ok()?;
            let command = parts.next()?.to_string();
            Some(ProcSample {
                pid,
                cpu_percent,
                rss_mb: rss_kb / 1024,
                command,
            })
        })
        .collect()
}

async fn root_disk_use_percent() -> Result<u8> {
    let out = Command::new("df").args(["-P", "/"]).output().await?;
    if !out.status.success() {
        return Err(anyhow::anyhow!("df command failed"));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    parse_df_use_percent(&text).ok_or_else(|| anyhow::anyhow!("failed parsing df output"))
}

async fn process_samples() -> Result<Vec<ProcSample>> {
    let out = Command::new("ps")
        .args(["-eo", "pid=,pcpu=,rss=,comm="])
        .output()
        .await?;
    if !out.status.success() {
        return Err(anyhow::anyhow!("ps command failed"));
    }
    Ok(parse_ps_output(&String::from_utf8_lossy(&out.stdout)))
}

fn disk_state(used: u8, guard: &GuardPolicy) -> &'static str {
    if used >= guard.disk_critical_percent {
        "critical"
    } else if used >= guard.disk_action_percent {
        "action"
    } else if used >= guard.disk_warn_percent {
        "warn"
    } else {
        "ok"
    }
}

async fn send_notification(guard: &GuardPolicy, title: &str, body: &str) {
    if !guard.notify || guard.notify_command.trim().is_empty() {
        return;
    }
    let _ = Command::new(guard.notify_command.trim())
        .arg(title)
        .arg(body)
        .output()
        .await;
}

fn should_notify(state: &mut GuardRuntimeState, key: &str, cooldown_secs: u64) -> bool {
    let now = Instant::now();
    if let Some(until) = state.notify_cooldowns.get(key).copied() {
        if now < until {
            return false;
        }
    }
    state.notify_cooldowns.insert(
        key.to_string(),
        now + Duration::from_secs(cooldown_secs.max(1)),
    );
    true
}

fn sync_freeze_marker_path(guard: &GuardPolicy) -> PathBuf {
    PathBuf::from(guard.sync_freeze_marker.clone())
}

async fn renice_process(pid: i32, value: i32) {
    let _ = Command::new("renice")
        .args(["-n", &value.to_string(), "-p", &pid.to_string()])
        .output()
        .await;
}

async fn run_guard_once(
    guard: &GuardPolicy,
    state: &mut GuardRuntimeState,
) -> Result<GuardReport> {
    let used = root_disk_use_percent().await?;
    let dstate = disk_state(used, guard).to_string();
    let marker = sync_freeze_marker_path(guard);
    let mut sync_frozen = marker.exists();

    if guard.freeze_sync_at_action && (dstate == "action" || dstate == "critical") {
        if !sync_frozen {
            if let Some(parent) = marker.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&marker, format!("dracon-system guard freeze: disk={}%\n", used));
            sync_frozen = true;
        }
    } else if sync_frozen && used <= guard.unfreeze_below_percent {
        let _ = fs::remove_file(&marker);
        sync_frozen = false;
    }

    if state.last_disk_state != dstate {
        let key = format!("disk-state-{dstate}");
        if should_notify(state, &key, guard.notify_cooldown_secs) {
            send_notification(
                guard,
                "Dracon System Guard",
                &format!("Disk pressure state changed to {} (used={}%)", dstate, used),
            )
            .await;
        }
        state.last_disk_state = dstate.clone();
    }

    let exempt = parse_names(&guard.process_exempt_names);
    let samples = process_samples().await?;
    let mut current_heavy = HashSet::new();
    let mut alerts = Vec::new();

    for p in samples {
        if exempt.contains(&p.command) {
            continue;
        }
        let heavy = p.cpu_percent >= guard.process_cpu_percent || p.rss_mb >= guard.process_rss_mb;
        if !heavy {
            continue;
        }
        current_heavy.insert(p.pid);
        let now = Instant::now();
        let since = state.heavy_since.entry(p.pid).or_insert(now);
        let sustained = now.duration_since(*since).as_secs();
        if sustained < guard.process_sustain_secs {
            continue;
        }

        let mut action = "notify".to_string();
        if guard.auto_renice {
            renice_process(p.pid, guard.renice_value).await;
            action = format!("renice:{}", guard.renice_value);
        }

        let key = format!("proc-{}", p.pid);
        if should_notify(state, &key, guard.notify_cooldown_secs) {
            send_notification(
                guard,
                "Dracon System Guard",
                &format!(
                    "Heavy process {} (pid={} cpu={:.1}% rss={}MiB) sustained {}s",
                    p.command, p.pid, p.cpu_percent, p.rss_mb, sustained
                ),
            )
            .await;
        }

        alerts.push(GuardProcessAlert {
            pid: p.pid,
            command: p.command,
            cpu_percent: p.cpu_percent,
            rss_mb: p.rss_mb,
            sustained_secs: sustained,
            action,
        });
    }

    state.heavy_since.retain(|pid, _| current_heavy.contains(pid));

    Ok(GuardReport {
        enabled: guard.enabled,
        disk_use_percent: used,
        disk_state: dstate,
        sync_frozen,
        alerts,
    })
}

#[derive(Debug, Serialize)]
struct LinkEntryStatus {
    link: String,
    target: String,
    exists: bool,
    is_symlink: bool,
    target_exists: bool,
    points_to: String,
    in_sync: bool,
    issue: String,
}

#[derive(Debug, Serialize)]
struct LinkStatusReport {
    entries: Vec<LinkEntryStatus>,
    total: usize,
    healthy: usize,
    drifted: usize,
    missing_target: usize,
    missing_link: usize,
}

fn path_display(path: &Path) -> String {
    path.display().to_string()
}

fn evaluate_link(entry: &LinkEntry) -> LinkEntryStatus {
    let link = expand_tilde(&entry.link);
    let target = expand_tilde(&entry.target);
    let target_exists = target.exists();
    let meta = fs::symlink_metadata(&link).ok();
    let exists = meta.is_some();
    let is_symlink = meta
        .as_ref()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    let mut points_to = String::new();
    let mut in_sync = false;
    let issue = if !target_exists {
        "target_missing".to_string()
    } else if !exists {
        "link_missing".to_string()
    } else if !is_symlink {
        "path_not_symlink".to_string()
    } else {
        match fs::read_link(&link) {
            Ok(actual) => {
                let actual_abs = if actual.is_absolute() {
                    actual
                } else {
                    link.parent().unwrap_or_else(|| Path::new("/")).join(actual)
                };
                points_to = path_display(&actual_abs);
                if normalize_path(&actual_abs) == normalize_path(&target) {
                    in_sync = true;
                    "ok".to_string()
                } else {
                    "link_target_mismatch".to_string()
                }
            }
            Err(_) => "readlink_failed".to_string(),
        }
    };

    LinkEntryStatus {
        link: path_display(&link),
        target: path_display(&target),
        exists,
        is_symlink,
        target_exists,
        points_to,
        in_sync,
        issue,
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn build_link_report(policy: &SystemPolicy) -> LinkStatusReport {
    let mut entries = Vec::with_capacity(policy.links.entries.len());
    let mut healthy = 0usize;
    let mut drifted = 0usize;
    let mut missing_target = 0usize;
    let mut missing_link = 0usize;

    for entry in &policy.links.entries {
        let status = evaluate_link(entry);
        match status.issue.as_str() {
            "ok" => healthy += 1,
            "target_missing" => {
                drifted += 1;
                missing_target += 1;
            }
            "link_missing" => {
                drifted += 1;
                missing_link += 1;
            }
            _ => drifted += 1,
        }
        entries.push(status);
    }

    LinkStatusReport {
        total: entries.len(),
        entries,
        healthy,
        drifted,
        missing_target,
        missing_link,
    }
}

fn backup_path_for(link: &Path) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = link
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "link".to_string());
    let backup_name = format!("{name}.dracon-system-backup-{ts}");
    link.with_file_name(backup_name)
}

fn apply_link_policy(policy: &SystemPolicy, force_replace: bool) -> Result<LinkStatusReport> {
    for entry in &policy.links.entries {
        let link = expand_tilde(&entry.link);
        let target = expand_tilde(&entry.target);

        if !target.exists() {
            continue;
        }

        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent)?;
        }

        let meta = fs::symlink_metadata(&link).ok();
        if let Some(meta) = meta {
            if meta.file_type().is_symlink() {
                fs::remove_file(&link)?;
            } else if force_replace {
                let backup = backup_path_for(&link);
                fs::rename(&link, backup)?;
            } else {
                continue;
            }
        }

        #[cfg(unix)]
        {
            symlink(&target, &link)?;
        }
        #[cfg(not(unix))]
        {
            return Err(anyhow::anyhow!("link apply is only supported on unix"));
        }
    }

    Ok(build_link_report(policy))
}

fn resolve_system_policy_path() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("DRACON_SYSTEM_POLICY") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Some(p);
        }
    }

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    let candidates = [
        home.join("dracon/utilities/system/dracon-system.toml"),
        home.join("dracon/utilities/system/config.toml"),
        home.join("dracon/system/dracon-system.toml"),
        home.join("dracon/system/config.toml"),
    ];

    candidates.into_iter().find(|p| p.exists())
}

fn load_system_policy() -> (Option<PathBuf>, SystemPolicy) {
    let Some(path) = resolve_system_policy_path() else {
        return (None, SystemPolicy::default());
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return (Some(path), SystemPolicy::default()),
    };
    let parsed: SystemPolicy = toml::from_str(&content).unwrap_or_default();
    (Some(path), parsed)
}

async fn is_user_service_active(service: &str) -> bool {
    let output = Command::new("systemctl")
        .args(["--user", "is-active", service])
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim() == "active"
        }
        _ => false,
    }
}

async fn build_status_report() -> StatusReport {
    let root = canonical_system_root();
    let (system_policy_path, _) = load_system_policy();
    StatusReport {
        system_root: root.display().to_string(),
        nixos_root: root.join("nixos").display().to_string(),
        sync_policy: root
            .join("utilities/sync/dracon-sync.toml")
            .display()
            .to_string(),
        system_policy: system_policy_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<default>".to_string()),
        sync_service_active: is_user_service_active("dracon-sync.service").await,
        warden_service_active: is_user_service_active("dracon-warden.service").await,
    }
}

fn normalize_guard_policy(policy: &mut GuardPolicy) {
    policy.interval_secs = policy.interval_secs.max(5);
    policy.disk_warn_percent = policy.disk_warn_percent.max(1).min(100);
    policy.disk_action_percent = policy
        .disk_action_percent
        .max(policy.disk_warn_percent)
        .min(100);
    policy.disk_critical_percent = policy
        .disk_critical_percent
        .max(policy.disk_action_percent)
        .min(100);
    policy.unfreeze_below_percent = policy
        .unfreeze_below_percent
        .min(policy.disk_action_percent.saturating_sub(1));
    policy.process_cpu_percent = policy.process_cpu_percent.max(1.0);
    policy.process_rss_mb = policy.process_rss_mb.max(64);
    policy.process_sustain_secs = policy.process_sustain_secs.max(5);
    policy.notify_cooldown_secs = policy.notify_cooldown_secs.max(5);
    if policy.sync_freeze_marker.trim().is_empty() {
        policy.sync_freeze_marker = default_sync_freeze_marker();
    }
    if policy.notify_command.trim().is_empty() {
        policy.notify_command = default_notify_command();
    }
}

async fn build_doctor_report() -> DoctorReport {
    let root = canonical_system_root();
    let nixos = root.join("nixos");
    let libs = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join("Dev/dracon-libs");
    let utils = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join("Dev/dracon-utilities");
    let policy = root.join("utilities/sync/dracon-sync.toml");
    let legacy_cfg = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join(".config/dracon");

    DoctorReport {
        system_root_exists: root.exists(),
        nixos_root_exists: nixos.exists(),
        canonical_libs_exists: libs.exists(),
        canonical_utils_exists: utils.exists(),
        sync_policy_exists: policy.exists(),
        legacy_config_dracon_exists: legacy_cfg.exists(),
        sync_service_active: is_user_service_active("dracon-sync.service").await,
        warden_service_active: is_user_service_active("dracon-warden.service").await,
    }
}

async fn is_git_tracked_dir(path: &Path) -> Result<bool> {
    let parent = match path.parent() {
        Some(p) => p,
        None => return Ok(false),
    };
    let name = match path.file_name() {
        Some(n) => n.to_string_lossy().to_string(),
        None => return Ok(false),
    };

    let top_out = Command::new("git")
        .arg("-C")
        .arg(parent)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .await?;
    if !top_out.status.success() {
        return Ok(false);
    }

    let repo_root = String::from_utf8_lossy(&top_out.stdout).trim().to_string();
    if repo_root.is_empty() {
        return Ok(false);
    }

    let ls_out = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["ls-files", "--", &name])
        .output()
        .await?;
    if !ls_out.status.success() {
        return Ok(false);
    }

    Ok(!String::from_utf8_lossy(&ls_out.stdout).trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn defaults_are_expected() {
        assert_eq!(default_min_size_mb(), 512);
        assert_eq!(default_kinds(), "rust-build,node-deps,build-output,cache");
    }

    #[test]
    fn human_bytes_formats_units() {
        assert_eq!(human_bytes(1), "1.0 B");
        assert_eq!(human_bytes(1024), "1.0 KiB");
        assert_eq!(human_bytes(1024 * 1024), "1.0 MiB");
    }

    #[test]
    fn parse_kinds_trims_and_dedupes() {
        let kinds = parse_kinds(" rust-build, node-deps ,rust-build,,cache ");
        assert_eq!(kinds.len(), 3);
        assert!(kinds.contains("rust-build"));
        assert!(kinds.contains("node-deps"));
        assert!(kinds.contains("cache"));
    }

    #[test]
    fn expand_tilde_uses_home_when_available() {
        let _guard = env_lock().lock().expect("lock");
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", "/tmp/dracon-home-test");

        assert_eq!(expand_tilde("~"), PathBuf::from("/tmp/dracon-home-test"));
        assert_eq!(
            expand_tilde("~/Dev/project"),
            PathBuf::from("/tmp/dracon-home-test/Dev/project")
        );
        assert_eq!(expand_tilde("/x/y"), PathBuf::from("/x/y"));

        if let Some(v) = old_home {
            std::env::set_var("HOME", v);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn build_link_report_counts_states() {
        let policy = SystemPolicy {
            storage: StoragePolicy::default(),
            links: LinkPolicy {
                entries: vec![LinkEntry {
                    link: "/tmp/does-not-exist-link".into(),
                    target: "/tmp/does-not-exist-target".into(),
                }],
            },
            guard: GuardPolicy::default(),
        };
        let report = build_link_report(&policy);
        assert_eq!(report.total, 1);
        assert_eq!(report.healthy, 0);
        assert_eq!(report.drifted, 1);
        assert_eq!(report.missing_target, 1);
    }

    #[cfg(unix)]
    #[test]
    fn evaluate_link_handles_missing_and_sync_cases() {
        let base = std::env::temp_dir().join(format!(
            "dracon_system_test_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(&base).expect("base dir");

        let target = base.join("target.txt");
        fs::write(&target, "x").expect("target");

        let missing_link = LinkEntry {
            link: base.join("missing-link").display().to_string(),
            target: target.display().to_string(),
        };
        let s1 = evaluate_link(&missing_link);
        assert_eq!(s1.issue, "link_missing");

        let normal_file_link = base.join("normal-file");
        fs::write(&normal_file_link, "x").expect("file");
        let not_symlink = LinkEntry {
            link: normal_file_link.display().to_string(),
            target: target.display().to_string(),
        };
        let s2 = evaluate_link(&not_symlink);
        assert_eq!(s2.issue, "path_not_symlink");

        let good_link = base.join("good-link");
        symlink(&target, &good_link).expect("symlink");
        let synced = LinkEntry {
            link: good_link.display().to_string(),
            target: target.display().to_string(),
        };
        let s3 = evaluate_link(&synced);
        assert_eq!(s3.issue, "ok");
        assert!(s3.in_sync);

        let wrong_target = base.join("other.txt");
        fs::write(&wrong_target, "y").expect("other");
        let mismatch_link = base.join("mismatch-link");
        symlink(&wrong_target, &mismatch_link).expect("symlink mismatch");
        let mismatch = LinkEntry {
            link: mismatch_link.display().to_string(),
            target: target.display().to_string(),
        };
        let s4 = evaluate_link(&mismatch);
        assert_eq!(s4.issue, "link_target_mismatch");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn parse_and_format_repeated_scenarios() {
        for i in 0..220usize {
            let csv = if i % 2 == 0 {
                "rust-build,node-deps,cache"
            } else {
                " rust-build , build-output , cache , rust-build "
            };
            let kinds = parse_kinds(csv);
            assert!(kinds.contains("rust-build"));
            assert!(kinds.contains("cache"));

            let bytes = (i as u64 + 1) * 2048;
            let out = human_bytes(bytes);
            assert!(!out.is_empty());
            assert!(out.contains(' '));
        }
    }

    #[test]
    fn parse_df_use_percent_works() {
        let sample = "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/root 100 91 9 91% /\n";
        assert_eq!(parse_df_use_percent(sample), Some(91));
    }

    #[test]
    fn parse_ps_output_works() {
        let sample = "123 250.5 4194304 git\n456 12.0 2048 zsh\n";
        let rows = parse_ps_output(sample);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].pid, 123);
        assert_eq!(rows[0].command, "git");
        assert_eq!(rows[0].rss_mb, 4096);
    }

    #[test]
    fn guard_policy_normalization_is_safe() {
        let mut p = GuardPolicy::default();
        p.interval_secs = 0;
        p.disk_warn_percent = 99;
        p.disk_action_percent = 10;
        p.disk_critical_percent = 20;
        p.unfreeze_below_percent = 255;
        p.process_cpu_percent = 0.0;
        p.process_rss_mb = 0;
        p.process_sustain_secs = 0;
        p.notify_cooldown_secs = 0;
        p.sync_freeze_marker = String::new();
        p.notify_command = String::new();
        normalize_guard_policy(&mut p);
        assert!(p.interval_secs >= 5);
        assert!(p.disk_action_percent >= p.disk_warn_percent);
        assert!(p.disk_critical_percent >= p.disk_action_percent);
        assert!(p.unfreeze_below_percent < p.disk_action_percent);
        assert!(p.process_cpu_percent >= 1.0);
        assert!(p.process_rss_mb >= 64);
        assert!(p.process_sustain_secs >= 5);
        assert!(p.notify_cooldown_secs >= 5);
        assert!(!p.sync_freeze_marker.is_empty());
        assert!(!p.notify_command.is_empty());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Commands::Status { json } => {
            let report = build_status_report().await;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("system_root: {}", report.system_root);
                println!("nixos_root: {}", report.nixos_root);
                println!("sync_policy: {}", report.sync_policy);
                println!("system_policy: {}", report.system_policy);
                println!("sync_service_active: {}", report.sync_service_active);
                println!("warden_service_active: {}", report.warden_service_active);
            }
        }
        Commands::Doctor { json, strict } => {
            let report = build_doctor_report().await;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("system_root_exists: {}", report.system_root_exists);
                println!("nixos_root_exists: {}", report.nixos_root_exists);
                println!("canonical_libs_exists: {}", report.canonical_libs_exists);
                println!("canonical_utils_exists: {}", report.canonical_utils_exists);
                println!("sync_policy_exists: {}", report.sync_policy_exists);
                println!(
                    "legacy_config_dracon_exists: {}",
                    report.legacy_config_dracon_exists
                );
                println!("sync_service_active: {}", report.sync_service_active);
                println!("warden_service_active: {}", report.warden_service_active);
            }
            if strict {
                let mut violations = Vec::new();
                if report.legacy_config_dracon_exists {
                    violations.push("legacy ~/.config/dracon exists".to_string());
                }
                if !violations.is_empty() {
                    return Err(anyhow::anyhow!(
                        "strict doctor failed: {}",
                        violations.join("; ")
                    ));
                }
            }
        }
        Commands::Storage {
            root,
            json,
            cleanup,
            apply,
            allow_tracked,
            min_size_mb,
            kinds,
        } => {
            let (_, policy) = load_system_policy();
            let root = root.unwrap_or_else(|| {
                if !policy.storage.default_root.trim().is_empty() {
                    return PathBuf::from(policy.storage.default_root.clone());
                }
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/home"))
                    .join("Dev")
            });
            let min_size_mb = min_size_mb.unwrap_or(policy.storage.min_size_mb);
            let kinds = kinds.unwrap_or_else(|| policy.storage.kinds.clone());

            let report = analyze_workspace_storage(&root, 15, 25).await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
                return Ok(());
            }

            println!("Workspace: {}", report.root.display());
            println!();
            println!("Top projects:");
            for item in &report.top_projects {
                println!("  {:>10}  {}", human_bytes(item.bytes), item.path.display());
            }

            println!();
            println!("Top hotspots:");
            for item in &report.top_hotspots {
                println!(
                    "  {:>10}  {:<12} {}",
                    human_bytes(item.bytes),
                    item.kind,
                    item.path.display()
                );
            }

            if cleanup {
                let cfg = CleanupConfig {
                    apply,
                    allow_tracked,
                    min_size_mb,
                    kinds: parse_kinds(&kinds),
                };
                let threshold = cfg.min_size_mb.saturating_mul(1024 * 1024);
                let selected: Vec<_> = report
                    .top_hotspots
                    .iter()
                    .filter(|h| cfg.kinds.contains(&h.kind) && h.bytes >= threshold)
                    .collect();

                println!();
                println!(
                    "Cleanup mode: {}",
                    if cfg.apply { "APPLY" } else { "DRY-RUN" }
                );
                println!(
                    "Kinds: {}",
                    {
                        let mut v: Vec<_> = cfg.kinds.iter().cloned().collect();
                        v.sort();
                        v.join(",")
                    }
                );
                println!("Min size: {} MiB", cfg.min_size_mb);
                println!("Allow tracked: {}", cfg.allow_tracked);
                println!("Selected paths: {}", selected.len());

                let mut total = 0u64;
                let mut actionable = Vec::new();
                for item in selected {
                    let tracked = is_git_tracked_dir(&item.path).await.unwrap_or(false);
                    if tracked && !cfg.allow_tracked {
                        println!(
                            "  {:>10}  {:<12} {}  [SKIP tracked]",
                            human_bytes(item.bytes),
                            item.kind,
                            item.path.display()
                        );
                        continue;
                    }
                    total += item.bytes;
                    println!(
                        "  {:>10}  {:<12} {}{}",
                        human_bytes(item.bytes),
                        item.kind,
                        item.path.display(),
                        if tracked { "  [tracked]" } else { "" }
                    );
                    actionable.push(item.path.clone());
                }
                println!("Estimated reclaimed: {}", human_bytes(total));

                if cfg.apply {
                    for path in actionable {
                        if path.exists() {
                            println!("Deleting {}", path.display());
                            tokio::fs::remove_dir_all(path).await?;
                        }
                    }
                } else {
                    println!("No changes made. Re-run with --apply to execute cleanup.");
                }
            }
        }
        Commands::Link { cmd } => {
            let (_, policy) = load_system_policy();
            match cmd {
                LinkCommands::Status { json } | LinkCommands::Doctor { json } => {
                    let report = build_link_report(&policy);
                    if json {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    } else {
                        println!("links_total: {}", report.total);
                        println!("links_healthy: {}", report.healthy);
                        println!("links_drifted: {}", report.drifted);
                        println!("links_missing_target: {}", report.missing_target);
                        println!("links_missing_link: {}", report.missing_link);
                        for item in report.entries {
                            println!(
                                "- {} -> {} [{}]",
                                item.link,
                                item.target,
                                if item.issue == "ok" {
                                    "ok".to_string()
                                } else {
                                    item.issue
                                }
                            );
                        }
                    }
                }
                LinkCommands::Apply {
                    json,
                    force_replace,
                } => {
                    let report = apply_link_policy(&policy, force_replace)?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    } else {
                        println!("Applied link policy.");
                        println!("links_total: {}", report.total);
                        println!("links_healthy: {}", report.healthy);
                        println!("links_drifted: {}", report.drifted);
                    }
                }
            }
        }
        Commands::Guard { cmd } => {
            let (_, policy) = load_system_policy();
            let mut guard = policy.guard;
            normalize_guard_policy(&mut guard);
            let mut runtime = GuardRuntimeState::default();
            match cmd {
                GuardCommands::Once { json } => {
                    let report = run_guard_once(&guard, &mut runtime).await?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    } else {
                        println!("guard_enabled: {}", report.enabled);
                        println!("disk_use_percent: {}", report.disk_use_percent);
                        println!("disk_state: {}", report.disk_state);
                        println!("sync_frozen: {}", report.sync_frozen);
                        println!("alerts: {}", report.alerts.len());
                        for a in report.alerts {
                            println!(
                                "- pid={} cmd={} cpu={:.1}% rss={}MiB sustained={}s action={}",
                                a.pid,
                                a.command,
                                a.cpu_percent,
                                a.rss_mb,
                                a.sustained_secs,
                                a.action
                            );
                        }
                    }
                }
                GuardCommands::Daemon => {
                    if !guard.enabled {
                        println!("guard disabled in policy");
                        return Ok(());
                    }
                    println!("guard daemon started (interval={}s)", guard.interval_secs);
                    loop {
                        if let Err(e) = run_guard_once(&guard, &mut runtime).await {
                            eprintln!("guard pass failed: {}", e);
                        }
                        sleep(Duration::from_secs(guard.interval_secs)).await;
                    }
                }
            }
        }
    }

    Ok(())
}
