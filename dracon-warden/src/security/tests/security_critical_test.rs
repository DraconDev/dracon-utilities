mod common;

use common::HomeGuard;
use secrecy::ExposeSecret;
use std::fs;
use std::io::Write;

fn init_security() -> (dracon_security::DemonSecurity, HomeGuard) {
    let _guard = HomeGuard::new();
    let mut security = dracon_security::DemonSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);
    (security, _guard)
}

fn init_security_with_repo(repo_root: &std::path::Path) -> (dracon_security::DemonSecurity, HomeGuard) {
    let _guard = HomeGuard::new();
    let mut security = dracon_security::DemonSecurity::new(Some(repo_root.as_ref())).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);
    (security, _guard)
}

fn make_keys_dir(repo_root: &std::path::Path) -> std::path::PathBuf {
    repo_root.join(".git").join("arcane").join("keys")
}

fn encrypt_for_recipient(recipient: &age::x25519::Recipient, plaintext: &[u8]) -> Vec<u8> {
    let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient.clone())];
    let encryptor = age::Encryptor::with_recipients(recipients).expect("encryptor");
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted).expect("wrap");
    writer.write(plaintext).expect("write");
    writer.finish().expect("finish");
    encrypted
}

fn write_age_key(keys_dir: &std::path::Path, identity: &age::x25519::Identity, filename: &str) {
    fs::create_dir_all(keys_dir).expect("create keys dir");
    fs::write(keys_dir.join(filename), identity.to_string().expose_secret().as_bytes()).expect("write age key");
}

// =============================================================================
// EnvironmentManager tests
// =============================================================================

#[test]
fn test_env_manager_to_env_file_variables() {
    let mut em = dracon_security::EnvironmentManager::new();
    em.add_variable("USER".to_string(), "alice".to_string());
    em.add_variable("HOME".to_string(), "/home/alice".to_string());

    let output = em.to_env_file();
    assert!(output.contains("USER=\"alice\""));
    assert!(output.contains("HOME=\"/home/alice\""));
}

#[test]
fn test_env_manager_to_env_file_secrets() {
    let mut em = dracon_security::EnvironmentManager::new();
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
    let mut em = dracon_security::EnvironmentManager::new();
    em.add_variable("MESSAGE".to_string(), "He said \"hello\"".to_string());

    let output = em.to_env_file();
    assert!(output.contains("MESSAGE=\"He said \\\"hello\\\"\""));
}

#[test]
fn test_env_manager_load_from_env_file() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let env_path = tmp.path().join("test.env");

    fs::write(&env_path, "VAR1=\"value1\"\nVAR2=value2\nEMPTY=\n").expect("write env file");

    let mut em = dracon_security::EnvironmentManager::new();
    em.load_from_env_file(&env_path).expect("load from env file");

    assert_eq!(em.variables.get("VAR1").map(|s| s.as_str()), Some("value1"));
    assert_eq!(em.variables.get("VAR2").map(|s| s.as_str()), Some("value2"));
    assert_eq!(em.variables.get("EMPTY").map(|s| s.as_str()), Some(""));
}

#[test]
fn test_env_manager_load_from_env_file_nonexistent() {
    let _guard = HomeGuard::new();
    let mut em = dracon_security::EnvironmentManager::new();
    let result = em.load_from_env_file(std::path::Path::new("/nonexistent/.env"));
    assert!(result.is_ok(), "nonexistent path should return Ok (no-op)");
}

#[test]
fn test_env_manager_load_from_env_file_with_single_quotes() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let env_path = tmp.path().join("test.env");
    fs::write(&env_path, "KEY='single quoted value'\n").expect("write env file");

    let mut em = dracon_security::EnvironmentManager::new();
    em.load_from_env_file(&env_path).expect("load");
    assert_eq!(em.variables.get("KEY").map(|s| s.as_str()), Some("single quoted value"));
}

#[test]
fn test_env_manager_load_from_env_file_with_embedded_equals() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let env_path = tmp.path().join("test.env");
    fs::write(&env_path, "EQUATION=\"a=b=c\"\n").expect("write env file");

    let mut em = dracon_security::EnvironmentManager::new();
    em.load_from_env_file(&env_path).expect("load");
    assert_eq!(em.variables.get("EQUATION").map(|s| s.as_str()), Some("a=b=c"));
}

#[test]
fn test_env_manager_combined() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let env_path = tmp.path().join("test.env");
    fs::write(&env_path, "VAR=value\nSECRET=hidden\n").expect("write env file");

    let mut em = dracon_security::EnvironmentManager::new();
    em.add_variable("FROM_CODE".to_string(), "code_val".to_string());
    em.load_from_env_file(&env_path).expect("load");
    em.add_secret("creds".to_string(), "API_KEY".to_string(), "key".to_string());

    let output = em.to_env_file();
    assert!(output.contains("FROM_CODE=\"code_val\""));
    assert!(output.contains("VAR=\"value\""));
    assert!(output.contains("# Group: credds"));
    assert!(output.contains("API_KEY=\"key\""));
}

