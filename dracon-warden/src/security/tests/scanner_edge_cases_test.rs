use dracon_security::SecretScanner;
use std::time::Duration;

#[test]
fn test_patterns_are_valid_regexes() {
    let patterns = SecretScanner::get_patterns();
    for (_name, pattern) in patterns {
        let re = regex::Regex::new(pattern);
        assert!(re.is_ok(), "pattern '{}' should compile", pattern);
    }
}

#[test]
fn test_scanner_handles_empty_input() {
    let scanner = SecretScanner::new().unwrap();
    let result = scanner.scan_and_replace("", |_, _| "[REDACTED]".to_string());
    assert_eq!(result, "");
}

#[test]
fn test_scanner_handles_clean_text() {
    let scanner = SecretScanner::new().unwrap();
    let clean = "this is just regular text with no secrets in it";
    let result = scanner.scan_and_replace(clean, |_, _| "[REDACTED]".to_string());
    assert_eq!(result, clean, "clean text should pass through unchanged");
}

#[test]
fn test_scanner_handles_unicode_content() {
    let scanner = SecretScanner::new().unwrap();
    let unicode = "日本語と한국어_[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB2MDdXQnhQb1VJWXhSTzZJdUxCR0xLYk1MaGtTYWdTOW10akt0V05vOURVCjEvRWdQVkhTLzhhTlgzQktxQzc2LzBYakM2YlRkRFhtdjlhL1ZpalhScmMKLT4gWDI1NTE5IEhWVDh1bzY2d054RmJEanZDTnVOSjlpK3JIenhGaXQzVUptNkpDUlhiRmMKbUtvS2ZBem1OSVFXazIzdGlndjdSSElrelJ4Uk12ZkZlZk9yelRHa2lnNAotPiBYMjU1MTkgZUJRZUw3Z25RaldiaDc0ckhWcWFKc0o0Qk1JcHh2eU0weDFTUFduNmhFQQpadjA1TU14MW9zQzNGOVdtMC9WZVQ5c0NVZ1VIb2RLQ1ZsVkx6VFFVSTdFCi0+IFgyNTUxOSBzTzNnM01rUVRnOGxnaGgvK3RQVU8wM0RiYnliS1pKcytsbTBKOHBldXdrClpMQ2VoS2l1aDl0ZE1vLzN3dnNieEV3U3ZpMUUyUHBwd3Z6U1drTWF3M1kKLT4gWDI1NTE5IGNVeWZUdXY3bjZPaHZVN1F6WDJLemNnb2JFNm15TCs1VHVoVm9vQ0FBaVkKSVJnaHJuRnQzalZwZUR4SndhOHlRS3ZVN2dDZnJLcHlidU9qUEVRdWt6UQotPiBYMjU1MTkgUG5XZkdGRUt2bFBVUkhobWNkcFJBR2JtQjF5bTRqTUNndk5sbHkwZjYwdwpjU3ZBY01ML2ovMmRpWTVISTJTb0xnd01Qa1FEYjJaUVFJQ3Z6disvMFZ3Ci0+IGIzLWdyZWFzZQp1bFNuRU1Fd0gzMlJvMENFRFZJeHZMOE9UcFZzZitpUE9GbHpwRGVIYjBpWHhHdHBnQTlvaEwzQ21SV2VkTUlPCgotLS0gTXFsdEljVnhRMTRxWHRuYnFVNi9LZGErSngvY2VUbmZYbndTRXdUUG9BdwpOr4IVeM695HEvgfBlbH/7xpTu07EyFS544d5p4EXtLRJvPv4am+MZd0DsYV7Hyc/eSK2QuAlYyukr0EsiR2Y5XTk=]";
    let result = scanner.scan_and_replace(unicode, |_, _| "[REDACTED]".to_string());
    assert!(
        result.contains("[REDACTED]"),
        "secrets in unicode should be detected"
    );
}

#[test]
fn test_scanner_completes_quickly_on_large_clean_input() {
    let scanner = SecretScanner::new().unwrap();
    let large = "normal text content\n".repeat(1000);
    let now = std::time::Instant::now();
    let result = scanner.scan_and_replace(&large, |_, _| "[REDACTED]".to_string());
    let elapsed = now.elapsed();
    assert_eq!(result, large, "clean large input should pass through");
    assert!(
        elapsed < Duration::from_secs(2),
        "large clean input took {:?}, should be < 2s",
        elapsed
    );
}

