//! Miscellaneous git utilities — secret loading, orphan origin detection, path locking.

use std::path::Path;
use anyhow::{Context, Result};
#[cfg(test)]
use std::sync::OnceLock;

/// Load a secret by environment variable name using the sync secrets directory.
pub(crate) fn load_secret(env_name: &str) -> Option<String> {
    crate::secrets::load_secret(env_name, &crate::secrets::sync_secrets_dir())
}

/// Detect if origin URL points to an orphan -N suffixed repo.
/// Returns Some((current_url, canonical_url)) if orphan detected, None otherwise.
pub(crate) fn detect_orphan_origin(repo: &Path) -> Option<(String, String)> {
    let current = crate::git::multi_remote::get_remote_url(repo, "origin")?;
    let path_part = current.rsplit('/').next()?;
    let (repo_part, suffix) = if let Some(dot) = path_part.rfind('.') {
        (&path_part[..dot], &path_part[dot..])
    } else {
        (path_part, "")
    };
    if let Some(dash) = repo_part.rfind('-') {
        let suffix_num = &repo_part[dash + 1..];
        if suffix_num.len() == 1
            && suffix_num.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
        {
            let prefix = &current[..current.len() - path_part.len()];
            let canonical_repo = &repo_part[..dash];
            let canonical = format!("{}{}{}", prefix, canonical_repo, suffix);
            return Some((current, canonical));
        }
    }
    None
}

/// Fix an orphan origin URL by updating the remote and tracking.
pub(crate) fn fix_orphan_origin(repo: &Path, canonical_url: &str) -> Result<()> {
    crate::policy::std_git_command()
        .args(["remote", "set-url", "origin", canonical_url])
        .current_dir(repo)
        .status()
        .with_context(|| format!("failed to set origin URL in {}", repo.display()))?;
    Ok(())
}

/// Acquire a test path lock for serializing PATH-modifying tests.
#[cfg(test)]
pub(crate) fn acquire_path_lock() -> parking_lot::MutexGuard<'static, ()> {
    static PATH_LOCK: OnceLock<parking_lot::Mutex<()>> = OnceLock::new();
    let lock = PATH_LOCK.get_or_init(|| parking_lot::Mutex::new(()));
    loop {
        if let Some(guard) = lock.try_lock() { return guard; }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
