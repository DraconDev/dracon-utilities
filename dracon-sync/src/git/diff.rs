//! Diff and status operations — parse git diff/status output and collect staged entries.

use anyhow::{Context, Result};
use dracon_git::{
    types::{DiffFile, FileStatus},
    GitService,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Get the list of files that actually differ from HEAD (filter-aware).
/// Unlike `git status`, `git diff HEAD` applies clean filters and correctly
/// ignores files that only differ due to smudge filter decryption.
pub(crate) async fn git_diff_head_files(repo: &Path) -> Result<HashSet<PathBuf>> {
    let r = repo.to_path_buf();
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::task::spawn_blocking(move || -> anyhow::Result<HashSet<PathBuf>> {
            let output = crate::git::git_cmd()
                .current_dir(&r)
                .args(["diff", "HEAD", "--name-only", "-z"])
                .output()?;
            if !output.status.success() {
                anyhow::bail!("git diff HEAD exited with {}", output.status);
            }
            let files: HashSet<PathBuf> = String::from_utf8_lossy(&output.stdout)
                .split('\0')
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .collect();
            Ok(files)
        }),
    )
    .await;
    let inner = match outcome {
        Ok(inner) => inner,
        Err(_) => return Err(anyhow::anyhow!("git diff HEAD timed out")),
    };
    match inner {
        Ok(Ok(files)) => Ok(files),
        Ok(Err(e)) => Err(anyhow::anyhow!("git diff HEAD task failed: {}", e)),
        Err(e) => Err(anyhow::anyhow!("git diff HEAD task failed: {}", e)),
    }
}

/// Parse a single line from `git status --porcelain` or `git diff --name-status`.
pub(crate) fn parse_name_status_line(line: &str) -> Option<(PathBuf, FileStatus)> {
    let mut parts = line.split('\t');
    let status_raw = parts.next()?.trim();
    if status_raw.is_empty() {
        return None;
    }
    let status_char = status_raw.chars().next()?;
    let (path, status) = match status_char {
        'M' => (parts.next()?, FileStatus::Modified),
        'A' => (parts.next()?, FileStatus::Added),
        'D' => (parts.next()?, FileStatus::Deleted),
        'T' => (parts.next()?, FileStatus::TypeChange),
        'R' => {
            let _old = parts.next()?;
            let new = parts.next()?;
            (new, FileStatus::Renamed)
        }
        _ => return None,
    };
    Some((PathBuf::from(path.trim()), status))
}

/// Get name-status entries via `git diff --name-status` with custom args.
pub(crate) async fn git_name_status_entries(
    repo: &Path,
    args: &[&str],
) -> Result<Vec<(PathBuf, FileStatus)>> {
    let output = crate::git::tokio_git_cmd()
        .args(args)
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .with_context(|| format!("failed to run git {:?} in {}", args, repo.display()))?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter_map(parse_name_status_line)
        .collect::<Vec<_>>())
}

/// Git status rank for sorting: higher = more relevant to sync.
#[cfg(test)]
pub(crate) fn fallback_status_rank(status: &FileStatus) -> u8 {
    match status {
        FileStatus::Deleted => 5,
        FileStatus::Renamed => 4,
        FileStatus::TypeChange => 3,
        FileStatus::Added => 2,
        FileStatus::Modified => 1,
        FileStatus::Unknown => 0,
        _ => 0,
    }
}

/// Get diff entries via `git diff` CLI (fallback when libgit2 fails).
pub(crate) async fn cli_diff_entries(repo: &Path) -> Result<Vec<DiffFile>> {
    let output = crate::git::tokio_git_cmd()
        .args(["diff", "--name-status", "HEAD"])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();
    for line in stdout.lines() {
        if let Some((path, status)) = parse_name_status_line(line) {
            entries.push(DiffFile::new(path, status));
        }
    }
    Ok(entries)
}

/// Get untracked file entries via `git ls-files --others --exclude-standard`.
pub(crate) async fn untracked_entries(repo: &Path) -> Result<Vec<DiffFile>> {
    let output = crate::git::tokio_git_cmd()
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|p| DiffFile::new(PathBuf::from(p), FileStatus::Added))
        .collect())
}