// =============================================================================
// RepoKey::from_file edge case tests
// =============================================================================

#[test]
fn test_repokey_from_file_truncated() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let key_path = tmp.path().join("key");

    fs::write(&key_path, vec![0u8; 16]).expect("write truncated key");

    let result = dracon_security::RepoKey::from_file(&key_path);
    assert!(result.is_err(), "truncated key should be rejected");
}

#[test]
fn test_repokey_from_file_overlength() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let key_path = tmp.path().join("key");

    let mut long_key = vec![0u8; 32];
    long_key.extend_from_slice(&[1, 2, 3, 4]);
    fs::write(&key_path, long_key).expect("write overlength key");

    let result = dracon_security::RepoKey::from_file(&key_path);
    assert!(result.is_err(), "overlength key should be rejected");
}

#[test]
fn test_repokey_from_file_empty() {
    let _guard = HomeGuard::new();
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let key_path = tmp.path().join("key");
    fs::write(&key_path, b"").expect("write empty key");

    let result = dracon_security::RepoKey::from_file(&key_path);
    assert!(result.is_err(), "empty key file should be rejected");
}

#[test]
fn test_repokey_from_file_nonexistent() {
    let result = dracon_security::RepoKey::from_file(std::path::Path::new("/nonexistent/key"));
    assert!(result.is_err(), "nonexistent path should return error");
}

// =============================================================================
// load_repo_key tests
// =============================================================================

fn setup_repo_with_age_key(repo_root: &std::path::Path, master_identity: &age::x25519::Identity) -> Vec<u8> {
    let keys_dir = make_keys_dir(repo_root);
    fs::create_dir_all(&keys_dir).expect("create keys dir");

    write_age_key(&keys_dir, master_identity, "identity.age");

    let repo_key_bytes: [u8; 32] = rand::random();
    let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(master_identity.to_public())];
    let encryptor = age::Encryptor::with_recipients(recipients).expect("encryptor");
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted).expect("wrap");
    writer.write(&repo_key_bytes).expect("write repo key");
    writer.finish().expect("finish");

    fs::write(keys_dir.join("repo.key.age"), encrypted).expect("write repo key");
    repo_key_bytes.to_vec()
}

#[test]
fn test_load_repo_key_master_identity_success() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let repo_root = tmp.path();

    let master_identity = age::x25519::Identity::generate();
    let expected_key_bytes = setup_repo_with_age_key(repo_root, &master_identity);

    let (mut security, _guard) = init_security_with_repo(repo_root);
    security.add_memory_identity(master_identity);

    let loaded = security.load_repo_key().expect("load repo key");
    assert_eq!(loaded.get_key(), expected_key_bytes.as_slice());
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
    let keys_dir = make_keys_dir(tmp.path());
    fs::create_dir_all(&keys_dir).expect("create empty keys dir");

    let (security, _guard) = init_security_with_repo(tmp.path());

    let result = security.load_repo_key();
    assert!(result.is_err(), "empty keys dir should error");
}

#[test]
fn test_load_repo_key_machine_key_env_var() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let repo_root = tmp.path();
    let keys_dir = make_keys_dir(repo_root);
    fs::create_dir_all(&keys_dir).expect("create keys dir");

    let machine_identity = age::x25519::Identity::generate();
    write_age_key(&keys_dir, &machine_identity, "machine:runner.age");

    let repo_key_bytes: [u8; 32] = rand::random();
    let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(machine_identity.to_public())];
    let encryptor = age::Encryptor::with_recipients(recipients).expect("encryptor");
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted).expect("wrap");
    writer.write(&repo_key_bytes).expect("write");
    writer.finish().expect("finish");
    fs::write(keys_dir.join("machine.runner.age"), encrypted).expect("write machine key");

    let (security, _guard) = init_security_with_repo(repo_root);

    std::env::set_var("ARCANE_MACHINE_KEY", machine_identity.to_string().expose_secret().as_bytes().iter().map(|&b| char::from(b)).collect::<String>());
    let loaded = security.load_repo_key().expect("load repo key via machine key");
    assert_eq!(loaded.get_key(), repo_key_bytes.as_slice());
    std::env::remove_var("ARCANE_MACHINE_KEY");
}

