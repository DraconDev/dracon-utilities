#![warn(missing_docs)]
#![allow(missing_docs)] // Internal security module — docs deferred

//! Age-based encryption, secret scanning, and filter pipeline for dracon-warden.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use age::x25519;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use cfb_mode::cipher::{AsyncStreamCipher, KeyIvInit};

use once_cell::sync::OnceCell;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;


pub mod modules;

pub use modules::scanner::SecretScanner;
pub use modules::scanner::SecretFinding;
pub use modules::environment::EnvironmentManager;
pub use modules::keys::RepoKey;
pub use modules::keys::TeamKey;

// V2 Encryption Constants
const HEADER_V2_MAGIC: &[u8] = b"age-encryption.org/v1";
const DEFAULT_SECRET_MARKER: &str = "DRACON_SECRET";

static DEFAULT_SECURITY_CACHE: OnceCell<DemonSecurity> = OnceCell::new();

static ALLOW_V1_FALLBACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_allow_v1_fallback(allow: bool) {
    ALLOW_V1_FALLBACK.store(allow, std::sync::atomic::Ordering::Relaxed);
}

pub fn is_v1_fallback_allowed() -> bool {
    ALLOW_V1_FALLBACK.load(std::sync::atomic::Ordering::Relaxed)
}

const ENV_VERSION_HEADER_TEMPLATE: &str = r#"# =============================================================================
# Dracon Warden Encrypted Environment File
# This file is encrypted by dracon-warden for secure team collaboration.
# Version: {}
# DO NOT EDIT THE ENCRYPTED CONTENT MANUALLY - Use `dracon-warden smudge` to decrypt.
# =============================================================================
"#;

fn get_env_version(content: &str) -> u32 {
    if let Some(pos) = content.find("Version: ") {
        let after = &content[pos + 9..];
        if let Some(end) = after.find('\n').or_else(|| after.find('\r')) {
            if let Ok(v) = after[..end].trim().parse::<u32>() {
                return v;
            }
        }
    }
    0
}

fn make_env_version_header(content: &str) -> String {
    let current_version = get_env_version(content);
    let next_version = if current_version == 0 {
        1
    } else {
        current_version + 1
    };
    ENV_VERSION_HEADER_TEMPLATE.replace("{}", &next_version.to_string())
}

fn strip_env_version_header(content: &str) -> &str {
    let header_marker = "Dracon Warden Encrypted Environment File";
    if let Some(start_pos) = content.find(header_marker) {
        let after_header = &content[start_pos..];
        let closing_marker =
            "# =============================================================================";
        if let Some(closing_pos) = after_header.find(closing_marker) {
            let after_closing = &after_header[closing_pos + closing_marker.len()..];
            return after_closing
                .trim_start_matches('\n')
                .trim_start_matches('\r');
        }
    }
    content
}

pub const REPO_KEY_LEN: usize = 32;

fn normalize_secret_marker(raw: &str) -> Option<String> {
    let marker = raw.trim().to_ascii_uppercase();
    let valid = !marker.is_empty()
        && marker
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        && marker.ends_with("_SECRET");
    if valid {
        Some(marker)
    } else {
        None
    }
}