/// Get diff entries from both repo status, diff, and untracked files.
/// This ensures untracked files are included in the diff entries so the
/// daemon can detect and commit them.
pub(crate) async fn repo_diff_entries(repo: &Path) -> Result<Vec<DiffFile>> {
    let svc = GitService::new(repo)?;
    let status = svc.get_status().await?;
    if status.is_clean {
        return Ok(Vec::new());
    }
    // Get diff entries between HEAD and working tree (includes both staged
    // and unstaged modifications, but NOT untracked files).
    let diff = cli_diff_entries(repo).await?;
    if !diff.is_empty() {
        // Only return diff entries if there are actual changes.
        // Also include any untracked files that may exist alongside mods.
        let untracked = untracked_entries(repo).await.unwrap_or_default();
        if untracked.is_empty() {
            return Ok(diff);
        }
        let mut combined = diff;
        combined.extend(untracked);
        return Ok(combined);
    }
    // cli_diff_entries returned empty. Check for untracked files or
    // staged-only changes.
    let untracked = untracked_entries(repo).await.unwrap_or_default();
    if !untracked.is_empty() {
        return Ok(untracked);
    }
    // Only staged files (git add'ed but no working tree differences yet)
    // or repos where diff parsing produced no results.
    Ok(Vec::new())
}

/// Get staged file paths from `git diff --cached --name-only`.
pub(crate) async fn staged_paths(repo: &Path) -> Result<HashSet<PathBuf>> {
    let output = crate::git::tokio_git_cmd()
        .args(["diff", "--cached", "--name-only", "-z"])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect())
}

/// Get the set of all currently-tracked file paths in the index.
/// Used to distinguish untracked (new) files from freshly-staged
/// tracked files when `auto_stage_untracked = false` is set.
pub(crate) async fn tracked_paths(repo: &Path) -> Result<HashSet<PathBuf>> {
    let output = crate::git::tokio_git_cmd()
        .args(["ls-files", "-z"])
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect())
}

/// Get the list of untracked `.md` and `.txt` files in a repo that
/// are NOT gitignored. These are the files the operator most likely
/// cares about: research artifacts, audit reports, design docs,
/// deliverables, and notes that have been written but not yet
/// `git add`-ed.
///
/// The daemon's commit-all policy auto-stages tracked changes but
/// intentionally leaves untracked content alone. This function gives
/// the operator (and the daemon's periodic guard) visibility into
/// the specific subset of untracked files that are usually
/// deliverables: documentation, notes, design docs.
///
/// Returns paths relative to the repo root, sorted. Files that
/// ARE gitignored are excluded (the `!*.md` re-include rule and
/// other exceptions are respected because we use
/// `git ls-files --others --exclude-standard`, which honors
/// `.gitignore`).
///
/// This is the defensive guard added in goal `e680cfa9` (2026-06-16)
/// to prevent the CWD-drift class of bug where an AI agent launches
/// from a repo subdirectory and writes a file at a doubled path,
/// leaving it untracked.
pub(crate) async fn noteworthy_untracked(
    repo: &Path,
) -> Result<Vec<String>> {
    let r = repo.to_path_buf();
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<String>> {
            let output = crate::git::git_cmd()
                .current_dir(&r)
                .args(["ls-files", "--others", "--exclude-standard", "-z"])
                .output()?;
            if !output.status.success() {
                anyhow::bail!("git ls-files exited with {}", output.status);
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut result: Vec<String> = stdout
                .split('\0')
                .filter(|s| !s.is_empty())
                .filter(|s| {
                    s.ends_with(".md") || s.ends_with(".txt")
                })
                .map(|s| s.to_string())
                .collect();
            result.sort();
            Ok(result)
        }),
    )
    .await;
    let inner = match outcome {
        Ok(inner) => inner,
        Err(_) => return Err(anyhow::anyhow!("noteworthy_untracked timed out")),
    };
    match inner {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(anyhow::anyhow!("noteworthy_untracked task failed: {}", e)),
    }
}

