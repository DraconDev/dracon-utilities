//! Tests for the plaintext-sibling escape hatch.
//!
//! A file with a `<path>.plaintext` sibling is treated as intentionally
//! plaintext: the clean filter returns it unchanged, and the smudge filter
//! never sees it. See `docs/design/warden-plaintext-sibling.md`.

use dracon_security::modules::filter::is_hatched;
use dracon_security::DemonSecurity;
use std::fs;
use tempfile::TempDir;

#[test]
fn is_hatched_returns_false_for_empty_path() {
    assert!(!is_hatched(""));
}

#[test]
fn is_hatched_returns_false_when_sibling_missing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config").join("secrets.env");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "secret=hunter2\n").unwrap();
    let rel = path.to_str().unwrap();
    assert!(!is_hatched(rel));
}

#[test]
fn is_hatched_returns_true_when_sibling_exists() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("secrets.env");
    let sibling = dir.path().join("secrets.env.plaintext");
    fs::write(&path, "secret=hunter2\n").unwrap();
    fs::write(&sibling, "").unwrap();
    assert!(is_hatched(path.to_str().unwrap()));
}

#[test]
fn clean_skips_encryption_when_plaintext_sibling_exists() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example.env");
    let sibling = dir.path().join("example.env.plaintext");
    let [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA0VFNmZWhNdWRjbmp0eDdSQThBM3d2MWUwSVk3cWx4cWR1T2JpZ3ROMzM4CkdhcDZKVFdJUm9NRFdMQzk5OEhGS2RRQXAzMDRGY1FURXZYM1pPZEVNT2sKLT4gWDI1NTE5IEVzY3VuRHBkR0x6a3VCQWRCZXVyUmVPVER1b3hvK0wzSkhodGdKQU95bmsKbXNDNFpRZUM2Q2hBalpLa1hMT3lXVWJBV0FoVmFoYmtwam5LdU5QUnZRSQotPiBYMjU1MTkgV2xtZTMwSGkzQ3ZzaXZHdFJyWVdaTmtZSnl6dGRrQlhodVBhQUtjZ0ExQQp1Z3JpQlNsUERiUUxMdHJNVEo3S2NRMlZLem9YV3h0bTRoTHBtVFVISHF3Ci0+IFgyNTUxOSA3MlFUSDE1NWQ4OHNZQkoyNk1ROGpTdHRKTDhkeXdDZ3FIL0xtd05kb1JFCnRidTN5VGF6V1ZTbnZmVjVPQkxTaXd3NzROSkFwQ1FtK3ErY3R6Vk5xa2MKLT4gWDI1NTE5IEZDeWZOTGNyM0ptUmdpOUZCZ3Y3V2hZL1k4UFQ0U29MdTJ0T0ZGTDh2MmMKZVE2SnorcTRMZVFOczlyOHIvWkIxWUhIWm9GUk93MmdCTUVmZ0lPQ00vTQotPiBYMjU1MTkgbUloekVib0FuVHo1dGo1ZFBiTGQzUjg0d0Z4Z2xsc2pLZGNjemVKMzNXdwpNakpSc3VkQjJlR3FtYXpmeUVicFNXd0QzVDNCejdNOXhnczhZOWtpOUxZCi0+IFJ5K2wtZ3JlYXNlIENqW2QgcGBccm52Ck80VlJQSWhvOUtwK3JLVU15ZkNzTFNWcQotLS0gY1BhMHBvVWRNYWpmUzh0Yks3cEtlRUNaalBMNmVXT3dyN1I0TUhqZkhFNAo1Mrab4eXgkszoJuJ9lIlpf8PxVwnX63ez8eI4xBsHmQu0ro86HQWS65Njygao9xUUfF/EowfZEsXMrKdRs+ODtHj+WIQkRPVDKaBODiRx1dZXqzcLMgSd0NHW];

    fs::write(&path, secret).unwrap();
    fs::write(&sibling, "").unwrap();

    let security = DemonSecurity::new(None).unwrap();
    let cleaned = security
        .smart_clean_with_path(secret.as_bytes(), path.to_str().unwrap())
        .expect("clean should succeed");

    // The hatch must cause the content to be returned VERBATIM.
    assert_eq!(cleaned, secret.as_bytes());
}

