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
fn test_backup_file_recursion_guard_rejects_backups_dir() {
    let (security, _temp_home) = init_with_temp_home();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let bad_path = temp_home.join(".demon").join("backups").join("self.backup");

    let result = security.backup_file(&bad_path, b"sensitive data");
    assert!(result.is_err(), "backing up a file inside demon/backups should be rejected");
}

#[test]
fn test_backup_file_recursion_guard_rejects_arcane_backups() {
    let (security, _temp_home) = init_with_temp_home();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let bad_path = temp_home.join("arcane").join("backups").join("self.bak.age");

    let result = security.backup_file(&bad_path, b"sensitive data");
    assert!(result.is_err(), "backing up a file inside arcane/backups should be rejected");
}

#[test]
fn test_backup_file_creates_encrypted_backup() {
    let (security, _temp_home) = init_with_temp_home();

    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let file_path = temp_home.join("secret_backup_test.txt");
    let content = b"Super Secret Blueprint of the Death Star";
    fs::write(&file_path, content).expect("write original file");

    let backup_path = security.backup_file(&file_path, content).expect("backup");
    assert!(backup_path.exists(), "backup file should exist");
    assert!(
        backup_path.to_string_lossy().contains("demon/backups"),
        "backup should be in demon/backups"
    );

    let backup_bytes = fs::read(&backup_path).expect("read backup");
    assert!(
        backup_bytes.starts_with(b"age-encryption.org/v1"),
        "backup should be age-encrypted"
    );
}

#[test]
fn test_restore_file_error_when_no_backups() {
    let (security, _temp_home) = init_with_temp_home();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let file_path = temp_home.join("nonexistent_file.txt");

    let result = security.restore_file(&file_path);
    assert!(result.is_err(), "restore should fail when no backups exist");
}

#[test]
fn test_accept_team_invite_rejects_nonexistent_path() {
    let (security, _temp_home) = init_with_temp_home();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let nonexistent_invite = temp_home.join("nonexistent_invite.invite");

    let result = security.accept_team_invite(&nonexistent_invite);
    assert!(result.is_err(), "accept_team_invite should fail for nonexistent path");
}
