//! Secret scanning patterns and detection.

use regex::Regex;
use anyhow::Result;

use crate::is_inside_secret_tag;

pub struct SecretFinding {
    pub name: String,
    pub line: usize,
    pub snippet: String,
}


pub struct SecretScanner {
    patterns: Vec<(String, Regex)>,
    full_regex: Regex,
}

impl SecretScanner {
    /// Expose patterns for integrity testing (e.g. Max Length Check)
    pub fn get_patterns() -> Vec<(&'static str, &'static str)> {
        vec![
            // ============================================================
            // AWS
            // ============================================================
            ("AWS Access Key ID", r"AKIA[0-9A-Z]{16}"),
            (
                "AWS Secret Access Key",
                r#"(?i)aws(.{0,20})?["'][0-9a-zA-Z/+]{40}["']"#,
            ),
            (
                "AWS Session Token",
                r"(?i)aws_session_token\s*=\s*[a-zA-Z0-9/+=]{16,}",
            ),
            // ============================================================
            // Cloud Providers Extended
            // ============================================================
            ("GCP API Key", r"AIza[0-9A-Za-z\-_]{35}"),
            ("GCP OAuth Access Token", r"ya29\.[0-9A-Za-z_\-]{20,80}"),
            (
                "Azure Shared Access Signature",
                r"sv=\d{4}-\d{2}-\d{2}&(?:[a-z]{2,3}=[a-z0-9%]+&)+sig=[a-zA-Z0-9%+\/]{10,}",
            ),
            ("Azure Storage Account Key", r"[a-zA-Z0-9+/]{86}=="),
            ("Alibaba Access Key ID", r"LTAI[a-zA-Z0-9]{20}"),
            (
                "AWS MWS Key",
                r"amzn\.mws\.[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
            ),
            // ============================================================
            // Google Cloud
            // ============================================================
            ("Google API Key", r"AIza[0-9A-Za-z\\-_]{35}"),
            (
                "Google Client ID",
                r"[0-9]+-[0-9a-z_]{32}\.apps\.googleusercontent\.com",
            ),
            (
                "Google Service Account",
                r#"(?i)"type":\s*"service_account""#,
            ),
            (
                "Firebase Database URL",
                r"https://[a-z0-9-]+\.firebaseio\.com",
            ),
            (
                "Firebase API Key",
                r#"(?i)firebase.{0,20}["'][A-Za-z0-9_]{30,}["']"#,
            ),
            // ============================================================
            // Azure / Microsoft
            // ============================================================
            (
                "Azure Shared Access Signature",
                r"sv=\d{4}-\d{2}-\d{2}&(?:[a-z]{2,3}=[a-z0-9%]+&)+sig=[a-zA-Z0-9%+\/]{10,}",
            ),
            ("Azure Storage Account Key", r"[a-zA-Z0-9+/]{86}=="),
            (
                "Azure Storage Key",
                r"DefaultEndpointsProtocol=https;AccountName=[^;]+;AccountKey=[A-Za-z0-9+/=]{88}",
            ),
            ("Azure SAS Token", r"sig=[A-Za-z0-9%]+&se=[0-9]+"),
            (
                "Azure AD Client Secret",
                r#"(?i)azure.{0,20}client.{0,20}secret.{0,20}["'][A-Za-z0-9_.\-~]{34,}["']"#,
            ),
            // ============================================================
            // Alibaba / IBM / Oracle
            // ============================================================
            ("Alibaba Access Key ID", r"LTAI[a-zA-Z0-9]{20}"),
            (
                "Alibaba Secret Key",
                r"(?i)(?:alibaba|aliyun).{0,20}(?:secret|key).{0,20}\s*[:=]\s*[a-zA-Z0-9]{30}",
            ),
            (
                "IBM Cloud API Key",
                r"(?i)(?:ibm).{0,20}(?:cloud|api|iam).{0,20}(?:key).{0,20}\s*[:=]\s*[a-zA-Z0-9_\-]{44}",
            ),
            (
                "Oracle Cloud API Key",
                r"(?i)ocid1\.[a-z]+\.[a-z0-9]+\.[a-z0-9]+",
            ),
            // ============================================================
            // GitHub / GitLab / Bitbucket
            // ============================================================
            ("GitHub Token (ghp)", r"ghp_[A-Za-z0-9_]{30,40}"),
            ("GitHub Token (gho)", r"gho_[A-Za-z0-9_]{30,40}"),
            ("GitHub Token (ghu)", r"ghu_[A-Za-z0-9_]{30,40}"),
            ("GitHub Token (ghs)", r"ghs_[A-Za-z0-9_]{30,40}"),
            ("GitHub Token (ghr)", r"ghr_[A-Za-z0-9_]{30,40}"),
            (
                "GitHub Client Secret",
                r#"(?i)github.{0,20}client.{0,20}secret.{0,20}["']?[a-f0-9]{40}["']?"#,
            ),
            ("Google Client Secret", r#"(?i)GOCSPX-[A-Za-z0-9_\-]{28,}"#),
            (
                "Discord Client Secret",
                r#"(?i)discord.{0,20}client.{0,20}secret.{0,20}["']?[A-Za-z0-9_\-]{32}["']?"#,
            ),
            (
                "Microsoft Client Secret",
                r#"(?i)microsoft.{0,20}client.{0,20}secret.{0,20}["']?[A-Za-z0-9_.\-~]{34,}["']?"#,
            ),
            (
                "GitHub App Token",
                r#"(?i)github.{0,20}["'][A-Za-z0-9_]{35,40}["']"#,
            ),
            ("GitLab Token", r"glpat-[A-Za-z0-9\-_]{20,}"),
            ("GitLab Runner Token", r"GR1348941[A-Za-z0-9\-_]{20,}"),
            (
                "Bitbucket Token",
                r#"(?i)bitbucket.{0,20}["'][A-Za-z0-9_]{30,}["']"#,
            ),
            // ============================================================
            // Stripe (ONLY LIVE KEYS)
            // ============================================================
            ("Stripe Live Secret Key", r"sk_live_[0-9a-zA-Z]{24,}"),
            ("Stripe Live Restricted Key", r"rk_live_[0-9a-zA-Z]{24,}"),
            ("Stripe Test Secret Key", r"sk_test_[0-9a-zA-Z]{24,}"),
            ("Stripe Test Restricted Key", r"rk_test_[0-9a-zA-Z]{24,}"),
            ("Stripe Webhook Secret", r"whsec_[0-9a-zA-Z]{24,}"),
            // ============================================================
            // Slack
            // ============================================================
            (
                "Slack Token",
                r"xox[baprs]-[0-9]{10,13}-[0-9]{10,13}[a-zA-Z0-9-]*",
            ),
            (
                "Slack Webhook",
                r"https://hooks\.slack\.com/services/T[A-Z0-9]+/B[A-Z0-9]+/[A-Za-z0-9]+",
            ),
            (
                "Slack Bot Token",
                r"xoxb-[0-9]{11}-[0-9]{11}-[a-zA-Z0-9]{24}",
            ),
            ("Slack Bot Token (Compact)", r"xoxb-[A-Za-z0-9]{24,68}"),
            // ============================================================
            // Discord
            // ============================================================
            ("Discord Token", r"[MN][A-Za-z\d]{23,}\.[\w-]{6}\.[\w-]{27}"),
            (
                "Discord Webhook",
                r"https://discord(?:app)?\.com/api/webhooks/[0-9]+/[A-Za-z0-9_-]+",
            ),
            ("Telegram Bot Token", r"[0-9]{8,10}:[a-zA-Z0-9_-]{35}"),
            // ============================================================
            // Twilio / SendGrid / Mailgun
            // ============================================================
            ("Twilio API Key", r"SK[a-f0-9]{32}"),
            ("Twilio Account SID", r"AC[a-f0-9]{32}"),
            (
                "SendGrid API Key",
                r"SG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}",
            ),
            ("Mailgun API Key", r"key-[0-9a-zA-Z]{28,34}"),
            ("Mailchimp API Key", r"[0-9a-f]{32}-us[0-9]{1,2}"),
            // ============================================================
            // Database / Connection Strings
            // ============================================================
            ("PostgreSQL URL", r"postgres(?:ql)?://[^:]+:[^@]+@[^/]+"),
            ("MySQL URL", r"[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBkdGpHUjYzV3EwWXlEMW9qNlJMZjNlTElNZkVicHFlQ1NkZjBiRDMwOHhFCks4M1lRK3VBL29pMzQxOWpJY2FmdVk2bTNoY251TnVkcEZjeFZ3S1BKb2sKLT4gWDI1NTE5IFpLOFdWVkpIbzFhQ0p4MkhKOUx4SUFnbzEycDdDOXBid3VpUlpnUnEzQm8KTlBTMktqOXVnWmNGdFRGaEJ2ajAxTzR6bnY2NUxmNVIzK3duTlJCb293TQotPiAnUyUtZ3JlYXNlICZXcnhjTSowIE1fN1w1OnU4CndSYUhKM1hXck1JR21YK1YwckZFNGsrbEtDRit0b1d5UmdvbURvUmlPcDQwSDJWM0pRdWdMN3hCK0hKK0dRCi0tLSBZb3lBM3F1VmxOTm9RTEM1WUlpbytOekxqUFJHbVY1cXA1NXBJWFNGODRRCkN0SqGsWjvnWDv9pJrFmu7G82ymwjarlmKefD1kmCwlf7O5rPyt7ctj0rpJ1J3k3xIjpDTfpQ==]/]+"),
            ("MongoDB URL", r"mongodb(?:\+srv)?://[^:]+:[^@]+@[^/]+"),
            ("Redis URL", r"[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBVZlNmSXRHNVdlTjlON1dpYlBqdlA4emNjV3pGOFRITkdvRmFUQ3VJL21FCllhYXFxMmtGY3VkK0dkL1hpVkJVODhrcTlGN0xhUGhLdGpXcTAwWEp2alEKLT4gWDI1NTE5IEZVNDhGQkllTUYwWE02b200U05Zd1BzdVkzYWQza2JHb2d5R1BMSVV6QmMKZDN1dEhOM1Q2NGlPSUZTRGRjc2liZ3M5c1RicTZpanhSMVdQc2M0YTNGawotPiBGaU9eOy1ncmVhc2UKZFQ4Ci0tLSBzM3lVak9BeHBESWR0ZkVaRllwbXVTTE1uRmlPb2d6TUpNcURMWVorK1hzCqk+LyS94oErXO25YbNXUgG1NTEMSkF4kv0CUaq7AGrC9RV66kOFIlk1DD/riqiliqChM3D37g==]/]+"),
            (
                "Database Password",
                r#"(?i)(?:db|database)(?:_)?(?:pass|password|pwd).{0,10}[=:].{0,5}["'][^"']{8,}["']"#,
            ),
            // ============================================================
            // Auth / Tokens / JWT
            // ============================================================
            (
                "JWT Token",
                r"eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}",
            ),
            ("Bearer Token", r"(?i)bearer\s+[A-Za-z0-9_\-\.=]{20,}"),
            ("Basic Auth Header", r"(?i)basic\s+[A-Za-z0-9+/=]{20,}"),
            (
                "OAuth Token",
                r#"(?i)oauth.{0,20}["'][A-Za-z0-9_-]{20,}["']"#,
            ),
            // ============================================================
            // SSH / Private Keys
            // ============================================================
            (
                "RSA Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBuTWlPSDUyRk9NWFhkVFNhWVlTWXRGSVRYOGhhNytHVzB1bis1ZHJURnljCktFQ2ZpejdHcWNSZ3NaRjBxcG1ua1V6WG1MQmR1c3hhUTlvWFRsL29Pam8KLT4gWDI1NTE5IHJYNDE1eUxLeVdiRWorRkJDK2tQRVduZnB4cHVGejJEV1BRS1g5V3ZTUjgKdElMVEwxUC94RWtrVDFEN0ZRTVFtZ211TXZJRHk4WkJCWFNhYnlEMnBVcwotPiAzUEY4PS1ncmVhc2UgRyB3P2RRUTNXNQpDdGZURk90TmRJbjN1bFl6Sjl5VUc5VCtJTFBySkFRUlExYlpYWHNERFBKYThEWFVMSGV3dDFXbmdMeUNha3d3CkpKTGRocWQzaHN1cWh0YjJLMSswRXJDdVQrR3AzRXFwRHdobEdBN0Q4OUQwUjg4Ci0tLSBYRU8rVFJrajdYb3hiQk5CaEZ3SUtNd0xMTFVEWGIxMG13ZHZQY2dyUHkwCvH4wmRHEda6s9klRP/HqUKDBaTQ/83+9hIPl5pNOrVCGl8GB6Zs1zBfn1lRpbLbmNUOaBpZ/DoD9KzNy3VpjGIq+ZQJ6twY3qPEWzo1Z0y8KsJA4wNtyqWG5cUOGqAH]",
            ),
            (
                "DSA Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBwNExLNUJYUWJFQnNYWStaYm5SWXhPcm9HUXFuQXBXNU13QmlVOGxSaVI0CmVYS2hXZ2ZaL1VvT0ttcmpRbS9jVjJFOTl3amdVQ1JaQ3Zka0hDazJ4djAKLT4gWDI1NTE5IGp1RjVIZjZPS0Q0Q3M1OHJzSkM0NTFjZHlWTld3S1dFaW9Zb1ZoTzVYMVEKOC9jYmtuMFZqSS9rVndqTWtpcU5GRUliajkyWVZ6ejVDbWdFaWt2S28rVQotPiBKL3stZ3JlYXNlIF54PzotJyggPX4oX28gZUcKOEM1b1F1cmppTkVUL1ZCMHVHbTY1UDAzR2NLaHl3OUZLUXJuY3FyRWpEODR6UEpYT3p3TkZjTm1TYTVzUUR4UgpicTN0cm9ja0xVTWNRR0x3QktybUtRSitGTU1ZaFhIUlNJZmJwSnNLRU9mK29oN2k4SzdMeVgrNUxnU3J5eENyClV3Ci0tLSBVd0NqTzFNSEYrVmEzdEp2TEVBRjdVWnNUUVNrNWxCZVVZeDc0cEI5R2I0CrqSZufIcgZy5Q23rQsWqcokYmZr9gQHbKKogaKsgfhCL3M676pX5fpj4JpfBSlSrVSjjgVpJzbRmVrIbyNduvZlX28/z4IASErZFKL3oZ7bXu6HRzoNJkIQ6Blp1dSW]",
            ),
            (
                "EC Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA0SEJzWlVuYnlxeUJYUHREQ25mV05zcDhFeTlaZlJuSUhOYVVZWEtDNm4wCjhjSnd1Slhsa0U4c2MyMjRHRU5HZ0RXdFRmbkt6ZGVKN2ZubWZvZmVPSnMKLT4gWDI1NTE5IFpVQ3NpSUNWTWNucXdCb05sdjRWS1Q0M0lqQitqTEEraUVuVHhCTllIMVEKczZhOHlXMWtvZ3ZNTUlVTENPK1c3cWJXamIrWlBvclgvVkVGSCtVOGVQbwotPiBLQmckIUdEPy1ncmVhc2UgOzI6JEsKR2lLSUNwamRZVStsQ1BVYTRsWHUvTWlINHVEKzNjNU5FS092ZGFobEN4bW93dm9KWmxyQkVFd0N0aHpucit6SgppNC9NazRreGxrMDhzY0MzK0c5b0hxYXhPRHd5UEl1TWkxNi9WSExIalIxa0IySm5icjhvRndYWQotLS0gUG81S1Z4WnZyNUYxNWcyVTNrU2ptUlB4YVhGWHJqTEFsYWRia0pVSmhhQQqhJvJ1RoDJP7pRX/ky6WmVauMwghBSZHfyVifM9GveqAk/p+2EtorIJn7uEuQz8XTpW3F2BrfRjGjZpNnBw++bzraDLjOP86D0l4rCmw7XRGjcLNMtn95Lb7pcF2Q=]",
            ),
            (
                "OpenSSH Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBwZnFFeUZiMWVxMEJaZkc2N3BrRUJQQUFtUEJvbzFJVmRFSVI5YkJtbkJ3CmVjcjljMTMweC9FMWhlMG9aSXY3Y0RobHNUUHRwVkFQakpXYnBobWFRdmsKLT4gWDI1NTE5IFlFc0NsSEk3ZU1PSjJEWkxJRVU2TGJtWHRRQVd4OHhqZHBxZmVDS3hGdzQKWmN5NWNpZzN3RWVDTGJCSGc0TVBlRXBheWNWODRBb0hrVWs5Y2RGOW9QbwotPiBCMm80Ond4US1ncmVhc2UgSlwxan0hZFcKRlpSTWxrL1RXeVNKL2YrUGZqeU1WeHJmUEtUUE81aVExUEkvZEx5U2FMdnIrSjhyRjRZNFBEM3lUTkZYU2cKLS0tIHNGbHlxYTNyYWlnVktiQmRJMkF6M01uVVRZL1RZVGVZdU92R3h2UkRya0kKcG3NLQ58p7xg/4kQiSn3knHc9msGhceroanT7zz1ThGLE+/TAaijgTRfyqtH2ova/Fd/M/WrROCXs1kg7p60IsR56HJwRZnuwoh+Yz9ynT1P9Ob6pTxXJs8JwJN+khYj4enlt9YBcg==]",
            ),
            (
                "PGP Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBzNnJrSk4xM0lGUGJRQ0MyTUJoQk1zeUZjZ3BwQ0F4MmR6ajdkYldTQndZCnVzeVd6dFdCNCtPS040a2ZYdXB4QjJXME1ua2h4YjhZSXU5UGtsSVh2clUKLT4gWDI1NTE5IDZjZ05HcmlEaU5CcVB4dnlkWTFkK2NjaXd2bVB4REl2SmlWN1RlYjRPQWcKNUJPdko3TzdsWlZDSkNwL3BES3g4dTBEajhRQm9WOG4xQmhDQTd0SjBUSQotPiBALWdNVkwtZ3JlYXNlIFBBaFQ4MjUgcC8gRloKUDBsR0ZGUlE1OUZxNTFPZ3BQaXZHNUxIL2pSOXF0OFcKLS0tIEk3L0lmeEFoWWhycFBxcjQ2S1dsQko4M3FYMkE0ZEZ0K0dJcC9ScVpoZkEKOb/Ee3GzW5dPAlximRuWDAaBWBV2p/VNXjWnJx8r7vONVLgp8kETG4gOrUDNGcgahCYrUFo5wW7uYsNipqFWOy12L6LuiPFkLMrphCgb0Gquv2gF61720jGwtMmY09c=]",
            ),
            (
                "SSH Private Key (generic)",
                r"(?s)-----BEGIN [A-Z ]+ PRIVATE KEY-----.*?-----END [A-Z ]+ PRIVATE KEY-----",
            ),
            // ============================================================
            // NPM / PyPI / Package Managers
            // ============================================================
            (
                "NPM Token",
                r"//registry\.npmjs\.org/:_authToken=[A-Za-z0-9_-]+",
            ),
            ("NPM Access Token", r"npm_[A-Za-z0-9]{36}"),
            ("PyPI Token", r"pypi-AgEIcHlwaS5vcmc[A-Za-z0-9_-]{50,}"),
            ("NuGet API Key", r"oy2[a-z0-9]{43}"),
            // ============================================================
            // Heroku / Vercel / Netlify
            // ============================================================
            (
                "Heroku API Key",
                r#"(?i)heroku.{0,20}["'][0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}["']"#,
            ),
            (
                "Vercel Token",
                r#"(?i)vercel.{0,20}["'][A-Za-z0-9]{24}["']"#,
            ),
            (
                "Netlify Token",
                r#"(?i)netlify.{0,20}["'][A-Za-z0-9_-]{40,}["']"#,
            ),
            // ============================================================
            // OpenAI / Anthropic / AI APIs
            // ============================================================
            ("OpenAI API Key", r"sk-[a-zA-Z0-9_\-]{20,}"),
            (
                "Cohere API Key",
                r#"(?i)cohere.{0,20}["'][A-Za-z0-9]{40}["']"#,
            ),
            // ============================================================
            // DigitalOcean / Linode / Vultr
            // ============================================================
            ("DigitalOcean Token", r"dop_v1_[a-f0-9]{64}"),
            (
                "DigitalOcean Spaces Key",
                r#"(?i)digitalocean.{0,20}spaces.{0,20}["'][A-Z0-9]{20}["']"#,
            ),
            ("Linode Token", r#"(?i)linode.{0,20}["'][a-f0-9]{64}["']"#),
            // ============================================================
            // Shopify / Square / Payment
            // ============================================================
            ("Shopify Token", r"shpat_[a-fA-F0-9]{32}"),
            ("Shopify Secret", r"shpss_[a-fA-F0-9]{32}"),
            ("Square Access Token", r"sq0atp-[A-Za-z0-9_-]{22}"),
            ("Square OAuth Secret", r"sq0csp-[A-Za-z0-9_-]{43}"),
            (
                "PayPal Client ID",
                r#"(?i)paypal.{0,20}client.{0,20}id.{0,10}["'][A-Za-z0-9_-]{80}["']"#,
            ),
            // ============================================================
            // HashiCorp / Vault
            // ============================================================
            ("HashiCorp Vault Token", r"hvs\.[A-Za-z0-9_-]{24,}"),
            (
                "HashiCorp Terraform Token",
                r#"(?i)terraform.{0,20}["'][A-Za-z0-9]{14}\.[A-Za-z0-9]{24}\.[A-Za-z0-9]{67}["']"#,
            ),
            // ============================================================
            // Age Encryption (Arcane uses this!)
            // ============================================================
            (
                "Age Secret Key",
                r"AGE-SECRET-KEY-1[QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L]{58}",
            ),
            // ============================================================
            // AI / Cloud Provider API Keys
            // ============================================================
            ("NVIDIA API Key", r"nvapi-[A-Za-z0-9_-]{20,}"),
            ("OpenRouter API Key", r"sk-or-v1-[A-Za-z0-9_-]{20,}"),
            ("MiniMax API Key", r"sk-cp-[A-Za-z0-9_-]{20,}"),
            ("Modal API Key", r"modalresearch_[A-Za-z0-9_-]{20,}"),
            ("Resend API Key", r"re_[A-Za-z0-9_-]{20,}"),
            ("Together AI API Key", r"tly_[A-Za-z0-9_-]{20,}"),
            ("Groq API Key", r"gsk_[A-Za-z0-9_-]{20,}"),
            ("DeepSeek API Key", r"sk-[A-Za-z0-9]{20,}"),
            ("Mistral API Key", r"mistral-[A-Za-z0-9_-]{20,}"),
            // Cloudflare R2
            (
                "Cloudflare R2 Account ID",
                r"(?:account[_-]?id|cf[_-]?account[_-]?id).{0,10}[0-9a-f]{32}",
            ),
            (
                "Cloudflare R2 Access Key",
                r"(?:access[_-]?key[_-]?id|cf[_-]?access[_-]?key[_-]?id).{0,10}[0-9a-f]{20}",
            ),
            (
                "Cloudflare R2 Secret Key",
                r"(?:secret[_-]?key|cf[_-]?secret[_-]?key).{0,10}[a-f0-9]{40}",
            ),
            // Backblaze B2
            ("Backblaze B2 Key ID", r"0055[a-f0-9]{16}"),
            ("Backblaze B2 Application Key", r"K005[a-zA-Z0-9]{20,}"),
            // ============================================================
            // Generic High-Entropy / Passwords
            // ============================================================
            (
                "Hex Secret (Quoted)",
                r#"(?i)(?:secret|token|key|password|credential|auth).{0,20}["'][a-fA-F0-9]{32,}["']"#,
            ),
            (
                "High-Entropy Secret (Quoted)",
                r#"(?i)(?:secret|token|key|password|credential|auth).{0,20}["'][A-Za-z0-9]{24,}["']"#,
            ),
            (
                "Generic API Key",
                r#"(?i)(?:api[_-]?key|apikey).{0,10}[=:].{0,5}["'][^\s"\[]{20,}["']"#,
            ),
            (
                "Generic Secret",
                r#"(?i)(?:secret|token|password|passwd|pwd|credential).{0,10}[=:].{0,5}["'][^\s"\[]{16,}["']"#,
            ),
            (
                "Private Token Pattern",
                r#"(?i)private[_-]?(?:key|token).{0,10}[=:].{0,5}["'][A-Za-z0-9_-]{20,}["']"#,
            ),
            // ============================================================
            // Unquoted Assignments (Env Vars / Configs)
            // ============================================================
            // (
            //     "Generic Secret (Unquoted)",
            //     r#"(?i)(?:secret|token|password|passwd|pwd|credential).{0,10}=[^\s"\[]{16,}"#,
            // ),
            (
                "Generic API Key (Unquoted)",
                r#"(?i)(?:api[_-]?key|apikey).{0,10}=[A-Za-z0-9_-]{20,}"#,
            ),
            (
                "Private Key Variable (Unquoted)",
                r#"(?i)[A-Z0-9_]*PRIVATE_KEY[A-Z0-9_]*=[A-Za-z0-9_-]{20,}"#,
            ),
            (
                "Password Variable (Unquoted)",
                r#"(?i)[A-Z0-9_]*PASSWORD[A-Z0-9_]*=[a-zA-Z0-9!$%&*+\-.=?@^_~]{8,}"#,
            ),
            (
                "Generic Assignment (Unquoted)",
                r#"(?i)[A-Z][A-Z0-9_]*(?:KEY|SECRET|TOKEN|PASSWORD|PASSWD|CREDENTIAL|AUTH|ACCESS)[A-Z0-9_]*=[^\s"'`]{20,}"#,
            ),
        ]
    }

    pub fn new() -> Result<Self> {
        let patterns_raw = Self::get_patterns();

        let patterns: Vec<(String, Regex)> = patterns_raw
            .iter()
            .filter_map(|(name, pattern)| {
                // Build the processed pattern exactly as it appears in the
                // combined regex so individual and combined behavior match.
                let p = if pattern.starts_with("(?") {
                    pattern.to_string()
                } else {
                    format!("(?sm){}", pattern)
                };
                Regex::new(&p).ok().map(|re| (name.to_string(), re))
            })
            .collect();

        let combined: String = patterns_raw
            .iter()
            .map(|(_, p)| format!("(?:{})", p))
            .collect::<Vec<_>>()
            .join("|");
        // Use RegexBuilder to cap DFA memory and prevent excessive compilation
        // costs from the large alternation. The regex crate uses finite automata
        // (no catastrophic backtracking), but very large combined regexes can
        // still use prohibitive memory during DFA construction.
        let full_regex = regex::RegexBuilder::new(&format!("(?sm){}", combined))
            .size_limit(10 * (1 << 20)) // 10 MiB total regex memory
            .dfa_size_limit(5 * (1 << 20)) // 5 MiB DFA cache
            .build()
            .map_err(|e| anyhow::anyhow!("invalid regex pattern in SecretScanner::new: {}", e))?;

        Ok(Self {
            patterns,
            full_regex,
        })
    }

    /// Create a scanner that excludes age identity key patterns.
    /// Used for master.age and identity.age files to prevent encrypting
    /// the age key itself while still scanning for other secrets.
    pub fn new_without_age_keys() -> Result<Self> {
        let patterns_raw = Self::get_patterns();

        let patterns: Vec<(String, Regex)> = patterns_raw
            .iter()
            .filter(|(name, _)| *name != "Age Secret Key")
            .filter_map(|(name, pattern)| {
                // Build the processed pattern exactly as it appears in the
                // combined regex so individual and combined behavior match.
                let p = if pattern.starts_with("(?") {
                    pattern.to_string()
                } else {
                    format!("(?sm){}", pattern)
                };
                Regex::new(&p).ok().map(|re| (name.to_string(), re))
            })
            .collect();

        let combined: String = patterns_raw
            .iter()
            .filter(|(name, _)| *name != "Age Secret Key")
            .map(|(_, p)| format!("(?:{})", p))
            .collect::<Vec<_>>()
            .join("|");
        let full_regex = regex::RegexBuilder::new(&format!("(?sm){}", combined))
            .size_limit(10 * (1 << 20))
            .dfa_size_limit(5 * (1 << 20))
            .build()
            .map_err(|e| {
                anyhow::anyhow!(
                    "invalid regex pattern in SecretScanner::new_without_age_keys: {}",
                    e
                )
            })?;

        Ok(Self {
            patterns,
            full_regex,
        })
    }

    pub fn scan(&self, content: &str) -> Vec<SecretFinding> {
        use rayon::prelude::*;

        // Fast-path: Use the optimized single-pass regex to see if ANY secret exists
        if !self.full_regex.is_match(content) {
            return Vec::new();
        }

        let found: Vec<SecretFinding> = self
            .patterns
            .par_iter()
            .flat_map(|(name, re)| {
                let mut results = Vec::new();
                for mat in re.find_iter(content) {
                    let start_idx = mat.start();

                    // SAFEGUARD: Ignore secrets already inside an encrypted tag.
                    // Accepts any marker name that ends with "_SECRET".
                    if is_inside_secret_tag(content, start_idx) {
                        continue;
                    }

                    let line_num = content[..start_idx].chars().filter(|&c| c == '\n').count() + 1;
                    let matching_str = mat.as_str();
                    let snippet = if matching_str.len() > 60 {
                        format!("{}...", &matching_str[..60])
                    } else {
                        matching_str.to_string()
                    };

                    results.push(SecretFinding {
                        name: name.clone(),
                        line: line_num,
                        snippet,
                    });
                }
                results
            })
            .collect();

        // Sort by line number for consistent output
        let mut sorted = found;
        sorted.sort_by_key(|f| f.line);
        sorted
    }
    /// Returns the number of patterns loaded
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Scans content and replaces detected secrets using a callback.
    /// This allows for in-situ transformation (e.g. wrapping in REDACTED_REGEX)
    pub fn scan_and_replace<F>(&self, content: &str, mut f: F) -> String
    where
        F: FnMut(&str, &str) -> String,
    {
        let mut new_result = String::new();
        let mut last_end = 0;

        for mat in self.full_regex.find_iter(content) {
            let matched_str = mat.as_str();

            // 1. SAFEGUARD: Check if we are inside an existing tag
            if is_inside_secret_tag(content, mat.start()) {
                continue;
            }

            // 3. Find which specific pattern matched
            let mut pattern_name = "Unknown";
            for (name, re) in &self.patterns {
                if re.is_match(matched_str) {
                    pattern_name = name;
                    break;
                }
            }

            new_result.push_str(&content[last_end..mat.start()]);
            new_result.push_str(&f(pattern_name, matched_str));
            last_end = mat.end();
        }

        new_result.push_str(&content[last_end..]);
        new_result
    }
}
