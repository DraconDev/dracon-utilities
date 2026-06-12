use dracon_security::SecretScanner;

#[test]
fn test_scanner_stress_1000() {
    let scanner = SecretScanner::new().unwrap();
    let clean_content = "fn main() { println!(\"Hello, world!\"); }";
    let secret_content = "let [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBSVUthVEhkOGlFaEVZM0gySHhIWEVhOG5FSDJ0U1phMGw0VlUvU3NHMWhvCllnZlJ3VmhsTXFxSGJBMmI1K0RBTVFISXhPQUNCaGNuKy81eVZwWmh3eG8KLT4gWDI1NTE5IFZsR3RqSUtia0lOZnFiNjJteEJBMTg3aFcyM2tPVm1FL0c5eS9yNUJ4UjQKd0lzL1VIWFB5Qm80OU9iMjI2QTg1REVpYVN3NTZISGsxbG90UVVKTFBTMAotPiBYMjU1MTkgTngxYTZnRHo2VVI2LzlFVENXUVFDcVB5eDdzbjZEWk0wWWQ4Y1ZTVzl4bwpPcmEvdDNKcWtLZWd3N2F3dnNEUzhyYlBabEJqN2dzZ3hrb3RLTitWeVpVCi0+IFgyNTUxOSBwUWhyTi9pVmNKZjV2WG82eVV5SFNRUzZkYjU2ZklTcGx4c05jZVNWV0JFCktUUTB2anhkYWo1dkplejBvWklzSmpVZzZQSEJJeGhlMUhxVC9WSEo4WlUKLT4gWDI1NTE5IE1SNkFENFlVazVSeHg2bnA4UDBSeUd1d1ZHdEFid0xEdmswb3FMVm9mRjAKLzdydjRkSlBsZnRTQVhDYndpVzB4Vm9lUis1M040Q0NhN0JBMWEwbXc5NAotPiBYMjU1MTkgZUZiNkM0MnFpVnU1dHo5MnNJalB2Smc0RFdBL05mVGZsdjZlVENjLzZCYwpLb3pSbWNCL2NpTWZTa0pNdlFka1BLeGdBZDFJeGdiVE1rTkMyU1lyRU5zCi0+IDFpfjglLWdyZWFzZSBCe0siU3sKTER5S3R6SWEySmRML005TnZpR1h1T3pvMk9kUk5ua2R1ejRpWFN4NVZQL3kvVTUzYUt3V0taNEVyT3pVSFF3dwpxeDQ4MXQxbnFNS1FTUQotLS0gSkEwOUw5UksyWEREZzhWZnZGWm02YTV1VGtGbDFzM1lNR0tEK0hCaVpnYwpkhL7lStg+zX8WE5SgMe/nbRPjinTwn+iufONRLQskOPweeTRyeqFBk1Y9zQOHVxVfxJF6HDrYrctq9T6+mWaqgUDnPTVeWSrr14dDni2wHJXV+wCX61e1bg==];";

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
