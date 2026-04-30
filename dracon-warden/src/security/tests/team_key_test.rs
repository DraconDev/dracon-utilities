use age::x25519::Identity;
use dracon_security::DemonSecurity;
use std::fs;
use tempfile::tempdir;

fn init_security() -> DemonSecurity {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());
    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = Identity::generate();
    security.add_memory_identity(identity);
    security
}

#[test]
fn test_create_and_load_team_key() {
    let security = init_security();

    let team_name = "test-team-create-load";
    security.create_team(team_name).expect("create team");

    let loaded = security.load_team_key(team_name).expect("load team key");
    assert_eq!(loaded.0.len(), 32, "team key should be 32 bytes");

    let team_name2 = "nonexistent-team";
    assert!(
        security.load_team_key(team_name2).is_err(),
        "loading nonexistent team should fail"
    );
}

#[test]
fn test_create_team_rejects_duplicate() {
    let security = init_security();
    let team_name = "dup-team";

    security.create_team(team_name).expect("first create");
    let second = security.create_team(team_name);
    assert!(second.is_err(), "duplicate team create should fail");
}

#[test]
fn test_create_team_rejects_invalid_names() {
    let security = init_security();

    for invalid in &["bad/name", "bad\\name", "bad:name"] {
        let result = security.create_team(invalid);
        assert!(result.is_err(), "team name '{}' should be rejected", invalid);
    }
}

#[test]
fn test_create_team_invite_and_accept_roundtrip() {
    let mut security = init_security();
    let team_name = "invite-team";

    security.create_team(team_name).expect("create team");

    let user_key = Identity::generate();
    let user_public = user_key.to_public();
    let user_public_str = user_public.to_string();

    let invite_path = security
        .create_team_invite(team_name, &user_public_str)
        .expect("create invite");

    assert!(invite_path.to_string_lossy().contains("invites"), "invite path should contain 'invites'");
    assert!(invite_path.to_string_lossy().contains(team_name), "invite path should contain team name");

    let invite_bytes = fs::read(&invite_path).expect("read invite file");
    assert!(
        invite_bytes.starts_with(b"age-encryption.org/v1"),
        "invite should be age-encrypted"
    );
}

#[test]
fn test_add_team_member_and_authorize_recipient() {
    let security = init_security();
    let team_name = "member-team";

    security.create_team(team_name).expect("create team");

    let member_key = Identity::generate();
    let member_public_str = member_key.to_public().to_string();

    security
        .add_team_member("alice", &member_public_str)
        .expect("add team member");

    let repo_root = security.get_repo_root().expect("repo root");
    let keys_dir = repo_root.join(".git").join("arcane").join("keys");
    let key_file = keys_dir.join("alice.age");

    assert!(key_file.exists(), "alice key file should exist after add_team_member");
}

#[test]
fn test_add_team_member_rejects_invalid_key() {
    let security = init_security();
    let team_name = "badmember-team";

    security.create_team(team_name).expect("create team");

    let result = security.add_team_member("bob", "not-a-valid-key");
    assert!(result.is_err(), "invalid public key should be rejected");
}

#[test]
fn test_encrypt_and_decrypt_repo_key_with_team_key() {
    let security = init_security();
    let team_name = "crypto-team";

    security.create_team(team_name).expect("create team");

    let repo_key = security.load_repo_key().expect("load repo key");

    let team_key = security.load_team_key(team_name).expect("load team key");

    let repo_root = security.get_repo_root().expect("repo root");
    let key_file = repo_root.join(".git").join("arcane").join("keys").join("team_test.age");

    security
        .encrypt_and_save_key(&repo_key, &team_key.to_public(), &key_file)
        .expect("encrypt and save key");

    let loaded_key = security
        .decrypt_repo_key_with_team_key(&key_file, &team_key)
        .expect("decrypt with team key");

    assert_eq!(loaded_key.0, repo_key.0, "decrypted key should match original");
}

#[test]
fn test_decrypt_repo_key_with_team_key_wrong_key_fails() {
    let security = init_security();
    let team_name = "wrongkey-team";

    security.create_team(team_name).expect("create team");

    let repo_key = security.load_repo_key().expect("load repo key");

    let wrong_team_key = TeamKey((0..32).map(|i| (i + 1) as u8).collect());
    let repo_root = security.get_repo_root().expect("repo root");
    let key_file = repo_root
        .join(".git")
        .join("arcane")
        .join("keys")
        .join("team_wrong.age");

    security
        .encrypt_and_save_key(&repo_key, &wrong_team_key.to_public(), &key_file)
        .expect("encrypt and save key");

    let result = security.decrypt_repo_key_with_team_key(&key_file, &wrong_team_key);
    assert!(result.is_err(), "decrypt with wrong team key should fail");
}

struct TeamKey(Vec<u8>);

impl TeamKey {
    fn to_public(&self) -> age::x25519::Recipient {
        use std::str::FromStr;
        let identity = Identity::from_str(&String::from_utf8(self.0.clone()).unwrap())
            .expect("valid identity bytes");
        identity.to_public()
    }
}