//! Encryption and decryption operations.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use age::x25519;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::RepoKey;
use crate::WardenSecurity;

const HEADER_V2_MAGIC: &[u8] = b"age-encryption.org/v1";
const RECIPIENT_AUTH_VERSION: u8 = 1;
const RECIPIENT_AUTH_ROLE_MACHINE: &str = "machine";
const RECIPIENT_AUTH_ROLE_TEAM: &str = "team";
const MAX_RECIPIENT_AUTH_BYTES: usize = 4096;

#[derive(Debug, Deserialize, Serialize)]
struct RepoRecipientAuthorization {
    version: u8,
    role: String,
    file_name: String,
    recipient: String,
}

/// Mirrors `is_owner_pubkey_filename` in the warden binary (main.rs):
/// the canonical mesh files written by keygen/publish are `owner_*.pub`.
/// This crate is published separately, so the predicate is duplicated.
fn is_owner_pubkey_filename(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.starts_with("owner_") && name.ends_with(".pub")
}

/// The dedicated master recipient is also a supported repository key name.
/// Unlike the old arbitrary `*.pub`/`*.key` scan, this name is only a source
/// candidate; its single recipient still has to match a local trust anchor.
fn is_canonical_repo_recipient_filename(path: &Path) -> bool {
    is_owner_pubkey_filename(path)
        || path.file_name().and_then(|name| name.to_str()) == Some("master.pub")
}

fn recipient_authorization_path(public_path: &Path) -> PathBuf {
    public_path.with_extension("auth")
}

fn parse_public_recipient_lines(content: &str) -> Vec<x25519::Recipient> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            line.parse::<x25519::Recipient>().ok()
        })
        .collect()
}

/// Repository recipient files are an authorization boundary, not a key
/// transport format. Require exactly one valid recipient and compare it to
/// the operator's local trust anchors; a contributor cannot authorize a new
/// recipient by choosing an `owner_*.pub` basename.
fn parse_single_repo_recipient(content: &str) -> Option<x25519::Recipient> {
    let mut parsed = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let recipient = line.parse::<x25519::Recipient>().ok()?;
        if parsed.replace(recipient).is_some() {
            return None;
        }
    }
    parsed
}

/// Warn-once-per-process eprintln (a gather runs on every protected-file
/// clean; repeating per file would spam the journal).
fn warn_once(flag: &AtomicBool, msg: &str) {
    if !flag.swap(true, Ordering::Relaxed) {
        eprintln!("⚠️ warden: {}", msg);
    }
}

static WARNED_NON_OWNER_REPO_FILE: AtomicBool = AtomicBool::new(false);
static WARNED_UNTRUSTED_REPO_FILE: AtomicBool = AtomicBool::new(false);
static WARNED_SUSPICIOUS_FILE: AtomicBool = AtomicBool::new(false);

impl WardenSecurity {
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

