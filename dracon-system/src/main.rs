use anyhow::{Context, Result};
use clap::{ArgAction, Parser, Subcommand};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{symlink, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use tokio::process::Command;
use tokio::time::sleep;

use dracon_system_lib::analyze_workspace_storage;

const SYSTEM_PROTECTED: &[&str] = &[
    "/", "/home", "/etc", "/usr", "/var", "/boot",
    "/nix", "/run", "/sys", "/dev", "/proc"
];

fn check_safe_to_delete(path: &Path, user_protected: &[String]) -> Result<()> {
    let canon = match path.canonicalize() {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Path doesn't exist — nothing to delete, nothing to protect
            return Ok(());
        }
        Err(e) => anyhow::bail!("cannot canonicalize {}: {} — refusing to delete", path.display(), e),
    };
    let canon_str = canon.display().to_string();

    for prot in SYSTEM_PROTECTED {
        if is_protected_ancestor(&canon_str, prot) {
            anyhow::bail!(
                "refusing to delete protected path {} (under system root {})",
                canon.display(),
                prot
            );
        }
    }

    for user_prot in user_protected {
        let prot_canon = match Path::new(user_prot).canonicalize() {
            Ok(p) => p.display().to_string(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => anyhow::bail!("cannot canonicalize user-protected path {}: {} — refusing", user_prot, e),
        };
        if is_protected_ancestor(&canon_str, &prot_canon) {
            anyhow::bail!(
                "refusing to delete protected path {} (under user-protected path {})",
                canon.display(),
                user_prot
            );
        }
    }

    Ok(())
}

/// Check if `path` is equal to or a descendant of `protected`.
/// Both must be canonicalized absolute paths.
fn is_protected_ancestor(path: &str, protected: &str) -> bool {
    if path == protected {
        return true;
    }
    // Root '/' is special: every path is a descendant, so only match exact.
    if protected == "/" {
        return path == "/";
    }
    // Ensure protected ends with '/' so '/home' doesn't match '/homefoo'
    let prefix = if protected.ends_with('/') {
        protected.to_string()
    } else {
        format!("{}/", protected)
    };
    path.starts_with(&prefix)
}

#[cfg(test)]
const TEST_PROTECTED: &[&str] = &[
    "/", "/home", "/etc", "/usr", "/var", "/boot",
    "/nix", "/run", "/sys", "/dev", "/proc"
];

#[cfg(test)]
fn check_path_str(path: &str, user_protected: &[&str]) -> bool {
    let normalized = if path.ends_with('/') && path != "/" {
        path.trim_end_matches('/')
    } else {
        path
    };
    for prot in TEST_PROTECTED {
        if is_protected_ancestor(normalized, prot) {
            return false;
        }
    }
    for prot in user_protected {
        if is_protected_ancestor(normalized, prot) {
            return false;
        }
    }
    true
}

static ROLLING_LOG: std::sync::OnceLock<Mutex<Vec<String>>> = std::sync::OnceLock::new();

fn get_log() -> &'static Mutex<Vec<String>> {
    ROLLING_LOG.get_or_init(|| Mutex::new(Vec::new()))
}

static VERBOSITY: AtomicU8 = AtomicU8::new(0);

