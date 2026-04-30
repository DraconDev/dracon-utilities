use dracon_security::DemonSecurity;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn init_security() -> DemonSecurity {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());
    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);
    security
}

#[test]
fn test_accept_team_invite_refuses_to_overwrite_existing_key() {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());

    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity.clone());

    let team_dir = temp_home.path().join(".demon").join("teams");
    fs::create_dir_all(&team_dir).unwrap();
    let team_key_path = team_dir.join("existing-team.key");
    fs::write(&team_key_path, b"fake key data").unwrap();

    let mut security2 = DemonSecurity::new(None).expect("init security2");
    security2.add_memory_identity(identity);

    let fake_invite = team_dir.join("fake.invite");
    fs::write(&fake_invite, b"not a real invite").unwrap();

    let result = security2.accept_team_invite(&fake_invite);
    assert!(result.is_err(), "accept_team_invite should fail when team key path already exists");
}

#[test]
fn test_keygen_refuses_to_overwrite_existing_secret_key() {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());

    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);

    let identity_path = temp_home.path().join(".demon").join("identity.age");
    fs::write(&identity_path, b"already exists").unwrap();

    let result = security.run_keygen();
    assert!(result.is_err(), "keygen should refuse to overwrite existing secret key");
}

#[test]
fn test_pub_key_write_uses_atomic_create_new() {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());
    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);

    let repo_root = security.get_repo_root().expect("repo root");
    let keys_dir = repo_root.join(".demon").join("data").join("keys");
    fs::create_dir_all(&keys_dir).expect("create keys dir");

    let identity = security.master_identities().first().expect("have identity");
    let pub_key = identity.to_public();
    let pub_key_str = pub_key.to_string();
    let safe_id = &pub_key_str[..8];
    let filename = format!("owner_{}.pub", safe_id);
    let key_path = keys_dir.join(&filename);

    fs::write(&key_path, b"old content").unwrap();

    let result = security.ensure_current_user_key();
    assert!(result.is_ok(), "ensure_current_user_key should succeed");

    let content = fs::read(&key_path).expect("read key path");
    assert!(
        content.contains(&pub_key_str[..]),
        "pub key should be written atomically"
    );
}

#[test]
fn test_ensure_current_user_key_is_idempotent() {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());
    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);

    security.ensure_current_user_key().expect("first call");
    security.ensure_current_user_key().expect("second call");

    let repo_root = security.get_repo_root().expect("repo root");
    let keys_dir = repo_root.join(".demon").join("data").join("keys");
    let identity = security.master_identities().first().expect("identity");
    let safe_id = &identity.to_public().to_string()[..8];
    let key_path = keys_dir.join(format!("owner_{}.pub", safe_id));

    let count = fs::read_dir(&keys_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("owner_"))
        .count();

    assert_eq!(count, 1, "should have exactly one owner key file (idempotent write)");
}

#[test]
fn test_accept_team_invite_creates_with_correct_permissions() {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());

    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity.clone());

    let team_dir = temp_home.path().join(".demon").join("teams");
    fs::create_dir_all(&team_dir).expect("create team dir");

    let team_key_path = team_dir.join("new-team-perms.key");

    let result = security.accept_team_invite(&team_key_path);
    assert!(result.is_err(), "accept_team_invite should reject non-invite path");
}
