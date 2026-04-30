mod common;

use common::HomeGuard;
use dracon_security::{RepoKey, REPO_KEY_LEN, TeamKey};
use std::fs;

fn make_key_bytes() -> [u8; 32] {
    let identity = age::x25519::Identity::generate();
    *identity.secret_key().as_bytes()
}

fn init_security() -> (dracon_security::DemonSecurity, HomeGuard) {
    let _guard = HomeGuard::new();
    let mut security = dracon_security::DemonSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);
    (security, _guard)
}

fn init_security_with_repo(
    repo_root: &std::path::Path,
) -> (dracon_security::DemonSecurity, HomeGuard) {
    let _guard = HomeGuard::new();
    let mut security = dracon_security::DemonSecurity::new(Some(repo_root.to_path_buf()))
        .expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);
    (security, _guard)
}

fn make_repo_keys_dir(repo_root: &std::path::Path) -> std::path::PathBuf {
    repo_root
        .join(".git")
        .join("arcane")
        .join("keys")
}

fn write_age_key(path: &std::path::Path, identity: &age::x25519::Identity) {
    fs::create_dir_all(path.parent().unwrap()).expect("create keys dir");
    fs::write(path, identity.to_string()).expect("write age key");
}

fn write_team_key(
    home: &std::path::Path,
    team_name: &str,
    team_identity: &age::x25519::Identity,
    master_identity: &age::x25519::Identity,
) {
    let team_dir = home.join(".demon").join("teams");
    fs::create_dir_all(&team_dir).expect("create team dir");

    let recipients: Vec<Box<dyn age::Recipient + Send>> =
        vec![Box::new(master_identity.to_public())];
    let encryptor = age::Encryptor::with_recipients(recipients)
        .context("failed to create encryptor")
        .expect("encryptor");
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted).expect("wrap");
    writer
        .write_all(team_identity.to_string().as_bytes())
        .expect("write team key");
    writer.finish().expect("finish");

    fs::write(team_dir.join(format!("{}.key", team_name)), encrypted)
        .expect("write team key file");
}

#[test]
fn test_unlock_payload_v1_format() {
    let (security, _guard) = init_security();
    let key_bytes = make_key_bytes();
    let repo_key = RepoKey(key_bytes);

    let plaintext = b"V1 format payload";
    let encrypted = security
        .encrypt_with_repo_key(&repo_key, plaintext)
        .expect("encrypt with repo key");

    let decrypted = security.unlock_payload(&encrypted).expect("unlock v1");
    assert_eq!(decrypted, plaintext.to_vec());
}

#[test]
fn test_unlock_payload_too_short_for_aes_gcm() {
    let (security, _guard) = init_security();

    // 11 bytes — too short for 12-byte nonce, should fail gracefully
    let result = security.unlock_payload(&[0u8; 11]);
    assert!(result.is_err(), "too short payload should fail");
}

#[test]
fn test_unlock_payload_wrong_key_produces_error() {
    let (security, _guard) = init_security();
    let key1 = RepoKey(make_key_bytes());
    let key2 = RepoKey(make_key_bytes());

    let plaintext = b"secret payload";
    let encrypted = security
        .encrypt_with_repo_key(&key1, plaintext)
        .expect("encrypt");

    // Unlock with security that has key2, not key1
    let result = security.unlock_payload(&encrypted);
    assert!(result.is_err(), "wrong key should fail unlock");
}

#[test]
fn test_unlock_payload_empty_data() {
    let (security, _guard) = init_security();

    let result = security.unlock_payload(b"");
    assert!(result.is_err(), "empty payload should fail");
}

#[test]
fn test_team_key_to_public_valid() {
    let _guard = HomeGuard::new();
    let identity = age::x25519::Identity::generate();
    let identity_str = identity.to_string();

    let team_key = TeamKey(identity_str.into_bytes());
    let recipient = team_key.to_public().expect("to_public");
    assert_eq!(recipient.to_string(), identity.to_public().to_string());
}

#[test]
fn test_team_key_to_public_invalid_utf8() {
    let _guard = HomeGuard::new();
    // 32 bytes that are not valid UTF-8
    let mut invalid_bytes = vec![0x80u8; 32];
    invalid_bytes[0] = b'A'; // make first byte valid to not trigger UTF-8 error at string level
    invalid_bytes[1] = 0x80; // invalid UTF-8 continuation

    let team_key = TeamKey(invalid_bytes);
    let result = team_key.to_public();
    assert!(result.is_err(), "invalid UTF-8 should produce error");
}

#[test]
fn test_team_key_to_public_not_identity_string() {
    let _guard = HomeGuard::new();
    // Valid UTF-8 but not a valid x25519 identity string
    let not_identity = b"age1notavalididentitystringxxxxxxx".to_vec();
    let team_key = TeamKey(not_identity);
    let result = team_key.to_public();
    assert!(result.is_err(), "non-identity string should produce error");
}

#[test]
fn test_team_key_len() {
    let identity = age::x25519::Identity::generate();
    let team_key = TeamKey(identity.to_string().into_bytes());
    assert_eq!(team_key.len(), identity.to_string().len());
}

#[test]
fn test_team_key_is_empty() {
    let team_key = TeamKey(Vec::new());
    assert!(team_key.is_empty());
}