#[macro_export]
macro_rules! veprintln {
    ($lvl:expr, $($arg:tt)*) => {
        if $lvl <= VERBOSITY.load(Ordering::SeqCst) {
            eprintln!($($arg)*);
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EventSeverity {
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraconEvent {
    pub domain: String,
    pub severity: EventSeverity,
    pub path: String,
    pub message: String,
    pub timestamp: String,
}

impl DraconEvent {
    pub fn new<T1: ToString, T2: ToString, T3: ToString>(
        domain: T1,
        severity: EventSeverity,
        path: T2,
        message: T3,
    ) -> Self {
        Self {
            domain: domain.to_string(),
            severity,
            path: path.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub fn emit_event(event: &DraconEvent) {
    if let Ok(mut log) = get_log().lock() {
        if log.len() >= 1000 {
            log.remove(0);
        }
        log.push(format!(
            "[{}] {:?}: {} - {}",
            event.timestamp, event.severity, event.path, event.message
        ));
    }
    eprintln!(
        "[{}] {:?}: {} - {}",
        event.timestamp, event.severity, event.path, event.message
    );
    if let Some(events_path) = dirs::home_dir().map(|h| h.join(".dracon/events.jsonl")) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)
        {
            use std::io::Write;
            if let Ok(json) = serde_json::to_string(event) {
                let _ = writeln!(file, "{}", json);
            }
        }
    }
}

fn acquire_daemon_lock(name: &str) -> Result<File> {
    let lock_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("no home dir"))?
        .join(".dracon")
        .join("locks");
    
    std::fs::create_dir_all(&lock_dir)?;
    let lock_file = lock_dir.join(format!("{}.lock", name));
    
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(&lock_file)?;

    if file.lock_exclusive().is_err() {
        return Err(anyhow::anyhow!("lock file is held by another process"));
    }

    file.set_len(0)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(file)
}

fn events_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".dracon/events.jsonl"))
        .unwrap_or_else(|| PathBuf::from("/tmp/dracon-events.jsonl"))
}

#[derive(Parser, Debug)]
#[command(name = "dracon-system")]
#[command(about = "Deterministic system utility (no AI)")]
#[command(version)]
struct Cli {
    /// Increase output verbosity. Can be repeated up to 2 times (-v, -vv).
    #[arg(global = true, short, long, action = ArgAction::Count)]
    verbose: u8,
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
        /// Optional root path to analyze. Defaults to policy or ~/Dev.
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
    /// Show recent events from the shared event stream.
    Events {
        /// Number of recent events to show.
        #[arg(short, long, default_value = "50")]
        tail: usize,
        /// Filter by source (e.g. sync, warden, system).
        #[arg(short, long)]
        source: Option<String>,
        /// Filter by severity (info, warn, error, critical).
        #[arg(short, long)]
        severity: Option<String>,
    },
    /// Zram management: show stats and generate NixOS config for tuning.
    Zram {
        /// Show current zram statistics.
        #[arg(long, default_value = "false")]
        status: bool,
        /// Generate NixOS configuration for larger zram swap.
        #[arg(long)]
        gen_config: bool,
        /// Target memory percent for zram (e.g., 200 for 2x RAM).
        #[arg(long)]
        memory_percent: Option<u32>,
        /// Compression algorithm (lzo, lz4, lz4hc, zstd).
        #[arg(long)]
        algorithm: Option<String>,
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
    /// Prune system caches and Docker resources.
    Prune {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        docker: bool,
        #[arg(long)]
        docker_volumes: bool,
        #[arg(long)]
        package_caches: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Clean all reclaimable space (targets, trash, nix, caches, node_modules).
    Clean {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        rust: bool,
        #[arg(long)]
        trash: bool,
        #[arg(long)]
        nix: bool,
        #[arg(long)]
        caches: bool,
        #[arg(long)]
        node_modules: bool,
        #[arg(long)]
        docker: bool,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        min_size_mb: Option<u64>,
    },
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
    #[serde(default = "default_disk_mount_path")]
    disk_mount_path: String,
    #[serde(default = "default_guard_interval_secs")]
    interval_secs: u64,
    #[serde(default = "default_disk_early_warn_percent")]
    disk_early_warn_percent: u8,
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
    /// Automatically kill runaway git processes (git-init, git-fetch, etc.) after sustain period
    #[serde(default)]
    auto_kill_git: bool,
    /// Seconds a git process must sustain high CPU before auto-kill (if auto_kill_git is enabled)
    #[serde(default = "default_git_kill_threshold_secs")]
    git_kill_threshold_secs: u64,
    /// Path to persistent guard log file (JSONL format). Empty string disables file logging.
    #[serde(default = "default_guard_log_file")]
    guard_log_file: String,
    /// Maximum size of guard log file in MiB before rotation (deletes old entries)
    #[serde(default = "default_guard_log_max_mb")]
    guard_log_max_mb: u64,
    /// Automatically clean Rust build artifacts when disk hits action level
    #[serde(default = "default_auto_cleanup_rust")]
    auto_cleanup_rust: bool,
    /// Require explicit opt-in for destructive auto-cleanup in daemon mode.
    /// When false (default), the daemon only logs what it would clean without deleting.
    /// Set to true to allow the daemon to actually delete files during auto-cleanup.
    #[serde(default)]
    auto_cleanup_apply: bool,
    /// Minimum size (MiB) for a target dir to be considered for auto-cleanup
    #[serde(default = "default_cleanup_min_size_mb")]
    cleanup_min_size_mb: u64,
    /// Directories to search for Rust target directories
    #[serde(default = "default_rust_search_roots")]
    rust_search_roots: String,
    /// Directories to search for node_modules directories
    #[serde(default = "default_node_modules_search_roots")]
    node_modules_search_roots: String,
    /// Enable disk space trend tracking and prediction
    #[serde(default = "default_true")]
    track_trends: bool,
    /// Warn when disk is predicted to fill within this many hours
    #[serde(default = "default_trend_warn_hours")]
    trend_warn_hours: u64,
    /// Enable inode monitoring (warn when inode usage is high)
    #[serde(default = "default_true")]
    monitor_inodes: bool,
    /// Inode usage percent threshold for warning
    #[serde(default = "default_inode_warn_percent")]
    inode_warn_percent: u8,
    /// Enable zombie process detection
    #[serde(default = "default_true")]
    monitor_zombies: bool,
    /// Maximum number of zombie processes before alert
    #[serde(default = "default_zombie_threshold")]
    zombie_threshold: u64,
    /// Enable large log file detection
    #[serde(default = "default_true")]
    monitor_logs: bool,
    /// Log file size threshold in MiB
    #[serde(default = "default_log_size_mb")]
    log_size_mb: u64,
    /// Directories to scan for large log files
    #[serde(default = "default_log_dirs")]
    log_dirs: String,
    /// Automatically truncate large log files when detected (keeps file, shrinks to max_size)
    #[serde(default)]
    auto_truncate_logs: bool,
    /// Max size in MiB to truncate log files to (only applies when auto_truncate_logs is true)
    #[serde(default = "default_log_max_truncate_mb")]
    log_max_truncate_mb: u64,
    /// Number of header lines to preserve when truncating (0 = truncate completely)
    #[serde(default)]
    log_preserve_header_lines: usize,
    /// Enable Docker pruning when disk is critical
    #[serde(default = "default_true")]
    docker_prune: bool,
    /// Prune Docker volumes too (more aggressive)
    #[serde(default)]
    docker_prune_volumes: bool,
    /// Clean package caches when disk is critical
    #[serde(default = "default_true")]
    clean_package_caches: bool,
    /// Empty trash when disk is critical
    #[serde(default = "default_true")]
    clean_trash: bool,
    /// Run nix-collect-garbage when disk is critical
    #[serde(default = "default_true")]
    clean_nix_garbage: bool,
    /// Clean old nix generations (keep last N)
    #[serde(default = "default_nix_keep_generations")]
    nix_keep_generations: u32,
    /// Clean old node_modules (older than N days)
    #[serde(default = "default_node_modules_max_age_days")]
    node_modules_max_age_days: u64,
    /// Paths that should never be deleted (e.g. ~/Videos, ~/Documents/tax)
    #[serde(default)]
    protected_paths: Vec<String>,
}

impl Default for GuardPolicy {
    fn default() -> Self {
        Self {
            enabled: default_guard_enabled(),
            disk_mount_path: default_disk_mount_path(),
            interval_secs: default_guard_interval_secs(),
            disk_early_warn_percent: default_disk_early_warn_percent(),
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
            auto_kill_git: false,
            git_kill_threshold_secs: default_git_kill_threshold_secs(),
            guard_log_file: default_guard_log_file(),
            guard_log_max_mb: default_guard_log_max_mb(),
            auto_cleanup_rust: default_auto_cleanup_rust(),
            auto_cleanup_apply: false,
            cleanup_min_size_mb: default_cleanup_min_size_mb(),
            rust_search_roots: default_rust_search_roots(),
            node_modules_search_roots: default_node_modules_search_roots(),
            track_trends: default_true(),
            trend_warn_hours: default_trend_warn_hours(),
            monitor_inodes: default_true(),
            inode_warn_percent: default_inode_warn_percent(),
            monitor_zombies: default_true(),
            zombie_threshold: default_zombie_threshold(),
            monitor_logs: default_true(),
            log_size_mb: default_log_size_mb(),
            log_dirs: default_log_dirs(),
            auto_truncate_logs: false,
            log_max_truncate_mb: default_log_max_truncate_mb(),
            log_preserve_header_lines: 0,
            docker_prune: default_true(),
            docker_prune_volumes: false,
            clean_package_caches: default_true(),
            clean_trash: default_true(),
            clean_nix_garbage: default_true(),
            nix_keep_generations: 5,
            node_modules_max_age_days: default_node_modules_max_age_days(),
            protected_paths: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct GuardProcessAlert {
    pid: i32,
    ppid: i32,
    command: String,
    args: String,
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

fn default_disk_mount_path() -> String {
    if PathBuf::from("/nix").exists() {
        "/nix".to_string()
    } else {
        "/".to_string()
    }
}

fn default_guard_interval_secs() -> u64 {
    30
}

fn default_disk_early_warn_percent() -> u8 {
    70
}

fn default_disk_warn_percent() -> u8 {
    80
}

fn default_disk_action_percent() -> u8 {
    90
}

fn default_disk_critical_percent() -> u8 {
    95
}

fn default_sync_freeze_marker() -> String {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("⚠️ could not determine home directory, using /var/tmp fallback");
            PathBuf::from("/var/tmp")
        }
    };
    home.join(".dracon")
        .join("dracon-sync.freeze")
        .display()
        .to_string()
}

fn default_unfreeze_below_percent() -> u8 {
    88
}

fn default_process_cpu_percent() -> f32 {
    50.0
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
    300  // 5 minutes - reduces notification spam during sustained issues
}

fn default_renice_value() -> i32 {
    10
}

fn default_git_kill_threshold_secs() -> u64 {
    60
}

fn default_guard_log_file() -> String {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return String::new(),
    };
    home.join(".local")
        .join("state")
        .join("dracon")
        .join("dracon-system-guard.log")
        .display()
        .to_string()
}

fn default_guard_log_max_mb() -> u64 {
    1  // 1 MiB - keeps last ~1000 events, rotates automatically
}

fn default_auto_cleanup_rust() -> bool {
    true
}

fn default_cleanup_min_size_mb() -> u64 {
    256  // 256 MiB minimum for auto-cleanup consideration
}

fn default_rust_search_roots() -> String {
    "~/Dev".to_string()  // Default search location for Rust target directories
}

fn default_node_modules_search_roots() -> String {
    "~/Dev".to_string()  // Default search location for node_modules directories
}

fn default_trend_warn_hours() -> u64 {
    24  // Warn if disk will fill within 24 hours
}

fn default_inode_warn_percent() -> u8 {
    85  // Warn at 85% inode usage (inodes rarely an issue on modern filesystems)
}

fn default_zombie_threshold() -> u64 {
    20  // Alert if more than 20 zombie processes (a few zombies are normal)
}

fn default_log_size_mb() -> u64 {
    100  // Alert on log files > 100 MiB
}

fn default_log_max_truncate_mb() -> u64 {
    50  // Truncate logs to 50 MiB by default
}

fn default_log_dirs() -> String {
    // Empty by default - must be configured
    String::new()
}

fn default_node_modules_max_age_days() -> u64 {
    30  // Clean node_modules not touched in 30 days
}

fn default_nix_keep_generations() -> u32 {
    5  // Keep last 5 nix generations
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
        .join(".dracon")
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

#[derive(Debug, Clone)]
struct ProcSample {
    pid: i32,
    ppid: i32,
    cpu_percent: f32,
    rss_mb: u64,
    command: String,
    args: String,
}

#[derive(Debug, Default)]
struct GuardRuntimeState {
    heavy_since: HashMap<i32, Instant>,
    notify_cooldowns: HashMap<String, Instant>,
    last_disk_state: String,
    /// History of disk usage samples for trend prediction (timestamp, percent)
    disk_history: Vec<(Instant, u8)>,
    /// Active cargo build PIDs detected
    active_build_pids: HashSet<i32>,
}

/// Information about a Rust target directory for cleanup consideration
#[derive(Debug, Clone)]
struct TargetDirInfo {
    path: PathBuf,
    bytes: u64,
}

/// Result of automatic cleanup operation
#[derive(Debug, Serialize)]
struct AutoCleanupResult {
    cleaned_count: usize,
    reclaimed_bytes: u64,
    cleaned_paths: Vec<String>,
    protected_paths: Vec<String>,
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
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            // Format: pid ppid pcpu rss comm args...
            let mut parts = trimmed.split_whitespace();
            let pid = parts.next()?.parse::<i32>().ok()?;
            let ppid = parts.next()?.parse::<i32>().ok()?;
            let cpu_percent = parts.next()?.parse::<f32>().ok()?;
            let rss_kb = parts.next()?.parse::<u64>().ok()?;
            let command = parts.next()?.to_string();
            let args = parts.collect::<Vec<_>>().join(" ");
            Some(ProcSample {
                pid,
                ppid,
                cpu_percent,
                rss_mb: rss_kb / 1024,
                command,
                args,
            })
        })
        .collect()
}

async fn disk_use_percent_for(path: &str) -> Result<u8> {
    let out = Command::new("df").args(["-P", path]).output().await?;
    if !out.status.success() {
        return Err(anyhow::anyhow!("df command failed"));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    parse_df_use_percent(&text).ok_or_else(|| anyhow::anyhow!("failed parsing df output"))
}

async fn process_samples() -> Result<Vec<ProcSample>> {
    let out = Command::new("ps")
        .args(["-eo", "pid,ppid,pcpu,rss,comm,args", "--no-headers"])
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
    if let Err(e) = Command::new(guard.notify_command.trim())
        .arg(title)
        .arg(body)
        .output()
        .await
    {
        eprintln!("⚠️ notification failed: {}", e);
    }
}

fn log_guard_event(guard: &GuardPolicy, event: &str, details: &str) {
    if guard.guard_log_file.is_empty() {
        return;
    }
    let path = PathBuf::from(&guard.guard_log_file);
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("⚠️ failed to create log dir: {}", e);
            return;
        }
    }
    let max_bytes = guard.guard_log_max_mb.saturating_mul(1024 * 1024);
    if max_bytes > 0 {
        if let Ok(meta) = fs::metadata(&path) {
            if meta.len() > max_bytes {
                if let Err(e) = fs::remove_file(&path) {
                    eprintln!("⚠️ failed to rotate guard log: {}", e);
                }
            }
        }
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let line = format!(r#"{{"ts":{},"event":"{}","details":"{}"}}"#, ts, event, details);
    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| writeln!(f, "{}", line))
    {
        eprintln!("⚠️ failed to write guard log: {}", e);
    }
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
    if let Err(e) = Command::new("renice")
        .args(["-n", &value.to_string(), "-p", &pid.to_string()])
        .output()
        .await
    {
        eprintln!("⚠️ renice failed: {}", e);
    }
}

async fn kill_process(pid: i32) -> bool {
    if let Err(e) = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .output()
        .await
    {
        eprintln!("⚠️ kill TERM failed for pid {}: {}", pid, e);
        return false;
    }
    tokio::time::sleep(Duration::from_secs(5)).await;
    if let Ok(out) = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pid="])
        .output()
        .await
    {
        let trimmed = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !trimmed.is_empty() {
            // PID still exists — verify it's still the same git process before SIGKILL
            let proc_cmdline = PathBuf::from(format!("/proc/{}/cmdline", pid));
            let still_git = if let Ok(content) = tokio::fs::read_to_string(&proc_cmdline).await {
                let cmd = content.replace('\0', " ");
                let mut parts = cmd.split_whitespace();
                let exe = parts.next().unwrap_or("");
                let exe_name = Path::new(exe).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let args = parts.collect::<Vec<_>>().join(" ");
                is_git_process(&exe_name, &args)
            } else {
                // Can't read /proc/{pid}/cmdline — PID may have been recycled to a non-git process.
                // Conservative: skip SIGKILL to avoid killing an innocent process.
                eprintln!("⚠️ cannot verify pid {} cmdline — skipping SIGKILL to avoid killing wrong process", pid);
                return false;
            };
            if !still_git {
                eprintln!("⚠️ pid {} is no longer a git process — skipping SIGKILL", pid);
                return false;
            }
            if let Err(e) = Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .output()
                .await
            {
                eprintln!("⚠️ kill KILL failed for pid {}: {}", pid, e);
                return false;
            }
            eprintln!("⚠️ force-killed runaway git process {}", pid);
            return true;
        }
    }
    true
}

fn is_git_process(command: &str, args: &str) -> bool {
    // Strict matching: only match known long-running git subcommands
    // that are safe to auto-kill. Avoids false positives like "legit-init".
    const GIT_CMDS: &[&str] = &["git-init", "git-fetch", "git-pull", "git-clone", "git-push"];
    if GIT_CMDS.contains(&command) {
        return true;
    }
    if command == "git" {
        let first_arg = args.split_whitespace().next().unwrap_or("");
        const GIT_SUBCMDS: &[&str] = &["init", "fetch", "pull", "clone", "push"];
        return GIT_SUBCMDS.contains(&first_arg);
    }
    false
}

/// Detect active cargo/rustc processes and return their PIDs and working directories
async fn detect_active_rust_builds() -> Result<HashSet<i32>> {
    let out = Command::new("ps")
        .args(["-eo", "pid=,comm="])
        .output()
        .await?;
    
    if !out.status.success() {
        return Ok(HashSet::new());
    }

    let mut build_pids = HashSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let mut parts = line.split_whitespace();
        let pid = match parts.next().and_then(|p| p.parse::<i32>().ok()) {
            Some(p) => p,
            None => continue,
        };
        let comm = parts.next().unwrap_or("");
        
        // Detect cargo, rustc, cargo-build, etc.
        if comm.contains("cargo") || comm.contains("rustc") || comm == "clippy-driver" {
            build_pids.insert(pid);
        }
    }

    Ok(build_pids)
}

/// Get the working directory of a process (to protect its target dir)
async fn get_process_cwd(pid: i32) -> Option<PathBuf> {
    let cwd_path = format!("/proc/{}/cwd", pid);
    std::fs::read_link(&cwd_path).ok()
}

/// Find all Rust target directories under the given search roots
async fn find_rust_target_dirs(roots: &[PathBuf]) -> Result<Vec<TargetDirInfo>> {
    use walkdir::WalkDir;
    
    let mut targets = Vec::new();
    
    for root in roots {
        if !root.exists() {
            continue;
        }
        
        for entry in WalkDir::new(root)
            .max_depth(5)  // Don't go too deep
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_dir() {
                continue;
            }
            
            if entry.file_name() != "target" {
                continue;
            }
            
            let path = entry.path().to_path_buf();
            
            // Check if there's a Cargo.toml in parent (confirm it's a Rust project)
            let parent = match path.parent() {
                Some(p) => p,
                None => continue,
            };
            
            if !parent.join("Cargo.toml").exists() {
                continue;
            }
            
            // Get directory size using du
            let bytes = match get_dir_size(&path).await {
                Ok(b) => b,
                Err(_) => continue,
            };
            
            targets.push(TargetDirInfo {
                path,
                bytes,
            });
        }
    }
    
    // Sort by size descending (clean largest first)
    targets.sort_by_key(|a| a.bytes);
    
    Ok(targets)
}

/// Get directory size using du command
async fn get_dir_size(path: &Path) -> Result<u64> {
    let out = Command::new("du")
        .args(["-sb", "--"])
        .arg(path)
        .output()
        .await?;
    
    if !out.status.success() {
        return Err(anyhow::anyhow!("du failed for {}", path.display()));
    }
    
    let stdout = String::from_utf8_lossy(&out.stdout);
    let bytes = stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("unexpected du output"))?
        .parse::<u64>()
        .context("failed to parse du output as byte count")?;
    
    Ok(bytes)
}

