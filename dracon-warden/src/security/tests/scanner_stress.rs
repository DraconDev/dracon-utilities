use dracon_security::SecretScanner;

#[test]
fn test_scanner_stress_1000() {
    let scanner = SecretScanner::new().unwrap();
    let clean_content = "fn main() { println!(\"Hello, world!\"); }";
    let secret_content = "let [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBEb1dXUGdDcVlCYUMvc3B2TEFqV3NsU0tXOWc1VUgzZUZIaXc4aDNsVWhvCmdLT0I5SVp6UWJHR1RkOExzU1BQTUdxMUVRMlJsYnd0aGg1Qnd3ZWlidWsKLT4gWDI1NTE5IE9BTkp5N0RxODlpWllwdCtsMDlsU3o1c3R4SllHUTNMVCtTdE9xWkdwMm8KdUdvcjZVaytQR0htV01OUmxnMS9OdnY5ODN2TVhwT3BCdEh1OGwrTnRTawotPiBYMjU1MTkgcXQvMVVnbm9QaVR1TFM0cTFFb2xkNnluWURRR0czMWhSZHVROVlVcVJpUQpJditxT2JETWpDS2lySVRCSDNicDJ4bWJjYVpad2dKWnFFbjhXSm9HT0F3Ci0+IFgyNTUxOSBvbGRoT05DWkFLbklJU2MzZGkzSTZ2aXFXeUtYeFRFd3o3VDQxUUxSZDBNCnRqSTd2VkNIUllBWEZRQjBXWU1GNWVOTXJ5RVRtTVRsZ3doMW9RbGIyWGsKLT4gWDI1NTE5IENZWExMRUJ1MytScDFJWXdmM0MwNFJLVUJKczJZTnp6eUl0Ykw5QVRGaFkKSkNqQXBXeCtuMEdjUXJCOEFVWEM2Qmt2R1JKVjlHUFQ0MlF1WGUzUkVjbwotPiBYMjU1MTkgeFBKWHBva2FkaCtZNXlYSFp2MGNYeDBHdkVDYTZKaHJhcG9SQnNGcGNBbwp6c2xuQmRaUW5IWGVEbXZCUUJKQ1N1a0gvYXR0cG9aazV6bkhGVGg5NW84Ci0+IFVCLmAwYl8tZ3JlYXNlIGpKTStxZV4pIC5YSi93Cit3QzBBRW1vUG1scElqcTRLQ0VOMG9FYUZKNnJQZXVHZEJsZi9VT1RTUDVYazgwN3VGT0E0ZnIraEhsbmIrSDkKajh6eXRxU3JjMGZhR25CajU5LzZKL05pRy9Hd3ZBcFZrc3AxTkVURDV2SQotLS0gTy80NHJjdE91aVM0NWNZTjNzMEFGZm9lVnFIb3l3WlArdm1STTg1dHVjNAqT/3jJxoT4/Ff6YuP5eBE9iC3I/7icG3hyc2jE7RH49r6Ahbf8X3IAk6guLEtSv9v42qGOFusXVfG2f3BbXn2fwnVJ8yLbFYgUatGSJ5/5BFD5LhDFmWYU9g==];";

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