#[test]
fn test_scanner_detects_ghp_token() {
    let scanner = SecretScanner::new().unwrap();
    let content = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBWM3JJTGdWWmdtcWcrOTdaMFJua2FJVkZFSDExaEFDV2J1UG9mVWsvZms0ClZKa2xCRy8vR2dvdGk0SlJGdzR6Y2FpMHlQT1B6Z05SNlBwczRvWUVCb3MKLT4gWDI1NTE5IGIwalJLcWRadkVZOVlYMSt0OVI4SHQvQ0ZaQzVkdzRwd3FUR0FMRVhlbXcKMjdSbXpGQ284UjFFNEZrZVhveUtQV012dlp0cm43SGpaV3d3NjI3TmZkbwotPiBYMjU1MTkgZ2cwRnRYNW0rSDd3WXh2MUhXMWlFUDVETVFKYmt4R0NuN213UWp5Q3l6Ywo5aFBRMDE2eEtEMkZoNk53aWhQM3laRnd3S3FDWUF2cjlDYll0QW43V2xRCi0+IFgyNTUxOSB4UFBtbXdEb0N3cElMNkVwSm5GbXNqU1RuNURtNXRBL1JvWFBKTWxIOFUwCmswL0hNQUdPNThPYXlGMDVSNHpJaDlNMGN0U294L0QxdUhnK09iNXh5T0EKLT4gWDI1NTE5IEhuOThlVTEzcEdvZzlwam54ZHFwQkd1RkFMeXBUTUF4OHFod0YyblhrVkEKZ0FOOEM4b2dGdDJ5aHRwNGw5bDBZWGRWVWhYNnRENlA2a0JxZFhnYjFZWQotPiBYMjU1MTkgdVlmeUdxUmUyTDVQNkdCTjhOamtFNGY3ZjltRCt0WkFxbFdhaWI0ejBScwpRcFMySU9oOWpVUTdDQWFrRzVLNFdvMm5sWm5ESjR5SC9ycExjSHhTNXpRCi0+ICh5bUFpXGwtZ3JlYXNlIFx3c2pFCi9sbnRZbitkempZNGR0Wmt5SFZhU0Q0Ci0tLSBHTlcyd29SZEZBZ2EzOUpBVVMzSDBSYWFSeGdna2d0bzg3azlnZWVBdVdnCiiziTl2bCovQTO4beiRETZgFYs5Sw9TnDm45oD8GipZ88DhwtBkoBT7bG7TrtH2Q5zhWKqBFg/MWxOFE8S8IBuoYJ4AZogofw==]";
    let result = scanner.scan_and_replace(content, |name, _| name.to_string());
    assert!(
        result.contains("GitHub Token"),
        "GitHub token should be detected, got: {}",
        result
    );
}

