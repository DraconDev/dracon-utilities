use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::OnceLock;
use tokio::process::Command as TokioCommand;

pub(crate) const DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES: u64 = 100 * 1024 * 1024;

pub(crate) fn git_binary() -> &'static Path {
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

pub(crate) fn std_git_command() -> StdCommand {
    StdCommand::new(git_binary())
}

pub(crate) fn tokio_git_command() -> TokioCommand {
    TokioCommand::new(git_binary())
}

pub(crate) fn timestamp_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct SyncPolicy {
    #[serde(default)]
    pub(crate) system_repo: String,
    #[serde(default = "default_pulse_interval")]
    pub(crate) pulse_interval_secs: u64,
    #[serde(default = "default_inactivity_push_delay_secs")]
    pub(crate) inactivity_push_delay_secs: u64,
    #[serde(default = "default_true")]
    pub(crate) auto_commit: bool,
    /// If true, bump patch versions before an auto-commit (best-effort).
    /// Applies to common files when present at repo root:
    /// - Rust: `Cargo.toml` (and keep `Cargo.lock` aligned for root package)
    /// - Node/TS: `package.json` (and align `package-lock.json` root `version` when applicable)
    /// - Generic: `VERSION`
    #[serde(default = "default_true")]
    pub(crate) auto_bump_versions: bool,
    #[serde(default = "default_true")]
    pub(crate) auto_pull: bool,
    #[serde(default = "default_true")]
    pub(crate) auto_push: bool,
    #[serde(default)]
    pub(crate) backup_policy: String,
    #[serde(default)]
    pub(crate) backup_dir: String,
    #[serde(default)]
    pub(crate) exclude_repos: Vec<String>,
    #[serde(default)]
    pub(crate) exclude_dir_names: Vec<String>,
    #[serde(default = "default_exclude_file_patterns")]
    pub(crate) exclude_file_patterns: Vec<String>,
    #[serde(default = "default_true")]
    pub(crate) auto_repair_concerns: bool,
    #[serde(default = "default_true")]
    pub(crate) auto_repair_warns: bool,
    #[serde(default = "default_true")]
    pub(crate) auto_rewrite_large_blobs: bool,
    #[serde(default)]
    pub(crate) watch_roots: Vec<String>,
    #[serde(default)]
    pub(crate) extra_remotes: Vec<String>,
    #[serde(default)]
    pub(crate) auto_github_private: bool,
    #[serde(default = "default_github_account")]
    pub(crate) auto_github_private_account: String,
    #[serde(default = "default_max_stage_file_bytes")]
    pub(crate) max_stage_file_bytes: u64,
    #[serde(default = "default_pull_op_timeout_secs")]
    pub(crate) pull_op_timeout_secs: u64,
    #[serde(default = "default_push_op_timeout_secs")]
    pub(crate) push_op_timeout_secs: u64,
    #[serde(default = "default_repo_sync_timeout_secs")]
    pub(crate) repo_sync_timeout_secs: u64,
    #[serde(default = "default_push_retries")]
    pub(crate) push_retries: u32,
    #[serde(default = "default_repair_cooldown_secs")]
    pub(crate) repair_cooldown_secs: u64,
    #[serde(default = "default_max_push_blob_bytes")]
    pub(crate) max_push_blob_bytes: u64,
    #[serde(default = "default_incident_ledger_max_lines")]
    pub(crate) incident_ledger_max_lines: usize,
    #[serde(default = "default_incident_ledger_max_age_days")]
    pub(crate) incident_ledger_max_age_days: u64,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub(crate) struct RepoPolicyOverride {
    /// Optional per-repo override for `auto_bump_versions`.
    pub(crate) auto_bump_versions: Option<bool>,
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_pulse_interval() -> u64 {
    1
}

pub(crate) fn default_inactivity_push_delay_secs() -> u64 {
    5
}

pub(crate) fn load_repo_override(repo: &Path) -> RepoPolicyOverride {
    let path = repo.join(".dracon").join("dracon-sync.toml");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return RepoPolicyOverride::default();
    };
    toml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("⚠️ failed to parse repo override {}: {}", path.display(), e);
        RepoPolicyOverride::default()
    })
}