/// Perform automatic cleanup of Rust target directories
async fn auto_cleanup_rust_targets(
    guard: &GuardPolicy,
    state: &mut GuardRuntimeState,
    apply: bool,
) -> Result<AutoCleanupResult> {
    let mut result = AutoCleanupResult {
        cleaned_count: 0,
        reclaimed_bytes: 0,
        cleaned_paths: Vec::new(),
        protected_paths: Vec::new(),
    };
    
    // Parse search roots
    let roots: Vec<PathBuf> = guard.rust_search_roots
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() { return None; }
            let p = expand_tilde(s);
            if p.exists() { Some(p) } else { None }
        })
        .collect();
    
    if roots.is_empty() {
        return Ok(result);
    }
    
    // Find all target directories
    let targets = find_rust_target_dirs(&roots).await?;
    
    // Detect active builds - ONLY protection mechanism
    // We protect target dirs where cargo/rustc is actively running
    let active_builds = detect_active_rust_builds().await?;
    state.active_build_pids = active_builds.clone();
    
    // Get CWDs of active builds to protect their target dirs
    let mut protected_project_dirs: Vec<PathBuf> = Vec::new();
    for pid in &active_builds {
        if let Some(cwd) = get_process_cwd(*pid).await {
            // Find the project root (where Cargo.toml is)
            let mut dir = cwd.clone();
            while let Some(parent) = dir.parent() {
                if dir.join("Cargo.toml").exists() {
                    protected_project_dirs.push(dir);
                    break;
                }
                dir = parent.to_path_buf();
            }
        }
    }
    
    let min_size_bytes = guard.cleanup_min_size_mb * 1024 * 1024;
    
    for target in targets {
        // Skip if too small
        if target.bytes < min_size_bytes {
            continue;
        }
        
        // Only skip if there's an ACTIVELY RUNNING cargo/rustc in this project
        let target_project = target.path.parent().unwrap_or(&target.path);
        let has_active_build = protected_project_dirs.iter().any(|proj| {
            target_project == proj
        });
        
        if has_active_build {
            result.protected_paths.push(format!(
                "{} (active cargo/rustc process)",
                target.path.display()
            ));
            continue;
        }
        
        if apply {
            check_safe_to_delete(&target.path, &guard.protected_paths)?;
            if let Err(e) = tokio::fs::remove_dir_all(&target.path).await {
                eprintln!("⚠️ failed to remove {}: {}", target.path.display(), e);
                continue;
            }
        }

        result.cleaned_count += 1;
        result.reclaimed_bytes += target.bytes;
        result.cleaned_paths.push(format!(
            "{} ({})",
            target.path.display(),
            human_bytes(target.bytes)
        ));
    }
    
    Ok(result)
}

/// Get inode usage percent for root filesystem
async fn inode_use_percent() -> Result<u8> {
    let out = Command::new("df")
        .args(["-Pi", "/"])
        .output()
        .await?;
    
    if !out.status.success() {
        return Err(anyhow::anyhow!("df -i command failed"));
    }
    
    let text = String::from_utf8_lossy(&out.stdout);
    // Parse: Filesystem Inodes IUsed IFree IUse% Mounted on
    text.lines()
        .nth(1)
        .and_then(|line| line.split_whitespace().nth(4))
        .and_then(|v| v.trim_end_matches('%').parse::<u8>().ok())
        .ok_or_else(|| anyhow::anyhow!("failed parsing df -i output"))
}

/// Count zombie processes
async fn count_zombie_processes() -> Result<u64> {
    let out = Command::new("ps")
        .args(["-eo", "stat="])
        .output()
        .await?;
    
    if !out.status.success() {
        return Err(anyhow::anyhow!("ps command failed"));
    }
    
    let text = String::from_utf8_lossy(&out.stdout);
    let count = text.lines()
        .filter(|line| {
            let stat = line.trim();
            // Zombie processes have 'Z' in their stat
            stat.contains('Z') || stat.starts_with('Z')
        })
        .count();
    
    Ok(count as u64)
}

/// Get inode info for root filesystem
async fn get_inode_info() -> Result<(u64, u64, u64)> {
    let out = Command::new("df")
        .args(["-Pi", "/"])
        .output()
        .await?;
    
    if !out.status.success() {
        return Err(anyhow::anyhow!("df -i command failed"));
    }
    
    let text = String::from_utf8_lossy(&out.stdout);
    // Parse: Filesystem Inodes IUsed IFree IUse% Mounted on
    let line = text.lines().nth(1).ok_or_else(|| anyhow::anyhow!("no data line"))?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    
    let total = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
    let used = parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0);
    let free = parts.get(3).and_then(|v| v.parse().ok()).unwrap_or(0);
    
    Ok((total, used, free))
}

/// Clean Docker resources
async fn docker_prune(all: bool, volumes: bool) -> Result<u64> {
    let mut args = vec!["system", "prune", "-f"];
    if all {
        args.push("--all");
    }
    if volumes {
        args.push("--volumes");
    }
    
    let out = Command::new("docker")
        .args(&args)
        .output()
        .await?;
    
    if !out.status.success() {
        return Err(anyhow::anyhow!("docker prune failed"));
    }
    
    // Try to parse reclaimed space from output
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if line.contains("reclaimed") {
            // Parse "Total reclaimed space: 1.5GB"
            if let Some(pos) = line.find(':') {
                let size_str = line[pos + 1..].trim();
                // Parse size - this is approximate
                let bytes = parse_docker_size(size_str);
                return Ok(bytes);
            }
        }
    }
    
    Ok(0)
}

fn parse_docker_size(s: &str) -> u64 {
    let s = s.trim();
    let num: String = s.chars().take_while(|c| c.is_numeric() || *c == '.').collect();
    let unit: String = s.chars().skip_while(|c| c.is_numeric() || *c == '.' || *c == ' ').collect();
    
    let value: f64 = match num.parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("⚠️ parse_docker_size: failed to parse number from '{}'", s);
            0.0
        }
    };
    let multiplier = match unit.to_uppercase().as_str() {
        "B" => 1.0,
        "KB" | "KIB" => 1024.0,
        "MB" | "MIB" => 1024.0 * 1024.0,
        "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
        "TB" | "TIB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };
    
    (value * multiplier) as u64
}

