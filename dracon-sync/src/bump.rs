use anyhow::{Context, Result};
use std::path::Path;

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
];

const VERSION_FILES: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "VERSION",
    "Cargo.lock",
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


#[cfg(feature = "ai-bumper")]
pub async fn ai_decide_bump_level(
    _repo: &Path,
    current_version: &str,
    staged_diff: &str,
    project_state: &str,
) -> BumpLevel {
    use crate::simple_ai::{ChatMessage, SimpleAiService};

    let version_only_patterns = ["Cargo.toml", "package.json", "VERSION", "Cargo.lock"];
    let has_source_changes = staged_diff.lines()
        .filter(|line| !line.is_empty())
        .any(|line| {
            !version_only_patterns.iter().any(|p| line.contains(p))
        });

    if !has_source_changes {
        return BumpLevel::None;
    }

    let prompt = format!(r##"You are a version bump advisor. Analyze the changes and decide if a version bump is warranted.

Current Version: {current_version}

Project State:
{project_state}

Staged Changes:
{staged_diff}

Respond with ONLY ONE WORD:
- "major": BREAKING CHANGE
- "minor": NEW FEATURE  
- "patch": BUG FIX / improvement
- "none": NOISY/CHORE (docs, deps, config only)

Respond with ONLY ONE WORD."##);

    let service = SimpleAiService::new();
    if service.is_empty() {
        return BumpLevel::None;
    }

    let messages = vec![ChatMessage::user(&prompt)];

    match service.chat(messages).await {
        Ok(content) => {
            match content.trim().to_lowercase().as_str() {
                "major" => BumpLevel::Major,
                "minor" => BumpLevel::Minor,
                "patch" => BumpLevel::Patch,
                _ => BumpLevel::None,
            }
        }
        Err(_) => BumpLevel::None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_semver_patch() {
        assert_eq!(bump_semver_patch("1.2.3"), Some("1.2.4".to_string()));
        assert_eq!(bump_semver_patch("0.0.0"), Some("0.0.1".to_string()));
        assert_eq!(bump_semver_patch("v1.2.3"), None);
        assert_eq!(bump_semver_patch("1.2"), None);
    }

    #[test]
    fn test_bump_semver_minor() {
        assert_eq!(bump_semver_minor("1.2.3"), Some("1.3.0".to_string()));
        assert_eq!(bump_semver_minor("0.0.0"), Some("0.1.0".to_string()));
    }

    #[test]
    fn test_bump_semver_major() {
        assert_eq!(bump_semver_major("1.2.3"), Some("2.0.0".to_string()));
        assert_eq!(bump_semver_major("0.9.9"), Some("1.0.0".to_string()));
    }
}
