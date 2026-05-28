use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Load a secret value from an environment variable or `.env` files.
///
/// Strategy:
/// 1. Check the env var `env_name` directly — if set and non-empty, return it.
/// 2. Scan all `*.env` files in the given `secrets_dir`, parse `KEY=VALUE` lines,
///    and return the matching value.
///
/// Security: if the secrets directory is world-writable, secrets are refused
/// to prevent malicious injection by other users.
///
/// The secrets directory:
/// - `~/.dracon/utilities/sync/secrets` — general sync secrets (git.rs)
pub(crate) fn load_secret(env_name: &str, secrets_dir: &Path) -> Option<String> {
    // 1. Check env var directly
    if let Ok(val) = std::env::var(env_name) {
        if !val.is_empty() {
            return Some(val);
        }
    }

    // 2. Permission check on secrets directory
    if let Err(e) = check_secrets_dir_permissions(secrets_dir) {
        eprintln!(
            "⚠️ secrets directory permission check failed for {}: {}",
            secrets_dir.display(),
            e
        );
        return None;
    }

    // 3. Scan .env files
    if let Ok(entries) = std::fs::read_dir(secrets_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "env") {
                #[cfg(unix)]
                warn_if_world_readable(&path);
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

/// Verify that the secrets directory is not world-writable.
/// A world-writable secrets directory allows any user to inject malicious
/// credential files, which could lead to credential theft or repo hijacking.
#[cfg(unix)]
fn check_secrets_dir_permissions(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        // Directory doesn't exist yet — not a security issue
        return Ok(());
    }
    let metadata = std::fs::metadata(dir).map_err(|e| format!("cannot read metadata: {}", e))?;
    let mode = metadata.permissions().mode();
    if mode & 0o002 != 0 {
        return Err(format!(
            "directory is world-writable (mode {:o}). Refusing to load secrets.",
            mode & 0o7777
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_secrets_dir_permissions(_dir: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn warn_if_world_readable(path: &Path) {
    if let Ok(metadata) = std::fs::metadata(path) {
        let mode = metadata.permissions().mode();
        if mode & 0o044 != 0 {
            eprintln!(
                "⚠️ secret file {} is world-readable (mode {:o}). Consider chmod 600.",
                path.display(),
                mode & 0o7777
            );
        }
    }
}

/// Returns the default sync secrets directory: `~/.dracon/utilities/sync/secrets`.
pub(crate) fn sync_secrets_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dracon/utilities/sync/secrets")
}
