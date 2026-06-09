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
    let unicode = "日本語と한국어_[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSArc00xZlBaaE95R3NBYmJ5MVlBMXdLemdTNVNhVkd3Yy81d2l6elJMTW1rCm9KVEZETkhaQ3lSNlJ3RkMvdkVTRjYvOGlzNy9JNHowanZyUFdVSENFMDQKLT4gWDI1NTE5IDZJZVljSTZranNkelk3NUlad2s5MHZ1Yy9tcm5sSDFFb0phVjJhblBOV00KeVlManNMSmNLclRXOWJ5UzFSNjVJWFB6ckx1RkgyeUlHTkJoSlpNdjZ0OAotPiBYMjU1MTkgOTA1WFg1bFMySnUrVmw5YVprdUd5R0VZWjZrUmcwRWdtQlVsNSt5U1FFWQpGeGRCNi9oS0t1WlA3eUVGeGIwZjhPQmxFcmhORTBKOFRaMEYxV05nZzU0Ci0+IFgyNTUxOSAycEEwZy9Wei9CajNKMHBqUE0wZlRmNDQ3SXA3ZGNmWXFLOElMajRhZUIwCmlCM29WVnEvaE10Q0k1WDhIWHI4c0Q3SGpqMG1tSDh1WkVGcCt0aDhHQWcKLT4gWDI1NTE5IFd3RjZkc1IzYnVwckN1OWJvQzNLZW9UWEQxYlFock02VWNubkdxNTFZWDQKV2FEenlHdHA4ZjVNS0pLMXBNMXdJL2xXOUxxNUM4d3Y4S2E5cVNuc01PWQotPiBYMjU1MTkgNFpDeTdtYkpNQzV6R2NMd2FuYVlMMlpFTEdVMVo2UHBCeUxxRnlNQ0FUNApPMGx2UHZGVlBHTlpNODBFRnYzaFhuUVFIbDlvTS81aXRYYzZmSkRJUzRNCi0+IG1BLS1ncmVhc2UgZk0KVGY2QUtsWk5URkQyd1pwRzZvV3JFdkYwc0wvZXFMZ1hjcUVEa2F4M1lRCi0tLSBKSnkwcXhmbktrWlRuWjg4S2x3endHeTdtYk1mbzVMdjlFZnpmbkczZ0pNCq+dqmYcfJwrbNBEhEpy01+ZrogGeC5wg7KWidAkoFyOpz5X+pRmeoHNhA/rLnH6iu+kTVMbGI/aE1iET62JT9yxtw==]";
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
    let content = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA2WVVUNDdVRlhVbC9VaGNFeXlPaWI4NzE4UHRkZkdyZVZ1SlcxcnR5Snl3CmhxUEFFc0I0Q01ka242S042SnhSRktqUXp0OUNTSTE1Zzgwa2xtZGtRVEkKLT4gWDI1NTE5IDQzWWpaRmdVTzVUVmI4NlYzcENKdlNpUEZabGVlK3djMi9IZjV0dkxYenMKMjc5Q1ZhQzJGMEdQcTIwcEpMQ2VXMzZFbHJnVVM0QjE1QXdwUHJoOVVUUQotPiBYMjU1MTkgOTNYcnN4czNUOUlEUVV0ZFBqSFNob3d3UGVLSXhMM2tuTzduNmI3Zm5BdwpuNjBKcEZKMnN2cnViTmUrbmp2YWQ1VE5BcmpvQkVNbFlodUY2SUxEYkFJCi0+IFgyNTUxOSBjSVU4L0VPRmVMZjlKRjUyT21udXIyVE9RUWtRRXZ0K2RaSmdaVmRVcVZRCkw3b0o3YThVa2tKZW13cVNvcmdFZEMvNVZkOU1zL3ZJSnRrbmlrYnZPWU0KLT4gWDI1NTE5IHp5ZkFuQkcrRlRkaTgrVS9xRFV5WEF2OEY2RGlnNSswQTdBVzd5NTlZMDQKMzZzSWp4djUyM2ZwbURBTXhtbXBNTkFDcVMrKzVFYTBhT2xCcXNmU1IvUQotPiBYMjU1MTkgR2FJQ0J1YjZkdWxoTUJSUUh2Wjk1NFNwcFJwcWh1alQrR3NkYmJQSmR5cwppWWJ6RUhWVTZaMXB2a1BZRUVmbUg3S1d0UERPUDd2U3JFRitNZmxDNExvCi0+IGFqLWdyZWFzZSBtLEdOO0MgaH1ZK0xvTCByTE4KSmYwNXpRNzhWdUwrUGg4VTNFWmwKLS0tIGh1VlJPT1ZxZEMvVC82WlI3SEJLbDhuTGpxU2hHcGdxZ1NnKzV5WXA3OU0KX6qB/O/k2RAk7qrmtJ+2+ultjrjT/JJr1KmycXbwlON+rd90o+Xutpof5wD1ojzRrDRpi8I5X3IWEXMWfdLQh8Wzi5tB4PSk]";
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
    let content = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBWSW1JbkRJVWlNQ205ZmV4aER6NEdsTmxMYmRQRmgxTEYzeVdzWW9VbnpRCktjSW42M3JkcTQ1VEZaODRBbUFrNU9IQjZFY1F5SVErTllGZ3loaVJhMjgKLT4gWDI1NTE5IDdpNGdkVzFwTVU4b3Y1MGtVRndIOTZJZ0cwN25DOUJoTGRhTmx4QVNkQWMKOXZhT2crU0hZd3FVY0VoekZCOUlrekFlRXBvK0lJWit5ZlREVGJhazN3awotPiBYMjU1MTkgelJ3d2RhZkN2M240bWVHT2VCZlhDenZOSWkrTWFPV1M4M0QzOGxvdm8xZwpQeTJBL3I1YUNacENJeWoxU3k5eExSVWJwZkV1c3k2cVRuOG44TmFVQUlvCi0+IFgyNTUxOSBscHlRVTgrUmRmc1lhUFV3WjdydDJXL3FXeEdiVndnVGFiSlhyRjB0M2pVClcrTkJuSnVDT0lpNXZLZDY4UjlUcVNhbEw3QldKZVJwNWhZUUpjdWhnUFEKLT4gWDI1NTE5IDkrNERMZkV4U2RQYm9Sblk5ekdkckZIK0FweGFJOE93My9sbURNOTl6WEUKb2FKeUVnL3VtSXBNMUpncHlLUFJNYUVyWnB3eTBwODdwSG9SQ2xVSldyRQotPiBYMjU1MTkgcHFrTnhWYlBsaVhmTGJrSjNKdy8rVGFMY0lpbnVyYXVDYWUrOEg3cGxuOApzTlRUSk1DQUYrTU9jRjlnNGZLVmx2aVRNV1hEbFFtQ09BN0QwQUQyUnFJCi0+IFdsNS8pRHktZ3JlYXNlIGA6RkItQ3ZlIGggOi9GUXVfTApJN043ay9QTXdhN3Q1SVF6dk5jMTVFNVlwNFhhQ01lUTBGM2NEb1JxMzduSlFuTHk0Tkw3elFCc0NlM1BGVEI5CllSdU45b1lSamFMWktOa2tFbjBlWElIU3A3NU4KLS0tIHJiQ25LWW9ZVlhWcEZWZVlVTkxTREZQNjExOE5BU2pBRm5RWWpiYXJPRHMKolOrXa/ZfAEw0VBDlzyyc4REuqyiMnZ5daVkPsuKfqVRWx00QpLFzf26DVXQzZRqDFbUx+A4hYMka4Un0wPSXUmiiFIDGwIJ4BHS1ZDVS8hfPUM=]";
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
    let content = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAzb1pPR2tmSXRlRU1FVDA2NHo4UHRyYS9kMGZwa3c5M2hVRGMvQmFNLzBrCnEvdCtibTZudnU5S1l3ZTZoc05oVFBXQnE5WTRRdU9NbHpOTGs2QnYycVEKLT4gWDI1NTE5IFRYVjh5cENPV2hBd3NMNjhQZ2hlNUxzajE5bzMxNWNycEVFMlFldkJESDQKWjcvL2NXbXdMRWVHejVqVFRGcXRhNWhoWCtqMGhBVVZYbGFjZHo4V0I5YwotPiBYMjU1MTkgQzNobithblE5VUZnaXZBb0ZFSVhkekFTRFVwYUMvanJybTRHNjI5dVNWVQpMQlIwUXgzcWJVbUUvc3lxWUV4R2Mwb0lVaHFvL1d3WU80eGdtOC9Va2NNCi0+IFgyNTUxOSBxZWhwMFgvd2NOejZ4MWhPTXZjTzVlVlNIejdZd3VjK2NFNnZ5VDAzcmtjCkE0YWtJUDV2cTJzYmxHQ3VrNnlVcTkzdU8zcFVDMVR2emJTcUZVLzZudkUKLT4gWDI1NTE5IHJhYkVMSlRXc29tVXZQczhGNnFqcFMxR1d5RjNFZUtGaTNEaDkwT3loekkKN3NtQSs5WTFJVkFkOFc0bzZkMEJpRHNhU0pQbGRLM1ZLSFl2UDJJaTRBNAotPiBYMjU1MTkgcElwd3NkcytZTU1PY0xSeGF1S0hYRXFWZWJZUk1CZjMwTDgzelJWZHpFWQorbEJibmY1S0djTzg3ZWJaSkdLUng1YWs1TGwrdW15eUVsa1VvYVUrWXVVCi0+IHB3LWdyZWFzZQptbW9iMzRzQm8wM1dadG9hbUg1anl6V1NsZWdMbmoxbmZoSit4ZEd0MlRCWWR3MEN0VXhacDhlS0hqaHlnWGxPCkZnV2tjNnl1K1EKLS0tIFBDM2l1MTg1MER4SEMzbDlCd0MvYlR5KzRIUGtiTFZ1cUdnNEFqUDVEUDgKrCIvMgCTjDQnr3fv2BDamhHWXCSOYDGugmwPR7aI8zA9Ch+pIc55rAPyCN0BVPlN3n4xFw==]";
    let result = scanner.scan_and_replace(content, |name, _| name.to_string());
    assert!(
        result.contains("AWS Access Key ID"),
        "AWS access key should be detected, got: {}",
        result
    );
}
