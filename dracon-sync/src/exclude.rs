use anyhow::Result;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use dracon_git::types::{DiffFile, FileStatus};

use crate::policy::SyncPolicy;

pub(crate) fn normalized_dir_name(value: &str) -> String {
    value.trim_matches('/').to_ascii_lowercase()
}

pub(crate) fn excluded_dir_names_set(policy: &SyncPolicy) -> BTreeSet<String> {
    policy
        .exclude_dir_names
        .iter()
        .map(|d| normalized_dir_name(d))
        .filter(|d| !d.is_empty())
        .collect()
}

pub(crate) fn is_nested_git_repo_path(path: &Path, current_repo_git: &Path) -> Option<PathBuf> {
    let mut current = path;
    while let Some(parent) = current.parent() {
        if parent == current {
            break;
        }
        let candidate = parent.join(".git");
        if candidate.exists() {
            if candidate == current_repo_git {
                break;
            }
            return Some(parent.to_path_buf());
        }
        current = parent;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_dir_name() {
        assert_eq!(normalized_dir_name("target"), "target");
        assert_eq!(normalized_dir_name("/target/"), "target");
        assert_eq!(normalized_dir_name("Target"), "target");
        assert_eq!(normalized_dir_name(".tmp-123"), ".tmp-123");
    }

    #[test]
    fn test_is_excluded_dir_name_exact() {
        let excluded: BTreeSet<String> = ["target", "node_modules", ".cache"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        assert!(is_excluded_dir_name("target", &excluded));
        assert!(is_excluded_dir_name("node_modules", &excluded));
        assert!(is_excluded_dir_name(".cache", &excluded));
        assert!(!is_excluded_dir_name("src", &excluded));
    }

    #[test]
    fn test_is_excluded_dir_name_pattern() {
        let excluded: BTreeSet<String> = [".tmp-".to_string()].into_iter().collect();

        assert!(is_excluded_dir_name(".tmp-abc", &excluded));
        assert!(is_excluded_dir_name(".tmp-123", &excluded));
        assert!(!is_excluded_dir_name(".tmp", &excluded));
        assert!(!is_excluded_dir_name("tmp-abc", &excluded));
    }

    #[test]
    fn test_excluded_dir_names_set() {
        let policy = SyncPolicy {
            exclude_dir_names: vec![
                "target".to_string(),
                "/node_modules/".to_string(),
                ".cache".to_string(),
            ],
            ..Default::default()
        };

        let set = excluded_dir_names_set(&policy);
        assert!(set.contains("target"));
        assert!(set.contains("node_modules"));
        assert!(set.contains(".cache"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_excluded_dir_names_set_removes_empty() {
        let policy = SyncPolicy {
            exclude_dir_names: vec!["target".to_string(), "".to_string(), "   ".to_string()],
            ..Default::default()
        };

        let set = excluded_dir_names_set(&policy);
        assert!(set.contains("target"));
        assert!(!set.contains(""));
    }
}

impl Default for SyncPolicy {
    fn default() -> Self {
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
}

pub(crate) fn is_excluded_dir_name(name: &str, excluded_dir_names: &BTreeSet<String>) -> bool {
    let normalized = normalized_dir_name(name);
    for pattern in excluded_dir_names {
        if *pattern == normalized {
            return true;
        }
        if pattern.ends_with('-')
            && pattern.starts_with('.')
            && normalized.starts_with(&pattern[..pattern.len() - 1])
        {
            return true;
        }
        if pattern.ends_with('*') && normalized.starts_with(&pattern[..pattern.len() - 1]) {
            return true;
        }
    }
    false
}

pub(crate) fn is_excluded_change_path(path: &Path, excluded_dir_names: &BTreeSet<String>) -> bool {
    path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .any(|name| is_excluded_dir_name(name, excluded_dir_names))
}

pub(crate) fn matches_file_pattern(file_name: &str, pattern: &str) -> bool {
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

pub(crate) fn is_excluded_file(file_path: &Path, excluded_patterns: &[String]) -> bool {
    let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    for pattern in excluded_patterns {
        if matches_file_pattern(file_name, pattern) {
            return true;
        }
    }
    false
}

/// Check if a path is a gitlink (mode 160000) with an unchanged pointer.
/// Returns true if the entry is a submodule-like directory whose HEAD commit
/// matches what the parent repo tracks, meaning the "dirty" state is just
/// the submodule's own working tree being dirty (not a pointer change).
pub(crate) fn is_gitlink_unchanged(repo: &Path, path: &Path) -> bool {
    let output = std::process::Command::new("git")
        .current_dir(repo)
        .args(["ls-tree", "HEAD", "--"])
        .arg(path)
        .output();
    let Ok(out) = output else { return false };
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Format: "160000 commit <sha>\t<path>"
    if !stdout.starts_with("160000 ") {
        return false;
    }
    let Some(sha) = stdout.split_whitespace().nth(2) else {
        return false;
    };
    // Check if the submodule's current HEAD matches the tracked sha
    let sub_output = std::process::Command::new("git")
        .current_dir(repo.join(path))
        .args(["rev-parse", "HEAD"])
        .output();
    let Ok(sub_out) = sub_output else {
        return false;
    };
    let sub_sha = String::from_utf8_lossy(&sub_out.stdout).trim().to_string();
    sub_sha == sha
}

pub(crate) fn should_stage_entry(
    repo: &Path,
    entry: &dracon_git::types::DiffFile,
    excluded_dir_names: &BTreeSet<String>,
    excluded_file_patterns: &[String],
    max_stage_file_bytes: u64,
) -> bool {
    if is_excluded_change_path(&entry.path, excluded_dir_names) {
        return false;
    }

    if is_excluded_file(&entry.path, excluded_file_patterns) {
        return false;
    }

    let full_path = repo.join(&entry.path);

    // Skip staging entries that live inside a nested git repo.
    // The nested repo has its own daemon managing it — staging its files from
    // the parent creates a conflict where both daemons fight over the same tree.
    let current_repo_git = repo.join(".git");
    if let Some(nested) = is_nested_git_repo_path(&full_path, &current_repo_git) {
        eprintln!(
            "ℹ️ skip {}: nested git repo detected ({}) — managed by its own daemon",
            full_path.display(),
            nested.display()
        );
        return false;
    }

    // Submodules and directory type changes
    if matches!(entry.status, dracon_git::types::FileStatus::TypeChange) {
        return true;
    }

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
            // Skip gitlink entries with unchanged pointers (dirty submodule
            // working trees that don't represent a pointer change)
            if is_gitlink_unchanged(repo, &entry.path) {
                return false;
            }
            true
        }
        Ok(_) => true,
        Err(_) => {
            // File doesn't exist on disk
            if matches!(entry.status, dracon_git::types::FileStatus::Deleted) {
                // Deleted files should be staged - they don't exist on disk by definition
                true
            } else {
                // File doesn't exist and isn't a deletion - don't stage
                // This handles partial checkouts or files that were never there
                false
            }
        }
    }
}

pub(crate) fn can_restore_entry(repo: &Path, entry: &dracon_git::types::DiffFile) -> bool {
    use dracon_git::types::FileStatus;
    if !matches!(
        entry.status,
        FileStatus::Modified | FileStatus::TypeChange | FileStatus::Renamed
    ) {
        return false;
    }
    let full_path = repo.join(&entry.path);
    let current_repo_git = repo.join(".git");
    is_nested_git_repo_path(&full_path, &current_repo_git).is_none()
}

pub(crate) fn is_large_untracked(
    entry: &dracon_git::types::DiffFile,
    repo: &Path,
    threshold: u64,
) -> bool {
    use dracon_git::types::FileStatus;
    if entry.status != FileStatus::Added {
        return false;
    }
    let full_path = repo.join(&entry.path);
    let current_repo_git = repo.join(".git");
    if is_nested_git_repo_path(&full_path, &current_repo_git).is_some() {
        return false;
    }
    match std::fs::metadata(&full_path) {
        Ok(meta) if meta.is_file() => meta.len() > threshold,
        _ => false,
    }
}

pub(crate) fn append_to_gitignore(repo: &Path, patterns: &[String]) -> Result<()> {
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
    let block_begin_idx = lines
        .iter()
        .position(|l| l.contains("--- BEGIN DRACON MANAGED BLOCK ---"));
    let block_end_idx = lines
        .iter()
        .position(|l| l.contains("--- END DRACON MANAGED BLOCK ---"));

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
    let has_large_files_section = lines
        .iter()
        .any(|l| l.contains("# Large files (auto-added by dracon-sync)"));

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
pub(crate) fn handle_large_untracked(
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

/// Common build / generated output directory names that should never be tracked.
/// These are checked IN ADDITION to exclude_dir_names to catch directories
/// that aren't in the standard exclusion list but are clearly build artifacts.
fn is_build_output_dir_name(name: &str) -> bool {
    matches!(
        name,
        ".output" | ".out" | "output" | "bin" | "obj" | "generated" | "gen" | ".next" | "dist-new"
    ) || name.ends_with(".output")
        || name.ends_with("_output")
        || name.starts_with("output-")
}

/// Find tracked files that live inside excluded directories or common build
/// output directories. These should never have been committed (build artifacts,
/// generated files, etc.). Removes them from git tracking and adds the directory
/// patterns to .gitignore.
pub(crate) fn remove_tracked_excluded_paths(
    repo: &Path,
    excluded_dir_names: &BTreeSet<String>,
) -> Result<Option<Vec<String>>> {
    let output = std::process::Command::new("git")
        .current_dir(repo)
        .args(["ls-files", "-z"])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    let mut top_level_excluded: BTreeSet<String> = BTreeSet::new();
    let mut to_remove: Vec<String> = Vec::new();

    for file in &files {
        let path = Path::new(file);
        let mut found_excluded = false;

        // Check standard excluded dir names
        if is_excluded_change_path(path, excluded_dir_names) {
            for component in path.components() {
                let name = component.as_os_str().to_str().unwrap_or("");
                if is_excluded_dir_name(name, excluded_dir_names) {
                    top_level_excluded.insert(name.to_string());
                    found_excluded = true;
                    break;
                }
            }
        }

        // Also detect common build output directories
        if !found_excluded {
            for component in path.components() {
                let name = component.as_os_str().to_str().unwrap_or("");
                if is_build_output_dir_name(name) {
                    top_level_excluded.insert(name.to_string());
                    found_excluded = true;
                    break;
                }
            }
        }

        if found_excluded {
            to_remove.push(file.to_string());
        }
    }

    if to_remove.is_empty() {
        return Ok(None);
    }

    let patterns: Vec<String> = top_level_excluded
        .iter()
        .map(|d| format!("{}/", d))
        .collect();
    eprintln!(
        "📝 {} has {} tracked file(s) inside build-artifact dirs {:?} — removing from git and adding to .gitignore",
        repo.display(),
        to_remove.len(),
        patterns
    );

    append_to_gitignore(repo, &patterns)?;

    for chunk in to_remove.chunks(50) {
        let mut args = vec!["rm", "-q", "--cached", "--"];
        for f in chunk {
            args.push(f);
        }
        let status = std::process::Command::new("git")
            .current_dir(repo)
            .args(&args)
            .status()?;
        if !status.success() {
            eprintln!(
                "⚠️ git rm --cached failed for some files in {}",
                repo.display()
            );
        }
    }

    Ok(Some(top_level_excluded.into_iter().collect()))
}

pub(crate) fn has_sync_relevant_dirty_entries(
    repo: &Path,
    entries: &[dracon_git::types::DiffFile],
    excluded_dir_names: &BTreeSet<String>,
    excluded_file_patterns: &[String],
    max_stage_file_bytes: u64,
) -> bool {
    let current_repo_git = repo.join(".git");
    entries.iter().any(|entry| {
        let full_path = repo.join(&entry.path);

        // Skip entries inside nested git repos — managed by their own daemon
        if is_nested_git_repo_path(&full_path, &current_repo_git).is_some() {
            return false;
        }

        // Skip gitlink entries with unchanged pointers entirely
        // Use repo.join() because entry.path is relative to repo, not CWD
        if full_path.is_dir() && is_gitlink_unchanged(repo, &entry.path) {
            return false;
        }
        should_stage_entry(
            repo,
            entry,
            excluded_dir_names,
            excluded_file_patterns,
            max_stage_file_bytes,
        ) || can_restore_entry(repo, entry)
            || is_large_untracked(entry, repo, max_stage_file_bytes)
    })
}
