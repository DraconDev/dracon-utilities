use anyhow::Context;
use std::path::Path;
use std::process::Command as StdCommand;
use tokio::time::{Duration, timeout};

#[cfg(feature = "scribe")]
pub(crate) async fn update_project_state_from_ai(repo: &Path) -> anyhow::Result<()> {
    // Collect git context
    let git_log = StdCommand::new("git")
        .args(["log", "--format=%s", "-20"])
        .current_dir(repo)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let git_files = StdCommand::new("git")
        .args(["log", "--oneline", "--name-only", "-10"])
        .current_dir(repo)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let blueprint = dracon_git::read_blueprint_content(repo);

    // Resolve AI provider via the config system (reads routing policy + env vars + secrets)
    let resolved = ai_runtime_adapters::resolve_ai_runtime_config();
    // Use lane model policy to find free-tier models for scribe tasks
    let free_models = resolved.lane_model_policy.resolve("free", None);
    eprintln!("📝 scribe: free_models={:?}", free_models);
    eprintln!("📝 scribe: providers_with_keys={:?}", resolved.openai_providers.iter().filter(|p| !p.api_keys.is_empty()).map(|p| &p.model_id).collect::<Vec<_>>());
    let provider = if !free_models.is_empty() {
        resolved.openai_providers.iter()
            .find(|p| free_models.contains(&p.model_id) && !p.api_keys.is_empty())
            .or_else(|| resolved.openai_providers.iter().find(|p| !p.api_keys.is_empty()))
    } else {
        resolved.openai_providers.iter().find(|p| !p.api_keys.is_empty())
    };

    let (api_key, endpoint, model) = match provider {
        Some(p) => (p.api_keys[0].clone(), p.endpoint.clone(), p.payload_model.clone()),
        None => {
            eprintln!("📝 scribe: no AI provider configured (set up ~/.dracon/ai/routing-policy.json or set OPENROUTER_API_KEY)");
            return Ok(());
        }
    };

    let prompt = format!(
        "You are a scribe. Analyze git history and write a concise project-state.md.\n\n\
         ## Recent Git Log\n{}\n\n## File Changes\n{}\n\n## Blueprint\n{}\n\n\
         Write EXACTLY this format:\n\
         # Project State\n\n## Current Focus\n{{one line}}\n\n\
         ## Completed\n- [x] {{done}}\n\n## In Progress\n- [ ] {{active}}\n\n\
         ## Open Issues\n- {{blockers}}\n\n\
         Be factual. Infer from evidence.",
        git_log, git_files, blueprint
    );

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 1000,
        "temperature": 0.3,
    });

    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
    eprintln!("📝 scribe: calling {} (model: {})", endpoint, model);
    let request = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&body)
        .send();

    let resp = match timeout(Duration::from_secs(30), request).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return Err(anyhow::anyhow!("AI scribe request failed: {e}")),
        Err(_) => return Err(anyhow::anyhow!("AI scribe request timed out after 30s")),
    };

    if !resp.status().is_success() {
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            anyhow::bail!("auth failed (check ~/.dracon/ai/secrets/)");
        }
        anyhow::bail!("AI returned {}", resp.status());
    }

    let json: serde_json::Value = resp.json().await?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("AI response missing content"))?;

    let markdown = if let Some(start) = content.find("# Project State") {
        content[start..].trim()
    } else {
        content.trim()
    };

    let state_path = repo.join(".dracon/project-state.md");
    std::fs::create_dir_all(repo.join(".dracon"))?;
    std::fs::write(&state_path, markdown)
        .with_context(|| format!("writing {}", state_path.display()))?;
    eprintln!("📝 scribe: updated {}", state_path.display());

    Ok(())
}
