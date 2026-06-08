//! Secret scanning patterns and detection.

use anyhow::Result;
use regex::Regex;

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
            ("MySQL URL", r"[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBObEpxMnVaVElWYXJuLytNWlZ3SWRhZFhnUjJHL0RpRlhSRm9SNHRZSFNrCkpCaSs3dkpCMGdjRVBWTm1GeUJ4VjJyMGZrL1FhZkR4VGNiVGVVbFIxNmcKLT4gWDI1NTE5IElOdGNUbDFsY20wVkZZczJ5SVNNdFVqcnI2Q3ZocldJSnJISm9sM0hRd2cKb2J5TnczYzJQZUI5eUZJVG8xaUtGZVd3SW9tSzVaNE0yUFVpelZUSEVVSQotPiBYMjU1MTkgdjROemtKandiTi9tUmUrUGlSdm4zMlVTdnpGSVBMcnMrVEJxbXRsRE9TawphMzdZTUhGMHMvcy9sbFFGUVluNUtnN3Ruci8rRlBFc2JpNEVLWE4zMlBBCi0+IFgyNTUxOSBwVVMrVXNBWlo0b2pLTnQxS2VSUk9nWXBaQ1A0NDZYNmRTNC9mdzVaaERRCkRWQW8vSWJuWDhYSFhCNHM3SFhseGw3czNHcTc4NVQ2VWRZT3hHT29MWHcKLT4gWDI1NTE5IHJsQjlxRkZlVTFkc2xEOWVHV0N2SFZheTVseGZONjd5a3RUN01adjZ0MHMKc2FqVmR3TUpnZU9kTUtLU0tMR2tvd3lWZ09MZlJiNGNLSjJKK2s2bTZqZwotPiBYMjU1MTkgcVNHWnQwTGIzOEsyNDA1bVpyQkxyWnMzM3lKZ1k4NDBydXVPMEs0VEFsOApDdUEvdjBXWmN5czgzZ1I4WFRMNmFobzZlN3grRk5NM0JKdkVDU0RONC9NCi0+IElvIlIoZiYjLWdyZWFzZSBpTkhyT0kKdUtQdnVwVnN2VEs2VXRMR3hMVGxkb2FCL3cyamlDbC8xZ2pneXFuVmh2cFFmajNrS1hjanl5NG5UQjEvS0krUwpGZ1ltdTNGc2xISG5XZUwwTmcKLS0tIDJsSnBVeUd2RFYveTh2djZ4L0R4Zko2dVkyZGlwYWtZV25YL0hyYVNyYzAKV/DFFUldeHcKnZZXu2I0odJHg+RkAKzUkd9V82glVHgbaaPZfuUavfOWL5EaB/VhNUyfpyYw]/]+"),
            ("MongoDB URL", r"mongodb(?:\+srv)?://[^:]+:[^@]+@[^/]+"),
            ("Redis URL", r"[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBBU0p5UnMxTEpwNXd3dkY2bzUrOFpZdUNja0hFVXJmZXA3YnJrQ2U3Y204CkNLbVEzNm1wazU2RnFsckZhbXhjTlVSVlpZc0dkNk5naVYveXFHMG9QNXcKLT4gWDI1NTE5IFFrZ1pVYjN2WVZLVTExUWpKUWkweVlvS1p6VmtiQVU3QmR2WkVWNWgxQkkKb2Y1cllYczlaWUQ5a3ZEWHdnYjBna1NBNTh6T0J1a3VuazdWRks4QklGWQotPiBYMjU1MTkgREpOdDBwRWxjYVQ1UWFMeWgyajJkMUJGMXRYbGVqdUh4RThmd0hYTGtEawpnRkR5YzIrSWpPTkFxWGdiN2gzRWd2N0ZZcE1yK28vb0hjY3VlenJIbEFNCi0+IFgyNTUxOSBHY0ljNXNoSzFzZFFOSksvMWo3S0lGczYyZjVOeW5pQWZhcVR5TlN0VjA4Ck1vczJwazR0MGlicWJ3ajY5N0ZNelpPeXp3ek5PL0JVa2swVkVkaG9Cd0UKLT4gWDI1NTE5IFNEbjkyYUJYWkswbzJhc0NEa3NGVkJUUHBKeUZvQlR6MEdQendCTWIzVE0KNVNCaDdBdmlZVGdnaDA5MkZDTVpQK0txVzZLU1I4TlIvamJaRWtLWDRicwotPiBYMjU1MTkgVEV5dmZPMDNSMXdMdFcwdTg2Rm95VWpmTENrZEVhZkdDN0pDMHM4Q2ZEdwozQmhCS0pCOUk0eWxPVVlaeFRuelNnWS9xdFlSN0NKS0dudTQ1cWRIcmE4Ci0+IG96TzdKei1ncmVhc2UgXCB1bU8KYnFLWmpFR0tCMEN5R0VPdFNlNVhXejB6dXNPZHcxZTRSZmdFSjdXUzBiVTlmZ0pWcVFBNjRMWmFFRXMwM2lsVAp6WWVLMU4yU3QrYi9XaEZMSHNYZUlqbGNTZXhFejdkU1FXbWx0anIzM3dERkc4S2M5NEtLRmcKLS0tIE51ZmJ5dUIyTXhQWU1UMVQ1YU9WTTloMEVPSDI0SlI5VEx4dE9RYkRjeUkK+APw9fR0c400Z42/SX2ZG+GKk5OBjmW0jPFKgXQwERH4zMCjH7Jw71qiVTw1j3qptC05sZLR]/]+"),
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
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBPdmNWSmpsVi90S09HWkRvVzllS1ZtellaL0JrR2l0Y1pFRUtqcVU0QUdFCml2N2VYV1h5dE9WTXVjd1liOEpINHRMRHhRbFgzYldvaGZHQjdmcExndVUKLT4gWDI1NTE5IG5HV0FnN3hKL3BXdjRtM2RzVi9nVURzcTA2d0pnbnBId0RSSG5RWnlrMzQKQm5nV3cycEJmZGZyV3Y4QUNuNzJUZ2Z0YnZzNHlHNTZZdy8rVktMSkhTVQotPiBYMjU1MTkga0dmYlkwQ3pVM3ZKb0praFdPZHMvc2tCQzNUbFpKRW9KNllxa2k1Q3JsMApGQ3BJVExEOEpGWTJKZzZKNWU4SEN4cVljZmduK2ZMTS9Rb1NRS2J6ZTBRCi0+IFgyNTUxOSBJMnMxZWtONjdlNVJGb1JMV0NSZHhONHZPWnBjZmZaNTFnaGJEWmdyNVJFCm0yZGJhcjlySmNJRUJrZnE0SDZZTDdkVXNUeTU1alRiNWtyUjB5TkZSSDAKLT4gWDI1NTE5IGNpUDA3U3MxLzg3ZlAwWlFicGoyK2pUcWhHcldyR0ZZWWZ4Rm56QXltbFUKUkg1WkdoQVYrVHlWQmNlUEdLK2pTV1ozRGtxc3dJaHhqSXVSVzlQaStUWQotPiBYMjU1MTkgMit1MkkveXYwbmdDdDdRUDVOR3g2d1ZnQzRPWlJsQVV1QUVlb2xLNkpCcwo2emVBYkFaRkJVNE80TVFrcVl4d0NEWU4rbjFzMld4b3FIbUJZMi9yYW00Ci0+IGd1bVgtZ3JlYXNlCmJnd3RNMFI4SjVZOEFsbzNYbjlDVkJycGp5Mzc4WjdycGVwOU11NURpb3FEU0pOZktraTBRamlyZGZHYW1YVXEKSzFEZnhiZDVMQ2VpRGlKTVNPeUhxRzcxU3lLTDZpTjgveTFqZU9KTHZnYUM4TTgKLS0tIDJGUFQrVkE3TjRORzVMaWRNSmM4RFhOb001cmJob0hUR3p4cDFsS29qamcKc43jgOa3hE8POG33q9j9rqWT1xQzUii10RQmbg/3slEn0i13ubvD4NXy5e1Aefu7tQ2vdfySWPJu/2/6SYD4hPXLtCznDQOD1WnDLdzicNX0Vnr1ABfhAS+SGu05+yw=]",
            ),
            (
                "DSA Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB1aE4wT0xZS28zaVhpM0VxZXg4QURWbStoTU9ESWVaekxydkd3UGp0OW1RClhYcFVGZ0VUdnhlUWRvQnRBZnhmcE4waFhCMnc0ZGx6QVpLdERzNnkxMG8KLT4gWDI1NTE5IGF2c1dKUzVhUk1LOHVqbXBQSFY1dHZOY1pyRC92aFNWMEFaUndZS2ZoMDgKTDU3ZW8yRVVhSDBTT3B6NTNYNFMvMjJ3WHkxZEdicFhxb2RyRE9nOElWcwotPiBYMjU1MTkgMDZsMy9tVmlqM0lyb0JzOXpmb3BLSXZMQy85azVrU2M2YzRHaEUydFFqcwprQmtvbEY3NVdwUVZZWWxMRTd0KzBNL1grZkJkcklmNDBXVTkvdmxwWnRnCi0+IFgyNTUxOSAxRFRvc29FZzhkRHlKODlhbmhTZDFlZ29PQ2VkZWdtcm9FNWVqUllWYUZjCjk4bElxRlg0Vlc4V1FKU09uRmMwL20wOTFRVkVWYk9XSFhXZ0tsUW0wRTQKLT4gWDI1NTE5IENGUmhwNjQvWXZHbUxSMDlzSHJLNHBqRzJIWGR5NGRKU3VxRFpXdzhDbGMKNVJsdmhvWDR1d01pNEhPZ0NXU24zSUtpRm40L0k4dlBuamNjT1gwajNkTQotPiBYMjU1MTkgKzlPWDBkZ1FlWGNCdHB4VCtKQnJGOXBzcVk0cXBEaEt0SnZNS0oyTEdrUQoyeUY3R3NKVzNSWVFxNUVNdFhwbDJveDFhdVZjeUFwMHdxWEl1Sjk1NWo0Ci0+IGZwLjMsbi1ncmVhc2UKQXpTQ0picWsvTEp6WkRKWFN3dElmOE1SNkp3L3lVQVRUMXkxMnUxeVVyWElNdFhkd3cKLS0tIFk3QTlkTVhhT2pUekJCVlFhMmREbUZsZnJxWkFQcnBGRTkzUzZzRkxXSXcKHrsrTTNgCa2XERNL5ePO5K+eZv+J5z2x8w6zQy52irIzeC016V1Yj7ywKYVdsTSzPUoKq7BUr6TqINIh8EAIl18AKOss/E5zpPp/1J/8qfvX+coa5dtTSIXeALbtHz8=]",
            ),
            (
                "EC Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBJODJEUWtHa2dRRWtlMTNBT0tYRHprRG9yRDRmNjN1cjRud0tjS3RRWWpVCkZ1bXA2OGcwTWtYY2dUTUxrZGdkNU5uNFVOdlJTZ0p3VEd4WkEyQWdwQ28KLT4gWDI1NTE5IGxvM1FHb2pzRmE1aHhiUXZUcnlGTjlETkwyV0cvU0U5NVh4anVWeWJzVEEKQUhEdHlScm9wK0pXMUxDMVhMSXRQT2FBSnBSenZvU0xqTzVManlCbk95VQotPiBYMjU1MTkgUUg4alpzSmhMK0xnQUdQd3BFVk9Dc3ZwK2xkU0EyTzNOKzF1NlBBWVRFawpHZjk0V1p1RjQxYzMyazBhdUF1eXNTRTVMbk1OOGoyR2syRU9QTzl0SUFJCi0+IFgyNTUxOSAvKzJIdTVDa1pVUlF0WkNpaVo1WmJNMk94b1ZEQXBLbXVXbCtNNnZ4ZlVRCmNia29qZmluNTlHelZzYUk3ZjRsMFVyN0FFWis2cDZManFNV3A1elJZRDAKLT4gWDI1NTE5IG4rVUdaSE5NZmZiM2pSMGlFaENJcHFaSEtxcDZwbDhNcHJha1dXdGdHalEKS0lLSzNkUGNRRVRKWjdSRm5raHJlcmgwVGFxa3k2MkhBc1c1QXUzY2tlQQotPiBYMjU1MTkgUkdoZVo4ajFXdlBGby9zZWtWaDgwRVFqemFmRHpGQmNxdHErNjZ0MnVuYwp5enMvSDNJeEZTZDIyWjhTR2FrbnNUS2Y0cGRNczBqYTRDSlVINGtiakgwCi0+IHYrO3wtZ3JlYXNlCklwT0s3ZWFXMHkxZUxCcEVLMlU1a2JCTzQ0Vml6aXVPZUQyNnJGeDFaTUNJa0drOGVMM2ZQc3JEOXZYZExSN3EKbjVBCi0tLSByVmFNUGE3MEx6V2dxQzhsM094ZUt5R2JXbHVPWlNsQU5HWGM1UHdZZkVrCi4g3p7JR45sHHQkEl/m54vHXx9sAVuijwtIyHix4/7GM0EJn3lh+CePWmoTCLARpo+4A9EkXHaG71bpcWAGsd7Cjx+xA6hnICjO+SwFHVj7zKZgiwTevBrkaf88Mg==]",
            ),
            (
                "OpenSSH Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBZbE5OVE9mb1lncWl0bk5WZFcwOHpWU1ZkbVJJeGFqbG9SNnRwYzdNclFjCjN0cC9kUjVHSHRWNnZvRmFYWUVLZ1cxWnM4enFLZEFENjJ4QnhSYVdnbmMKLT4gWDI1NTE5IEZuWWs1OFdaaXBuTzFlSC9oaGZnMUhaU0t5WGdDQ2FWZVcxOSs1aDNrRm8KYUw2Y0psazltUXVMTFFXQnBaaTMxNytGTWN4OFVqVGt6eG05VC8xeURlYwotPiBYMjU1MTkgdXlVcWVrbmswUVB3d1ErN01aVFBtZElub0dqWmNMZDVQeFFEUU0wb0QyVQovcGpFemNLWGZIcncxYUNTeE9WeWVXc2lWckEya1ZwbjJ4clBQbjRUUmVBCi0+IFgyNTUxOSB1cFB4Z3FBTnZBamZCRFdiS3RNMHJ3ZzdYZTNBNkd4UnVDb3RQRWhGNWtjCjg1dzdUa2xGeE14Z2NzQzJEWWlKU1pKOVRYbEhYUEtudUNFck5WVjNqTjAKLT4gWDI1NTE5IDE0OG1wMUdNZ2Zib1I2Zk1pRndha2NMYnpRZlFMTWVVd0JFTEFwTmFjbDgKcVptUEl3VjFZYVJEcDZ5MkJpd28zMC84K0Q2bmQzUjNLWGVlQkxRaXRnRQotPiBYMjU1MTkgdjFPZThZb2ZFSGtSZWVvc0RqZlYzN1NYZHhsUDdYbi9JakxlV0IrSExWawpmZkpWSGVLcDl1d1daVVZhaHg4R3AzSnVrSGpmWDVDblBMZmU0dk13ZXkwCi0+IDQtZ3JlYXNlIFEpeTd7IDY+KCc/WyAsTFU5bF8gaXEnCklZRUdBV2NrclN2a2JoWTlKZUpVR0tVWVNYY3YKLS0tIG5TTm9ZaWtlZzJTaWw2L1NRRy95MC92b1BmeE0zR2t3YUZyMERERDlGcjAKOimCk0BxQwc/2q33BnecngOt04BjxOS7I/5nZFDfh6/HLvCaF581YNbQKVyz4rZwpARzhNOPgB5jNtYNsVzyfhuwU1Vw2BAp9u2luCZdWkcf6Sq2NGptzWjtwoe6VIexe56EQJQ8Hw==]",
            ),
            (
                "PGP Private Key",
                r"(?s)[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBselhMZXZHd3ZQazdXRk5WaFAycGVuam8rbjlsQnhKODJyMW4rL0UvU1h3CmR6eHVTdVNtclJXY1J4S0JIUUZZSHB6TzJqMDZxdHREQkdpQk5kMVJ3YkUKLT4gWDI1NTE5IGpXTGhRaWdJa2Q1LzluaWlSeEdxMFZVMTVnNkN1L0FBMVlFcWVaTVVFUW8KMXVEM1hucUVHUHJLZS85ZzN4aTVraXVVVnRqS2xoaVRhS2pUQ05CWGNGZwotPiBYMjU1MTkgMURWQnF6MnBOK1Q0dWFMcDZtRFV1VVY5bzJHK2tUbTNiNitsZE9vM0ZqdwpxZkx1cEg2eFFGV0dUYko2RnRQcXBlczd2L2NyUXVqWTVtQ1dZZnpBclowCi0+IFgyNTUxOSB6OUEydXphTkh2ckpSdzQ4NFU4aGVXS3NPZFcrWXBYekZoQlpVeTFoTEN3CkNwWVdHQ21kNzRvUkV0VjN3cmxVWGVEcTI4RkEzZnM0ZTB3QzlwUkliTncKLT4gWDI1NTE5IHI2V2NVMld2bjgvNmlXMUxvRG96ZHRFeE90SFpYN0c3SHlwRXZTOVJhMG8KRTFkRXErSGx5amVsWGZvR28reEdwNWdxdFFzZFA2ckkzbWlsSGIwa2dBQQotPiBYMjU1MTkgZnFTNGx5WGQrVWkwSFVTek9FR21VeTdSOURpMXlrQlBkdEl1MTZuNFpHOApDbmNPaUlOd0tWOTk5TDE2R1ZtYmFRbnMzT3dkcm9MSVdzNjNoQVVVeThBCi0+IGVGaHx+LWdyZWFzZQpyRlgrcjlDWnpHYktRdTBxelBBOHphaHRMKzRNeTh5Rk5xTzJKOFdvSE41OHRubVJETmE5ZXRTamV6aDRpaXJqCkNpaitQZwotLS0gc215YU5WZzUwMEMzcGtyRzdjL09ZM1J0aDdidnVYYlRoMlFSaUxqT1EwOApFgESJg6YjjC9+IjHS8AM+ScyQ7Xi/x6XqIGZSu3vzz4Ht9fGzFgMcvnMDu8SmNCd6IBn+ZVC6/DANz5pkVYanqUinPeyc9PoudTzASanxyIvue1qQ59VOKo4QqQRrqg==]",
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

    /// Returns the names of all loaded patterns (for diagnostics).
    pub fn pattern_names(&self) -> Vec<String> {
        self.patterns.iter().map(|(n, _)| n.clone()).collect()
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
        let findings = scanner.scan("[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAvNHdFMXkxdUhHbUU2cjljK3NSeXIrUUdnM2FlVFVtSzhvOGZFMG9GYm1RClZhWW8wenovWEhxSjFBSXQ2bDlaRWU4RiszR1JBZUxQNGJuV1Y4QkpFZWcKLT4gWDI1NTE5IEhCNnNob2NFcFpGUkpWN1ZPYVVCUlVINzlHNGMxWDhabXVNdUlYY2pxd0UKSHZYTmYyK3VQUHhHemk0TEozRDh3anZWbzJUMmNyR09RZHdPc0dzeVNlUQotPiBYMjU1MTkgakRtOFFxOUNyR2xXZ1liYUpscUxzVEh2UWV5NGpremI3V2VSUlROS25VdwppMTJBaEpwVkVLWGJ6MzZUbmFJdWRQdU9GOFozWXhCMC9IeDdkRXFKZnRrCi0+IFgyNTUxOSBMZ2pJNTlmdStIZ1dLRXlxUENwTjZhNUltQ2k4Y0JNbmR3eUFBU2pnMTNRCkhRdENwbFVBajlLUWh1N2U2YStJTmp0ZFJIMU1CSjl6cC81aUxpMTl2cWsKLT4gWDI1NTE5IFpmcEV3Sk5hVkZHOUExWUpGdkZseDhlQ0ZTY1dXOURGS3F6WXRHVkF1VncKUlBJZFJja24yN1pZNUxobWFLR3oxN1hMa3dzbm5ackJ6RmNFeitwSEh6SQotPiBYMjU1MTkganBzT1NNM0hVdmFiTzVBd3lrc1FIQlFkRDFESHJmYUl2K3RjYTBqckFUNApYc2lkSmxXUTdyc0hCQlVSUDNWRDlVYm56RGYxY2FrbStkVXBKRUlHWHJzCi0+ICQtZ3JlYXNlIDYgJ1p9TjFmKiBTKl8tRHIKRVlIeFFDOFZjSXBMS0RGVlJ2aUE3VVRLYkZrSW9LRkE4N3BSSEVJWGFaNmxXcUxaR1FpRUtRZnFZQTlzeUxTcApFUmdieTI3alRVSExPWHVQd3I4Wkh0OTFIRWR3SCtlbng3c2tMTGcKLS0tIGc5c3pUVTJPaWFsb0Rad1Z4L2tEWU0rYjdQZTQzbUtJYUlvNlRhS0VKdGsKTtk6aKWktTL/1za+G0/h18b4VW4dVdGmtgLhW2AybaskcjK/0JI0JOgu3V/VMTjCpjyPGw==]");
        assert!(findings.iter().any(|f| f.name == "AWS Access Key ID"));
    }

    #[test]
    fn test_scanner_custom_patterns_empty() {
        let scanner = SecretScanner::new_with_custom_patterns(&[]).unwrap();
        let findings = scanner.scan("no secrets here");
        assert!(findings.is_empty());
    }
}
