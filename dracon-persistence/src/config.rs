use anyhow::{Context, Result};
use std::{fs, path::PathBuf, collections::HashMap};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[derive(PartialEq)]
pub enum BackupPolicy {
    #[default]
    /// BUNDLE: Create compressed git bundle files in the backup directory before sync.
    Bundle,
    /// NONE: Disable automated work preservation.
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistencePolicy {
    /// THE SOVEREIGN CORE: Primary repository for machine state and laws.
    pub system_repo: PathBuf,
    /// THE WATCH: Roots recursively patrolled for .git repositories.
    pub watch_roots: Vec<PathBuf>,
    /// THE SYMMETRY MAP: Mapping of "~/.home/link" = "relative/repo/state".
    #[serde(default)]
    pub symmetry: HashMap<String, String>,
    /// THE NETWORK: Mapping of 'name = "url_or_path"' for extra remotes.
    #[serde(default)]
    pub extra_remotes: HashMap<String, String>,

    /// THE RHYTHM: Seconds between automatic synchronization checks.
    pub pulse_interval_secs: u64,
    /// THE VAULT: Work preservation strategy before risky sync operations.
    pub backup_policy: BackupPolicy,
    pub backup_dir: PathBuf,

    /// PERSISTENCE TOGGLES
    pub auto_commit: bool,
    pub auto_push: bool,
    pub auto_pull: bool,
}

impl Default for PersistencePolicy {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap();
        Self {
            system_repo: home.join("dracon"),
            watch_roots: vec![home.join("Dev"), home.join("dracon")],
            symmetry: HashMap::new(),
            extra_remotes: HashMap::new(),
            pulse_interval_secs: 300,
            backup_policy: BackupPolicy::Bundle,
            backup_dir: home.join("dracon/backups"),
            auto_commit: true,
            auto_push: true,
            auto_pull: true,
        }
    }
}

impl PersistencePolicy {
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() { return Ok(Self::default()); }
        let s = fs::read_to_string(path)?;
        toml::from_str(&s).context("Failed to parse technical persistence policy")
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::dir()?; fs::create_dir_all(&dir)?;
        let mut s = String::new();
        s.push_str("# =============================================================================\n");
        s.push_str("# 🦾  DRACON PERSISTENCE & SYMMETRY POLICY\n");
        s.push_str("# =============================================================================\n");
        s.push_str("# This file defines how your work is preserved across time (Git) and space (Links).\n\n");

        s.push_str("### 🏛️  SECTION 1: THE SOVEREIGN CORE\n");
        s.push_str("# [system_repo] - The primary repository for machine state and laws.\n\n");

        s.push_str("### 🔗 SECTION 2: THE SYMMETRY MAP\n");
        s.push_str("# Mapping of \"~/.link/path\" = \"relative/repo/path\".\n");
        s.push_str("# The utility restores these links and ingests any direct changes (Drift).\n\n");

        s.push_str("### 📡 SECTION 3: THE RHYTHM\n");
        s.push_str("# Frequency of the persistence loop and automated Git toggles.\n\n");

        s.push_str(&toml::to_string_pretty(self)?);
        fs::write(Self::path()?, s)?; Ok(())
    }

    pub fn dir() -> Result<PathBuf> { let home = dirs::home_dir().unwrap(); Ok(home.join("dracon/git")) }
    pub fn path() -> Result<PathBuf> { Ok(Self::dir()?.join("dracon-persistence.toml")) }
}