fn is_inside_secret_tag(content: &str, start_idx: usize) -> bool {
    let prefix = &content[..start_idx];
    if let Some(tag_start) = prefix.rfind('[') {
        if prefix[tag_start..].contains(']') {
            return false;
        }
        let window = &content[tag_start..start_idx];
        return window.contains("_SECRET:");
    }
    false
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegistryCredential {
    pub registry: String, // e.g. "ghcr.io"
    pub username: String,
    #[serde(skip)]
    pub password: String, // Token or Password
}

impl RegistryCredential {
    pub fn new(registry: &str, username: &str, password: &str) -> Self {
        Self {
            registry: registry.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MarkerMigrationStats {
    pub files_scanned: usize,
    pub files_changed: usize,
    pub markers_changed: usize,
    pub walk_errors: usize,
}

#[derive(Clone)]
pub struct DemonSecurity {
    master_identities: Vec<x25519::Identity>,
    imported_identities: Vec<x25519::Identity>,
    managed_patterns: Vec<String>,
    secret_marker: String,
    repo_root: Option<PathBuf>,
    mock_home: Option<PathBuf>,
    pub dev_mode: bool,
}

impl DemonSecurity {
    pub fn master_identities(&self) -> &[x25519::Identity] {
        &self.master_identities
    }

    pub fn with_managed_patterns(mut self, patterns: Vec<String>) -> Self {
        self.managed_patterns = patterns;
        self
    }

    pub fn with_secret_marker(mut self, marker: &str) -> Self {
        if let Some(normalized) = normalize_secret_marker(marker) {
            self.secret_marker = normalized;
        }
        self
    }

    pub fn secret_marker(&self) -> &str {
        &self.secret_marker
    }

    pub fn get_or_init() -> Result<&'static DemonSecurity> {
        DEFAULT_SECURITY_CACHE.get_or_try_init(|| {
            let mut security = DemonSecurity::new(None)?;
            if let Ok(ids) = security.load_master_identities() {
                security.master_identities = ids;
            }
            if let Ok(imported) = security.load_imported_identities() {
                if !imported.is_empty() {
                    security.imported_identities = imported;
                }
            }
            Ok(security)
        })
    }

    fn supported_secret_markers(&self) -> Vec<String> {
        vec![self.secret_marker.clone()]
    }

    fn secret_tag_prefixes(&self) -> Vec<String> {
        self.supported_secret_markers()
            .into_iter()
            .map(|m| format!("[{}:", m))
            .collect()
    }

    fn contains_any_secret_tag(&self, content: &str) -> bool {
        self.secret_tag_prefixes()
            .iter()
            .any(|prefix| content.contains(prefix))
    }

    fn count_secret_tags(&self, content: &str) -> usize {
        self.secret_tag_prefixes()
            .iter()
            .map(|prefix| content.matches(prefix).count())
            .sum()
    }

    fn starts_with_any_secret_tag(&self, content: &[u8]) -> bool {
        let trimmed = content.strip_suffix(b"\n").unwrap_or(content);
        self.secret_tag_prefixes()
            .iter()
            .any(|prefix| trimmed.starts_with(prefix.as_bytes()) && trimmed.ends_with(b"]"))
    }

    pub fn get_identity_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        Ok(home.join(".demon").join("identity.age"))
    }

    pub fn new(repo_path: Option<&Path>) -> Result<Self> {
        let mut security = Self {
            master_identities: Vec::new(),
            imported_identities: Vec::new(),
            managed_patterns: Vec::new(),
            secret_marker: std::env::var("DRACON_SECRET_MARKER")
                .ok()
                .and_then(|m| normalize_secret_marker(&m))
                .unwrap_or_else(|| DEFAULT_SECRET_MARKER.to_string()),
            repo_root: repo_path.map(|p| p.to_path_buf()),
            mock_home: None,
            dev_mode: false,
        };

        match security.load_master_identities() {
            Ok(ids) => {
                security.master_identities = ids;
            }
            Err(e) => {
                eprintln!("⚠️ Failed to load master identities: {}", e);
            }
        };

        // Load imported legacy keys
        if let Ok(imported) = security.load_imported_identities() {
            if !imported.is_empty() {
                security.imported_identities = imported;
            }
        }

        Ok(security)
    }

    /// Load generic identities from ~/demon/keys/*.age (e.g. Git Seal keys)
    fn load_imported_identities(&self) -> Result<Vec<x25519::Identity>> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let keys_dir = home.join(".demon").join("keys");
        let mut identities = Vec::new();

        if !keys_dir.exists() {
            return Ok(identities);
        }

        for entry in std::fs::read_dir(keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                // Try reading as string (Bech32 Identity)
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(key_str) = content
                        .lines()
                        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
                    {
                        if let Ok(id) = std::str::FromStr::from_str(key_str.trim()) {
                            identities.push(id);
                            continue;
                        }
                    }
                }

                // Legacy Git-Seal/Raw Key loading removed in Phase 48 (Clean Slate)
            }
        }
        Ok(identities)
    }

    /// Load ALL Master Identities from all known paths (Multi-Key Ring)
    /// Returns a vector of identities, prioritized by path.
    pub fn load_master_identities(&self) -> Result<Vec<x25519::Identity>> {
        let home = dirs::home_dir().context("cannot determine home directory")?;
        let mut identities = Vec::new();

        // Path Priority List
        let candidate_paths = vec![
            // 1. Sovereign Master (The active key)
            home.join(".demon").join("master.age"),
            // 2. Standard Identity
            home.join(".demon").join("identity.age"),
            // 3. Fallback/Backups
            home.join(".demon").join("keys").join("identity.age"),
        ];

        // 6. GENERAL SCAN: ~/demon/keys/*.age (and similar dirs)
        // This satisfies "if a user adds their key whatever it is called we can try it"
        let general_keys = vec![
            home.join(".demon").join("keys"), // key storage
        ];

        for dir in general_keys {
            if dir.exists() {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().and_then(|s| s.to_str()) == Some("age") {
                            // Avoid duplicates if we already added it explicitly
                            if !candidate_paths.contains(&p) {
                                // Add to check list (or just parse here)
                                // Let's just parse here to avoid modifying 'paths' which is consumed.
                                if let Ok(c) = fs::read_to_string(&p) {
                                    // Quick parse
                                    if let Some(k) = c
                                        .lines()
                                        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
                                    {
                                        if let Ok(id) = std::str::FromStr::from_str(k.trim()) {
                                            identities.push(id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for path in candidate_paths {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(key_str) = content
                        .lines()
                        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
                    {
                        if let Ok(id) = std::str::FromStr::from_str(key_str.trim()) {
                            identities.push(id);
                        }
                    }
                }
            }
        }

        if identities.is_empty() {
            return Err(anyhow::anyhow!(
                "No valid identities found in any search path."
            ));
        }

        Ok(identities)
    }

    pub fn has_master_identity(&self) -> bool {
        !self.master_identities.is_empty()
    }

    /// Add an identity explicitly from memory (useful for tests or ephemeral agents)
    pub fn add_memory_identity(&mut self, key: x25519::Identity) {
        self.master_identities.push(key);
    }

    /// Set a custom backup root directory (useful for tests)
    pub fn set_mock_home(&mut self, path: PathBuf) {
        self.mock_home = Some(path);
    }

    fn get_home(&self) -> Result<PathBuf> {
        if let Some(ref h) = self.mock_home {
            Ok(h.clone())
        } else {
            dirs::home_dir().context("Could not find home directory")
        }
    }

    /// Explicitly generate and save a new Master Identity
    /// CRITICAL: This should only ever be called ONCE per user.
    /// If an identity already exists, this will refuse to overwrite it.
    pub fn generate_master_identity(&mut self) -> Result<()> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let identity_path = home.join(".demon").join("identity.age");
        // Protection: Check legacy path too
        let legacy_path = home.join(".demon").join("identity.txt");

        // PROTECTION: Scan for ANY existing identity files (backups, corrupted, legacy)
        // We refuse to init if there is ANY trace of an identity to prevent data loss.
        if let Some(parent) = identity_path.parent() {
            if parent.exists() {
                for entry in fs::read_dir(parent)? {
                    let entry = entry?;
                    let path = entry.path();
                    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                    if file_name.starts_with("identity") {
                        return Err(anyhow::anyhow!(
                            "🛡️ SAFETY TRIGGERED: Found existing identity artifact '{:?}'.\n\n\
                             CRITICAL: Demon refuses to overwrite, modify, or delete Master Identity files.\n\
                             This ensures you can NEVER be locked out of your secrets by an automated process.\n\n\
                             To generate a NEW identity, you must MANUALLY move or delete all 'identity*' files in {:?}.",
                            file_name,
                            parent
                        ));
                    }
                }
            }
        }

        if legacy_path.exists() {
            return Err(anyhow::anyhow!(
                "🛡️ SAFETY TRIGGERED: Legacy identity found at {:?}. Please remove explicitly.",
                legacy_path
            ));
        }

        // Generate new identity
        let key = x25519::Identity::generate();
        if let Some(parent) = identity_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Save Private Identity
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o400)
            .open(&identity_path)?;
        let mut writer = file;
        #[cfg(not(unix))]
        {
            let mut perms = writer.metadata()?.permissions();
            perms.set_mode(0o400);
            if let Err(e) = fs::set_permissions(&identity_path, perms) {
                eprintln!(
                    "⚠️ failed to set permissions on {}: {}",
                    identity_path.display(),
                    e
                );
            }
        }
        writeln!(writer, "{}", key.to_string().expose_secret())?;

        // Save Public Key for sharing
        let pub_path = home.join(".demon").join("identity.pub");
        fs::write(&pub_path, key.to_public().to_string())?;

        // Auto-Backup Master Identity
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_dir = home.join(".demon").join("backups");
        if let Err(e) = fs::create_dir_all(&backup_dir) {
            eprintln!(
                "⚠️ failed to create backup dir {}: {}",
                backup_dir.display(),
                e
            );
        }
        let backup_path = backup_dir.join(format!("master_{}.age", timestamp));
        if let Ok(mut b_file) = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o400)
            .open(&backup_path)
        {
            if let Err(e) = writeln!(b_file, "{}", key.to_string().expose_secret()) {
                eprintln!("⚠️ failed to write backup {}: {}", backup_path.display(), e);
            }
        }

        self.master_identities = vec![key];
        Ok(())
    }

    /// Helper to get the repo root, either from configured path or CWD
    fn get_repo_root(&self) -> Result<PathBuf> {
        if let Some(root) = &self.repo_root {
            return Ok(root.clone());
        }
        Self::find_repo_root()
    }

    /// Find the git repository root from the current directory
    pub fn find_repo_root() -> Result<PathBuf> {
        let mut current = std::env::current_dir()?;
        loop {
            if current.join(".git").exists() {
                return Ok(current);
            }
            if !current.pop() {
                return Err(anyhow::anyhow!("Not in a git repository"));
            }
        }
    }

    /// Load the repo key from .git/arcane/keys/*.age or history
    pub fn load_repo_key(&self) -> Result<RepoKey> {
        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");

        if !keys_dir.exists() {
            return Err(anyhow::anyhow!("No keys found. Run 'demon init'."));
        }

        // 0. Try Machine Key (Env Var) - Priority for CI/CD
        if let Ok(machine_key_str) = std::env::var("ARCANE_MACHINE_KEY") {
            // Derive identity from the env var string
            use std::str::FromStr;
            if let Ok(machine_identity) = x25519::Identity::from_str(&machine_key_str) {
                if let Ok(key) = self.try_decrypt_directory_machine(&keys_dir, &machine_identity) {
                    return Ok(key);
                }
            }
        }

        // 1. Try ALL Master Identities (Key Ring)
        for identity in &self.master_identities {
            // 1a. Try direct User access (keys/*.age)
            if let Ok(key) = self.try_decrypt_directory(&keys_dir, identity) {
                return Ok(key);
            }

            // 1b. Try history keys (latest to oldest)
            let history_dir = keys_dir.join("history");
            if history_dir.exists() {
                if let Ok(entries) = fs::read_dir(&history_dir) {
                    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                    entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
                    for entry in entries {
                        if entry.path().is_dir() {
                            if let Ok(key) = self.try_decrypt_directory(&entry.path(), identity) {
                                return Ok(key);
                            }
                        }
                    }
                }
            }
        }

        // 1c. Try Imported Identities (Heritage Keys / Git Seal)
        for imported_id in &self.imported_identities {
            if let Ok(key) = self.try_decrypt_directory(&keys_dir, imported_id) {
                return Ok(key);
            }
        }

        // 2. Try Team access (keys/team:*.age)
        for entry in fs::read_dir(&keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                if filename.starts_with("team:") && filename.ends_with(".age") {
                    let team_name = filename
                        .trim_start_matches("team:")
                        .trim_end_matches(".age");

                    if let Ok(team_key) = self.load_team_key(team_name) {
                        if let Ok(repo_key) = self.decrypt_repo_key_with_team_key(&path, &team_key)
                        {
                            return Ok(repo_key);
                        }
                    }
                }
            }
        }

        // 4. Last Resort: Legacy repo.key (Removed Phase 48)

        Err(anyhow::anyhow!(
            "Access Denied. Missing valid Key (User, Team, or Machine)."
        ))
    }

    /// Authorize a new recipient (Machine or User) to access this repository
    pub fn authorize_recipient(&self, recipient: &age::x25519::Recipient) -> Result<()> {
        let repo_key = self.load_repo_key()?;
        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");
        std::fs::create_dir_all(&keys_dir)?;

        let output_path = keys_dir.join(format!("{}.age", recipient));

        // Encrypt the repo key for the recipient
        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient.clone())];
        let encryptor =
            age::Encryptor::with_recipients(recipients).context("failed to create encryptor")?;

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(&repo_key.0)?;
        writer.finish()?;

        std::fs::write(&output_path, &encrypted)?;
        Ok(())
    }

    // specialized helper for machine key scanning
    fn try_decrypt_directory_machine(
        &self,
        dir: &Path,
        identity: &x25519::Identity,
    ) -> Result<RepoKey> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                if filename.starts_with("machine:") {
                    if let Ok(repo_key) = self.try_decrypt_key_file(&path, identity) {
                        return Ok(repo_key);
                    }
                }
            }
        }
        Err(anyhow::anyhow!("No matching machine key found"))
    }

    /// Generate a new Machine Identity (Private Key, Public Key)
    pub fn generate_machine_identity() -> (String, String) {
        let identity = x25519::Identity::generate();
        let pub_key = identity.to_public().to_string();
        let identity_str = identity.to_string();
        let priv_key = identity_str.expose_secret();
        (priv_key.to_string(), pub_key)
    }

    /// Authorize a Machine (Public Key) to access this repo
    pub fn whitelist_machine(&self, public_key_str: &str) -> Result<()> {
        let recipient: x25519::Recipient = public_key_str
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid machine public key: {}", e))?;

        let repo_key = self
            .load_repo_key()
            .context("Must have access to repo to whitelist machines")?;

        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");

        // Use hash or similar ID for filename
        let safe_name = public_key_str
            .replace(":", "_")
            .chars()
            .take(12)
            .collect::<String>();
        let machine_file = keys_dir.join(format!("machine:{}.age", safe_name));
        let pub_file = keys_dir.join(format!("machine:{}.pub", safe_name));

        // Save public key (for V2 filtering)
        fs::write(&pub_file, public_key_str)?;

        self.encrypt_and_save_key(&repo_key, &recipient, &machine_file)?;

        Ok(())
    }

    /// Load a Team Key from ~/demon/teams/<name>.key
    pub fn load_team_key(&self, team_name: &str) -> Result<TeamKey> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let team_key_path = home
            .join(".demon")
            .join("teams")
            .join(format!("{}.key", team_name));

        if !team_key_path.exists() {
            return Err(anyhow::anyhow!(
                "Team key '{}' not found in keychain",
                team_name
            ));
        }

        // Team keys are encrypted with Master Identity
        // Team keys are encrypted with Master Identity. Use PRIMARY (first in ring).
        let identity = self
            .master_identities
            .first()
            .context("Master identity required to unlock team keys")?;

        // Decrypt the file
        let encrypted_bytes = fs::read(&team_key_path)?;
        // Fix: Wrap input in Cursor for Decryptor
        let decryptor = age::Decryptor::new(std::io::Cursor::new(&encrypted_bytes))?;

        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        };

        let mut key_bytes = Vec::new();
        use std::io::Read;
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid team key length"));
        }

        Ok(TeamKey(key_bytes))
    }

    fn decrypt_repo_key_with_team_key(&self, path: &Path, team_key: &TeamKey) -> Result<RepoKey> {
        let encrypted_bytes = fs::read(path)?;

        let team_identity_bytes = &team_key.0;
        let team_identity_str = String::from_utf8(team_identity_bytes.clone())
            .map_err(|_| anyhow::anyhow!("Invalid team identity bytes"))?;
        let team_identity = x25519::Identity::from_str(&team_identity_str)
            .map_err(|e| anyhow::anyhow!("Invalid team identity format: {}", e))?;

        // Fix: Wrap input in Cursor for Decryptor
        let decryptor = age::Decryptor::new(std::io::Cursor::new(&encrypted_bytes))?;
        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(&team_identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        };

        let mut key_bytes = Vec::new();
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != REPO_KEY_LEN {
            return Err(anyhow::anyhow!("Invalid decrypted key length"));
        }

        Ok(RepoKey(key_bytes))
    }

    /// Create a new Team (Generates a new Identity, saves to keychain)
    pub fn create_team(&self, team_name: &str) -> Result<()> {
        // Validate name
        if team_name.contains('/') || team_name.contains('\\') || team_name.contains(':') {
            return Err(anyhow::anyhow!("Invalid team name"));
        }

        let home = dirs::home_dir().context("Could not find home directory")?;
        let team_dir = home.join(".demon").join("teams");
        fs::create_dir_all(&team_dir)?;

        let team_key_path = team_dir.join(format!("{}.key", team_name));
        if team_key_path.exists() {
            return Err(anyhow::anyhow!(
                "Team '{}' already exists in your keychain",
                team_name
            ));
        }

        // Generate new Identity for the team
        let team_identity = x25519::Identity::generate();
        let team_identity_string = team_identity.to_string(); // Extend lifetime
        let team_secret = team_identity_string.expose_secret();

        // Encrypt this secret with Master Identity for storage
        let master = self
            .master_identities
            .first()
            .context("Master identity required")?;
        let recipient = master.to_public();

        // Encryption logic
        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient.clone())];
        let encryptor =
            age::Encryptor::with_recipients(recipients).context("failed to create encryptor")?;

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(team_secret.as_bytes())?;
        writer.finish()?;

        std::fs::write(&team_key_path, encrypted)?;
        Ok(())
    }

    /// Add a new team member by encrypting the repo key for them
    pub fn add_team_member(&self, alias: &str, public_key_str: &str) -> Result<()> {
        let recipient: x25519::Recipient = public_key_str
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid public key: {}", e))?;

        let repo_key = self
            .load_repo_key()
            .context("Must have access to repo to add members")?;

        // Sanitize alias
        let alias = alias.trim();
        if alias.is_empty() || alias.contains('/') || alias.contains('\\') {
            return Err(anyhow::anyhow!("Invalid alias"));
        }

        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");
        let key_path = keys_dir.join(format!("{}.age", alias));
        let pub_key_path = keys_dir.join(format!("{}.pub", alias));

        if key_path.exists() {
            return Err(anyhow::anyhow!("Member '{}' already exists", alias));
        }

        // Save public key (for V2 filtering)
        fs::write(&pub_key_path, public_key_str)?;

        // Save Age key (encrypted for member)
        self.encrypt_and_save_key(&repo_key, &recipient, &key_path)?;

        Ok(())
    }

    /// Create an Invite for a user to join a Team
    pub fn create_team_invite(&self, team_name: &str, user_public_key: &str) -> Result<PathBuf> {
        let team_key = self.load_team_key(team_name)?;

        let recipient: x25519::Recipient = user_public_key
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid user public key: {}", e))?;

        let repo_root = self.get_repo_root()?;

        // Generate a random invite ID
        let invite_id = uuid::Uuid::new_v4().to_string();

        let invites_dir = repo_root.join(".demon").join("invites").join(team_name);
        fs::create_dir_all(&invites_dir)?;

        let invite_path = invites_dir.join(format!("{}.age", invite_id));

        // Encrypt the TEAM KEY for the USER
        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient)];
        let encryptor =
            age::Encryptor::with_recipients(recipients).context("Failed to create encryptor")?;

        let mut file = fs::File::create(&invite_path)?;
        let mut writer = encryptor.wrap_output(&mut file)?;
        writer.write_all(&team_key.0)?;
        writer.finish()?;

        Ok(invite_path)
    }

    /// Accept a Team Invite
    pub fn accept_team_invite(&self, invite_path: &Path) -> Result<String> {
        let identity = self
            .master_identities
            .first()
            .context("Master identity required to accept invite")?;

        let encrypted_bytes = fs::read(invite_path)?;
        let decryptor = age::Decryptor::new(std::io::Cursor::new(&encrypted_bytes))?;

        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        };

        let mut key_bytes = Vec::new();
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid invite content"));
        }

        // Determine team name from path: demon/invites/<team_name>/...
        let team_name = invite_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine team name from path"))?;

        // Save to key chain
        let home = dirs::home_dir().context("Could not find home directory")?;
        let team_dir = home.join(".demon").join("teams");
        fs::create_dir_all(&team_dir)?;

        let team_key_path = team_dir.join(format!("{}.key", team_name));

        // Encrypt for local storage (Master Identity)
        let master = self
            .master_identities
            .first()
            .context("Master identity required to accept invite")?;
        let recipient = master.to_public();

        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient)];
        let encryptor =
            age::Encryptor::with_recipients(recipients).context("Failed to create encryptor")?;

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(&key_bytes)?;
        writer.finish()?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&team_key_path)?;
            file.write_all(&encrypted)?;
        }
        #[cfg(not(unix))]
        {
            fs::write(&team_key_path, &encrypted)?;
            let metadata = fs::metadata(&team_key_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&team_key_path, perms)?;
        }

        Ok(team_name.to_string())
    }

    /// Revoke a recipient's access to this repo by removing their key files
    pub fn revoke_recipient(&self, public_key_str: &str) -> Result<()> {
        // SAFETY: Refuse to revoke ANY master identity key.
        for id in &self.master_identities {
            if id.to_public().to_string() == public_key_str {
                return Err(anyhow::anyhow!(
                    "🛡️ SAFETY TRIGGERED: Refusing to revoke a Master Identity key.\n\
                     Master identities must be managed MANUALLY via the filesystem to prevent lockout."
                ));
            }
        }

        let repo_root = self.get_repo_root()?;
        let search_paths = vec![
            repo_root.join(".demon").join("data").join("keys"),
            repo_root.join(".git").join("arcane").join("keys"),
        ];

        let mut removed_count = 0;
        for dir in search_paths {
            if dir.exists() {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if let Ok(content) = fs::read_to_string(&path) {
                        if content.contains(public_key_str) {
                            if let Err(e) = fs::remove_file(&path) {
                                eprintln!("⚠️ failed to remove {}: {}", path.display(), e);
                            }

                            let age_path = path.with_extension("age");
                            if age_path.exists() {
                                if let Err(e) = fs::remove_file(&age_path) {
                                    eprintln!("⚠️ failed to remove {}: {}", age_path.display(), e);
                                }
                            }

                            removed_count += 1;
                        }
                    }
                }
            }
        }

        if removed_count > 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("No files found for this recipient"))
        }
    }

    /// List all authorized recipients in the current repository
    pub fn list_authorized_recipients(&self) -> Result<Vec<(String, String)>> {
        let repo_root = self.get_repo_root()?;
        let mut recipients = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let search_paths = vec![
            repo_root.join(".demon").join("data").join("keys"),
            repo_root.join(".git").join("arcane").join("keys"),
        ];

        for dir in search_paths {
            if dir.exists() {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        if ext == "pub" || ext == "key" {
                            if let Ok(content) = fs::read_to_string(&path) {
                                let name = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                for line in content.lines() {
                                    let line = line.trim();
                                    if !line.is_empty()
                                        && !line.starts_with('#')
                                        && seen.insert(line.to_string())
                                    {
                                        recipients.push((name.clone(), line.to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(recipients)
    }

    /// List all team members (aliases)
    pub fn list_team_members(&self) -> Result<Vec<String>> {
        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");

        if !keys_dir.exists() {
            return Ok(Vec::new());
        }

        let mut members = Vec::new();
        for entry in fs::read_dir(keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if !name.starts_with("machine:")
                        && !name.starts_with("team:")
                        && name != "repo"
                        && name != "owner"
                    {
                        members.push(name.to_string());
                    }
                }
            }
        }
        Ok(members)
    }

    fn try_decrypt_directory(&self, dir: &Path, identity: &x25519::Identity) -> Result<RepoKey> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                if let Ok(repo_key) = self.try_decrypt_key_file(&path, identity) {
                    return Ok(repo_key);
                }
            }
        }
        Err(anyhow::anyhow!(
            "No matching key found in directory for this identity"
        ))
    }

    fn try_decrypt_key_file(&self, path: &Path, identity: &x25519::Identity) -> Result<RepoKey> {
        let encrypted_bytes = fs::read(path)?;
        // Fix: Wrap input in Cursor for Decryptor
        let decryptor = age::Decryptor::new(std::io::Cursor::new(&encrypted_bytes))?;

        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        };
        let mut key_bytes = Vec::new();
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != REPO_KEY_LEN {
            return Err(anyhow::anyhow!("Invalid decrypted key length"));
        }
        Ok(RepoKey(key_bytes))
    }

    // Encrypt Repo Key for a Recipient and save to file
    fn encrypt_and_save_key(
        &self,
        repo_key: &RepoKey,
        recipient: &x25519::Recipient,
        output_path: &Path,
    ) -> Result<()> {
        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient.clone())];
        let encryptor =
            age::Encryptor::with_recipients(recipients).context("failed to create encryptor")?;

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(&repo_key.0)?;
        writer.finish()?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(output_path)?
                .write_all(&encrypted)?;
        }
        #[cfg(not(unix))]
        {
            fs::write(output_path, &encrypted)?;
        }
        Ok(())
    }

    fn backup_secret(&self, original_path: &str, content: &[u8]) -> Result<()> {
        if original_path.contains("demon/backups") || original_path.contains("arcane/backups") {
            return Ok(()); // Silent skip for internal Git/Arcane backups
        }
        let repo_root = self.get_repo_root()?;
        let backup_dir = repo_root.join(".git").join("arcane").join("backups");
        fs::create_dir_all(&backup_dir)?;

        let safe_name = original_path.replace("/", "_").replace("\\", "_");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let backup_path = backup_dir.join(format!("{}.{}.bak.age", safe_name, timestamp));

        let identity = self
            .master_identities
            .first()
            .context("Master identity required for secure backup")?;
        let recipient = identity.to_public();

        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient)];
        let encryptor = age::Encryptor::with_recipients(recipients)
            .context("Failed to create encryptor for backup")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o400)
                .open(&backup_path)?;
            let mut writer = encryptor.wrap_output(&mut file)?;
            writer.write_all(content)?;
            writer.finish()?;
        }
        #[cfg(not(unix))]
        {
            let mut file = fs::File::create(&backup_path)?;
            let mut perms = file.metadata()?.permissions();
            perms.set_mode(0o400);
            fs::set_permissions(&backup_path, perms)?;
            let mut writer = encryptor.wrap_output(&mut file)?;
            writer.write_all(content)?;
            writer.finish()?;
        }

        Ok(())
    }

    // ============================================================
    // V2: DIRECT RECIPIENT ENCRYPTION (Standard Age)
    // ============================================================

    /// V2 Encryption: Encrypt directly to a list of recipients (No RepoKey)
    pub fn encrypt_v2(
        &self,
        data: &[u8],
        recipients: Vec<Box<dyn age::Recipient + Send>>,
    ) -> Result<Vec<u8>> {
        let encryptor =
            age::Encryptor::with_recipients(recipients).context("Failed to create encryptor")?;

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(data)?;
        writer.finish()?;

        Ok(encrypted)
    }

    /// V2 Decryption: Decrypt using the User's Identities (Try ALL known keys)
    /// V2 Decryption: Decrypt using the User's Identities (Try ALL known keys)
    pub fn decrypt_v2(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        if self.master_identities.is_empty() {
            return Err(anyhow::anyhow!(
                "Master identity required for V2 decryption"
            ));
        }

        let decryptor = age::Decryptor::new(std::io::Cursor::new(encrypted_data))?;

        match decryptor {
            age::Decryptor::Recipients(d) => {
                // Pass ALL identities to the decryptor at once.
                // Age will try them one by one.
                let identities: Vec<&dyn age::Identity> = self
                    .master_identities
                    .iter()
                    .map(|id| id as &dyn age::Identity)
                    .collect();

                let mut reader = d
                    .decrypt(identities.into_iter())
                    .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

                let mut plaintext = Vec::new();
                reader.read_to_end(&mut plaintext)?;
                Ok(plaintext)
            }
            age::Decryptor::Passphrase(_) => {
                Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        }
    }

    /// Gather all known recipients (Master, Imported, Team, Machine) for encryption.
    pub fn gather_all_recipients(&self) -> Result<Vec<x25519::Recipient>> {
        let master = self
            .master_identities
            .first()
            .context("Master identity required to gathering recipients")?;
        let mut seen_keys = std::collections::HashSet::new();
        let mut recipients = Vec::new();

        // 1. Self (Master)
        let master_pub = master.to_public();
        seen_keys.insert(master_pub.to_string());
        recipients.push(master_pub);

        // 2. Imported Heritage Identities
        for id in &self.imported_identities {
            let pub_key = id.to_public();
            let pub_str = pub_key.to_string();
            if seen_keys.insert(pub_str) {
                recipients.push(pub_key);
            }
        }

        // 3. Authorized Machine & Team Keys from the current repo
        if let Ok(repo_root) = self.get_repo_root() {
            // Check BOTH new committed path (V2 Standard) and legacy path
            let search_paths = vec![
                repo_root.join(".demon").join("data").join("keys"), // V2 Standard
                repo_root.join(".git").join("arcane").join("keys"), // Legacy
            ];

            for keys_dir in search_paths {
                if keys_dir.exists() {
                    if let Ok(entries) = fs::read_dir(keys_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            // Accept .pub files (standard) and .key files (legacy pubkeys sometimes named .key)
                            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                            if ext == "pub" || ext == "key" {
                                if let Ok(pub_str) = fs::read_to_string(&path) {
                                    // Parse potential multiple keys per file or single key
                                    for line in pub_str.lines() {
                                        let line = line.trim();
                                        if !line.is_empty()
                                            && !line.starts_with('#')
                                            && seen_keys.insert(line.to_string())
                                        {
                                            if let Ok(recipient) = line.parse::<x25519::Recipient>()
                                            {
                                                recipients.push(recipient);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(recipients)
    }

    /// Unified payload unlocking logic: try ALL known keys.
    pub fn unlock_payload(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        // 1. Try Age (V2) format
        if encrypted_data.starts_with(HEADER_V2_MAGIC) {
            // Try ALL Master keys
            for id in &self.master_identities {
                if let Ok(plaintext) = self.decrypt_v2_with_identity(encrypted_data, id) {
                    return Ok(plaintext);
                }
            }
            // Try Imported
            for id in &self.imported_identities {
                if let Ok(plaintext) = self.decrypt_v2_with_identity(encrypted_data, id) {
                    return Ok(plaintext);
                }
            }
        }

        // 2. Try RepoKey (V1) format - if we have a RepoKey
        if let Ok(repo_key) = self.load_repo_key() {
            if let Ok(plaintext) = self.decrypt_with_repo_key(&repo_key, encrypted_data) {
                return Ok(plaintext);
            }
            if let Ok(plaintext) = self.decrypt_git_seal(&repo_key, encrypted_data) {
                return Ok(plaintext);
            }
        }

        // 3. Drunk guy with keychain (brute force all keys in ~/demon/keys)
        if let Some(plaintext) = self.try_keychain_bruteforce(encrypted_data) {
            return Ok(plaintext);
        }

        Err(anyhow::anyhow!(
            "Decryption failed after trying all keys (V2 + V1 + Keychain). Magic: {:?}, Len: {}",
            &encrypted_data.get(0..20).unwrap_or(&[]),
            encrypted_data.len()
        ))
    }

    fn decrypt_v2_with_identity(
        &self,
        encrypted_data: &[u8],
        identity: &x25519::Identity,
    ) -> Result<Vec<u8>> {
        let decryptor = age::Decryptor::new(std::io::Cursor::new(encrypted_data))?;
        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase not supported"))
            }
        };
        let mut plaintext = Vec::new();
        reader.read_to_end(&mut plaintext)?;
        Ok(plaintext)
    }

    /// Helper for encrypting data to all known recipients.
    pub fn encrypt_v2_for_all(&self, data: &[u8]) -> Result<Vec<u8>> {
        let recipients = self.gather_all_recipients()?;
        let age_recipients: Vec<Box<dyn age::Recipient + Send>> = recipients
            .into_iter()
            .map(|r| Box::new(r) as Box<dyn age::Recipient + Send>)
            .collect();
        self.encrypt_v2(data, age_recipients)
    }

    /// Encrypt data for a specific node (runner) + master keys
    pub fn encrypt_for_node(&self, data: &[u8], node_recipient: &str) -> Result<Vec<u8>> {
        let mut recipients = Vec::new();

        // 1. Add node recipient
        let node: x25519::Recipient = node_recipient
            .parse::<x25519::Recipient>()
            .map_err(|e| anyhow::anyhow!("Invalid node recipient: {}", e))?;
        recipients.push(node);

        // 2. Add master keys (so Director/User can still recover/debug)
        let master_ids = self.load_master_identities()?;
        for id in master_ids {
            recipients.push(id.to_public());
        }

        let age_recipients: Vec<Box<dyn age::Recipient + Send>> = recipients
            .into_iter()
            .map(|r| Box::new(r) as Box<dyn age::Recipient + Send>)
            .collect();

        self.encrypt_v2(data, age_recipients)
    }

    /// Ensure the current user's public key is present in the repo keys.
    /// This prevents "lockout" by ensuring we can always decrypt what we encrypt.
    pub fn ensure_current_user_key(&self) -> Result<()> {
        let repo_root = match self.get_repo_root() {
            Ok(r) => r,
            Err(_) => return Ok(()), // Not in a repo, skip
        };

        let identity = self
            .master_identities
            .first()
            .context("No master identity found")?;
        let pub_key = identity.to_public();
        let pub_key_str = pub_key.to_string();

        // Use a short hash of the key for the filename to allow multiple owners
        // without conflict or overwriting.
        let safe_id = pub_key_str.chars().take(8).collect::<String>();
        let filename = format!("owner_{}.pub", safe_id);

        let keys_dir = repo_root.join(".demon").join("data").join("keys");
        fs::create_dir_all(&keys_dir)?;

        let key_path = keys_dir.join(&filename);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o644)
                .open(&key_path)?;
            file.write_all(pub_key_str.as_bytes())?;
        }
        #[cfg(not(unix))]
        {
            fs::write(&key_path, &pub_key_str)?;
        }
        Ok(())
    }

    /// Create a secure backup of a file before modification.
    /// The backup is encrypted with all known keys and stored in ~/demon/backups/
    /// Returns the path to the backup file.
    pub fn backup_file(&self, file_path: &Path, content: &[u8]) -> Result<PathBuf> {
        let path_str = file_path.to_string_lossy();
        if path_str.contains("demon/backups") || path_str.contains("arcane/backups") {
            return Err(anyhow::anyhow!(
                "Recursion guard: Skipping backup of backup file"
            ));
        }

        // Auto-ensure our key is in the repo before we do anything that might rely on it later

        let home = self.get_home()?;
        let backup_dir = home.join(".demon").join("backups");
        fs::create_dir_all(&backup_dir)?;

        // Hash the path to create a deterministic but safe filename
        let mut hasher = Sha256::new();
        hasher.update(file_path.to_string_lossy().as_bytes());
        let hash = hasher.finalize();
        let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

        // Timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Filename: <hash>_<timestamp>.age
        let filename = format!("{}_{}.age", hash_hex, timestamp);
        let backup_path = backup_dir.join(filename);

        // Encrypt logic (using encrypt_v2_for_all)
        let encrypted = self.encrypt_v2_for_all(content)?;

        fs::write(&backup_path, encrypted)?;
        Ok(backup_path)
    }

    /// Restore a file from the latest secure backup.
    /// Finds the backup matching the file path hash and decrypts it to the target path.
    pub fn restore_file(&self, file_path: &Path) -> Result<PathBuf> {
        let home = self.get_home()?;
        let backup_dir = home.join(".demon").join("backups");

        if !backup_dir.exists() {
            return Err(anyhow::anyhow!(
                "No backups found for file: {:?}",
                file_path
            ));
        }

        // Hash the path to find matching backups
        let mut hasher = Sha256::new();
        hasher.update(file_path.to_string_lossy().as_bytes());
        let hash = hasher.finalize();
        let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

        // Find matching backup files
        let mut backups = Vec::new();
        if backup_dir.exists() {
            for entry in fs::read_dir(&backup_dir)? {
                let entry = entry?;
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Check if file starts with the hash
                    if name.starts_with(&hash_hex) && name.ends_with(".age") {
                        backups.push(path);
                    }
                }
            }
        }

        if backups.is_empty() {
            return Err(anyhow::anyhow!(
                "No backups found for file: {:?}",
                file_path
            ));
        }

        // Sort to get the latest
        backups.sort();
        let latest_backup = backups
            .last()
            .ok_or_else(|| anyhow::anyhow!("No backups found for file: {:?}", file_path))?;

        // Decrypt
        let encrypted_content = fs::read(latest_backup)?;
        let decrypted_content = self.unlock_payload(&encrypted_content)?;

        // Write back
        fs::write(file_path, decrypted_content)?;

        Ok(latest_backup.clone())
    }

    /// List all available backups for a given file path, sorted by timestamp (newest first).
    pub fn list_backups(&self, file_path: &Path) -> Result<Vec<PathBuf>> {
        let home = self.get_home()?;
        let backup_dir = home.join(".demon").join("backups");

        if !backup_dir.exists() {
            return Ok(Vec::new());
        }

        // Hash the path to find matching backups
        let mut hasher = Sha256::new();
        hasher.update(file_path.to_string_lossy().as_bytes());
        let hash = hasher.finalize();
        let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

        // Find matching backup files
        let mut backups = Vec::new();
        for entry in fs::read_dir(&backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Check if file starts with the hash
                if name.starts_with(&hash_hex) && name.ends_with(".age") {
                    backups.push(path);
                }
            }
        }

        // Sort reverse (newest first)
        backups.sort_by(|a, b| b.cmp(a));

        Ok(backups)
    }

    /// In-situ Clean: Scan for secrets and replace with REDACTED_REGEX tags.
    pub fn smart_clean(&self, content: &str) -> Result<String> {
        let scanner = SecretScanner::new()?;
        self.smart_clean_with_scanner(content, &scanner)
    }

    /// In-situ Clean with a specific scanner (allows filtering patterns)
    fn smart_clean_with_scanner(&self, content: &str, scanner: &SecretScanner) -> Result<String> {
        let mut had_error = false;
        let mut last_err: String = String::new();
        let cleaned = scanner.scan_and_replace(content, |_, secret| {
            match self.encrypt_v2_for_all(secret.as_bytes()) {
                Ok(encrypted) => {
                    let b64 = general_purpose::STANDARD.encode(encrypted);
                    format!("[{}:{}]", self.secret_marker, b64)
                }
                Err(e) => {
                    last_err = e.to_string();
                    had_error = true;
                    // MUST NOT return raw secret - encryption failed, so signal error
                    // to prevent plaintext secret from being committed to git.
                    // The closure returns a String, but the outer function returns Err
                    // when had_error is true. Using empty string as sentinel since
                    // had_error=true will cause the function to return Err regardless.
                    String::new()
                }
            }
        });
        if had_error {
            Err(anyhow::anyhow!("smart_clean: encryption failed for one or more secrets: {}. NOT committing plaintext.", last_err))
        } else {
            Ok(cleaned)
        }
    }

    /// Smart Clean with Path Context:
    /// If the path is in a sensitive directory (e.g. .ssh, .aws) OR force_encrypt is true, encrypt the ENTIRE file.
    /// Otherwise, use regex-based in-situ encryption (if text).
    pub fn smart_clean_with_path(&self, content: &[u8], path_str: &str) -> Result<Vec<u8>> {
        // 1. Definition of Sensitive Paths (Still used for binary detection)
        let sensitive_dirs = [
            ".ssh",
            "demon/keys",
            "demon/secrets",
            ".aws",
            ".kube",
            ".gnupg",
            ".azure",
            ".config/gcloud",
        ];

        let sensitive_exts = [
            ".age", ".key", ".p12", ".pfx", ".pem", ".crt", ".der", ".asc", ".zip", ".tar", ".gz",
            ".bz2", ".7z", ".rar", ".tgz", ".xz", ".tar.gz", ".tar.bz2", ".tar.xz", ".sqlite",
            ".sqlite3", ".db", ".vmdk", ".img", ".qcow2", ".vdi", ".iso", ".docker", ".oci",
            ".xlsx", ".csv", ".ods", ".kdbx", ".1pif", ".sql", ".apk", ".aab", ".dmg", ".pcap",
            ".pcapng", ".ovpn", ".tfstate", ".tfplan", ".tfvars",
        ];

        let sensitive_filenames = [
            "id_rsa",
            "id_ed25519",
            "id_ecdsa",
            "id_dsa",
            "id_xmss",
            "master.age",
            "identity.age",
            "owner.age",
            "demon-key",
            "id_rsa.pub",
            "id_ed25519.pub",
            "credentials",
            ".bash_history",
            ".zsh_history",
            ".sh_history",
            "core",
            "known_hosts",
            "vault.yml",
            ".terraform.lock.hcl",
            "terraform.tfvars",
            ".env",
            ".env.local",
            ".env.production",
            ".env.development",
            ".env.staging",
            ".npmrc",
            ".pypirc",
            "netrc",
            ".pgpass",
            ".my.cnf",
        ];

        let filename = std::path::Path::new(path_str)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Check if any path component exactly matches a sensitive directory name.
        // Using component-level matching avoids false positives like "my.ssh.config"
        // matching ".ssh" via substring contains.
        let path_components: Vec<&str> = std::path::Path::new(path_str)
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        // Single-component matching
        let has_single_component = sensitive_dirs
            .iter()
            .any(|dir| !dir.contains('/') && path_components.contains(dir));
        // Multi-component sequence matching (e.g. ".config/gcloud")
        let has_multi_component = sensitive_dirs.iter().any(|dir| {
            let parts: Vec<&str> = dir.split('/').collect();
            if parts.len() < 2 {
                return false;
            }
            path_components
                .windows(parts.len())
                .any(|window| window == parts.as_slice())
        });

        let is_sensitive_location = has_single_component
            || has_multi_component
            || sensitive_exts.iter().any(|ext| path_str.ends_with(ext))
            || sensitive_filenames.contains(&filename)
            || sensitive_filenames
                .iter()
                .any(|p| filename == *p || filename.starts_with(&format!("{}.", p)))
            || self
                .managed_patterns
                .iter()
                .any(|p| filename == p || path_str.contains(p));

        // 2. Process based on content type
        match std::str::from_utf8(content) {
            Ok(text_content) => {
                // Full encryption for sensitive files that shouldn't leak structure
                let is_full_encrypt = is_sensitive_location
                    && (filename.starts_with(".env")
                        || filename == "credentials"
                        || filename.starts_with(".bash_history")
                        || filename.starts_with(".zsh_history")
                        || filename.starts_with(".sh_history")
                        || filename == "vault.yml");
                if is_full_encrypt {
                    // Don't double-encrypt
                    if content.starts_with(HEADER_V2_MAGIC)
                        || self.starts_with_any_secret_tag(content)
                    {
                        return Ok(content.to_vec());
                    }
                    // Add/increment version header for .env files to track changes
                    let content_to_encrypt = if filename.starts_with(".env") {
                        // Check if this is already a warden-managed file by looking for our marker
                        if text_content.contains("Dracon Warden") {
                            // Remove old header and add new one with incremented version
                            let stripped = strip_env_version_header(text_content);
                            format!(
                                "{}\n{}",
                                make_env_version_header(text_content),
                                stripped.trim()
                            )
                        } else {
                            // First time encryption - add v1 header
                            format!(
                                "{}\n{}",
                                make_env_version_header(text_content),
                                text_content
                            )
                        }
                    } else {
                        text_content.to_string()
                    };
                    return self.encrypt_v2_to_b64_tag(content_to_encrypt.as_bytes());
                }
                // For identity files (master.age, identity.age), use a scanner that
                // skips age key patterns to avoid encrypting the identity itself,
                // but still catches other embedded secrets like API keys.
                let is_identity_file = filename == "master.age" || filename == "identity.age";
                let cleaned = if is_identity_file {
                    let scanner = SecretScanner::new_without_age_keys()?;
                    self.smart_clean_with_scanner(text_content, &scanner)?
                } else {
                    self.smart_clean(text_content)?
                };
                Ok(cleaned.into_bytes())
            }
            Err(_) => {
                // Binary Data: Only encrypt if it is in a sensitive location
                if is_sensitive_location {
                    // Don't double-encrypt
                    if content.starts_with(HEADER_V2_MAGIC)
                        || self.starts_with_any_secret_tag(content)
                    {
                        return Ok(content.to_vec());
                    }
                    self.encrypt_v2_to_b64_tag(content)
                } else {
                    // Normal binary path -> Passthrough (preserves images, etc)
                    Ok(content.to_vec())
                }
            }
        }
    }

    fn encrypt_v2_to_b64_tag(&self, content: &[u8]) -> Result<Vec<u8>> {
        match self.encrypt_v2_for_all(content) {
            Ok(encrypted) => {
                let b64 = general_purpose::STANDARD.encode(encrypted);
                Ok(format!("[{}:{}]", self.secret_marker, b64).into_bytes())
            }
            Err(e) => Err(anyhow::anyhow!("Failed to encrypt sensitive file: {}", e)),
        }
    }

    /// In-situ Smudge: Decrypt REDACTED_REGEX tags back to plaintext.
    pub fn smart_smudge(&self, content: &str) -> Result<String> {
        let markers = self.secret_tag_prefixes();
        let mut result = String::new();
        let mut last_end = 0;

        while last_end < content.len() {
            let mut next: Option<(usize, usize)> = None;
            for marker in &markers {
                if let Some(start_idx) = content[last_end..].find(marker) {
                    let absolute_start = last_end + start_idx;
                    let marker_len = marker.len();
                    if next
                        .map(|(best_idx, _)| absolute_start < best_idx)
                        .unwrap_or(true)
                    {
                        next = Some((absolute_start, marker_len));
                    }
                }
            }

            let Some((absolute_start, marker_len)) = next else {
                break;
            };

            result.push_str(&content[last_end..absolute_start]);

            // Find closing bracket
            if let Some(end_offset) = content[absolute_start..].find(']') {
                let absolute_end = absolute_start + end_offset + 1;
                let b64 = &content[absolute_start + marker_len..absolute_end - 1];

                match general_purpose::STANDARD.decode(b64.trim()) {
                    Ok(encrypted) => match self.unlock_payload(&encrypted) {
                        Ok(plaintext) => {
                            result.push_str(&String::from_utf8_lossy(&plaintext));
                        }
                        Err(_) => result.push_str(&content[absolute_start..absolute_end]),
                    },
                    Err(_) => result.push_str(&content[absolute_start..absolute_end]),
                }
                last_end = absolute_end;
            } else {
                // No closing bracket found, treat as normal text
                result.push_str(&content[absolute_start..]);
                last_end = content.len();
            }
        }

        result.push_str(&content[last_end..]);
        Ok(result)
    }

    /// Git Clean Filter: Encrypt stdin -> stdout
    /// V2 Upgrade: Encrypts to ALL known public keys (User + Machines + Teams)
    pub fn seal_clean(&self, file_path: Option<&str>) -> Result<()> {
        use std::io::{Read, Write};

        // 1. Read plaintext from stdin
        let mut buffer = Vec::new();
        std::io::stdin().read_to_end(&mut buffer)?;

        // Auto-add key to avoid lockout (Ensure keys folder exists)
        if let Err(e) = self.ensure_current_user_key() {
            eprintln!("⚠️ failed to ensure user key: {}", e);
        }

        // 3. Backup (Safety Net) - must happen before buffer is potentially moved
        if let Some(path) = file_path {
            if path.contains(".env") {
                if let Err(e) = self.backup_secret(path, &buffer) {
                    eprintln!("⚠️ failed to backup .env file: {}", e);
                }
            }
        }

        // 4. Smart Clean: Targeted encryption only to preserve Git diffs.
        // Every file (UTF-8) is scanned for secrets.
        // Binary files are passed through untouched to preserve Git diffs.
        let output = if let Ok(text_content) = std::str::from_utf8(&buffer) {
            self.smart_clean(text_content)?.into_bytes()
        } else {
            buffer
        };

        // 5. Write to stdout
        std::io::stdout().write_all(&output)?;

        Ok(())
    }

    /// Recursive disk-wide decryption: Replaces all [*_SECRET:...] tags with plaintext in-place.
    pub fn decrypt_path(&self, root: &Path, recursive: bool, dry_run: bool) -> Result<usize> {
        let mut total_restored = 0;
        let mut walk_errors = 0;

        if !root.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {:?}", root));
        }

        if root.is_file() {
            return self.decrypt_file(root, dry_run);
        }

        let walker = walkdir::WalkDir::new(root)
            .max_depth(if recursive { usize::MAX } else { 1 })
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                if e.path() == root {
                    return true;
                }
                !name.starts_with('.') || name == ".env"
            });

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!(
                        "⚠️ walk error during secret restore at {}: {}",
                        root.display(),
                        e
                    );
                    walk_errors += 1;
                    continue;
                }
            };
            if entry.file_type().is_file() {
                if let Ok(count) = self.decrypt_file(entry.path(), dry_run) {
                    total_restored += count;
                }
            }
        }

        if walk_errors > 0 {
            return Err(anyhow::anyhow!(
                "decrypt_path completed with {} walk error(s)",
                walk_errors
            ));
        }

        Ok(total_restored)
    }

    fn decrypt_file(&self, path: &Path, dry_run: bool) -> Result<usize> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Ok(0),
        };

        if !self.contains_any_secret_tag(&content) {
            return Ok(0);
        }

        let smudged = self.smart_smudge(&content)?;
        if smudged == content {
            return Ok(0);
        }

        // Count how many tags were replaced
        let tag_count = self.count_secret_tags(&content);

        if !dry_run {
            // Write back to disk
            std::fs::write(path, smudged)?;
            println!("  🔓 Restored {} secrets in {:?}", tag_count, path);
        } else {
            println!("  🔍 Would restore {} secrets in {:?}", tag_count, path);
        }

        Ok(tag_count)
    }

    /// Migrate secret marker prefixes in-place without touching encrypted payload bytes.
    /// Example: `[OLD_MARKER:...]` -> `[DRACON_SECRET:...]`.
    pub fn migrate_markers_in_path(
        &self,
        root: &Path,
        recursive: bool,
        dry_run: bool,
        from_marker: &str,
        to_marker: &str,
    ) -> Result<MarkerMigrationStats> {
        let from = normalize_secret_marker(from_marker)
            .ok_or_else(|| anyhow::anyhow!("Invalid source marker: {}", from_marker))?;
        let to = normalize_secret_marker(to_marker)
            .ok_or_else(|| anyhow::anyhow!("Invalid target marker: {}", to_marker))?;

        let from_prefix = format!("[{}:", from);
        let to_prefix = format!("[{}:", to);
        let mut stats = MarkerMigrationStats::default();

        if !root.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {:?}", root));
        }

        let mut process_file = |path: &Path| -> Result<()> {
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => return Ok(()),
            };
            stats.files_scanned += 1;

            let count = content.matches(&from_prefix).count();
            if count == 0 {
                return Ok(());
            }

            let migrated = content.replace(&from_prefix, &to_prefix);
            if !dry_run {
                fs::write(path, migrated)?;
            }

            stats.files_changed += 1;
            stats.markers_changed += count;
            Ok(())
        };

        if root.is_file() {
            process_file(root)?;
            return Ok(stats);
        }

        let walker = walkdir::WalkDir::new(root)
            .max_depth(if recursive { usize::MAX } else { 1 })
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                if e.path() == root {
                    return true;
                }
                !name.starts_with('.') || name == ".env"
            });

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!(
                        "⚠️ walk error during marker scan at {}: {}",
                        root.display(),
                        e
                    );
                    stats.walk_errors += 1;
                    continue;
                }
            };
            if entry.file_type().is_file() {
                if let Err(e) = process_file(entry.path()) {
                    eprintln!("⚠️ failed to process {}: {}", entry.path().display(), e);
                }
            }
        }

        if stats.walk_errors > 0 {
            return Err(anyhow::anyhow!(
                "migrate_markers_in_path completed with {} walk error(s)",
                stats.walk_errors
            ));
        }

        Ok(stats)
    }

    /// Git Smudge Filter: Decrypt stdin/file -> stdout
    /// Gracefully handles: V2 (Direct), V1 (RepoKey), Plaintext, REDACTED_REGEX wrapped
    pub fn seal_smudge(&self, file_path: Option<&str>) -> Result<()> {
        use std::io::{Read, Write};

        // 1. Read content
        let mut buffer = Vec::new();
        if let Some(path) = file_path {
            let mut file = fs::File::open(path)?;
            file.read_to_end(&mut buffer)?;
        } else {
            std::io::stdin().read_to_end(&mut buffer)?;
        }

        // 2. Check for V2 (Age) Header
        if buffer.starts_with(HEADER_V2_MAGIC) {
            match self.unlock_payload(&buffer) {
                Ok(plaintext) => {
                    std::io::stdout().write_all(&plaintext)?;
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("⚠️ V2 Decryption Failed: {}", e);
                    // Fallthrough to pass raw (might be intended?)
                }
            }
        }

        // 3. Check for *_SECRET text wrapper format
        if let Ok(text) = std::str::from_utf8(&buffer) {
            if self.contains_any_secret_tag(text) {
                let smudged = self.smart_smudge(text)?;
                std::io::stdout().write_all(smudged.as_bytes())?;
                return Ok(());
            }
        }

        // 4. Fallback: Pass raw buffer (Plaintext or already decrypted)
        std::io::stdout().write_all(&buffer)?;
        Ok(())
    }

    /// Encrypt data using the repo key with AES-256-GCM.
    ///
    /// SECURITY NOTE: Uses a random 12-byte nonce per encryption. For very high-volume
    /// repositories (2^48+ encrypted files with the same repo key), nonce collision
    /// becomes a meaningful risk for GCM mode. For typical use, the random nonce
    /// per-file is sufficient. Consider key rotation if your repo will exceed this scale.
    pub fn encrypt_with_repo_key(&self, repo_key: &RepoKey, plaintext: &[u8]) -> Result<Vec<u8>> {
        let key = Key::<Aes256Gcm>::from_slice(&repo_key.0);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("Encryption failure: {}", e))?;

        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypt data using the repo key
    pub fn decrypt_with_repo_key(
        &self,
        repo_key: &RepoKey,
        encrypted_data: &[u8],
    ) -> Result<Vec<u8>> {
        if encrypted_data.len() < 12 {
            // Graceful fallback: If data is too short, it might be plain text or empty.
            // For filter, error to be safe.
            return Err(anyhow::anyhow!("Invalid ciphertext length"));
        }

        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];

        let key = Key::<Aes256Gcm>::from_slice(&repo_key.0);
        let cipher = Aes256Gcm::new(key);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failure: {}", e))?;

        Ok(plaintext)
    }

    /// Decrypt data using the legacy Git Seal V1 format (AES-256-CFB with derived IV).
    /// WARNING: This format uses a deterministic IV derived from the key (SHA-256 hash → first 16 bytes), which violates AES-CFB security requirements. Using the same IV for multiple encryptions leaks information about plaintext relationships. This format exists for backward compatibility with legacy git-seal ciphertexts. DO NOT use this for new encryptions. If you have ciphertexts created with this format, consider migrating to AES-256-GCM (encrypt_with_repo_key) with random nonces.
    pub fn decrypt_git_seal(&self, repo_key: &RepoKey, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if !is_v1_fallback_allowed() {
            return Err(anyhow::anyhow!(
                "V1 decryption disabled: legacy format uses deterministic IV which violates AES-CFB security. \
                 Enable with allow_v1_fallback = true in policy, then migrate ciphertexts to V2."
            ));
        }
        eprintln!("⚠️ WARNING: decrypting legacy V1 ciphertext (deterministic IV — insecure). Migrate to V2.");
        let mut hasher = Sha256::new();
        hasher.update(&repo_key.0);
        let key_hash = hasher.finalize();

        let key = &key_hash[..32];
        let iv = &key_hash[..16];

        // Using dynamic dispatch or just specific type
        // Using specific Decryptor type from cfb_mode
        let cipher = cfb_mode::Decryptor::<aes::Aes256>::new_from_slices(key, iv)
            .map_err(|e| anyhow::anyhow!("CFB init error: {}", e))?;

        let mut plaintext = ciphertext.to_vec();
        cipher.decrypt(&mut plaintext);

        // Simple heuristic: check if result is likely plaintext
        // If it's garbage, it was probably not encrypted this way
        let is_likely_plaintext = plaintext
            .iter()
            .take(20)
            .all(|&b| b.is_ascii() && (b.is_ascii_graphic() || b.is_ascii_whitespace() || b == 0));

        if !is_likely_plaintext {
            return Err(anyhow::anyhow!(
                "Git Seal decryption produced binary garbage"
            ));
        }

        Ok(plaintext)
    }

    /// "Drunk guy with keychain" - try all keys from ~/demon/keys/
    fn try_keychain_bruteforce(&self, ciphertext: &[u8]) -> Option<Vec<u8>> {
        let home = match std::env::var("HOME") {
            Ok(h) => PathBuf::from(h),
            Err(_) => return None,
        };

        // Check demon key directories
        let keychain_dirs = vec![
            home.join(".demon").join("keys"), // key storage
            home.join(".arcane").join("keys"),
        ];

        for keys_dir in &keychain_dirs {
            if !keys_dir.exists() {
                continue;
            }

            // Collect all key files
            let entries = match fs::read_dir(keys_dir) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!(
                        "⚠️ failed to read keychain directory {}: {}",
                        keys_dir.display(),
                        e
                    );
                    continue;
                }
            };

            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!(
                            "⚠️ failed to read keychain entry in {}: {}",
                            keys_dir.display(),
                            e
                        );
                        continue;
                    }
                };
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                // Try to read as raw key (32 bytes)
                if let Ok(key_bytes) = fs::read(&path) {
                    if key_bytes.len() == 32 {
                        let repo_key = RepoKey(key_bytes);

                        // Try AES-GCM
                        if let Ok(plaintext) = self.decrypt_with_repo_key(&repo_key, ciphertext) {
                            // SECURITY: Only log in debug mode - attacker gaining access to stderr
                            // would learn which key format succeeded, reducing bruteforce cost.
                            #[cfg(debug_assertions)]
                            eprintln!(
                                "🔓 Decrypted with keychain key (AES-GCM): {:?}",
                                path.file_name()
                            );
                            return Some(plaintext);
                        }

                        // Try AES-CFB (git-seal style)
                        if let Ok(plaintext) = self.decrypt_git_seal(&repo_key, ciphertext) {
                            #[cfg(debug_assertions)]
                            eprintln!(
                                "🔓 Decrypted with keychain key (AES-CFB): {:?}",
                                path.file_name()
                            );
                            return Some(plaintext);
                        }
                    }
                }
            }
        } // end for keys_dir

        None
    }

    /// Recursively list files and check for ignored files (using whitelist)
    pub fn scan_dir(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    // Skip hidden directories (except .git? no, skip .git)
                    if path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .starts_with('.')
                    {
                        continue;
                    }
                    if let Ok(sub) = self.scan_dir(&path) {
                        files.extend(sub);
                    }
                } else {
                    files.push(path);
                }
            }
        }
        Ok(files)
    }

    fn get_registries_path(&self) -> Result<PathBuf> {
        let home = self.get_home()?;
        Ok(home.join(".demon").join("registries.age"))
    }

    pub fn load_registry_credentials(&self) -> Result<Vec<RegistryCredential>> {
        let path = self.get_registries_path()?;
        if !path.exists() {
            return Ok(Vec::new());
        }

        let encrypted_bytes = fs::read(&path)?;
        let identity = self
            .master_identities
            .first()
            .context("Master identity required to unlock registry credentials")?;

        let decryptor = age::Decryptor::new(std::io::Cursor::new(&encrypted_bytes))?;
        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        };

        let mut json_bytes = Vec::new();
        reader.read_to_end(&mut json_bytes)?;

        let creds: Vec<RegistryCredential> = serde_json::from_slice(&json_bytes)?;
        Ok(creds)
    }

    pub fn save_registry_credential(&self, cred: RegistryCredential) -> Result<()> {
        let mut creds = self.load_registry_credentials().unwrap_or_default();

        // Upsert logic
        if let Some(existing) = creds.iter_mut().find(|c| c.registry == cred.registry) {
            existing.username = cred.username;
            existing.password = cred.password;
        } else {
            creds.push(cred);
        }

        self.save_registry_credentials_list(&creds)
    }

    fn save_registry_credentials_list(&self, creds: &[RegistryCredential]) -> Result<()> {
        let path = self.get_registries_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json_bytes = serde_json::to_vec(creds)?;

        let master = self
            .master_identities
            .first()
            .context("Master identity required to encrypt registry credentials")?;
        let recipient = master.to_public();

        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient)];
        let encryptor = age::Encryptor::with_recipients(recipients)
            .context("Failed to create encryptor for registry credentials")?;

        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)?;
        let mut writer = encryptor.wrap_output(&mut file)?;
        writer.write_all(&json_bytes)?;
        writer.finish()?;

        Ok(())
    }
}

