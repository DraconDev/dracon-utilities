mod common;

use common::HomeGuard;
use dracon_security::{RepoKey, REPO_KEY_LEN};
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

#[test]
fn test_repokey_from_file_exact_length() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let key_path = tmp.path().join("key");

    let key_bytes = make_key_bytes();
    fs::write(&key_path, &key_bytes).expect("write key");

    let key = RepoKey::from_file(&key_path).expect("load exact-length key");
    assert_eq!(key.get_key().len(), REPO_KEY_LEN);
}

#[test]
fn test_repokey_from_file_truncated() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let key_path = tmp.path().join("key");

    let key_bytes = make_key_bytes();
    let short = &key_bytes[..16]; // too short
    fs::write(&key_path, short).expect("write truncated key");

    let result = RepoKey::from_file(&key_path);
    assert!(result.is_err(), "truncated key should be rejected");
}

#[test]
fn test_repokey_from_file_overlength() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let key_path = tmp.path().join("key");

    let key_bytes = make_key_bytes();
    let long = [&key_bytes[..], &[1, 2, 3, 4][..]].concat();
    fs::write(&key_path, long).expect("write overlength key");

    let result = RepoKey::from_file(&key_path);
    assert!(result.is_err(), "overlength key should be rejected");
}

#[test]
fn test_repokey_from_file_empty() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let key_path = tmp.path().join("key");

    fs::write(&key_path, b"").expect("write empty key");

    let result = RepoKey::from_file(&key_path);
    assert!(result.is_err(), "empty key file should be rejected");
}

#[test]
fn test_repokey_from_file_nonexistent() {
    let _guard = HomeGuard::new();
    let result = RepoKey::from_file(std::path::Path::new("/nonexistent/key"));
    assert!(result.is_err(), "nonexistent path should return error");
}

#[test]
fn test_encrypt_decrypt_with_repo_key_roundtrip() {
    let (security, _guard) = init_security();
    let key_bytes = make_key_bytes();
    let repo_key = RepoKey(key_bytes);

    let plaintext = b"Hello, World! This is a test message.";
    let encrypted = security
        .encrypt_with_repo_key(&repo_key, plaintext)
        .expect("encrypt");
    assert_ne!(encrypted, plaintext.to_vec());

    let decrypted = security
        .decrypt_with_repo_key(&repo_key, &encrypted)
        .expect("decrypt");
    assert_eq!(decrypted, plaintext.to_vec());
}

#[test]
fn test_encrypt_with_repo_key_empty_plaintext() {
    let (security, _guard) = init_security();
    let key_bytes = make_key_bytes();
    let repo_key = RepoKey(key_bytes);

    let encrypted = security
        .encrypt_with_repo_key(&repo_key, b"")
        .expect("encrypt empty");
    // Decrypt should also work
    let decrypted = security
        .decrypt_with_repo_key(&repo_key, &encrypted)
        .expect("decrypt empty");
    assert_eq!(decrypted, b"");
}

#[test]
fn test_decrypt_with_repo_key_too_short_ciphertext() {
    let (security, _guard) = init_security();
    let key_bytes = make_key_bytes();
    let repo_key = RepoKey(key_bytes);

    // 11 bytes — less than 12-byte nonce
    let result = security.decrypt_with_repo_key(&repo_key, &[0u8; 11]);
    assert!(result.is_err(), "too short ciphertext should error");
}

#[test]
fn test_decrypt_with_repo_key_empty_ciphertext() {
    let (security, _guard) = init_security();
    let key_bytes = make_key_bytes();
    let repo_key = RepoKey(key_bytes);

    let result = security.decrypt_with_repo_key(&repo_key, b"");
    assert!(result.is_err(), "empty ciphertext should error");
}

#[test]
fn test_encrypt_decrypt_with_repo_key_different_keys_produce_different_ciphertext() {
    let (security, _guard) = init_security();
    let key1 = RepoKey(make_key_bytes());
    let key2 = RepoKey(make_key_bytes());

    let plaintext = b"same message";
    let ct1 = security.encrypt_with_repo_key(&key1, plaintext).expect("encrypt1");
    let ct2 = security.encrypt_with_repo_key(&key2, plaintext).expect("encrypt2");

    assert_ne!(ct1, ct2, "different keys should produce different ciphertext");
}

#[test]
fn test_decrypt_with_repo_key_wrong_key_fails() {
    let (security, _guard) = init_security();
    let key1 = RepoKey(make_key_bytes());
    let key2 = RepoKey(make_key_bytes());

    let plaintext = b"secret message";
    let ct1 = security.encrypt_with_repo_key(&key1, plaintext).expect("encrypt");

    let result = security.decrypt_with_repo_key(&key2, &ct1);
    assert!(result.is_err(), "wrong key should fail to decrypt");
}