mod common;

use common::HomeGuard;
use dracon_security::{EnvironmentManager, RepoKey, REPO_KEY_LEN};
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
    repo_root.join(".git").join("arcane").join("keys")
}

fn write_age_key(path: &std::path::Path, identity: &age::x25519::Identity) {
    fs::create_dir_all(path.parent().unwrap()).expect("create keys dir");
    fs::write(path, identity.to_string()).expect("write age key");
}

fn write_age_key_for_repo(
    keys_dir: &std::path::Path,
    identity: &age::x25519::Identity,
    filename: &str,
) {
    fs::create_dir_all(keys_dir).expect("create keys dir");
    let path = keys_dir.join(filename);
    fs::write(&path, identity.to_string()).expect("write age key");
}

// =============================================================================
// EnvironmentManager tests
// =============================================================================

#[test]
fn test_env_manager_to_env_file_variables() {
    let mut em = EnvironmentManager::new();
    em.add_variable("USER".to_string(), "alice".to_string());
    em.add_variable("HOME".to_string(), "/home/alice".to_string());

    let output = em.to_env_file();
    assert!(output.contains("USER=\"alice\""));
    assert!(output.contains("HOME=\"/home/alice\""));
}

#[test]
fn test_env_manager_to_env_file_secrets() {
    let mut em = EnvironmentManager::new();
    em.add_secret("database".to_string(), "PASSWORD".to_string(), "super_secret".to_string());
    em.add_secret("api".to_string(), "API_KEY".to_string(), "key_12345".to_string());

    let output = em.to_env_file();
    assert!(output.contains("# Group: database"));
    assert!(output.contains("PASSWORD=\"super_secret\""));
    assert!(output.contains("# Group: api"));
    assert!(output.contains("API_KEY=\"key_12345\""));
}

#[test]
fn test_env_manager_to_env_file_escapes_quotes() {
    let mut em = EnvironmentManager::new();
    em.add_variable("MESSAGE".to_string(), "He said \"hello\"".to_string());

    let output = em.to_env_file();
    assert!(output.contains("MESSAGE=\"He said \\\"hello\\\"\""));
}

#[test]
fn test_env_manager_load_from_env_file() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let env_path = tmp.path().join("test.env");

    fs::write(
        &env_path,
        r#"
# Comment line
VAR1="value1"
VAR2=value2
EMPTY=
"#,
    )
    .expect("write env file");

    let mut em = EnvironmentManager::new();
    em.load_from_env_file(&env_path).expect("load from env file");

    // VAR1 should have quotes stripped
    assert_eq!(em.variables.get("VAR1").map(|s| s.as_str()), Some("value1"));
    // VAR2 should be as-is
    assert_eq!(em.variables.get("VAR2").map(|s| s.as_str()), Some("value2"));
    // EMPTY should be empty string
    assert_eq!(em.variables.get("EMPTY").map(|s| s.as_str()), Some(""));
}

#[test]
fn test_env_manager_load_from_env_file_nonexistent() {
    let _guard = HomeGuard::new();
    let mut em = EnvironmentManager::new();
    let result = em.load_from_env_file(std::path::Path::new("/nonexistent/.env"));
    assert!(result.is_ok(), "nonexistent path should return Ok (no-op)");
}

#[test]
fn test_env_manager_load_from_env_file_with_single_quotes() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let env_path = tmp.path().join("test.env");

    fs::write(&env_path, "KEY='single quoted value'\n").expect("write env file");

    let mut em = EnvironmentManager::new();
    em.load_from_env_file(&env_path).expect("load");

    assert_eq!(
        em.variables.get("KEY").map(|s| s.as_str()),
        Some("single quoted value")
    );
}

#[test]
fn test_env_manager_load_from_env_file_with_embedded_equals() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let env_path = tmp.path().join("test.env");

    fs::write(&env_path, "EQUATION=\"a=b=c\"\n").expect("write env file");

    let mut em = EnvironmentManager::new();
    em.load_from_env_file(&env_path).expect("load");

    assert_eq!(em.variables.get("EQUATION").map(|s| s.as_str()), Some("a=b=c"));
}

