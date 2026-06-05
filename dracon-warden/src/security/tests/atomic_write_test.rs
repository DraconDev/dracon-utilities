mod common;

use dracon_security::DemonSecurity;
use std::path::PathBuf;

use common::HomeGuard;

fn init_with_temp_home() -> (DemonSecurity, HomeGuard) {
    let _guard = HomeGuard::new();
    let mut security = DemonSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);
    (security, _guard)
}

#[test]
fn test_backup_file_recursion_guard_rejects_backups_dir() {
    let (security, _guard) = init_with_temp_home();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let bad_path = temp_home.join(".dracon").join("backups").join("self.backup");

    let result = security.backup_file(&bad_path, b"sensitive data");
    assert!(
        result.is_err(),
        "backing up a file inside dracon/backups should be rejected"
    );
}

#[test]
fn test_backup_file_recursion_guard_rejects_arcane_backups() {
    let (security, _guard) = init_with_temp_home();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let bad_path = temp_home
        .join("arcane")
        .join("backups")
        .join("self.bak.age");

    let result = security.backup_file(&bad_path, b"sensitive data");
    assert!(
        result.is_err(),
        "backing up a file inside arcane/backups should be rejected"
    );
}

#[test]
fn test_resto[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBobWtYM0xZbG5sK0JpN0lxV2tMV29TUFg0L29QWnlEMXljMG1NTmd5UEdFCjl3Vjc1TkZFRnVmbGFkV3dFSDg2TWJubWxRdWZKd0wybXpZNGlJN1Vmdm8KLT4gWDI1NTE5IFZ5SGN4R1VKOU5EVmxJa3B1UlIxclJEVEZQak40WW5lTUN0RFJRL1J5REEKTk5RSzBMNDlURlRQalFuR1BaU2tRTE1tNG9RUlBMbmU0MHJEL3F3bHZ2UQotPiA8QG5LQDktZ3JlYXNlICk/Ri8ncX08IFRYVVQ3TiMKUzJSRlVJdXRDWjA1ZWV2TFBoUmtPMjVKZ1BhT2ZkemgvWklJVHBmSkxoQTc2ZC91cGpwUGZCOVRaeEV2YXcKLS0tIGFhSkEyQ29MbFcyRUM1VkhzYzZsSG5udnd6NVZhR2tjWFU0Y1V3eDRFS1kKy23uFFUNF8N6AfyaElL9nt3sMnIv4WWilQVc215NJDHa0yPmFa2fyS28AvO54dvsakI9LxMLQQz/Ic1IhQ==]() {
    let (security, _guard) = init_with_temp_home();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let file_path = temp_home.join("nonexistent_file.txt");

    let result = security.restore_file(&file_path);
    assert!(result.is_err(), "restore should fail when no backups exist");
}

#[test]
fn test_accept_team_invite_rejects_nonexistent_path() {
    let (security, _guard) = init_with_temp_home();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let nonexistent_invite = temp_home.join("nonexistent_invite.invite");

    let result = security.accept_team_invite(&nonexistent_invite);
    assert!(
        result.is_err(),
        "accept_team_invite should fail for nonexistent path"
    );
}
