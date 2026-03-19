use anyhow::Context;
use std::path::Path;
use std::process::Command as StdCommand;
use tokio::time::{Duration, timeout};

/// Update project-state.md by calling `dracon-ai scribe` (the full AI service).
/// This delegates model selection, key resolution, and routing to dracon-ai.
#[cfg(feature = "scribe")]
pub(crate) async fn update_project_state_from_ai(repo: &Path) -> anyhow::Result<()> {
    let workdir = repo.to_path_buf();
    let repo_display = repo.display().to_string();

    let result = timeout(
        Duration::from_secs(60),
        tokio::task::spawn_blocking(move || {
            StdCommand::new("dracon-ai")
                .arg("scribe")
                .arg(&workdir)
                .output()
                .with_context(|| format!("failed to run dracon-ai scribe for {repo_display}"))
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
                // Skip gracefully if no API key configured
                if stderr.contains("no AI provider")
                    || stderr.contains("401")
                    || stderr.contains("Unauthorized")
                {
                    eprintln!("📝 scribe: skipped (no API key configured)");
                    return Ok(());
                }
                Err(anyhow::anyhow!(
                    "dracon-ai scribe failed for {repo_display}: {stderr}"
                ))
            }
        }
        Ok(Ok(Err(e))) => Err(e),
        Ok(Err(e)) => Err(anyhow::anyhow!("dracon-ai scribe task failed: {e}")),
        Err(_) => {
            eprintln!("📝 scribe: timed out after 60s for {repo_display}");
            Ok(())
        }
    }
}