#[test]
fn test_load_repo_key_team_key() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let repo_root = tmp.path();
    let keys_dir = make_keys_dir(repo_root);
    fs::create_dir_all(&keys_dir).expect("create keys dir");

    let master_identity = age::x25519::Identity::generate();
    let team_identity = age::x25519::Identity::generate();

    write_age_key(&keys_dir, &master_identity, "identity.age");

    let home_guard = HomeGuard::new();
    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap();
    let team_dir = home.join(".demon").join("teams");
    fs::create_dir_all(&team_dir).expect("create team dir");

    let encrypted_team = encrypt_for_recipient(&master_identity.to_public(), team_identity.to_string().expose_secret().as_bytes());
    fs::write(team_dir.join("my-team.key"), encrypted_team).expect("write team key file");

    let repo_key_bytes: [u8; 32] = rand::random();
    let encrypted = encrypt_for_recipient(&team_identity.to_public(), &repo_key_bytes);
    fs::write(keys_dir.join("team:my-team.age"), encrypted).expect("write team-encrypted repo key");

    let (mut security, _guard2) = init_security_with_repo(repo_root);
    security.add_memory_identity(master_identity);

    let loaded = security.load_repo_key().expect("load via team key");
    assert_eq!(loaded.get_key(), repo_key_bytes.as_slice());
}

// =============================================================================
// unlock_payload tests
// =============================================================================

fn make_test_setup() -> (dracon_security::DemonSecurity, Vec<u8>, HomeGuard) {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let repo_root = tmp.path();

    let master_identity = age::x25519::Identity::generate();
    let repo_key_bytes = setup_repo_with_age_key(repo_root, &master_identity);

    let (mut security, guard) = init_security_with_repo(repo_root);
    security.add_memory_identity(master_identity);

    (security, repo_key_bytes, guard)
}

fn repo_key_from_bytes(bytes: &[u8]) -> dracon_security::RepoKey {
    // Use from_file with a temp file since we can't construct RepoKey directly
    // But we need 32 bytes - use the first 32 of the input
    let mut key_bytes = vec![0u8; 32];
    key_bytes.copy_from_slice(&bytes[..32.min(bytes.len())]);
    // This won't work directly - we need a different approach
    // Use encrypt_with_repo_key which internally uses the repo key from load_repo_key
    // For testing, we use load_repo_key to get a real repo key
    panic!("use load_repo_key instead");
}

#[test]
fn test_unlock_payload_v1_format() {
    let (security, repo_key_bytes, _guard) = make_test_setup();
    let loaded_key = security.load_repo_key().expect("load repo key");

    let plaintext = b"V1 format payload";
    let encrypted = security
        .encrypt_with_repo_key(&loaded_key, plaintext)
        .expect("encrypt with repo key");

    let decrypted = security.unlock_payload(&encrypted).expect("unlock v1");
    assert_eq!(decrypted, plaintext.to_vec());
}

#[test]
fn test_unlock_payload_too_short() {
    let (security, _, _guard) = make_test_setup();

    let result = security.unlock_payload(&[0u8; 11]);
    assert!(result.is_err(), "too short payload should fail");
}

#[test]
fn test_unlock_payload_wrong_key() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let repo_root = tmp.path();

    let master_identity = age::x25519::Identity::generate();
    let _ = setup_repo_with_age_key(repo_root, &master_identity);

    let (mut security, _guard) = init_security_with_repo(repo_root);
    security.add_memory_identity(master_identity);

    // Create a repo key manually via from_file workaround: use generate_machine_identity
    let (_priv, pub_key) = dracon_security::DemonSecurity::generate_machine_identity();
    // We can't construct RepoKey directly from bytes in integration tests
    // Instead test that unlock_payload with a key we don't have fails
    // Use the team key scenario where we have master but not the right recipient

    // Create encrypted data with a key that security doesn't have
    let other_identity = age::x25519::Identity::generate();
    let other_recipient = other_identity.to_public();
    let wrong_recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(other_recipient)];
    let encryptor = age::Encryptor::with_recipients(wrong_recipients).expect("encryptor");
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted).expect("wrap");
    writer.write(b"secret").expect("write");
    writer.finish().expect("finish");

    let result = security.unlock_payload(&encrypted);
    assert!(result.is_err(), "unlock with wrong key should fail");
}

#[test]
fn test_unlock_payload_empty() {
    let (security, _, _guard) = make_test_setup();

    let result = security.unlock_payload(b"");
    assert!(result.is_err(), "empty payload should fail");
}

// =============================================================================
// generate_master_identity tests
// =============================================================================

#[test]
fn test_generate_master_identity_refuses_existing_identity() {
    let home_guard = HomeGuard::new();
    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap();

    let identity_dir = home.join(".demon");
    fs::create_dir_all(&identity_dir).expect("create .demon dir");
    fs::write(identity_dir.join("identity.age"), "age1xxxxx").expect("create fake identity");

    let (mut security, _guard2) = init_security();
    security.add_memory_identity(age::x25519::Identity::generate());

    let result = security.generate_master_identity();
    assert!(result.is_err(), "should refuse to overwrite existing identity");
    assert!(result.unwrap_err().to_string().contains("SAFETY TRIGGERED"));
}

