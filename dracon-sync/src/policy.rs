use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tokio::process::Command as TokioCommand;

pub(crate) const DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub(crate) struct RemoteConfig {
    pub(crate) name: String,
    pub(crate) push_url: String,
    #[serde(default)]
    pub(crate) auto_create: bool,
    #[serde(default)]
    pub(crate) auto_create_account: String,
    #[serde(default = "default_auth_type")]
    pub(crate) auth_type: AuthType,
    #[serde(default = "default_priority")]
    pub(crate) priority: u32,
    #[serde(default)]
    pub(crate) api_endpoint: Option<String>,
    #[serde(default)]
    pub(crate) auto_create_token_var: Option<String>,
    /// Optional per-remote name mapping for repos that need sanitization.
    /// Key = local repo basename, Value = remote project name.
    /// Example: { ".dracon" = "dracon-home" } maps .dracon → dracon-home on this remote.
    #[serde(default)]
    pub(crate) repo_name_map: std::collections::HashMap<String, String>,
    /// If true, when a push to this remote fails with non-fast-forward, the daemon
    /// will diagnose divergence. If the remote is purely behind (0 commits ahead
    /// of local), it force-pushes with --force-with-lease. If the remote has
    /// commits local lacks (divergent), the repo is marked CONCERN instead.
    #[serde(default)]
    pub(crate) force_push_when_behind: bool,
}

#[allow(dead_code)]
impl RemoteConfig {
    pub(crate) fn resolve_push_url(&self, repo_name: &str) -> String {
        let resolved_name = self.repo_name_map.get(repo_name).map(|s| s.as_str()).unwrap_or(repo_name);
        let url = self.push_url.replace("{repo}", resolved_name);
        url.replace("{account}", &self.auto_create_account)
    }

    pub(crate) fn resolve_repo_name(&self, repo_name: &str) -> String {
        self.repo_name_map.get(repo_name).cloned().unwrap_or_else(|| repo_name.to_string())
    }
}

fn default_auth_type() -> AuthType {
    AuthType::GitHub
}

fn default_priority() -> u32 {
    50
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AuthType {
    #[default]
    GitHub,
    GitLab,
    Codeberg,
    Generic,
}

fn deserialize_remotes_or_extra<'de, D>(deserializer: D) -> Result<Vec<RemoteConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RemotesOrExtra {
        New(Vec<RemoteConfig>),
        Legacy(Vec<String>),
    }

    let raw = RemotesOrExtra::deserialize(deserializer)?;
    match raw {
        RemotesOrExtra::New(configs) => Ok(configs),
        RemotesOrExtra::Legacy(names) => {
            let defaults = vec![
        RemoteConfig {
            name: "github".to_string(),
            push_url: "git@github.com:{account}/{repo}.git".to_string(),
            auto_create: false,
            auto_create_account: "DraconDev".to_string(),
            auth_type: AuthType::GitHub,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
            repo_name_map: Default::default(),
            force_push_when_behind: false,
        },
        RemoteConfig {
            name: "gitlab".to_string(),
            push_url: "git@gitlab.com:{account}/{repo}.git".to_string(),
            auto_create: false,
            auto_create_account: "DraconDev".to_string(),
            auth_type: AuthType::GitLab,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
            repo_name_map: Default::default(),
            force_push_when_behind: false,
        },
        RemoteConfig {
            name: "codeberg".to_string(),
            push_url: "git@codeberg.org:{account}/{repo}.git".to_string(),
            auto_create: false,
            auto_create_account: "dracondev".to_string(),
            auth_type: AuthType::Codeberg,
            priority: 50,
            api_endpoint: Some("https://codeberg.org/api/v1/repos".to_string()),
            auto_create_token_var: None,
            repo_name_map: Default::default(),
            force_push_when_behind: false,
        },
            ];

            let filtered: Vec<RemoteConfig> = defaults
                .into_iter()
                .filter(|d| names.contains(&d.name))
                .map(|mut d| {
                    d.auto_create = true;
                    d
                })
                .collect();
            Ok(filtered)
        }
    }
}

