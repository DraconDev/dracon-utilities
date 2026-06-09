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
    fn test_extract_version_from_cargo_igno[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAzTlA2cjMrNnZXdVNURlZsTXF3L3JFSXdGbDNEZTNwLzNQVzgzOHFFdVZVCk1aV2dRUFdsVnlPZE5qSkxDaURSRmZRR3VyU0lmZHNaK09FMGh1RDI3UUkKLT4gWDI1NTE5IDE3c2lDMFROOTgxOVVXWDZRdVh4WGdrekI0WlByeElXWmhkTml5NjN3MFUKVENLdUpCUW1naWNPY0ZQUGVEQjJKQ1BQWWtWZ1F5eUlONnZtTng5cE1tOAotPiBYMjU1MTkgY1FFVVFGSHNpaVFSS1dMVFBLUkwvUlhQckNFM0IwYnJQd2E5Y2RBalRudwptWUZycWFjaEkzMDVGSXlUalNWM1phM3A4b1l1cmFUN1BDalhZSWthKzNjCi0+IFgyNTUxOSBsemFyT3RuM21yZ1VVWDdFMVhMME8vd0ZjN0dOZE85OW5RVXBhM2ZJSWlzClQ5WVozYm9kUEw4bDVzTVI5QUQyanZHTk9KekFiczVCVHJsNEZVTFhTd2cKLT4gWDI1NTE5IDd6MW5qTVdVM2lvN2hhWE8zL1hDUnJJcHVvNXVkNFpUSEF6UzE3dFd5VFUKU0h1Tkg5cDhNeThsRnNRWVdtVXdKdlZuR2p6WUkyK3BGblNKV096U2MxTQotPiBYMjU1MTkgVk9nYXY4bGpzL2djVUUrNlpzU0RDbGhGaGp4YXNsUVk0Vmp4dEYvMU0zRQppallXNkY0aHlqMzNtS2Nlai94M0R6RU1HS1R6YUJFWCtkRDZZVGxGQWlFCi0+IHV0Ly1ncmVhc2UKci9PTit4Y052R2xHMXhSTHRpbzdScGhaYXJBUXVnUmo0bmNYWFVSYVZwNnl3TDl4K3lleEE4RzdFaHpEcGQ1cwpMUGVIS05wa1JFV2c2bGRGQ1QrMC93aEVodwotLS0gRFdWdWNmYkkxMmJOWSt4ZDRXQ2JTVkpzZXpxV2djcHhodXJUcXovbzJDMAosGgWsIxL9nrLsW2kni/IBqiWuSpSEs8K8Gtb3p+d+1/7lIe9p+Fw9eNc4dyXmCP6dtuQCgcpVHevhXV0=]() {
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
