use dracon_security::SecretScanner;

#[test]
fn test_scanner_stress_1000() {
    let scanner = SecretScanner::new().unwrap();
    let clean_content = "fn main() { println!(\"Hello, world!\"); }";
    let secret_content = "let [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA1cFl4U3VjN2JPWlBKSzlqMFVSUVdXN1hXNzFiMEpGT3RhWTFJNk0yQUJVClBVZUZEK2c1cTUxZ29zb3l2L0lSYlRXV2JrTUZVUllHcmNueFpITmpMYzAKLT4gWDI1NTE5IE9PU3kxNk0wQ0Y3TUZBWUhKVk1LVUpzZElOS0FZOGhHZnFhWmkyWDNIVEkKWjZrbGVWcitXUFZCRUZvRFY3NjZIOCtwZnFFWG42VmlGa1BhYS9QNUtEawotPiBYMjU1MTkgME8xWkxsOEJ1UW5iNUd5bXFZdk1qSVk5Ym8zd0pQWnh0bElkYWpxNzFHcwp6Q0JPOEdCdm96ejR6dGFuZ0JmL2p3V0o3cnN4d2hydHUvOTJaVERTeUtZCi0+IFgyNTUxOSBVRkZYQmQvQ3RKZFlYVURkQ00vMHozaU00ZzgvS2dDQTM1c2NYVDN4cUFRCk9Uby90NlZVa2RIWVN4aGZ1NUw0czJaTW0wZEh6aHEydE43djI1R0toV2sKLT4gWDI1NTE5IGI3RTgyRFNlS05EbXV0Z1FyOEc5VlgrWktmc1UvcTVkUDdZbk5MNWhhQjAKa3RlMGZKQjJFTm04L3RlNUxRazU0MjhibUhhb3FDcGJlL1lhRUNSZ2FiQQotPiBdWTEtZ3JlYXNlIFlvPkUsIHpzV2ooJT1kIGt1Zzk9XgpUQVArdkYyZTJtWEs3VVovWStTZVhGZllLdVljak8ycGhkMDlHOXNETXRkeAotLS0gRTRieG1CNTlYQWovc2Q0M3I1NzAxRE1pVWJSZ3VjL25TUmEzWEZmTnN6dwopjuxDFla4KL5Lc7QztJeP47htzPCFf6Q8tv0T1EQ44kn+TdSwA9KH0f/yOnnLKzv3EGh6fjAiTTM9fM/aUD5ked+ZbaQh78hww3iAuZZlkG144NiBIZ+amw==];";

    for i in 0..1000 {
        let findings = scanner.scan(clean_content);
        assert!(
            findings.is_empty(),
            "Clean content should have no findings at iteration {}",
            i
        );

        let findings = scanner.scan(secret_content);
        assert!(
            !findings.is_empty(),
            "Secret content should have findings at iteration {}",
            i
        );
    }
}