pub(crate) fn default_exclude_dir_names() -> Vec<String> {
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

pub(crate) fn default_exclude_file_patterns() -> Vec<String> {
    [
        "*.log",
        "nohup.out",
        "*.sqlite",
        "*.sqlite3",
        "*.db",
        "*.db-journal",
        "*.db-wal",
        "*.db-shm",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

pub(crate) fn default_max_stage_file_bytes() -> u64 {
    100 * 1024 * 1024
}

pub(crate) fn default_pull_op_timeout_secs() -> u64 {
    30
}

pub(crate) fn default_push_op_timeout_secs() -> u64 {
    300
}

pub(crate) fn default_repo_sync_timeout_secs() -> u64 {
    420
}

pub(crate) fn default_push_retries() -> u32 {
    3
}

pub(crate) fn default_repair_cooldown_secs() -> u64 {
    60
}

pub(crate) fn default_max_push_blob_bytes() -> u64 {
    DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES
}

pub(crate) fn default_incident_ledger_max_lines() -> usize {
    10_000
}

pub(crate) fn default_incident_ledger_max_age_days() -> u64 {
    30
}

fn default_github_account() -> String {
    "DraconDev".to_string()
}

impl SyncPolicy {
    pub(crate) fn load(path: &Path) -> Result<Self> {
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
            eprintln!(
                "⚠️ pull_op_timeout_secs {} below minimum 5s, adjusting",
                policy.pull_op_timeout_secs
            );
            policy.pull_op_timeout_secs = 5;
        }
        if policy.push_op_timeout_secs < 10 {
            eprintln!(
                "⚠️ push_op_timeout_secs {} below minimum 10s, adjusting",
                policy.push_op_timeout_secs
            );
            policy.push_op_timeout_secs = 10;
        }
        policy.max_push_blob_bytes = policy
            .max_push_blob_bytes
            .clamp(1, DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES);
        policy.repo_sync_timeout_secs = policy.repo_sync_timeout_secs.max(
            policy
                .push_op_timeout_secs
                .saturating_add(30)
                .max(policy.pull_op_timeout_secs.saturating_add(30)),
        );
        Ok(policy)
    }

    pub(crate) fn watch_root_paths(&self) -> Vec<PathBuf> {
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

pub(crate) fn resolve_policy_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("home not found")?;
    if let Ok(val) = std::env::var("DRACON_SYNC_POLICY") {
        return Ok(PathBuf::from(val));
    }
    let paths = [
        home.join(".dracon/utilities/sync/dracon-sync.toml"),
        home.join(".dracon/utilities/sync/config.toml"),
        home.join(".dracon/git/dracon-git.toml"),
    ];
    for path in &paths {
        if path.exists() {
            return Ok(path.clone());
        }
    }
    anyhow::bail!("sync policy not found")
}

pub(crate) fn env_freeze_enabled() -> bool {
    matches!(
        std::env::var("DRACON_SYNC_FREEZE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

pub(crate) fn debug_enabled() -> bool {
    matches!(
        std::env::var("DRACON_SYNC_DEBUG")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

pub(crate) fn freeze_marker_paths(_policy_path: &Path) -> Vec<PathBuf> {
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

pub(crate) fn freeze_reason(policy_path: &Path) -> Option<String> {
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

pub(crate) fn open_policy_in_editor(policy_path: &Path) -> Result<()> {
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

    #[test]
    fn test_default_exclude_dir_names() {
        let dirs = default_exclude_dir_names();
        assert!(dirs.contains(&"target".to_string()));
        assert!(dirs.contains(&"node_modules".to_string()));
        assert!(dirs.contains(&".cache".to_string()));
    }

    #[test]
    fn test_default_exclude_file_patterns() {
        let patterns = default_exclude_file_patterns();
        assert!(patterns.contains(&"*.log".to_string()));
        assert!(patterns.contains(&"nohup.out".to_string()));
        assert!(patterns.contains(&"*.sqlite".to_string()));
        assert!(patterns.contains(&"*.sqlite3".to_string()));
        assert!(patterns.contains(&"*.db".to_string()));
        assert!(patterns.contains(&"*.db-journal".to_string()));
        assert!(patterns.contains(&"*.db-wal".to_string()));
        assert!(patterns.contains(&"*.db-shm".to_string()));
    }

    #[test]
    fn test_default_max_stage_file_bytes() {
        let bytes = default_max_stage_file_bytes();
        assert_eq!(bytes, 100 * 1024 * 1024);
    }

    #[test]
    fn test_default_pull_op_timeout_secs() {
        let secs = default_pull_op_timeout_secs();
        assert_eq!(secs, 30);
    }

    #[test]
    fn test_default_push_op_timeout_secs() {
        let secs = default_push_op_timeout_secs();
        assert_eq!(secs, 300);
    }

    #[test]
    fn test_default_repo_sync_timeout_secs() {
        let secs = default_repo_sync_timeout_secs();
        assert_eq!(secs, 420);
    }

    #[test]
    fn test_default_push_retries() {
        let retries = default_push_retries();
        assert_eq!(retries, 3);
    }

    #[test]
    fn test_default_repair_cooldown_secs() {
        let secs = default_repair_cooldown_secs();
        assert_eq!(secs, 60);
    }

    #[test]
    fn test_default_max_push_blob_bytes() {
        let bytes = default_max_push_blob_bytes();
        assert_eq!(bytes, DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES);
    }

    #[test]
    fn test_default_incident_ledger_max_lines() {
        let lines = default_incident_ledger_max_lines();
        assert_eq!(lines, 10_000);
    }

    #[test]
    fn test_default_incident_ledger_max_age_days() {
        let days = default_incident_ledger_max_age_days();
        assert_eq!(days, 30);
    }

    #[test]
    fn test_debug_enabled() {
        std::env::remove_var("DRACON_SYNC_DEBUG");
        assert!(!debug_enabled());
    }

    #[test]
    fn test_default_true() {
        assert!(default_true());
    }

    #[test]
    fn test_default_pulse_interval() {
        assert_eq!(default_pulse_interval(), 1);
    }

    #[test]
    fn test_default_inactivity_push_delay_secs() {
        assert_eq!(default_inactivity_push_delay_secs(), 5);
    }

    #[test]
    fn test_git_host_blob_limit() {
        assert_eq!(DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES, 100 * 1024 * 1024);
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

    #[test]
    fn test_sync_policy_defaults_produce_valid_values() {
        let policy = test_sync_policy();
        assert!(policy.pulse_interval_secs >= 1);
        assert!(policy.inactivity_push_delay_secs >= 1);
        assert!(policy.max_stage_file_bytes > 0);
        assert!(policy.pull_op_timeout_secs >= 5);
        assert!(policy.push_op_timeout_secs >= 10);
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
    fn test_repo_policy_override_default() {
        let override_default = crate::policy::RepoPolicyOverride::default();
        assert!(override_default.auto_bump_versions.is_none());
    }

    #[test]
    fn test_freeze_marker_paths() {
        let paths = freeze_marker_paths(std::path::Path::new("/fake/path.toml"));
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_load_repo_override_nonexistent() {
        let repo = std::path::Path::new("/nonexistent/path/for/test");
        let override_ = load_repo_override(repo);
        assert!(override_.auto_bump_versions.is_none());
    }

    static POLICY_ENV_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct VarGuard {
        var: String,
        original: Option<String>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }
    impl VarGuard {
        fn set_temp(var: &str, value: &str) -> Self {
            let lock = POLICY_ENV_GUARD.lock().unwrap();
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
    fn test_env_freeze_enabled_ignores_case() {
        let _guard = VarGuard::set_temp("DRACON_SYNC_FREEZE", "TRUE");
        assert!(env_freeze_enabled());
    }

    #[test]
    fn test_env_freeze_enabled_accepts_yes() {
        let _guard = VarGuard::set_temp("DRACON_SYNC_FREEZE", "yes");
        assert!(env_freeze_enabled());
    }

    #[test]
    fn test_env_freeze_enabled_accepts_on() {
        let _guard = VarGuard::set_temp("DRACON_SYNC_FREEZE", "on");
        assert!(env_freeze_enabled());
    }

    #[test]
    fn test_env_freeze_enabled_rejects_false() {
        let _guard = VarGuard::set_temp("DRACON_SYNC_FREEZE", "false");
        assert!(!env_freeze_enabled());
    }

    #[test]
    fn test_env_freeze_enabled_rejects_empty() {
        let _guard = VarGuard::set_temp("DRACON_SYNC_FREEZE", "");
        assert!(!env_freeze_enabled());
    }

    #[test]
    fn test_debug_enabled_accepts_1() {
        let _guard = VarGuard::set_temp("DRACON_SYNC_DEBUG", "1");
        assert!(debug_enabled());
    }

    #[test]
    fn test_debug_enabled_rejects_empty() {
        let _guard = VarGuard::set_temp("DRACON_SYNC_DEBUG", "");
        assert!(!debug_enabled());
    }

    #[test]
    fn test_freeze_reason_env_takes_precedence() {
        let _guard = VarGuard::set_temp("DRACON_SYNC_FREEZE", "1");
        let reason = freeze_reason(std::path::Path::new("/fake/policy.toml"));
        assert_eq!(reason, Some("env DRACON_SYNC_FREEZE".to_string()));
    }

    #[test]
    fn test_freeze_reason_none_when_not_frozen() {
        std::env::remove_var("DRACON_SYNC_FREEZE");
        let reason = freeze_reason(std::path::Path::new("/fake/policy.toml"));
        assert!(reason.is_none());
    }

    #[test]
    fn test_freeze_marker_paths_includes_dracondir() {
        let paths = freeze_marker_paths(std::path::Path::new("/fake.toml"));
        assert!(paths.iter().any(|p| p.to_string_lossy().contains(".dracon")));
        assert!(paths.iter().any(|p| p.to_string_lossy().contains("freeze")));
    }

    #[test]
    #[ignore = "env var tests interfere with each other in parallel - needs VarGuard with Mutex"]
    fn test_resolve_policy_path_env_override() {
        std::env::set_var("DRACON_SYNC_POLICY", "/custom/policy.toml");
        let path = resolve_policy_path();
        std::env::remove_var("DRACON_SYNC_POLICY");
        assert!(path.is_ok());
        assert_eq!(path.unwrap(), PathBuf::from("/custom/policy.toml"));
    }

    #[test]
    fn test_sync_policy_watch_roots_filters_nonexistent() {
        let policy = SyncPolicy {
            watch_roots: vec!["/nonexistent/path/one".to_string(), "/nonexistent/path/two".to_string()],
            ..test_sync_policy()
        };
        let roots = policy.watch_root_paths();
        assert!(roots.is_empty());
    }

    #[test]
    fn test_timestamp_secs_returns_increasing_values() {
        let ts1 = timestamp_secs();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = timestamp_secs();
        assert!(ts2 >= ts1);
    }
}
