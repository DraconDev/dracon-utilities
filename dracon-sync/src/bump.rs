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
    fn test_extract_version_from_cargo_igno[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBsREdUQ1VveDBwN01GQlNJNXo0Q0NQekNiSkRkMStxQWRxMWw1YndTZlhrCkNiMlZOTG9GeXUrNjZBZ1UxNHRtQzIvbFA2dFU1VkxtQytOMXJnOTdBMzQKLT4gWDI1NTE5IHV5UHdmVjBhVHd6SkhuOGIrb3lFcXJhV29lbTlqd2pBMU9WUENwVitOMEUKOUEvS1ZNRjF2MENkZnZrMnBrN3Z4WXlLakFtMW03a29GWEJmY3pUaVRvbwotPiBYMjU1MTkgYUJWQ2o1dTZvYXlOVzlqLzAzLytPOHpFUW1GY1pCZnFQeEVNOXJROHpIUQpaLzlLcUhKNGpNaTh5SlVNL2ZaSVRKWERUSDFMNFNMUCtFNDRIcmViVkp3Ci0+IFgyNTUxOSByTVQ4N2U1Q0E3Rk9oWkR2WTlyRjV4bUNLTWQ0V0RrWDlhbGFCK1dSY1hvCmZseWxpeU5DRjVSQW5zQUNkSGJWY2VlU0w3Z1dpYjFZQVFZd0p5cTd3YkUKLT4gWDI1NTE5IDRrNWtJRHBHMlNhNFZ1N3B4czJjL3NIR2FsaGRDdENpbHZHZVVPblVsWFUKeGczWUpxbVF3L3h6Z0M5TEdabmEzTFc1bkJMeGdSVWl4T1A4Rnhtc0pEVQotPiBYMjU1MTkgbHhUWWs2enJvYzVIRmJ5ZUxuUUZiZXFNeU9JWmJiMCtsVUdseGdlN0pSawpweDUrWmJ4d1BmaFEyOWhMckpGM3V3YkdiNndDUkNKN0l6WWYwbjJtWHVvCi0+IGFKPS1ncmVhc2UgdU02KGIKc1FtbGo5ak8KLS0tICtON250RTM3KzlrRjdycmhSQmFGMUFEblE2bDZpS29tK3pYbjR0bmw1Qm8KHw20q31PXDeXLno32Bcp0jzhBbjNgpm+56FhYjtI1v06A070BqUfv8JsRxeqO/vyBotAERsVBF0o55tc]() {
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
