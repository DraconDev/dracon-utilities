use dracon_security::SecretScanner;
use std::time::Duration;

#[test]
fn test_azu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBGNVBiYzNlVnU5L1RiTnJmdGNhT05MLzJCQWljVTVVWlJJU2E3UmtsV1g4CkU4TSswOVZJaUMwMmRKMHYxYVBRempuL0FJYlU2bnA1Zjk1Vlh1b3dHckUKLT4gWDI1NTE5IGlPS2krLy9naTN6VmNZMEJFSm5DSU9tTHVsSHFSSXoyemZGWExsMVNkaDAKdzZXVmJFL2F0Ujg1T3V4ZXNIYWg3cEVYdTAvLzhrSHgyQVVxbVBtYVh1awotPiBYMjU1MTkgQ29uZCtzbUc1YTNHdVdpZmMwdWh6NEtKaEdaZFNYNWVwY21qTVUxb05UZwo4VzhKK0NhSVdhWE9hVEF3WFdjeGZOQlRHYUJnai9PcE1QL1J3WUVIeFB3Ci0+IFgyNTUxOSBRRmlTT0R3dmVlYUJXdm5QNWswcVNzUldYOTgrK3F6bytBOTZWTXNVdDE0Cm9JbjFGSTBTcHcvL2RlWnkzSlFxRUllVkw5OFJxNFZlT0xCQnZSdFprcnMKLT4gWDI1NTE5IHJvc3BLbVQwbGZGVjlyam1hR3h2RllCeVJwWUVMdUNBdERMNEVMTmhmSEUKOERTTTF2QVRSNlRSWkpGbWxEK2tLM1BEVTRjSE5PcGZ5ZUlpRTVrczZoRQotPiBpSSV5Ym9zLWdyZWFzZSBdemtCIFwkNSQKU0VzCi0tLSBQNUxsKzdJdVdXVnFvVDFpQXh4ZyszOVFIRnZZWXZSSGJOVS9uTmRWdCt3CkZ1v2B1b00C8vvtrrdmo1xSBC2eSj9/lImO+bTcEjOtbAwpBSO1bwRKvwCc/SvgfJ8bU55M8Ksp8pXqyzatkcPVPFTRvJmq8t4QvA==]() {
    let scanner = SecretScanner::new().unwrap();
    let input = "sv=2019-02-02&sig=abc123def456".to_string();

    let now = std::time::Instant::now();
    let _result = scanner.scan_and_replace(&input, |_, _| "[REDACTED]".to_string());
    let elapsed = now.elapsed();

    assert!(
        elapsed < Duration::from_secs(1),
        "Azure SAS scan took {:?}, should be < 1s",
        elapsed
    );
}

#[test]
fn test_generic_assignment_pattern_completes_in_reasonable_time() {
    let scanner = SecretScanner::new().unwrap();
    let input = "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBZVHNZK2Z6b0h1NjdwZmpaQnNvTS85dU9VQUlFS0Z3OHpTYjh2aEhPTFhRCmwvalJyeGdGWlhjRlFmcmRUOTR3YnJnQ05wM2JMQ2xrR1ZZRVFEQzZjNUkKLT4gWDI1NTE5IElPbDFqbkhaUFYwdnhsVmdMbjkzbHFnUTJFTlREMGlwT1ArVHNUM0FIVG8KeWpLUkFUWHEyNk9TZkErUUpUMjg1S1NrSk1uWDVKMENHU1RTTE5wOVdmbwotPiBYMjU1MTkgQ2lETlJjaW92cXNNNFBhLy9SYVRmNDVFcmN3YTBmdmhrZ0Mwcm16WmJnSQpYTzRHcFB1YWd5MThuVnFUWlhHSms0WU16c3IxbHc4MG9lUVU3TmJ0SHNNCi0+IFgyNTUxOSBSTlFva0VaYjIwV1JVU1g5SklxYnM4dVN2VXY0L2thT2l4a2lFak1VdlJNClJ3QVdXRll0QlA0LytzYzQ4cVNWeXlBeXQxRFB0ekpIdnBKNStXZ2pORTgKLT4gWDI1NTE5IG5sTlAxYitqR25FMDdHaC96K3ZvQ3V1bEsvMU1EMTRIckxublpmRWMwbXcKS1B2cGRkTFFUc0Y0SFNZdG9TeTUzVHRzaVZwekdFT3hEZmtobUYrcFQ4awotPiBBLWdyZWFzZSB0MCBqRVslIDopbwpDbXlZbkY1cXNhbVUzWHdvUWtTV1Z0a0FheisvVkhMYzZwejZGZ2F5UUJwbGdaRmdQUmJGSTI3L2s4bFAyMTR4CkxlelU0cWZlQWFZeUtjTUNaQzQ0azFzCi0tLSA0V1p0N2JYL3d6d1ZSUmx2YXFneGhmT0Q2d3IrU3YwQ3Y3eHZraE9qYVpvChdHaROW3FFf4eCOLB5TLrzWOT448BopgUeycOQorugBvIWoWLCfVQ6Mw95GFXL7rVg2N8knMQiPx0TFa7n7hv2c1S/ovw==]".to_string();

    let now = std::time::Instant::now();
    let _result = scanner.scan_and_replace(&input, |_, _| "[REDACTED]".to_string());
    let elapsed = now.elapsed();

    assert!(
        elapsed < Duration::from_secs(1),
        "assignment scan took {:?}, should be < 1s",
        elapsed
    );
}

