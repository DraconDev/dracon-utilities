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
fn test_resto[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB3S3JhcCsySTA4OEJYYkxWUnlrSVBHZWIrT3JwZ3FYUmYzZndKMmR3OHpvCmkwK3g1RXVwODh3UVJoNWc3RzV3azVHVmpvQmRNMHB5dE5TV1VEdXZMT3cKLT4gWDI1NTE5IGNFKzlxUENYTHdMVW9Za1pMZytBVkVLVUVxbUNGTUNMSFpuYlRGZ0dneTQKOE5ESFdHN0N5LzFNaTROQzE2OTh5LzFEaXJ2eDlRSXFuZVN0Q3N2L3BYMAotPiBYMjU1MTkgc2szR3kwdUpOdUlzZXB2aWEvT0g4WDNWNHJQa1BqYmRzZkVtZmtrby9WTQpMaU1tT3VsMUJBNWtDb1lJeW5KU25VbTZKMDNiVzhvTnhPN0d0SVRvVVJrCi0+IFgyNTUxOSB5ZXk2d3VjNjFlUVdic0JteEVQOW5EUkNlbm9Nc0liNWhQQTZ4MUJkU1I4CmFlSFRuNTFrWkw0cjRqcWZRZU9kTEl4ZG90QTJqMHNCZGJMeTZYRmxMdm8KLT4gWDI1NTE5IHdyNG1JNk1nQ0xUZUt5Vm16TVo2ZmJiVVhjZXFsZ21FTzk3VTVHeTBCMWcKMXd1VnEvWjE4OUtuOXhGWXdWRG9VOGdXd2FzcjRGLy9oeWFZMEN5OWQ5awotPiBYMjU1MTkgc0JMcXhuSUdacEIrWG9DS0FSM1lRYkQ5YTUrU2JUNHY1ck5TNmhJTkxoRQpTY2gva1RsQ29jV1owZWVESkR2WURwbFJmY3E3RGJKNW9kZk1NRjk1S1drCi0+IHMjSj50aC4tZ3JlYXNlIGgKTm4veVRiWkpuUQotLS0gdWlmM21xSk5HNFJ4UlBQOUVIZjhrVlhLVTVxTFRaZXVYbXRaNGJwVHFHSQqTCPeir/koLF9nmCvqSfz7q0d2rTFbM138PzBMlNKfxgxXFJ9V9ZxWQWu+1l75h11FE2WH/5Gks0qPEOfzSyTFdNw=]() {
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
