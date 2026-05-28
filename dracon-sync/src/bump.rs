pub(crate) const CONVENTIONAL_COMMIT_TYPES: &[&str] = &[
    "feat",
    "fix",
    "docs",
    "style",
    "refactor",
    "perf",
    "test",
    "build",
    "ci",
    "chore",
    "revert",
    "improvement",
    "security",
];

pub(crate) fn extract_version_from_cargo(content: &str) -> Option<String> {
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

pub(crate) fn extract_version_from_json(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    if let Some(idx) = content.find(&needle) {
        let key_pos = idx;
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
        Some(content[q1..q2].to_string())
    } else {
        None
    }
}

#[cfg(feature = "ai-bumper")]
pub async fn ai_decide_bump_level(
    _repo: &Path,
    current_version: &str,
    staged_diff: &str,
    project_state: &str,
) -> BumpLevel {
    use crate::simple_ai::{ChatMessage, SimpleAiService};
    use std::path::Path;

    let version_only_patterns = ["Cargo.toml", "package.json", "VERSION", "Cargo.lock"];
    let has_source_changes = staged_diff
        .lines()
        .filter(|line| !line.is_empty())
        .any(|line| !version_only_patterns.iter().any(|p| line.contains(p)));

    if !has_source_changes {
        return BumpLevel::None;
    }

    let prompt = format!(
        r##"You are a version bump advisor. Analyze the changes and decide if a version bump is warranted.

Current Version: {current_version}

Project State:
{project_state}

Staged Changes:
{staged_diff}

Respond with ONLY ONE WORD:
- "minor": NEW FEATURE
- "patch": BUG FIX / improvement
- "none": NOISY/CHORE (docs, deps, config only)

NEVER respond "major" — major version bumps are manual-only.
Respond with ONLY ONE WORD."##
    );

    let service = SimpleAiService::new();
    if service.is_empty() {
        return BumpLevel::None;
    }

    let messages = vec![ChatMessage::user(&prompt)];

    match service.chat(messages).await {
        Ok(content) => match content.trim().to_lowercase().as_str() {
            "major" => BumpLevel::None,
            "minor" => BumpLevel::Minor,
            "patch" => BumpLevel::Patch,
            _ => BumpLevel::None,
        },
        Err(_) => BumpLevel::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_from_cargo_package() {
        let content = r#"[package]
name = "test"
version = "1.2.3""#;
        assert_eq!(
            extract_version_from_cargo(content),
            Some("1.2.3".to_string())
        );
    }

    #[test]
    fn test_extract_version_from_cargo_workspace_package() {
        let content = r#"[workspace.package]
version = "2.0.0"

[package]
name = "test""#;
        assert_eq!(
            extract_version_from_cargo(content),
            Some("2.0.0".to_string())
        );
    }

    #[test]
    fn test_extract_version_from_cargo_no_version() {
        let content = r#"[package]
name = "test""#;
        assert_eq!(extract_version_from_cargo(content), None);
    }

    #[test]
    fn test_extract_version_from_cargo_ignore_workspace_without_version() {
        let content = r#"[workspace]
members = ["crate1", "crate2"]

[package]
name = "test"
version = "1.0.0""#;
        assert_eq!(
            extract_version_from_cargo(content),
            Some("1.0.0".to_string())
        );
    }

    #[test]
    fn test_extract_version_from_json() {
        let content = r#"{"version": "1.2.3"}"#;
        assert_eq!(
            extract_version_from_json(content, "version"),
            Some("1.2.3".to_string())
        );
    }

    #[test]
    fn test_extract_version_from_json_not_found() {
        let content = r#"{"name": "test"}"#;
        assert_eq!(extract_version_from_json(content, "version"), None);
    }

    #[test]
    fn test_extract_version_from_json_multiple_keys() {
        let content = r#"{"name": "test", "version": "1.0.0", "other": "value"}"#;
        assert_eq!(
            extract_version_from_json(content, "version"),
            Some("1.0.0".to_string())
        );
    }
}
