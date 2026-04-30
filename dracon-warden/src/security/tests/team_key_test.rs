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
fn test_create_and_load_team_key() {
    let (security, _temp_home) = init_with_temp_home();
    let team_name = "test-team-create-load";

    security.create_team(team_name).expect("create team");

    let loaded = security.load_team_key(team_name).expect("load team key");
    assert_eq!(loaded.0.len(), 32, "team key should be 32 bytes");

    let result = security.load_team_key("nonexistent-team");
    assert!(result.is_err(), "loading nonexistent team should fail");
}

#[test]
fn test_create_team_rejects_duplicate() {
    let (security, _temp_home) = init_with_temp_home();
    let team_name = "dup-team";

    security.create_team(team_name).expect("first create");
    let second = security.create_team(team_name);
    assert!(second.is_err(), "duplicate team create should fail");
}

#[test]
fn test_create_team_rejects_invalid_names() {
    let (security, _temp_home) = init_with_temp_home();
    for invalid in &["bad/name", "bad\\name", "bad:name"] {
        let result = security.create_team(invalid);
        assert!(result.is_err(), "team name '{}' should be rejected", invalid);
    }
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
fn test_add_team_member_creates_key_file() {
    let (security, _temp_home) = init_with_temp_home();
    let team_name = "member-team";

    security.create_team(team_name).expect("create team");

    let member_key = Identity::generate();
    let member_public_str = member_key.to_public().to_string();

    security
        .add_team_member("alice", &member_public_str)
        .expect("add team member");

    let temp_home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap();
    let keys_dir = temp_home
        .join(".demon")
        .join("teams")
        .join(team_name)
        .join("keys");

    let key_file = keys_dir.join("alice.age");
    assert!(key_file.exists(), "alice key file should exist after add_team_member");
}

#[test]
fn test_add_team_member_rejects_invalid_key() {
    let (security, _temp_home) = init_with_temp_home();
    let team_name = "badmember-team";

    security.create_team(team_name).expect("create team");

    let result = security.add_team_member("bob", "not-a-valid-key");
    assert!(result.is_err(), "invalid public key should be rejected");
}
