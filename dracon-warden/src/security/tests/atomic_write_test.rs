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
fn test_resto[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBSVHlQdVc1NDROd2VLUzR2OGtXcDFOeXYrcGdxcnpoQ2tFYksxM3ptYlVNCjlxVWROd2ZuUks3NDBucm1HUyttU0FxVDZ3b3NZS3BtYnYzUGZLOGliejgKLT4gWDI1NTE5IHdUektlTitWcWcvVGxrdmR6cllyZENIRmZET3VBamNmckJ6cEk1VG9IaWsKNHQvaHlKQzVjU2o3SkpoOEpDRzR0aHl6NS9TOEh2V2ZKTWVCSDN2eHFSTQotPiBYMjU1MTkga0NaOVVXN2ZkOU1SWkl0S2ROVk5FZDJ4dTV2T3p3d052Y3FhOWdPQ0Fudwo1enE0UGd5VE92ZTA1UDlZMnFvaWMxbGlyellFSmsybmswdUg3WTlWK0pvCi0+IFgyNTUxOSBKVmE2TzBFL1JrOU82eGxMcFkySVFMZmkwcUYrRmszb3FGMGZUSHpGUXdnCkdlRkpET0wwZm1SNkI5SDBhbm9LZ0Nzajg5Zmt1OHJYUkJxamdpWGR3bWMKLT4gWDI1NTE5IGttdDNxNm1NOWdjRDRBendqeHAxdU9kam5RMVEvSnRCb2Zqa1FuNDdVRDAKUjFnZlpvSVJ6aXorWW1EMXdVNGl5V1MrUlA1UWoxbmdOL09WTnZpT0FCZwotPiBWS0ZiLWdyZWFzZSAuZTYyVi4oClk2OGZsL21mcFlSOWJsMXMrYTFSSGVGSHM2cHN1R3ZDVzZpK3owNktPQ2RTanhGVGtSZnBHZitPSlFraFFMUW4KdkpNCi0tLSBYYjlpOUtvVi80RDQzRmlGaWJ2dTlUaFN3bUpRU1lFUDFIaTJjSWNmMGpFCsGrm+lvtGZoe1Kdu+xg34mDitxE0UIWLNDVYnyIhd16Ts8CqVPhurIVsuZejY6JFXpTOBQpnpekwZsDGr1FsRq+yg==]() {
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