#[test]
fn clean_encrypts_normally_without_plaintext_sibling() {
    // Negative test: when the sibling does not exist, the filter still
    // encrypts the file (this proves the hatch is a real opt-in, not a no-op).
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("secrets.env");
    let [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBOTnVzcFdYc01LM3VvUFdsWEJHUllidzVxYnNLR244dEtoVUsxWCt2NmdvCnVncnhkTk1jOHpjVVd5QzRZWmdOMFh6YzlEVktOQnNjYzQ5NmhvRFppNTgKLT4gWDI1NTE5IGtrd0dydWorSE85TVk0ZU9aR1A2YjlSbzdyM1FoNGs0Qnlob1FyQ1hvd0EKeWlmY0x5R3BhVHlhOWxDUnVvanViYktGVHBtbzUzQzVXVk9lRk5RMWFzOAotPiBYMjU1MTkgbnFMVzMxRXVuTktKZzJkZktYTkhBRExWV0lFaXovU1c3a25MUzJheTgwcwpBSzloaFY2R2VjcmFBa01xSmRHZnQvYmxYUVJHbGx0ZGx4bHRtK2I3cVBjCi0+IFgyNTUxOSB2dEd6RVcveDBQTi9LN1lnR0wvRFFUcktYQmoxNjBOZ2ZUOEduYncvYUZnCjJ4WTVhYy9YbWVTRVNuY1Q5QnRlbStXbmVSSlFGMW9jQ3laZ29WRElLRHcKLT4gWDI1NTE5IGt1Y2ZNeVh1SFlTZnpZdnZwc0FvUWxVbmZvSVVWMzJ5NUVkTm9zeUlRUXcKV1RjUUdXa2xGYjVTWUtVYXBnVlRYM0RlK0swcStuYktMYmpWRkI3OTFIWQotPiBYMjU1MTkgdld4NFUrN2huNk0wOFUvcmttb0c1Zy8xNmFwNG5wN2Q4UXV4clRPWnV4MApiaXhrYVhTeDkxSEZMdkp3Yk9qV0Z2d3kwSjhZUUhZMUM1MGlDMHptS2lJCi0+IH0tZ3JlYXNlIGJnWDMnQmpcIFEKL09DbUZwOEZUVHNQRXZLNHhLTEJ2aGhVMVlLUnVFTnpHY2FHaXJJQzZvMGxBUGZieFhZCi0tLSBBOHh1UzRJcDY4WTJRZ0JkblZCcTBtZmNyRlJXUXlYV3V2MTZTS0w3Tk9NCnCbOLp0w041uscwBlxRA1T08PAy3v4vG/20HTZqOmWxy1UUmClZg6uHjreHUZq1vZEry/1KYsDqFYnzBJ0DFUBY];

    fs::write(&path, secret).unwrap();
    // No `.plaintext` sibling

    let security = DemonSecurity::new(None).unwrap();
    let cleaned = security
        .smart_clean_with_path(secret.as_bytes(), path.to_str().unwrap())
        .expect("clean should succeed");

    // Without the hatch, the secret content must NOT appear verbatim.
    let cleaned_str = String::from_utf8_lossy(&cleaned);
    assert!(
        !cleaned_str.contains("[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBmNW91QUNTMkFTUzVSWWZMNnVySzFVNG8yT3RmdE01NFRuQjhGK3d5dFJnCkVraEhPaDFHZGptLzVxUXl4aTc5d3BDT25Rd2pZdEozcHR3S2I2WUtPaWMKLT4gWDI1NTE5IFpYMnpJcXRTNWNrZDVYUm1PSHJGakgvZ05xb3pqUUJoOWJOQXFCSUJJUUEKbkQ1QnpvYUpZNlByRTQ2U0Z5Smt3T25RUFlmUW5wWnFsQ3Y2ZTdJd1M1MAotPiBYMjU1MTkgVCtXc2VsT1pRQ05ZeHRzSGd3MHc5M3oyb0FlK0pvRlhSYUI3ckthemtXdwpRTEROelBLMVB0aEl3Z3RnTFBBOFZ4dzJGM3c0S2NDUVhrWWRKRWx0VmZFCi0+IFgyNTUxOSBIaGVnM09jR09XMDkzSHZVaHd1RnVuNm0xSm9oaTUxMTJhVHUycTdDdWxVCmlTdGFQMm5OTkYveU5Na0lCV2Y1SlJzWS9UOHVhc0FocVN2T0x4QTRwcVUKLT4gWDI1NTE5IGphNEd3MlRlR0Y5TEFXTGlNOEtvd2hFQTZ5UWZCMVZGRnJ1bU44TmNqVzgKcmcxZE9ZWU4rdTlMdUVzV3FsTkJuWlk3aWl2QzYzdy9GMlBJS05TMjlyVQotPiBYMjU1MTkgdENNSE9hRTEyR2FpT0ZXN1JUb3pyckMzelRzMURvYkFKRldLZjJRMkNHVQpYS0M5Y1NTUjFSb1NSZjF4UExHdVA5NTA4VEVtMzZFRE9VNG51eStuM09BCi0+IHBOI0xTOS1ncmVhc2UgSDx9TWsgT3o1Q2VUICglJSBtXQp2ZUJhYWJCS1grZXUwbDM4QklRYlFFRmwwdjJUVEY4bkxmeUhnUEVITEd0eWNxYS9zbHBYZFZ4QXBrVThnRWEzCitnV1ZwK0huYWZRSDBZK0wKLS0tIDI5emJLT3NyWUxYckVJbVE3MjdEeWtTempLSi9YMHIxc2hlb2pZbFhlU00K1G7xr3y0+u8VLuHGYTqaqmoeAx+vav5GcMm2ZhxN2EcCUZjoIMVJPP7G5R3mJswmK1roEA==]"),
        "secret leaked through clean filter without hatch: {}",
        cleaned_str
    );
}

