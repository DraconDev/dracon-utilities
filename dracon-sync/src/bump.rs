use ai_router_core::{infer_lane, LaneModelPolicy, RoutingMessage, RoutingTask};
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;

fn resolve_openrouter_key() -> Option<String> {
    let env_path = dirs::home_dir()?.join(".dracon/ai/secrets/openrouter.env");
    let content = std::fs::read_to_string(&env_path).ok()?;
    for line in content.lines() {
        if line.starts_with("OPENROUTER_API_KEY=") {
            return Some(line.split('=').nth(1)?.trim().to_string());
        }
    }
    None
}

fn load_policy() -> Option<LaneModelPolicy> {
    let policy_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".dracon/ai/routing-policy.json");
    let content = std::fs::read_to_string(&policy_path).ok()?;
    LaneModelPolicy::from_json(&content).ok()
}

fn resolve_models_for_task(task: RoutingTask, prompt: &str) -> Vec<String> {
    let policy = match load_policy() {
        Some(p) => p,
        None => return vec![format!("openrouter/{}", task.as_task_key())],
    };

    let models = policy.resolve_for_task(task, None);
    if !models.is_empty() {
        return models;
    }

    let inferred = infer_lane(&[RoutingMessage::user(prompt)]);
    if inferred != task {
        let fallback = policy.resolve_for_task(inferred, None);
        if !fallback.is_empty() {
            return fallback;
        }
    }

    vec![format!("openrouter/{}", task.as_task_key())]
}

#[derive(Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: i32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

pub(crate) fn bump_semver_patch(ver: &str) -> Option<String> {
    let parts: Vec<&str> = ver.split('.').collect();
    if parts.len() < 3 {
        return None;
    }
    if !parts[0].chars().all(|c| c.is_ascii_digit())
        || !parts[1].chars().all(|c| c.is_ascii_digit())
        || !parts[2].chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let major: u64 = parts[0].parse().ok()?;
    let minor: u64 = parts[1].parse().ok()?;
    let patch: u64 = parts[2].parse().ok()?;
    Some(format!("{}.{}.{}", major, minor, patch + 1))
}

pub(crate) fn bump_semver_minor(ver: &str) -> Option<String> {
    let parts: Vec<&str> = ver.split('.').collect();
    if parts.len() < 3 {
        return None;
    }
    if !parts[0].chars().all(|c| c.is_ascii_digit())
        || !parts[1].chars().all(|c| c.is_ascii_digit())
        || !parts[2].chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let major: u64 = parts[0].parse().ok()?;
    let minor: u64 = parts[1].parse().ok()?;
    let patch: u64 = parts[2].parse().ok()?;
    Some(format!("{}.{}.{}", major, minor + 1, 0))
}

pub(crate) fn bump_semver_major(ver: &str) -> Option<String> {
    let parts: Vec<&str> = ver.split('.').collect();
    if parts.len() < 3 {
        return None;
    }
    if !parts[0].chars().all(|c| c.is_ascii_digit())
        || !parts[1].chars().all(|c| c.is_ascii_digit())
        || !parts[2].chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let major: u64 = parts[0].parse().ok()?;
    let minor: u64 = parts[1].parse().ok()?;
    let patch: u64 = parts[2].parse().ok()?;
    Some(format!("{}.{}.{}", major + 1, 0, 0))
}

pub(crate) fn bump_first_json_string_field(
    content: &str,
    key: &str,
) -> Option<(String, String, String)> {
    // Tiny, formatting-preserving bump helper:
    // finds the first `"key": "x.y.z"` occurrence and bumps patch.
    let needle = format!("\"{}\"", key);
    let mut start = 0usize;
    while let Some(idx) = content[start..].find(&needle) {
        let key_pos = start + idx;
        let after_key = key_pos + needle.len();
        let rest = &content[after_key..];
        let colon_rel = rest.find(':')?;
        let after_colon = after_key + colon_rel + 1;
        let rest2 = &content[after_colon..];
        let q1_rel = rest2.find('"')?;
        let q1 = after_colon + q1_rel + 1;
        let rest3 = &content[q1..];
        let q2_rel = rest3.find('"')?;
        let q2 = q1 + q2_rel;
        let old_ver = &content[q1..q2];
        if let Some(new_ver) = bump_semver_patch(old_ver) {
            let mut out = String::with_capacity(content.len());
            out.push_str(&content[..q1]);
            out.push_str(&new_ver);
            out.push_str(&content[q2..]);
            return Some((out, old_ver.to_string(), new_ver));
        }
        start = after_key;
    }
    None
}