pub struct Warden;

/// Detect binary content by checking for null bytes.
/// Git uses a similar heuristic: any null byte means binary.
fn is_binary_content(bytes: &[u8]) -> bool {
    bytes.contains(&0)
}

impl Warden {
    pub fn new() -> Result<Self> {
        Ok(Warden)
    }

    pub fn smudge(&self, bytes: &[u8], _path: Option<&str>) -> Result<Vec<u8>> {
        if is_binary_content(bytes) {
            return Ok(bytes.to_vec());
        }
        let content = String::from_utf8_lossy(bytes);
        let smudged = DemonSecurity::new(None)?.smart_smudge(&content)?;
        Ok(smudged.into_bytes())
    }
}

pub struct DraconWarden;

impl DraconWarden {
    pub fn new() -> Result<Self> {
        Ok(DraconWarden)
    }

    pub fn smudge(&self, bytes: &[u8], _path: Option<&str>) -> Result<Vec<u8>> {
        if is_binary_content(bytes) {
            return Ok(bytes.to_vec());
        }
        let content = String::from_utf8_lossy(bytes);
        let security = DemonSecurity::get_or_init()?;
        let smudged = security.smart_smudge(&content)?;
        Ok(smudged.into_bytes())
    }

    pub fn clean(&self, bytes: &[u8], path: Option<&str>) -> Result<Vec<u8>> {
        let security = DemonSecurity::get_or_init()?;
        let cleaned = security.smart_clean_with_path(bytes, path.unwrap_or(""))?;
        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smudge_robustness() {
        let security = DemonSecurity::new(None).unwrap();
        assert_eq!(
            security.smart_smudge("[DRACON_SECRET:]").unwrap(),
            "[DRACON_SECRET:]"
        );
        let long_junk = "A".repeat(5000);
        let tag = format!("[DRACON_SECRET:{}]", long_junk);
        let input = format!("before {} after", tag);
        let output = security.smart_smudge(&input).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn test_smudge_passes_binary_unchanged() {
        let warden = DraconWarden::new().expect("create warden");
        // Binary content with null bytes should pass through unchanged
        let binary = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        let result = warden.smudge(binary, None).expect("smudge binary");
        assert_eq!(
            result, binary,
            "binary content should pass through smudge unchanged"
        );
    }

    #[test]
    fn test_protection_exemptions() {
        let scanner = SecretScanner::new_without_age_keys().unwrap();
        let patterns = scanner
            .patterns
            .iter()
            .map(|(n, _)| n.clone())
            .collect::<Vec<_>>();
        eprintln!(
            "Patterns in scanner (excluding age keys): {}",
            patterns.len()
        );
        eprintln!(
            "Age Secret Key in patterns: {}",
            patterns.contains(&"Age Secret Key".to_string())
        );

        let content = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAwWXNxYkxXd01JcDgxOHZvdEI1b29ib3Y4ck9DZUdIblpqdXBwVnpnRFFRCmlTckZESFdkQW1hK2VIVEhJdXdUUzBsbDFES2VuREJab2J1VWo5dGxOQkUKLT4gWDI1NTE5IFM0c1FwbHhVR25XUnpqZm16Q2tPZXQrMFAwU0gxUnpuYU5rSm1XNnVaMm8KVFI2NWZhemF4QTBKcklJdlJIeURjcXl3U1FSN1prcmNFem5STjhPdS9ZUQotPiBvVEVxSGRGTC1ncmVhc2UKYzFaSDJuL0o3cVZDRDFNbHdTOHlSZUZCb1lNV283Q1cvMkljdXFjUUZnTjdYcVVtK0VGYWVyTDE3V0s4YXI5dgppZjlrWUFGbXo1ZU4wejhuZVdYQzZhc0xhcXdNMStjZ3dOcjlkSUNXOHNZQjMzZTEwQQotLS0gbE9sRnJaell0TXhqM2dab2VrUGJDdWplbW4xYWNEdnY0aTVIclFRT251VQq/Et75hzLiCxv/1cZ9Ti2YiP1Dsr56fuJJuBi8i9F+o9qJ3iWKzqE6MqxK16tlCcDQXkZLndQEjKnzbyxAlloYMPH9Le3YsToYTI65kvN6ilJdyKNIbvzPopxrOwcdqSHe3dIAPN6nvzIo]";
        let scanned = scanner.scan_and_replace(content, |name, secret| {
            eprintln!("Match found: {} -> {}", name, secret);
            format!("[MATCHED:{}]", name)
        });
        eprintln!("Scanned result: {}", scanned);

        let security = DemonSecurity::new(None).unwrap();
        let result = security
            .smart_clean_with_path(content.as_bytes(), "master.age")
            .unwrap();
        let result_str = String::from_utf8_lossy(&result);
        assert!(
            result_str.contains("AGE-SECRET-KEY"),
            "Age key should be excluded from scanning and passed through unchanged! Result: {}",
            &result_str[..result_str.len().min(500)]
        );
    }

    #[test]
    fn test_marker_migration_in_place() {
        let security = DemonSecurity::new(None).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sample.env");
        std::fs::write(&file, "A=[OLD_SECRET:abc]\nB=[OLD_SECRET:def]\nC=plain\n").unwrap();

        let stats = security
            .migrate_markers_in_path(dir.path(), true, false, "OLD_SECRET", "DRACON_SECRET")
            .unwrap();
        assert_eq!(stats.files_changed, 1);
        assert_eq!(stats.markers_changed, 2);

        let migrated = std::fs::read_to_string(file).unwrap();
        assert!(migrated.contains("[DRACON_SECRET:abc]"));
        assert!(migrated.contains("[DRACON_SECRET:def]"));
        assert!(!migrated.contains("[OLD_SECRET:"));
    }

    #[test]
    fn test_get_env_version_extracts_version() {
        let v1_content = r#"# =============================================================================
# Dracon Warden Encrypted Environment File
# Version: 1
# =============================================================================
API_KEY=secret"#;
        assert_eq!(get_env_version(v1_content), 1);

        let v5_content = r#"# =============================================================================
# Dracon Warden Encrypted Environment File
# Version: 5
# =============================================================================
API_KEY=secret"#;
        assert_eq!(get_env_version(v5_content), 5);

        let no_version = r#"API_KEY=secret"#;
        assert_eq!(get_env_version(no_version), 0);
    }

    #[test]
    fn test_make_env_version_header_increments_version() {
        let v1_content = r#"# Version: 1
API_KEY=secret"#;
        let header = make_env_version_header(v1_content);
        assert!(header.contains("Version: 2"));

        let v0_content = r#"API_KEY=secret"#;
        let header = make_env_version_header(v0_content);
        assert!(header.contains("Version: 1"));
    }

    #[test]
    fn test_strip_env_version_header_removes_header() {
        let with_header = r#"# =============================================================================
# Dracon Warden Encrypted Environment File
# Version: 1
# =============================================================================
API_KEY=secret"#;
        let stripped = strip_env_version_header(with_header);
        assert!(!stripped.contains("Dracon Warden"));
        assert!(stripped.contains("API_KEY=secret"));
        assert!(stripped.starts_with("API_KEY=secret"));
    }

    #[test]
    fn test_strip_env_version_header_passthrough_when_no_header() {
        let no_header = "API_KEY=secret";
        let stripped = strip_env_version_header(no_header);
        assert_eq!(stripped, no_header);
    }

    #[test]
    fn test_env_versioning_increment_flow() {
        let security = DemonSecurity::new(None).unwrap();

        let v1_content = r#"# =============================================================================
# Dracon Warden Encrypted Environment File
# Version: 1
# =============================================================================
API_KEY=original"#;

        let encrypted = security
            .smart_clean_with_path(v1_content.as_bytes(), ".env.local")
            .unwrap();
        let encrypted_str = String::from_utf8_lossy(&encrypted);

        let decrypted = security.smart_smudge(&encrypted_str).unwrap();
        assert!(
            decrypted.contains("Version: 2"),
            "Version should increment to 2, got:\n{}",
            decrypted
        );
        assert!(
            decrypted.contains("API_KEY=original"),
            "Content should be preserved"
        );
    }

    #[test]
    fn test_demon_security_once_cell_caching() {
        let s1 = DemonSecurity::get_or_init().unwrap();
        let s2 = DemonSecurity::get_or_init().unwrap();
        assert_eq!(
            s1 as *const _ as usize, s2 as *const _ as usize,
            "get_or_init should return the same cached instance"
        );
    }

    fn test_security_with_identity() -> DemonSecurity {
        let mut security = DemonSecurity::new(None).unwrap();
        let key = x25519::Identity::generate();
        security.master_identities.push(key);
        security
    }

    #[test]
    fn test_encrypt_v2_decrypt_v2_roundtrip() {
        let security = test_security_with_identity();
        let plaintext = b"hello world, this is a secret message";

        let recipient = security.master_identities()[0].to_public();
        let encrypted = security
            .encrypt_v2(plaintext, vec![Box::new(recipient)])
            .unwrap();
        assert!(!encrypted.is_empty());
        assert_ne!(
            encrypted,
            plaintext.to_vec(),
            "encrypted should differ from plaintext"
        );

        let decrypted = security.decrypt_v2(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext, "decrypted should match original");
    }

    #[test]
    fn test_encrypt_v2_empty_data() {
        let security = test_security_with_identity();
        let recipient = security.master_identities()[0].to_public();
        let encrypted = security.encrypt_v2(b"", vec![Box::new(recipient)]).unwrap();
        let decrypted = security.decrypt_v2(&encrypted).unwrap();
        assert_eq!(decrypted, b"", "empty data should roundtrip");
    }

    #[test]
    fn test_encrypt_v2_binary_data() {
        let security = test_security_with_identity();
        let plaintext: Vec<u8> = (0..256).map(|i| i as u8).collect();
        let recipient = security.master_identities()[0].to_public();
        let encrypted = security
            .encrypt_v2(&plaintext, vec![Box::new(recipient)])
            .unwrap();
        let decrypted = security.decrypt_v2(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext, "binary data should roundtrip");
    }

    #[test]
    fn test_unlock_payload_v2_roundtrip() {
        let security = test_security_with_identity();
        let plaintext = b"secret data for unlock_payload test";
        let recipient = security.master_identities()[0].to_public();
        let encrypted = security
            .encrypt_v2(plaintext, vec![Box::new(recipient)])
            .unwrap();

        let unlocked = security.unlock_payload(&encrypted).unwrap();
        assert_eq!(unlocked, plaintext, "unlock_payload should decrypt v2");
    }

    #[test]
    fn test_decrypt_v2_fails_with_wrong_identity() {
        let tempdir = tempfile::tempdir().unwrap();
        let mut security1 = DemonSecurity::new(None).unwrap();
        security1.set_mock_home(tempdir.path().to_path_buf());
        security1.master_identities.clear();
        let key1 = x25519::Identity::generate();
        security1.master_identities.push(key1);

        let mut security2 = DemonSecurity::new(None).unwrap();
        security2.set_mock_home(tempdir.path().to_path_buf());
        security2.master_identities.clear();
        let key2 = x25519::Identity::generate();
        security2.master_identities.push(key2);

        let plaintext = b"data encrypted to key1";
        let recipient = security1.master_identities()[0].to_public();
        let encrypted = security1
            .encrypt_v2(plaintext, vec![Box::new(recipient)])
            .unwrap();

        let result = security2.decrypt_v2(&encrypted);
        assert!(result.is_err(), "decrypt with wrong identity should fail");
    }

    #[test]
    fn test_decrypt_v2_requires_master_identity() {
        let security = DemonSecurity::new(None).unwrap();
        let result = security.decrypt_v2(b"some encrypted data");
        assert!(
            result.is_err(),
            "decrypt_v2 should fail without master identities"
        );
    }

    #[test]
    fn test_encrypt_v2_for_all_roundtrip() {
        let security = test_security_with_identity();
        let plaintext = b"encrypt to all recipients test";
        let encrypted = security.encrypt_v2_for_all(plaintext).unwrap();
        let decrypted = security.decrypt_v2(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext, "encrypt_v2_for_all should roundtrip");
    }

    #[test]
    fn test_encrypt_v2_for_all_empty_data() {
        let security = test_security_with_identity();
        let encrypted = security.encrypt_v2_for_all(b"").unwrap();
        let decrypted = security.decrypt_v2(&encrypted).unwrap();
        assert_eq!(decrypted, b"", "empty data roundtrip");
    }

    #[test]
    fn test_normalize_secret_marker_valid() {
        assert_eq!(
            normalize_secret_marker("API_SECRET"),
            Some("API_SECRET".to_string())
        );
        assert_eq!(
            normalize_secret_marker("DB_SECRET"),
            Some("DB_SECRET".to_string())
        );
        assert_eq!(
            normalize_secret_marker("  api_secret  "),
            Some("API_SECRET".to_string())
        );
    }

    #[test]
    fn test_normalize_secret_marker_invalid() {
        assert_eq!(normalize_secret_marker("no_suffix"), None);
        assert_eq!(normalize_secret_marker(""), None);
        assert_eq!(normalize_secret_marker("API-SECRET"), None);
        assert_eq!(normalize_secret_marker("API SECRET"), None);
    }

    #[test]
    fn test_is_inside_secret_tag_detection() {
        let content = "prefix [API_SECRET:abc] suffix";
        assert!(is_inside_secret_tag(content, 20), "inside tag");
        assert!(!is_inside_secret_tag(content, 5), "before tag");
        assert!(!is_inside_secret_tag(content, 30), "after tag");
    }

    #[test]
    fn test_get_env_version_edge_cases() {
        assert_eq!(get_env_version(""), 0);
        assert_eq!(get_env_version("no version here"), 0);
        assert_eq!(get_env_version("Version: abc\n"), 0);
        assert_eq!(get_env_version("Version: 42\n"), 42);
    }

    #[test]
    fn test_github_token_patterns_accept_variable_length() {
        let scanner = SecretScanner::new_without_age_keys().unwrap();
        let short = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBoeEovOTB6M2s1RGNBdThhNzlrM29MQVNHWm9WVkhGZUo1RFdyUWpEbUJRCllnc1REdE9DWTVSdDF2YWQybUJUbGIzV3p3bC9ScVRFaE9paUk3b2xucW8KLT4gWDI1NTE5IHZGQXlvVVJpNDRDYjhLUHBWWGZJTXk0eHZCbk16bnlhUEFlSjVCWHNXMzAKVzMrZWFiNEtJOWhiQ1pGdzN1cXNPNnJrMzVkbHFkQ2c0cms0ZVB5ZW51YwotPiBzay0oOkgtZ3JlYXNlIFNvaWogWyhnK21RZiA6CkhpRFJQSkYvdFBmTStxcnA5YVRnbkJxVkhRSHNMVDdibVdWYmQ1b1VsYU9oNnRzVUNENUFxd25SNU9UazhKYkwKaXBqYjZ0eE9FaEl5bUQwWm5uKzZVK055K0FZCi0tLSAvanBjc2ZGYkErQ3hJWG9wa2FMdTJqc05kNzVpYm5yeUdGeENjbWxSNHlZCo+2Qhc9V/8chGfGDXvUanHUCbfMvchoRIdbpalPZy3nk14cqelUCeG4AaVAqbaCI/nUoohYHDRR7Z0+rZ3luWNwSg==]";
        let long = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB1dHdadnlneFhKY1E2V1JEeWFXckJNT1BhVmhlNWs5RE5NR2ZrTTZ1M1hFCjNiN2pWMDVYZS9DNnhVVUNuY2NON2IrODRmdGZZcWRjZlVSTnR6N3p2SncKLT4gWDI1NTE5IEwyVWxQcTZhSHVuSFdIQ216UWJoUXJwZmE0T1Zwc24wSDVMSXZMcUluWG8KVzhHaStIU0htQVVXUzdOOTFRay9OVnhZWFlMTjgrSzlkR1dtR1B4d3pnawotPiAhWVYoLWdyZWFzZSA5WnVsYkMgLHJ+TXRZWkggWWRVeQpGUHprVkloM0tZRDgvT1hXNW1yWFZnQ0h0S2MyRStiTlBER0QKLS0tIEt5VG5DWVJPUlE3NnlvbEYxczd3WnZvRk43bUNqVWZENG5FdFA0c2RZZ2sK9qQbTIPuVr7y3cQ4RTHUWA3q17dPHhewLTCF7LK59rgNAJ2dArBPCjjjb8l6NvVwNIiRUGu2p8EEtkGdss/2s6iF/Nx08vSxiabuBA==]";
        let found_short = scanner.scan(short);
        let found_long = scanner.scan(long);
        assert!(
            found_short
                .iter()
                .any(|f| f.name.contains("GitHub Token (ghp)")),
            "should detect short ghp_ token (30 chars after prefix), found: {:?}",
            found_short
        );
        assert!(
            found_long
                .iter()
                .any(|f| f.name.contains("GitHub Token (ghp)")),
            "should detect long ghp_ token (40 chars after prefix), found: {:?}",
            found_long
        );
    }

    #[test]
    fn test_mailgun_key_accepts_variable_length() {
        let scanner = SecretScanner::new_without_age_keys().unwrap();
        let short = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBRVzZ3WlduSHhSd1IySi9rSHBCSTJSMEM5dFpoUE11T25NOVlJUFR0bWdvClRhTVkxRmhRUStveXF0SlpBR2RxaURTYWxVQ3RpSEoyeWZtMm5LaUhZaWcKLT4gWDI1NTE5IForTGNEWVVOTm1VMVFhWExDYkR5Wm81VkkzVU1qeXpCTG5EdkFkeERkaDgKMDlvY0xQUnRBSnZpSWQ0QVNmZmpyNTU1WjdsVk1nOHJ3amcxYjcyL3VsSQotPiBbXX1Dbi1ncmVhc2UgUkQlNiAnOXtbaSBkOydBTip0PSAlfXgKeFdVbGROT2YvNGdZK1NWbXhDTURjbytGbml6eDQ4NnJlL3FCSE9ZZC9XUmtsSkZKTWQyVAotLS0gZ1U3ZXB6WW1IU3YweTRyZjlFYkRlazFxcUVsMVh6UTRVVitoREJBZkxKMAoeoboxl9x54ip0oeaKHKRwVyjaH6lRAmT8wsCa5Pm5O0hjX8PYDwxGWVfWESAdyKpNyV/yXkUADQW6wBGrBxrY8g==]";
        let long = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBnWlhHTUtBN2l4eGhSMWdHTDBrZ2tUMXpycEhFRXJZblZCQk9zdU81SGlRCkpvdkdkUndaR1pQOUxlZjZVWnF6bVBXbThVeU9rMThTNVZRYXhOM0tEUWMKLT4gWDI1NTE5IDFPOU9vK2l2WTNldThjdENPS2dlcmZ3K3BrV2xUakNja2FIbm1tcVFLa00KbTBrS2VpTWg4bXFiWlE3dE52TCsyRFljVjlpd0UxWDZjWGNBZ3dQN21SYwotPiBGcS1ncmVhc2UgZC0uIEMKUU9OKzdLbmxoVWdqc0JDRW1PVGpIamQyUFJSWmM0Z3VzKzZ0Z0dzUWZrQ1J2QVIxL2J3aHh4VDNtVCtoZFQzbwp3NXZ6c3FucDJwZkNyMHUwZWtOK1E2ZVpTcXpRT2VTZFU0TUxTQQotLS0gNGZCbnIyTUxYdUNJR0NZYXhXQWcrVG10UDY4R1g5TS9vZkI5cFRtMEhiRQqk4AERIOz8VaMXH8uclMRfu1qy2ZiUDlPEBhUQrMjZ4av7OcPrjJy9eHMxZ/6xSeXXRktFVBuBNes+qwOP6PMsk6zW7p0=]";
        let found_short = scanner.scan(short);
        let found_long = scanner.scan(long);
        assert!(
            found_short.iter().any(|f| f.name == "Mailgun API Key"),
            "should detect 28-char Mailgun key (after prefix), found: {:?}",
            found_short
        );
        assert!(
            found_long.iter().any(|f| f.name == "Mailgun API Key"),
            "should detect 34-char Mailgun key (after prefix), found: {:?}",
            found_long
        );
    }

    #[test]
    fn test_slack_bot_token_compact() {
        let scanner = SecretScanner::new_without_age_keys().unwrap();
        let [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBiMVdtYTIzb1p2TTFUZE5KeGhTRldGWVNnbmZWSlhabXY5N2xSaHl4MUJzCjF2bnh4dEM3U2o3b1JrT0RXVDNFMUFyYTdzQU9iaGkzM09mdXBCQW4zYkkKLT4gWDI1NTE5IDdzNHNYNWM4S0orZjIxbnRJWFRvc1pPU1hHL1ExTmFLaE12UVZlZzNkVmMKSjhRL0dGaVYxR0VKZ211ZzhhV3ROVkxFREYyaUxCMGwvL3NIRWtuTVAzZwotPiA2RVstZ3JlYXNlIHcvXSU6dCAiNXA1TzwgM3B3UWclVSAuSm5ECi9ZWFgxTTM3MndUOEM0OUs0MXRsMzJrMAotLS0gc3loKy95MG5RKzZ6ejFNM0t6SjV1WFkrQWRkUG9tdmpHZDcxeGNZQlVROAo/tEZf8CjCi0VXmjDYHQXDs8IFhdWAUJcA7VZ6BxC+DK74oUHoyjTZe7CtODoQA1CIR2rcpTiVICmsMLsG23H9AlPVPZU9WF8h];
        let found = scanner.scan(token);
        assert!(
            found.iter().any(|f| f.name == "Slack Bot Token (Compact)"),
            "should detect compact Slack bot token, found: {:?}",
            found
        );
    }

    #[test]
    fn test_slack_bot_token_compact_has_length_cap() {
        let scanner = SecretScanner::new_without_age_keys().unwrap();
        let reasonable = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBGVDZtV0c2ekdjOGF5bTQwYTdkRlNVQ2FUSUJZQWxab09oZEZWWWpvOEg4CngyWWhpZFJRdnM4QzY5dlFDQkJFL2pyTHg1TVo4QlUxVmV0YmZ0Z2ZyZ1kKLT4gWDI1NTE5IGZ5TjNuMWJUL1NUbG83UDVDN3NyY0haNDdoVUZLcG9zMmVVUXBLTHRSam8KT25xeW44ZVJpaGFjdnJuaGZ5LzFmNFkrRk1FRThoQjR0TitBUWh3M3dWdwotPiBMcTAtZ3JlYXNlIGstTG16czYqIHB4XywgW2tNdSAyTiMuLlhtPgozL2M0bFV4dG5LanpJUk5VdU1ZM0xOemFXV2I2bitMMThoOUUxeWRNTG9IYkFZek52cWo5U1dMRGFrVmk5WEZ2CnJpSUVLUQotLS0gSkpCVC9BUFM5QXducllEOHhYbGFRL25jZnJ3RlJKa2VHcHd6bkRVK05YRQphwvqCbcMzlQz+q5v2871SNsTzifw9QHITa0vmilCWEcnKIpGqGUPTQYLahdkZ5Y6wkBdgmL1HFAmIFujOTC+XNvQVfis8vauSYq5NvqxYEODczfAlQa9fvFmgEigM+58foV0wTQ==]";
        let found = scanner.scan(reasonable);
        assert!(
            found.iter().any(|f| f.name == "Slack Bot Token (Compact)"),
            "should match slack bot token up to 68 chars after xoxb-, found: {:?}",
            found
        );
    }

    #[test]
    fn test_hex_secret_quoted_requires_context() {
        let scanner = SecretScanner::new_without_age_keys().unwrap();
        let with_context = r#"[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBHcFlicXFKcWJWYXR4alB1OWs1SFhWUlQ2Qm9pN3R4dldFZm16U0I2Tlc0CnlhTDVKbTA4Q1dmUmErTXVsUkN0bVR5MUdRMGdGNXZ0MDFOYkRIVmpmOXMKLT4gWDI1NTE5IE4xMEJ2ekN5bnEvVFRyUmdWTENmc1FEd0xOZndNRVFEVnJzWEF2ZGwwdzgKRFJjcmd3dzhnNjhGTFlIeDdOdFpqQXB4S0hGTUJ0Wk9MMWIzMHlETHMzWQotPiBqdlA9bTc7LS1ncmVhc2UgagpEdwotLS0gekdjcWpockZEV3UwTk8wQVNyVXQ3d1BxdVJDSUhzbHBza255Tjl1cnhRRQo/OO6GWCbEvcEQ/2tzLQjFQ90EUPLLnIw0uVAi9t0JAcbHjDHP6eGeR84pR0s4ELG5NoG3rgEc1OdoplZJVw/jxWgyYGOB9vja+GDZ8OtN]"#;
        let without_context = r#"label = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4""#;
        let found_with = scanner.scan(with_context);
        let found_without = scanner.scan(without_context);
        assert!(
            found_with.iter().any(|f| f.name == "Hex Secret (Quoted)"),
            "should detect hex secret with context keyword, found: {:?}",
            found_with
        );
        assert!(
            !found_without
                .iter()
                .any(|f| f.name == "Hex Secret (Quoted)"),
            "should NOT detect hex string without context keyword, found: {:?}",
            found_without
        );
    }

    #[test]
    fn test_high_entropy_secret_quoted_requires_context() {
        let scanner = SecretScanner::new_without_age_keys().unwrap();
        let with_context = r#"[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBwTTJTbGNaUDlFUTBrZWtvQW85eEFaSGd1RHRpb0kvYnV0RGVrV1h3dUNnCmo1Yk1rMUZxWEpSVHBHREhrY2w3MTZISXRyaURHb3ZDSENuSUEwWGJIcmcKLT4gWDI1NTE5IFl6VlY2YU9Pem5reWJzOTJtK2pMb20wWFkyb0ttdnZNNk1kT0w3OEJOVVUKeWdFbVdzMEk2TnIzQnRkTk9Ub1hjWkx4VjNXK2VucjEydU14T0hnUiszZwotPiBRakxLKS1ncmVhc2UgWUc5TF1seCBMITF3CmtWVEhwSW9wMTRDa3JKQWppRUhzZ09YOGdMcGttbldzM21aZ0FUdVpVNHl1ZjJHUFFDa2FzRzgzTVFNZS9LY3UKMTA5clNFTFhubWpGNFdOTjVZWllnSENkTU9UbTY0WU9wSys4TFEKLS0tIFBTeW5iT2ErdU1VV0dkeEd4TzNmVTF1SDM4ZDlVeHhwM1pTWDhaaVcremMK2feYn5ZVb3pDZOKfFvHTen3kfc/D48XVE7xRpoi+b5qTK5CvNb2sWWqPkpR8Jv9iSLfmTCf2rJityVElgmosK1l8]"#;
        let without_context = r#"class_name = "aBcDeFgHiJkLmNoPqRsTuVwX""#;
        let found_with = scanner.scan(with_context);
        let found_without = scanner.scan(without_context);
        assert!(
            found_with
                .iter()
                .any(|f| f.name == "High-Entropy Secret (Quoted)"),
            "should detect high-entropy secret with context keyword, found: {:?}",
            found_with
        );
        assert!(
            !found_without
                .iter()
                .any(|f| f.name == "High-Entropy Secret (Quoted)"),
            "should NOT detect alphanumeric string without context keyword, found: {:?}",
            found_without
        );
    }
}