    fn load_public_recipients_from_dir(
        &self,
        keys_dir: &Path,
        seen_keys: &mut HashSet<String>,
        recipients: &mut Vec<x25519::Recipient>,
        require_owner_naming: bool,
        trusted_repo_recipients: &HashSet<String>,
        repo_authorization_key: Option<&RepoKey>,
    ) {
        if !keys_dir.exists() {
            return;
        }
        if require_owner_naming {
            let Ok(metadata) = fs::symlink_metadata(keys_dir) else {
                return;
            };
            if metadata.file_type().is_symlink() {
                warn_once(
                    &WARNED_SUSPICIOUS_FILE,
                    &format!(
                        "ignoring repository recipient directory {} (symlinks are not a trust boundary)",
                        keys_dir.display()
                    ),
                );
                return;
            }
        }

        let Ok(entries) = fs::read_dir(keys_dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if require_owner_naming {
                let Ok(metadata) = fs::symlink_metadata(&path) else {
                    continue;
                };
                if metadata.file_type().is_symlink() {
                    warn_once(
                        &WARNED_SUSPICIOUS_FILE,
                        &format!("ignoring repository recipient symlink {}", path.display()),
                    );
                    continue;
                }
            }
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext != "pub" && ext != "key" {
                continue;
            }

            let canonical_name = is_canonical_repo_recipient_filename(&path);
            let Ok(pub_str) = fs::read_to_string(&path) else {
                continue;
            };
            // FIXED 2026-08-11 (audit MEDIUM): publish-path content
            // validation — secret key material or oversized files are
            // not recipient sources (mirrors validate_owner_age_pubkey_bytes:
            // no AGE-SECRET-KEY- material, <= 256 bytes).
            if pub_str.contains(concat!("AGE", "-", "SECRET", "-", "KEY", "-")) {
                warn_once(
                    &WARNED_SUSPICIOUS_FILE,
                    &format!(
                        "ignoring suspicious recipient file {} (contains secret key material)",
                        path.display()
                    ),
                );
                continue;
            }
            if pub_str.len() > 256 {
                warn_once(
                    &WARNED_SUSPICIOUS_FILE,
                    &format!(
                        "ignoring suspicious recipient file {} ({} bytes > 256)",
                        path.display(),
                        pub_str.len()
                    ),
                );
                continue;
            }

            if require_owner_naming {
                let Some(recipient) = parse_single_repo_recipient(&pub_str) else {
                    warn_once(
                        &WARNED_SUSPICIOUS_FILE,
                        &format!(
                            "ignoring recipient file {} (expected exactly one valid age recipient)",
                            path.display()
                        ),
                    );
                    continue;
                };
                let recipient_string = recipient.to_string();
                let local_owner = trusted_repo_recipients.contains(&recipient_string);
                let delegated_age_path = path.with_extension("age");
                let has_delegated_age = fs::symlink_metadata(&delegated_age_path)
                    .map(|metadata| metadata.file_type().is_file())
                    .unwrap_or(false);
                let authenticated_machine_or_team = has_delegated_age
                    && repo_authorization_key.is_some_and(|key| {
                        self.verify_repo_recipient_authorization(&path, &recipient, key)
                    });
                if !local_owner && !authenticated_machine_or_team {
                    if !canonical_name {
                        warn_once(
                            &WARNED_NON_OWNER_REPO_FILE,
                            &format!(
                                "ignoring recipient file {} (not a canonical owner/master public key or authenticated machine/team key)",
                                path.display()
                            ),
                        );
                    } else {
                        warn_once(
                            &WARNED_UNTRUSTED_REPO_FILE,
                            &format!(
                                "ignoring recipient file {} (recipient is not in the local owner trust anchors)",
                                path.display()
                            ),
                        );
                    }
                    continue;
                }
                if seen_keys.insert(recipient_string) {
                    recipients.push(recipient);
                }
                continue;
            }

            for recipient in parse_public_recipient_lines(&pub_str) {
                let recipient_string = recipient.to_string();
                if seen_keys.insert(recipient_string) {
                    recipients.push(recipient);
                }
            }
        }
    }

    /// Return the HOME key directories that are not also controlled by the
    /// repository being processed. Physical-path checks are intentional:
    /// `repo_root` may be a symlink, or a repository key directory may be a
    /// symlink into HOME. In either case, loading the path permissively would
    /// let a contributor-controlled checkout bypass the repository gate.
    fn home_key_dirs_for_repo(&self, repo_root: Option<&Path>) -> Vec<PathBuf> {
        let Ok(home) = self.get_home() else {
            return Vec::new();
        };
        let home_dirs = vec![
            home.join(".dracon").join("data").join("keys"),
            home.join(".dracon").join("keys"),
        ];
        let Some(repo_root) = repo_root else {
            return home_dirs;
        };
        let Ok(repo_root) = fs::canonicalize(repo_root) else {
            // If the boundary cannot be established, fail closed rather than
            // silently treating a potentially repository-controlled path as
            // operator-owned HOME.
            return Vec::new();
        };
        let repo_key_dirs = [
            repo_root.join(".dracon").join("data").join("keys"),
            repo_root.join(".git").join("arcane").join("keys"),
        ]
        .into_iter()
        .filter_map(|path| fs::canonicalize(path).ok())
        .collect::<Vec<_>>();

        home_dirs
            .into_iter()
            .filter(|home_dir| {
                let Ok(home_dir) = fs::canonicalize(home_dir) else {
                    // A missing or inaccessible HOME directory has nothing
                    // safe to load in this pass. Fail closed if its boundary
                    // cannot be resolved.
                    return false;
                };
                // A repository rooted at HOME (or below it) owns this path.
                if home_dir.starts_with(&repo_root) {
                    return false;
                }
                // A repository symlink can point its key directory directly
                // at a HOME key directory even when the roots differ.
                !repo_key_dirs.iter().any(|repo_dir| repo_dir == &home_dir)
            })
            .collect()
    }