pub(crate) fn set_first_json_string_field_to_value(
    content: &str,
    key: &str,
    expected_old: &str,
    new_value: &str,
) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let mut start = 0usize;
    while let Some(idx) = content[start..].find(&needle) {
        let key_pos = start + idx;
        let after_key = key_pos + needle.len();
        let rest = &content[after_key..];
        let colon_rel = rest.find(':')?;
        let after_colon = after_key + colon_rel + 1;
        let rest2 = &content[after_colon..];
        let q1_rel = rest2.find('"')?;
        let q1 = after_colon + q1_rel + 1;
        let rest3 = &content[q1..];
        let q2_rel = rest3.find('"')?;
        let q2 = q1 + q2_rel;
        let old = &content[q1..q2];
        if old == expected_old {
            let mut out = String::with_capacity(content.len());
            out.push_str(&content[..q1]);
            out.push_str(new_value);
            out.push_str(&content[q2..]);
            return Some(out);
        }
        start = after_key;
    }
    None
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BumpOutcome {
    pub bumped_cargo_toml: bool,
    pub updated_cargo_lock: bool,
    pub bumped_workspace_package: bool,
}

pub(crate) fn bump_patch_version_in_repo(repo: &Path) -> Result<BumpOutcome> {
    fn bump_in_section(content: &str, target_section: &str) -> Option<(String, String)> {
        let mut out = String::with_capacity(content.len() + 16);
        let mut section = String::new();
        let mut changed = false;
        let mut new_version = String::new();

        for raw in content.split_inclusive('\n') {
            let line = raw.trim_end_matches('\n');
            let newline = if raw.ends_with('\n') { "\n" } else { "" };
            let trimmed = line.trim();

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section = trimmed.trim_matches(&['[', ']'][..]).trim().to_string();
                out.push_str(line);
                out.push_str(newline);
                continue;
            }

            if !changed && section == target_section {
                // Match `version = "x.y.z"` only inside the target section.
                if let Some(rest) = trimmed.strip_prefix("version") {
                    let rest = rest.trim_start();
                    if let Some(rest) = rest.strip_prefix('=') {
                        let rest = rest.trim_start();
                        if let Some((_, after_q1)) = rest.split_once('"') {
                            if let Some((ver, after_q2)) = after_q1.split_once('"') {
                                let parts: Vec<&str> = ver.split('.').collect();
                                if parts.len() >= 3
                                    && parts[0].chars().all(|c| c.is_ascii_digit())
                                    && parts[1].chars().all(|c| c.is_ascii_digit())
                                    && parts[2].chars().all(|c| c.is_ascii_digit())
                                {
                                    let major: u64 = parts[0].parse().ok()?;
                                    let minor: u64 = parts[1].parse().ok()?;
                                    let patch: u64 = parts[2].parse().ok()?;
                                    new_version = format!("{}.{}.{}", major, minor, patch + 1);

                                    // Reconstruct preserving indentation and any trailing comment.
                                    let indent: String =
                                        line.chars().take_while(|c| c.is_whitespace()).collect();
                                    out.push_str(&indent);
                                    out.push_str("version = \"");
                                    out.push_str(&new_version);
                                    out.push('"');
                                    out.push_str(after_q2);
                                    out.push_str(newline);
                                    changed = true;
                                    continue;
                                }
                            }
                        }
                    }
                }
            }

            out.push_str(line);
            out.push_str(newline);
        }

        if changed {
            Some((out, new_version))
        } else {
            None
        }
    }

    fn find_package_name(content: &str, target_section: &str) -> Option<String> {
        let mut section = String::new();
        for raw in content.split_inclusive('\n') {
            let line = raw.trim_end_matches('\n');
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section = trimmed.trim_matches(&['[', ']'][..]).trim().to_string();
                continue;
            }
            if section != target_section {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("name") {
                let rest = rest.trim_start();
                if let Some(rest) = rest.strip_prefix('=') {
                    let value = rest.trim();
                    if let Some(s) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                        return Some(s.to_string());
                    }
                }
            }
        }
        None
    }

    fn update_cargo_lock_package_version(
        repo: &Path,
        package_name: &str,
        new_version: &str,
    ) -> Result<bool> {
        let lock_path = repo.join("Cargo.lock");
        let Ok(content) = std::fs::read_to_string(&lock_path) else {
            return Ok(false);
        };

        let mut out = String::with_capacity(content.len());
        let mut in_pkg = false;
        let mut name_matches = false;
        let mut changed = false;

        for raw in content.split_inclusive('\n') {
            let line = raw.trim_end_matches('\n');
            let newline = if raw.ends_with('\n') { "\n" } else { "" };
            let trimmed = line.trim();

            if trimmed == "[[package]]" {
                in_pkg = true;
                name_matches = false;
                out.push_str(line);
                out.push_str(newline);
                continue;
            }

            if in_pkg {
                if let Some(rest) = trimmed.strip_prefix("name") {
                    let rest = rest.trim_start();
                    if let Some(rest) = rest.strip_prefix('=') {
                        let value = rest.trim();
                        if let Some(s) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                            name_matches = s == package_name;
                        }
                    }
                    out.push_str(line);
                    out.push_str(newline);
                    continue;
                }
            }

            if in_pkg && name_matches && trimmed.starts_with("version") {
                let rest = trimmed["version".len()..].trim_start();
                if rest.starts_with('=') {
                    let replacement = format!("version = \"{}\"", new_version);
                    if trimmed != replacement {
                        changed = true;
                        out.push_str(&replacement);
                        out.push_str(newline);
                        continue;
                    }
                }
            }

            out.push_str(line);
            out.push_str(newline);
        }

        if changed {
            std::fs::write(&lock_path, out)
                .with_context(|| format!("failed writing {}", lock_path.display()))?;
        }
        Ok(changed)
    }

    let cargo = repo.join("Cargo.toml");
    let Ok(content) = std::fs::read_to_string(&cargo) else {
        return Ok(BumpOutcome {
            bumped_cargo_toml: false,
            updated_cargo_lock: false,
            bumped_workspace_package: false,
        });
    };

    // Prefer workspace versioning when present.
    let (next, new_ver, bumped_section) =
        if let Some((next, v)) = bump_in_section(&content, "workspace.package") {
            (next, v, "workspace.package")
        } else if let Some((next, v)) = bump_in_section(&content, "package") {
            (next, v, "package")
        } else {
            return Ok(BumpOutcome {
                bumped_cargo_toml: false,
                updated_cargo_lock: false,
                bumped_workspace_package: false,
            });
        };

    if next == content {
        return Ok(BumpOutcome {
            bumped_cargo_toml: false,
            updated_cargo_lock: false,
            bumped_workspace_package: false,
        });
    }

    std::fs::write(&cargo, next).with_context(|| format!("failed writing {}", cargo.display()))?;

    // Keep Cargo.lock consistent for single-package repos: if we can find the package name in
    // the same bumped section (typically [package]), update the matching lock entry's version.
    let mut updated_cargo_lock = false;
    if bumped_section == "package" && !new_ver.is_empty() {
        if let Some(name) = find_package_name(&content, "package") {
            match update_cargo_lock_package_version(repo, &name, &new_ver) {
                Ok(changed) => updated_cargo_lock = changed,
                Err(e) => eprintln!(
                    "⚠️ failed to update Cargo.lock for {}: {}",
                    repo.display(),
                    e
                ),
            }
        }
    }

    Ok(BumpOutcome {
        bumped_cargo_toml: true,
        updated_cargo_lock,
        bumped_workspace_package: bumped_section == "workspace.package",
    })
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SimpleBumpOutcome {
    pub bumped: bool,
    pub updated_lock: bool,
}

