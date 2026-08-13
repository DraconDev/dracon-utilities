//! Encryption and decryption operations.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use age::x25519;
use anyhow::{Context, Result};
use curve25519_dalek::montgomery::MontgomeryPoint;
use ed25519_dalek::hazmat::{raw_sign, ExpandedSecretKey};
use ed25519_dalek::{Signature, VerifyingKey};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::RepoKey;
use crate::WardenSecurity;

const HEADER_V2_MAGIC: &[u8] = b"age-encryption.org/v1";
const RECIPIENT_AUTH_VERSION: u8 = 2;
const RECIPIENT_AUTH_ROLE_DIRECT: &str = "direct";
const RECIPIENT_AUTH_ROLE_MACHINE: &str = "machine";
const RECIPIENT_AUTH_ROLE_TEAM: &str = "team";
const MAX_RECIPIENT_AUTH_BYTES: usize = 4096;

#[derive(Debug, Deserialize, Serialize)]
struct RepoRecipientAuthorization {
    version: u8,
    role: String,
    file_name: String,
    recipient: String,
    #[serde(default)]
    repo_key_commitment: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OwnerRecipientAuthorization {
    signer: String,
    signature: Vec<u8>,
    ciphertext: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RepoRecipientAuthorizationEnvelope {
    version: u8,
    repo_key_ciphertext: Vec<u8>,
    owner_proofs: Vec<OwnerRecipientAuthorization>,
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
pub(crate) fn parse_single_repo_recipient(content: &str) -> Option<x25519::Recipient> {
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

fn decode_identity_bytes(identity: &x25519::Identity) -> Option<[u8; 32]> {
    let identity_text = identity.to_string();
    let (identity_hrp, identity_bytes) = bech32::decode(identity_text.expose_secret()).ok()?;
    if !identity_hrp.to_string().eq_ignore_ascii_case("age-secret-key-") {
        return None;
    }
    identity_bytes.try_into().ok()
}

fn decode_recipient_bytes(recipient: &x25519::Recipient) -> Option<[u8; 32]> {
    let recipient_text = recipient.to_string();
    let (recipient_hrp, recipient_bytes) = bech32::decode(&recipient_text).ok()?;
    if recipient_hrp.to_string() != "age" {
        return None;
    }
    recipient_bytes.try_into().ok()
}

/// Derive a proof-encryption key from an age X25519 identity and recipient.
/// This is only a confidential transport for the authorization payload; it is
/// not the authentication mechanism. Authentication is provided by the
/// owner signature below because either DH participant can derive this key.
fn recipient_proof_key(
    identity: &x25519::Identity,
    recipient: &x25519::Recipient,
) -> Option<RepoKey> {
    let identity_bytes = decode_identity_bytes(identity)?;
    let recipient_bytes = decode_recipient_bytes(recipient)?;
    let secret = StaticSecret::from(identity_bytes);
    let public = PublicKey::from(recipient_bytes);
    let shared = secret.diffie_hellman(&public);
    if shared.as_bytes().iter().all(|byte| *byte == 0) {
        return None;
    }
    let mut hasher = Sha256::new();
    hasher.update(b"dracon-warden recipient authorization v1");
    hasher.update(shared.as_bytes());
    Some(RepoKey(hasher.finalize().to_vec()))
}

/// Build an Ed25519 signing key whose scalar is the same clamped scalar used
/// by the age X25519 identity. Curve25519's Montgomery and Edwards forms share
/// the base point, so the public key can be recovered from the age recipient
/// alone. The domain-separated prefix keeps deterministic EdDSA nonces from
/// reusing any age protocol hash state.
fn owner_signature_key(
    identity: &x25519::Identity,
) -> Option<(ExpandedSecretKey, VerifyingKey)> {
    let identity_bytes = decode_identity_bytes(identity)?;
    let mut expanded_bytes = [0u8; 64];
    expanded_bytes[..32].copy_from_slice(&identity_bytes);
    let mut prefix_hasher = Sha512::new();
    prefix_hasher.update(b"dracon-warden owner authorization signature prefix v1");
    prefix_hasher.update(identity_bytes);
    expanded_bytes[32..].copy_from_slice(&prefix_hasher.finalize()[..32]);
    let expanded = ExpandedSecretKey::from_bytes(&expanded_bytes);
    let verifying_key = VerifyingKey::from(&expanded);
    Some((expanded, verifying_key))
}

fn owner_signature(
    identity: &x25519::Identity,
    payload: &[u8],
) -> Option<Vec<u8>> {
    let (expanded, verifying_key) = owner_signature_key(identity)?;
    Some(raw_sign::<Sha512>(&expanded, payload, &verifying_key).to_bytes().to_vec())
}

fn owner_signature_is_valid(
    signer: &x25519::Recipient,
    payload: &[u8],
    signature_bytes: &[u8],
) -> bool {
    let Ok(signature_bytes) = <[u8; 64]>::try_from(signature_bytes) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    let Some(recipient_bytes) = decode_recipient_bytes(signer) else {
        return false;
    };
    let montgomery = MontgomeryPoint(recipient_bytes);
    [0u8, 1u8].into_iter().any(|sign| {
        montgomery
            .to_edwards(sign)
            .map(VerifyingKey::from)
            .is_some_and(|verifying_key| verifying_key.verify_strict(payload, &signature).is_ok())
    })
}

fn authorization_matches(
    auth: &RepoRecipientAuthorization,
    public_path: &Path,
    recipient: &x25519::Recipient,
    required_role: Option<&str>,
    expected_repo_key: Option<&RepoKey>,
) -> bool {
    let Some(file_name) = public_path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    auth.version == RECIPIENT_AUTH_VERSION
        && required_role.is_none_or(|role| auth.role == role)
        && expected_repo_key.is_some_and(|repo_key| {
            auth.repo_key_commitment == repo_key_commitment(repo_key)
        })
        && matches!(
            auth.role.as_str(),
            RECIPIENT_AUTH_ROLE_DIRECT
                | RECIPIENT_AUTH_ROLE_MACHINE
                | RECIPIENT_AUTH_ROLE_TEAM
        )
        && auth.file_name == file_name
        && auth.recipient == recipient.to_string()
}

fn repo_key_commitment(repo_key: &RepoKey) -> Vec<u8> {
    Sha256::digest(&repo_key.0).to_vec()
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

#[cfg(test)]
mod authorization_tests {
    use super::*;
    use std::fs;

    fn encrypt_for_age_recipient(
        recipient: &x25519::Recipient,
        plaintext: &[u8],
    ) -> Vec<u8> {
        let encryptor = age::Encryptor::with_recipients(vec![Box::new(recipient.clone())])
            .expect("create age encryptor");
        let mut ciphertext = Vec::new();
        let mut writer = encryptor
            .wrap_output(&mut ciphertext)
            .expect("wrap age output");
        writer.write_all(plaintext).expect("write age plaintext");
        writer.finish().expect("finish age output");
        ciphertext
    }

    #[test]
    fn delegated_dh_cannot_forge_owner_authorization_signature() {
        let tmp = tempfile::tempdir().expect("create temp directory");
        let repo_root = tmp.path().join("repo");
        let home = tmp.path().join("home");
        let keys_dir = repo_root.join(".git/arcane/keys");
        let home_keys_dir = home.join(".dracon/data/keys");
        fs::create_dir_all(&keys_dir).expect("create repository keys");
        fs::create_dir_all(&home_keys_dir).expect("create home keys");

        let owner = x25519::Identity::generate();
        let delegated = x25519::Identity::generate();
        let owner_recipient = owner.to_public();
        let delegated_recipient = delegated.to_public();
        fs::write(
            home_keys_dir.join("owner_operator.pub"),
            owner_recipient.to_string(),
        )
        .expect("write owner trust anchor");

        let forged_repo_key_bytes: [u8; 32] = rand::random();
        fs::write(
            keys_dir.join("repo.key.age"),
            encrypt_for_age_recipient(&owner_recipient, &forged_repo_key_bytes),
        )
        .expect("write forged canonical repo key");
        fs::write(
            keys_dir.join("evil.pub"),
            delegated_recipient.to_string(),
        )
        .expect("write delegated recipient");
        fs::write(keys_dir.join("evil.age"), b"delegation")
            .expect("write delegated age marker");

        let auth = RepoRecipientAuthorization {
            version: RECIPIENT_AUTH_VERSION,
            role: RECIPIENT_AUTH_ROLE_DIRECT.to_string(),
            file_name: "evil.pub".to_string(),
            recipient: delegated_recipient.to_string(),
            repo_key_commitment: repo_key_commitment(&RepoKey(
                forged_repo_key_bytes.to_vec(),
            )),
        };
        let payload = serde_json::to_vec(&auth).expect("serialize authorization");
        let forged_repo_key = RepoKey(forged_repo_key_bytes.to_vec());
        let repo_key_ciphertext = {
            let security = WardenSecurity::new(Some(&repo_root)).expect("init security");
            security
                .encrypt_with_repo_key(&forged_repo_key, &payload)
                .expect("encrypt repository authorization")
        };

        // The attacker can derive the mutual-DH transport key with its own
        // delegated private key and the trusted owner's public key. It can
        // also sign with its own unrelated scalar, while labeling the proof
        // as if the owner signed it. The signature must be rejected.
        let proof_key = recipient_proof_key(&delegated, &owner_recipient)
            .expect("derive attacker-visible DH transport key");
        let proof_ciphertext = {
            let security = WardenSecurity::new(Some(&repo_root)).expect("init security");
            security
                .encrypt_with_repo_key(&proof_key, &payload)
                .expect("encrypt forged owner proof payload")
        };
        let forged_signature = owner_signature(&delegated, &payload)
            .expect("derive attacker signature");
        let envelope = RepoRecipientAuthorizationEnvelope {
            version: RECIPIENT_AUTH_VERSION,
            repo_key_ciphertext,
            owner_proofs: vec![OwnerRecipientAuthorization {
                signer: owner_recipient.to_string(),
                signature: forged_signature,
                ciphertext: proof_ciphertext,
            }],
        };
        fs::write(
            keys_dir.join("evil.auth"),
            serde_json::to_vec(&envelope).expect("serialize forged authorization envelope"),
        )
        .expect("write forged authorization envelope");

        let mut security = WardenSecurity::new(Some(&repo_root)).expect("init security");
        security.set_mock_home(home);
        security.add_memory_identity(owner);
        let recipients = security
            .gather_all_recipients()
            .expect("gather recipients without failing closed");
        assert!(!recipients
            .iter()
            .any(|recipient| recipient == &delegated_recipient));
    }
}

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
                // Canonical names are reserved for owner/master trust
                // anchors. Noncanonical names are accepted only through an
                // authenticated delegated/direct proof; a trusted owner
                // recipient copied into `evil.pub` is not a valid exception.
                let accepted = if canonical_name {
                    local_owner
                } else {
                    authenticated_machine_or_team
                };
                if !accepted {
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

    fn read_authorization_envelope(
        &self,
        public_path: &Path,
    ) -> Option<RepoRecipientAuthorizationEnvelope> {
        let auth_path = recipient_authorization_path(public_path);
        let metadata = fs::symlink_metadata(&auth_path).ok()?;
        if !metadata.file_type().is_file() {
            return None;
        }
        let ciphertext = fs::read(auth_path).ok()?;
        if ciphertext.len() > MAX_RECIPIENT_AUTH_BYTES {
            return None;
        }
        serde_json::from_slice(&ciphertext).ok()
    }

    /// Verify the signature over an authorization payload and require its
    /// signer to be a local owner trust anchor. The DH ciphertext in an owner
    /// proof only lets a delegated reader recover the payload; it is not used
    /// as authentication because either DH participant can forge it.
    fn trusted_owner_signature_matches(
        &self,
        public_path: &Path,
        recipient: &x25519::Recipient,
        auth: &RepoRecipientAuthorization,
        owner_proof: &OwnerRecipientAuthorization,
        repo_key: &RepoKey,
        required_role: Option<&str>,
    ) -> bool {
        let repo_root = self.get_repo_root().ok();
        let home_dirs = self.home_key_dirs_for_repo(repo_root.as_deref());
        let trusted = self.local_recipient_trust_anchors(&home_dirs);
        if !trusted.contains(&owner_proof.signer) {
            return false;
        }
        let Ok(signer) = owner_proof.signer.parse::<x25519::Recipient>() else {
            return false;
        };
        if !authorization_matches(auth, public_path, recipient, required_role, Some(repo_key)) {
            return false;
        }
        let Ok(payload) = serde_json::to_vec(auth) else {
            return false;
        };
        owner_signature_is_valid(&signer, &payload, &owner_proof.signature)
    }

    /// Verify a V2 owner-authenticated authorization sidecar written by
    /// `whitelist_machine`, `add_team_member`, or `authorize_recipient`. The
    /// sidecar binds the exact basename, recipient, and repository-key
    /// commitment, so copying it cannot authorize a different file or key.
    /// V1 sidecars are rejected because their repo-key-only proof could be
    /// forged by replacing an unauthenticated canonical repo-key ciphertext.
    pub(crate) fn verify_repo_recipient_authorization(
        &self,
        public_path: &Path,
        recipient: &x25519::Recipient,
        repo_key: &RepoKey,
    ) -> bool {
        let auth_path = recipient_authorization_path(public_path);
        let Ok(auth_metadata) = fs::symlink_metadata(&auth_path) else {
            return false;
        };
        if !auth_metadata.file_type().is_file() {
            return false;
        }
        let Ok(auth_bytes) = fs::read(&auth_path) else {
            return false;
        };
        if auth_bytes.len() > MAX_RECIPIENT_AUTH_BYTES {
            return false;
        }

        // Pre-envelope V1 sidecars are deliberately rejected. They bind only
        // to the repository-key ciphertext, so a contributor who can replace
        // a canonical `repo.key.age` blob with one encrypted to the owner's
        // public key could choose the plaintext key and forge a V1 sidecar.
        // Existing entries must be explicitly re-authorized; new output is a
        // V2 envelope with an owner signature and a DH-encrypted copy of the
        // authorization payload.
        let Some(envelope) = serde_json::from_slice::<RepoRecipientAuthorizationEnvelope>(&auth_bytes).ok() else {
            return false;
        };
        if envelope.version != RECIPIENT_AUTH_VERSION {
            return false;
        }
        let Ok(plaintext) = self.decrypt_with_repo_key(repo_key, &envelope.repo_key_ciphertext) else {
            return false;
        };
        let Ok(auth) = serde_json::from_slice::<RepoRecipientAuthorization>(&plaintext) else {
            return false;
        };
        envelope.owner_proofs.iter().any(|owner_proof| {
            self.trusted_owner_signature_matches(
                public_path,
                recipient,
                &auth,
                owner_proof,
                repo_key,
                None,
            )
        })
    }

    /// Verify an owner-signed proof when the local process only has a
    /// delegated identity. The repo key cannot be its own trust anchor: a
    /// contributor could otherwise forge both an arbitrary `.age` blob and a
    /// proof under that chosen value. The owner public recipient is the trust
    /// anchor, and the signature binds the proof to that owner.
    pub(crate) fn verify_owner_recipient_authorization(
        &self,
        public_path: &Path,
        recipient: &x25519::Recipient,
        delegated_identity: &x25519::Identity,
        repo_key: &RepoKey,
        required_role: &str,
    ) -> bool {
        let Some(envelope) = self.read_authorization_envelope(public_path) else {
            return false;
        };
        if envelope.version != RECIPIENT_AUTH_VERSION {
            return false;
        }
        let repo_root = self.get_repo_root().ok();
        let home_dirs = self.home_key_dirs_for_repo(repo_root.as_deref());
        let trusted = self.local_recipient_trust_anchors(&home_dirs);

        for owner_proof in envelope.owner_proofs {
            if !trusted.contains(&owner_proof.signer) {
                continue;
            }
            let Ok(owner_public) = owner_proof.signer.parse::<x25519::Recipient>() else {
                continue;
            };
            let Some(proof_key) = recipient_proof_key(delegated_identity, &owner_public) else {
                continue;
            };
            let Ok(plaintext) = self.decrypt_with_repo_key(&proof_key, &owner_proof.ciphertext)
            else {
                continue;
            };
            let Ok(auth) = serde_json::from_slice::<RepoRecipientAuthorization>(&plaintext) else {
                continue;
            };
            if self.trusted_owner_signature_matches(
                public_path,
                recipient,
                &auth,
                &owner_proof,
                repo_key,
                Some(required_role),
            ) {
                return true;
            }
        }
        false
    }

    pub(crate) fn verify_machine_recipient_authorization(
        &self,
        public_path: &Path,
        recipient: &x25519::Recipient,
        machine_identity: &x25519::Identity,
        repo_key: &RepoKey,
    ) -> bool {
        self.verify_owner_recipient_authorization(
            public_path,
            recipient,
            machine_identity,
            repo_key,
            RECIPIENT_AUTH_ROLE_MACHINE,
        )
    }

    pub(crate) fn write_repo_public_recipient(
        &self,
        public_path: &Path,
        recipient: &x25519::Recipient,
    ) -> Result<()> {
        let parent = public_path
            .parent()
            .context("recipient public key has no parent directory")?;
        let parent_metadata = fs::symlink_metadata(parent)
            .with_context(|| format!("inspect recipient directory {}", parent.display()))?;
        if !parent_metadata.file_type().is_dir() {
            anyhow::bail!("recipient public-key directory is not a regular directory")
        }
        if let Ok(metadata) = fs::symlink_metadata(public_path) {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                anyhow::bail!("refusing to overwrite non-regular recipient public key")
            }
            anyhow::bail!("recipient public key already exists")
        }
        let bytes = format!("{}\n", recipient);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o644)
                .open(public_path)?
                .write_all(bytes.as_bytes())?;
        }
        #[cfg(not(unix))]
        {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(public_path)?
                .write_all(bytes.as_bytes())?;
        }
        Ok(())
    }

    /// Write a V2 authorization envelope for a machine/team/direct recipient.
    /// The repo-key ciphertext and DH ciphertext provide confidentiality, but
    /// the owner signature is the authentication boundary: a delegated
    /// recipient can derive the DH key, while only the owner can sign.
    pub(crate) fn write_repo_recipient_authorization(
        &self,
        repo_key: &RepoKey,
        public_path: &Path,
        role: &str,
        recipient: &x25519::Recipient,
    ) -> Result<()> {
        if !matches!(
            role,
            RECIPIENT_AUTH_ROLE_DIRECT
                | RECIPIENT_AUTH_ROLE_MACHINE
                | RECIPIENT_AUTH_ROLE_TEAM
        ) {
            anyhow::bail!("invalid repository recipient authorization role")
        }
        if self.master_identities.is_empty() {
            anyhow::bail!("owner identity required to authorize repository recipients")
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
            repo_key_commitment: repo_key_commitment(repo_key),
        };
        let payload = serde_json::to_vec(&auth).context("serialize recipient authorization")?;
        let repo_key_ciphertext = self.encrypt_with_repo_key(repo_key, &payload)?;
        let mut owner_proofs = Vec::new();
        for signer in &self.master_identities {
            let Some(proof_key) = recipient_proof_key(signer, recipient) else {
                continue;
            };
            let Some(signature) = owner_signature(signer, &payload) else {
                continue;
            };
            owner_proofs.push(OwnerRecipientAuthorization {
                signer: signer.to_public().to_string(),
                signature,
                ciphertext: self.encrypt_with_repo_key(&proof_key, &payload)?,
            });
        }
        if owner_proofs.is_empty() {
            anyhow::bail!("failed to derive owner authorization proof")
        }
        let envelope = RepoRecipientAuthorizationEnvelope {
            version: RECIPIENT_AUTH_VERSION,
            repo_key_ciphertext,
            owner_proofs,
        };
        let encoded = serde_json::to_vec(&envelope).context("serialize authorization envelope")?;
        if encoded.len() > MAX_RECIPIENT_AUTH_BYTES {
            anyhow::bail!("recipient authorization envelope is too large")
        }
        let auth_path = recipient_authorization_path(public_path);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&auth_path)
                .with_context(|| format!("create recipient authorization {}", auth_path.display()))?
                .write_all(&encoded)?;
        }
        #[cfg(not(unix))]
        {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&auth_path)
                .with_context(|| format!("create recipient authorization {}", auth_path.display()))?
                .write_all(&encoded)?;
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