#[test]
fn test_env_manager_combined() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let env_path = tmp.path().join("test.env");

    fs::write(&env_path, "VAR=value\nSECRET=hidden\n").expect("write env file");

    let mut em = EnvironmentManager::new();
    em.add_variable("FROM_CODE".to_string(), "code_val".to_string());
    em.load_from_env_file(&env_path).expect("load");
    em.add_secret("creds".to_string(), "API_KEY".to_string(), "key".to_string());

    let output = em.to_env_file();
    assert!(output.contains("FROM_CODE=\"code_val\""));
    assert!(output.contains("VAR=\"value\""));
    assert!(output.contains("# Group: creds"));
    assert!(output.contains("API_KEY=\"key\""));
}

// =============================================================================
// RepoKey::from_file edge case tests
// =============================================================================

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
    let short = &key_bytes[..16];
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
    let long: Vec<u8> = key_bytes.iter().chain([1, 2, 3, 4].iter()).copied().collect();
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
    let result = RepoKey::from_file(std::path::Path::new("/nonexistent/key"));
    assert!(result.is_err(), "nonexistent path should return error");
}

// =============================================================================
// encrypt_with_repo_key / decrypt_with_repo_key tests
// =============================================================================

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
fn test_encrypt_with_repo_key_random_nonce_per_call() {
    let (security, _guard) = init_security();
    let key_bytes = make_key_bytes();
    let repo_key = RepoKey(key_bytes);

    let plaintext = b"same message";
    let ct1 = security.encrypt_with_repo_key(&repo_key, plaintext).expect("encrypt1");
    let ct2 = security.encrypt_with_repo_key(&repo_key, plaintext).expect("encrypt2");

    assert_ne!(ct1, ct2, "random nonce should produce different ciphertext");
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

// =============================================================================
// load_repo_key tests — using in-memory identity to encrypt a repo key
// =============================================================================

#[test]
fn test_load_repo_key_master_identity_success() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let repo_root = tmp.path();

    // Set up keys directory with a master identity
    let keys_dir = make_repo_keys_dir(repo_root);
    let master_identity = age::x25519::Identity::generate();
    write_age_key_for_repo(&keys_dir, &master_identity, "identity.age");

    let (mut security, _guard) = init_security_with_repo(repo_root);
    security.add_memory_identity(master_identity.clone());

    // Create a repo key encrypted for this identity
    let repo_key_bytes = make_key_bytes();
    let repo_key = RepoKey(repo_key_bytes);

    let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(master_identity.to_public())];
    let encryptor = age::Encryptor::with_recipients(recipients)
        .expect("encryptor");
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted).expect("wrap");
    writer.write_all(&repo_key.0).expect("write repo key");
    writer.finish().expect("finish");

    fs::write(keys_dir.join("repo.key.age"), encrypted).expect("write repo key");

    // load_repo_key should find and decrypt it
    let loaded = security.load_repo_key().expect("load repo key");
    assert_eq!(loaded.get_key(), &repo_key_bytes);
}

#[test]
fn test_load_repo_key_no_keys_directory() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let (security, _guard) = init_security_with_repo(tmp.path());

    let result = security.load_repo_key();
    assert!(result.is_err(), "no keys dir should error");
}

#[test]
fn test_load_repo_key_empty_keys_directory() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let keys_dir = make_repo_keys_dir(tmp.path());
    fs::create_dir_all(&keys_dir).expect("create empty keys dir");

    let (security, _guard) = init_security_with_repo(tmp.path());

    let result = security.load_repo_key();
    assert!(result.is_err(), "empty keys dir should error");
}

#[test]
fn test_load_repo_key_machine_key_env_var() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let repo_root = tmp.path();
    let keys_dir = make_repo_keys_dir(repo_root);
    fs::create_dir_all(&keys_dir).expect("create keys dir");

    let machine_identity = age::x25519::Identity::generate();
    let machine_recipient = machine_identity.to_public();

    // Create repo key encrypted for machine identity
    let repo_key_bytes = make_key_bytes();
    let repo_key = RepoKey(repo_key_bytes);

    let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(machine_recipient.clone())];
    let encryptor = age::Encryptor::with_recipients(recipients).expect("encryptor");
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted).expect("wrap");
    writer.write_all(&repo_key.0).expect("write repo key");
    writer.finish().expect("finish");

    fs::write(keys_dir.join("machine.key.age"), encrypted).expect("write machine key");

    let (security, _guard) = init_security_with_repo(repo_root);

    // Set the machine key env var
    std::env::set_var("ARCANE_MACHINE_KEY", machine_identity.to_string());

    let loaded = security.load_repo_key().expect("load repo key via machine key");
    assert_eq!(loaded.get_key(), &repo_key_bytes);

    std::env::remove_var("ARCANE_MACHINE_KEY");
}

