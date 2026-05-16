use crate::policy::{RepoPolicyOverride, SyncPolicy};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub(crate) fn ensure_standard_files(
    repo: &Path,
    policy: &SyncPolicy,
    repo_override: &RepoPolicyOverride,
    policy_base_dir: Option<&Path>,
) -> Result<Vec<PathBuf>> {
    if policy.standard_files.is_empty() {
        return Ok(vec![]);
    }

    eprintln!("DEBUG ensure_standard_files called for {}", repo.display());

    let sync_base = policy_base_dir
        .map(|p| p.to_path_buf())
        .or_else(|| dirs::home_dir().map(|h| h.join(".dracon/utilities/sync")));

    let Some(base) = sync_base else {
        anyhow::bail!("cannot resolve standard files base dir: no policy path and no home dir");
    };

    eprintln!("DEBUG sync_base = {}", base.display());

    let mut copied = Vec::new();

    for cfg in &policy.standard_files {
        if repo_override.skip_standard_files.contains(&cfg.target) {
            eprintln!("DEBUG skipping {} (in skip list)", cfg.target);
            continue;
        }

        let target_path = repo.join(&cfg.target);

        eprintln!("DEBUG checking target {} exists={}", target_path.display(), target_path.exists());

        if target_path.exists() && !cfg.overwrite {
            eprintln!("DEBUG skipping {} (exists, overwrite=false)", cfg.target);
            continue;
        }

        let source_path = cfg.source_path(&base);
        eprintln!("DEBUG source for {} = {}", cfg.target, source_path.display());

        if !source_path.exists() {
            eprintln!(
                "⚠️ standard file template missing: {} (tried {})",
                cfg.target,
                source_path.display()
            );
            continue;
        }

        if target_path.exists() && cfg.overwrite {
            std::fs::remove_file(&target_path)
                .with_context(|| format!("failed to remove existing {}", cfg.target))?;
        }

        std::fs::copy(&source_path, &target_path).with_context(|| {
            format!(
                "failed to copy {} to {}",
                source_path.display(),
                target_path.display()
            )
        })?;

        eprintln!("DEBUG copied {} -> {}", source_path.display(), target_path.display());
        copied.push(target_path);
    }

    eprintln!("DEBUG ensure_standard_files returning {} files", copied.len());
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::StandardFileConfig;
    use tempfile::TempDir;

    fn make_policy(standard_files: Vec<StandardFileConfig>) -> SyncPolicy {
        SyncPolicy {
            standard_files,
            ..Default::default()
        }
    }

    fn make_override(skip: Vec<String>) -> RepoPolicyOverride {
        RepoPolicyOverride {
            skip_standard_files: skip,
            ..Default::default()
        }
    }

    #[test]
    fn test_copies_file_when_missing() {
        let dir = TempDir::new().unwrap();
        let repo_dir = dir.path();
        let template_dir = dir.path().join("templates");
        std::fs::create_dir(&template_dir).unwrap();
        std::fs::write(template_dir.join("LICENSE"), "AGPL").unwrap();
        let sync_path = dir.path().join("sync.toml");
        let sync_dir = sync_path.parent().unwrap();

        let policy = make_policy(vec![StandardFileConfig {
            source: "templates/LICENSE".to_string(),
            target: "LICENSE".to_string(),
            overwrite: false,
        }]);

        let repo_override = make_override(vec![]);
        let result = ensure_standard_files(repo_dir, &policy, &repo_override, Some(sync_dir));
        assert!(result.is_ok());
        let copied = result.unwrap();
        assert_eq!(copied.len(), 1);
        assert_eq!(
            std::fs::read_to_string(repo_dir.join("LICENSE")).unwrap(),
            "AGPL"
        );
    }

    #[test]
    fn test_skips_when_target_exists() {
        let dir = TempDir::new().unwrap();
        let repo_dir = dir.path();
        let template_dir = dir.path().join("templates");
        std::fs::create_dir(&template_dir).unwrap();
        std::fs::write(template_dir.join("LICENSE"), "AGPL").unwrap();
        std::fs::write(repo_dir.join("LICENSE"), "EXISTING").unwrap();
        let sync_path = dir.path().join("sync.toml");
        let sync_dir = sync_path.parent().unwrap();

        let policy = make_policy(vec![StandardFileConfig {
            source: "templates/LICENSE".to_string(),
            target: "LICENSE".to_string(),
            overwrite: false,
        }]);

        let repo_override = make_override(vec![]);
        let result = ensure_standard_files(repo_dir, &policy, &repo_override, Some(sync_dir));
        assert!(result.is_ok());
        let copied = result.unwrap();
        assert!(copied.is_empty());
        assert_eq!(
            std::fs::read_to_string(repo_dir.join("LICENSE")).unwrap(),
            "EXISTING"
        );
    }

    #[test]
    fn test_overwrites_when_configured() {
        let dir = TempDir::new().unwrap();
        let repo_dir = dir.path();
        let template_dir = dir.path().join("templates");
        std::fs::create_dir(&template_dir).unwrap();
        std::fs::write(template_dir.join("LICENSE"), "NEW_AGPL").unwrap();
        std::fs::write(repo_dir.join("LICENSE"), "OLD_LICENSE").unwrap();
        let sync_path = dir.path().join("sync.toml");
        let sync_dir = sync_path.parent().unwrap();

        let policy = make_policy(vec![StandardFileConfig {
            source: "templates/LICENSE".to_string(),
            target: "LICENSE".to_string(),
            overwrite: true,
        }]);

        let repo_override = make_override(vec![]);
        let result = ensure_standard_files(repo_dir, &policy, &repo_override, Some(sync_dir));
        assert!(result.is_ok());
        let copied = result.unwrap();
        assert_eq!(copied.len(), 1);
        assert_eq!(
            std::fs::read_to_string(repo_dir.join("LICENSE")).unwrap(),
            "NEW_AGPL"
        );
    }

    #[test]
    fn test_skips_from_repo_override() {
        let dir = TempDir::new().unwrap();
        let repo_dir = dir.path();
        let template_dir = dir.path().join("templates");
        std::fs::create_dir(&template_dir).unwrap();
        std::fs::write(template_dir.join("CLA.md"), "CLA content").unwrap();

        let policy = make_policy(vec![StandardFileConfig {
            source: "templates/CLA.md".to_string(),
            target: "CLA.md".to_string(),
            overwrite: false,
        }]);

        let repo_override = make_override(vec!["CLA.md".to_string()]);
        let result = ensure_standard_files(repo_dir, &policy, &repo_override, None);
        assert!(result.is_ok());
        let copied = result.unwrap();
        assert!(copied.is_empty());
        assert!(!repo_dir.join("CLA.md").exists());
    }

    #[test]
    fn test_warns_when_template_missing() {
        let dir = TempDir::new().unwrap();
        let repo_dir = dir.path();
        std::fs::write(repo_dir.join("LICENSE"), "EXISTING").unwrap();

        let policy = make_policy(vec![StandardFileConfig {
            source: "templates/NONEXISTENT".to_string(),
            target: "NONEXISTENT.txt".to_string(),
            overwrite: false,
        }]);

        let repo_override = make_override(vec![]);
        let result = ensure_standard_files(repo_dir, &policy, &repo_override, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_short_form_expansion() {
        let dir = TempDir::new().unwrap();
        let repo_dir = dir.path();
        let template_dir = dir.path().join("templates");
        std::fs::create_dir(&template_dir).unwrap();
        std::fs::write(template_dir.join("LICENSE"), "AGPLv3").unwrap();
        let sync_path = dir.path().join("sync.toml");
        let sync_dir = sync_path.parent().unwrap();

        let policy = make_policy(vec![StandardFileConfig {
            source: "templates/LICENSE".to_string(),
            target: "LICENSE".to_string(),
            overwrite: false,
        }]);

        let repo_override = make_override(vec![]);
        let result = ensure_standard_files(repo_dir, &policy, &repo_override, Some(sync_dir));
        assert!(result.is_ok());
        let copied = result.unwrap();
        assert_eq!(copied.len(), 1);
        assert_eq!(
            std::fs::read_to_string(repo_dir.join("LICENSE")).unwrap(),
            "AGPLv3"
        );
    }
}
