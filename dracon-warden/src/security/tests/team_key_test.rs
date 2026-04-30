use age::x25519::Identity;
use dracon_security::DemonSecurity;
use std::fs;
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
fn test_create_team_invite_encrypts_team_key() {
    let (security, _temp_home) = init_with_temp_home();
    let team_name = "invite-team";

    security.create_team(team_name).expect("create team");

    let user_key = Identity::generate();
    let user_public_str = user_key.to_public().to_string();

    let invite_path = security
        .create_team_invite(team_name, &user_public_str)
        .expect("create invite");

    assert!(
        invite_path.to_string_lossy().contains("invites"),
        "invite path should contain 'invites'"
    );

    let invite_bytes = fs::read(&invite_path).expect("read invite file");
    assert!(
        invite_bytes.starts_with(b"age-encryption.org/v1"),
        "invite should be age-encrypted"
    );
}

#[test]
fn test_add_team_member_rejects_invalid_key() {
    let (security, _temp_home) = init_with_temp_home();
    let team_name = "badmember-team";

    security.create_team(team_name).expect("create team");

    let result = security.add_team_member("bob", "not-a-valid-key");
    assert!(result.is_err(), "invalid public key should be rejected");
}