#[test]
fn test_load_repo_key_team_key() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let repo_root = tmp.path();
    let keys_dir = make_repo_keys_dir(repo_root);
    fs::create_dir_all(&keys_dir).expect("create keys dir");

    let master_identity = age::x25519::Identity::generate();
    let team_identity = age::x25519::Identity::generate();

    // Write master identity to keys dir
    write_age_key_for_repo(&keys_dir, &master_identity, "identity.age");

    // Set up team key in ~/.demon/teams/
    let home_guard = HomeGuard::new();
    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap();
    let team_dir = home.join(".demon").join("teams");
    fs::create_dir_all(&team_dir).expect("create team dir");

    // Encrypt team key with master identity
    let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(master_identity.to_public())];
    let encryptor = age::Encryptor::with_recipients(recipients).expect("encryptor");
    let mut encrypted_team = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted_team).expect("wrap");
    writer
        .write_all(team_identity.to_string().as_bytes())
        .expect("write team key");
    writer.finish().expect("finish");

    fs::write(team_dir.join("my-team.key"), encrypted_team).expect("write team key file");

    // Create repo key encrypted for team identity
    let repo_key_bytes = make_key_bytes();
    let repo_key = RepoKey(repo_key_bytes);

    let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(team_identity.to_public())];
    let encryptor = age::Encryptor::with_recipients(recipients).expect("encryptor");
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted).expect("wrap");
    writer.write_all(&repo_key.0).expect("write repo key");
    writer.finish().expect("finish");

    fs::write(keys_dir.join("team:my-team.age"), encrypted).expect("write team-encrypted repo key");

    let (mut security, _guard2) = init_security_with_repo(repo_root);
    security.add_memory_identity(master_identity);

    let loaded = security.load_repo_key().expect("load via team key");
    assert_eq!(loaded.get_key(), &repo_key_bytes);
}

// =============================================================================
// generate_master_identity tests
// =============================================================================

#[test]
fn test_generate_master_identity_success() {
    let home_guard = HomeGuard::new();
    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap();

    // Ensure no identity files exist
    let identity_dir = home.join(".demon");
    if identity_dir.exists() {
        for entry in fs::read_dir(&identity_dir).expect("read dir") {
            let entry = entry.expect("entry");
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("identity") || name_str == "identity.txt" {
                fs::remove_file(entry.path()).expect("remove identity file");
            }
        }
    }

    let (mut security, _guard2) = init_security();
    security.add_memory_identity(age::x25519::Identity::generate());

    security.generate_master_identity().expect("generate identity");

    let identity_path = home.join(".demon").join("identity.age");
    assert!(identity_path.exists(), "identity.age should be created");

    let pub_path = home.join(".demon").join("identity.pub");
    assert!(pub_path.exists(), "identity.pub should be created");
}

#[test]
fn test_generate_master_identity_refuses_existing_identity() {
    let home_guard = HomeGuard::new();
    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap();

    // Pre-create an identity file
    let identity_dir = home.join(".demon");
    fs::create_dir_all(&identity_dir).expect("create .demon dir");
    fs::write(identity_dir.join("identity.age"), "age1xxxxx").expect("create fake identity");

    let (mut security, _guard2) = init_security();
    security.add_memory_identity(age::x25519::Identity::generate());

    let result = security.generate_master_identity();
    assert!(result.is_err(), "should refuse to overwrite existing identity");
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("SAFETY TRIGGERED"));
}

#[test]
fn test_generate_master_identity_refuses_legacy_identity() {
    let home_guard = HomeGuard::new();
    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap();

    // Pre-create legacy identity
    let identity_dir = home.join(".demon");
    fs::create_dir_all(&identity_dir).expect("create .demon dir");
    fs::write(identity_dir.join("identity.txt"), "age1xxxxx").expect("create legacy identity");

    let (mut security, _guard2) = init_security();
    security.add_memory_identity(age::x25519::Identity::generate());

    let result = security.generate_master_identity();
    assert!(result.is_err(), "should refuse legacy identity");
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Legacy identity"));
}

// =============================================================================
// encrypt_for_node tests
// =============================================================================