#[cfg(test)]
mod noteworthy_untracked_tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    fn init_test_repo(dir: &Path) {
        // Use a fresh git user identity for the test repo to avoid
        // touching the operator's real config.
        let _ = Command::new("git")
            .args(["init", "--initial-branch=main"])
            .arg(dir)
            .output();
        let _ = Command::new("git")
            .args(["-C", dir.to_str().unwrap(), "config", "user.email", "test@test.local"])
            .output();
        let _ = Command::new("git")
            .args(["-C", dir.to_str().unwrap(), "config", "user.name", "Test"])
            .output();
    }

    #[tokio::test]
    async fn test_noteworthy_untracked_empty_repo() {
        let tmp = tempdir().unwrap();
        let repo = tmp.path();
        init_test_repo(repo);
        let result = noteworthy_untracked(repo).await.unwrap();
        assert!(result.is_empty(), "fresh repo should have no untracked .md/.txt");
    }

    #[tokio::test]
    async fn test_noteworthy_untracked_finds_md_files() {
        let tmp = tempdir().unwrap();
        let repo = tmp.path();
        init_test_repo(repo);
        // Create the docs/ dir FIRST, then write the file inside
        std::fs::create_dir_all(repo.join("docs")).unwrap();
        std::fs::write(repo.join("docs").join("research.md"), "# Research").unwrap();
        let result = noteworthy_untracked(repo).await.unwrap();
        assert!(
            result.iter().any(|p| p.ends_with("research.md")),
            "should find untracked research.md, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_noteworthy_untracked_finds_txt_files() {
        let tmp = tempdir().unwrap();
        let repo = tmp.path();
        init_test_repo(repo);
        std::fs::write(repo.join("notes.txt"), "scratch notes").unwrap();
        let result = noteworthy_untracked(repo).await.unwrap();
        assert!(
            result.iter().any(|p| p.ends_with("notes.txt")),
            "should find untracked notes.txt, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_noteworthy_untracked_ignores_gitignored() {
        let tmp = tempdir().unwrap();
        let repo = tmp.path();
        init_test_repo(repo);
        std::fs::write(repo.join(".gitignore"), "scratch/\n").unwrap();
        std::fs::create_dir_all(repo.join("scratch")).unwrap();
        std::fs::write(repo.join("scratch").join("notes.md"), "should be ignored").unwrap();
        // Stage and commit the .gitignore so the file is properly
        // excluded by --exclude-standard.
        let _ = Command::new("git")
            .args(["-C", repo.to_str().unwrap(), "add", ".gitignore"])
            .output();
        let _ = Command::new("git")
            .args(["-C", repo.to_str().unwrap(), "commit", "-m", "init"])
            .output();
        let result = noteworthy_untracked(repo).await.unwrap();
        assert!(
            !result.iter().any(|p| p.contains("scratch")),
            "gitignored scratch/notes.md should NOT appear, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_noteworthy_untracked_excludes_other_extensions() {
        let tmp = tempdir().unwrap();
        let repo = tmp.path();
        init_test_repo(repo);
        std::fs::write(repo.join("photo.png"), b"fake png").unwrap();
        std::fs::write(repo.join("data.json"), "{}").unwrap();
        std::fs::write(repo.join("research.md"), "# ok").unwrap();
        let result = noteworthy_untracked(repo).await.unwrap();
        assert_eq!(result.len(), 1, "only the .md should appear, got: {:?}", result);
        assert!(result[0].ends_with("research.md"));
    }

    #[tokio::test]
    async fn test_noteworthy_untracked_finds_doubled_path() {
        // The actual bug case from goal e680cfa9: an AI agent launched
        // from /repo/subdir/ writes to a relative path
        // 'subdir/file.md' which lands at /repo/subdir/subdir/file.md.
        // This test simulates that scenario at the file-system level.
        let tmp = tempdir().unwrap();
        let repo = tmp.path();
        init_test_repo(repo);
        // The repo's CWD-equivalent is repo/subdir
        let subdir = repo.join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();
        // AI agent writes to "subdir/file.md" relative to its CWD
        let doubled_path = subdir.join("subdir");
        std::fs::create_dir_all(&doubled_path).unwrap();
        std::fs::write(doubled_path.join("file.md"), "# content").unwrap();
        let result = noteworthy_untracked(repo).await.unwrap();
        assert!(
            result.iter().any(|p| p.contains("subdir/subdir/file.md")),
            "the doubled-path file should be found, got: {:?}",
            result
        );
    }
}
