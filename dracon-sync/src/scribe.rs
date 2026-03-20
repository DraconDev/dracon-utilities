use anyhow::Context;
use std::path::Path;
use std::process::Command as StdCommand;
use tokio::time::{Duration, timeout};

#[cfg(feature = "scribe")]
pub(crate) async fn update_project_state_from_ai(repo: &Path) -> anyhow::Result<()> {
    let workdir = repo.to_path_buf();
    let repo_display = repo.display().to_string();

    let result = timeout(
        Duration::from_secs(150),
        tokio::task::spawn_blocking(move || {
            StdCommand::new("dracon-ai")
                .arg("scribe")
                .arg(&workdir)
                .output()
                .with_context(|| format!("failed to run dracon-ai scribe for {}", workdir.display()))
        }),
    )
    .await;

    match result {
        Ok(Ok(Ok(output))) => {
            if output.status.success() {
                eprintln!("📝 scribe: updated {repo_display}/.dracon/project-state.md");
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("no AI provider")
                    || stderr.contains("401")
                    || stderr.contains("Unauthorized")
                {
                    eprintln!("📝 scribe: skipped (no API key configured)");
                    return Ok(());
                }
                Err(anyhow::anyhow!("dracon-ai scribe failed: {}", stderr))
            }
        }
        Ok(Ok(Err(e))) => {
            eprintln!("📝 scribe: failed for {}: {}", repo_display, e);
            Err(e)
        }
        Ok(Err(e)) => {
            eprintln!("📝 scribe: timed out for {}", repo_display);
            Ok(())
        }
        Err(_) => {
            eprintln!("📝 scribe: timed out after 150s for {}", repo_display);
            Ok(())
        }
    }
}

#[cfg(not(feature = "scribe"))]
pub(crate) async fn update_project_state_from_ai(_repo: &Path) -> anyhow::Result<()> {
    Ok(())
}
