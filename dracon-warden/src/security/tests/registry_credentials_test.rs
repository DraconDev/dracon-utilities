use dracon_security::DemonSecurity;
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
fn test_load_registry_credentials_when_none_exist() {
    let (security, _temp_home) = init_with_temp_home();
    let loaded = security.load_registry_credentials().unwrap_or_default();
    assert!(loaded.is_empty(), "no credentials should exist initially");
}
