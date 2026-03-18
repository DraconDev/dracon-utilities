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

pub(crate) fn should_stage_entry(
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

pub(crate) fn can_restore_entry(entry: &dracon_git::types::DiffFile) -> bool {
    use dracon_git::types::FileStatus;
    matches!(
        entry.status,
        FileStatus::Modified | FileStatus::TypeChange | FileStatus::Renamed
    )
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

pub(crate) fn has_sync_relevant_dirty_entries(
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