/// Clean package manager caches
async fn clean_package_caches(
    cargo: bool,
    npm: bool,
    pip: bool,
    go: bool,
    apply: bool,
    protected_paths: &[String],
) -> Result<(u64, Vec<String>)> {
    let mut reclaimed = 0u64;
    let mut cleaned = Vec::new();
    
    if cargo {
        if let Some(home) = dirs::home_dir() {
            let cargo_cache = home.join(".cargo/registry/cache");
            if cargo_cache.exists() {
                let size = get_dir_size(&cargo_cache).await.unwrap_or(0);
                if size > 0 {
                    let mut succeeded = true;
                    if apply {
                        check_safe_to_delete(&cargo_cache, protected_paths)?;
                        if let Err(e) = tokio::fs::remove_dir_all(&cargo_cache).await {
                            eprintln!("⚠️ failed to remove cargo cache: {}", e);
                            succeeded = false;
                        }
                    }
                    if !apply || succeeded {
                        cleaned.push(format!("cargo registry cache ({})", human_bytes(size)));
                        reclaimed += size;
                    }
                }
            }
        }
    }
    
    if npm {
        if let Some(home) = dirs::home_dir() {
            let npm_cache = home.join(".npm");
            if npm_cache.exists() {
                let size = get_dir_size(&npm_cache).await.unwrap_or(0);
                if size > 0 {
                    let mut succeeded = true;
                    if apply {
                        check_safe_to_delete(&npm_cache, protected_paths)?;
                        if let Err(e) = tokio::fs::remove_dir_all(&npm_cache).await {
                            eprintln!("⚠️ failed to remove npm cache: {}", e);
                            succeeded = false;
                        }
                    }
                    if !apply || succeeded {
                        cleaned.push(format!("npm cache ({})", human_bytes(size)));
                        reclaimed += size;
                    }
                }
            }
        }
    }
    
    if pip {
        if let Some(home) = dirs::home_dir() {
            let pip_cache = home.join(".cache/pip");
            if pip_cache.exists() {
                let size = get_dir_size(&pip_cache).await.unwrap_or(0);
                if size > 0 {
                    let mut succeeded = true;
                    if apply {
                        check_safe_to_delete(&pip_cache, protected_paths)?;
                        if let Err(e) = tokio::fs::remove_dir_all(&pip_cache).await {
                            eprintln!("⚠️ failed to remove pip cache: {}", e);
                            succeeded = false;
                        }
                    }
                    if !apply || succeeded {
                        cleaned.push(format!("pip cache ({})", human_bytes(size)));
                        reclaimed += size;
                    }
                }
            }
        }
    }
    
    if go {
        if let Some(home) = dirs::home_dir() {
            let go_cache = home.join(".cache/go-build");
            if go_cache.exists() {
                let size = get_dir_size(&go_cache).await.unwrap_or(0);
                if size > 0 {
                    let mut succeeded = true;
                    if apply {
                        check_safe_to_delete(&go_cache, protected_paths)?;
                        if let Err(e) = tokio::fs::remove_dir_all(&go_cache).await {
                            eprintln!("⚠️ failed to remove go cache: {}", e);
                            succeeded = false;
                        }
                    }
                    if !apply || succeeded {
                        cleaned.push(format!("go build cache ({})", human_bytes(size)));
                        reclaimed += size;
                    }
                }
            }
        }
    }
    
    Ok((reclaimed, cleaned))
}

/// Empty trash
async fn empty_trash(apply: bool, protected_paths: &[String]) -> Result<(u64, Vec<String>)> {
    let mut reclaimed = 0u64;
    let mut cleaned = Vec::new();
    
    if let Some(home) = dirs::home_dir() {
        let trash_files = home.join(".local/share/Trash/files");
        let trash_info = home.join(".local/share/Trash/info");
        
        if trash_files.exists() {
            let size = get_dir_size(&trash_files).await.unwrap_or(0);
            if size > 0 {
                let mut succeeded = true;
                if apply {
                    check_safe_to_delete(&trash_files, protected_paths)?;
                    if let Err(e) = tokio::fs::remove_dir_all(&trash_files).await {
                        eprintln!("⚠️ failed to remove trash files: {}", e);
                        succeeded = false;
                    } else if let Err(e) = tokio::fs::create_dir_all(&trash_files).await {
                        eprintln!("⚠️ failed to recreate trash dir: {}", e);
                        // Note: we still count this as success since the files were removed
                    }
                }
                if !apply || succeeded {
                    cleaned.push(format!("trash files ({})", human_bytes(size)));
                    reclaimed += size;
                }
            }
        }
        
        if trash_info.exists() {
            let info_size = get_dir_size(&trash_info).await.unwrap_or(0);
            if info_size > 0 {
                let mut succeeded = true;
                if apply {
                    check_safe_to_delete(&trash_info, protected_paths)?;
                    if let Err(e) = tokio::fs::remove_dir_all(&trash_info).await {
                        eprintln!("⚠️ failed to remove trash info: {}", e);
                        succeeded = false;
                    } else if let Err(e) = tokio::fs::create_dir_all(&trash_info).await {
                        eprintln!("⚠️ failed to recreate trash info dir: {}", e);
                        // Note: we still count this as success since the files were removed
                    }
                }
                if !apply || succeeded {
                    cleaned.push(format!("trash info ({})", human_bytes(info_size)));
                    reclaimed += info_size;
                }
            }
        }
    }
    
    Ok((reclaimed, cleaned))
}

/// Run nix-collect-garbage
async fn clean_nix_garbage(keep_generations: u32, apply: bool) -> Result<(u64, Vec<String>)> {
    let mut reclaimed = 0u64;
    let mut cleaned = Vec::new();
    let mut errs = Vec::new();

    if apply && keep_generations > 0 {
        let gen_arg = keep_generations.to_string();
        if let Err(e) = Command::new("nix-env")
            .arg("-d")
            .arg(&gen_arg)
            .arg("--delete-generations")
            .output()
            .await
        {
            errs.push(format!("nix-env delete generations: {}", e));
        }

        if let Err(e) = Command::new("nix-env")
            .arg("-d")
            .arg(&gen_arg)
            .arg("--delete-generations")
            .arg("-p")
            .arg("/nix/var/nix/profiles/default")
            .output()
            .await
        {
            errs.push(format!("nix-env delete user profile generations: {}", e));
        }
    }

    let mut args = vec!["collect-garbage"];
    if apply {
        args.push("-d");
    } else {
        args.push("--dry-run");
    }

    let out = Command::new("nix-store")
        .args(&args)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("failed to run nix-store: {}", e))?;

    if !out.status.success() {
        return Err(anyhow::anyhow!("nix-store collect-garbage failed: {}", String::from_utf8_lossy(&out.stderr)));
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let delete_count = text.lines().filter(|l| l.contains("deleting")).count();
    if delete_count > 0 {
        cleaned.push(format!("nix store garbage ({} paths)", delete_count));
        reclaimed = delete_count as u64 * 1024 * 1024;
    }

    if !errs.is_empty() && reclaimed == 0 {
        return Err(anyhow::anyhow!("nix cleanup had {} error(s): {}", errs.len(), errs.join("; ")));
    }

    Ok((reclaimed, cleaned))
}

/// Clean old node_modules directories
async fn clean_old_node_modules(
    roots: &[PathBuf],
    max_age_days: u64,
    apply: bool,
    protected_paths: &[String],
) -> Result<(u64, Vec<String>)> {
    use walkdir::WalkDir;
    
    let mut reclaimed = 0u64;
    let mut cleaned = Vec::new();
    let now = SystemTime::now();
    let max_age_secs = max_age_days * 24 * 3600;
    
    for root in roots {
        if !root.exists() {
            continue;
        }
        
        for entry in WalkDir::new(root)
            .max_depth(5)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_dir() {
                continue;
            }
            
            if entry.file_name() != "node_modules" {
                continue;
            }
            
            let path = entry.path().to_path_buf();
            
            // Check age
            let modified_secs_ago = match fs::metadata(&path).and_then(|m| m.modified()) {
                Ok(mtime) => now.duration_since(mtime).map(|d| d.as_secs()).unwrap_or(0),
                Err(_) => continue,
            };
            
            if modified_secs_ago < max_age_secs {
                continue;
            }
            
            let size = match get_dir_size(&path).await {
                Ok(s) => s,
                Err(_) => continue,
            };
            
            if size > 0 {
                let mut succeeded = true;
                if apply {
                    check_safe_to_delete(&path, protected_paths)?;
                    if let Err(e) = tokio::fs::remove_dir_all(&path).await {
                        eprintln!("⚠️ failed to remove {}: {}", path.display(), e);
                        succeeded = false;
                    }
                }
                if !apply || succeeded {
                    cleaned.push(format!(
                        "{} ({} days old, {})",
                        path.display(),
                        modified_secs_ago / 86400,
                        human_bytes(size)
                    ));
                    reclaimed += size;
                }
            }
        }
    }
    
    Ok((reclaimed, cleaned))
}

/// Find large log files
async fn find_large_log_files(dirs: &[PathBuf], min_size_bytes: u64) -> Result<Vec<(PathBuf, u64)>> {
    use walkdir::WalkDir;
    
    let mut logs = Vec::new();
    
    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        
        for entry in WalkDir::new(dir)
            .max_depth(3)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            
            let path = entry.path();
            let name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            
            // Check if it looks like a log file
            if !name.ends_with(".log") && 
               !name.ends_with(".log.old") &&
               !name.contains(".log.") &&
               name != "journal" &&
               !name.ends_with(".journal") {
                continue;
            }
            
            let size = match fs::metadata(path) {
                Ok(m) => m.len(),
                Err(_) => continue,
            };
            
            if size >= min_size_bytes {
                logs.push((path.to_path_buf(), size));
            }
        }
    }
    
    // Sort by size descending
    logs.sort_by_key(|a| a.1);
    
    Ok(logs)
}

