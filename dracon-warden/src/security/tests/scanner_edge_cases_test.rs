use dracon_security::SecretScanner;
use regex::Regex;

#[test]
fn test_patterns_are_valid_regexes() {
    let patterns = SecretScanner::get_patterns();
    for (name, pattern) in patterns {
        assert!(
            Regex::new(pattern).is_ok(),
            "Pattern '{}' ( '{}' ) failed to compile",
            name,
            pattern
        );
    }
}

#[test]
fn test_patterns_are_not_suspiciously_long() {
    let patterns = SecretScanner::get_patterns();
    for (name, pattern) in patterns {
        assert!(
            pattern.len() <= 300,
            "Pattern '{}' is {} chars (limit 300). Did you paste a key by accident?",
            name,
            pattern.len()
        );
    }
}

#[test]
fn test_azure_sas_no_catastrophic_backtracking() {
    let scanner = SecretScanner::new().unwrap();

    let safe = "sv=2019-02-02&se=2020-01-01&sr=b&sig=dD8%2Bfd2%2B1234567890abcdef1234567890abcdef123%3D";
    let result = scanner.scan_and_replace(safe, |name, _| {
        format!("[MATCHED:{}]", name)
    });
    assert!(
        result.contains("Azure Shared Access Signature"),
        "valid SAS should be detected"
    );

    let evil_repeating = format!(
        "sv=2019-02-02&{}",
        "&sig=".repeat(30)
    );
    let result2 = scanner.scan_and_replace(&evil_repeating, |name, _| {
        format!("[MATCHED:{}]", name)
    });
    assert!(
        result2.len() < evil_repeating.len() * 2,
        "ReDoS input should not cause explosive output growth"
    );
}

#[test]
fn test_generic_assignment_no_catastrophic_backtracking() {
    let scanner = SecretScanner::new().unwrap();

    let safe = "MY_API_SECRET_TOKEN=abcdefghij1234567890ABCDEF";
    let result = scanner.scan_and_replace(safe, |name, _| {
        format!("[MATCHED:{}]", name)
    });
    assert!(
        result.contains("Generic Assignment"),
        "valid assignment should be detected"
    );

    let long_key = format!("MY_API_SECRET_TOKEN={}", "A".repeat(100));
    let result2 = scanner.scan_and_replace(&long_key, |_, _| "[REDACTED]".to_string());
    assert!(
        result2.len() < long_key.len() * 3,
        "long key should not cause exponential expansion"
    );
}

#[test]
fn test_scanner_handles_empty_input() {
    let scanner = SecretScanner::new().unwrap();
    let result = scanner.scan_and_replace("", |_,_| "[REDACTED]".to_string());
    assert_eq!(result, "");
}

#[test]
fn test_scanner_handles_binary_like_content() {
    let scanner = SecretScanner::new().unwrap();
    let binary: Vec<u8> = (0..255).collect();
    let result = scanner.scan_and_replace(std::str::from_utf8(&binary).unwrap(), |_,_| "[REDACTED]".to_string());
    assert!(result.len() <= binary.len() * 2, "binary content should not explode");
}

#[test]
fn test_scanner_handles_unicode_content() {
    let scanner = SecretScanner::new().unwrap();
    let unicode = "日本語API_KEY=abcdefghij한국어TOKEN=xoxb-123456";
    let result = scanner.scan_and_replace(unicode, |_,_| "[REDACTED]".to_string());
    assert!(result.contains("[REDACTED]"), "unicode with secrets should be scanned");
}

#[test]
fn test_scanner_handles_large_content() {
    let scanner = SecretScanner::new().unwrap();
    let large = format!("API_KEY=abcdefghij{}\n", " normal content".repeat(1000));
    let result = scanner.scan_and_replace(&large, |_,_| "[REDACTED]".to_string());
    assert!(
        result.len() < large.len() * 2,
        "large content should not cause memory explosion"
    );
}

#[test]
fn test_scanner_detects_all_corpus_secrets() {
    let scanner = SecretScanner::new().unwrap();

    let test_cases = vec![
        ("ghp_1234567890abcdef1234567890abcdef1234", "GitHub Token"),
        ("sk_live_51ABCDEF1234567890abcdef1234567890abcdef123", "Stripe"),
        ("AKIAIOSFODNN7EXAMPLE", "AWS Access Key"),
        ("xoxb-123456789012-1234567890123-AbCdEfGhIjKlMnOpQrStUvWx", "Slack"),
        ("AIzaSyD-1234567890abcdef1234567890abcde", "GCP"),
        ("ya29.a0AfH6SMC_1234567890abcdef1234567890abcdef1234567890", "GCP OAuth"),
        ("aws_session_token = abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890/+=abcdef", "AWS"),
        ("LTAI1234567890abcdef1234", "Alibaba"),
        ("sk_test_abcdefghijklmnopqrstuvwx123456", "Stripe Test"),
        ("ocid1.user.oc1.test.abcdef1234567890abcdef1234567890abcdef1234", "Oracle"),
    ];

    for (secret, _expected_name) in test_cases {
        let result = scanner.scan_and_replace(secret, |name, found| {
            format!("[{}:{}]", name, found)
        });
        assert!(
            result.contains("["),
            "secret '{}' should be detected, got: {}",
            secret,
            result
        );
    }
}
