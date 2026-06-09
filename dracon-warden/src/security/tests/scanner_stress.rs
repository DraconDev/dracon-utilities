use dracon_security::SecretScanner;

#[test]
fn test_scanner_stress_1000() {
    let scanner = SecretScanner::new().unwrap();
    let clean_content = "fn main() { println!(\"Hello, world!\"); }";
    let secret_content = "let [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBtTFhKait5VWVvSGhEZExPUzZ0N2dFcDNSSGZzdmNyOEpjL0ovQXA0SjJNClMrM2NnVUtTM1dUbmJxaENLUWoxYTVOVjR3VmhBNTQrR0t6UjludGQyY3MKLT4gWDI1NTE5IDc2c3N4azBhTEtWb09LZ0gzYVJMc1paV0xtMms0SDJ0YkozRG1UNCtWMzQKZ3NoRVdTUGN1bk4xbzJLNndRbVVKOU1sRjFHeFoxZEk5OXV1TERJbFVObwotPiBYMjU1MTkgTXBGZUNHV3BWS0dKVjQ4VWVlTnpUbWpOcDZ4MllFTm5WZWlRNjkrbGVEZwpmdmZxMTFPaXNJS2R1dVU3ZjhLQUozQWFTZnQ0dkt1UXRMRjMvTTNJcldRCi0+IFgyNTUxOSBrRDg3bzUzcUEyRE1yMnQ2cG9Idi9KVnQ0Z1dQb2dQMUNzSEFES2Z4SXpVCjR5Q2oxMHRxWGFnNmNhQ1RCU084TzBmcGRmR3VlcmFSZW5yRHBJOU0wZlEKLT4gWDI1NTE5IHVKVWJ3U0RhQzJXWlBUVFNDZFhpTitBeTVmNVBrK3NENWFYbFJ3VGl1V2sKYzRkeENBRFlXNnc0K1JwTkZ0THVjcWR1VjVlaG9DVWppL0x0YVlZV1FlbwotPiBYMjU1MTkgMjBsbnNWVFF2ZU1ISHYvaVZaWGU4cHRCRVpKY2pzdkJneEFtWi85QkpXRQpBaVpHaFRKY1d6a2hKUnA1VEZ6ZUlZZ0dTYzFVaTU5bStSUVRsazU1SVNzCi0+IHV6LWdyZWFzZSBEMTxEY34KdjMwCi0tLSBmaFNmWU9OUHFZNVhsZy82QnlJRWF6V2pkbVhwRFNyRVloaU5pRVpaNGNZCvs1+n1aIQZOKXdtHBnXHiaGuNpvqLfhI0AKZXcETI3FzORZVSezeWG22qxezPTzrNFhzJR/2cJXqlu0ycuMyXSzdhtIVyuGbf94N+Kuhdw9E93cp4OQeflg];";

    for i in 0..1000 {
        let findings = scanner.scan(clean_content);
        assert!(
            findings.is_empty(),
            "Clean content should have no findings at iteration {}",
            i
        );

        let findings = scanner.scan(secret_content);
        assert!(
            !findings.is_empty(),
            "Secret content should have findings at iteration {}",
            i
        );
    }
}