#[test]
fn clean_with_plaintext_sibling_does_not_add_env_version_header() {
    // Even for .env files (which normally get a Dracon Warden version
    // header), the hatch must cause pass-through with NO modification.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".env");
    let sibling = dir.path().join(".env.plaintext");
    let [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBaVi9oNDlMcGdvZFFxamNldjRBR1NhaW1xRVpCNHdCbGdXZUVFWHRIK0NRClVQRTZOUWlvTnBVdlpUeHUxZkpXcG5OTXcvMUJkeTdTL0xhQ2FpMyt4ZnMKLT4gWDI1NTE5IDBTdkZibzY4RW8xNjdUUi8xRlhUSmh0MnJnVHJFcmFFb1V1WVhURGMzaEUKWVVtRG54UDQwK08ydG9CUDMrdTNtV0w4cVU0ODRPYkhpOGI2ZnkwQ3FPYwotPiBYMjU1MTkgZlk2YytUN2d3azg5OTBJeWZHTUVTVFpyc24xTjR5d2J0aHN3L21LeXlIbwpRUUltdWg0UFFvaHpybG4xWHRKSCtJTUxreitTS3BFb1l3UExQTzlpaVAwCi0+IFgyNTUxOSA3RnBUTTBqZ29CUjAvVThaL0FSdkhEWloxT1cyRFNtd0p5eTJBVmVxK0JzCml5cmdWTkhtWjdoaWI3YzN5M3NONUM3SVJEM1BHNU1rUTdISzhUVEdEY1UKLT4gWDI1NTE5IGoyQmhxOUFNMGVTLzUvUjF2a2xXZ1ZyZDJRRzI4alY2a3RsS3U5TzRVbEUKampLUlNKU3FpZEpOOGkrcENHSmZmSUhVTmtQeVBqc3ZGTzZEdnhzTGJQMAotPiBYMjU1MTkgNVYxcGJoeXJ6ZG1mYUpYVzBSbWhMQlhNek16Z05CMEc0bUF5bmllK0hHawp5T210aE1GRWdXc1JCdjJpQmJqa2ErUk5kWlhXMUxqaUNxZEk1SndrSVFFCi0+IHtGLWdyZWFzZQppWTZTaEh1a0lUbGlBUzc4aWs4Uk1oc2ZKOFlJM3I3Y2lQRS9IbXhZTEpzY0o4NXk4OVZKOHJQSjRxSi80NklICnhablZhTEJXbDY2eWlzRzl2RjA5clh6ZUhVUGtGK0JPcjNlMGpRCi0tLSBsRjV2M01tSlZJdVg3cHpiWXlYekNSWXdkbkNzcDZQcTFYelhZQXBNL2x3Cio4j7ZG4c/rwBKA6XTfxhMAwG7tMusedqUORk/+ba3PtCzPZSF9HbFS46yOy5QL7ZWIrLUSQcT3lEYiGWE=];

    fs::write(&path, secret).unwrap();
    fs::write(&sibling, "").unwrap();

    let security = DemonSecurity::new(None).unwrap();
    let cleaned = security
        .smart_clean_with_path(secret.as_bytes(), path.to_str().unwrap())
        .expect("clean should succeed");

    assert_eq!(cleaned, secret.as_bytes());
    let s = String::from_utf8_lossy(&cleaned);
    assert!(
        !s.contains("Dracon Warden"),
        "hatch should suppress version header"
    );
}

