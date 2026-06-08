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
fn test_resto[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBuQTNOODlaOHNLVUlYcHNCMjFGS2VtdTZHVEFIa09YYTBsZWNwWEZIRG13CnBDWkl1TEl4c2UvOG1hZHhSTWJRd1pJbEovR2RXREFHc2hMSGNBVVZZK0EKLT4gWDI1NTE5IE5heS9GOUVtNnVmTDduTm5tS0ZxVjVZZ1ZrS1kwNEdHaUp0Zm5yK09SZzQKeUhodzZ1akk5QThKSUJPeDUwVGZrRzdYWUdnNXlIRHBBSS84ZEFXRnRudwotPiBYMjU1MTkgVmt2SmUrZURhdDIyelpNUXROb3N3YkF4YVhlZjh4M1VXMHM4OE1YK2lsWQpQL3BpbVNlQ1VRYVNoWlZINUxTYmxWa1gyVjJFYzFxeERkU0pLQy85SUc0Ci0+IFgyNTUxOSBXa2F6aUhkbHo2WlZrUmFFdGhFeVJDU1FHVzEzQThZQ2lISEZyWXd6d1hVCjZZdStxaTJNTzRDRkdJQ3g3MElJSGJSbGJib3BPNFYybzNXS1NWbkNkKzQKLT4gWDI1NTE5IDR1SW5ueHgzaUZpWFJKOWJTdXBYeWFSdE15cTBZWTd5c0MzbnpyQU5kaVUKbUFHRTZ6MUR2cnZSZW1ZTndJaEdHeUJRQURPTnIrTWgvcHpFR0hlVVZXYwotPiBYMjU1MTkgNWtzR205U20xUXBwTlU4KzJGY09GOVlUeFA0cVBJdEdmQjQrRGVKTUtsZwpET1VHVFhoTHpYRDcvNGl0WjVPWW5ENTlNdlhIN0ZHMC9DYXh1QUJybkJZCi0+IDVGLU8ucmstZ3JlYXNlIFhIM1k1dgpSa0RPK1VzVXF0OXJmbVo2MU00Q1ZHU20wcUhTbTd4ZmxUYXoveXRSMDVRZjFQSUx0bmlkdCtGMWdWUFNmd29kCnUxY0ptS0hJTEd0WFhmM2hqM3hMCi0tLSBPbUUzVXRoMXpqVTVGR0I3QTNoY2hlLzRkSk5YaWMzRWZObjhuc0ViZlpJCl/PZtREFdQmmXOG3hoLz4PwD54iFRPyuIceAAFG7bq6oWAGjWV0LGAPGEiEzZ/QjtpxbgyldvLkefys8CCddn7rIA==]() {
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
