use regex::Regex;
use dracon_security::SecretScanner;

#[test]
fn test_patterns_integrity() {
    let patterns = SecretScanner::get_patterns();

    for (name, pattern) in patterns {
        // 1. Check for ReDOS risk via length (payloads vs regexes)
        // Most regexes are < 100 chars. Encrypted payloads are 1000+.
        if pattern.len() > 300 {
            panic!(
                "Pattern '{}' is suspiciously long ({} chars). Did you accidentally paste a key?",
                name,
                pattern.len()
            );
        }

        // 2. Check for compilation validity
        assert!(
            Regex::new(pattern).is_ok(),
            "Pattern '{}' failed to compile: {}",
            name,
            pattern
        );
    }
}