pub(crate) fn bump_node_package_version_in_repo(repo: &Path) -> Result<SimpleBumpOutcome> {
    let pkg = repo.join("package.json");
    let Ok(content) = std::fs::read_to_string(&pkg) else {
        return Ok(SimpleBumpOutcome {
            bumped: false,
            updated_lock: false,
        });
    };
    let Some((next, old_ver, new_ver)) = bump_first_json_string_field(&content, "version") else {
        return Ok(SimpleBumpOutcome {
            bumped: false,
            updated_lock: false,
        });
    };
    if next != content {
        std::fs::write(&pkg, next).with_context(|| format!("failed writing {}", pkg.display()))?;
    }

    // Best-effort: keep package-lock.json root version aligned if it matches the old version.
    let mut updated_lock = false;
    let lock = repo.join("package-lock.json");
    if let Ok(lock_content) = std::fs::read_to_string(&lock) {
        if let Some(lock_next) =
            set_first_json_string_field_to_value(&lock_content, "version", &old_ver, &new_ver)
        {
            if lock_next != lock_content {
                std::fs::write(&lock, lock_next)
                    .with_context(|| format!("failed writing {}", lock.display()))?;
                updated_lock = true;
            }
        }
    }

    Ok(SimpleBumpOutcome {
        bumped: true,
        updated_lock,
    })
}

