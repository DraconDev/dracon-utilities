use dracon_security::DemonSecurity;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn init_with_temp_home() -> (DemonSecurity, tempfile::TempDir) {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());
    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);
    (security, temp_home)
}

#[test]
fn test_save_and_load_registry_credentials() {
    let (security, _temp_home) = init_with_temp_home();

    let cred = dracon_security::RegistryCredential::new("ghcr.io", "myuser", "super_secret_pass");
    security.save_registry_credential(cred.clone()).expect("save credential");

    let loaded = security.load_registry_credentials().expect("load credentials");
    assert!(!loaded.is_empty(), "should have loaded credentials");

    let found = loaded.iter().find(|c| c.registry == "ghcr.io");
    assert!(found.is_some(), "ghcr.io credential should exist");
    assert_eq!(found.unwrap().username, "myuser", "username should match");
}

#[test]
fn test_save_registry_credentials_upserts_existing() {
    let (security, _temp_home) = init_with_temp_home();

    let cred1 = dracon_security::RegistryCredential::new("npm.io", "user1", "first_password");
    security.save_registry_credential(cred1).expect("save first");

    let cred2 = dracon_security::RegistryCredential::new("npm.io", "user2", "second_password");
    security.save_registry_credential(cred2).expect("save second (upsert)");

    let loaded = security.load_registry_credentials().expect("load credentials");
    let found = loaded.iter().find(|c| c.registry == "npm.io");
    assert!(found.is_some(), "npm.io should still exist");
    assert_eq!(found.unwrap().username, "user2", "username should be updated");
    assert_eq!(found.unwrap().password, "second_password", "password should be updated");
}

#[test]
fn test_load_registry_credentials_nonexistent_returns_empty() {
    let (security, _temp_home) = init_with_temp_home();
    let loaded = security.load_registry_credentials().unwrap_or_default();
    assert!(loaded.is_empty(), "nonexistent registry creds should return empty vec");
}

#[test]
fn test_save_registry_credentials_multiple_registries() {
    let (security, _temp_home) = init_with_temp_home();

    let registries = vec![
        ("ghcr.io", "user1", "pass1"),
        ("docker.io", "user2", "pass2"),
        ("npm.io", "user3", "pass3"),
    ];

    for (registry, username, password) in registries.clone() {
        let cred = dracon_security::RegistryCredential::new(registry, username, password);
        security.save_registry_credential(cred).expect("save credential");
    }

    let loaded = security.load_registry_credentials().expect("load credentials");
    assert_eq!(loaded.len(), registries.len(), "all registries should be saved");

    for (registry, _, _) in registries {
        assert!(
            loaded.iter().any(|c| c.registry == registry),
            "{} should be in loaded credentials",
            registry
        );
    }
}

#[test]
fn test_registry_credentials_password_skipped_in_serialization() {
    let (security, _temp_home) = init_with_temp_home();

    let cred = dracon_security::RegistryCredential::new("ghcr.io", "admin", "S3cr3tP@ssw0rd!");
    security.save_registry_credential(cred).expect("save credential");

    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let creds_file = temp_home.join(".demon").join("registries.json");

    if creds_file.exists() {
        let content = fs::read_to_string(&creds_file).expect("read creds file");
        assert!(
            !content.contains("S3cr3tP@ssw0rd!"),
            "password should NOT appear as plaintext in the registries file"
        );
    }
}
