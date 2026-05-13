use anyhow::{Context, Result};
use dracon_git::types::{DiffFile, FileStatus};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;

use crate::policy::{std_git_command, tokio_git_command};

pub(crate) fn run_git_capture_output(repo: &Path, args: &[&str], op_label: &str) -> Result<String> {
    let output = std_git_command()
        .args(args)
        .current_dir(repo)
        .output()
        .with_context(|| format!("failed to run git {} in {}", op_label, repo.display()))?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(text)
}

pub(crate) async fn git_list_paths(repo: &Path, args: &[&str]) -> Result<Vec<PathBuf>> {
    let output = tokio_git_command()
        .args(args)
        .current_dir(repo)
        .output()
        .await
        .with_context(|| format!("failed to run git {:?} in {}", args, repo.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            eprintln!("⚠️ git {:?} failed in {}: {}", args, repo.display(), stderr.trim());
        }
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect())
}

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

pub(crate) async fn git_name_status_entries(
    repo: &Path,
    args: &[&str],
) -> Result<Vec<(PathBuf, FileStatus)>> {
    let output = tokio_git_command()
        .args(args)
        .current_dir(repo)
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

pub(crate) fn fallback_status_rank(status: &FileStatus) -> u8 {
    match status {
        FileStatus::Deleted => 5,
        FileStatus::Renamed => 4,
        FileStatus::TypeChange => 3,
        FileStatus::Added => 2,
        FileStatus::Modified => 1,
        FileStatus::Unknown => 0,
    }
}

pub(crate) async fn cli_diff_entries(repo: &Path) -> Result<Vec<DiffFile>> {
    let mut entries: BTreeMap<PathBuf, FileStatus> = BTreeMap::new();

    for args in [
        &["diff", "--name-status"][..],
        &["diff", "--cached", "--name-status"][..],
    ] {
        for (path, status) in git_name_status_entries(repo, args).await? {
            let should_replace = entries
                .get(&path)
                .map(|old| fallback_status_rank(&status) >= fallback_status_rank(old))
                .unwrap_or(true);
            if should_replace {
                entries.insert(path, status);
            }
        }
    }

    for path in git_list_paths(repo, &["ls-files", "--others", "--exclude-standard"]).await? {
        let should_replace = entries
            .get(&path)
            .map(|old| fallback_status_rank(&FileStatus::Added) >= fallback_status_rank(old))
            .unwrap_or(true);
        if should_replace {
            entries.insert(path, FileStatus::Added);
        }
    }

    Ok(entries
        .into_iter()
        .map(|(path, status)| DiffFile {
            path,
            status,
        })
        .collect())
}

pub(crate) async fn repo_diff_entries(repo: &Path) -> Result<Vec<DiffFile>> {
    cli_diff_entries(repo).await
}

/// Get the list of files that actually differ from HEAD (filter-aware).
/// Unlike `git status`, `git diff HEAD` applies clean filters and correctly
/// ignores files that only differ due to smudge filter decryption.
pub(crate) async fn git_diff_head_files(repo: &Path) -> Result<HashSet<PathBuf>> {
    let repo = repo.to_path_buf();
    let outcome = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::task::spawn_blocking(move || -> anyhow::Result<HashSet<PathBuf>> {
            let output = std::process::Command::new("git")
                .current_dir(&repo)
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
    ).await;
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

pub(crate) async fn staged_paths(repo: &Path) -> Result<Vec<PathBuf>> {
    git_list_paths(repo, &["diff", "--cached", "--name-only", "-z"]).await
}

pub(crate) async fn unstage_excluded_paths(
    repo: &Path,
    excluded_dir_names: &BTreeSet<String>,
) -> Result<usize> {
    let paths = staged_paths(repo).await?;
    let excluded: Vec<PathBuf> = paths
        .into_iter()
        .filter(|p| {
            p.components().any(|c| {
                if let std::path::Component::Normal(n) = c {
                    if let Some(s) = n.to_str() {
                        return excluded_dir_names.contains(s);
                    }
                }
                false
            })
        })
        .collect();
    if excluded.is_empty() {
        return Ok(0);
    }

    let mut args = vec!["reset", "HEAD", "--"];
    for p in &excluded {
        args.push(p);
    }

    let output = tokio_git_command()
        .args(&args)
        .current_dir(repo)
        .output()
        .await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("⚠️ unstage_excluded_paths failed: {}", stderr.trim());
    }
    Ok(excluded.len())
}

pub(crate) async fn unstage_oversized_paths(repo: &Path, max_stage_file_bytes: u64) -> Result<usize> {
    let paths = staged_paths(repo).await?;
    let mut oversized = Vec::new();

    for path in &paths {
        let full = repo.join(path);
        if let Ok(meta) = std::fs::metadata(&full) {
            if meta.len() > max_stage_file_bytes {
                oversized.push(path.clone());
            }
        }
    }

    if oversized.is_empty() {
        return Ok(0);
    }

    let mut args = vec!["reset", "HEAD", "--"];
    for p in &oversized {
        args.push(p);
    }

    let output = tokio_git_command()
        .args(&args)
        .current_dir(repo)
        .output()
        .await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("⚠️ unstage_oversized_paths failed: {}", stderr.trim());
    }
    Ok(oversized.len())
}

pub(crate) async fn restore_paths(repo: &Path, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut args = vec!["checkout", "--"];
    args.extend(paths);
    let output = tokio_git_command()
        .args(&args)
        .current_dir(repo)
        .output()
        .await
        .with_context(|| format!("failed to restore {} paths in {}", paths.len(), repo.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("did not match any file(s) known to git") {
            eprintln!("⚠️ restore_paths warning: {}", stderr.trim());
        }
    }
    Ok(())
}

pub(crate) fn top_level_dir(path: &str) -> Option<String> {
    path.split('/').next().map(|s| s.to_string())
}

pub(crate) fn rewrite_ahead_paths(
    ahead: &[String],
    rewrite_rules: &[(String, String)],
) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(ahead.len());
    for path in ahead {
        if let Some(first_dir) = top_level_dir(path) {
            let mut replaced = path.clone();
            for (from, to) in rewrite_rules {
                if first_dir == *from {
                    replaced = path.replacen(&format!("{}/", from), &format!("{}/", to), 1);
                    break;
                }
            }
            result.push(replaced);
        } else {
            result.push(path.clone());
        }
    }
    result
}