#[test]
fn test_scanner_detects_stripe_key() {
    let scanner = SecretScanner::new().unwrap();
    let content = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBBU3FEMHViR3FXcFVqcFdNZ1U2d2k0cEtJVndGekRPZXEyVlo5Ym1xRzJNClhINk5pNko1NjdMVE5rYXBzVVNiQzV0UldpNDJuN2RyL0V1WmJxdmYyaFkKLT4gWDI1NTE5IGtreGthWVF5Rmp2NmpSMEpxRVlWbEFIakdvVVBDOEd4aGV4RHZuMnhhMTgKS3RJcmc0NFhaWCtmWSsvdmFHZEdBVUdwOXhXK293cENzb3JYL0RWOStCbwotPiBYMjU1MTkgNVpkY1NNcVBndHREeHNwekRZY21Yb2RwRUk1UzV1QkVXazFPblI3VFJYNAovT3pZQ1BjeXhCNStWTU5IdFl0MmRZQlNEeEdUL01kL1hNazZlZ1VSaUVvCi0+IFgyNTUxOSBmNkhEYlJVYjZpVmtoWGVjd2ZLMDBia2JTRmVmUENwbTRpOHVmc2UyazNvCk1lMGhsVlZTSGFsUkVTMGJnK2gxN2syWU81Z1NtbTAranFiVWJIWFM3OUkKLT4gWDI1NTE5IC9WeUJUNm1yN1p0SVRCak5NRGJhaGsyeFJFcWxLVUc0RStSOXBTSnpqMm8KckYrVHoyV1NXb29HRWpqWGxnMEZJSFdsdVg4TWluUkdaU0lEQkd4Z2pTRQotPiBYMjU1MTkgbTBrTHhWNWREejcxTmxiZ2VvS2dTTFRCam0zZFFBNk5xU3k3azlBOXFETQpOaUJmUVAyU3RlT0NVTWU4dzNTekJNekxGWm14SEUzd05zK0lqbXp1MWxVCi0+IGs6LWdyZWFzZSB4RjcgYG1bXnJWK1cKVnpMaUlJWDNRQTY0YVpPbE5SQ0Jna0NKYVNMRmNPUnJqampZN1hwUWtpOVBPL05RUVorZndTeDBhdmdtZUpnNQp3c3Jma3FWQi9QRGdyZjNhMjhPS2I0UUlydFpKSUJ6cHFFMXZHM00KLS0tIDRQanlEYWd2RkJYQUpyNFQrTlhaeU1Fc2ZGZ09YTWtUa2p4THk5TGRuM3MKqMwK3JDdI6lTWfbWbXz5PDjMImj19hdu0LN0ZIubV4P8y0wA9vJKvfxMimzNP5KrLeGTiEt0nCBQMyAUi9eu+bNVDxgPlKhMKDU4JsgckkaQLN8=]";
    let result = scanner.scan_and_replace(content, |name, _| name.to_string());
    assert!(
        result.contains("Stripe"),
        "Stripe key should be detected, got: {}",
        result
    );
}

#[test]
fn test_scanner_detects_aws_access_key() {
    let scanner = SecretScanner::new().unwrap();
    let content = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA5Z1JsWmlVM3htOTloZTBnRWJxY3p6bjAzMnNYRE9DMWhUaVhYdkNBWXhBClZEMWRYT0JtMU1TSU84dVc1Rk1iRHZvQkkrTzVzNGxMd2JWWWx6c2VwblEKLT4gWDI1NTE5IDJEeEZlQ09FUVBPNXNYZmxiMWpUT0N6VzRqd25lVXF1UzFJd3ovTXltaWsKYk1CZy9HUS8yK0xSY0YyQzNjNlkyR0JsYURleWJyMzNNN0lMbXVZcHExZwotPiBYMjU1MTkgZmdJdW1MYUovODgxU096aGhTa001KzV0c213aXFoQkwxSXlBekVqUnNRSQovL01sM3c4M294WktLRGdVSGY4MkZKR1U1U0ZvbjZFUU1CVXBmZ2FBa29ZCi0+IFgyNTUxOSBMZHhyTGtGMUdxSThNOHNsMkViUzFheWNvcDlRTTg0eDBTNUF2ZWRoRjFnClBYTjNiOEU5QjdnZXUxdURDZVBxWTBiSUVDRFRXQ3pyRlEyYjBKUkZRYUEKLT4gWDI1NTE5IHFVeGk5S2o1T3V0bHp3VllWNzhPa2pPUkdRSUd5UC9sOStXOU5uRGFGbWcKbFJvQW5aQi92eUNQV01MU0psTFNrTlhVaTMvNUo0dXkxYXgvZ3duMHE3cwotPiBYMjU1MTkgU0lnMUFPRE1iZ0xxS2UxN3hiNWJzWURBWmtxR1FGby9VWVgzaVNxaUxWMApsN3I0YmE2ckVJWnVqSDdRbzFhV2JvRUlRTFZjN1JsMGZaTm5JRUtvZ1RNCi0+IDUtZ3JlYXNlIFM8THU+PX5bID4tfU5jUSIgcmNYCkI3dEpLTncvWUdWcUV0UWdwWkpkbkxReVFzUFhFSGNlCi0tLSB0OENjVHY2Y1VtMlZCVFlnVzhLQTBMaE9LQUN5WFR3N042SWFwNnNEZ2hRCoKo+aZ5ZEU+sa923Y3x7MFJKPR6EcqM3AhQyflDk50egesk01IwVZhgkfRTvW8ojv8VAoo=]";
    let result = scanner.scan_and_replace(content, |name, _| name.to_string());
    assert!(
        result.contains("AWS Access Key ID"),
        "AWS access key should be detected, got: {}",
        result
    );
}