#[test]
fn clean_with_plaintext_sibling_preserves_binary_content() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("blob.bin");
    let sibling = dir.path().join("blob.bin.plaintext");
    // Real binary content (PNG header)
    let bytes: Vec<u8> = vec![
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d,
    ];

    fs::write(&path, &bytes).unwrap();
    fs::write(&sibling, "").unwrap();

    let security = DemonSecurity::new(None).unwrap();
    let cleaned = security
        .smart_clean_with_path(&bytes, path.to_str().unwrap())
        .expect("clean should succeed");

    assert_eq!(cleaned, bytes);
}

#[test]
fn clean_with_empty_sibling_path_is_a_noop() {
    // Defensive: empty path_str must NOT match `<empty>.plaintext` at CWD
    // and accidentally skip encryption. The hatch is opt-in: empty path = no
    // hatch decision.
    let security = DemonSecurity::new(None).unwrap();
    let [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBTVjV6S1FjWnRkTGVxTHp4aUluNVhxMnRlWklxdC9SekpHekx6bkhLWFZzCnhLUWQyZDkzbUlCcWxjQjNzZXphZ0pUYWFiVndxNTdCVmpsWURnMWVaNFEKLT4gWDI1NTE5IEtadU9YckJnSitmZEZWb2R5YkRhcWpPVWpqclR2c0h0dFZMMmpRTktya0UKcFZTMWl5SEFWRStlTnZ0ckw2dXRER0EwYThzSEZXQ0lCVS9zN0JhRjh3bwotPiBYMjU1MTkgdlJmaXhGc0xWbDBNNVlKVVRaZUw2T3VFaU1iZ0xLbWFXUVZ6MWtVL2V4ZwpzV3dKRkZaT0tMWk14UW1Nb25FaWZaOWordlVDUEg1NE5yU05lYURhMFBNCi0+IFgyNTUxOSB0TFc5UUFMYXJFdENDK2twRHU4NExUbmFWTCt1RE15Nkx3TkV1TFlDampNCkFNV3Jkb3VDRDM4R0lISXRwM2YvNU5ZdDJJTHYyWHNyWFphSXlOdUpva0UKLT4gWDI1NTE5IFI0Q3k3WjlMTmVnOElMcUNucmwydWxxT21zb2t2KzV4TTJ3eUVXam1HbVkKYnNWZXdFRC83ZXIvd08vSVdXYWRJMGlxWjFJY28rVjVlRW1qR3gyYWpaWQotPiBYMjU1MTkgNjFVcWRrV3R4THVDdGpVU05EWlEvcVhKWkJkNHV2QVlhVzMwK2lBRFJVMAovRmFha3JDSTMxS0FLcWF6aXVzWlZ1eWl4NDJkZWh4TTVOWjdjV3FjQjdvCi0+IG8qXGQ/LWdyZWFzZSA7eGl2bSB6fiJMUyBsajA5bCAxClAwcUJrOHZoS1EKLS0tIHAxTHdPaWl5RnlnNmJITXprZlVMS2laSGVISzVuTkdYSXpLL2R0ODdJdmcK1QDRmRJzvGQw3cOu8NquuIw9NMfG07KZM49FhDg1/HWbXOvJO0rXDXa66jWBb+oq1J88W9MZwFQE6bAHY8/YEHc=];
    let cleaned = security
        .smart_clean_with_path(secret.as_bytes(), "")
        .expect("clean should succeed");
    // The cleaned output should NOT contain the plaintext secret.
    let s = String::from_utf8_lossy(&cleaned);
    assert!(
        !s.contains("[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBCcTlkZzcvVFIzdjZteE8xZmorYXg1V0grTmcvUVdiSnAwM25qUm1neFNVCks0MVNLU25PTGUybXRvbm1xc3dVUGpWNkZ1NTc0aVFhZnZzLytiMXlYV1UKLT4gWDI1NTE5IG9kbi9NcnR4OWowR3V2YjArc3FWRElSdXRmSWR4djQrcGsvQzlUVFVqMncKMmpqSk9HZUxaclZqMGp0cHFFL2xqaTVFY1JJUTgxUk9EZDdCYWtrbHNTSQotPiBYMjU1MTkgSlFvYmFzQnBPNG04aGdhSlVRMnhRMU5RVVp5Rm5ibVlnZW9pTG5hemNoVQpEZ1RkckJxMWM4dWdHM085ejdtbTByajliWUd5ZEVwVDFQaDQvSXdGYm9RCi0+IFgyNTUxOSBFcC9Kc2RtQytRRVNzSnl3NDRXZzlaQVF6aStDaHJ0bTlEMWtNbmFDekFNCm9sTlQ2dkxIWk5KbFdzbmdGU2FTM0VjK2lIVHh1NXlYYUduSW9qMFM2TmMKLT4gWDI1NTE5IHZPaXM5NGJ6M1VrUnBkSUdVYnNvS0dGTEtYendhUUJHVE0zWUdTZ25pQjgKS2hnamowL2R6RDRUUXZ1ME1wTDlNVzZ6b0huM3BRYzNOT2E1dnhTVElUUQotPiBYMjU1MTkgY1Z0cWxQOFVqai9DL2ZFMk1Fdm9QaVVsNjU2eEZRRElidHFkRkxtSTFUZwo0YTRtZFNCQ0o2d2N1bi9SY0t5L3h0OW9NTVhNZFRJcUFRUnBVRXR6cDBFCi0+IGctZ3JlYXNlIEd6N14mRCB8QHgpVz8jWSBRIENecEBEewp5Y01jOUMyN1lVQXQvSXQyK0U3bWRJZVpYS2M0NU0yV2RPUFJ2Q1VGYnhTa1RVRG4KLS0tIHo5VkdRYlNLbDJoUjNldWU3MWI3K1dLUlFiVTE2eWpFa0JYK0crNjZOL3cKb6CXSrZKihsDY9RCuD8wNjoVWzEmaqvs7n5BkD2f7BVKnxSFBWPv7cO3sTtpqjhuRsfmiw==]"),
        "empty path leaked plaintext: {}",
        s
    );
}
