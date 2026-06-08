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
    fn test_extract_version_from_cargo_igno[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBiMUxlZHhTaWZYdDRZRzNCRXdweUhZRGE2M2svMDRkaDRrVDF0V0xCVUFFCjNZUi9FTVozbE0zNU5jdXIrQmxyMWpmNm5FTWpTdmRkaS9NMUdMV29KOVUKLT4gWDI1NTE5IDNHcUt1RzlnUHVScW1ON2I5Z05sc2V5UmlQV2dETUF5K2NJSThuYUsvQlkKOWFBYkV5K2JHUS9zZW5ldVhDUHFZb0FBM0hJQ3AwOGkzTy9FL1d3aXhPNAotPiBYMjU1MTkgcmplOG5aQnVybzhOeVkvK0V1L2F2OStjNjVVTkd0d0N0Y3J2YXFvL1BodwptUFhRd3Q0VnEvZUs3RjIvTVc2QUY0SEp3ME1ac2tSa21nYkl5YWhiRlhzCi0+IFgyNTUxOSB5eUVGWlZnbUtwMU1EemZBVnRnVjRXekcycitEMndUN242MHdwWTNrVTNRCnRIck1jcnUrMGZrb2tMTmhNemMzbnhGWFA3MzJSTjgrbzAwWHNqWG9WUEEKLT4gWDI1NTE5IGRXR0pHeFVrVG5sdTlHQkdIR0dTWVp1dTRLVTFOcHpXcmdzTVRIRHZ3Qm8KRkhXREFsdTNNMFNObjBnR1UwYm5OUmNsR2hxTE5POEhMN25laEQ5bTRabwotPiBYMjU1MTkgcHExSk8xdGRLSm9jQVNOWmhHVFlNQytPTGVNdmZGTUlLbUMyUXZCeUl6TQo5cVVoeXBQVXVKanZ1N3JOTjFkalphNHlLQS9XYlE4ZlB1dlJqTDl1SjdvCi0+IChLWy1ncmVhc2UgJnkrPiBzM1EgdGAvVms7S0AgLUxzLQppdXJ0cW1xbnBJMWJ3QkJpMWNYNjlTRTZNK2xtUVdCY2Vtb3czSDhvTmRTMU52R1IKLS0tIE1qS2JFSk5MdVVrN0h5ZTVyRHJtVFFCbW9NWmZPdEIrUUhqalJXcnEzQWsKFOfsXXRRRTCTFN9eYiPQIvRZo9F4OguQuMusBZ/jlg+Z74O0NRztG+nEl/CTPoKjJFCntJC71wr2pkyF]() {
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
