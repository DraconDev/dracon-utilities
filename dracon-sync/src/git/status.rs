//! Repository status checks — origin, upstream, conflict state, readiness.

use std::path::{Path, PathBuf};

use super::current_branch;

/// RAII guard that acquires `.git/index.lock` using the same protocol git uses.
///
/// Git commands (checkout, add, reset, etc.) hold this lock while modifying
/// the working tree. By acquiring it too, we guarantee mutual exclusion with
/// any in-flight git operation. If the lock is held, we skip; if we hold it,
/// git's checkout waits for us.
///
/// This is the definitive fix for the clone race: during `git clone`, checkout
/// holds index.lock. Our `ensure_standard_files` / `publish_repo_pubkey`
/// write files to the working tree. Without the lock, these appear before
/// checkout completes → "Untracked working tree file would be overwritten by
/// merge." With the lock, either git holds it (we skip) or we hold it
/// (git's checkout waits until we're done).
pub(crate) struct IndexLock {
    path: PathBuf,
    /// True if we successfully created the lock (our responsibility to clean up).
    held: bool,
}

impl IndexLock {
    /// Try to acquire `.git/index.lock` for a repo.
    /// Returns Ok(lock) if acquired, Err if another process holds it.
    /// Uses `O_EXCL` (create_new) for atomic creation — no TOCTOU race.
    pub(crate) fn acquire(repo: &Path) -> Result<Self, String> {
        let path = repo.join(".git").join("index.lock");
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true) // O_EXCL — fails if file exists
            .open(&path)
        {
            Ok(_file) => Ok(Self { path, held: true }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(format!(
                "index.lock held by another git operation, skipping {}",
                repo.display()
            )),
            Err(e) => Err(format!(
                "failed to create index.lock for {}: {}",
                repo.display(),
                e
            )),
        }
    }
}

impl Drop for IndexLock {
    fn drop(&mut self) {
        if self.held {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Check whether an `origin` remote exists via config or git CLI.
pub(crate) fn has_origin_remote(repo: &Path) -> bool {
    let config_path = repo.join(".git").join("config");
    if let Ok(config) = std::fs::read_to_string(&config_path) {
        return config
            .lines()
            .any(|line| line.trim() == "[remote \"origin\"]");
    }
    crate::policy::std_git_command()
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check whether the current branch has a configured upstream.
pub(crate) fn has_tracking_upstream(repo: &Path) -> bool {
    let config_path = repo.join(".git").join("config");
    if let Ok(config) = std::fs::read_to_string(&config_path) {
        if let Some(branch) = current_branch(repo) {
            let section = format!("[branch \"{}\"]", branch);
            if let Some(pos) = config.find(&section) {
                let after = &config[pos + section.len()..];
                let next_section = after.find('[').unwrap_or(after.len());
                let branch_config = &after[..next_section];
                return branch_config.contains("remote = ") && branch_config.contains("merge = ");
            }
        }
        return false;
    }
    // Config file not readable (worktree, symlink, etc.) —
    // fall back to git subprocess which handles these cases natively.
    crate::policy::std_git_command()
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Whether a rebase operation is in progress.
pub(crate) fn is_rebase_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("rebase-merge").exists()
        || repo.join(".git").join("rebase-apply").exists()
}

/// Whether a merge operation is in progress.
pub(crate) fn is_merge_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("MERGE_HEAD").exists()
}

/// Whether a cherry-pick operation is in progress.
pub(crate) fn is_cherry_pick_in_progress(repo: &Path) -> bool {
    repo.join(".git").join("CHERRY_PICK_HEAD").exists()
}

/// Check if a repository is ready for operations (has valid HEAD with commits).
pub(crate) fn is_repo_ready(repo: &Path) -> bool {
    let head = repo.join(".git").join("HEAD");
    if !head.exists() {
        return false;
    }
    if let Ok(content) = std::fs::read_to_string(&head) {
        if content.trim().is_empty() {
            return false;
        }
    } else {
        return false;
    }
    let output = super::git_cmd()
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok();
    match output {
        Some(o) => {
            if !o.status.success() {
                return false;
            }
            let hash = String::from_utf8_lossy(&o.stdout).trim().to_string();
            !hash.is_empty()
        }
        None => false,
    }
}

/// Correct (modified, untracked) counts for a repo via `git status --porcelain`.
///
/// **Workaround for `dracon-git` library bug (v94.2.7)**: the library's
/// `get_status()` returns `modified_files` as the count of files with
/// any working-tree status flag set — including `is_wt_new()` (untracked).
/// This conflates "modified" with "untracked", which makes
/// Junk-Runner-bevy show 91 "MOD" for what is actually 3 untracked
/// test-results/ PNGs.
///
/// This helper queries `git status --porcelain` directly and returns
/// the correct split. Use this for live-report display and WARN
/// classification. The library's `modified_files` should NOT be used
/// for these purposes; it's only correct for daemon-side staging
/// logic (where the conflation is harmless).
///
/// Date: 2026-06-15 (goal 0ab367b5 / Junk-Runner-bevy WARN fix).
pub(crate) fn count_dirty_files_porcelain(repo: &Path) -> (usize, usize) {
    let output = super::git_cmd()
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .ok();
    let Some(o) = output else {
        return (0, 0);
    };
    if !o.status.success() {
        return (0, 0);
    }
    let stdout = String::from_utf8_lossy(&o.stdout);
    let mut modified = 0usize;
    let mut untracked = 0usize;
    for line in stdout.lines() {
        if line.len() < 3 {
            continue;
        }
        // Porcelain v1 format: "XY path" where X is index status, Y is
        // working-tree status. Both may be a space (no change).
        let bytes = line.as_bytes();
        let x = bytes[0];
        let y = bytes[1];
        // "Untracked" is when both X and Y are "?".
        if x == b'?' && y == b'?' {
            untracked += 1;
            continue;
        }
        // Anything else (including M, A, D, R, C, T, U) is a
        // modified/deleted/renamed/staged tracked file.
        if x != b' ' || y != b' ' {
            modified += 1;
        }
    }
    (modified, untracked)
}
