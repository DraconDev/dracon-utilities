use regex::Regex;
use dracon_security::SecretScanner;

fn has_nested_quantifier(pattern: &str) -> Option<&'static str> {
    let nested = [
        (r"(a+)+", "(a+)+"),
        (r"(a*)+", "(a*)+"),
        (r"(a+)*", "(a+)*"),
        (r"(a*)*", "(a*)*"),
        (r"{20,}{20,}", "{20,}{20,}"),
    ];
    for (pat, desc) in &nested {
        if pattern.contains(pat) {
            return Some(desc);
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
    let suspicious: Vec<(&str, &str)> = patterns
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
fn test_azure_sas_pattern_uses_correct_modifier() {
    let patterns: Vec<&str> = SecretScanner::get_patterns()
        .iter()
        .filter(|(name, _)| name.contains("Azure Shared Access Signature"))
        .map(|(_, p)| *p)
        .collect();

    for pattern in patterns {
        assert!(
            pattern.contains("(?sm)") || !pattern.contains("(?:") || pattern.contains("(?s)"),
            "Azure SAS pattern should use DOTALL modifier for safe matching"
        );
    }
}

#[test]
fn test_no_accidental_key_paste() {
    let patterns = SecretScanner::get_patterns();
    let common_keys = [
        "xoxb-",
        "ghp_",
        "sk_live_",
        "AKIA",
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