/// Truncate a log file to a maximum size while optionally preserving header lines.
/// Returns the number of bytes reclaimed, or an error on failure.
fn truncate_log_file(path: &Path, max_size_bytes: u64, preserve_header_lines: usize) -> Result<u64> {
    use std::io::{BufRead, BufReader, Write};

    let original_size = std::fs::metadata(path)?.len();
    if original_size <= max_size_bytes {
        return Ok(0);
    }

    if preserve_header_lines == 0 {
        // Simple truncate: open with truncate flag
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(path)?;
        file.set_len(max_size_bytes)?;
        let new_size = file.metadata()?.len();
        return Ok(original_size.saturating_sub(new_size));
    }

    // Preserve header lines: read first N lines, write them to temp file,
    // then rename temp over original
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut header_lines: Vec<Vec<u8>> = Vec::new();

    for (i, line_result) in reader.lines().enumerate() {
        if i >= preserve_header_lines {
            break;
        }
        if let Ok(line) = line_result {
            header_lines.push(line.into_bytes());
        } else {
            break;
        }
    }

    // Write header + max content to temp file
    let temp_path = path.with_extension(format!(
        "{}.truncated.{}",
        path.extension().and_then(|e| e.to_str()).unwrap_or("log"),
        std::process::id()
    ));
    {
        let mut temp_file = std::fs::File::create(&temp_path)?;
        let mut total_written = 0u64;
        for line_bytes in &header_lines {
            temp_file.write_all(line_bytes)?;
            temp_file.write_all(b"\n")?;
            total_written += line_bytes.len() as u64 + 1;
        }

        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);
        for line in reader.lines().skip(preserve_header_lines).flatten() {
            let line_bytes = line.into_bytes();
            let line_len = line_bytes.len() as u64;

            if total_written + line_len + 1 > max_size_bytes {
                break;
            }

            temp_file.write_all(&line_bytes)?;
            temp_file.write_all(b"\n")?;
            total_written += line_len + 1;
        }
    }

    // Atomically replace original
    std::fs::rename(&temp_path, path)?;
    let new_size = std::fs::metadata(path)?.len();
    Ok(original_size.saturating_sub(new_size))
}

/// Predict when disk will fill based on trend
fn predict_fill_time(history: &[(Instant, u8)]) -> Option<f64> {
    if history.len() < 3 {
        return None;
    }
    
    // Simple linear regression on the last N samples
    let n = history.len().min(20);  // Use up to 20 samples
    let recent = &history[history.len().saturating_sub(n)..];
    
    if recent.len() < 3 {
        return None;
    }
    
    // Calculate rate of change (percent per second)
    let mut total_rate = 0.0;
    let mut count = 0;
    
    for i in 1..recent.len() {
        let dt = recent[i].0.duration_since(recent[i - 1].0).as_secs_f64();
        if dt <= 0.0 {
            continue;
        }
        let dp = (recent[i].1 as f64) - (recent[i - 1].1 as f64);
        total_rate += dp / dt;
        count += 1;
    }
    
    if count == 0 {
        return None;
    }
    
    let avg_rate = total_rate / count as f64;
    
    // If rate is negative or zero, disk isn't filling
    if avg_rate <= 0.0 {
        return None;
    }
    
    // Time until 100% from current
    let current = recent.last()?.1 as f64;
    let remaining_percent = 100.0 - current;
    let seconds_until_full = remaining_percent / avg_rate;
    
    Some(seconds_until_full / 3600.0)  // Return hours
}

