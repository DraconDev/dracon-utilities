use regex::Regex;
use dracon_security::SecretScanner;

fn has_nested_quantifier(pattern: &str) -> Option<String> {
    let nested = [
        (r"\(\.[+]\\)+\\)", "(a+)+"),
        (r"\(\.\*\\)\+\)", "(a*)+"),
        (r"\(\.[+]\\)\*\)", "(a+)*"),
        (r"\(\.\*\\)\*\)", "(a*)*"),
        (r"\{[0-9]+,\}\{[0-9]+,\}", "{20,}{20,}"),
    ];
    for (pat, desc) in &nested {
        if pattern.contains(*pat) {
            return Some(desc.to_string());
        }
    }
    None
}

#[test]
fn test_patterns_integrity() {
    let patterns = SecretScanner::get_patterns();

    for (name, pattern) in patterns {
        if pattern.len() > 300 {
            panic!(
                "Pattern '{}' is suspiciously long ({} chars). Did you accidentally paste a key?",
                name,
                pattern.len()
            );
        }

        assert!(
            Regex::new(pattern).is_ok(),
            "Pattern '{}' failed to compile: {}",
            name,
            pattern
        );
    }
}

#[test]
fn test_no_nested_quantifiers() {
    let patterns = SecretScanner::get_patterns();
    let suspicious: Vec<(String, String)> = patterns
        .iter()
        .filter_map(|(name, pattern)| {
            has_nested_quantifier(pattern).map(|q| (name.clone(), q))
        })
        .collect();

    assert!(
        suspicious.is_empty(),
        "Patterns with nested quantifiers found: {:?}",
        suspicious
    );
}

#[test]
fn test_azure_sas_pattern_not_vulnerable() {
    let patterns: Vec<(String, String)> = SecretScanner::get_patterns()
        .iter()
        .filter(|(name, _)| name.contains("Azure Shared Access Signature"))
        .cloned()
        .collect();

    for (_, pattern) in patterns {
        assert!(
            !pattern.contains("(?:") || pattern.contains("(?sm)"),
            "Azure SAS pattern should use DOTALL modifier or avoid nested alternation"
        );
    }
}

#[test]
fn test_no_accidental_key_paste() {
    let patterns = SecretScanner::get_patterns();
    let common_keys = [
        "xoxb-",   // Slack real
        "ghp_",    // GitHub real
        "sk_live_", // Stripe real
        "AKIA",    // AWS real
    ];

    for (name, pattern) in patterns {
        for key_prefix in common_keys {
            assert!(
                !pattern.contains(key_prefix),
                "Pattern '{}' contains key prefix '{}' — may be a pasted key",
                name,
                key_prefix
            );
        }
    }
}
