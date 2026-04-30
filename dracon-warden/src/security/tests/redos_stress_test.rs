use dracon_security::SecretScanner;
use std::time::Duration;

#[test]
fn test_azure_sas_pattern_completes_in_reasonable_time() {
    let scanner = SecretScanner::new().unwrap();

    let input = "sv=2019-02-02&sig=abc123def456".to_string();
    let now = std::time::Instant::now();
    let _result = scanner.scan_and_replace(&input, |_,_| "[REDACTED]".to_string());
    let elapsed = now.elapsed();

    assert!(
        elapsed < Duration::from_secs(1),
        "simple Azure SAS scan should complete in < 1s, took {:?}",
        elapsed
    );
}

#[test]
fn test_generic_assignment_pattern_completes_in_reasonable_time() {
    let scanner = SecretScanner::new().unwrap();

    let input = "MY_API_KEY=abcdefghij1234567890abcdef".to_string();
    let now = std::time::Instant::now();
    let _result = scanner.scan_and_replace(&input, |_,_| "[REDACTED]".to_string());
    let elapsed = now.elapsed();

    assert!(
        elapsed < Duration::from_secs(1),
        "simple assignment scan should complete in < 1s, took {:?}",
        elapsed
    );
}

#[test]
fn test_scanner_performance_under_evil_input() {
    let scanner = SecretScanner::new().unwrap();

    let evil = "x".repeat(10_000);
    let now = std::time::Instant::now();
    let result = scanner.scan_and_replace(&evil, |_,_| "[REDACTED]".to_string());
    let elapsed = now.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "10k-char evil input should scan in < 5s, took {:?}",
        elapsed
    );
    assert!(
        result.len() < evil.len() * 3,
        "output should not explode on non-matching evil input"
    );
}

#[test]
fn test_scanner_performance_mixed_secret_and_evil() {
    let scanner = SecretScanner::new().unwrap();

    let secret = "API_KEY=super_secret_value_12345";
    let filler = "x".repeat(5_000);
    let input = format!("{}\n{}\n{}", filler, secret, filler);
    let now = std::time::Instant::now();
    let result = scanner.scan_and_replace(&input, |_,_| "[REDACTED]".to_string());
    let elapsed = now.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "mixed secret+evil scan should complete in < 5s, took {:?}",
        elapsed
    );
    assert!(
        result.contains("[REDACTED]"),
        "secret should still be detected amid evil input"
    );
}

#[test]
fn test_nested_quantifier_patterns_do_not_cause_exponential_blowup() {
    let scanner = SecretScanner::new().unwrap();

    let input = "password=".to_string() + &"a".repeat(50);
    let now = std::time::Instant::now();
    let result = scanner.scan_and_replace(&input, |_,_| "[REDACTED]".to_string());
    let elapsed = now.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "nested quantifier pattern should not cause exponential blowup"
    );
    assert!(
        result.len() < input.len() * 4,
        "output should not explode"
    );
}
