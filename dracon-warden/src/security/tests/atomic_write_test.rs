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
fn test_resto[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBtSjFhYkZ4RGx1SGxzbjJLVGRWOXBRekNXeG9hRHJIbHNVbExjQ0EySXpJCm8xeUFHZmNValZUZmJTbDI5NlE2UzBrZlRVdEJ0eXBvMFR4Ty9GeEVxaVkKLT4gWDI1NTE5IFZaczd2eGNNOTl2M2RhYVgyNUN5aFErenNkNnZ2VmRtb0s2My9TTThGSDAKZWtwc054aFA5WThMampZRjYwdzY5WTJRam1HczVQSURjSFRacW0yVE1DVQotPiBYMjU1MTkgMlJqNnBSY1kwOHdHMkdURWZ2YXV0T0h2UFBMUmFmZ0tCVnAzdFd3WVVtOAptRG9XUHVZSE1oV0ZhYis0a1UrS3NrN0k4WGVDN1Ntb2NiY1R3RkhQNG5JCi0+IFgyNTUxOSBMc1RXNkFFUU56OEJFSWpEN0JJK2JpekZmTUlHbmF6YWFrK0tETkl2bm44Ck4xQmY0ZFVYdTdPSTB1ZVViYm9CZWZ0Y1hFak9uenRKQWdNS2JKUnJBNTgKLT4gWDI1NTE5IEx0cmtzQnVwK2JIQVZKbW9zWXNuVzMraUtBTGdsd1Y4cFNid09ZQlN5Q0EKS2FUQmlTbWk1T0lEYzBzbXRkaitJYUFHUlQzVFU2V1k5c01qckNPM1hXdwotPiBYMjU1MTkgTWhkZDZXZjU5Q0FZOGhVNXV4QlVXTXFES2p0WUQrMDR3Wnc2SDNKZmJ4awp3OUJlOUk2WjVqdkVuVHA2S0VSdlpiazJMcFl6aGFoeWFRT2tQazIzcE5vCi0+IFYtZ3JlYXNlIGo9IGole304Iz0KTmRjRWdETkhXOTg1cURNNmJoMEd5SXYwd1Q0aHEzczF5Y2V2T2s3Sk9Dd2cwaG9KQnpuTmxxdldMM2xiZjhvYwozclMyVWRQc256UUI4MHFDTnIxM1pwcmZ5eHp1UU9jdUhTNHFtd0R3U244Ci0tLSAwdkR2M3JYMGNuaWpNMEV2WE5kcEtLNlR5TEk0UnhhNnU3TUxwWmxnWFJJCob9AKLkBzDXZKpI0loi23paoXVLcpRew5UHxK4BydEby0xcWWgrBFIogtlIc8nXXvI2pPayfUc5bDy6dPILb9Vc2w==]() {
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
