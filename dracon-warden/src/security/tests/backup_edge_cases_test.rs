use dracon_security::DemonSecurity;
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
fn test_restore_file_error_when_no_backups() {
    let (security, _temp_home) = init_with_temp_home();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let file_path = temp_home.join("nonexistent_file.txt");

    let result = security.restore_file(&file_path);
    assert!(result.is_err(), "restore should fail when no backups exist");
}