#[test]
fn test_scanner_performance_under_large_evil_input() {
    let scanner = SecretScanner::new().unwrap();
    let evil = "x".repeat(10_000);

    let now = std::time::Instant::now();
    let result = scanner.scan_and_replace(&evil, |_, _| "[REDACTED]".to_string());
    let elapsed = now.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "10k-char evil input took {:?}, should be < 5s",
        elapsed
    );
    assert!(
        result.len() == evil.len(),
        "output should not explode on non-matching evil input"
    );
}

#[test]
fn test_scanner_performance_mixed_secret_and_filler() {
    let scanner = SecretScanner::new().unwrap();
    let [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBkdEs2ZGYxK2lldFB0WENPdGgrM1M5U2VJblcrNnQ5czdSNUFYSHZmNHdvClBTd3ZaTTh5Mno0UkdtTllNZmpPMktpVGpzOUhVK2o4Um5FWDhoUnpBckkKLT4gWDI1NTE5IHdsRTVaVW9EQmwxQVFzdWxNM01Lemt1WVJyNlNhOVBCWnIyTzZFVmtTVk0Kb21IZTNGVmhxTmhVQlpHakdXanJuZ1VCcmZBK0xZVGNTQmJGV3JHcGlCcwotPiBYMjU1MTkgSHg5dkoyVm5CL1RlU1VUNVI0T1RWSUV0dlRSNTFCUE93eUNBR21hY3V5UQppR0JKdnVaNWFjVVlqWmZDQmI0MFMxUzJpNEhoQlBqdWJvRU4yR2oxeG1FCi0+IFgyNTUxOSBjVzVveEM4YXFOVGRmUUNOTGxWWlIrcE5SWHlYTjAwMnlCQ3haS0x1UFJjCjJXQ0QvV3NhZkxmRHVKeG9DWlFYVlVvQXpSdTdTbG8rZlFBVWcrMHVEMDQKLT4gWDI1NTE5IGpvOGljZ3FjSndtZEU3TTcyWW50TnFadWhZdWNrZG5ZVFNrenJEajFhblEKRnlaeFp4RTFjbHI0TE93VHEzT2UrVEJnVnJyQzRwei9GMTZTUEQ3WEJ1dwotPiAoWDlRMS1ncmVhc2Ugb3c1Z2EgMUZdaXFiPSBtTDpvLiBICjNISVhmVnNkREphNEFZU1RVbVFQckVSd0hycTlEYU1XZ0Jpakt6MTV6VmJlTGFLL0c4d284TGxXQ3h2akJscHgKTWdvUWhidEE5bGI1TTQ4RwotLS0gZWF0aW5DaTZ3VG9EOTNEM1RjODBUR2NudWNLNGhmTGowRm1ZVk4zTWRMQQrSFcVXGF71K3navngEgWdkimhNMgwj4xoBcrhtl/+F+GNpuXcH1sYLAQBkdewWSGXtHMyf4RuPWw4u5FNEZh8ddXF62N4GwbcfCL0=];
    let filler = "x".repeat(5_000);
    let input = format!("{}\n{}\n{}", filler, secret, filler);

    let now = std::time::Instant::now();
    let result = scanner.scan_and_replace(&input, |_, _| "[REDACTED]".to_string());
    let elapsed = now.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "mixed secret+filler scan took {:?}, should be < 5s",
        elapsed
    );
    assert!(
        result.contains("[REDACTED]"),
        "secret should still be detected amid filler"
    );
}

