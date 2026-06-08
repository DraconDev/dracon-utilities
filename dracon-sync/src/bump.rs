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
    fn test_extract_version_from_cargo_igno[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBFR1g0ZlJpdFFYbVlQbm5zWmpVdHJoRHU4UFU5Z2hZdWl5eTJzRFZsNGg4CjFXMmU2blAvTG5QVWdTNVBGcGxkRGo5aU5mUDEyN1hUZEw1V0xScklxUVEKLT4gWDI1NTE5IFA4eUhyTllUT25TejNuZ3pIVlNYVThqc2wyMitPTXd1Z0o3RG5XNDBFVjAKMUpwN2RnczJYQ1pKb3R0RnpKNDZpYVNSdW1RZ2VNdU1RYVBkTEFCZm5nTQotPiBYMjU1MTkgaFB1K2FCN3NUN2Z0d1diVmZuSVFXaWd6UWxSM28yaFVHeTlSNXF2WW8wUQpVVXNMV1pFbytEd3dIUHE0OTI2YWp2aHVFUWUzQTljajhWZHNaNGgyNE93Ci0+IFgyNTUxOSA4cFI1Mk5Fc0l6bVRQT0lzNDFWWUNJbEU1ZnFvYit6QVo5RE1Jb1hHQkRRCnZGMFFoSDVqVFRyQ0NFZ1Q3eEpIRUV3cytBOFdsKzdTZUlMTDIyS1RJRFEKLT4gWDI1NTE5IHJKb3J0d0Y0U0tHVWhSYUpYV0s4MXpGdkc1SW9zT2toc0JvMEthNGtLekEKTGRDdFMrM2V1Z0ZXWkIyM2h1eHhzLzJjSUpQTVZPV0Y4Z2VnVDMrN0tDWQotPiBYMjU1MTkgM1FkMnhmUTAwQmJ0TFJ1S2U3V1NUOElSMUdaYjV4cDVEVjRZUkpVekJFOApqNXJtWTdSY1ZYVjg1K2Z4bUlsRUJRL0V6OEV4QXplazhMRFNhaXFMaitvCi0+IEIpLWdyZWFzZSB2OVIlCnRCNHYrdEhCZ3VDeWpwdWtmS2JXNkF3YzdLMVNHQytqeUoyNVhxb0IrbGI3ck1sRGhGQ01PZC9FTHNzdwotLS0gYktkMmdiaWFZVjJZVGVRU3BFN3pqSllGcUhzc0tyUlBrSFk0ZE5qaDUrdwp7n2LfJdjOQ9WbuxlAOb7onviz04Tr38PL/Xs1DzbMM0vpOt5fB6HCewUnwS+/mTQSV2TcbzNmxziCurE=]() {
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
