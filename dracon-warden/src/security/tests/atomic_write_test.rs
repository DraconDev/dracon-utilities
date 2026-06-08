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
fn test_resto[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB2SnZZS3VTMWFEdXJRb283bzRlaWRjZG12KytKL1lGbyt6QUlYN3ZrWUZ3Cms4QklqSGhrNElCcFFaeWtxSkVRUnFmTXh0Q2NTRFhXMHlrandQUENNWm8KLT4gWDI1NTE5IFRBWVJuaFZPemVwcE9oS0pDNGMycGZkWDhDc1NlN0R6RExhbVIxNjBxQWcKaDRicUt5aloyNXF5aVZIamNmUTVHQXFjTzF5Mi81R3V5Z1NSWWtQb2ZXTQotPiBYMjU1MTkgZXUyYWJnQnpxclBWRDE4VnUzWThJY1pyam1iYnZLdXVvUGlrN3Y5d3dTbwpMMlQ1clNUS2tTV042NDRDMVdHQU5OM2hxc1hLVkpZUkRXbFJwSE1KcUNjCi0+IFgyNTUxOSB2MkJjbEpGUmdvOWh6YThqOVJwTnEzeXZjN25VTHFOUDBxZ3g3WDFUeUFZCnZhdUdUdnVsdUh4MThvSG1kWG1xZkFMckRYYi9tS2xnNjRVWHdHOU83TnMKLT4gWDI1NTE5IFBkak9wUmNtaFpwMzRuNU93N0dmUjlsZlBvNUFEYjdkeWxFY0d3RmhtaGMKZHN3blVVc280dGpYcjA2cGdWWXZzUVlPSVFKUFFlaEsxSEV0L3RWOERzVQotPiBYMjU1MTkgbjFmNlZqU1M4M3M2S3dBR0xkYXM3cXF2WHdzSXYzMUltSi83WmhlWlZVOApHV29PZlppZzlieElhazV6NTJyMlV5cGE0Q1VydjF4S0twUkFhZHhSUXQ0Ci0+IE0tZ3JlYXNlIGxIVmo0RE1tIGNUeygkCldlUXZ1K3ljYVo5WjRiU2lkNkRFcTRMVkd0YktjeCtmN095bzlodXd6UQotLS0gOXFrQ1FETUEvMHJSajc0SDhPblZSQ2VlbkhjSEZHM09MSWwvbXR6RzhTZwp+CRUo475HUnD3bWkUP/9upJ94Rvwp8+fY7ojsZo58eyT+2zhPk+0cTH7U48yiR+bC2LfY7H/UK9FrVYRD]() {
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