    /// Return recipients that are trusted by the local operator and may
    /// therefore be accepted from a repository-controlled key file.
    ///
    /// A repository filename is not an authorization mechanism: contributors
    /// can choose both its basename and its contents. The local private
    /// identities and operator-owned HOME key directory are the trust anchors.
    fn local_recipient_trust_anchors(&self, home_key_dirs: &[PathBuf]) -> HashSet<String> {
        let mut trusted = self
            .master_identities
            .iter()
            .chain(self.imported_identities.iter())
            .map(|identity| identity.to_public().to_string())
            .collect::<HashSet<_>>();
        let mut seen_keys = HashSet::new();
        let mut home_recipients = Vec::new();
        let no_repo_trust_anchors = HashSet::new();

        for keys_dir in home_key_dirs {
            self.load_public_recipients_from_dir(
                keys_dir,
                &mut seen_keys,
                &mut home_recipients,
                false,
                &no_repo_trust_anchors,
                None,
            );
        }
        trusted.extend(home_recipients.into_iter().map(|recipient| recipient.to_string()));
        trusted
    }

    /// Verify a repo-key-authenticated authorization sidecar written by
    /// `whitelist_machine` or `add_team_member`. The sidecar binds the exact
    /// basename and recipient, so copying it cannot authorize a different
    /// file or key. The repo key is secret to an authorized operator; a
    /// contributor who can push repository files cannot forge this proof.
    fn verify_repo_recipient_authorization(
        &self,
        public_path: &Path,
        recipient: &x25519::Recipient,
        repo_key: &RepoKey,
    ) -> bool {
        let Ok(file_name) = public_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(())
        else {
            return false;
        };
        let auth_path = recipient_authorization_path(public_path);
        let Ok(auth_metadata) = fs::symlink_metadata(&auth_path) else {
            return false;
        };
        if !auth_metadata.file_type().is_file() {
            return false;
        }
        let Ok(ciphertext) = fs::read(auth_path) else {
            return false;
        };
        if ciphertext.len() > MAX_RECIPIENT_AUTH_BYTES {
            return false;
        }
        let Ok(plaintext) = self.decrypt_with_repo_key(repo_key, &ciphertext) else {
            return false;
        };
        let Ok(auth) = serde_json::from_slice::<RepoRecipientAuthorization>(&plaintext) else {
            return false;
        };
        auth.version == RECIPIENT_AUTH_VERSION
            && matches!(
                auth.role.as_str(),
                RECIPIENT_AUTH_ROLE_MACHINE | RECIPIENT_AUTH_ROLE_TEAM
            )
            && auth.file_name == file_name
            && auth.recipient == recipient.to_string()
    }

