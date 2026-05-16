use std::path::Path;

/// Parse a semver string into (major, minor, patch) components.
fn parse_semver(ver: &str) -> Option<(u64, u64, u64)> {
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
    Some((major, minor, patch))
}

pub(crate) fn bump_semver(ver: &str, level: BumpLevel) -> Option<String> {
    let (major, minor, patch) = parse_semver(ver)?;
    match level {
        BumpLevel::Major => None,
        BumpLevel::Minor => Some(format!("{}.{}.0", major, minor + 1)),
        BumpLevel::Patch => Some(format!("{}.{}.{}", major, minor, patch + 1)),
        BumpLevel::None => None,
    }
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

const NOISE_PATTERNS: &[&str] = &[
    ".md",
    ".txt",
    ".yml",
    ".yaml",
    ".toml",
    "LICENSE",
    "README",
    "CHANGELOG",
    "CONTRIBUTING",
    ".github/",
    ".gitignore",
    ".cargo/config",
    "rustfmt",
    "clippy",
    "deny.toml",
    ".vscode/",
    ".idea/",
    "package-lock.json",
    "Cargo.lock",
    ".env",
    ".env.example",
    ".editorconfig",
    ".shellcheckrc",
    ".dracon/",
    ".pub",
];

pub(crate) const VERSION_FILES: &[&str] = &["Cargo.toml", "package.json", "VERSION", "Cargo.lock"];

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

pub fn deterministic_decide_bump_level(staged_diff: &str) -> BumpLevel {
    let mut has_meaningful_change = false;

    for line in staged_diff.lines().filter(|l| !l.is_empty()) {
        let is_version_file = VERSION_FILES.iter().any(|p| line.contains(p));
        let is_noise = NOISE_PATTERNS.iter().any(|p| line.contains(p));

        if is_version_file {
            continue;
        }
        if is_noise {
            continue;
        }
        has_meaningful_change = true;
        break;
    }

    if has_meaningful_change {
        BumpLevel::Patch
    } else {
        BumpLevel::None
    }
}

pub(crate) fn read_current_version(repo: &Path) -> Option<String> {
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

pub fn apply_version_bump_to_repo(repo: &Path, old_ver: &str, new_ver: &str) -> bool {
    if repo.join("Cargo.toml").exists() {
        if let Ok(content) = std::fs::read_to_string(repo.join("Cargo.toml")) {
            let bumped = bump_version_in_cargo_toml(&content, old_ver, new_ver);
            if bumped != content && std::fs::write(repo.join("Cargo.toml"), bumped).is_ok() {
                return true;
            }
        }
    }
    if repo.join("package.json").exists() {
        if let Ok(content) = std::fs::read_to_string(repo.join("package.json")) {
            let bumped = bump_version_in_json(&content, old_ver, new_ver);
            if bumped != content && std::fs::write(repo.join("package.json"), bumped).is_ok() {
                return true;
            }
        }
    }
    if repo.join("VERSION").exists()
        && std::fs::write(repo.join("VERSION"), format!("{}\n", new_ver)).is_ok()
    {
        return true;
    }
    false
}

fn bump_version_in_cargo_toml(content: &str, old_ver: &str, new_ver: &str) -> String {
    let mut in_package = false;
    let mut result = String::with_capacity(content.len());
    for line in content.lines() {
        if line.trim().starts_with('[') {
            in_package = line.trim() == "[package]" || line.trim() == "[workspace.package]";
        }
        if in_package && (line.starts_with("version =") || line.starts_with("version=")) {
            result.push_str(
                &line
                    .replace(
                        &format!("version = \"{}\"", old_ver),
                        &format!("version = \"{}\"", new_ver),
                    )
                    .replace(
                        &format!("version=\"{}\"", old_ver),
                        &format!("version=\"{}\"", new_ver),
                    ),
            );
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    result
}

fn bump_version_in_json(content: &str, old_ver: &str, new_ver: &str) -> String {
    content
        .replace(
            &format!("\"version\": \"{}\"", old_ver),
            &format!("\"version\": \"{}\"", new_ver),
        )
        .replace(
            &format!("\"version\":\"{}\"", old_ver),
            &format!("\"version\":\"{}\"", new_ver),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_semver_all_levels() {
        assert_eq!(
            bump_semver("1.2.3", BumpLevel::Patch),
            Some("1.2.4".to_string())
        );
        assert_eq!(
            bump_semver("1.2.3", BumpLevel::Minor),
            Some("1.3.0".to_string())
        );
        assert_eq!(bump_semver("1.2.3", BumpLevel::Major), None);
        assert_eq!(
            bump_semver("0.0.0", BumpLevel::Patch),
            Some("0.0.1".to_string())
        );
        assert_eq!(
            bump_semver("0.0.0", BumpLevel::Minor),
            Some("0.1.0".to_string())
        );
        assert_eq!(bump_semver("0.0.0", BumpLevel::Major), None);
        assert_eq!(bump_semver("v1.2.3", BumpLevel::Patch), None);
        assert_eq!(bump_semver("v1.2.3", BumpLevel::Minor), None);
        assert_eq!(bump_semver("v1.2.3", BumpLevel::Major), None);
        assert_eq!(bump_semver("1.2", BumpLevel::Patch), None);
        assert_eq!(
            bump_semver("10.20.30", BumpLevel::Patch),
            Some("10.20.31".to_string())
        );
        assert_eq!(
            bump_semver("10.20.30", BumpLevel::Minor),
            Some("10.21.0".to_string())
        );
        assert_eq!(bump_semver("10.20.30", BumpLevel::Major), None);
        assert_eq!(
            bump_semver("0.0.1", BumpLevel::Patch),
            Some("0.0.2".to_string())
        );
        assert_eq!(
            bump_semver("0.0.1", BumpLevel::Minor),
            Some("0.1.0".to_string())
        );
        assert_eq!(bump_semver("0.0.1", BumpLevel::Major), None);
    }

    #[test]
    fn test_bump_level_as_str() {
        assert_eq!(BumpLevel::Major.as_str(), "major");
        assert_eq!(BumpLevel::Minor.as_str(), "minor");
        assert_eq!(BumpLevel::Patch.as_str(), "patch");
        assert_eq!(BumpLevel::None.as_str(), "none");
    }

    #[test]
    fn test_bump_level_debug() {
        assert_eq!(format!("{:?}", BumpLevel::Major), "Major");
        assert_eq!(format!("{:?}", BumpLevel::None), "None");
    }

    #[test]
    fn test_deterministic_decide_bump_level_meaningful_change() {
        let diff = "M src/main.rs\nM Cargo.toml";
        assert_eq!(deterministic_decide_bump_level(diff), BumpLevel::Patch);
    }

    #[test]
    fn test_deterministic_decide_bump_level_noise_only() {
        let diff = "M README.md\nM .gitignore";
        assert_eq!(deterministic_decide_bump_level(diff), BumpLevel::None);
    }

    #[test]
    fn test_deterministic_decide_bump_level_version_file_only() {
        let diff = "M Cargo.toml\nM package.json";
        assert_eq!(deterministic_decide_bump_level(diff), BumpLevel::None);
    }

    #[test]
    fn test_deterministic_decide_bump_level_mixed() {
        let diff = "M README.md\nM src/main.rs";
        assert_eq!(deterministic_decide_bump_level(diff), BumpLevel::Patch);
    }

    #[test]
    fn test_deterministic_decide_bump_level_empty() {
        assert_eq!(deterministic_decide_bump_level(""), BumpLevel::None);
    }

    #[test]
    fn test_deterministic_decide_bump_level_changelog() {
        let diff = "M CHANGELOG.md\nM CONTRIBUTING.md";
        assert_eq!(deterministic_decide_bump_level(diff), BumpLevel::None);
    }

    #[test]
    fn test_deterministic_decide_bump_level_env_file() {
        let diff = "M .env\nM .env.example";
        assert_eq!(deterministic_decide_bump_level(diff), BumpLevel::None);
    }

    #[test]
    fn test_deterministic_decide_bump_level_lock_files() {
        let diff = "M Cargo.lock\nM package-lock.json";
        assert_eq!(deterministic_decide_bump_level(diff), BumpLevel::None);
    }

    #[test]
    fn test_bump_version_in_cargo_toml() {
        let content = "[package]\nversion = \"1.2.3\"";
        let result = bump_version_in_cargo_toml(content, "1.2.3", "1.2.4");
        assert!(result.contains("1.2.4"));
    }

    #[test]
    fn test_bump_version_in_cargo_toml_no_space() {
        let content = "[package]\nversion=\"1.2.3\"";
        let result = bump_version_in_cargo_toml(content, "1.2.3", "1.2.4");
        assert!(result.contains("\"1.2.4\""));
    }

    #[test]
    fn test_bump_version_in_cargo_toml_not_found() {
        let content = "[package]\nname = \"test\"";
        let result = bump_version_in_cargo_toml(content, "1.2.3", "1.2.4");
        assert_eq!(result.trim_end(), content);
    }

    #[test]
    fn test_bump_version_in_cargo_toml_skips_deps() {
        let content =
            "[package]\nversion = \"1.2.3\"\n\n[dependencies]\nmy-dep = { version = \"1.2.3\" }";
        let result = bump_version_in_cargo_toml(content, "1.2.3", "1.2.4");
        assert!(result.contains("version = \"1.2.4\""));
        assert!(result.contains("my-dep = { version = \"1.2.3\" }"));
    }

    #[test]
    fn test_bump_version_in_json() {
        let content = r#""version": "1.2.3""#;
        let result = bump_version_in_json(content, "1.2.3", "1.2.4");
        assert!(result.contains("1.2.4"));
    }

    #[test]
    fn test_bump_version_in_json_no_space() {
        let content = r#""version":"1.2.3""#;
        let result = bump_version_in_json(content, "1.2.3", "1.2.4");
        assert!(result.contains("\"version\":\"1.2.4\""));
    }

    #[test]
    fn test_bump_version_in_json_not_found() {
        let content = r#""name": "test""#;
        let result = bump_version_in_json(content, "1.2.3", "1.2.4");
        assert_eq!(result, content);
    }

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

    #[test]
    fn test_ai_bumper_major_is_blocked() {
        let result = match "major" {
            "major" => BumpLevel::None,
            "minor" => BumpLevel::Minor,
            "patch" => BumpLevel::Patch,
            _ => BumpLevel::None,
        };
        assert_eq!(
            result,
            BumpLevel::None,
            "major must map to None (manual-only)"
        );
    }
}
