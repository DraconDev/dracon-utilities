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
    fn test_extract_version_from_cargo_igno[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBJSnN6SThuRys4dTRJVnNPYkJiYmlwQk5JWnp6cUJnUmVSQW1uQnA1VzFjCmJTVzg1SmVlc1V2Q2FSczh0QzNmWllyWHQrZmlOc0dGTldpSU1nOVA5c0EKLT4gWDI1NTE5IE9HdGtraGZGSmo2aktVRTZ4OWd0RTIyRWQ1aUVaM0VwUUxqdkg4QTFTU2sKZEdGcEZiWVFna0JUdnExSUxlUHhRTmlJUkE1NVY2ellWUEpPZ1hXOWJoYwotPiBYMjU1MTkgbngxYVpxL1pOa1VkUC8wNTV3eWdpeFRaWDhTWHVTVUN2bnU2MGtrZG5sUQpyRmlDdGt1SXRtQysrZmVaQnYzdUNWQzNtSCtYRWs3NXRtMXNBaU1raUlNCi0+IFgyNTUxOSBIeUtvUWk1b2ZnamQzR3QzeUxqeERubG1rNS96bWsxdUt0ZHA3RXNEbHl3CmU4d2Z4WFpKV2RLbnJxRlpTdGRTdzhhWlZCWVNkNWlmN1YzTjRPanNpSUkKLT4gWDI1NTE5IHV1NXdnb096RGt6OFowZFdoNm4rbjZoUThEMFVjYmFRSHZoMW9zaVd4a0EKbUNBZ0RmY0NMRFU1VlhYY1l6VGNJWDlHWGI1UHRnUnU2VjRHV0ZqeDhRNAotPiBYMjU1MTkgbTlmVXBBMmg3U2VITmhKNUQwNWtPWkhIcEZWb0R6WHNVUmhBYlo2c2lpWQpKZDk4UFd0cC9ocldKM2hnSFBJeFF3T1NpbzZlS05tWHRoVEVkRUQ0V0JzCi0+ICtQcGUmSSktZ3JlYXNlIF5hID0oL3pjCmpDOU41dHRneit2cm1ENDJpa1JQQ0k5N2RxMlhEN0ZzNFZrY1VRSkQ5aG85NWhpdTMyTGNhVXl2TlJFZjNyL24KS1p4a254NHp4Ykt0WGw1OHhYNzNrVDNXWGNmWmxDU3RaczQ0OTNoS0QzYkJmbE1kbEsvZVR6WEQ5NG5BbFVnUQoKLS0tIHpCUVhHUGlPQzI1czhwRFd6M254b3V1TDN3TGRCZzZvR29nUHlBaW5oVUkKkKSPhwbkjk5+2lYndl10IzwTXaPegp9sBwPqGSB8dsdZqnS2Y17uUNp6cyzDQuirPY5vLzauIzWGgslg]() {
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
