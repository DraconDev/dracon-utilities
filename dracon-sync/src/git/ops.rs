//! Git process operations — spawn, timeout, kill, and capture output.

use std::path::{Path, PathBuf};
use std::time::Duration;
use anyhow::{Context, Result};
use tokio::process::Command as TokioCommand;

/// Kill descendant processes of a given PID using pkill (TERM then KILL).
pub(crate) async fn kill_descendants(pid: u32) {
    let pid_s = pid.to_string();

    async fn kill_group(pid_s: &str, signal: &str) {
        if let Ok(output) = TokioCommand::new("pkill")
            .args([signal, "-P", pid_s])
            .output()
            .await
        {
            if output.status.success() {
                return;
            }
        }
        let _ = TokioCommand::new("kill")
            .args(["-".to_string() + signal, "--".to_string(), "-".to_string() + pid_s])
            .output()
            .await;
    }

    kill_group(&pid_s, "TERM").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    kill_group(&pid_s, "KILL").await;
}

/// Run a child process with a timeout, capturing stderr on failure.
pub(crate) async fn run_child(
    mut child: tokio::process::Child,
    workdir: &Path,
    timeout_secs: u64,
    label: &str,
) -> Result<()> {
    let pid = child.id();
    let stderr_handle = child.stderr.take();
    let stderr_task = tokio::spawn(async move {
        if let Some(mut stderr) = stderr_handle {
            use tokio::io::AsyncReadExt;
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).trim().to_string()
        } else {
            String::new()
        }
    });
    let wait_result = tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait()).await;
    let stderr_output = stderr_task.await.unwrap_or_default();
    match wait_result {
        Ok(Ok(status)) => {
            if status.success() {
                Ok(())
            } else if stderr_output.is_empty() {
                Err(anyhow::anyhow!("{} failed in {} with status {}", label, workdir.display(), status))
            } else {
                Err(anyhow::anyhow!("{} failed in {} with status {}: {}", label, workdir.display(), status, stderr_output))
            }
        }
        Ok(Err(e)) => {
            let detail = if stderr_output.is_empty() { format!("{}", e) } else { format!("{}: {}", e, stderr_output) };
            Err(anyhow::anyhow!("{} failed in {}: {}", label, workdir.display(), detail))
        }
        Err(_) => {
            if let Some(pid) = pid {
                kill_descendants(pid).await;
            }
            let _ = child.start_kill();
            let _ = child.wait().await;
            Err(anyhow::anyhow!("{} timeout in {} after {}s", label, workdir.display(), timeout_secs))
        }
    }
}

/// Run a git command with a timeout using the tokio git command builder.
pub(crate) async fn run_git_with_timeout(
    repo: &Path,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
) -> Result<()> {
    let label = format!("git {}", op_label);
    let child = crate::policy::tokio_git_command()
        .args(args)
        .current_dir(repo)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {} in {}", label, repo.display()))?;
    run_child(child, repo, timeout_secs, &label).await
}

/// Run a git command with extra environment variables and a timeout.
pub(crate) async fn run_git_with_timeout_env(
    repo: &Path,
    args: &[&str],
    timeout_secs: u64,
    op_label: &str,
    env: &[(&str, &str)],
) -> Result<()> {
    let label = format!("git {}", op_label);
    let mut cmd = crate::policy::tokio_git_command();
    cmd.args(args)
        .current_dir(repo)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let child = cmd.spawn()
        .with_context(|| format!("failed to spawn {} in {}", label, repo.display()))?;
    run_child(child, repo, timeout_secs, &label).await
}

/// Create a temporary GIT_ASKPASS script that outputs the given token.
#[cfg(unix)]
pub(crate) async fn git_askpass_script(token: &str) -> Result<PathBuf> {
    let nano = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_path = std::env::temp_dir().join(format!(
        "dracon-git-askpass-{}-{}.sh",
        std::process::id(),
        nano
    ));
    let escaped = token.replace('\'', "'\"'\"'");
    let script = format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", escaped);
    tokio::fs::write(&tmp_path, &script).await.with_context(|| {
        format!("failed to write GIT_ASKPASS script to {}", tmp_path.display())
    })?;
    use std::os::unix::fs::PermissionsExt;
    let mut perms = tokio::fs::metadata(&tmp_path).await?.permissions();
    perms.set_mode(0o700);
    tokio::fs::set_permissions(&tmp_path, perms).await?;
    Ok(tmp_path)
}

#[cfg(not(unix))]
pub(crate) async fn git_askpass_script(_token: &str) -> Result<PathBuf> {
    anyhow::bail!("GIT_ASKPASS helper is only implemented on Unix")
}

/// Run a git command and capture its stdout as a string.
pub(crate) fn run_git_capture_output(repo: &Path, args: &[&str], op_label: &str) -> Result<String> {
    let output = crate::policy::std_git_command()
        .args(args)
        .current_dir(repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .with_context(|| format!("failed to run git {} in {}", op_label, repo.display()))?;
    if !output.status.success() {
        anyhow::bail!("git {} failed in {}", op_label, repo.display());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