pub(crate) fn git_binary() -> PathBuf {
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
    #[serde(default, deserialize_with = "deserialize_remotes_or_extra")]
    pub(crate) remotes: Vec<RemoteConfig>,
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
    #[serde(default)]
    pub(crate) webhook_url: Option<String>,
    #[serde(default = "default_alert_unpushed_threshold")]
    pub(crate) alert_unpushed_threshold: usize,
    #[serde(default)]
    pub(crate) sync_visibility: bool,
    #[serde(default = "default_sync_visibility_interval_hours")]
    pub(crate) sync_visibility_interval_hours: u64,
    /// When true, sync repo description and topics from GitHub to mirror remotes.
    /// Uses the same interval as visibility sync.
    #[serde(default)]
    pub(crate) sync_metadata: bool,
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

fn default_alert_unpushed_threshold() -> usize {
    10
}

fn default_sync_visibility_interval_hours() -> u64 {
    24
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

#[derive(Debug, Default)]
pub(crate) struct ValidateResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidateResult {
    fn error(&mut self, msg: String) {
        self.errors.push(msg);
    }

    fn warn(&mut self, msg: String) {
        self.warnings.push(msg);
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

pub(crate) fn validate_config(policy_path: &Path) -> ValidateResult {
    let mut result = ValidateResult::default();

    let content = match std::fs::read_to_string(policy_path) {
        Ok(c) => c,
        Err(e) => {
            result.error(format!("cannot read policy {}: {}", policy_path.display(), e));
            return result;
        }
    };

    let policy: SyncPolicy = match toml::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            result.error(format!("TOML parse error: {}", e));
            return result;
        }
    };

    for root in &policy.watch_roots {
        let path = Path::new(root);
        if !path.exists() {
            result.error(format!("watch root does not exist: {}", root));
        } else if !path.is_dir() {
            result.error(format!("watch root is not a directory: {}", root));
        }
    }

    if policy.watch_roots.is_empty() {
        result.error("no watch_roots defined (no directories will be synced)".to_string());
    }

    for (idx, remote) in policy.remotes.iter().enumerate() {
        if remote.push_url.is_empty() {
            result.error(format!("remote[{}] '{}': push_url is empty", idx, remote.name));
        }

        if remote.auto_create {
            if remote.auto_create_account.is_empty() {
                result.error(format!(
                    "remote[{}] '{}': auto_create=true but auto_create_account is empty",
                    idx, remote.name
                ));
            }

            if let Some(token_var) = &remote.auto_create_token_var {
                if token_var.is_empty() {
                    result.error(format!(
                        "remote[{}] '{}': auto_create_token_var is set but empty",
                        idx, remote.name
                    ));
                } else if std::env::var(token_var).is_err() {
                    let secrets_dir = crate::secrets::sync_secrets_dir();
                    let secrets_path = secrets_dir.join(format!("{}.env", token_var.to_lowercase()));
                    if !secrets_path.exists() {
                        result.warn(format!(
                            "remote[{}] '{}': auto_create_token_var '{}' not in env and no secret file at {}",
                            idx, remote.name, token_var, secrets_path.display()
                        ));
                    }
                }
            }

            if remote.auth_type == crate::policy::AuthType::Codeberg {
                if let Some(api_endpoint) = &remote.api_endpoint {
                    if api_endpoint.is_empty() {
                        result.error(format!(
                            "remote[{}] '{}': auth_type=codeberg but api_endpoint is empty",
                            idx, remote.name
                        ));
                    } else if !api_endpoint.starts_with("http://") && !api_endpoint.starts_with("https://") {
                        result.error(format!(
                            "remote[{}] '{}': api_endpoint '{}' is not a valid URL",
                            idx, remote.name, api_endpoint
                        ));
                    }
                } else {
                    result.warn(format!(
                        "remote[{}] '{}': auth_type=codeberg but no api_endpoint set (will use default)",
                        idx, remote.name
                    ));
                }
            }
        } else if !remote.push_url.contains("{repo}") && !remote.push_url.contains("{account}") {
            result.warn(format!(
                "remote[{}] '{}': push_url '{}' has no {{repo}} or {{account}} placeholder — repo names will not be substituted",
                idx, remote.name, remote.push_url
            ));
        }

        for (local_name, remote_name) in &remote.repo_name_map {
            if local_name.is_empty() {
                result.error(format!(
                    "remote[{}] '{}': repo_name_map has empty local name (maps to '{}')",
                    idx, remote.name, remote_name
                ));
            }
            if remote_name.is_empty() {
                result.error(format!(
                    "remote[{}] '{}': repo_name_map local '{}' maps to empty remote name",
                    idx, remote.name, local_name
                ));
            }
            if local_name.contains('/') || local_name.contains('\\') {
                result.error(format!(
                    "remote[{}] '{}': repo_name_map local name '{}' is not a valid directory name",
                    idx, remote.name, local_name
                ));
            }
        }
    }

    if policy.remotes.is_empty() {
        result.warn("no remotes defined (push operations will have no destination)".to_string());
    }

    for (idx, pattern) in policy.exclude_dir_names.iter().enumerate() {
        if pattern.is_empty() {
            result.warn(format!("exclude_dir_names[{}] is empty string", idx));
        }
    }

    for (idx, pattern) in policy.exclude_file_patterns.iter().enumerate() {
        if pattern.is_empty() {
            result.warn(format!("exclude_file_patterns[{}] is empty string", idx));
        }
    }

    if policy.auto_github_private {
        if policy.auto_github_private_account.is_empty() {
            result.error("auto_github_private=true but auto_github_private_account is empty".to_string());
        }
    }

    if policy.pulse_interval_secs == 0 {
        result.error("pulse_interval_secs must be > 0".to_string());
    }

    if policy.push_retries == 0 {
        result.error("push_retries must be > 0".to_string());
    }

    if policy.max_stage_file_bytes == 0 {
        result.error("max_stage_file_bytes must be > 0".to_string());
    }

    if let Some(ref url) = policy.webhook_url {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            result.error(format!("webhook_url '{}' is not a valid http/https URL", url));
        }
    }

    result
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
        let _guard = VarGuard::set_temp("DRACON_SYNC_DEBUG", "");
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
        let _guard = VarGuard::set_temp("DRACON_SYNC_FREEZE", "");
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
    fn test_resolve_policy_path_env_override() {
        let _guard = VarGuard::set_temp("DRACON_SYNC_POLICY", "/custom/policy.toml");
        let path = resolve_policy_path();
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

    #[test]
    fn test_resolve_push_url_template_substitution() {
        let config = RemoteConfig {
            name: "github".to_string(),
            push_url: "git@github.com:{account}/{repo}.git".to_string(),
            auto_create: false,
            auto_create_account: "myorg".to_string(),
            auth_type: AuthType::GitHub,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
repo_name_map: Default::default(),
        force_push_when_behind: false,
    };
    assert_eq!(
        config.resolve_push_url("my-repo"),
        "git@github.com:myorg/my-repo.git"
    );
    }

    #[test]
    fn test_resolve_push_url_no_template() {
        let config = RemoteConfig {
            name: "mirror".to_string(),
            push_url: "git@mirror.example.com:fixed/path.git".to_string(),
            auto_create: false,
            auto_create_account: "".to_string(),
            auth_type: AuthType::GitHub,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
repo_name_map: Default::default(),
        force_push_when_behind: false,
    };
    assert_eq!(
        config.resolve_push_url("any-repo"),
        "git@mirror.example.com:fixed/path.git"
    );
    }

    #[test]
    fn test_resolve_push_url_account_only() {
        let config = RemoteConfig {
            name: "gitlab".to_string(),
            push_url: "git@gitlab.com:{account}/".to_string(),
            auto_create: false,
            auto_create_account: "testuser".to_string(),
            auth_type: AuthType::GitLab,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
repo_name_map: Default::default(),
        force_push_when_behind: false,
    };
    assert_eq!(
        config.resolve_push_url("repo"),
        "git@gitlab.com:testuser/"
    );
    }

    #[test]
    fn test_resolve_push_url_with_name_mapping() {
        let mut config = RemoteConfig {
            name: "gitlab".to_string(),
            push_url: "git@gitlab.com:{account}/{repo}.git".to_string(),
            auto_create: false,
            auto_create_account: "myorg".to_string(),
            auth_type: AuthType::GitLab,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
repo_name_map: Default::default(),
        force_push_when_behind: false,
    };
    config.repo_name_map.insert(".dracon".to_string(), "dracon-home".to_string());

    assert_eq!(
        config.resolve_push_url(".dracon"),
        "git@gitlab.com:myorg/dracon-home.git"
    );
        assert_eq!(
            config.resolve_push_url("other-repo"),
            "git@gitlab.com:myorg/other-repo.git"
        );
    }

    #[test]
    fn test_resolve_repo_name_with_mapping() {
        let mut config = RemoteConfig {
            name: "gitlab".to_string(),
            push_url: "git@gitlab.com:{account}/{repo}.git".to_string(),
            auto_create: false,
            auto_create_account: "myorg".to_string(),
            auth_type: AuthType::GitLab,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
    repo_name_map: Default::default(),
        force_push_when_behind: false,
    };
    config.repo_name_map.insert(".dracon".to_string(), "dracon-home".to_string());

    assert_eq!(config.resolve_repo_name(".dracon"), "dracon-home");
        assert_eq!(config.resolve_repo_name("other-repo"), "other-repo");
    }

    #[test]
    fn test_resolve_repo_name_without_mapping() {
        let config = RemoteConfig {
            name: "github".to_string(),
            push_url: "git@github.com:{account}/{repo}.git".to_string(),
            auto_create: false,
            auto_create_account: "myorg".to_string(),
            auth_type: AuthType::GitHub,
            priority: 50,
            api_endpoint: None,
            auto_create_token_var: None,
    repo_name_map: Default::default(),
        force_push_when_behind: false,
    };

        assert_eq!(config.resolve_repo_name(".dracon"), ".dracon");
        assert_eq!(config.resolve_repo_name("my-repo"), "my-repo");
    }

    #[test]
    fn test_validate_config_valid_policy() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content = r#"
auto_github_private = false
auto_commit = true
auto_pull = true
auto_push = true
auto_bump_versions = false
watch_roots = ["/tmp"]
remotes = []
"#;
        std::fs::write(tmp.path().join("policy.toml"), content).unwrap();
        let result = validate_config(tmp.path().join("policy.toml").as_path());
        assert!(result.is_valid(), "valid policy should pass: {:?}", result.errors);
    }

    #[test]
    fn test_validate_config_missing_watch_root() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content = r#"
auto_github_private = false
watch_roots = ["/nonexistent/path/that/does/not/exist"]
remotes = []
"#;
        std::fs::write(tmp.path().join("policy.toml"), content).unwrap();
        let result = validate_config(tmp.path().join("policy.toml").as_path());
        assert!(!result.is_valid(), "missing watch root should fail");
        assert!(result.errors.iter().any(|e| e.contains("does not exist")), "should mention missing path");
    }

    #[test]
    fn test_validate_config_invalid_webhook_url() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content = r#"
auto_github_private = false
watch_roots = ["/tmp"]
remotes = []
webhook_url = "ftp://invalid.example.com/hook"
"#;
        std::fs::write(tmp.path().join("policy.toml"), content).unwrap();
        let result = validate_config(tmp.path().join("policy.toml").as_path());
        assert!(!result.is_valid(), "non-http webhook URL should fail");
        assert!(result.errors.iter().any(|e| e.contains("webhook_url")), "should mention webhook_url");
    }

    #[test]
    fn test_validate_config_empty_remote_push_url() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content = r#"
