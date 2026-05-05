use std::path::{Path, PathBuf};

/// Load a secret value from an environment variable or `.env` files.
///
/// Strategy:
/// 1. Check the env var `env_name` directly — if set and non-empty, return it.
/// 2. Scan all `*.env` files in the given `secrets_dir`, parse `KEY=VALUE` lines,
///    and return the matching value.
///
/// The two different secrets directories used across the codebase:
/// - `~/.dracon/utilities/sync/secrets` — general sync secrets (git.rs)
/// - `~/.dracon/utilities/sync/ai/secrets` — AI provider keys (simple_ai.rs)
pub(crate) fn load_secret(env_name: &str, secrets_dir: &Path) -> Option<String> {
    // 1. Check env var directly
    if let Ok(val) = std::env::var(env_name) {
        if !val.is_empty() {
            return Some(val);
        }
    }

    // 2. Scan .env files
    if let Ok(entries) = std::fs::read_dir(secrets_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "env") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }
                        if let Some((key, value)) = line.split_once('=') {
                            if key.trim() == env_name {
                                let value = value.trim();
                                if !value.is_empty() {
                                    return Some(value.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Returns the default sync secrets directory: `~/.dracon/utilities/sync/secrets`.
pub(crate) fn sync_secrets_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dracon/utilities/sync/secrets")
}

/// Returns the AI secrets directory: `~/.dracon/utilities/sync/ai/secrets`.
pub(crate) fn ai_secrets_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dracon/utilities/sync/ai/secrets")
}
