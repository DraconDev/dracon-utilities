use age::x25519::Identity;
use dracon_security::DemonSecurity;
use tempfile::tempdir;

fn init_with_temp_home() -> (DemonSecurity, tempfile::TempDir) {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());
    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = Identity::generate();
    security.add_memory_identity(identity);
    (security, temp_home)
}

#[test]
fn test_add_team_member_rejects_invalid_key() {
    let (security, _temp_home) = init_with_temp_home();
    let result = security.add_team_member("bob", "not-a-valid-key");
    assert!(result.is_err(), "invalid public key should be rejected");
}

#[test]
fn test_create_team_invite_requires_existing_team() {
    let (security, _temp_home) = init_with_temp_home();

    let user_key = Identity::generate();
    let user_public_str = user_key.to_public().to_string();
    let result = security.create_team_invite("nonexistent-team", &user_public_str);
    assert!(result.is_err(), "invite to nonexistent team should fail");
}