auto_github_private = false
watch_roots = ["/tmp"]
[[remotes]]
name = "test"
push_url = ""
"#;
        std::fs::write(tmp.path().join("policy.toml"), content).unwrap();
        let result = validate_config(tmp.path().join("policy.toml").as_path());
        assert!(!result.is_valid(), "empty push_url should fail");
    }

    #[test]
    fn test_validate_config_missing_auto_create_account() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content = r#"
auto_github_private = false
watch_roots = ["/tmp"]
[[remotes]]
name = "github"
push_url = "git@github.com:{account}/{repo}.git"
auto_create = true
auto_create_account = ""
"#;
        std::fs::write(tmp.path().join("policy.toml"), content).unwrap();
        let result = validate_config(tmp.path().join("policy.toml").as_path());
        assert!(!result.is_valid(), "auto_create=true with empty account should fail");
    }

    #[test]
    fn test_validate_config_no_watch_roots_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content = r#"
auto_github_private = false
watch_roots = []
remotes = []
"#;
        std::fs::write(tmp.path().join("policy.toml"), content).unwrap();
        let result = validate_config(tmp.path().join("policy.toml").as_path());
        assert!(!result.is_valid(), "no watch_roots should fail");
        assert!(result.errors.iter().any(|e| e.contains("watch_roots")), "should mention watch_roots");
    }

    #[test]
    fn test_validate_config_warns_on_no_remotes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content = r#"
auto_github_private = false
watch_roots = ["/tmp"]
remotes = []
"#;
        std::fs::write(tmp.path().join("policy.toml"), content).unwrap();
        let result = validate_config(tmp.path().join("policy.toml").as_path());
        assert!(result.is_valid(), "no remotes is a warning not error");
        assert!(result.warnings.iter().any(|w| w.contains("no remotes")), "should warn about no remotes");
    }
}
