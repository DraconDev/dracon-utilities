mod common;

use dracon_security::WardenSecurity;
use std::path::PathBuf;

use common::HomeGuard;

fn init_with_temp_home() -> (WardenSecurity, HomeGuard) {
    let _guard = HomeGuard::new();
    let mut security = WardenSecurity::new(None).expect("init security");
    let identity = age::x25519::Identity::generate();
    security.add_memory_identity(identity);
    (security, _guard)
}

#[test]
fn test_backup_file_recursion_guard_rejects_backups_dir() {
    let (security, _guard) = init_with_temp_home();
    let temp_home = std::env::var("HOME").map(PathBuf::from).unwrap();
    let bad_path = temp_home
        .join(".dracon")
        .join("backups")
        .join("self.backup");

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
fn test_resto[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA0QWJMWG43MXJ2ejg4bzFyMXBNeisrNWZvNXJnb2FWNWJSemwxcVp0N2hJClhnUWhuTVR0d0FBL3N6Y3RsUFZoZDdqU2dZZWFUNzhEVU93bExBZjAvTm8KLT4gWDI1NTE5IHAxbVUwb2RHSERjRVc0SXpDcHY3amcvaGZIMmtvZk1KQ1VsWUJ5QzQ2MWMKaGRYeExXTmk2MEdMVm9iMmhjZjFzbGY3M01wTnZ5aS9DdURUMDRZNG9YUQotPiBYMjU1MTkgK2tmV2tnNFFQMWRuYWpKaEpOSURHeVVaUEgrWWhQWnlIaVpneDc5Mi9oYwpPMEpTVlNsR0VlNyttMzM3RXhDYVdBMWRMUnhCVFFKUjdQMjFvL3NVOSs0Ci0+IFgyNTUxOSBGOHM5amFlL1hEMnlRbHg1RzZuZ2JUcmhvUDVVYzZnd2RLYXEzbklIOEEwCk9kNVd0YU9rdXI1QlJvNzI3RVFwRWwwYW1xZm1TczhtbEpZWVN2TGxPTlkKLT4gWDI1NTE5IGtBeWJXclNJZVBralpaRUJTNnU4YUFKQmhZcHZva1ovY1dNbXZPMlZMREkKaVlVQjVHbkhRdm9idDk0WWNtQ3lnaWxlbmVPSk84bVl1aXdXaWFPUmQvdwotPiBiOX1gLWdyZWFzZSBFT2o/c0MgXUdEUCBgM3ppWGwgXXVVVwozQ3BwQmptK00zeE55SjhjR0pJSURQQlVJejZtODhacDlQdlMvNVFuSWZFTWp3a2U2cU0KLS0tIE9YODFDZ0xXYVpIYVdmSUdMOXNMRlBTejFiTk84ZVpkWTJ1bnJHSk56UXMK2RVk4RbN5wOL4Krc7+nHbDkResfaoFhQ0OgccdRFrj0IF1oXY/DVD8w2TLyBsK0bVmbtHQS/jq5uj6AcLZU1wXYK]() {
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
