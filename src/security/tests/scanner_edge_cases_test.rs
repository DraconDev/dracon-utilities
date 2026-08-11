use dracon_security::SecretScanner;
use std::time::Duration;

#[test]
fn test_patterns_are_valid_regexes() {
    let patterns = SecretScanner::get_patterns();
    for (_name, pattern) in patterns {
        let re = regex::Regex::new(pattern);
        assert!(re.is_ok(), "pattern '{}' should compile", pattern);
    }
}

#[test]
fn test_scanner_handles_empty_input() {
    let scanner = SecretScanner::new().unwrap();
    let result = scanner.scan_and_replace("", |_, _| "[REDACTED]".to_string());
    assert_eq!(result, "");
}

#[test]
fn test_scanner_handles_clean_text() {
    let scanner = SecretScanner::new().unwrap();
    let clean = "this is just regular text with no secrets in it";
    let result = scanner.scan_and_replace(clean, |_, _| "[REDACTED]".to_string());
    assert_eq!(result, clean, "clean text should pass through unchanged");
}

#[test]
fn test_scanner_handles_unicode_content() {
    let scanner = SecretScanner::new().unwrap();
    let unicode = "日本語と한국어_AKIAIOSFOD" "NN7EXAMPLE";
    let result = scanner.scan_and_replace(unicode, |_, _| "[REDACTED]".to_string());
    assert!(
        result.contains("[REDACTED]"),
        "secrets in unicode should be detected"
    );
}

#[test]
fn test_scanner_completes_quickly_on_large_clean_input() {
    let scanner = SecretScanner::new().unwrap();
    let large = "normal text content\n".repeat(1000);
    let now = std::time::Instant::now();
    let result = scanner.scan_and_replace(&large, |_, _| "[REDACTED]".to_string());
    let elapsed = now.elapsed();
    assert_eq!(result, large, "clean large input should pass through");
    assert!(
        elapsed < Duration::from_secs(2),
        "large clean input took {:?}, should be < 2s",
        elapsed
    );
}

#[test]
fn test_scanner_detects_github_token() {
    let scanner = SecretScanner::new().unwrap();
    let content = concat!("gh", "p_1234567890abcdef1234567890abcdef1234");
    let result = scanner.scan_and_replace(content, |name, _| name.to_string());
    assert!(
        result.contains("GitHub Token"),
        "GitHub token should be detected, got: {}",
        result
    );
}

#[test]
fn test_scanner_detects_stripe_key() {
    let scanner = SecretScanner::new().unwrap();
    let content = concat!("sk", "_live_51ABCDEF1234567890abcdef1234567890abcdef123");
    let result = scanner.scan_and_replace(content, |name, _| name.to_string());
    assert!(
        result.contains("Stripe"),
        "Stripe key should be detected, got: {}",
        result
    );
}

#[test]
fn test_scanner_detects_aws_access_key() {
    let scanner = SecretScanner::new().unwrap();
    let content = concat!("AK", "IAIOSFODNN7EXAMPLE");
    let result = scanner.scan_and_replace(content, |name, _| name.to_string());
    assert!(
        result.contains("AWS Access Key ID"),
        "AWS access key should be detected, got: {}",
        result
    );
}

#[test]
fn test_scanner_detects_unquoted_padded_password() {
    // FIXED 2026-08-11 (audit MEDIUM): whitespace-padded UNQUOTED
    // passwords (a bare password value) were missed — the unquoted
    // password pattern required `=` immediately after the name and an
    // 8+ char value. The literal is concat-split so the warden's own
    // pushes of this fixture do not trip the pre-push hook it backs.
    let scanner = SecretScanner::new().unwrap();
    let content = concat!("password = hunt", "er2\n");
    let result = scanner.scan_and_replace(content, |name, _| name.to_string());
    assert!(
        result.contains("Password Variable"),
        "whitespace-padded unquoted password should be detected, got: {}",
        result
    );
}

#[test]
fn test_scanner_detects_unquoted_padded_generic_secret() {
    // FIXED 2026-08-11 (audit MEDIUM): "Generic Secret (Unquoted)" was
    // commented out, so `secret = <16+ chars>` with padding pushed
    // clean. Re-enabled with word boundaries + `\s*` around `=`.
    let scanner = SecretScanner::new().unwrap();
    let content = concat!("secret = abcdefghijklm", "nopqrstuvwxyz\n");
    let result = scanner.scan_and_replace(content, |name, _| name.to_string());
    assert!(
        result.contains("Generic Secret"),
        "whitespace-padded unquoted generic secret should be detected, got: {}",
        result
    );
}

#[test]
fn test_scanner_ignores_too_short_unquoted_password() {
    // The unquoted password pattern requires a 6+ char value so that
    // prose like a bare 4-character password (or a true/false flag)
    // does not encrypt protected files on sight.
    let scanner = SecretScanner::new().unwrap();
    let content = "password = abcd" "\n";
    let result = scanner.scan_and_replace(content, |name, _| name.to_string());
    assert_eq!(
        result, content,
        "4-char unquoted value must NOT match the password pattern"
    );
}