#[test]
fn test_generate_master_identity_refuses_legacy_identity() {
    let home_guard = HomeGuard::new();
    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap();

    let identity_dir = home.join(".demon");
    fs::create_dir_all(&identity_dir).expect("create .demon dir");
    fs::write(identity_dir.join("identity.txt"), "age1xxxxx").expect("create legacy identity");

    let (mut security, _guard2) = init_security();
    security.add_memory_identity(age::x25519::Identity::generate());

    let result = security.generate_master_identity();
    assert!(result.is_err(), "should refuse legacy identity");
    assert!(result.unwrap_err().to_string().contains("Legacy identity"));
}

// =============================================================================
// TeamKey tests — use the public API (create_team, load_team_key)
// =============================================================================

#[test]
fn test_create_team_name_validation_rejects_slash() {
    let (security, _guard) = init_security();
    let result = security.create_team("my/team");
    assert!(result.is_err(), "team name with / should be rejected");
}

#[test]
fn test_create_team_name_validation_rejects_backslash() {
    let (security, _guard) = init_security();
    let result = security.create_team("my\\team");
    assert!(result.is_err(), "team name with \\ should be rejected");
}

#[test]
fn test_create_team_name_validation_rejects_colon() {
    let (security, _guard) = init_security();
    let result = security.create_team("my:team");
    assert!(result.is_err(), "team name with : should be rejected");
}

// =============================================================================
// encrypt_for_node uses disk identities test
// =============================================================================

#[test]
fn test_encrypt_for_node_uses_disk_master_identities() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let repo_root = tmp.path();
    let keys_dir = make_keys_dir(repo_root);
    fs::create_dir_all(&keys_dir).expect("create keys dir");

    let disk_identity = age::x25519::Identity::generate();
    write_age_key(&keys_dir, &disk_identity, "identity.age");

    let (mut security, _guard) = init_security_with_repo(repo_root);
    let memory_identity = age::x25519::Identity::generate();
    security.add_memory_identity(memory_identity.clone());

    let home_guard = HomeGuard::new();
    let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap();
    let demon_dir = home.join(".demon");
    fs::create_dir_all(&demon_dir).expect("create .demon dir");
    fs::write(demon_dir.join("identity.age"), disk_identity.to_string().expose_secret().as_bytes()).expect("write disk identity");

    let node_identity = age::x25519::Identity::generate();
    let node_recipient_str = node_identity.to_public().to_string();

    let data = b"node payload";
    let encrypted = security
        .encrypt_for_node(data, &node_recipient_str)
        .expect("encrypt for node");

    // disk_identity should be able to decrypt (it was loaded via load_master_identities)
    let result = security.decrypt_v2(&encrypted);
    assert!(result.is_ok(), "disk identity should be able to decrypt what encrypt_for_node produced");
}

// =============================================================================
// encrypt_with_repo_key / decrypt_with_repo_key tests
// =============================================================================

#[test]
fn test_encrypt_decrypt_repo_key_roundtrip() {
    let (security, _, _guard) = make_test_setup();
    let repo_key = security.load_repo_key().expect("load repo key");

    let plaintext = b"Hello, World! This is a test message.";
    let encrypted = security
        .encrypt_with_repo_key(&repo_key, plaintext)
        .expect("encrypt");
    assert_ne!(encrypted.as_slice(), plaintext);

    let decrypted = security
        .decrypt_with_repo_key(&repo_key, &encrypted)
        .expect("decrypt");
    assert_eq!(decrypted, plaintext.to_vec());
}

#[test]
fn test_encrypt_with_repo_key_empty_plaintext() {
    let (security, _, _guard) = make_test_setup();
    let repo_key = security.load_repo_key().expect("load repo key");

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
    let (security, _, _guard) = make_test_setup();
    let repo_key = security.load_repo_key().expect("load repo key");

    let result = security.decrypt_with_repo_key(&repo_key, &[0u8; 11]);
    assert!(result.is_err(), "too short ciphertext should error");
}

#[test]
fn test_encrypt_with_repo_key_random_nonce_per_call() {
    let (security, _, _guard) = make_test_setup();
    let repo_key = security.load_repo_key().expect("load repo key");

    let plaintext = b"same message";
    let ct1 = security.encrypt_with_repo_key(&repo_key, plaintext).expect("encrypt1");
    let ct2 = security.encrypt_with_repo_key(&repo_key, plaintext).expect("encrypt2");

    assert_ne!(ct1, ct2, "random nonce should produce different ciphertext");
}