#[test]
fn test_nested_quantifier_patterns_do_not_cause_exponential_blowup() {
    let scanner = SecretScanner::new().unwrap();
    let input = "xx".to_string() + &"a".repeat(50);

    let now = std::time::Instant::now();
    let result = scanner.scan_and_replace(&input, |_name, _found| "[REDACTED]".to_string());
    let elapsed = now.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "nested quantifier pattern took {:?}, should be < 2s",
        elapsed
    );
    assert!(result.len() == input.len(), "output should not explode");
}

#[test]
fn test_scanner_detects_known_secret_patterns() {
    let scanner = SecretScanner::new().unwrap();

    let test_cases = vec![
        concat!("gh", "p_1234567890abcdef1234567890abcdef1234"),
        concat!("sk", "_live_51ABCDEF1234567890abcdef1234567890abcdef123"),
        concat!("AK", "IAIOSFODNN7EXAMPLE"),
        concat!("xox", "b-123456789012-1234567890123-AbCdEfGhIjKlMnOpQrStUvWx"),
        concat!("AI", "zaSyD-1234567890abcdef1234567890abcde"),
        concat!("ya", "29.a0AfH6SMC_1234567890abcdef1234567890abcdef1234567890"),
        "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBZVDVMZy9lM3NlNkhaV2JpbDkyUldOdHJETEFXSHZjZ09XUVhLVjRyakJzCmpqOW0veHlSdFFXQTAxWWJHUjBDRmFGckVYMGdDOTNPUTAzK1gxZmFaWkUKLT4gWDI1NTE5IExIZzVLb1krTG9kQ3F3Z09VRHZPMkJGT0VmeG4xajNWYUh2czRmWlROV3MKN09jQWtMcmhkNy9aZVRlL0xRV1VEREkvY0EyUUcvc1FJRnMvVUk4cVhDVQotPiBYMjU1MTkgZ3ZXaUo3R0tOSHQwa0VWMFliampRNGxqSjhUOHJ4akRjWTdkeHJIOFdHRQpOSEd1aElHaFB4N2U5TGp5RzdobDNiS0pCTjFJeHNaRUd0YlZjbDVVbFdBCi0+IFgyNTUxOSB2VEdWZ1VhTVRsWTFoUVk2RU0zU1NOaVZlV1ZxdFVRRFNSRTloa3U1UFc4Cks0cDNnVG14c2Y1ZGV2Ykt4dmJVMUpqOFdZK25Va2ZHVWRud0tzK1JFa0kKLT4gWDI1NTE5IG1zclFnQ1doWDRNOU9NTDgzQVYvUlA1bDdadmNnallYVVJPeFB4NWVZRzgKTEYxcC8wS0YyZzNDRWVuQmZxQ20xemVHcktRSWhqc0ZHTHR2eXc3eDQxSQotPiBuejYtZ3JlYXNlCm9UaG9aQzR3anJSMTEzNjVjVTVsa09VaUx0MHB1RFNDK3Q5Y21kaWpzWTVrKzB0WDFZMlpVRi9NMWd2QzE4VXYKCi0tLSBLc1prRU15aXMzSW9YakZDTW1FN2Mrc1FhM2ljV01KdjB0K1hUaDA3cENZCrrCyA4HFwS1yx6/3m6CQXxzbh4B2pbS5zhQOsxMUe/8rrJaO/wrtOwwLDEZFoVw3LKK1VbyD/qS/Lor4jiFDyIGwVLguterXASERSPsu5oPGLBtriSWulmT45p5j1fvFPpPkQvi6ITBLQqsIl579cHUHg4cIvK8Em1x7A==]",
        concat!("LT", "AI1234567890abcdef1234"),
        concat!("sk", "_test_abcdefghijklmnopqrstuvwx123456"),
        concat!("oc", "id1.user.oc1.test.abcdef1234567890abcdef1234567890abcdef1234"),
    ];

    for secret in test_cases {
        let result =
            scanner.scan_and_replace(secret, |name, found| format!("[{}:{}]", name, found));
        assert!(
            result.contains("["),
            "secret '{}' should be detected, got: {}",
            secret,
            result
        );
    }
}
