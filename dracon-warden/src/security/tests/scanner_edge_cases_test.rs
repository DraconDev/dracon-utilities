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
    let unicode = "日本語と한국어_[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBLL2ZrUHcxbHZxbjh3R2dEMzlhc3pJbFkxUERpQ3lBS1NLVk5uK1lDdEdBCndoL3daMkdBOXBCV252ckZIbjVZTkxlKzVySnU2aExSdm5kNkVMT2NjclUKLT4gWDI1NTE5IHU3TG9TbjJnTnZqelRtbDZmU0F1Y0htSUw5bm96VHRDV3lOSGg0MVlPbHcKNVl1Rm9uaG43ZXcrRjEyQnA3ZXFLUm9ISm5SdkpwNmN0M0dqc3ZHN2VuSQotPiBYMjU1MTkgQjR3NmFESXhZT05raGU4SDRERXdqem9KVHg1ZzB1bnRRdXFxdWlLck9rcwpybkhIUEhSM0xuWU1hVGhrYWV6b1RJc3k1bVpDV0NxSkNmQU5xbWxYY0ZZCi0+IFgyNTUxOSBQVzNUZ0hMNTg4dTFnREhrc1pra0E4TnVCUy9ISWNOOGQ3aGNJTVZXaFJBCmFGaFFzM3NsNjlNMUV6aFZmb2V5czdJaU1ZUDZWZ2t5SmZ6MmJUT0x1QXcKLT4gWDI1NTE5IEZ0V0l2K29NdGJqcmU0NG54RFllWXoyTmVpN092b0FCdTZPK1VoYjJ3UTgKMkdFaHJiQ1VaOGp0YnZycDlFWlRzZ25MQ2JpWS9RMkpHL0YrUzhVd0s3VQotPiAwLWdyZWFzZSBee0JMUkY/NSAwZmxBWjEhLCBacTk4N25+Nwo5VzRLCi0tLSAzb0F4a3hnK3c0eEZmUUZHZis4c2dYYnpJYzdJeHVTUU1LcjBoaUQ4QkFNCmXETF+0v6ipUy1yxAf6xvhAWF/VBVb748ym3yFKAeeNZizQ8+NMs+B0Aua1vMlXionveJGWHre15hPZ4FzbUUp8Ig==]";
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
