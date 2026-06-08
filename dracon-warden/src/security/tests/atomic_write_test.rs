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
fn test_resto[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBmTlFyVHVMS0piZDgydUV2NTMrMGQ1WHNtc2wxZ0lBQkJKWGtPQnZudWdNCkppeGJGSGk2T1hhcXhpRVZRcWxrYldOTFRwWm5hRHVKRkM2SGlIMmJyTDgKLT4gWDI1NTE5IFVRdUQzOHV0T3o3YzNzMDg2ZTFJbHhZYTNaMHVLZGduOWRYUzJ5UkR4VGcKQmY5RXRSUmZqdndrdXVHMjlLSFRDKzg0czRWSUZFWmZzT2cvNTNkVVE3MAotPiBYMjU1MTkgSDdtWnJ4TjVSYVYyYlZtWVVGU1FvcGtucTNBdzZNRHNFSHE3MUYxV3dVWQpUVS9venVjZXcwUllNbldNcnRFNGh0SExiczhORVNhT00xakFVUjVmTHpBCi0+IFgyNTUxOSBTcFJ2YUVESUZLbFc2ZEQ0UDJCN3NrRjVmKzdJUTJVWW40aDhEWVZnQmxzCmJKUitHbkROcTk3bFJ2dEFWeE5GOEZBRnZ6eUowWkFSUW1XRmxFS0NkYkEKLT4gWDI1NTE5IFZLMkRJTVJvMGdBMzZ3d29qL2N6aVZuTzRTVVZlemZ3S1BUS0kxQkEvaXMKdW5ncm02S3h0NnNsMktyVWNETUZNSTlXY1RlSWg5KzAvbGJyb3ZhclNMZwotPiBYMjU1MTkgVHl5L2E1ajljVXhmcHhNYnVUaXdDRENyUTVLb1J2OERjTjVlSGc1b3hRYwpFbnF6a3JQVkV6NENLdHI0cVJKQTgyekdxS3U4dWRRTzVRUjBPM3ZpT0dJCi0+IHdoZUQtZ3JlYXNlIDNLbzMmMlE7IChvakR8U1sgckclCmszRmcyK0k2ejNjanNhc0FIMTlDeGdQMUpmT3VYVXUyRUsrbzlGYmdjWmJnMmtGbVlpWUo2bWFHM2cKLS0tIHV6TlhnT1F0Rkwxb2pIQTdGcjZDazF4eVc5UWZxZWdtdGlIbk9MTGJ4MHcKL6Carw07OF/aJKE1SjhPsbsYHUg9YCShkkjyRgZHdGNZB4kst2JFBsGOgi1T0NgVLMa6lLMlxYoq0zhk0MA4uwoZ]() {
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
