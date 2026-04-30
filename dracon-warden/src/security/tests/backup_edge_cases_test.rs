use dracon_security::DemonSecurity;
use std::fs;
use std::io::Write;
use tempfile::tempdir;

fn init_security() -> DemonSecurity {
    let temp_home = tempdir().unwrap();
    std::env::set_var("HOME", temp_home.path());
    DemonSecurity::new(None).expect("init security")
}

#[test]
fn test_backup_file_rejects_self() {
    let security = init_security();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap_or_default();
    let backup_path = temp_home.join(".demon").join("backups").join("self.backup");

    let result = security.backup_file(&backup_path, b"some data");
    assert!(result.is_err(), "backing up a backup file should be rejected");
}

#[test]
fn test_backup_file_rejects_arcane_backup_path() {
    let security = init_security();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap_or_default();
    let backup_path = temp_home.join("arcane").join("backups").join("self.backup");

    let result = security.backup_file(&backup_path, b"some data");
    assert!(result.is_err(), "backing up an arcane/backups path should be rejected");
}

#[test]
fn test_newest_file_picks_latest() {
    use std::path::PathBuf;
    use std::time::SystemTime;

    let temp_dir = tempfile::tempdir().unwrap();
    let older = temp_dir.path().join("file_old.txt");
    let newer = temp_dir.path().join("file_new.txt");

    fs::write(&older, b"old").unwrap();
    fs::write(&newer, b"new").unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));
    let newer_modified = fs::metadata(&newer)
        .unwrap()
        .modified()
        .unwrap();

    let older_file = fs::File::create(&older).unwrap();
    let newer_file = fs::File::create(&newer).unwrap();

    drop(older_file);
    drop(newer_file);

    let security = init_security();
    let files = vec![PathBuf::from(&older), PathBuf::from(&newer)];
    let result = security.newest_file(&files);
    assert!(result.is_ok());
}

#[test]
fn test_newest_file_empty_list_fails() {
    let security = init_security();
    let result = security.newest_file(&[]);
    assert!(result.is_err(), "newest_file on empty list should fail");
}

#[test]
fn test_newest_file_nonexistent_paths() {
    let security = init_security();
    let nonexistent = tempfile::tempdir().unwrap().path().join("nonexistent.txt");
    let result = security.newest_file(&[nonexistent]);
    assert!(result.is_err(), "newest_file on nonexistent paths should fail");
}

use std::path::PathBuf;