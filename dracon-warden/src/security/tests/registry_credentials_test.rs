use dracon_security::DemonSecurity;
use std::fs;
use tempfile::tempdir;

fn init_security() -> DemonSecurity {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());
    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);
    security
}

fn read_passwords_from_json(path: &std::path::Path) -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct Cred {
        registry: String,
        username: String,
        password: String,
    }
    let content = fs::read_to_string(path).unwrap();
    let creds: Vec<Cred> = serde_json::from_str(&content).unwrap();
    creds.into_iter().map(|c| c.password).collect()
}

#[test]
fn test_save_and_load_registry_credentials() {
    let security = init_security();

    let cred = dracon_security::RegistryCredential::new("ghcr.io", "myuser", "super_secret_pass_123!");
    security
        .save_registry_credential(cred.clone())
        .expect("save credential");

    let loaded = security.load_registry_credentials().expect("load credentials");
    assert!(!loaded.is_empty(), "should have loaded credentials");

    let found = loaded.iter().find(|c| c.registry == "ghcr.io");
    assert!(found.is_some(), "ghcr.io credential should exist");
    assert_eq!(found.unwrap().username, "myuser", "username should match");
}

#[test]
fn test_save_registry_credentials_password_not_in_plaintext_file() {
    let security = init_security();

    let cred = dracon_security::RegistryCredential::new(
        "docker.io",
        "admin",
        "P@ssw0rd!With_Special$Chars",
    );
    security
        .save_registry_credential(cred.clone())
        .expect("save credential");

    let creds_path = security.get_registries_path().expect("registries path");

    let file_content = fs::read_to_string(&creds_path).expect("read creds file");
    assert!(
        !file_content.contains("P@ssw0rd!With_Special$Chars"),
        "password should NOT appear as plaintext in the encrypted file"
    );

    let loaded = security.load_registry_credentials().expect("load credentials");
    let found = loaded.iter().find(|c| c.registry == "docker.io");
    assert!(
        found.is_some(),
        "docker.io credential should be loadable after save"
    );
}

#[test]
fn test_save_registry_credentials_upserts_existing() {
    let security = init_security();

    let cred1 = dracon_security::RegistryCredential::new("npm.io", "user1", "first_password");
    security
        .save_registry_credential(cred1)
        .expect("save first");

    let cred2 = dracon_security::RegistryCredential::new("npm.io", "user2", "second_password");
    security
        .save_registry_credential(cred2)
        .expect("save second (upsert)");

    let loaded = security.load_registry_credentials().expect("load credentials");
    let found = loaded.iter().find(|c| c.registry == "npm.io");
    assert!(found.is_some(), "npm.io should still exist");
    assert_eq!(found.unwrap().username, "user2", "username should be updated");
}

#[test]
fn test_load_registry_credentials_nonexistent_returns_empty() {
    let security = init_security();
    let loaded = security.load_registry_credentials().unwrap_or_default();
    assert!(loaded.is_empty(), "nonexistent registry creds should return empty vec");
}

#[test]
fn test_save_registry_credentials_to_multiple_registries() {
    let security = init_security();

    let registries = vec![
        ("ghcr.io", "user1", "pass1"),
        ("docker.io", "user2", "pass2"),
        ("npm.io", "user3", "pass3"),
    ];

    for (registry, username, password) in registries.clone() {
        let cred = dracon_security::RegistryCredential::new(registry, username, password);
        security
            .save_registry_credential(cred)
            .expect("save credential");
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