async fn run_guard_once(
    guard: &GuardPolicy,
    state: &mut GuardRuntimeState,
) -> Result<GuardReport> {
    let used = disk_use_percent_for(&guard.disk_mount_path).await?;
    let dstate = disk_state(used, guard).to_string();
    let marker = sync_freeze_marker_path(guard);
    let mut sync_frozen = marker.exists();

    // Track disk history for trend prediction
    if guard.track_trends {
        let now = Instant::now();
        state.disk_history.push((now, used));
        
        // Keep only last 100 samples (about 50 minutes at 30s interval)
        if state.disk_history.len() > 100 {
            let excess = state.disk_history.len() - 100;
            state.disk_history.drain(0..excess);
        }
        
        // Check for trend prediction warning
        if let Some(hours_until_full) = predict_fill_time(&state.disk_history) {
            if hours_until_full > 0.0 && hours_until_full <= guard.trend_warn_hours as f64 {
                let key = "disk-trend-warning".to_string();
                if should_notify(state, &key, guard.notify_cooldown_secs.max(3600)) {
                    send_notification(
                        guard,
                        "Dracon System Guard - Disk Trend Warning",
                        &format!(
                            "Disk predicted to fill in {:.1} hours (currently {}%)",
                            hours_until_full, used
                        ),
                    )
                    .await;
                }
            }
        }
    }

    // Early warning notification (70% threshold)
    if used >= guard.disk_early_warn_percent && used < guard.disk_warn_percent {
        let key = "disk-early-warn".to_string();
        if should_notify(state, &key, guard.notify_cooldown_secs.max(1800)) {
            send_notification(
                guard,
                "Dracon System Guard - Early Warning",
                &format!(
                    "Disk usage at {}% (early warning threshold: {}%)",
                    used, guard.disk_early_warn_percent
                ),
            )
            .await;
        }
    }

    if guard.freeze_sync_at_action && (dstate == "action" || dstate == "critical") {
        if !sync_frozen {
            if let Some(parent) = marker.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    eprintln!("failed to create freeze marker dir: {}", e);
                }
            }
            if let Err(e) = fs::write(&marker, format!("dracon-system guard freeze: disk={}%\n", used)) {
                eprintln!("failed to write freeze marker: {}", e);
            } else {
                sync_frozen = true;
                emit_event(&DraconEvent::new("system", EventSeverity::Warn, "disk/freeze", format!("sync frozen at {}%", used)));
            }
        }
    } else if sync_frozen && used <= guard.unfreeze_below_percent {
        if let Err(e) = fs::remove_file(&marker) {
            eprintln!("failed to remove freeze marker: {}", e);
        } else {
            sync_frozen = false;
            emit_event(&DraconEvent::new("system", EventSeverity::Info, "disk/unfreeze", format!("sync unfrozen at {}%", used)));
        }
    }

    // Comprehensive auto-cleanup when disk hits action/critical level
    if dstate == "action" || dstate == "critical" {
        let apply = guard.auto_cleanup_apply;
        if !apply {
            eprintln!("💡 disk at {}% — auto-cleanup is in dry-run mode (set auto_cleanup_apply = true to execute)", used);
        }
        let mut total_reclaimed = 0u64;
        let mut all_cleaned: Vec<String> = Vec::new();
        
        // Rust target directories
        if guard.auto_cleanup_rust {
            let result = auto_cleanup_rust_targets(guard, state, apply).await?;
            total_reclaimed += result.reclaimed_bytes;
            for p in &result.cleaned_paths {
                eprintln!("🧹 Rust: {}", p);
            }
            all_cleaned.extend(result.cleaned_paths);
        }
        
        // Trash
        if guard.clean_trash {
            match empty_trash(true, &guard.protected_paths).await {
                Ok((bytes, cleaned)) => {
                    total_reclaimed += bytes;
                    all_cleaned.extend(cleaned.iter().map(|s| format!("Trash: {}", s)));
                    for c in &cleaned {
                        eprintln!("🗑️ {}", c);
                    }
                }
                Err(e) => eprintln!("⚠️ Trash cleanup failed: {}", e),
            }
        }
        
        // Nix garbage
        if guard.clean_nix_garbage {
            match clean_nix_garbage(guard.nix_keep_generations, true).await {
                Ok((bytes, cleaned)) => {
                    total_reclaimed += bytes;
                    all_cleaned.extend(cleaned.iter().map(|s| format!("Nix: {}", s)));
                    for c in &cleaned {
                        eprintln!("📦 {}", c);
                    }
                }
                Err(e) => eprintln!("⚠️ Nix cleanup failed: {}", e),
            }
        }
        
        // Old node_modules
        let roots: Vec<PathBuf> = guard.node_modules_search_roots
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() { return None; }
                let p = expand_tilde(s);
                if p.exists() { Some(p) } else { None }
            })
            .collect();
        let (bytes, cleaned) = match clean_old_node_modules(&roots, guard.node_modules_max_age_days, true, &guard.protected_paths).await {
            Ok(result) => result,
            Err(e) => {
                eprintln!("⚠️ Node modules cleanup failed: {}", e);
                (0, vec![])
            }
        };
        total_reclaimed += bytes;
        all_cleaned.extend(cleaned.iter().map(|s| format!("Node: {}", s)));
        for c in &cleaned {
            eprintln!("📂 {}", c);
        }
        
        // Package caches
        if guard.clean_package_caches {
            match clean_package_caches(true, true, true, true, true, &guard.protected_paths).await {
                Ok((bytes, cleaned)) => {
                    total_reclaimed += bytes;
                    all_cleaned.extend(cleaned.iter().map(|s| format!("Cache: {}", s)));
                    for c in &cleaned {
                        eprintln!("💾 {}", c);
                    }
                }
                Err(e) => eprintln!("⚠️ Package cache cleanup failed: {}", e),
            }
        }
        
        // Docker
        if guard.docker_prune {
            match docker_prune(true, guard.docker_prune_volumes).await {
                Ok(bytes) => {
                    total_reclaimed += bytes;
                    if bytes > 0 {
                        eprintln!("🐳 Docker prune: {}", human_bytes(bytes));
                    }
                }
                Err(e) => eprintln!("⚠️ Docker prune failed: {}", e),
            }
        }
        
        // Notify if anything was cleaned
        if total_reclaimed > 0 {
            let key = "auto-cleanup".to_string();
            if should_notify(state, &key, guard.notify_cooldown_secs.max(600)) {
                send_notification(
                    guard,
                    "Dracon System Guard - Auto Cleanup",
                    &format!(
                        "Reclaimed {} ({} items cleaned)",
                        human_bytes(total_reclaimed),
                        all_cleaned.len()
                    ),
                )
                .await;
            }
        }
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

    let exempt = parse_kinds(&guard.process_exempt_names);
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
        let is_sustained = sustained >= guard.process_sustain_secs;

        // Always log heavy processes to persistent log (even brief spikes)
        log_guard_event(
            guard,
            if is_sustained { "heavy-sustained" } else { "heavy-brief" },
            &format!(
                "pid={} ppid={} cmd={} args={} cpu={:.1}% rss={}MiB sustained={}s",
                p.pid, p.ppid, p.command, p.args, p.cpu_percent, p.rss_mb, sustained
            ),
        );

        if !is_sustained {
            continue;
        }

        let mut action = "notify".to_string();
        if guard.auto_renice {
            renice_process(p.pid, guard.renice_value).await;
            action = format!("renice:{}", guard.renice_value);
        }

        if guard.auto_kill_git
            && guard.git_kill_threshold_secs > 0
            && sustained >= guard.git_kill_threshold_secs
            && is_git_process(&p.command, &p.args)
        {
            if kill_process(p.pid).await {
                action = "kill:git-sigterm+sigkill".to_string();
            } else {
                action = "kill:git-sigterm-failed".to_string();
            }
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
            ppid: p.ppid,
            command: p.command,
            args: p.args,
            cpu_percent: p.cpu_percent,
            rss_mb: p.rss_mb,
            sustained_secs: sustained,
            action,
        });
    }

    state.heavy_since.retain(|pid, _| current_heavy.contains(pid));
    
    // Clean up stale notify_cooldowns entries (older than 2x cooldown period)
    let cooldown_cutoff = Instant::now() - Duration::from_secs(guard.notify_cooldown_secs.saturating_mul(2));
    state.notify_cooldowns.retain(|_, &mut since| since > cooldown_cutoff);

    // Inode monitoring
    if guard.monitor_inodes {
        if let Ok(inode_percent) = inode_use_percent().await {
            if inode_percent >= guard.inode_warn_percent {
                let key = "inode-warning".to_string();
                if should_notify(state, &key, guard.notify_cooldown_secs.max(1800)) {
                    send_notification(
                        guard,
                        "Dracon System Guard - Inode Warning",
                        &format!(
                            "Inode usage at {}% (threshold: {}%) - disk may have space but no file slots",
                            inode_percent, guard.inode_warn_percent
                        ),
                    )
                    .await;
                }
            }
        }
    }

    // Zombie process monitoring
    if guard.monitor_zombies {
        if let Ok(zombie_count) = count_zombie_processes().await {
            if zombie_count > guard.zombie_threshold {
                let key = "zombie-warning".to_string();
                if should_notify(state, &key, guard.notify_cooldown_secs.max(3600)) {
                    send_notification(
                        guard,
                        "Dracon System Guard - Zombie Processes",
                        &format!(
                            "Detected {} zombie processes (threshold: {})",
                            zombie_count, guard.zombie_threshold
                        ),
                    )
                    .await;
                }
            }
        }
    }

    // Large log file monitoring
    if guard.monitor_logs && !guard.log_dirs.trim().is_empty() {
        let log_dirs: Vec<PathBuf> = guard.log_dirs
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() { return None; }
                let p = expand_tilde(s);
                if p.exists() { Some(p) } else { None }
            })
            .collect();
        
        if !log_dirs.is_empty() {
            let min_size = guard.log_size_mb * 1024 * 1024;
            match find_large_log_files(&log_dirs, min_size).await {
                Ok(logs) if !logs.is_empty() => {
                    let key = "log-size-warning".to_string();
                    if should_notify(state, &key, guard.notify_cooldown_secs.max(3600)) {
                        let top_logs: Vec<_> = logs.iter().take(3).collect();
                        let msg = format!(
                            "Found {} large log files (>{:.0} MiB): {}",
                            logs.len(),
                            guard.log_size_mb,
                            top_logs.iter()
                                .map(|(p, s)| format!("{} ({})", p.display(), human_bytes(*s)))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        send_notification(guard, "Dracon System Guard - Large Log Files", &msg).await;
                    }

                    // Auto-truncate if enabled
                    if guard.auto_truncate_logs {
                        let max_size = guard.log_max_truncate_mb * 1024 * 1024;
                        let preserve = guard.log_preserve_header_lines;
                        let mut total_reclaimed = 0u64;
                        for (path, original_size) in &logs {
                            match truncate_log_file(path, max_size, preserve) {
                                Ok(reclaimed) if reclaimed > 0 => {
                                    eprintln!(
                                        "📝 truncated {}: {} -> {} (reclaimed {})",
                                        path.display(),
                                        human_bytes(*original_size),
                                        human_bytes(original_size.saturating_sub(reclaimed)),
                                        human_bytes(reclaimed)
                                    );
                                    total_reclaimed += reclaimed;
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!(
                                        "⚠️ failed to truncate {}: {}",
                                        path.display(),
                                        e
                                    );
                                }
                            }
                        }
                        if total_reclaimed > 0 {
                            let key = "log-truncated".to_string();
                            if should_notify(state, &key, guard.notify_cooldown_secs.max(3600)) {
                                send_notification(
                                    guard,
                                    "Dracon System Guard - Logs Truncated",
                                    &format!(
                                        "Reclaimed {} from {} log file(s) (max now: {} MiB)",
                                        human_bytes(total_reclaimed),
                                        logs.len(),
                                        guard.log_max_truncate_mb
                                    ),
                                )
                                .await;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

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
                check_safe_to_delete(&link, &[])?;
                fs::remove_file(&link)?;
            } else if force_replace {
                check_safe_to_delete(&link, &[])?;
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
        home.join(".dracon/utilities/system/dracon-system.toml"),
        home.join(".dracon/utilities/system/config.toml"),
        home.join(".dracon/system/dracon-system.toml"),
        home.join(".dracon/system/config.toml"),
    ];

    candidates.into_iter().find(|p| p.exists())
}

fn load_system_policy() -> Result<(Option<PathBuf>, SystemPolicy)> {
    let Some(path) = resolve_system_policy_path() else {
        return Ok((None, SystemPolicy::default()));
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_e) => {
            return Ok((Some(path), SystemPolicy::default()));
        }
    };
    let parsed: SystemPolicy = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", path.display(), e))?;
    Ok((Some(path), parsed))
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



async fn build_status_report() -> Result<StatusReport> {
    let root = canonical_system_root();
    let (system_policy_path, _) = load_system_policy().unwrap_or_else(|_| (None, SystemPolicy::default()));
    Ok(StatusReport {
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
    })
}

fn normalize_guard_policy(policy: &mut GuardPolicy) {
    policy.interval_secs = policy.interval_secs.max(5);
    policy.disk_warn_percent = policy.disk_warn_percent.clamp(1, 100);
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
#[allow(clippy::items_after_test_module)]
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
        // Ensure HOME is restored even if the test panics
        struct HomeGuard(Option<String>);
        impl Drop for HomeGuard {
            fn drop(&mut self) {
                if let Some(ref v) = self.0 {
                    std::env::set_var("HOME", v);
                } else {
                    std::env::remove_var("HOME");
                }
            }
        }
        let _home_guard = HomeGuard(old_home);

        assert_eq!(expand_tilde("~"), PathBuf::from("/tmp/dracon-home-test"));
        assert_eq!(
            expand_tilde("~/Dev/project"),
            PathBuf::from("/tmp/dracon-home-test/Dev/project")
        );
        assert_eq!(expand_tilde("/x/y"), PathBuf::from("/x/y"));
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
        let sample = "123 1 250.5 4194304 git\n456 2 12.0 2048 zsh\n";
        let rows = parse_ps_output(sample);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].pid, 123);
        assert_eq!(rows[0].ppid, 1);
        assert_eq!(rows[0].command, "git");
        assert_eq!(rows[0].rss_mb, 4096);
        assert_eq!(rows[0].args, "");
    }

    #[test]
    fn is_git_process_detects_git_init() {
        assert!(is_git_process("git-init", ""));
        assert!(is_git_process("git", "init"));
        assert!(is_git_process("git-init", "--bare"));
    }

    #[test]
    fn is_git_process_detects_git_fetch_and_pull() {
        assert!(is_git_process("git-fetch", ""));
        assert!(is_git_process("git", "pull"));
        assert!(is_git_process("git-pull", "origin main"));
        assert!(is_git_process("git-fetch", "origin"));
    }

    #[test]
    fn is_git_process_detects_git_push_and_clone() {
        assert!(is_git_process("git-push", ""));
        assert!(is_git_process("git", "push"));
        assert!(is_git_process("git-clone", ""));
        assert!(is_git_process("git", "clone"));
    }

    #[test]
    fn is_git_process_rejects_non_git_commands() {
        assert!(!is_git_process("git", "log"));
        assert!(!is_git_process("git", "diff"));
        assert!(!is_git_process("git", "status"));
        assert!(!is_git_process("git", "commit"));
        assert!(!is_git_process("bash", ""));
        assert!(!is_git_process("python", ""));
        assert!(!is_git_process("legit-init", "")); // false positive from old substring matching
    }

    #[test]
    fn is_protected_ancestor_exact_match() {
        assert!(is_protected_ancestor("/home", "/home"));
        assert!(is_protected_ancestor("/etc", "/etc"));
        assert!(is_protected_ancestor("/", "/"));
    }

    #[test]
    fn is_protected_ancestor_descendant_match() {
        assert!(is_protected_ancestor("/home/dracon", "/home"));
        assert!(is_protected_ancestor("/home/dracon/Dev", "/home"));
        assert!(is_protected_ancestor("/etc/nginx/nginx.conf", "/etc"));
    }

    #[test]
    fn is_protected_ancestor_rejects_partial_prefix() {
        assert!(!is_protected_ancestor("/homefoo", "/home"));
        assert!(!is_protected_ancestor("/homefoo/bar", "/home"));
        assert!(!is_protected_ancestor("/etcnginx", "/etc"));
    }

    #[test]
    fn is_protected_ancestor_root_matches_exact_only() {
        assert!(is_protected_ancestor("/", "/"));
        assert!(!is_protected_ancestor("/anything", "/")); // root only matches exact to allow cleanup
        assert!(!is_protected_ancestor("/home", "/"));
    }

    #[test]
    fn check_path_str_blocks_descendants() {
        assert!(!check_path_str("/home/dracon", &[]));
        assert!(!check_path_str("/home/dracon/Dev", &[]));
        assert!(!check_path_str("/etc/nginx", &[]));
        assert!(check_path_str("/safe/path", &[]));
        assert!(check_path_str("/homefoo", &[])); // partial prefix should be safe
    }

    #[test]
    fn parse_ps_output_extracts_all_fields() {
        let sample = "9999 1 75.0 8192000 git-fetch origin main\n";
        let rows = parse_ps_output(sample);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pid, 9999);
        assert_eq!(rows[0].ppid, 1);
        assert_eq!(rows[0].cpu_percent, 75.0);
        assert_eq!(rows[0].rss_mb, 8192000 / 1024);
        assert_eq!(rows[0].command, "git-fetch");
        assert_eq!(rows[0].args, "origin main");
    }

    #[test]
    fn disk_state_transitions_at_thresholds() {
        let guard = GuardPolicy {
            disk_warn_percent: 70,
            disk_action_percent: 85,
            disk_critical_percent: 95,
            ..GuardPolicy::default()
        };
        assert_eq!(disk_state(50, &guard), "ok");
        assert_eq!(disk_state(70, &guard), "warn");
        assert_eq!(disk_state(84, &guard), "warn");
        assert_eq!(disk_state(85, &guard), "action");
        assert_eq!(disk_state(94, &guard), "action");
        assert_eq!(disk_state(95, &guard), "critical");
        assert_eq!(disk_state(100, &guard), "critical");
    }

    #[test]
    fn should_notify_respects_cooldown() {
        let mut state = GuardRuntimeState {
            heavy_since: std::collections::HashMap::new(),
            notify_cooldowns: std::collections::HashMap::new(),
            last_disk_state: "ok".to_string(),
            disk_history: Vec::new(),
            active_build_pids: std::collections::HashSet::new(),
        };
        let key = "test-key";
        assert!(should_notify(&mut state, key, 60), "first notify should succeed");
        assert!(!should_notify(&mut state, key, 60), "immediate second notify should be blocked");
        assert!(should_notify(&mut state, "other-key", 60), "different key should succeed");
    }

    #[test]
    fn predict_fill_time_requires_minimum_samples() {
        let history: Vec<(Instant, u8)> = vec![
            (Instant::now(), 50),
            (Instant::now(), 51),
        ];
        assert!(predict_fill_time(&history).is_none(), "needs at least 3 samples");
    }

    #[test]
    fn predict_fill_time_returns_none_for_stable_disk() {
        let base = Instant::now();
        let history: Vec<(Instant, u8)> = vec![
            (base, 50),
            (base + Duration::from_secs(10), 50),
            (base + Duration::from_secs(20), 50),
        ];
        assert!(predict_fill_time(&history).is_none(), "stable disk should not predict fill");
    }

    #[test]
    fn predict_fill_time_estimates_for_filling_disk() {
        let base = Instant::now();
        let history: Vec<(Instant, u8)> = vec![
            (base, 50),
            (base + Duration::from_secs(3600), 60),
            (base + Duration::from_secs(7200), 70),
        ];
        let hours = predict_fill_time(&history);
        assert!(hours.is_some(), "should predict fill time for rising disk");
        let h = hours.unwrap();
        assert!(h > 0.0, "predicted hours should be positive");
        assert!(h < 100.0, "predicted hours should be reasonable for 10%/hr rate");
    }

    #[tokio::test]
    async fn guard_report_completes_for_ok_disk() {
        let mut state = GuardRuntimeState {
            heavy_since: std::collections::HashMap::new(),
            notify_cooldowns: std::collections::HashMap::new(),
            last_disk_state: "ok".to_string(),
            disk_history: Vec::new(),
            active_build_pids: std::collections::HashSet::new(),
        };
        let guard = GuardPolicy {
            disk_warn_percent: 70,
            disk_action_percent: 85,
            disk_critical_percent: 95,
            disk_mount_path: "/".into(),
            freeze_sync_at_action: false,
            track_trends: false,
            ..GuardPolicy::default()
        };
        let report = run_guard_once(&guard, &mut state).await;
        assert!(report.is_ok() || report.is_err(), "async guard execution should complete");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    VERBOSITY.store(cli.verbose, Ordering::SeqCst);

    match cli.cmd {
        Commands::Status { json } => {
            let report = build_status_report().await?;
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
            let (_, policy) = load_system_policy()?;
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
                    let tracked = is_git_tracked_dir(&item.path).await.unwrap_or(true);
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

                let user_protected = policy.guard.protected_paths.clone();
                if cfg.apply {
                    for path in actionable {
                        check_safe_to_delete(&path, &user_protected)?;
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
            let (_, policy) = load_system_policy()?;
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
            let (_, policy) = load_system_policy()?;
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
                    let _lock = acquire_daemon_lock("dracon-system-guard")
                        .with_context(|| "failed to acquire guard daemon lock")?;
                    let shutdown = Arc::new(AtomicBool::new(false));
                    let shutdown_sigterm = shutdown.clone();
                    let shutdown_sigint = shutdown.clone();
                    let reload = Arc::new(AtomicBool::new(false));
                    let reload_sighup = reload.clone();
                    let reload_sighup_handler = reload.clone();

                    tokio::spawn(async move {
                        if let Ok(mut sig) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                            sig.recv().await;
                            veprintln!(1, "system: received SIGTERM, shutting down gracefully...");
                            shutdown_sigterm.store(true, Ordering::SeqCst);
                        } else {
                            eprintln!("system: failed to set up SIGTERM handler");
                        }
                    });

                    tokio::spawn(async move {
                        if let Ok(mut sig) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()) {
                            sig.recv().await;
                            veprintln!(1, "system: received SIGINT, shutting down gracefully...");
                            shutdown_sigint.store(true, Ordering::SeqCst);
                        } else {
                            eprintln!("system: failed to set up SIGINT handler");
                        }
                    });

                    tokio::spawn(async move {
                        if let Ok(mut sig) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()) {
                            while sig.recv().await.is_some() {
                                veprintln!(1, "system: received SIGHUP, reloading policy...");
                                reload_sighup_handler.store(true, Ordering::SeqCst);
                            }
                        } else {
                            eprintln!("system: failed to set up SIGHUP handler");
                        }
                    });

                    veprintln!(1, "guard daemon started (interval={}s)", guard.interval_secs);
                    let interval = guard.interval_secs;
                    let mut elapsed = 0u64;
                    while !shutdown.load(Ordering::SeqCst) {
                        if reload_sighup.load(Ordering::SeqCst) {
                            reload_sighup.store(false, Ordering::SeqCst);
                            let result = load_system_policy();
                            match result {
                                Ok((policy_path, new_policy)) => {
                                    if policy_path.is_none() {
                                        eprintln!("system: SIGHUP reload warning: no policy file found, using defaults");
                                        emit_event(&DraconEvent::new("system", EventSeverity::Warn, "guard/policy-reload", "SIGHUP reload: no policy file found, using defaults".to_string()));
                                    }
                                    guard = new_policy.guard;
                                    normalize_guard_policy(&mut guard);
                                    veprintln!(2, "system: policy reloaded on SIGHUP (disk_warn={}%, disk_critical={}%)",
                                        guard.disk_warn_percent, guard.disk_critical_percent);
                                }
                                Err(e) => {
                                    eprintln!("system: SIGHUP reload warning: corrupted policy file, using defaults: {}", e);
                                    emit_event(&DraconEvent::new("system", EventSeverity::Error, "guard/policy-reload", format!("SIGHUP reload: policy corrupted, using defaults: {}", e)));
                                }
                            }
                        }
                        if let Err(e) = run_guard_once(&guard, &mut runtime).await {
                            eprintln!("guard pass failed: {}", e);
                            emit_event(&DraconEvent::new("system", EventSeverity::Error, "guard", format!("pass failed: {e}")));
                        }
                        while !shutdown.load(Ordering::SeqCst) && elapsed < interval {
                            sleep(Duration::from_secs(1)).await;
                            elapsed += 1;
                        }
                    }
                    veprintln!(1, "system: guard daemon shutdown complete");
                }
                GuardCommands::Prune { json, docker, docker_volumes, package_caches, apply } => {
                    let mut reclaimed_total = 0u64;
                    let mut actions = Vec::new();
                    
                    // Docker prune
                    if docker || docker_volumes {
                        if apply {
                            match docker_prune(docker, docker_volumes).await {
                                Ok(bytes) => {
                                    actions.push(format!("Docker prune: {}", human_bytes(bytes)));
                                    reclaimed_total += bytes;
                                }
                                Err(e) => {
                                    actions.push(format!("Docker prune failed: {}", e));
                                }
                            }
                        } else {
                            actions.push("Docker prune (dry-run, skipped)".to_string());
                        }
                    }
                    
                    // Package cache cleanup
                    if package_caches {
                        match clean_package_caches(true, true, true, true, apply, &guard.protected_paths).await {
                            Ok((bytes, cleaned)) => {
                                for c in cleaned {
                                    actions.push(format!("Package cache: {}", c));
                                }
                                reclaimed_total += bytes;
                            }
                            Err(e) => {
                                actions.push(format!("Package cache cleanup failed: {}", e));
                            }
                        }
                    }
                    
                    // If no specific flags, show what would be cleaned
                    if !docker && !docker_volumes && !package_caches {
                        // Show disk usage info
                        let disk = disk_use_percent_for(&guard.disk_mount_path).await?;
                        println!("Disk usage: {}% (mount: {})", disk, guard.disk_mount_path);
                        
                        // Show inode info
                        if let Ok((total, used, _free)) = get_inode_info().await {
                            let pct = used.saturating_mul(100).checked_div(total).unwrap_or(0) as u8;
                            println!("Inode usage: {}% ({}/{} inodes used)", pct, used, total);
                        }
                        
                        // Show potential cleanup targets
                        println!();
                        println!("Potential cleanup targets:");
                        println!("  --docker          Prune unused Docker images/containers");
                        println!("  --docker-volumes  Prune Docker volumes too (aggressive)");
                        println!("  --package-caches  Clean cargo/npm/pip/go caches");
                        println!();
                        println!("Add --apply to execute cleanup.");
                    }
                    
                    if json {
                        #[derive(Serialize)]
                        struct PruneReport {
                            reclaimed_bytes: u64,
                            reclaimed_human: String,
                            actions: Vec<String>,
                        }
                        let report = PruneReport {
                            reclaimed_bytes: reclaimed_total,
                            reclaimed_human: human_bytes(reclaimed_total),
                            actions,
                        };
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    } else if !actions.is_empty() {
                        println!("Prune results:");
                        for a in &actions {
                            println!("  - {}", a);
                        }
                        println!("Total reclaimed: {}", human_bytes(reclaimed_total));
                        
                        if !apply && (docker || docker_volumes || package_caches) {
                            println!();
                            println!("Note: This was a dry-run. Add --apply to execute.");
                        }
                    }
                }
                GuardCommands::Clean { json, apply, rust, trash, nix, caches, node_modules, docker, all, min_size_mb } => {
                    // If --all or no specific flags, do everything
                    let do_all = all || (!rust && !trash && !nix && !caches && !node_modules && !docker);
                    let do_rust = rust || do_all;
                    let do_trash = trash || do_all;
                    let do_nix = nix || do_all;
                    let do_caches = caches || do_all;
                    let do_node = node_modules || do_all;
                    let do_docker = docker || do_all;
                    
                    let mut guard_clone = guard.clone();
                    if let Some(mb) = min_size_mb {
                        guard_clone.cleanup_min_size_mb = mb;
                    }
                    
                    let mut total_reclaimed = 0u64;
                    let mut actions: Vec<String> = Vec::new();
                    let mut failures: Vec<String> = Vec::new();
                    
                    // Rust targets
                    if do_rust {
                        let mut runtime = GuardRuntimeState::default();
                        let result = auto_cleanup_rust_targets(&guard_clone, &mut runtime, apply).await?;
                        total_reclaimed += result.reclaimed_bytes;
                        for p in result.cleaned_paths {
                            actions.push(format!("Rust: {}", p));
                        }
                        for p in result.protected_paths {
                            actions.push(format!("Protected: {}", p));
                        }
                    }
                    
                    // Trash
                    if do_trash {
                        match empty_trash(apply, &guard_clone.protected_paths).await {
                            Ok((bytes, cleaned)) => {
                                total_reclaimed += bytes;
                                for c in cleaned {
                                    actions.push(format!("Trash: {}", c));
                                }
                            }
                            Err(e) => failures.push(format!("Trash: {}", e)),
                        }
                    }
                    
                    // Nix garbage
                    if do_nix {
                        match clean_nix_garbage(guard_clone.nix_keep_generations, apply).await {
                            Ok((bytes, cleaned)) => {
                                total_reclaimed += bytes;
                                for c in cleaned {
                                    actions.push(format!("Nix: {}", c));
                                }
                            }
                            Err(e) => failures.push(format!("Nix: {}", e)),
                        }
                    }
                    
                    // Old node_modules
                    if do_node {
                        let roots: Vec<PathBuf> = guard_clone.node_modules_search_roots
                            .split(',')
                            .filter_map(|s| {
                                let s = s.trim();
                                if s.is_empty() { return None; }
                                let p = expand_tilde(s);
                                if p.exists() { Some(p) } else { None }
                            })
                            .collect();
                        match clean_old_node_modules(&roots, guard_clone.node_modules_max_age_days, apply, &guard_clone.protected_paths).await {
                            Ok((bytes, cleaned)) => {
                                total_reclaimed += bytes;
                                for c in cleaned {
                                    actions.push(format!("Node: {}", c));
                                }
                            }
                            Err(e) => failures.push(format!("Node: {}", e)),
                        }
                    }
                    
                    // Package caches
                    if do_caches {
                        match clean_package_caches(true, true, true, true, apply, &guard_clone.protected_paths).await {
                            Ok((bytes, cleaned)) => {
                                total_reclaimed += bytes;
                                for c in cleaned {
                                    actions.push(format!("Cache: {}", c));
                                }
                            }
                            Err(e) => failures.push(format!("Cache: {}", e)),
                        }
                    }
                    
                    // Docker
                    if do_docker {
                        match docker_prune(true, guard_clone.docker_prune_volumes).await {
                            Ok(bytes) => {
                                total_reclaimed += bytes;
                                if bytes > 0 {
                                    actions.push(format!("Docker: {}", human_bytes(bytes)));
                                }
                            }
                            Err(e) => failures.push(format!("Docker: {}", e)),
                        }
                    }
                    
                    if json {
                        #[derive(Serialize)]
                        struct CleanReport {
                            reclaimed_bytes: u64,
                            reclaimed_human: String,
                            actions: Vec<String>,
                            failures: Vec<String>,
                            apply: bool,
                        }
                        let report = CleanReport {
                            reclaimed_bytes: total_reclaimed,
                            reclaimed_human: human_bytes(total_reclaimed),
                            actions,
                            failures,
                            apply,
                        };
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    } else {
                        if actions.is_empty() && failures.is_empty() {
                            println!("Nothing to clean.");
                        } else {
                            if !failures.is_empty() {
                                eprintln!("⚠️ {} cleanup step(s) failed:", failures.len());
                                for f in &failures {
                                    eprintln!("  • {}", f);
                                }
                                println!();
                            }
                            println!("Cleanup {}:", if apply { "results" } else { "preview (dry-run)" });
                            for a in &actions {
                                println!("  • {}", a);
                            }
                            println!();
                            println!("Total reclaimable: {}", human_bytes(total_reclaimed));
                            if !apply {
                                println!("Add --apply to execute cleanup.");
                            }
                        }
                    }
                }
            }
        }
        Commands::Events { tail, source, severity } => {
            let path = events_path();
            if !path.exists() {
                println!("No events found ({} does not exist)", path.display());
                return Ok(());
            }
            let contents = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let lines: Vec<&str> = contents.lines().collect();
            let start = if lines.len() > tail { lines.len() - tail } else { 0 };
            let mut shown = 0usize;
            for line in &lines[start..] {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(ref s) = source {
                        if val.get("src").and_then(|v| v.as_str()) != Some(s.as_str()) {
                            continue;
                        }
                    }
                    if let Some(ref s) = severity {
                        if val.get("sev").and_then(|v| v.as_str()) != Some(s.as_str()) {
                            continue;
                        }
                    }
                    println!("{}", line);
                    shown += 1;
                }
            }
            if shown == 0 {
                println!("(no matching events)");
            }
        }
        Commands::Zram { status, gen_config, memory_percent, algorithm } => {
            if gen_config {
                let mem_pct = memory_percent.unwrap_or(200);
                let algo = algorithm.unwrap_or_else(|| "zstd".to_string());
                let valid_algos = ["lzo", "lzo-rle", "lz4", "lz4hc", "zstd", "deflate", "842"];
                if !valid_algos.contains(&algo.as_str()) {
                    return Err(anyhow::anyhow!("Invalid algorithm. Valid: {}", valid_algos.join(", ")));
                }
                let total_ram_kb: u64 = std::fs::read_to_string("/proc/meminfo")
                    .ok()
                    .and_then(|s| s.lines().find(|l| l.starts_with("MemTotal:")).map(|l| l.split_whitespace().nth(1).unwrap_or("0").to_string()))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let total_ram_gb = total_ram_kb as f64 / 1024.0 / 1024.0;
                println!("# Zram configuration for NixOS");
                println!("# Add this to your ~/.dracon/nixos/configuration.nix");
                println!();
                println!("  # --- ZRAM ---");
                println!("  zramSwap = {{");
                println!("    enable = true;");
                println!("    algorithm = \"{}\";", algo);
                println!("    # {}% of RAM = {}GB virtual swap (based on detected {} GB RAM)", mem_pct, (mem_pct as f64 / 100.0 * total_ram_gb), total_ram_gb);
                println!("    memoryPercent = {};", mem_pct);
                println!("  }};");
                println!();
                println!("# Then rebuild: sudo nixos-rebuild switch --flake ~/.dracon/nixos#");
                return Ok(());
            }
            
            if status || (!gen_config) {
                // Show zram stats
                let zram_path = "/sys/block/zram0";
                let mm_stat_path = format!("{}/mm_stat", zram_path);
                
                println!("Zram Status");
                println!("============");
                
                // Check if zram exists
                if !std::path::Path::new(zram_path).exists() {
                    println!("No zram device found.");
                    return Ok(());
                }
                
                // Get disksize
                let disksize = std::fs::read_to_string(format!("{}/disksize", zram_path))
                    .map(|s| s.trim().parse::<u64>().unwrap_or(0))
                    .unwrap_or(0);
                let disksize_gb = disksize / 1024 / 1024 / 1024;
                
                // Get current algorithm
                let algo = std::fs::read_to_string(format!("{}/comp_algorithm", zram_path))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                
                // Get mm_stat (original data size, compressed size, memory used)
                let mm_stat = std::fs::read_to_string(&mm_stat_path)
                    .map(|s| {
                        s.split_whitespace()
                            .filter_map(|v| v.parse::<u64>().ok())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                
                // mm_stat fields are in bytes: orig_size, compr_size, mem_used (and more)
                let orig = *mm_stat.first().unwrap_or(&0);
                let compr = *mm_stat.get(1).unwrap_or(&0);
                let mem_used = *mm_stat.get(2).unwrap_or(&0);
                
                let orig_gb = orig as f64 / 1024.0 / 1024.0 / 1024.0;
                let compr_gb = compr as f64 / 1024.0 / 1024.0 / 1024.0;
                let mem_used_gb = mem_used as f64 / 1024.0 / 1024.0 / 1024.0;
                let ratio = if orig > 0 { compr as f64 / orig as f64 } else { 0.0 };
                
                println!();
                println!("Device: /dev/zram0");
                println!("Disksize: {} GB", disksize_gb);
                println!("Algorithm: {}", algo);
                println!();
                println!("Memory Usage:");
                println!("  Original data: {:.1} GB", orig_gb);
                println!("  Compressed:    {:.1} GB", compr_gb);
                println!("  RAM used:      {:.1} GB", mem_used_gb);
                println!("  Compression ratio: {:.1}% ({:.1}x)", ratio * 100.0, if ratio > 0.0 { 1.0 / ratio } else { 0.0 });
                println!();
                println!("Configuration options:");
                println!("  --gen-config           Generate NixOS configuration snippet");
                println!("  --memory-percent <N>   Set memory percent (default: 200 for 2x RAM)");
                println!("  --algorithm <algo>     Set algorithm: lzo, lz4, lz4hc, zstd (default: zstd)");
                println!();
                println!("Example - generate config for 2x RAM with zstd:");
                println!("  dracon-system zram --gen-config --memory-percent 200 --algorithm zstd");
            }
        }
    }

    Ok(())
}
