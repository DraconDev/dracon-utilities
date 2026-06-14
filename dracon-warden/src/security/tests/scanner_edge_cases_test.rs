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
    let unicode = "日本語と한국어_[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBhWkMxMElIQ2dqZmFZdlRIOFd6aXFMU3R3T05pYlE2azY5citUT3MyMEJVCkhnS1A0NGNhYks5ZEY1K0lJZDY5ZENlVzFvQm9TMWdtYU5RZnNEbEY0TTAKLT4gWDI1NTE5IERIdHliVUQ3VTlHUDVWV05tdUc3MURLc1lWVG1rK2E4OENSVVZvMUFYQVEKTWZMMmVuanYwR0tydGtTaU96SmNpNnpyQStlcm05Skl2aWpBNURIK0k3NAotPiBYMjU1MTkgODEvNk1NQkxzeWQ3eTgzSzV3NDJKUXovRlI5RDVDRGdWaDBmVEJzUldETQo3Y1gybjI4S1JBR1UrLyt4RmxnTFYrM28yOUw2UlpOMndqVHlhZWpqaW04Ci0+IFgyNTUxOSBEUktJd25odERLcWFrdEtJeXArRjRJU3pxQVJRcmI3QVgxdDFuT0R5anlJCkNKUThQbitmbVR4eDgzS005bndpUi9Rd01nRUppUEVlNWQvbDhKQzNoaW8KLT4gWDI1NTE5IHV3OHNtWmpMMkZSMm9kL1FJcWRaWWxpbkFnYnA2dWNFazM0aWNlVGJqV0kKbE8xREFVamVleXFlUjNNcUVWaTVWb3JER0hwakVaZzAwRHIybitVTE9vNAotPiB+LWdyZWFzZSBlLk9Md0wKN2pWTEZFVCtzVUFjZkJ6S0FYTEZIc2lRZ3lUbjhHekNNMkc3NDlmTHkxdHVsa3pabllRcitZMWxDc2JkOFlILwpSamcxbmcKLS0tIE9OcUNaNnBtdFA3SFVFbUFhTXlZVHR3N1dFeEhOV0dWTHl0cFE2TkhEeGMK1TIU2s3uifKY0WxLxJBiaSzOpqoEuz6SPe2Ku7nPBWYjil4fsm995DcASu75oIRl0o13cvnvo7kGwxebDIDDStlF]";
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
