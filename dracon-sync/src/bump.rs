pub(crate) fn extract_version_from_cargo(content: &str) -> Option<String> {
    let mut section = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed.trim_matches(&['[', ']'][..]).trim().to_string();
        }
        if section == "package" || section == "workspace.package" {
            if let Some(rest) = trimmed.strip_prefix("version") {
                let rest = rest.trim_start().trim_start_matches('=').trim();
                if let Some(v) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

pub(crate) fn extract_version_from_json(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    if let Some(idx) = content.find(&needle) {
        let key_pos = idx;
        let after_key = key_pos + needle.len();
        let rest = &content[after_key..];
        let colon_rel = rest.find(':')?;
        let after_colon = after_key + colon_rel + 1;
        let rest2 = &content[after_colon..];
        let q1_rel = rest2.find('"')?;
        let q1 = after_colon + q1_rel + 1;
        let rest3 = &content[q1..];
        let q2_rel = rest3.find('"')?;
        let q2 = q1 + q2_rel;
        Some(content[q1..q2].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_from_cargo_package() {
        let content = r#"[package]
name = "test"
version = "1.2.3""#;
        assert_eq!(
            extract_version_from_cargo(content),
            Some("1.2.3".to_string())
        );
    }

    #[test]
    fn test_extract_version_from_cargo_workspace_package() {
        let content = r#"[workspace.package]
version = "2.0.0"

[package]
name = "test""#;
        assert_eq!(
            extract_version_from_cargo(content),
            Some("2.0.0".to_string())
        );
    }

    #[test]
    fn test_extract_version_from_cargo_no_version() {
        let content = r#"[package]
name = "test""#;
        assert_eq!(extract_version_from_cargo(content), None);
    }

    #[test]
    fn test_extract_version_from_cargo_igno[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBPY2VybWdWZmFwamIwY0NOSXRCVXNOQURrNGRneWk2b0ZXWFl5aWRqOVh3Cm5mNVovWlZiRVZvemwyZWw3SjdkMktwY0ZQUCttQ2dWaWtYb3puR1RzUGsKLT4gWDI1NTE5IEZ2L05IYXNXeUp4UHM2ZVFVVmQ2M0t3RnRDamliTk5CWDE3M0JDVEo2U1UKeXMzaTlwTFppOWxROXF1eEJNc0p5RWRjWGN2ZmFnZlMvWTVETzA4Mjc1TQotPiBYMjU1MTkgNW03MS94YnE1UHdSaXIyaFVyZkJNc25FWjVpNGxlY3V6Yyt0SVhFaWZWVQo2YnRReVBtZ1cvZzVpSHNyWGVVQjZRYllzazlGZTlma0JEN0JUSzF4bFhzCi0+IFgyNTUxOSBsRW85TmhVVTk0NFJLUnFUS3dKL1cwNHQ2M0VyRjFrYUIyOENiSWM3c0JRClFIc1B4SjJBcjFKN1EwMnVoLzRJYjRrTnJ3VmkzeG9STHB2bkREbkJTYWcKLT4gWDI1NTE5IFdtRFpORVlqMHN1MnNodnI1U2RQQU11dVIyTjViUkg4aXIvdW9yaUZxaUEKQWNOWmlBbU1XN3BQRjllOGZueElDTkxaK0hXU1Q2YW9BN2xLRHNDWmRaMAotPiBYMjU1MTkgeHhHMTZMMW50cUxsTWdmUk1KbE1ONFI1emJ6OGdvOWtMcE5XbklqY3BEZwp0MktzeG1OOUxGMlB3ZUZaU0IvVjUxTGZSdXBkMVpidkd6ZFllaGVFZjlVCi0+IGxRKmVTLWdyZWFzZQpZaXMKLS0tICtUaWdoZzJkL0pBQlJDeVcxTEFYNU9GRE1qbW1MeWJSdThQQ1lnN2Q0RW8KVmJTINNtTzkTp+Idq18yOQaHkpgoRobWAypo0Lc4+ql1AOyNlYMWYPZ0fQBRfPJNqUUqG6MmndRM4hlj]() {
        let content = r#"[workspace]
members = ["crate1", "crate2"]

[package]
name = "test"
version = "1.0.0""#;
        assert_eq!(
            extract_version_from_cargo(content),
            Some("1.0.0".to_string())
        );
    }

    #[test]
    fn test_extract_version_from_json() {
        let content = r#"{"version": "1.2.3"}"#;
        assert_eq!(
            extract_version_from_json(content, "version"),
            Some("1.2.3".to_string())
        );
    }

    #[test]
    fn test_extract_version_from_json_not_found() {
        let content = r#"{"name": "test"}"#;
        assert_eq!(extract_version_from_json(content, "version"), None);
    }

    #[test]
    fn test_extract_version_from_json_multiple_keys() {
        let content = r#"{"name": "test", "version": "1.0.0", "other": "value"}"#;
        assert_eq!(
            extract_version_from_json(content, "version"),
            Some("1.0.0".to_string())
        );
    }
}
