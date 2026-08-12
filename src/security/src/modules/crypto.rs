//! Encryption and decryption operations.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use age::x25519;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::RepoKey;
use crate::WardenSecurity;

const HEADER_V2_MAGIC: &[u8] = b"age-encryption.org/v1";

/// Mirrors `is_owner_pubkey_filename` in the warden binary (main.rs):
/// the canonical mesh files written by keygen/publish are `owner_*.pub`.
/// This crate is published separately, so the predicate is duplicated.
fn is_owner_pubkey_filename(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.starts_with("owner_") && name.ends_with(".pub")
}

/// Warn-once-per-process eprintln (a gather runs on every protected-file
/// clean; repeating per file would spam the journal).
fn warn_once(flag: &AtomicBool, msg: &str) {
    if !flag.swap(true, Ordering::Relaxed) {
        eprintln!("⚠️ warden: {}", msg);
    }
}

static WARNED_NON_OWNER_REPO_FILE: AtomicBool = AtomicBool::new(false);
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
    ) {
        if !keys_dir.exists() {
            return;
        }

        let Ok(entries) = fs::read_dir(keys_dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext != "pub" && ext != "key" {
                continue;
            }

            // FIXED 2026-08-11 (audit MEDIUM): any `.pub`/`.key` file
            // was trusted as a recipient source. In REPO key dirs
            // (`.dracon/data/keys`, `.git/arcane/keys` — the
            // contributor-pushable surface) only the canonical
            // `owner_*.pub` mesh files written by keygen/publish
            // (main.rs publish_repo_pubkey) are honored; anything else
            // is refused with a warning instead of silently joining
            // every future encryption. The operator's HOME key dir is
            // its own trust domain: files there stay honored (parsed
            // per line, see below) but a warning flags non-canonical
            // names.
            if require_owner_naming && !is_owner_pubkey_filename(&path) {
                warn_once(
                    &WARNED_NON_OWNER_REPO_FILE,
                    &format!(
                        "ignoring recipient file {} (not owner_*.pub); \
only canonical mesh keys are honored in repo key dirs",
                        path.display()
                    ),
                );
                continue;
            }

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

            for line in pub_str.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if !seen_keys.insert(line.to_string()) {
                    continue;
                }
                if let Ok(recipient) = line.parse::<x25519::Recipient>() {
                    recipients.push(recipient);
                }
            }
        }
    }

    pub fn gather_all_recipients(&self) -> Result<Vec<x25519::Recipient>> {
        let mut seen_keys = HashSet::new();
        let mut recipients = Vec::new();

        // 1. Local master identity, when the private key is available on this machine.
        if let Some(master) = self.master_identities.first() {
            let master_pub = master.to_public();
            seen_keys.insert(master_pub.to_string());
            recipients.push(master_pub);
        }

        // 2. Canonical mesh recipients from ~/.dracon/data/keys/*.pub. This keeps
        // encryption aligned with the documented mesh even when the owner/master
        // private key is stored off-box and only the public recipient is present.
        // FIXED 2026-08-11 (audit MEDIUM): the home dir is the operator's own
        // trust domain — non-owner_* files remain honored (micro2_*, master.pub
        // are legitimate mesh additions) but load_public_recipients_from_dir
        // flags them once. Repo key dirs below are strict (owner_*.pub only).
        if let Ok(home) = self.get_home() {
            self.load_public_recipients_from_dir(
                &home.join(".dracon").join("data").join("keys"),
                &mut seen_keys,
                &mut recipients,
                false,
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
        if let Ok(repo_root) = self.get_repo_root() {
            // Check BOTH new committed path (V2 Standard) and legacy path
            // FIXED 2026-08-11 (audit MEDIUM): repo key dirs are the
            // contributor-pushable surface — require the canonical
            // `owner_*.pub` naming there so a pushed `evil.pub` can no
            // longer add its holder to every future encryption.
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

        // 2. Try RepoKey (V1) format - if we have a RepoKey
        if let Ok(repo_key) = self.load_repo_key() {
            if let Ok(plaintext) = self.decrypt_with_repo_key(&repo_key, encrypted_data) {
                return Ok(plaintext);
            }
            if let Ok(plaintext) = self.decrypt_git_seal(&repo_key, encrypted_data) {
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