pub(crate) fn bump_version_file_in_repo(repo: &Path) -> Result<bool> {
    let p = repo.join("VERSION");
    let Ok(content) = std::fs::read_to_string(&p) else {
        return Ok(false);
    };
    let raw = content.trim();
    let Some(new_ver) = bump_semver_patch(raw) else {
        return Ok(false);
    };
    let next = format!("{}\n", new_ver);
    if next != content {
        std::fs::write(&p, next).with_context(|| format!("failed writing {}", p.display()))?;
        return Ok(true);
    }
    Ok(false)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BumpLevel {
    Major,
    Minor,
    Patch,
    None,
}

impl BumpLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            BumpLevel::Major => "major",
            BumpLevel::Minor => "minor",
            BumpLevel::Patch => "patch",
            BumpLevel::None => "none",
        }
    }
}



pub fn read_current_version(repo: &Path) -> Option<String> {
    if let Ok(cargo) = std::fs::read_to_string(repo.join("Cargo.toml")) {
        if let Some(version) = extract_version_from_cargo(&cargo) {
            return Some(version);
        }
    }
    if let Ok(pkg) = std::fs::read_to_string(repo.join("package.json")) {
        if let Some(version) = extract_version_from_json(&pkg, "version") {
            return Some(version);
        }
    }
    if let Ok(version_file) = std::fs::read_to_string(repo.join("VERSION")) {
        let trimmed = version_file.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn extract_version_from_cargo(content: &str) -> Option<String> {
    let mut section = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed.trim_matches(&['[', ']'][..]).trim().to_string();
        }
        if section == "package" || section == "workspace.package" {
            if let Some(rest) = trimmed.strip_prefix("version") {
                let rest = rest.trim_start().trim_start_matches('=').trim();
                if let Some(v) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn extract_version_from_json(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let mut start = 0usize;
    while let Some(idx) = content[start..].find(&needle) {
        let key_pos = start + idx;
        let after_key = key_pos + needle.len();
        let rest = &content[after_key..];
        let colon_rel = rest.find(':')?;
        let after_colon = after_key + colon_rel + 1;
        let rest2 = &content[after_colon..];
        let q1_rel = rest2.find('"')?;
        let q1 = after_colon + q1_rel + 1;
        let rest3 = &content[q1..];
        let q2_rel = rest3.find('"')?;
        let q2 = q1 + q2_rel;
        return Some(content[q1..q2].to_string());
    }
    None
}

async fn send_openrouter_request(
    client: &Client,
    api_key: &str,
    models: &[String],
    prompt: &str,
) -> Option<String> {
    for model in models {
        let request = OpenRouterRequest {
            model: model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: 20,
        };

        let resp = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                if let Ok(body) = r.json::<OpenRouterResponse>().await {
                    if let Some(choice) = body.choices.first() {
                        return Some(choice.message.content.clone());
                    }
                }
            }
            _ => continue,
        }
    }
    None
}

pub async fn ai_decide_bump_level(
    _repo: &Path,
    current_version: &str,
    staged_diff: &str,
    project_state: &str,
) -> BumpLevel {
    // First check: if only version-related files changed (Cargo.toml, package.json, VERSION, Cargo.lock)
    // and no source files, skip the bump - it means we already bumped this version
    let version_only_patterns = ["Cargo.toml", "package.json", "VERSION", "Cargo.lock"];
    let has_source_changes = staged_diff.lines()
        .filter(|line| !line.is_empty())
        .any(|line| {
            !version_only_patterns.iter().any(|p| line.contains(p))
        });
    
    if !has_source_changes {
        // Only version files changed - likely a duplicate bump, skip
        return BumpLevel::None;
    }
    
    let prompt = format!(r##"
You are a version bump advisor for a software project. Analyze the changes and decide if a version bump is warranted.

## Current Version
{current_version}

## Project State
{project_state}

## Staged Changes
{staged_diff}

## Decision Criteria
- "major": BREAKING CHANGE - incompatible API, removed features/options, major restructuring
- "minor": NEW FEATURE - backwards-compatible additions, new capabilities users would want
- "patch": BUG FIX - corrections to existing functionality, performance improvements, refactors with user-visible impact
- "none": NOISY/CHORE - docs, formatting, comments, CI config, version-only changes, dependencies without feature changes

IMPORTANT: Only bump if there is a MEANINGFUL change to the actual software. If the changes are just:
- Version/dependency updates without new features
- Documentation only
- CI/tooling changes  
- Small refactors with no user-visible impact
Then respond "none".

Respond with ONLY ONE WORD: major, minor, patch, or none. Nothing else."##);

    let api_key = match resolve_openrouter_key() {
        Some(k) => k,
        None => return BumpLevel::None,
    };

    let client = match Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(_) => return BumpLevel::None,
    };

    let models = resolve_models_for_task(RoutingTask::Free, &prompt);

    match send_openrouter_request(&client, &api_key, &models, &prompt).await {
        Some(content) => match content.trim().to_lowercase().as_str() {
            "major" => BumpLevel::Major,
            "minor" => BumpLevel::Minor,
            "patch" => BumpLevel::Patch,
            _ => BumpLevel::None,
        },
        None => BumpLevel::None,
    }
}

pub fn apply_version_bump_to_repo(repo: &Path, old_ver: &str, new_ver: &str) -> bool {
    if repo.join("Cargo.toml").exists() {
        if let Ok(content) = std::fs::read_to_string(repo.join("Cargo.toml")) {
            let bumped = bump_version_in_cargo_toml(&content, old_ver, new_ver);
            if bumped != content {
                if std::fs::write(repo.join("Cargo.toml"), bumped).is_ok() {
                    return true;
                }
            }
        }
    }
    if repo.join("package.json").exists() {
        if let Ok(content) = std::fs::read_to_string(repo.join("package.json")) {
            let bumped = bump_version_in_json(&content, old_ver, new_ver);
            if bumped != content {
                if std::fs::write(repo.join("package.json"), bumped).is_ok() {
                    return true;
                }
            }
        }
    }
    if repo.join("VERSION").exists() {
        if std::fs::write(repo.join("VERSION"), format!("{}\n", new_ver)).is_ok() {
            return true;
        }
    }
    false
}

fn bump_version_in_cargo_toml(content: &str, old_ver: &str, new_ver: &str) -> String {
    content.replace(&format!("version = \"{}\"", old_ver), &format!("version = \"{}\"", new_ver))
        .replace(&format!("version=\"{}\"", old_ver), &format!("version=\"{}\"", new_ver))
}

fn bump_version_in_json(content: &str, old_ver: &str, new_ver: &str) -> String {
    content.replace(&format!("\"version\": \"{}\"", old_ver), &format!("\"version\": \"{}\"", new_ver))
}