    /// Write an authenticated authorization sidecar for a machine/team public
    /// recipient. This is deliberately repo-key encrypted rather than based
    /// on a contributor-chosen filename; the loader can therefore preserve
    /// the machine/team APIs without reopening the arbitrary-file trust gap.
    pub(crate) fn write_repo_recipient_authorization(
        &self,
        repo_key: &RepoKey,
        public_path: &Path,
        role: &str,
        recipient: &x25519::Recipient,
    ) -> Result<()> {
        if !matches!(role, RECIPIENT_AUTH_ROLE_MACHINE | RECIPIENT_AUTH_ROLE_TEAM) {
            anyhow::bail!("invalid repository recipient authorization role")
        }
        let file_name = public_path
            .file_name()
            .and_then(|name| name.to_str())
            .context("recipient public key path is not valid UTF-8")?;
        let auth = RepoRecipientAuthorization {
            version: RECIPIENT_AUTH_VERSION,
            role: role.to_string(),
            file_name: file_name.to_string(),
            recipient: recipient.to_string(),
        };
        let payload = serde_json::to_vec(&auth).context("serialize recipient authorization")?;
        let ciphertext = self.encrypt_with_repo_key(repo_key, &payload)?;
        let auth_path = recipient_authorization_path(public_path);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&auth_path)
                .with_context(|| {
                    format!("create recipient authorization {}", auth_path.display())
                })?
                .write_all(&ciphertext)?;
        }
        #[cfg(not(unix))]
        {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&auth_path)
                .with_context(|| {
                    format!("create recipient authorization {}", auth_path.display())
                })?
                .write_all(&ciphertext)?;
        }
        Ok(())
    }

    pub fn gather_all_recipients(&self) -> Result<Vec<x25519::Recipient>> {
        let mut seen_keys = HashSet::new();
        let mut recipients = Vec::new();
        let repo_root = self.get_repo_root().ok();
        let home_key_dirs = self.home_key_dirs_for_repo(repo_root.as_deref());
        let trusted_repo_recipients = self.local_recipient_trust_anchors(&home_key_dirs);
        let no_repo_trust_anchors = HashSet::new();
        let repo_authorization_key = repo_root.as_ref().and_then(|_| self.load_repo_key().ok());

        // 1. Local master identity, when the private key is available on this machine.
        if let Some(master) = self.master_identities.first() {
            let master_pub = master.to_public();
            seen_keys.insert(master_pub.to_string());
            recipients.push(master_pub);
        }

        // 2. Public recipients from the operator's HOME key directory. HOME
        // is an explicit local trust domain, so legacy non-owner names remain
        // supported there; overlapping repository paths are excluded above.
        for keys_dir in &home_key_dirs {
            self.load_public_recipients_from_dir(
                keys_dir,
                &mut seen_keys,
                &mut recipients,
                false,
                &no_repo_trust_anchors,
                None,
            );
        }

        // 3. Imported Heritage Identities
        for id in &self.imported_identities {
            let pub_key = id.to_public();
            let pub_str = pub_key.to_string();
            if seen_keys.insert(pub_str) {
                recipients.push(pub_key);
            }
        }

        // 4. Authorized Machine & Team Keys from the current repo
        if let Some(repo_root) = repo_root {
            // Check BOTH new committed path (V2 Standard) and legacy path.
            // Owner files must match a local trust anchor. Machine/team files
            // additionally require a repo-key-authenticated sidecar generated
            // by their authorization APIs.
            let search_paths = vec![
                repo_root.join(".dracon").join("data").join("keys"), // V2 Standard
                repo_root.join(".git").join("arcane").join("keys"),  // Legacy
            ];

            for keys_dir in search_paths {
                self.load_public_recipients_from_dir(
                    &keys_dir,
                    &mut seen_keys,
                    &mut recipients,
                    true,
                    &trusted_repo_recipients,
                    repo_authorization_key.as_ref(),
                );
            }
        }

        if recipients.is_empty() {
            return Err(anyhow::anyhow!(
                "No master identity or public recipients found for encryption"
            ));
        }

        Ok(recipients)
    }

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

        // 2. Try the authenticated repo-key format (AES-GCM).
        // Legacy AES-CFB is intentionally not attempted: it has no
        // integrity mechanism and can return wrong-key plaintext.
        if let Ok(repo_key) = self.load_repo_key() {
            if let Ok(plaintext) = self.decrypt_with_repo_key(&repo_key, encrypted_data) {
                return Ok(plaintext);
            }
        }

        // 3. Drunk guy with keychain (brute force all keys in ~/.dracon/keys)
        if let Some(plaintext) = self.try_keychain_bruteforce(encrypted_data) {
            return Ok(plaintext);
        }

        // FIXED 2026-08-11 (audit MEDIUM): the old message dumped the
        // first 20 CIPHERTEXT bytes to stderr — a plaintext leak on any
        // machine where stderr is captured by CI/log pipelines. Report
        // only a safe classification (age magic? length).
        Err(anyhow::anyhow!(
            "Decryption failed after trying all keys (V2 + V1 + Keychain). Payload: age-format={}, len={}",
            encrypted_data.starts_with(HEADER_V2_MAGIC),
            encrypted_data.len()
        ))
    }

    pub fn encrypt_v2_for_all(&self, data: &[u8]) -> Result<Vec<u8>> {
        let recipients = self.gather_all_recipients()?;
        let age_recipients: Vec<Box<dyn age::Recipient + Send>> = recipients
            .into_iter()
            .map(|r| Box::new(r) as Box<dyn age::Recipient + Send>)
            .collect();
        self.encrypt_v2(data, age_recipients)
    }

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

    /// Refuse the unauthenticated legacy Git-Seal format.
    ///
    /// AES-CFB has no integrity tag, so no plaintext heuristic can tell a
    /// correct-key result from a wrong-key result: even a short wrong-key
    /// decrypt can be valid UTF-8 and printable. The configuration field
    /// `allow_v1_fallback` is retained for backwards-compatible policy
    /// parsing, but it cannot re-enable this unsafe path. Legacy ciphertexts
    /// must be recovered from a trusted plaintext source and re-encrypted
    /// using authenticated V2 encryption.
    pub fn decrypt_git_seal(
        &self,
        _repo_key: &RepoKey,
        _ciphertext: &[u8],
    ) -> Result<Vec<u8>> {
        Err(anyhow::anyhow!(
            "V1 decryption refused: legacy AES-CFB ciphertext has no authenticated integrity; \
             recover the plaintext from a trusted source and re-encrypt with V2"
        ))
    }
}