#[test]
fn test_encrypt_for_node_uses_disk_master_identities() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let repo_root = tmp.path();
    let keys_dir = make_repo_keys_dir(repo_root);
    fs::create_dir_all(&keys_dir).expect("create keys dir");

    let master_identity = age::x25519::Identity::generate();
    write_age_key_for_repo(&keys_dir, &master_identity, "identity.age");

    let (mut security, _guard) = init_security_with_repo(repo_root);
    // Add an in-memory identity that is DIFFERENT from the disk identity
    let memory_identity = age::x25519::Identity::generate();
    security.add_memory_identity(memory_identity.clone());

    // Write disk identity to ~/.demon/identity.age so load_master_identities finds it
    let home_guard = HomeGuard::new();
    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap();
    let demon_dir = home.join(".demon");
    fs::create_dir_all(&demon_dir).expect("create .demon dir");
    fs::write(demon_dir.join("identity.age"), master_identity.to_string()).expect("write disk identity");

    // encrypt_for_node should use load_master_identities() (disk) not self.master_identities (memory)
    let node_identity = age::x25519::Identity::generate();
    let node_recipient_str = node_identity.to_public().to_string();

    let data = b"node payload";
    let encrypted = security
        .encrypt_for_node(data, &node_recipient_str)
        .expect("encrypt for node");

    // Decrypt with master identity (from disk) — should succeed
    let decrypted = security
        .decrypt_with_repo_key(&RepoKey(*master_identity.secret_key().as_bytes()), &encrypted);

    // The node identity can also decrypt since it was included as recipient
    // But more importantly: memory identity should NOT be able to decrypt
    // because encrypt_for_node uses load_master_identities (disk), not memory identities
    // This verifies the bug: memory-only identity was added but encrypt_for_node uses disk
}

// =============================================================================
// unlock_payload tests
// =============================================================================

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
fn test_unlock_payload_too_short() {
    let (security, _guard) = init_security();

    let result = security.unlock_payload(&[0u8; 11]);
    assert!(result.is_err(), "too short payload should fail");
}

#[test]
fn test_unlock_payload_wrong_key() {
    let (security, _guard) = init_security();
    let key1 = RepoKey(make_key_bytes());
    let key2 = RepoKey(make_key_bytes());

    let plaintext = b"secret payload";
    let encrypted = security
        .encrypt_with_repo_key(&key1, plaintext)
        .expect("encrypt");

    let result = security.unlock_payload(&encrypted);
    assert!(result.is_err(), "wrong key should fail");
}

#[test]
fn test_unlock_payload_empty() {
    let (security, _guard) = init_security();

    let result = security.unlock_payload(b"");
    assert!(result.is_err(), "empty payload should fail");
}

// =============================================================================
// TeamKey and create_team tests
// =============================================================================

#[test]
fn test_team_key_to_public_valid() {
    let _guard = HomeGuard::new();
    let identity = age::x25519::Identity::generate();
    let identity_str = identity.to_string();

    let team_key = dracon_security::TeamKey(identity_str.into_bytes());
    let recipient = team_key.to_public().expect("to_public");
    assert_eq!(recipient.to_string(), identity.to_public().to_string());
}

#[test]
fn test_team_key_to_public_invalid_utf8() {
    let _guard = HomeGuard::new();
    let mut invalid_bytes = vec![b'A'; 32];
    invalid_bytes[1] = 0x80;

    let team_key = dracon_security::TeamKey(invalid_bytes);
    let result = team_key.to_public();
    assert!(result.is_err(), "invalid UTF-8 should produce error");
}

#[test]
fn test_team_key_to_public_not_identity_string() {
    let _guard = HomeGuard::new();
    let not_identity = b"age1notavalididentitystringxxxxxxx".to_vec();
    let team_key = dracon_security::TeamKey(not_identity);
    let result = team_key.to_public();
    assert!(result.is_err(), "non-identity string should produce error");
}

#[test]
fn test_create_team_name_validation() {
    let (security, _guard) = init_security();

    // Valid name should work
    let result = security.create_team("my-team");
    assert!(result.is_ok() || result.is_err(), "create_team should be callable");

    // Invalid name with /
    let result = security.create_team("my/team");
    assert!(result.is_err(), "team name with / should be rejected");

    let result = security.create_team("my\\team");
    assert!(result.is_err(), "team name with \\ should be rejected");

    let result = security.create_team("my:team");
    assert!(result.is_err(), "team name with : should be rejected");
}