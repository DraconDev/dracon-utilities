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
    fn test_extract_version_from_cargo_igno[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBUa0VCSU1PaFpMOUJweHNWUWFTd1Rnd2hjdmxmdHE4MUttNzltdi91dTJjCnZqZUYwdGQ3cmFlK2lqTksyQXl3d1gzcWtPdzRyKzZqejM0QlI1SFhCM1UKLT4gWDI1NTE5IDc5REF2OHkvMFczUTBNWDBpbEVrbWhVa1VTTi9lblFFQWxubEdqVktzUlkKU293eDgydDlIRVptNDl0MEJaREhnZURpQkN6V1dKcGppL08xNU5NNTh3SQotPiBYMjU1MTkgWDdndUxkL1o2MmhVNkZ6dGl0WW9PdFcxNUVyNzZhbUtheURDUGlYUXNRawpOQUhwSnkzbmpaVldIeTZFRkJvUWtsQThSNUUzaFpMMHpHcFJhZExGZHdNCi0+IFgyNTUxOSBlYzN5KytPSVppcElxRTVpY1BEbGM5MHhjSXdJNjgvTWM4ZitXSmh4RWhjCnYreWtWVXY0NWZyREpuLzY3SDJyOFUrdnFtRUIzKzEvcDFjQ3hhZm9SbGsKLT4gWDI1NTE5IDg0VVRMZ0hJeE5CNG1YY0lSYyttTEhHS1d1cC95MW8yakxvZFdYOUUrRmcKeDYxdGk1ZU0vbDRsR3Q3RldTbHloVkN0VXBUTW5uTzlqNHF3K0R4cVdvTQotPiBYMjU1MTkgaVo3WXlad2JrdXdUV1V1WE1jSjFMTkJRLzRtaWhnMU9TL2VUeXJsVGtScwppdThZRWoweDAvSnI5Y0VrYmU2MTF3UFd4ZnhCZWlTd1V1VjYrM2dCOTRnCi0+ICFuOC1ncmVhc2UKa0hNWlMyYWMzT2hNV1pHZDkwSUxTNVh4V3I3bElhRnhqcnJVMlFiMjQ5RE93R2tTMHhvdUl3eVZPTEFRS2FVMgpxZnBqblFPWkpST25PNTdySVJRNjAxaDVZcXFrRE90amIrQXk5Vm8rM3YzY2huZW5wNlpUTkIzYkRSNAotLS0gQVlIWU1XMVNTcVZIREZydlRJbEQ3bGxYQURXbDRNU1hXdlZuT2kvSStZWQqDIrvOHpGkfppP2gj5ynb+HjuSSrFoCJdnxa/QDsevQKGIkE1+e6iVSwPcpc3mLSPRuWidmuC1PabQ7Aw=]() {
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
