use age::secrecy::ExposeSecret;
use age::x25519::Identity;
use dracon_security::DemonSecurity;
use std::fs;
use std::path::PathBuf;
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
fn test_team_invite_file_is_age_encrypted() {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());
    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = Identity::generate();
    security.add_memory_identity(identity.clone());

    let team_dir = temp_home.path().join(".demon").join("teams");
    fs::create_dir_all(&team_dir).expect("create team dir");

    let team_key_path = team_dir.join("test-team.key");

    let team_identity = Identity::generate();
    let team_identity_str = team_identity.to_string();
    let team_secret = team_identity_str.expose_secret();

    let recipient = identity.to_public();
    let encryptor = age::Encryptor::with_recipients(vec![Box::new(recipient)])
        .expect("create encryptor");
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted).expect("wrap output");
    let _ = writer.write(team_secret.as_bytes());
    writer.finish().expect("finish");
    fs::write(&team_key_path, &encrypted).expect("write team key");

    let user_key = Identity::generate();
    let user_public_str = user_key.to_public().to_string();
    let invite_path = security
        .create_team_invite("test-team", &user_public_str)
        .expect("create invite");

    let invite_bytes = fs::read(&invite_path).expect("read invite");
    assert!(
        invite_bytes.starts_with(b"age-encryption.org/v1"),
        "invite should be age-encrypted"
    );
}
