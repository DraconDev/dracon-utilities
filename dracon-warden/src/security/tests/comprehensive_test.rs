mod common;

use anyhow::Result;
use dracon_security::DemonSecurity;
use proptest::prelude::*;
use std::fs;
use std::sync::OnceLock;

use common::HomeGuard;

#[allow(dead_code)]
static TEST_SECURITY: OnceLock<DemonSecurity> = OnceLock::new();

#[allow(dead_code)]
fn get_test_security() -> &'static DemonSecurity {
    TEST_SECURITY.get_or_init(|| {
        let mut security = DemonSecurity::new(None).expect("Failed to init security");
        if !security.has_master_identity() {
            let key = age::x25519::Identity::generate();
            security.add_memory_identity(key);
        }
        security
    })
}

#[derive(Debug, Clone)]
struct SecretExample {
    name: &'static str,
    #[allow(dead_code)]
    pattern: &'static str,
    raw: &'static str,
}

const CORPUS: &[SecretExample] = &[
    SecretExample {
        name: "Stripe Secret Key",
        pattern: "Stripe Live Secret Key",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB2dmxyeEhTVnV3TTkvSlNPbU51YVpKS29qVXQ4RjYxWG55MVg0RmtOeDF3Ck9hUzNuZVdBLzJacVJHY1pNZGRoVVZtUXR1R3lzZVdqeVRoSm84VWxGN0EKLT4gWDI1NTE5IFk0STNmUWplNjRJZXlDNDVKL0YrZkk1TTlNZWtNb1I5VldqSncvR3NVZ0kKY0t4b204V0gxT0ZpdmxOb08rN3BGRlZ2WGRveHRHV0diTllsMmhaL052cwotPiBxQSZFOyUtZ3JlYXNlIHEgNyMKRTQvOTlXbGtDZnhlNStHOVMzV3ljZjBCVTZVVGNYTXNmYUtuZ2FhbjBYT0svNFNUbURQb2J4cENQUWQ3TFFmaQpJcjJjc0F0R2NmVVZsWGFkRng2YzRFOUpONUs1V3BzYVVtMGdmRUlhdkEKLS0tIFREM3RVNFpUQlVrTkRkQzhTSlQybzJ3Z1JSQWJWdVpsTkRnYmZxYnZ2MzAKl/A5WOV6wviSTpklxPFlKkHOoaUdcOeRsveahp8+UJVnu1VH59DOyF7KRXLiHYFMtbi63S/3xn0JRhJzNWyh91E2hXo6FYoFIkC7BBOT/1DjZOY=]",
    },
    SecretExample {
        name: "GitHub Token",
        pattern: "GitHub Token (ghp)",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBCUExPdlh3QlEzbmIxbXNVcGlzVHFaRmVTb3JlaDlyWVArWHZmeGwrcUhZClE4NHBPRVF4a29ubStPYkRtMFRFT0FpVktkSXpqcFNCNk9NQUJiam1yUzAKLT4gWDI1NTE5IE1sbWJGSEgyaFZvblJKMUlUalZFbnRnQ0VUS1pzUm5BT1R3aUgxNDFIajQKcGo2dzBGQkgvb1pQVGN5a005U3NiYXQ1NXZIUXlURFhGRGk0TndQa0RGSQotPiB1LWdyZWFzZSBqSzpdXllZIEprdT0KKzhrelhBWC9ncDA2S20za1JicEdDcGpMa2FoR29WUzh6QUpoMUpRTTY1cmJvZFhSWnJKMkJCL3RGeFI1NkFMagpQWVUKLS0tIGxra2YwaUtZQlRzUVF4N251eHM3ZlU3TnVMcnRodm8rSS9tWHgzbE9aUGMKP3mn8CihhzSQqmEDb7aHp4NKe+hd8m0zB3bRXngi4dxBuaDazIbvnJApE0AoXZUU+KjOSq65IpLyCaV7Uafhlqy+GEJtdbEx]",
    },
    SecretExample {
        name: "Slack Token",
        pattern: "Slack Token",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB0WU90OG9hT0FuZWM1OGhPSjM2MVpjOU5kNVFPWGlqbkFGZktMYlppdnc4CmVUYWNzNTBCZjhpR0xTQTFlc050MnJPSDlPN1MxSitNWHdrVU5HZHVuR2cKLT4gWDI1NTE5IENwUXBSallkcTh6QmJENExtaDdnTllQWmZDRjVLQmMyK3pONTV1ZHRnV3MKREtycnE5TUpyZnBBOTdMVHdDaU9aczh0Wm4vOXVDU3JWZStFa0xGcCtqQQotPiByNV8tZ3JlYXNlIClRQWJUbyBSSiB3Q0MkWl09Ck1xVURBM2VndEhWeVBuUUJyN0NaOU1TV2dYRk1FQUhXU2NqdDllTkdUeDlGcXdVTVo5aFFyNHRzdy9jZjAwTjIKang0Ci0tLSA1clhES0Ewci9IUWFoVzhpeHpkaHlZUjd0emNPSEwxY2YvaGxxZysxK0VZCiFQH9JNXr8+I2JmY7RiCCcQhBPShwHUz6bqRVIZmzF7hwhWJxdnnU6BLTxIyQ6YsMXl05FvU+xfT2jZn8oEDU9IPgm+LucQu5BPr5gqM3KoVreiVel/xLQ=]",
    },
    SecretExample {
        name: "Generic API Key (Assignment)",
        pattern: "Generic API Key (Unquoted)",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSArNllKeG9rOHNDQmdnZFNPT3ZJUktXTG1HOFdxVUdOK2FyN3lGQm9kVjJFCnFWajhwYmNldFBMWHVMU09Ta1pDU1F4ZGtQVk1xUFBSb3V3dzMybFN4QXcKLT4gWDI1NTE5ICtPaGFJTDdXclhkMkk4Z2hUclEvU0puekRGNHpNekx1bVRvRzduWjIvalEKRmtzK29PVGQ1ZmRkQXJHR2hWYVdEWlFUQjV0bGxPWVZNRnRwTFRJYXlObwotPiBBSEBkIzgtZ3JlYXNlIHZUTGEge3g0KTpafSAtYjMiRz0gOSoqY2NKClB2SzJmOTZTekRueGxJWEprcFZQK0VIRS9PdkltNnIyR2d0cEJCTGxTaGFsTDFpTFZtQVYrZGlhR203MzFPa1UKa1RCVUVaelRPYk1kYUhSUTdDS2gvZkZiT28vL0tVS29QNzZCQVVSUUVxMG9NdmcKLS0tIGVrYXd3bUpOVHB0cFRXaHljVFJjdk54SGJCZTkvSlpxMzljNHg2cktyVk0KegH/A8PaixwPeTgY5vTt2IeEuetFxlPwVtMZVlciCjnyWY53Ih2Syv36iIsbovYNHt3ciTxloDbjyyRTWzSdrReaZzGX9jjM]",
    },
    SecretExample {
        name: "Generic Secret (Assignment)",
        pattern: "Generic Secret (Unquoted)",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBpUGprS2dHc2s1S2dPVnpIeEdUT1g1ck1sRlRIcXFDSmhTNzZCQWozaFd3CmRqVVhBMnFXY3dJdHF3SDlUeXppK1pZc0JCL3N4eVVMdTYxWjE4OWV5WGcKLT4gWDI1NTE5IEdvRERwaFFqNDVhc09RRWxLNHc3ZGVNaWVROVdySjJqMUtQMXBmaFdYejAKUWNpeFZqcjRDR3BYQ1RXRVNNUlpiNmo2VkZVOHk0QnVCL29ycVVwZTk2bwotPiBfUXg4Zy0tZ3JlYXNlCjJLN1NsTHlNZDNqTDJ2SnR3N3lGRWs4NzdYT1RTYVZ2KzZmamR5YmVsbDRCY091bGh2Umx2bnJMdHhUbEVvbGEKYzAvYnRGVlRNaWhpVmxkK3UyVnhlQQotLS0gcDRmVTJXcU5NaXQrWHp3MXV6VnM2QllhbWVPbVYrdWQ5Wm1zbkRaWTZEVQqroy864VBo/H83t8Yzk0fXeQX1EmskwWeh9yjRM33wxSjcu1eQxJiMGAsHnYa84lWldV0NJl5pN4k2bACuoQakD4iDI/m6]",
    },
    SecretExample {
        name: "AWS Access Key",
        pattern: "AWS Access Key ID",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBHK2pjMWNkVzRyMVZRMGtsVFJGNXlZZWdCOUpDcWszaXJ5bk4zTkxkeEg4Ckl2L3cxMFc4MGplMWllS1V3SFI2Mkx4RmpGMFNDZ0lSUXVYenpzaEpEWFEKLT4gWDI1NTE5IDNQaFQwMGhZVjMrTElzOEcxOFN3K2hGcmtjWHZvb0M2aXhWUGZzdUhoMVUKeWhxYmIrZFlaK2JhR2dub3FGaFBjenBxUmZKSkx6cHMwOHd5ajZpOTNtdwotPiBSYypNMVY0LWdyZWFzZSBDbVVFRCBaLz45bQpGa1M1RktRVHZiaHN0QXdycXNtWVNRaVlxNHFubVZodWRRblZxUXFIcDVDU1o1Smo4VVRObDN3RG42VGZkMXRtCnVreE9zSTlNNlJsQmR3eWo1OVFhUFhoVngrZmNpUnZ0TkZucytoSXF1VmlXU0YzbjBhVDV4dU9VcWxxTAotLS0gOFVjVXg2ZzlBNTJIdHg0K3o2SFRPbnVnZ0NucTEwVTJYbldZVzgzK1ZWbwo128QoyaBp5CNz+7MPinfP4p0C0Zp8ApCNpwYY090QVfW0U57UGDmmjYmkGDW1UMiEnYnI]",
    },
    SecretExample {
        name: "AWS Session Token",
        pattern: "AWS Session Token",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBFZk9XWWpBVmpoSFFYam1kMWtnb1BOWE5rUDJsdTd2Wmt3Kzd4cVErU2xvCjlUd01zVEhQY05HMjV4STF5OFV5ekRvMkRTY3hya3Rzb21JY2szdjV6OWMKLT4gWDI1NTE5IFhyaVBtRGtBNVNYeVpLRkJtR2Qzdk9Nblh4SW5sNmNxRHpUVUg4SWMxUjQKOVBEZWdQUHFxR3lzRHNNZTZhSTcxM2daeGlKREdUU0NMbXJEcWUxK2J5dwotPiAnQF8tZ3JlYXNlIHMgZ1wzJkMjMCBoO1hjSWRjIEUKNDcyeDRHSWg3bmM3a3ZUWUtQQ2NOUQotLS0gK3p6a1oySW9hR3QrQzBVQlhmaExsazF3NjZ3RFk2NjF3OUp1bDRQNGIxawqSgLpgenCsz6nuHBsNTKGezXJHyammtWezomUCDBCLnW4G1SxU2NStKuW1CoiqGhTlWQ50QvUDg+on2SYzAuX2Vbvzq6U5Qjf6oLNqJ80QuFkn2IhvgsMSg1KF3U+TYoA1aZpNrqu9TsTtitmsKJZ+2eMp8xeWcBNYaOA=]",
    },
    SecretExample {
        name: "GCP API Key",
        pattern: "GCP API Key",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBFZDk1d2xTRWxXN3JqYmpRVjRSOFRsbGN2RnViU3NUY29FWGFhZ1N6b1dRClB1MnZ2OXpXWks5azdmazdWaWJ5NzBzVFZQWnY5SXZhOEZsYXJzT1FCaEUKLT4gWDI1NTE5IGZkVVBYcUFnb2FMdXp5Q05WVlV4QTR5bjNjcXNwUGh6dWFvRGVlMnpmQTgKTWE4K2ZMWmFxU2R1WHB5VEs0MmZBSWI0a3d0RENJTVo4dlhXMkxGWGdyawotPiBjLWdyZWFzZSBLblIgLGI5MC9CaiIgLGo5YwpjMzdhdkFaK096ZUZUUjVVUStYZHBxQUdJdVI5ZE9VMEpvMmVBVFlOWjNvd3ZDVENuMHJQbU1kVjFEaHlxdUhSCnFGcEYwNG1pTURhNkdOWVJtTXovZkgzQQotLS0gZW9VTWRLbnB0bU5FWWVrMkdLZnVoL25hQ3R6eFpyWnUrS3NrZHkyeHJNUQqJfFtf763ejRN6SPpBE7mdvsS9Evs5DiMUpnHb+Zn4okZUpy55eRfPd5MrNh6Mm7x9/F72czFjSxpPwgSr3VugV55AldD1Kw==]",
    },
    SecretExample {
        name: "GCP OAuth Access Token",
        pattern: "GCP OAuth Access Token",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBRT3c1ZE1XTTRIek45MmpZNFBMV1NMa3lTbmprRDRKSFR6RG9IWTlaY3pnCndBVU5WdDAwbDNabnR6TXJoSy9OWjhnd0hXK3d1dkZQZkhaZVFyM3loT1kKLT4gWDI1NTE5IDdmbmJJcU1qM0VSU3gzRGZCbEdHeTIvSWFSNkdIWmxpWEhoQ2JBRXZyQlEKNENKMU5KSUVoT3FTZVlPU1M2c0UwRkR3U1VNREI5UTZWeWs2bS9MMjBYVQotPiBIYy1ncmVhc2UgW2BCPyByZkNlawpQQQotLS0gZ3VrQ0xzSDhZRTRLOHduMmRQVVFPbXY5VzhzZWFpS0Z6MHJZZ2tNaFc2NAoJLboByefTiSa7eskVazR8kNi/R2ZuaRU+ksuHQSV80rJqWjGtRVvq3fXmZbV1UAO4UJXKtg1pwG3DC+fgrwdUmpq483syAyabck/RHcyBY6pI0O9uUbxEmw==]",
    },
    SecretExample {
        name: "Azure Shared Access Signature",
        pattern: "Azure Shared Access Signature",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBTckFvcXNsWlZIcFRveWlaODJjaVc1RkVyWVEyYkNBRHJxQVVxTHRDZ0V3CjIxc2VrM1cwM2ZGaU1NZUh3KzJlMkNmalhYSnRNR2dtdWlPVTY1UXk4bGsKLT4gWDI1NTE5IFpKb0VEQnlPdng2bVRaR2E4d2R6bmxkVzZTRGoxZFUyQ0wxMDFaQnBqelEKZEpMcyt0V2crYklpWE1TeDdma01rY21venltTG9nZDl0dmd2RzhHaFZwSQotPiA/fkV8Ii55LWdyZWFzZSBFU05+NyAwJlZxLm8gbQo0WENBNXVLTWJOZkd0MlFBdk1ueDV3UjRtZlpZSzlIMVdPZFB4bVlDSGhJRkV0VzNrTDE4ZHNCYlRDUG0xWjdxCkNQcitRRzllZ2ZBWkFHUHlzWklWU3dZCi0tLSByMGhKVnNXazM4T2NYL29rYU4vdjFuNVlZK1gzODJxVXlNdUprd0NKY2RZCqEekoaAqrjSSERzF8WGck8GbZARB+GjHACmf3Crst4Lx5843JzjtPFBEG4i6vhStfWiif0fOfO29k2xV9vkXrqLBkoMURVAP8rm7xKf3VPk2GKO4WtbgjNNcX86GdUgTUUvhAtwjOx/pg==]",
    },
    SecretExample {
        name: "Azure Storage Account Key",
        pattern: "Azure Storage Account Key",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBSd3NWYm50NXhOV0JiQnJ3bmRLcmZzR09rdUJmNk5qdmE1a2FBQmJNMEJFCmZsakkwYWxZNUp1R1pmME45ZTZ1TERzNy82anVvSmZLK2hRU1NGL29Qc2MKLT4gWDI1NTE5IC9SdTVXbEdzYkNOZGNVakx5NDJKL0o4OHpNcjJrYUZjVGlsY0EvSVhxVlUKbmJ1OXErYllBOXNrMmNBNjJCcjZkcjRyZDRNbVBSVm0yYUhKRWdUYnBiOAotPiByLWdyZWFzZSAhSGosfEsgdmB7Pn4pKWEKakNuandWYjFOOGszdGUzYzh2alZRQTlwOTNSRkRFa1ovQ09HZXJrVWlsSXF0S0VmYmNDTjR0blQyS3piQVQ1dgpkazJubEdzUDJPU1I2TW95cUFGb0N0Z2I0UQotLS0gWnRHWUdYa1RUNGc3K295Q01vakJNa2xCdzk0SFg2Ykc3N2labWNoKzR6cwo9CWmbuwqGlHwaADiWa8bqKLdE901tGmpYbr8cil9xbCRIaaLAZ/55nSnKlwqGsSmE4dE/E2RApiotEn2HEAwZPUSVTeWgSTIgTCbcC080B5C0poq0RFGZ8tfBV0jL9IKM70TCirFHAGcQpdMJifxtY0fWnTNROMw=]",
    },
    SecretExample {
        name: "Alibaba Access Key ID",
        pattern: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBycENEWEYxL1drVmY4OHhzYmw3c0d4ZlVlYWxMbmVaOGdnZ3JQY3FLMTJjCnUwNlprQlRBd3d2NzIrczZpUWM3dmp3SnhZU1ZkYkg1Ynk0T2hFUnhsencKLT4gWDI1NTE5IGU2QUxmSmpWeDArVDdhZXF4dE1uaWtyRmxOaURIR0s4Mk1tRGNVVG9keEkKQ00vTnp1bzRkc1JOdS9Tdjk1YmtwS1VodWRac0ZCL0hTUTRVaVZlZWNGOAotPiBYMjU1MTkgVUNlb0FQMEM3SGc2S2JIRWxVcHFLbnlBMkVyUGcwalNocDNUUmlGUHd5dwprWGRwU0hzNUNRQVptcEQ3dE9xbFRtdlVkY1ZDMGkxSzdKMWlPZzB0Ny9ZCi0+IFgyNTUxOSBMcGpEQmhGbmVWZjNyZFpBMFVjbEFPYXhiMVhudDh5TDZ0K2VwR0VQUnlzCmpmSEpDMVVxRnV6eEViV3U1bEhBQ1RwcWNUc0l5M1A0Ky9jbkp6ZWVrY0kKLT4gWDI1NTE5IDl6MmdOVHNrRE1uZnhaKzZkcmQ5TkZzdm9nTDBlWVhrVEhSeEpZSWtQVWsKQVlmYlhjVHJ5YWJtYitOanZzQ0FKOEpIakVjVkpCeTZhcnl2ZTdTd2VNVQotPiBYMjU1MTkgL1lmekhhaTc5WDBrS2FsMHl3SXZsZFpKWTZLWWVsajVlcXJjcDBnTEJtSQpRUzgrbXJnTm9MUk9oT1dqMVB1VmUza1YwYUhJTFlaSzBSbjhBQ2dOUkxNCi0+ICdcLWdyZWFzZSBGSyslM31WcSBFXz8uSHEgXVVjbiAjNwpTTTFCcGFrZnRxazFkbGl2OEsvcWp0MUFXcXZhWGh2ZnhKeWM1ZwotLS0gYlBsbU9NS3ZGeXNmZllqNnV1TDRpa293S1czYkZmbVl4SUcwRkhCVHhEYwqfUCrq+PjIRbGQd7ua+yMVM9QVeFIY9iDp8qnlxxjZDaOBqmcKkfsJCeXL4vgAFpSudr3QZ4Eak7itRtqveK1zoSe5TL95airxR/KlifNjYPjbexCGnrj8TKdMlQ==]0+IFgyNTUxOSBsRzhHMGVWajV3VVZzMThrRDJnZENpdTJQbE9acHBxczlrR29CZVpiekFVCnEvcFkvYy9saS9xb0RCNzVIRk1UeWRVTjloamdaYlhlZ2JNYkhFTWpuOWMKLT4gWDI1NTE5IFd0eHA5UnpiZDU5NFI0WDRpcEw0N21makpaY1NKV2N3dkhOdXVMYS96UTgKaVgzNDRPMG5Gb1ZBSFQ2OWlkbmIrUkUyYlpPaDZJZTVDRTV0ZnBJL2Y1SQotPiBVRSRuVVNIdS1ncmVhc2UKOGJ0TEJ6UW1yRXVPVHVJaWNZSWYKLS0tIFJkZ1ZGN3JjMm1LSWZiK3RxQ3FmMHNxaXM1Ui9IaU9Kd283YUxQZ0ZPS2cK6ApBzXq69Jv280bosbNibjGyWLPs+o2Tlmq8cn3SmgvLkJGUcVyimsZ7Wdvf1rzKfO3bE4ItV2B6I8DNSLxzFSpJpNrKLw8UegyYm9ZVoZM=],
    },
    SecretExample {
        name: "Alibaba Secret Key",
        pattern: "Alibaba Secret Key",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB1THh2amhPY29FWlJhMTRXd2hnK09TNkdnVFkreVpObytOQXdzbEU0NGl3ClRnZkhBQlRwem8vTXZYbDBlOHg5OER1MGl2MDBZQ0tPVlJvSy9qc0xiTHcKLT4gWDI1NTE5IFAwUFdGSlRzK1JHdWNZVUowWU5KM3piT0RmUVRIZ2dDZGF0TldZSmRyVGMKTzZzTFZBVzYzWk8yWkluSTNRUjZZOWJyUHBtRFJiSGFSVlNyMFF2Q1N4RQotPiB5PzZpZEYtZ3JlYXNlICMKK3hyS2VEM2puWXIrRmZhVlBtc0tKYk5oN2NIYWpJbDFnWkFVSTFwaHpKbnVEK29mRnVxYVRFL2xxdjdDQ1ZqNgpJR3A1bWg3Q1ZmRmpkdmpqS01mQTdxT3FlV2hECi0tLSBUVWRQZHQ1L3NKa0wvMHhsZ2hmaFp1OC9KOWdKdTBIM1VrWFVPU0ZZL3JRCq0bBqQHPtxRlR5y/n90wYNqbgM61nHIpl/krsJdFjM/M3L8iqTGqj66VwLl7L6KnKnlLS6MlSXB9Pc/oMV1DqVs/n0DuCbd+PsLDAkbh7zGkDE=]",
    },
    SecretExample {
        name: "IBM Cloud API Key",
        pattern: "IBM Cloud API Key",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBCMk1iVHZnWW1UejdYVCt4cU9ZYUh2aFJzcjd6eW1mR2tBV1pwWGpBaVJrCnF5c21SMHkrQ05KUVdVVGJ6WUlkSnh1Q2hEaWY4QUE4b243cVRuQjV0RnMKLT4gWDI1NTE5IGZOQVBJd2VER01HUnlVRVNlMWxkRHJUbTd1WmY3Y3pRbVJqWlVhVWdPd2sKbFcyL2dyR0d0bVAxNDd5QUFRK1NwNnA1aC8remtNZUJWdmVzM0tzMzk2UQotPiA/b3lma0U0LS1ncmVhc2UgQyBGICgwIHdIN2tZCkJhVlJBanVWOEFpdlhKeG5Kd0JZWXZBQk1XQ0JHWHJTK1kxVEx2Mmx0STgKLS0tIHExL0pma0lzNnkwRCtKMW5lT2NUN2lTbXJFcDYzT2ZMWkVZeVBsSG9RRG8KiD/bMfpjO+tBYjzoG3kUY0T68OG+gVx/CX78gPIWAKVGga/amygbQFb5a7McyB1PNdiueSyA+AK08kIJ/oxET6z7c+JWYXjMoaNrNqJypj8xGJH+vOlXY8C2If4RxtSI]",
    },
    SecretExample {
        name: "Oracle Cloud API Key",
        pattern: "Oracle Cloud API Key",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB5OW1tcjFEbjRoNmErQUQyRXN6dTZEeE5SQW5ldlphaVFoZTQra1JkbndNCmRRVWJaZ2hJL0ZnNEgwczh3RjF3M0JYWGtQdmI3S05rOFdQajh0U01OeXcKLT4gWDI1NTE5IExNS1JiWFhGSW5NdittYmVrZU9hRUtzWEpEejh0M3M1S2F2ZmkrZGJOU1UKMlNCcEh4UGpwdGtBTXUrZHQxMjRnbDlMc3poWXVGSGxDUVo5YnV3OVJaUQotPiA5LWdyZWFzZSAsRE1PbiBtV1VRX3diWAo0Mm5yVGpkQzZ1Y3l5SXk1bzF5QlBxN1ZCSFd1em9teXRCYTh2K1dIZzhPWFJROVVWWjY1TXdWa2hobGdZUDJBCmVhWndpcXZCVG1DaVhSNFlqR01mb2lOcWdvQllzYWVRaEJzQWNmbFVhT000WkEKLS0tIFg4VnhDYmJ5U0VVUnhrMy9iRWxCQjhUL2ZiSjFxdEJPSGpWSitwOHNEcjQKcoAdjzUGKGDDok/PfksEMFEaGoyTYXQ6L68tFeZBf2wqkj3wCM1hB1kyIFuuLU7nLPAh].abcdef1234567890abcdef1234567890abcdef1234",
    },
    // SaaS
    SecretExample {
        name: "Stripe Live Secret Key",
        pattern: "Stripe Live Secret Key",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBKYTlVSkVWYXZ2Skc2UlArUWtnUCtmakxpcjVCSTVWYzY2TklSOEFZT3o0CjBRM2VUQVFCQmxQZzIwWGoxbW1xWGZjY1VyWGwzdmhMZ3hBRzFVNG4yN1UKLT4gWDI1NTE5IGVTd0hSQUVta09WeURRMWVIdFlWS002aFJRUDQxQkpIWjNraExKb3Nvd2cKb01tZnhJKzBVWllJSmEwRWMxbG9WcGlRc0FFek5jR2hvdHBvSE93VER3VQotPiBkPC1ncmVhc2UgezooIjYwRSBOPHFAXmovIENJIHRnaApzSFFnTmhhS3NwbHdJejMzNWtOeUR4cVQxMk02UjA0ZTFvaGpZaDR6MGF4Qkk3d1RFaldFOXg1NnhzSHpBVitVCmJCV3pFWWZGcSsrV3hSSnBPdnRLZmpTNUkvYWVzL3RkWm5qa2FzVkVEMFkKLS0tIGpaV2poZjVibmRKVnJjWklybWM1cUtGKzBNRUpSbm9sYWw1cTJQZGx6VU0KPcKjE5a+vio0tqU6yzbdg8bnk3FtlmX8YJVsow9WWamdjCbSr6Fc1bzW/1WJ7EAF3CTj59W7lJQyrfNz66GAdx/oTjZT/Q==]",
    },
    SecretExample {
        name: "Stripe Test Secret Key",
        pattern: "Stripe Test Secret Key",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBRNkVlcmhEUTNXMy8ya2s5bjZxM0x0WnRlMGd0Yi8vYzdoZjkrT2xnRlZzCk5rek94SVB5WGhkY3FjelRhbHJZQnFldTRHV1UrdCtIV1lGWE0xSWpuWk0KLT4gWDI1NTE5IDZPZmprTnNpOHlVVUorRVg2aExrcGRtV2cxV2h2L2lOQzNXUTBkYS9uQ1kKcmYvQ3hZMmVjakFhUktjeGVkSElteGhvL29JTHFwVFg3ZEhKVXBYQ0JjcwotPiAsLWdyZWFzZSBHKWNmfHEgJEYgLXo2ClBVMGxqc3ZEYXpFQ2dQWW1hN0NRbXd1N3YyUjhxL0tsWEUySGNKb2tXVFNMODZ5UzFQcVVtUEtuZFhjTlhZVHkKY2hjSnBkUVBQVm5rQW0xYQotLS0gZHB5T1hwaVRPVEg4UDNrcktZV0N2UVJLcmFuazV2R254YjZGZ0JvaUxwdwpztbU1ehFQCo5bhAavAgW1CV2G9CAl30StElYLsNu9YU3k0xILEAPYiaHI3OmWL9WuGK3IK2B8FjSvdC+vfik9FHMFHWmj]",
    },
    SecretExample {
        name: "Slack Token",
        pattern: "Slack Token",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAydnZZTExob1dSdm93eW9XTUtrbzhSb2JkVVdyYVJuL1p5VjBhN01NUEdnCkFFdE5LMWNXYmw1RXF4dHJ6SUt4UWl6d3ZiaHpkTE53d2dKZW1GZ0s4K3MKLT4gWDI1NTE5IDNHam5tREI3TnhYUFUydVJONHlrZlF4TmVOdHBtemZjVThsaWxWY0FqbVkKRjZOakJBRDMyMlJoaDdyaHpYeWlSUzhPeGVsOHJSSFliY2cvSWZIakJiOAotPiAqUGlAWC1ncmVhc2UgYUhiVjt8IGIKMkxTNnFmcVFlY2lmYUFUcm9hUm55VEdJR0h2OVQ2b1RWUHozUEYvVmZabXd2aVVRa2dPb1U1amdzd3ZYS1BLVwpVMGFoY2dmY0hMdGtrc1UyTnV4cS85WHBZb2syRXMrN0trZndXZW5Dd0VyeFR1RXRQYldYOWhzCi0tLSAvNC81azRqT1krY3hsbCtPWDBpdzdJZjhNWGF4c0kxS0RKcTZFdUk0S3VvCm9F9mmSupNKJ00IdJoqviV7qh02Rrgado7uRCKRaZ67dA0x9/9Yeclg3n5Qt7DGBhVLcYUGRKMhBKWJpMAHOPE7NnpqrXhCOxk9nQ==]",
    },
    SecretExample {
        name: "Slack Webhook",
        pattern: "Slack Webhook",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBIVEJYSHpqNy9pc3FHWFlCM2g2bFdpaFVDYlRNc2hZamNWeE5RV3ZXQm1NCkpJOHJBUW1HZ2tlQkNJYy9LYmJvcnNQSjUrajNzWlFjTUtxT0ZIZTlVa1UKLT4gWDI1NTE5IEF6TmszamFPTjY1dHlMT0l2WVhDbkZ5elJFNzAydFN0ajJRODZrMnZOVU0KTmowLzJXSDBzaThOVVFIckV4Q095dmRaejJBWEdHNEJPdVQ2eG9pcGxrcwotPiAkQS1jdG17Pi1ncmVhc2UgRkIgbSA1SFtYIDR9RSpgT2AKWHA4S3luNVBZaGRmQ05xQTRBaGFDaXd5TTNZZEwxdTVNSTNSQnNVK2x3Ci0tLSA3ZG9QUkFORitZN1pPTW9ZeGxxRWJJN0xnRHNGM1VENTl2QkVOZDNENWlRCq8ZMlbBUhdKudpb2HaWcfUx9hjbv+81dZsBpnWV8k5PI6VCqqhFTAm0icc8N/F51X87MsMh+K2VoOp7SFEMVD1YEiQcRQufq8647ch0vVjgQgEdDHxktIlDxPBXSgeQw8j8tX8SfdsuoYSo3uo=]",
    },
    SecretExample {
        name: "Discord Token",
        pattern: "Discord Token",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA4aUlUQ1M0a2NKbjZQMGlJOGh3Uk04MTRHMm1xWVJHaHVnYW5tcFBZK2tNCjdTVmg0U0RHQ0tLc1JENzc4STkwMHZFYllVVUZWSFhQRXNpdUpFT3cycVEKLT4gWDI1NTE5IFo3a2Vnb3l3SFowd1NHbm9uNGNMYTdjOHVaNDg4VFhWdkVYeTRuZGZ2eFUKMEI4b0psUDZkVGNHMzVmZ2kxVmFWTlhYUW81dmtCVUdWRVBpZVdUVVQ0RQotPiBwKDx4cEEtZ3JlYXNlIFpRLSxgIFtbI0MjCjhnZDNwRmRVNm83VHN6dVYydTR2N0libTRpd3RzaHVndkczS2Y0c1laY0hEaDJxY1RoWlRRUHp2dDBDV0VPR1EKQUEKLS0tIExPcFFvRkRnNmNNYjhibytLN2NNN2prcERJZm5MbXVYTlFqT1l3ZTdWbVEKkAyTzMo2S0sGksp0fzkEtRzPOIOFglNx2rDVWtwOjbKExf4ZJgZNRQEBL2HLqzEEs+wwPvgR76/FSAF9xO3udhCJW0NkAIMiKroUxVIJyNuhx3Y6s98pCvk7eg==]", // M + 23, . 6, . 27
    },
    SecretExample {
        name: "Discord Webhook",
        pattern: "Discord Webhook",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBuaVkwVzFBZUhoakNFMzc1ZStCRkVDaVArQldiZXdDTU5ETzlCSkd6T0hZCmNSazN5VG5qZTQvM1IwTWFrN2sraFVMcGM3S1dnSGFTUTFJNUpXYlgzZmMKLT4gWDI1NTE5IE9aUWVLU1FUcmliaWpIaTdkN3VrQ3ZFUG5iaVY4NFQyOGNjNG82MW1qaXMKbW85SzZ0cHVINkR0QWNlVGFqV0tCelBSWUl6NEh4a3dtd0hRMlo3bU1HawotPiBhd1stZ3JlYXNlIHMhJSA9IHxZZgowaEZBTFJPZ05XZWNORkJSdWJiK3VxYWpZcGlZZ2lyY3QwV1JFbEdXTTZQTURGYTJOVnhYbmFUNzVoOHhTQ0lSCjk2MXJraVlSbGx5eEtIb2RFRVM0MXBnTXFHUDYKLS0tIG9sWUl2ZmRCQ01QOHVGUis1cU1YeVIwM3YwRUhETzVxSEhrZEtEOTFSVVEKU85cBEbH0HVUSTWEQC9JRx4kFG00FJz5DFsJe0OchE1mRUjPnnYCWBXi/7ueMn1w6Jso5Y99Vgamas/++dATNxM0UzNg+ZfaeFETEWoeD3fnOHHFsAN7cj5tujViGo3Ns1hRtTwqYuaRL7CABs5dbd3i0bDhsCeB79RaVw4D+G6dn6Uh/4iQNGUZWn3EqQijnuY/zA==]",
    },
    SecretExample {
        name: "Telegram Bot Token",
        pattern: "Telegram Bot Token",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBuVlFQL0NtZ3NmQjJVcS9EcGZxTlVrRFhKTlJ3c0JtMWtZcDVNdFdjMGtRCkFDRHBtVmtrYi9UR3Y1ZlNQTEQ3emd1Uk40T3Ywem5EajNZMTBreDVjbEEKLT4gWDI1NTE5IHZQM2hBNHk4QW0xdFllQjVBSkdQOGlOZjJPWkRhT0xTQ2xjNjJWWktBM1EKOWFFdGtLTzVHTzhsaTk1UGRPUVYzb2hhejRtQ3hDV0QxeXRzbGNwQy9TOAotPiAmKX1PZ2RccC1ncmVhc2UgJEhGIHIhLlYgSnBJKTNvCnZjSDFGZmxIcXhRb05sdVQ5aGI2bnQ4bzBkTDB0cmlSNzhFNTZJTHhQT21PVXQxU29wN0NMc0tLcU9SN0tld1IKT2ZVdlduUWZPUm8rTG8rV0NiZXpHWVRQCi0tLSBYWWlDSnNKTWFMVFR4dGgzbjNUeU1WNURSQkh6U1B3emwrcFhXRFJ5U3dFCos44YiukWEDn1Cw3MdvNd/cxlTrj4ogfCM60ZpFeSzQrLs98ymixB3n6djbxyF5zJHZKtgSXQ3TTZb/8wuaoOpjFAItqDaGIGGAyAu6Tg==]",
    },
    SecretExample {
        name: "Twilio API Key",
        pattern: "Twilio API [DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBldEhHSDNvNGdVM0I1S25ieHM0R251L0hjd0FyVExDQ0hMOGtWUmdUcmdJCmxuSnRGenVRQitjNVlpN0Q5RVlZWExOaWJZR3hURHFSSUUvcE9SYVZkekkKLT4gWDI1NTE5IEVjQkJ5Z3d0c3FQOGVubFlHMTZUM3RvTFpLdXJxQ3lhclVkS1YxY2dLRGsKaEMyWk1pa2xTTytCbFBPem5VdFNiOU9NbzZXKzdoY0JYbHg0Ymd5aHFRNAotPiAzPlAtZ3JlYXNlCnFIaTVUTEx6UWFmM2NVR0tLT2Q3Q1JUaFFMdDlBWWtHeHFvWTRZYwotLS0gNlpGeUpMdDgxVDlSM3BTekpEMk9meE1PaEg2TThWRzRoSHpIVXZtZEc1dwoVTdud4ihMlRglguVO7b4it3FYaKyRmSIXTuur6g1OUro3XiUtVMWvy1YW9aob/fKHVZKN30LD11rDa+IVjT+FtlBiZ5a7stcupxZhxbg+uhT+Wf9ukqk=],
    },
    SecretExample {
        name: "Twilio Account SID",
        pattern: "Twilio Account SID",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBkQTJDVVNjUFg0cFlhamRXWDE5eWNyMmpFQzlBTWxKeHdPdXlFWDFOUnhnCkd5cUJjMHdWRnRsTjVWTjlNb0FoL1NESTVkV3VUcFVxTGVnbzk5MG01OXcKLT4gWDI1NTE5IFRGQXJZa2FJazdNc1FSR2J1b1VIVlJsdXRrN3htNVJweVpvN05GdUU4MUUKSHpJZTJUdWpxVFBzVU1uSHp6U1dXWXZDYW42TGNsc09KalY1UkcwcUY5WQotPiBZezMnbzBbaS1ncmVhc2UgXTthRE08LCBeIGVfMEI3VGArCmhKVXdXVE93YmxCSi82aVNZSVVQTXhRSTNjQ0dPYkVZSjQ3eUt6Ry9jSlpxNU9nam5RCi0tLSBROFBkY0lhM3V1YjN4UmpGaU1TYUphWm1xVkc3cFlPTFl3MlI0a0hrQndjCrigbnCzukxQ54GCeDNQadswfMQJIDsMNduX+1BRo5FVPKg7DemSF/JAtHsm/hxqyZXC58ViAPwv5JI7oPnKtBGvKw==]",
    },
    SecretExample {
        name: "SendGrid API Key",
        pattern: "SendGrid API Key",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAyekVzclZvYnFobndDcWo2YUxqNGZBa0tER052OGZtdUZFa1ZFa3VZYUJNCkVyVFF5WUZQSm5STkFqVVR1Tk82OVNvYTRZU0U2cGIyeVlmYXZiekRibUEKLT4gWDI1NTE5IFJCK3V4OXh4b0pBb3hQb2s0OWRHUVYrczdGVGM4RWM5V1gzdW5sYlF6aWsKTVhiR0VtSkFjYzlERUVvaTIvb2ZvQUZGSGdXc1did3AyOElNTDZ1ektWVQotPiA3V2pBci1ncmVhc2UgXyVjazptN3wKNGJxd2ZWbkxHUmRIdDJjMWM5YlRneStjMlRNSUY5eUFRRCtqc2FFYnNHMzZSdk5TT2U0WFJ2aEZlMVVQNHV1YwpvbUtsMlIzemd3S3FMMi94ZHJoMWx3YVppQ1daMjRRMUdmL0h2RG5COTNOYlpSbE1ENGhIWDNFSEZRCi0tLSBOY3hYT1hMRjlZaktGNjBTYURadVh0Tmx6akZVTk50cVlWWlFaNEdETHNvCmFHkxKZ03MWXFFGwG+/5rOJKdCSpiuJF/lfwQ/RveQfyJy6jWlOTBK6LeP3T25xRirejDd5GJ7tITHxYZdsjnxxv8znPgCeToovLYtVWnY/8cZj9kR6JnV90Z0irTgEoqboT/MT]",
    },
    SecretExample {
        name: "Mailgun API Key",
        pattern: "Mailgun API Key",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBDRmRCRkFXREFDWlc3M3ZuazM2aWJLbFMxSnpXYkQveGNETTdoMjRRbUdzCjQzVVV1QUVoV0xVNGpUMHNYVXBJTVRFbWRtNnZra3gwT2RKMjNWbkZqUjQKLT4gWDI1NTE5IHdiY29iMjRmSUh5cS9TSWpBSU9OTEpMY1ljdmVHcUo5akZkaW9udkE5MUkKallRSlpkeE4zdGYwUjJjRkdSNmpqcG1vYWNrcVFoY05VakFRODJoTjNUQQotPiBqZ2FzbiEtZ3JlYXNlCkdtRVNoamxHdW5TSzV3WjkKLS0tIE9ldGcrWVhBcnc0dUlScXBuVnJqYjJaeDRCNXhTelJWQmZta1ZpUlNGY28KV5gCoQFsPfm6dCEC6bEEuvNmB5AH4ADAb9CWaT/GIX2oTECIVuhjbLWN/QyLyk7p3Gaz/+9rskEIwLT3+2zEVBkblHo=]",
    },
    // Database & Generic
    SecretExample {
        name: "PostgreSQL URL",
        pattern: "PostgreSQL URL",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBYMUZQVDlzM05qNjk0cjhmZHFLQzVkMXBsMUpNYk5Za1FUbWtDMkY5V0c4Cnk0cDZpTW9Gd3B4TjFiS0J3RzlrUVBTOWdsU0ttaVpLMHJYaTJ5dUZaTDAKLT4gWDI1NTE5IFJQUFJIMTFDaEt1dUwzaEV4TThJa3dRbEJhd3Q4YWFERm5DMVNoTjZKVDAKQkJLaHp1ak9XU0szTTN2Z29PR0tTWlFkQldWQ1BhYyt4anREWTlrc3JFcwotPiBjQkAyYi9mLWdyZWFzZSBtIE4KTXRDWDYwbkI3ZGVkMmpKTTRnbVRKSEw4dVdzCi0tLSBlcllsRVpvSkx4RitUQTJ1N2M4bFh0cVNyQUNidHRVeVZnRFlKdGVnMENvCsmac4/ke8c7f4OHOT3uTceOQL0MB/0iIe+rgVyqRgncww0MpjilKsrl7qpFMK0dAuUtmj4oiKF12tBp6/6NWBgIuMzm8Zxz]/dbname",
    },
    SecretExample {
        name: "MySQL URL",
        pattern: "MySQL URL",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBwcmRKVzZIKzJ1eXNWN2ZkN0pYUTBTN0psMEI1VDQyRzdhZS93UFYwU1RJCitrMkgya3YyOXpYUzZ0SkYvaGtVeHR0bkpwUURycGxYd0d3d0VjcEdGeUEKLT4gWDI1NTE5IDNBSFZBOURGTGc5cTRLa3A0L3VHRnd6V0E4K0F0aFVhVjU1THNVNUdNRW8KNWVaUzlSUmNxSDZxbjA0L05IQ2VZTUhYNy9kblpWb3hDQlUzVmJxeDh5MAotPiBjPDxuJC1ncmVhc2UgQixtZDdfUyA5XCYgV1dZY1VqNigKMi9Wa2llRzhYSGsrcm95Y3c5a3ZaSG9qT2NGV1dYMDNCdUw0QkdkckhXOGhBVTNhRklZSTVWaThBOXNHY1pNegpYNGdjTHVBdmZ6aGdJbkp4SXVQcjM0aU5uYWF2MkVQN3dpd0c1NFV6ZlAxKytES3VXQnpUMkFjZGR6Zm5NVzVLCmlxNAotLS0gejlnNHF6dUNhOFZQRmZCSmt1anFwM1ZFUDFzK3BjS2RtZ3RRK0xMRlFzdwo5VSq+fy7f6/eihZfXrwzNawM70MfDWWO1KMSUMwNgSp8jzKeOuTjeUYRmzL/RA65EZteBeMGoay6qGIiPRo+IX1dMSw==]/dbname",
    },
    SecretExample {
        name: "MongoDB URL",
        pattern: "MongoDB URL",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBmVTdpRGNJcXZ3ZElaenR4cXl6bTlUUjdRS2pjeFF5ZGxjRFdCK0FoeEVvCjd2ZVA3N0tUVTlNNVZEdktobHdWaEo3dkk5VUF0cCtzZkhLNy9kZ3U5RHMKLT4gWDI1NTE5IHRoVllpQXVKWDhXSG8vRk9ITStKblRRWHBiandmbzB6cVJsN0pLbXJMa00KY0h2dUpZOW5rUk5VanZlR1BnbkV5M25qOUFuRHVKV2NDVlE1alhIZXpuRQotPiBCPmZhLWdyZWFzZSByJiA5VmtsIGEwOzpSQmNtIE1xaEtkelgqCk1RZzhEcDEwZ2w0dU9nbXF0bzRyWVJidUNGU0NVRzNqOGR1SjNQK05mNUVQcnduN2t2OEFSbTdhWkRIa2RURFEKVFpNRzVzWQotLS0gQ2cxQjByaGFEQ0V2S2gxMkVBMlpteHBTM0t6bGZybGl3Y1pycGxmTktFUQpm9yAjaYV18RfvZYmfaxpPaSltg6E+f/HhbU5dXfWpUtGEc4goeX+BP89BjGuR+Q7zgtUGh+H0FgH+16VOihC2SPbjQ2mywg==]/dbname",
    },
    SecretExample {
        name: "Redis URL",
        pattern: "Redis URL",
        raw: "[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBpejRiV094Q0RtaE0zYXZoeG9rcW1HUjNQRmlQTGdtc1A2Q1ZGYVhsM1h3Ck4veU1kWW9CbjRwc1d5T0QweGpSb0JoK0tlVWdLRG51aHV6dkZTT2N6dmcKLT4gWDI1NTE5IHplcm9BM1VNQUlXVHdudU1tTzRkcU9YQUdwUjFmZjRTVmJnWEpLd3pwbkEKMVZuTStjK1pEWGpBSDBRVVNHSmZ2ZHV3eDNubm0vUk1qL0RTWG5FTTBaTQotPiA5LWdyZWFzZQpLWWM1RFBhN0hWV21NWWE0QW4wZXZlU3RTYzI4bkU5WnFZaDVmc0NoQWdXay9URjJCdTJwV1N4dlpEbDA1UFNOCjIwTFZuV2h2Ci0tLSBnc2VGbjAzUEo1RE5mdHJHWWh1RC9OaFpaWHFFSTZ2a2dFOW9Qclo3aXRNCg3ZMqGd0dgqjaFb65AoV8E0RBW4Q2NZlYF2X806g+lIKjWrWWBYQK6GgYOD++z0NsXOD43T9oTkuRfa4bLA5dxWJmqGh3uDSLoaXEoUu6Ydr75J0mUH8u5SnVkSCwuQ0NyGrBG9UEALw6VHFgszciglXkKgtPv7YV5fe6Z5g75onW1qsbf0eGZe/YSWbchwGUNnCSuXbUTmRv8aFop68AOMSepCzzXdUr1vtY6MjtEAu8OsjhZAtaj1XXn2dRtO4Um5nk1B1s/scUl+ewoCJ9MFEVTn8hP3SqcPV8cZDKyPsGBQ0CY4wGHAlvzZ5otuaeidNRjYm7faaMixqCRy9Ifpb81kfUk/W0+DrNaZyBQrzzku/2vmI9lO/seunINIzMOMo4JKYjaZ+Ss9fp8Xp49wRuq+AkCE/oIicUzVohDfZ6bQXRIRLh++r74+lRU0ZLSeVXvgFjh1hhRuHzf5cC8a7As+U7LdM30PUJbsHQWxmmH4pzKJAnWntbm90uTI5nFol4X5k+RwmPzum+A94Mv6GIVk6Gcfj+6vA1KTEVu/gqJbbkDqrhG1q8guA74ACDxwzKLHWIauPYkcvek4UJSUf0s1l+KZTqyt/V/UBtwwZQ9zFTmQyhh5r/ZGQr3OSG+CNUNn+xVpLpisaoWkaM+LWrPZ0PRyImZYzzNaYuUIm/un1jOAJ1Ev5+0bLjy5dEXsX+3vpVfYh97SU1NTwETsj1GOq9q6U2HboiJVK43cSkxsH2SNxmUw1PxKgEHhp/UgQyT0EC0JPWVXgbT1LeK0lg6hZjJw4n3xBEUedy6puQ6y4qez631bHBKm3w2ojMTvF74uRqecfZR+FYqiOFI0jaQt7kXTOlSgBwEaMDmHOTS1NRbY4QA6NHxaeYjjhpIsoN/wz+i7Qc1Av5na2mcoJxXgDMwZ1r0HVMhqRklC]// Fuzzing: Generate random strings that look like secrets
proptest! {
    fn prop_fake_secret_roundtrip(s in "sk_live_[0-9a-zA-Z]{24,}") {
        let security = get_test_security();

        let cleaned = security.smart_clean(&s).unwrap();
        // Should be detected and encrypted
        prop_assert!(cleaned.contains("[DRACON_SECRET:"));
    }
    fn prop_arbitrary_roundtrip(s in "\\PC*") {
        let security = get_test_security();

        let cleaned = security.smart_clean(&s).unwrap();
        let smudged = security.smart_smudge(&cleaned).unwrap();
        prop_assert_eq!(smudged, s);
    }

    // Fuzzing: Mixed content with secrets
    fn prop_mixed_content(
        prefix in "[a-zA-Z0-9]{0,20}",
        secret in "sk_live_[0-9a-zA-Z]{24,}",
        suffix in "[a-zA-Z0-9]{0,20}"
    ) {
        let content = format!("{}{}{}", prefix, secret, suffix);
        let security = get_test_security();

        let cleaned = security.smart_clean(&content).unwrap();
        // The secret should be hidden
        prop_assert!(!cleaned.contains(&secret));
        prop_assert!(cleaned.contains("[DRACON_SECRET:"));
    }
}
#[test]
fn test_backup_functionality() -> Result<()> {
    let _guard = HomeGuard::new();

    let mut demon = DemonSecurity::new(None)?;
    let key = age::x25519::Identity::generate();
    demon.add_memory_identity(key);

    let temp_home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap();
    let file_path = temp_home.join("secret.env");
    let content = b"SECRET_API_KEY=12345";

    fs::write(&file_path, content)?;

    let res = demon.backup_file(&file_path, content);
    assert!(res.is_ok());

    Ok(())
}

#[test]
fn test_auto_key_generation() -> Result<()> {
    let temp_repo = tempfile::tempdir()?;
    let git_dir = temp_repo.path().join(".git");
    fs::create_dir(&git_dir)?;

    // Pass repo path directly instead of mutating global CWD
    let mut demon = DemonSecurity::new(Some(temp_repo.path()))?;
    // Inject a memory identity so we have a current user key to save
    let key = age::x25519::Identity::generate();
    demon.add_memory_identity(key);

    demon.ensure_current_user_key()?;

    let keys_dir = temp_repo.path().join(".dracon").join("data").join("keys");
    assert!(keys_dir.exists());

    let mut entries = fs::read_dir(keys_dir)?;
    let entry = entries.next().unwrap()?;
    let path = entry.path();
    assert_eq!(path.extension().unwrap(), "pub");

    Ok(())
}

#[test]
fn test_encrypt_decrypt_multiple_recipients() -> Result<()> {
    let mut demon = DemonSecurity::new(None)?;
    let key = age::x25519::Identity::generate();
    demon.add_memory_identity(key.clone());

    let plaintext = b"multi-recipient secret data";

    let recipient1 = key.to_public();
    let recipient2 = age::x25519::Identity::generate().to_public();

    let encrypted = demon.encrypt_v2(
        plaintext,
        vec![Box::new(recipient1.clone()), Box::new(recipient2.clone())],
    )?;

    let decrypted = demon.decrypt_v2(&encrypted)?;
    assert_eq!(
        &decrypted[..],
        plaintext,
        "multi-recipient should roundtrip"
    );

    Ok(())
}

#[test]
fn test_dracon_security_singleton_same_instance() -> Result<()> {
    let s1 = DemonSecurity::get_or_init()?;
    let s2 = DemonSecurity::get_or_init()?;
    assert_eq!(
        s1 as *const _ as usize, s2 as *const _ as usize,
        "get_or_init should return the same instance"
    );
    Ok(())
}
