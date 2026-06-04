//! Secret scanning patterns and detection.

use regex::Regex;
use anyhow::Result;

use crate::is_inside_secret_tag;

#[derive(Debug)]
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
            ("MySQL URL", r"[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB5RDh4NzBGalZRcXpQb2lZdzhmQmlZUGhYazBTUitSa2xYQTlLVDFDL1NFCjRQNUpsRWNjTkZaejRqVXdDNSsrVENRSkhGdmZSaDdKelFPUDNwcDk1bFkKLT4gWDI1NTE5IEFoemllUFIrZE8zOWlTOWdvT0ZLVGJoS1FHMU80dnltdlgwbzJkVE1RQU0KRFhjYStXUGE3ZjJYaENnNGZIeHZ5WnZPWUlzTWdjdmJWbWx2dDZrY2VzZwotPiAoLWdyZWFzZSA/MQo1bXFYRTJjdXl5T0UrR1EwZmRQNk42d2hzY2lOV1BvTllGMmhOeklCNCt5V2cvR3NOMHNjTTVScHVOcVI2UnpQCkpqckpaVjRMS3ZtYUkrQk05WktodnhtRVdhNGNQd0dNT05vTzNSdVc0c3d6OW1JUUUvMXJ4S2tLR2ZiVnZ5OEQKCi0tLSBxZ0grY2FDb3MwZUJDRTRna2tabDBMdEJCZis1b205NUV5MHdiM21XQzBZCi8KcMFQ8dc7O57lAwYCrLIZRpTapgxTIZ9JTFpPGOj/0KOaUjAyX4DRtkU5UdiPmYfGZyE47A==]/]+"),
            ("MongoDB URL", r"mongodb(?:\+srv)?://[^:]+:[^@]+@[^/]+"),
            ("Redis URL", r"[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBxODJhaXlENEhVRkNGNXpmZVQyZWdSNVArdVR5ZjRuNm9MSWRVVUIzRFFRCnhTMTlHMlVHRFEzSXVsb0FTL0swYXJwVVBHQVhFUU1vZTNwZkV3dlluN3cKLT4gWDI1NTE5IEZpblRCRWZOV2xxZWlSa1VVSHZiQ3ZrbjdGME9VTGV5VlAxNk5QSjBpSG8KOUlxeDRhRDBRNk9GWkhlbUxSMk5SbWtLVDVDTTNVbG1Ba1VWejVxam1ncwotPiBYVioueV88LWdyZWFzZSB8WVRdTSUgX05ycFR9ViBnVCM8CkJGRnRFRmpSRTRzQ0o1emJBZXZVM0tvSnhvQWlqRTNNcytDNXJCb3hLeTh6bnRUSHJUblMxS3FVcWRGdTJONCsKL1pCU0lRVTNHYVUwTzQydDI1Q2JlaWNobmlVdGwreEJXOEFuT2dBMXVTT09oZXZqVzc4Ci0tLSBsNWtBb2hvbWV2elRPYkFFcVlUdjNYajQxRmxCVG43eWF4My84NHFPalZRCn00cTOfWZ6P9RsZAPz+A6F0Z8FSFy9QC0sBtwwvwWT28x8d/Urq+sUQmWOtZTiN2IFyqUiCxQ==]/]+"),
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
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBmLzdLQmRTQVJUUEJOT1lzV1dvM3E4bE94MWlPamhieGJoWTh3Tk8wd0ZJCkNLODVELzlnRlNRbDk4bTRtUXhXUVZwWEs5WUtiSExqODNCbVMvelBDb3MKLT4gWDI1NTE5IEZtWFo4QlhKYjhUWG9yMkJUVk5BTjNndm8vNnN1Q2ZZU0xOZHpNOXBpMUUKbFlHbFhxREpGa2Z1ZGhUQWdsTDRlY2lPVmdnRGV4VUtFOU51dEI0MFgxNAotPiBmZEBKMSlVLWdyZWFzZSAvJWFaCjhHd3lUdDZiVWV2MHFsa1RIM0IvCi0tLSBaNmRGOGZ3eGxZZVBCVDQxVVFsSEhobHZ6K1ZzQzd4SS9US3pyRUNkQ3JJCtsTU8nF9q85S8wb6TIOZTQOHUVy1RhP5lWIqU8UpYu8Vd67exfgXrYUVfBU6wouQCJBJp1N14p+Oq04Z2JgeAkxyWs1KJQfD9tz7GLPe6LQy+vd2NK4F77FJGTF2ecX]",
            ),
            (
                "DSA Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA4S1JjZDN5TVFVOWk0dVN2VU8vTjJEcktvZ0NmaXNSQUhjU3pnVFdvWVdnClhYOEVRN0hMNTBLUnFSa0Flc24xMmw0RHNZeEgycWdvelVncVdNT0lSOFUKLT4gWDI1NTE5IHc3ckdMQlc4YjZwRjY0RVNIeGk3SWZHaTJYM3FJVS9ySUNOOWd3SVFHaUEKV0VxUjR3YXZNZldQSTNIcGZ3VG04ZlljYlFadmg4WmpIUEJGSzBqVmdKVQotPiAzNC1ncmVhc2UgdTheKSkgUApYamRIN2ZPZExDS2pBSlNxOHp1TTVrdGJXTC84Y0w2UUszQlJmUGZINDVCSU9ZV3hmeFlPMjdlRDFCLzd1a3BrCll5YWhFU20zaHJVbWk4VENwL1I0eG82S3R6MHlYd0JxK04wcllwWWo0RGo3RVB1WGh1Ym9oRGRZWEVQUwotLS0gbjJaYm5adzhuT3BFOHdRVUY4d0pBcW9sVWZrVGs0bFhWYzRmNXhVMGFxVQrKvW5tM3yAhmd+LYUxmkq6YQCb/skhak2lFYAN9/loBDHrBFh/n7zX9StSNdkxq4AbIlphzPMavHgyYjZnjoqA7JPGIV41COBRZQeqkcLc37i6UTj0Tf7fL3GbDKEMxw==]",
            ),
            (
                "EC Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB4TUt2em5EMy9XdHE0ZVJwSUprY1loV0ZOcFZwcjB3Wkg2SVVIQzNsK1ZvCml6OGhrakRleW9HNGxzQzd4VHNJYUNGWmR4Tk9FTC9hM0xNeTJWUjZKbTAKLT4gWDI1NTE5IFpWUnlYdGtGVU5wVXNuaFlQelcyTmZtdFNTSU5ITzJBR1pqcEtHbkZEUjgKTGVpZ1lnbkVuRlZWcFBBYU1qVzJuWGdGQU11ZUk2cTh5TnpubG1XL2s4SQotPiAnTVF7YiFqLWdyZWFzZSAlS0plIEBKdG0jIDIoJSlFIDs3UzU3OXwKQUdzR0J0K20KLS0tIFEyWHJwMlQrVVNIOXlCa3B2dWh1Z284aEYva3JwWjNERGlmVW16RkcyRTQKNesu/qKUNJrChLJ6XMccdniYj8W47JPfjYcUHEEDj+6KCkRpnDADD2sSZl+ZQD8mCtFiwBAsJ4N6Wey4xpIYONQo7m6kQHFWeG5man9UGZZxJxXflv5ZRqBm3Cde]",
            ),
            (
                "OpenSSH Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBZT3dzVmdaQUFIYUFwb3lTdjh1TXFNaUNpb2h2dXRpZmRaV25VdTNHeVZVClVkM2ZqZVpKT2NDR0Fra1Z2S1ZDNUNZNjBEV2lBNlJ5eWlCSlFIblM5dFkKLT4gWDI1NTE5IDU4UFY5NnhtM2ZiR2dZRTBFTWFwaDBrS2hTdXhFemNRNDNtUVJoUnl6am8KbVN4NlErNTZyeU5NZElwand0WE1JU3RvNXkyS1hOR1I3d0dDV2dkZ3MzYwotPiBFLWdyZWFzZSBMNnBdeXlYIGsoIDVTVSBECkJieWp4QnYzLytyK0l4ZzFGOW5hVEFPaUV3KzQrYjBJUjVmOGo2ZTVLeFVQNUxDeDlvbWM2VjYwNHh1WFRaSGgKS1hCc0owb2dOb29QS0YvQytSZURLWUhTME5sVGJOZzlRbXBBCi0tLSBkeXNrVmlkTUN2NVduQUFxWkdJNzdrOGprNUUrZ1MxNlhUY05YVkk2YVhFCt2Aed2r9kNN9DOnZhlr2NkbvOsz/9PN4wIBjOg0Zu7TYwujRMK3cK7pVWtd5wehgokoBFoUudCkR0XMZnr7JlYcrcO4i8fYKIa2f5Fw3oCZzOTUXsA974cZvSL7Cvq7fguaACEWhP8=]",
            ),
            (
                "PGP Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBDMkhSSEhncnB3NXIyTUJHekViYmpTSUxwRVowTlQvUnN2M3crWHZSc2prClJ2SnJJRUdxM3FNbmlZMnBWVlZoUjJqYnlwdmFJZUc1dldwbXFnRWpnMWMKLT4gWDI1NTE5IFNsQmNVaWk3M3V2dU9ZUVVOUERGNzRZdTRsemRUOTlXS0lKOTR4a3BySDAKS0doQUlQMkNwQ3lHSUkwVjkwVFlZcWJ1dURzcnNPS0FzWXdiWG1JYUx0QQotPiBoI0M2PH0tZ3JlYXNlIExxMEgyJgoyQ2lVclI0T2kyZ2hpa013UmpYbGwrUTB5aExnVmFJCi0tLSBZSHloa3UvdXdJRjdwdFVCK3l4UVZHMkJXZGNpOGptdFlVNlU3K28wci9BCpZLSNUEK04f2fq8swf7k05/CsxYMKYdCUGWJJaUxI4tkNrBzkyPUg9XsOWFpeqr+RsLIRCMD9eJQIb9KeCaBGQlGUSRdH8K0VOaSxnS2LuJYfExde0+0IxQUnl3pGCi]",
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
        Self::new_with_custom_patterns(&[])
    }

    /// Create a scanner with custom patterns merged with built-in patterns.
    /// Custom patterns are tuples of (name, regex_pattern).
    pub fn new_with_custom_patterns(custom: &[(&str, &str)]) -> Result<Self> {
        let mut patterns_raw = Self::get_patterns();
        for (name, pattern) in custom {
            patterns_raw.push((*name, *pattern));
        }

        let patterns_raw = patterns_raw;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_with_custom_patterns() {
        let custom = vec![("Custom Secret", r"CUSTOM_SECRET_[A-Z0-9]{16}")];
        let scanner = SecretScanner::new_with_custom_patterns(&custom).unwrap();
        
        // Should find custom pattern
        let findings = scanner.scan("CUSTOM_SECRET_ABCDEF1234567890");
        assert!(findings.iter().any(|f| f.name == "Custom Secret"));
        
        // Should also find built-in patterns
        let findings = scanner.scan("[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBRMnRSYVlUU0N4aXRZR3BuNWZwWjc0ZXV1cldpTGJOZGFlYTFZNytTdWwwCmpEdzljNWNOa0JvRkR2NVlCY29JZ1dDY1VSWTlPSXVwM1JhTDE1WkFFYk0KLT4gWDI1NTE5IGZQUy90dVFBbFJSWHdSZ2E5am93T09KM29Uc3pPNTNFZkNCOGZ4dXBaekEKZmFDYlJWSlJuR21Hc3E4VXhyZlBZanpobE9BN081cndLMkpEL3VtcVF0MAotPiBWeC1ncmVhc2UgU1R7PCAvcSwKUmNJbWhNeU5jTDVYMVp4UWlKYldyODlsYS8yN1VlVFBpd2dIWW9Lc2g0aVcraWIvbElZUEpHS0cxeENjVFZsOQo1cHpVK3A5Ti9NU3h0VHJyVXhXbGlsOAotLS0gWUFSa0t3bkRiMXdTc09KdTVnaVgrRURwRFMwa0VKZTlPZ0dZcWlVMjBEYwrZcyp7nbuMdsc+wZJO7PDbo/FrLrmbcu3l12p5qEhlRcrQyfxuyf2SJTI8/if7Jwkw3aKa]");
        assert!(findings.iter().any(|f| f.name == "AWS Access Key ID"));
    }

    #[test]
    fn test_scanner_custom_patterns_empty() {
        let scanner = SecretScanner::new_with_custom_patterns(&[]).unwrap();
        let findings = scanner.scan("no secrets here");
        assert!(findings.is_empty());
    }
}
