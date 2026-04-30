use age::x25519::Identity;
use dracon_security::DemonSecurity;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_backup_and_restore() {
    // 1. Setup Env with isolated HOME
    let temp_home = tempdir().unwrap();
    // Force HOME to be our temp dir so DemonSecurity looks there
    std::env::set_var("HOME", temp_home.path());

    let home_path = temp_home.path().to_owned();
    println!("Temporary HOME: {:?}", home_path);

    // 2. Init Security
    // This will try to load identities from the new empty HOME and find none.
    let mut security = DemonSecurity::new(None).expect("Failed to init DemonSecurity");

    // 3. Add Master Identity explicitly (since none on disk)
    let identity = Identity::generate();
    security.add_memory_identity(identity);

    // 4. Create dummy file to backup
    let original_path = home_path.join("secret_data.txt");
    let content = b"Super Secret Blueprint of the Death Star";
    fs::write(&original_path, content).expect("Failed to write original file");

    // 5. Backup
    println!("Backing up file...");
    let backup_path = security
        .backup_file(&original_path, content)
        .expect("Backup failed");

    println!("Backup created at: {:?}", backup_path);
    assert!(backup_path.exists(), "Backup file should exist");
    assert!(
        backup_path.to_string_lossy().contains("demon/backups"),
        "Backup should be in demon/backups"
    );

    // 6. Verify Backup Content is Encrypted
    let backup_content = fs::read(&backup_path).expect("Failed to read backup");
    assert_ne!(
        backup_content, content,
        "Backup content should be encrypted"
    );
    assert!(!backup_content.is_empty());
    // V2 header check - "age-encryption.org/v1"
    assert!(
        backup_content.starts_with(b"age-encryption.org/v1"),
        "Should have age header"
    );

    // 7. Verify we can Restore
    // First, modify or delete the original to prove restore works
    fs::remove_file(&original_path).expect("Failed to delete original file");
    assert!(!original_path.exists());

    println!("Restoring file...");
    let restored_backup_source = security
        .restore_file(&original_path)
        .expect("Restore failed");

    assert_eq!(
        restored_backup_source, backup_path,
        "Should restore from the created backup"
    );
    assert!(original_path.exists(), "Original file should be restored");

    let restored_content = fs::read(&original_path).expect("Failed to read restored file");
    assert_eq!(
        restored_content, content,
        "Restored content should match original"
    );
}
