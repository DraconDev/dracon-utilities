use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use age::x25519;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use cfb_mode::cipher::{AsyncStreamCipher, KeyIvInit};

use regex::Regex;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use zeroize::{Zeroize, ZeroizeOnDrop};

// V2 Encryption Constants
const HEADER_V2_MAGIC: &[u8] = b"age-encryption.org/v1";
const DEFAULT_SECRET_MARKER: &str = "DRACON_SECRET";

const ENV_VERSION_HEADER_TEMPLATE: &str = r#"# =============================================================================
# Dracon Warden Encrypted Environment File
# This file is encrypted by dracon-warden for secure team collaboration.
# Version: {}
# DO NOT EDIT THE ENCRYPTED CONTENT MANUALLY - Use `dracon-warden smudge` to decrypt.
# =============================================================================
"#;

fn get_env_version(content: &str) -> u32 {
    if let Some(pos) = content.find("Version: ") {
        let after = &content[pos + 9..];
        if let Some(end) = after.find('\n').or_else(|| after.find('\r')) {
            if let Ok(v) = after[..end].trim().parse::<u32>() {
                return v;
            }
        }
    }
    0
}

fn make_env_version_header(content: &str) -> String {
    let current_version = get_env_version(content);
    let next_version = if current_version == 0 {
        1
    } else {
        current_version + 1
    };
    ENV_VERSION_HEADER_TEMPLATE.replace("{}", &next_version.to_string())
}

fn strip_env_version_header(content: &str) -> &str {
    let header_marker = "Dracon Warden Encrypted Environment File";
    if let Some(start_pos) = content.find(header_marker) {
        let after_header = &content[start_pos..];
        let closing_marker =
            "# =============================================================================";
        if let Some(closing_pos) = after_header.find(closing_marker) {
            let after_closing = &after_header[closing_pos + closing_marker.len()..];
            return after_closing
                .trim_start_matches('\n')
                .trim_start_matches('\r');
        }
    }
    content
}

const REPO_KEY_LEN: usize = 32;

fn normalize_secret_marker(raw: &str) -> Option<String> {
    let marker = raw.trim().to_ascii_uppercase();
    let valid = !marker.is_empty()
        && marker
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        && marker.ends_with("_SECRET");
    if valid {
        Some(marker)
    } else {
        None
    }
}

fn is_inside_secret_tag(content: &str, start_idx: usize) -> bool {
    let prefix = &content[..start_idx];
    if let Some(tag_start) = prefix.rfind('[') {
        if prefix[tag_start..].contains(']') {
            return false;
        }
        let window = &content[tag_start..start_idx];
        return window.contains("_SECRET:");
    }
    false
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegistryCredential {
    pub registry: String, // e.g. "ghcr.io"
    pub username: String,
    pub password: String, // Token or Password
}

impl RegistryCredential {
    pub fn new(registry: &str, username: &str, password: &str) -> Self {
        Self {
            registry: registry.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }
}

#[derive(Clone)]
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
                r"sv=\d{4}-\d{2}-\d{2}&(?:[a-z]{2,3}=(?:[a-z0-9]|%[0-9a-f]{2})+&)+sig=[a-zA-Z0-9%+\/]{10,}",
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
                r"sv=\d{4}-\d{2}-\d{2}&(?:[a-z]{2,3}=(?:[a-z0-9]|%[0-9a-f]{2})+&)+sig=[a-zA-Z0-9%+\/]{10,}",
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
            ("GitHub Token (ghp)", r"ghp_[A-Za-z0-9_]{36}"),
            ("GitHub Token (gho)", r"gho_[A-Za-z0-9_]{36}"),
            ("GitHub Token (ghu)", r"ghu_[A-Za-z0-9_]{36}"),
            ("GitHub Token (ghs)", r"ghs_[A-Za-z0-9_]{36}"),
            ("GitHub Token (ghr)", r"ghr_[A-Za-z0-9_]{36}"),
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
            ("Mailgun API Key", r"key-[0-9a-zA-Z]{32}"),
            ("Mailchimp API Key", r"[0-9a-f]{32}-us[0-9]{1,2}"),
            // ============================================================
            // Database / Connection Strings
            // ============================================================
            ("PostgreSQL URL", r"postgres(?:ql)?://[^:]+:[^@]+@[^/]+"),
            ("MySQL URL", r"mysql://[^:]+:[^@]+@[^/]+"),
            ("MongoDB URL", r"mongodb(?:\+srv)?://[^:]+:[^@]+@[^/]+"),
            ("Redis URL", r"redis://[^:]+:[^@]+@[^/]+"),
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
                r"(?s)-----BEGIN RSA PRIVATE KEY-----.*?-----END RSA PRIVATE KEY-----",
            ),
            (
                "DSA Private Key",
                r"(?s)-----BEGIN DSA PRIVATE KEY-----.*?-----END DSA PRIVATE KEY-----",
            ),
            (
                "EC Private Key",
                r"(?s)-----BEGIN EC PRIVATE KEY-----.*?-----END EC PRIVATE KEY-----",
            ),
            (
                "OpenSSH Private Key",
                r"(?s)-----BEGIN OPENSSH PRIVATE KEY-----.*?-----END OPENSSH PRIVATE KEY-----",
            ),
            (
                "PGP Private Key",
                r"(?s)-----BEGIN PGP PRIVATE KEY-----.*?-----END PGP PRIVATE KEY-----",
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
            ("Mistral API Key", r"[A-Za-z0-9_-]{20,}"),
            // Cloudflare R2
            ("Cloudflare R2 Account ID", r"[0-9a-f]{32}"),
            ("Cloudflare R2 Access Key", r"[0-9a-f]{20}"),
            ("Cloudflare R2 Secret Key", r"[a-f0-9]{40}"),
            // Backblaze B2
            ("Backblaze B2 Key ID", r"0055[a-f0-9]{16}"),
            ("Backblaze B2 Application Key", r"K005[a-zA-Z0-9]{20,}"),
            // ============================================================
            // Generic High-Entropy / Passwords
            // ============================================================
            (
                "Generic API Key",
                r#"(?i)(?:api[_-]?key|apikey).{0,10}[=:].{0,5}["'][^\s"\[]{20,}["']"#,
            ),
            (
                "Generic Secret",
                r#"(?i)(?:secret|token|password|passwd|pwd|credential).{0,10}[=:].{0,5}["'][^\s"\[]{16,}["']"#,
            ),
            (
                "Paranoid Long String (Quoted)",
                r#"["'][A-Za-z0-9+/=_\-]{20,}["']"#, // Catch-all for long strings > 20 chars
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
                r#"(?i)[A-Z_]*[A-Z0-9_]*(?:KEY|SECRET|TOKEN|PASSWORD|PASSWD|CREDENTIAL|AUTH|ACCESS)[A-Z0-9_]*=[^\s"'`]{20,}"#,
            ),
        ]
    }

    pub fn new() -> Self {
        let patterns_raw = Self::get_patterns();

        let patterns: Vec<(String, Regex)> = patterns_raw
            .iter()
            .filter_map(|(name, pattern)| {
                // Ensure individual patterns also support multiline/dotall for the name-matching loop
                let p = if pattern.starts_with("(?") {
                    pattern.to_string()
                } else {
                    format!("(?sm){}", pattern)
                };
                Regex::new(&p).ok().map(|re| (name.to_string(), re))
            })
            .collect();

        // Build one giant regex for single-pass scan
        let combined: String = patterns_raw
            .iter()
            .map(|(_, p)| format!("(?:{})", p))
            .collect::<Vec<_>>()
            .join("|");
        let full_regex = Regex::new(&format!("(?sm){}", combined))
            .expect("Failed to build combined regex - check patterns for invalid regex syntax");

        Self {
            patterns,
            full_regex,
        }
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

/// Structured environment manager for complex secrets
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EnvironmentManager {
    pub variables: std::collections::HashMap<String, String>,
    pub secrets: std::collections::HashMap<String, std::collections::HashMap<String, String>>, // Grouped secrets
}

impl EnvironmentManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    pub fn add_secret(&mut self, group: String, key: String, value: String) {
        self.secrets
            .entry(group)
            .or_insert_with(std::collections::HashMap::new)
            .insert(key, value);
    }

    pub fn to_env_file(&self) -> String {
        let mut out = String::new();
        for (k, v) in &self.variables {
            out.push_str(&format!("{}=\"{}\"\n", k, v.replace('"', "\\\"")));
        }
        for (group, vars) in &self.secrets {
            out.push_str(&format!("# Group: {}\n", group));
            for (k, v) in vars {
                out.push_str(&format!("{}=\"{}\"\n", k, v.replace('"', "\\\"")));
            }
        }
        out
    }

    /// Load variables from a .env file path
    pub fn load_from_env_file(&mut self, path: &std::path::Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(path)?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim().to_string();
                let mut value = v.trim().to_string();
                // Strip quotes if present
                if (value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\''))
                {
                    value = value[1..value.len() - 1].to_string();
                }
                self.add_variable(key, value);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SecretFinding {
    pub name: String,
    pub line: usize,
    pub snippet: String,
}

#[derive(Zeroize, ZeroizeOnDrop, Clone)]
pub struct RepoKey(Vec<u8>);

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct TeamKey(Vec<u8>);

#[derive(Debug, Default, Clone, Copy)]
pub struct MarkerMigrationStats {
    pub files_scanned: usize,
    pub files_changed: usize,
    pub markers_changed: usize,
}

impl RepoKey {
    pub fn from_file(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)?;
        if bytes.len() != REPO_KEY_LEN {
            return Err(anyhow::anyhow!("Invalid key length"));
        }
        Ok(RepoKey(bytes))
    }

    pub fn get_key(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone)]
pub struct DemonSecurity {
    master_identities: Vec<x25519::Identity>,
    imported_identities: Vec<x25519::Identity>,
    managed_patterns: Vec<String>,
    secret_marker: String,
    repo_root: Option<PathBuf>,
    mock_home: Option<PathBuf>,
    pub dev_mode: bool,
}

impl DemonSecurity {
    pub fn master_identities(&self) -> &[x25519::Identity] {
        &self.master_identities
    }

    pub fn with_managed_patterns(mut self, patterns: Vec<String>) -> Self {
        self.managed_patterns = patterns;
        self
    }

    pub fn with_secret_marker(mut self, marker: &str) -> Self {
        if let Some(normalized) = normalize_secret_marker(marker) {
            self.secret_marker = normalized;
        }
        self
    }

    pub fn secret_marker(&self) -> &str {
        &self.secret_marker
    }

    fn supported_secret_markers(&self) -> Vec<String> {
        vec![self.secret_marker.clone()]
    }

    fn secret_tag_prefixes(&self) -> Vec<String> {
        self.supported_secret_markers()
            .into_iter()
            .map(|m| format!("[{}:", m))
            .collect()
    }

    fn contains_any_secret_tag(&self, content: &str) -> bool {
        self.secret_tag_prefixes()
            .iter()
            .any(|prefix| content.contains(prefix))
    }

    fn count_secret_tags(&self, content: &str) -> usize {
        self.secret_tag_prefixes()
            .iter()
            .map(|prefix| content.matches(prefix).count())
            .sum()
    }

    fn starts_with_any_secret_tag(&self, content: &[u8]) -> bool {
        let trimmed = content.strip_suffix(b"\n").unwrap_or(content);
        self.secret_tag_prefixes()
            .iter()
            .any(|prefix| trimmed.starts_with(prefix.as_bytes()) && trimmed.ends_with(b"]"))
    }

    pub fn get_identity_path() -> PathBuf {
        let home = dirs::home_dir().expect("Could not find home directory");
        home.join(".demon").join("identity.age")
    }

    pub fn new(repo_path: Option<&Path>) -> Result<Self> {
        let mut security = Self {
            master_identities: Vec::new(),
            imported_identities: Vec::new(),
            managed_patterns: Vec::new(),
            secret_marker: std::env::var("DRACON_SECRET_MARKER")
                .ok()
                .and_then(|m| normalize_secret_marker(&m))
                .unwrap_or_else(|| DEFAULT_SECRET_MARKER.to_string()),
            repo_root: repo_path.map(|p| p.to_path_buf()),
            mock_home: None,
            dev_mode: false,
        };

        match security.load_master_identities() {
            Ok(ids) => {
                security.master_identities = ids;
            }
            Err(e) => {
                eprintln!("⚠️ Failed to load master identities: {}", e);
            }
        };

        // Load imported legacy keys
        if let Ok(imported) = security.load_imported_identities() {
            if !imported.is_empty() {
                security.imported_identities = imported;
            }
        }

        Ok(security)
    }

    /// Load generic identities from ~/demon/keys/*.age (e.g. Git Seal keys)
    fn load_imported_identities(&self) -> Result<Vec<x25519::Identity>> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let keys_dir = home.join(".demon").join("keys");
        let mut identities = Vec::new();

        if !keys_dir.exists() {
            return Ok(identities);
        }

        for entry in std::fs::read_dir(keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                // Try reading as string (Bech32 Identity)
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(key_str) = content
                        .lines()
                        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
                    {
                        if let Ok(id) = std::str::FromStr::from_str(key_str.trim()) {
                            identities.push(id);
                            continue;
                        }
                    }
                }

                // Legacy Git-Seal/Raw Key loading removed in Phase 48 (Clean Slate)
            }
        }
        Ok(identities)
    }

    /// Load ALL Master Identities from all known paths (Multi-Key Ring)
    /// Returns a vector of identities, prioritized by path.
    pub fn load_master_identities(&self) -> Result<Vec<x25519::Identity>> {
        let home = dirs::home_dir().context("cannot determine home directory")?;
        let mut identities = Vec::new();

        // Path Priority List
        let candidate_paths = vec![
            // 1. Sovereign Master (The active key)
            home.join(".demon").join("master.age"),
            // 2. Standard Identity
            home.join(".demon").join("identity.age"),
            // 3. Fallback/Backups
            home.join(".demon").join("keys").join("identity.age"),
        ];

        // 6. GENERAL SCAN: ~/demon/keys/*.age (and similar dirs)
        // This satisfies "if a user adds their key whatever it is called we can try it"
        let general_keys = vec![
            home.join(".demon").join("keys"), // key storage
        ];

        for dir in general_keys {
            if dir.exists() {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().and_then(|s| s.to_str()) == Some("age") {
                            // Avoid duplicates if we already added it explicitly
                            if !candidate_paths.contains(&p) {
                                // Add to check list (or just parse here)
                                // Let's just parse here to avoid modifying 'paths' which is consumed.
                                if let Ok(c) = fs::read_to_string(&p) {
                                    // Quick parse
                                    if let Some(k) = c
                                        .lines()
                                        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
                                    {
                                        if let Ok(id) = std::str::FromStr::from_str(k.trim()) {
                                            identities.push(id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for path in candidate_paths {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(key_str) = content
                        .lines()
                        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
                    {
                        if let Ok(id) = std::str::FromStr::from_str(key_str.trim()) {
                            identities.push(id);
                        }
                    }
                }
            }
        }

        if identities.is_empty() {
            return Err(anyhow::anyhow!(
                "No valid identities found in any search path."
            ));
        }

        Ok(identities)
    }

    pub fn has_master_identity(&self) -> bool {
        !self.master_identities.is_empty()
    }

    /// Add an identity explicitly from memory (useful for tests or ephemeral agents)
    pub fn add_memory_identity(&mut self, key: x25519::Identity) {
        self.master_identities.push(key);
    }

    /// Set a custom backup root directory (useful for tests)
    pub fn set_mock_home(&mut self, path: PathBuf) {
        self.mock_home = Some(path);
    }

    fn get_home(&self) -> Result<PathBuf> {
        if let Some(ref h) = self.mock_home {
            Ok(h.clone())
        } else {
            dirs::home_dir().context("Could not find home directory")
        }
    }

    /// Explicitly generate and save a new Master Identity
    /// CRITICAL: This should only ever be called ONCE per user.
    /// If an identity already exists, this will refuse to overwrite it.
    pub fn generate_master_identity(&mut self) -> Result<()> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let identity_path = home.join(".demon").join("identity.age");
        // Protection: Check legacy path too
        let legacy_path = home.join(".demon").join("identity.txt");

        // PROTECTION: Scan for ANY existing identity files (backups, corrupted, legacy)
        // We refuse to init if there is ANY trace of an identity to prevent data loss.
        if let Some(parent) = identity_path.parent() {
            if parent.exists() {
                for entry in fs::read_dir(parent)? {
                    let entry = entry?;
                    let path = entry.path();
                    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

                    if file_name.starts_with("identity") {
                        return Err(anyhow::anyhow!(
                            "🛡️ SAFETY TRIGGERED: Found existing identity artifact '{:?}'.\n\n\
                             CRITICAL: Demon refuses to overwrite, modify, or delete Master Identity files.\n\
                             This ensures you can NEVER be locked out of your secrets by an automated process.\n\n\
                             To generate a NEW identity, you must MANUALLY move or delete all 'identity*' files in {:?}.",
                            file_name,
                            parent
                        ));
                    }
                }
            }
        }

        if legacy_path.exists() {
            return Err(anyhow::anyhow!(
                "🛡️ SAFETY TRIGGERED: Legacy identity found at {:?}. Please remove explicitly.",
                legacy_path
            ));
        }

        // Generate new identity
        let key = x25519::Identity::generate();
        if let Some(parent) = identity_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Save Private Identity
        let mut file = fs::File::create(&identity_path)?;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o400); // Read-only for owner
        if let Err(e) = fs::set_permissions(&identity_path, perms) {
            eprintln!(
                "⚠️ failed to set permissions on {}: {}",
                identity_path.display(),
                e
            );
        }
        writeln!(file, "{}", key.to_string().expose_secret())?;

        // Save Public Key for sharing
        let pub_path = home.join(".demon").join("identity.pub");
        fs::write(&pub_path, key.to_public().to_string())?;

        // Auto-Backup Master Identity
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_dir = home.join(".demon").join("backups");
        if let Err(e) = fs::create_dir_all(&backup_dir) {
            eprintln!(
                "⚠️ failed to create backup dir {}: {}",
                backup_dir.display(),
                e
            );
        }
        let backup_path = backup_dir.join(format!("master_{}.age", timestamp));
        if let Ok(mut b_file) = fs::File::create(&backup_path) {
            if let Err(e) = writeln!(b_file, "{}", key.to_string().expose_secret()) {
                eprintln!("⚠️ failed to write backup {}: {}", backup_path.display(), e);
            }
            let mut b_perms = b_file.metadata()?.permissions();
            b_perms.set_mode(0o400); // Private
            if let Err(e) = fs::set_permissions(&backup_path, b_perms) {
                eprintln!(
                    "⚠️ failed to set permissions on {}: {}",
                    backup_path.display(),
                    e
                );
            }
        }

        self.master_identities = vec![key];
        Ok(())
    }

    /// Helper to get the repo root, either from configured path or CWD
    fn get_repo_root(&self) -> Result<PathBuf> {
        if let Some(root) = &self.repo_root {
            return Ok(root.clone());
        }
        Self::find_repo_root()
    }

    /// Find the git repository root from the current directory
    pub fn find_repo_root() -> Result<PathBuf> {
        let mut current = std::env::current_dir()?;
        loop {
            if current.join(".git").exists() {
                return Ok(current);
            }
            if !current.pop() {
                return Err(anyhow::anyhow!("Not in a git repository"));
            }
        }
    }

    /// Load the repo key from .git/arcane/keys/*.age or history
    pub fn load_repo_key(&self) -> Result<RepoKey> {
        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");

        if !keys_dir.exists() {
            return Err(anyhow::anyhow!("No keys found. Run 'demon init'."));
        }

        // 0. Try Machine Key (Env Var) - Priority for CI/CD
        if let Ok(machine_key_str) = std::env::var("ARCANE_MACHINE_KEY") {
            // Derive identity from the env var string
            use std::str::FromStr;
            if let Ok(machine_identity) = x25519::Identity::from_str(&machine_key_str) {
                if let Ok(key) = self.try_decrypt_directory_machine(&keys_dir, &machine_identity) {
                    return Ok(key);
                }
            }
        }

        // 1. Try ALL Master Identities (Key Ring)
        for identity in &self.master_identities {
            // 1a. Try direct User access (keys/*.age)
            if let Ok(key) = self.try_decrypt_directory(&keys_dir, identity) {
                return Ok(key);
            }

            // 1b. Try history keys (latest to oldest)
            let history_dir = keys_dir.join("history");
            if history_dir.exists() {
                if let Ok(entries) = fs::read_dir(&history_dir) {
                    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                    entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
                    for entry in entries {
                        if entry.path().is_dir() {
                            if let Ok(key) = self.try_decrypt_directory(&entry.path(), identity) {
                                return Ok(key);
                            }
                        }
                    }
                }
            }
        }

        // 1c. Try Imported Identities (Heritage Keys / Git Seal)
        for imported_id in &self.imported_identities {
            if let Ok(key) = self.try_decrypt_directory(&keys_dir, imported_id) {
                return Ok(key);
            }
        }

        // 2. Try Team access (keys/team:*.age)
        for entry in fs::read_dir(&keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                if filename.starts_with("team:") && filename.ends_with(".age") {
                    let team_name = filename
                        .trim_start_matches("team:")
                        .trim_end_matches(".age");

                    if let Ok(team_key) = self.load_team_key(team_name) {
                        if let Ok(repo_key) = self.decrypt_repo_key_with_team_key(&path, &team_key)
                        {
                            return Ok(repo_key);
                        }
                    }
                }
            }
        }

        // 4. Last Resort: Legacy repo.key (Removed Phase 48)

        Err(anyhow::anyhow!(
            "Access Denied. Missing valid Key (User, Team, or Machine)."
        ))
    }

    /// Authorize a new recipient (Machine or User) to access this repository
    pub fn authorize_recipient(&self, recipient: &age::x25519::Recipient) -> Result<()> {
        let repo_key = self.load_repo_key()?;
        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");
        std::fs::create_dir_all(&keys_dir)?;

        let output_path = keys_dir.join(format!("{}.age", recipient));

        // Encrypt the repo key for the recipient
        // Fix: Properly collect the recipient into a Box<dyn Recipient> typed Vec
        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient.clone())];
        let encryptor =
            age::Encryptor::with_recipients(recipients).expect("Failed to create encryptor");

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(&repo_key.0)?;
        writer.finish()?;

        std::fs::write(&output_path, encrypted)?;
        Ok(())
    }

    // specialized helper for machine key scanning
    fn try_decrypt_directory_machine(
        &self,
        dir: &Path,
        identity: &x25519::Identity,
    ) -> Result<RepoKey> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                if filename.starts_with("machine:") {
                    if let Ok(repo_key) = self.try_decrypt_key_file(&path, identity) {
                        return Ok(repo_key);
                    }
                }
            }
        }
        Err(anyhow::anyhow!("No matching machine key found"))
    }

    /// Generate a new Machine Identity (Private Key, Public Key)
    pub fn generate_machine_identity() -> (String, String) {
        let identity = x25519::Identity::generate();
        let pub_key = identity.to_public().to_string();
        let identity_str = identity.to_string();
        let priv_key = identity_str.expose_secret();
        (priv_key.to_string(), pub_key)
    }

    /// Authorize a Machine (Public Key) to access this repo
    pub fn whitelist_machine(&self, public_key_str: &str) -> Result<()> {
        let recipient: x25519::Recipient = public_key_str
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid machine public key: {}", e))?;

        let repo_key = self
            .load_repo_key()
            .context("Must have access to repo to whitelist machines")?;

        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");

        // Use hash or similar ID for filename
        let safe_name = public_key_str
            .replace(":", "_")
            .chars()
            .take(12)
            .collect::<String>();
        let machine_file = keys_dir.join(format!("machine:{}.age", safe_name));
        let pub_file = keys_dir.join(format!("machine:{}.pub", safe_name));

        // Save public key (for V2 filtering)
        fs::write(&pub_file, public_key_str)?;

        self.encrypt_and_save_key(&repo_key, &recipient, &machine_file)?;

        Ok(())
    }

    /// Load a Team Key from ~/demon/teams/<name>.key
    pub fn load_team_key(&self, team_name: &str) -> Result<TeamKey> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let team_key_path = home
            .join(".demon")
            .join("teams")
            .join(format!("{}.key", team_name));

        if !team_key_path.exists() {
            return Err(anyhow::anyhow!(
                "Team key '{}' not found in keychain",
                team_name
            ));
        }

        // Team keys are encrypted with Master Identity
        // Team keys are encrypted with Master Identity. Use PRIMARY (first in ring).
        let identity = self
            .master_identities
            .first()
            .context("Master identity required to unlock team keys")?;

        // Decrypt the file
        let encrypted_bytes = fs::read(&team_key_path)?;
        // Fix: Wrap input in Cursor for Decryptor
        let decryptor = age::Decryptor::new(std::io::Cursor::new(&encrypted_bytes))?;

        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        };

        let mut key_bytes = Vec::new();
        use std::io::Read;
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid team key length"));
        }

        Ok(TeamKey(key_bytes))
    }

    fn decrypt_repo_key_with_team_key(&self, path: &Path, team_key: &TeamKey) -> Result<RepoKey> {
        let encrypted_bytes = fs::read(path)?;

        let key_str = std::str::from_utf8(&team_key.0)?;
        use std::str::FromStr;
        let team_identity = x25519::Identity::from_str(key_str)
            .map_err(|_| anyhow::anyhow!("Invalid team identity format"))?;

        // Fix: Wrap input in Cursor for Decryptor
        let decryptor = age::Decryptor::new(std::io::Cursor::new(&encrypted_bytes))?;
        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(&team_identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        };

        let mut key_bytes = Vec::new();
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != REPO_KEY_LEN {
            return Err(anyhow::anyhow!("Invalid decrypted key length"));
        }

        Ok(RepoKey(key_bytes))
    }

    /// Create a new Team (Generates a new Identity, saves to keychain)
    pub fn create_team(&self, team_name: &str) -> Result<()> {
        // Validate name
        if team_name.contains('/') || team_name.contains('\\') || team_name.contains(':') {
            return Err(anyhow::anyhow!("Invalid team name"));
        }

        let home = dirs::home_dir().context("Could not find home directory")?;
        let team_dir = home.join(".demon").join("teams");
        fs::create_dir_all(&team_dir)?;

        let team_key_path = team_dir.join(format!("{}.key", team_name));
        if team_key_path.exists() {
            return Err(anyhow::anyhow!(
                "Team '{}' already exists in your keychain",
                team_name
            ));
        }

        // Generate new Identity for the team
        let team_identity = x25519::Identity::generate();
        let team_identity_string = team_identity.to_string(); // Extend lifetime
        let team_secret = team_identity_string.expose_secret();

        // Encrypt this secret with Master Identity for storage
        let master = self
            .master_identities
            .first()
            .context("Master identity required")?;
        let recipient = master.to_public();

        // Encryption logic
        // Fix: Properly collect the recipient into a Box<dyn Recipient> typed Vec
        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient.clone())];

        let encryptor =
            age::Encryptor::with_recipients(recipients).expect("Failed to create encryptor");

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(team_secret.as_bytes())?;
        writer.finish()?;

        std::fs::write(&team_key_path, encrypted)?;
        Ok(())
    }

    /// Add a new team member by encrypting the repo key for them
    pub fn add_team_member(&self, alias: &str, public_key_str: &str) -> Result<()> {
        let recipient: x25519::Recipient = public_key_str
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid public key: {}", e))?;

        let repo_key = self
            .load_repo_key()
            .context("Must have access to repo to add members")?;

        // Sanitize alias
        let alias = alias.trim();
        if alias.is_empty() || alias.contains('/') || alias.contains('\\') {
            return Err(anyhow::anyhow!("Invalid alias"));
        }

        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");
        let key_path = keys_dir.join(format!("{}.age", alias));
        let pub_key_path = keys_dir.join(format!("{}.pub", alias));

        if key_path.exists() {
            return Err(anyhow::anyhow!("Member '{}' already exists", alias));
        }

        // Save public key (for V2 filtering)
        fs::write(&pub_key_path, public_key_str)?;

        // Save Age key (encrypted for member)
        self.encrypt_and_save_key(&repo_key, &recipient, &key_path)?;

        Ok(())
    }

    /// Create an Invite for a user to join a Team
    pub fn create_team_invite(&self, team_name: &str, user_public_key: &str) -> Result<PathBuf> {
        let team_key = self.load_team_key(team_name)?;

        let recipient: x25519::Recipient = user_public_key
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid user public key: {}", e))?;

        let repo_root = self.get_repo_root()?;

        // Generate a random invite ID
        let invite_id = uuid::Uuid::new_v4().to_string();

        let invites_dir = repo_root.join(".demon").join("invites").join(team_name);
        fs::create_dir_all(&invites_dir)?;

        let invite_path = invites_dir.join(format!("{}.age", invite_id));

        // Encrypt the TEAM KEY for the USER
        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient)];
        let encryptor =
            age::Encryptor::with_recipients(recipients).context("Failed to create encryptor")?;

        let mut file = fs::File::create(&invite_path)?;
        let mut writer = encryptor.wrap_output(&mut file)?;
        writer.write_all(&team_key.0)?;
        writer.finish()?;

        Ok(invite_path)
    }

    /// Accept a Team Invite
    pub fn accept_team_invite(&self, invite_path: &Path) -> Result<String> {
        let identity = self
            .master_identities
            .first()
            .context("Master identity required to accept invite")?;

        let encrypted_bytes = fs::read(invite_path)?;
        let decryptor = age::Decryptor::new(std::io::Cursor::new(&encrypted_bytes))?;

        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        };

        let mut key_bytes = Vec::new();
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid invite content"));
        }

        // Determine team name from path: demon/invites/<team_name>/...
        let team_name = invite_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine team name from path"))?;

        // Save to key chain
        let home = dirs::home_dir().context("Could not find home directory")?;
        let team_dir = home.join(".demon").join("teams");
        fs::create_dir_all(&team_dir)?;

        let team_key_path = team_dir.join(format!("{}.key", team_name));

        // Encrypt for local storage (Master Identity)
        let master = self
            .master_identities
            .first()
            .context("Master identity required to accept invite")?;
        let recipient = master.to_public();

        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient)];
        let encryptor =
            age::Encryptor::with_recipients(recipients).context("Failed to create encryptor")?;

        let mut file = fs::File::create(&team_key_path)?;
        let mut writer = encryptor.wrap_output(&mut file)?;
        writer.write_all(&key_bytes)?;
        writer.finish()?;

        Ok(team_name.to_string())
    }

    /// Revoke a recipient's access to this repo by removing their key files
    pub fn revoke_recipient(&self, public_key_str: &str) -> Result<()> {
        // SAFETY: Refuse to revoke ANY master identity key.
        for id in &self.master_identities {
            if id.to_public().to_string() == public_key_str {
                return Err(anyhow::anyhow!(
                    "🛡️ SAFETY TRIGGERED: Refusing to revoke a Master Identity key.\n\
                     Master identities must be managed MANUALLY via the filesystem to prevent lockout."
                ));
            }
        }

        let repo_root = self.get_repo_root()?;
        let search_paths = vec![
            repo_root.join(".demon").join("data").join("keys"),
            repo_root.join(".git").join("arcane").join("keys"),
        ];

        let mut removed_count = 0;
        for dir in search_paths {
            if dir.exists() {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if let Ok(content) = fs::read_to_string(&path) {
                        if content.contains(public_key_str) {
                            if let Err(e) = fs::remove_file(&path) {
                                eprintln!("⚠️ failed to remove {}: {}", path.display(), e);
                            }

                            let age_path = path.with_extension("age");
                            if age_path.exists() {
                                if let Err(e) = fs::remove_file(&age_path) {
                                    eprintln!("⚠️ failed to remove {}: {}", age_path.display(), e);
                                }
                            }

                            removed_count += 1;
                        }
                    }
                }
            }
        }

        if removed_count > 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("No files found for this recipient"))
        }
    }

    /// List all authorized recipients in the current repository
    pub fn list_authorized_recipients(&self) -> Result<Vec<(String, String)>> {
        let repo_root = self.get_repo_root()?;
        let mut recipients = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let search_paths = vec![
            repo_root.join(".demon").join("data").join("keys"),
            repo_root.join(".git").join("arcane").join("keys"),
        ];

        for dir in search_paths {
            if dir.exists() {
                if let Ok(entries) = fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        if ext == "pub" || ext == "key" {
                            if let Ok(content) = fs::read_to_string(&path) {
                                let name = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                for line in content.lines() {
                                    let line = line.trim();
                                    if !line.is_empty() && !line.starts_with('#') {
                                        if seen.insert(line.to_string()) {
                                            recipients.push((name.clone(), line.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(recipients)
    }

    /// List all team members (aliases)
    pub fn list_team_members(&self) -> Result<Vec<String>> {
        let repo_root = self.get_repo_root()?;
        let keys_dir = repo_root.join(".git").join("arcane").join("keys");

        if !keys_dir.exists() {
            return Ok(Vec::new());
        }

        let mut members = Vec::new();
        for entry in fs::read_dir(keys_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if !name.starts_with("machine:")
                        && !name.starts_with("team:")
                        && name != "repo"
                        && name != "owner"
                    {
                        members.push(name.to_string());
                    }
                }
            }
        }
        Ok(members)
    }

    fn try_decrypt_directory(&self, dir: &Path, identity: &x25519::Identity) -> Result<RepoKey> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("age") {
                if let Ok(repo_key) = self.try_decrypt_key_file(&path, identity) {
                    return Ok(repo_key);
                }
            }
        }
        Err(anyhow::anyhow!(
            "No matching key found in directory for this identity"
        ))
    }

    fn try_decrypt_key_file(&self, path: &Path, identity: &x25519::Identity) -> Result<RepoKey> {
        let encrypted_bytes = fs::read(path)?;
        // Fix: Wrap input in Cursor for Decryptor
        let decryptor = age::Decryptor::new(std::io::Cursor::new(&encrypted_bytes))?;

        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        };
        let mut key_bytes = Vec::new();
        reader.read_to_end(&mut key_bytes)?;

        if key_bytes.len() != REPO_KEY_LEN {
            return Err(anyhow::anyhow!("Invalid decrypted key length"));
        }
        Ok(RepoKey(key_bytes))
    }

    // Encrypt Repo Key for a Recipient and save to file
    fn encrypt_and_save_key(
        &self,
        repo_key: &RepoKey,
        recipient: &x25519::Recipient,
        output_path: &Path,
    ) -> Result<()> {
        // Fix: Properly collect the recipient into a Box<dyn Recipient> typed Vec
        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient.clone())];

        let encryptor =
            age::Encryptor::with_recipients(recipients).expect("Failed to create encryptor");

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(&repo_key.0)?;
        writer.finish()?;

        std::fs::write(output_path, encrypted)?;
        Ok(())
    }

    fn backup_secret(&self, original_path: &str, content: &[u8]) -> Result<()> {
        if original_path.contains("demon/backups") || original_path.contains("arcane/backups") {
            return Ok(()); // Silent skip for internal Git/Arcane backups
        }
        let repo_root = self.get_repo_root()?;
        let backup_dir = repo_root.join(".git").join("arcane").join("backups");
        fs::create_dir_all(&backup_dir)?;

        let safe_name = original_path.replace("/", "_").replace("\\", "_");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let backup_path = backup_dir.join(format!("{}.{}.bak.age", safe_name, timestamp));

        let identity = self
            .master_identities
            .first()
            .context("Master identity required for secure backup")?;
        let recipient = identity.to_public();

        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient)];
        let encryptor = age::Encryptor::with_recipients(recipients)
            .context("Failed to create encryptor for backup")?;

        let mut file = fs::File::create(&backup_path)?;
        let mut writer = encryptor.wrap_output(&mut file)?;
        writer.write_all(content)?;
        writer.finish()?;

        Ok(())
    }

    // ============================================================
    // V2: DIRECT RECIPIENT ENCRYPTION (Standard Age)
    // ============================================================

    /// V2 Encryption: Encrypt directly to a list of recipients (No RepoKey)
    pub fn encrypt_v2(
        &self,
        data: &[u8],
        recipients: Vec<Box<dyn age::Recipient + Send>>,
    ) -> Result<Vec<u8>> {
        let encryptor =
            age::Encryptor::with_recipients(recipients).context("Failed to create encryptor")?;

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(data)?;
        writer.finish()?;

        Ok(encrypted)
    }

    /// V2 Decryption: Decrypt using the User's Identities (Try ALL known keys)
    /// V2 Decryption: Decrypt using the User's Identities (Try ALL known keys)
    pub fn decrypt_v2(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        if self.master_identities.is_empty() {
            return Err(anyhow::anyhow!(
                "Master identity required for V2 decryption"
            ));
        }

        let decryptor = age::Decryptor::new(std::io::Cursor::new(encrypted_data))?;

        match decryptor {
            age::Decryptor::Recipients(d) => {
                // Pass ALL identities to the decryptor at once.
                // Age will try them one by one.
                let identities: Vec<&dyn age::Identity> = self
                    .master_identities
                    .iter()
                    .map(|id| id as &dyn age::Identity)
                    .collect();

                let mut reader = d
                    .decrypt(identities.into_iter())
                    .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

                let mut plaintext = Vec::new();
                reader.read_to_end(&mut plaintext)?;
                Ok(plaintext)
            }
            age::Decryptor::Passphrase(_) => {
                Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        }
    }

    /// Gather all known recipients (Master, Imported, Team, Machine) for encryption.
    pub fn gather_all_recipients(&self) -> Result<Vec<x25519::Recipient>> {
        let master = self
            .master_identities
            .first()
            .context("Master identity required to gathering recipients")?;
        let mut seen_keys = std::collections::HashSet::new();
        let mut recipients = Vec::new();

        // 1. Self (Master)
        let master_pub = master.to_public();
        seen_keys.insert(master_pub.to_string());
        recipients.push(master_pub);

        // 2. Imported Heritage Identities
        for id in &self.imported_identities {
            let pub_key = id.to_public();
            let pub_str = pub_key.to_string();
            if seen_keys.insert(pub_str) {
                recipients.push(pub_key);
            }
        }

        // 3. Authorized Machine & Team Keys from the current repo
        if let Ok(repo_root) = self.get_repo_root() {
            // Check BOTH new committed path (V2 Standard) and legacy path
            let search_paths = vec![
                repo_root.join(".demon").join("data").join("keys"), // V2 Standard
                repo_root.join(".git").join("arcane").join("keys"), // Legacy
            ];

            for keys_dir in search_paths {
                if keys_dir.exists() {
                    if let Ok(entries) = fs::read_dir(keys_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            // Accept .pub files (standard) and .key files (legacy pubkeys sometimes named .key)
                            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                            if ext == "pub" || ext == "key" {
                                if let Ok(pub_str) = fs::read_to_string(&path) {
                                    // Parse potential multiple keys per file or single key
                                    for line in pub_str.lines() {
                                        let line = line.trim();
                                        if !line.is_empty() && !line.starts_with('#') {
                                            if seen_keys.insert(line.to_string()) {
                                                if let Ok(recipient) =
                                                    line.parse::<x25519::Recipient>()
                                                {
                                                    recipients.push(recipient);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(recipients)
    }

    /// Unified payload unlocking logic: try ALL known keys.
    pub fn unlock_payload(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        // 1. Try Age (V2) format
        if encrypted_data.starts_with(HEADER_V2_MAGIC) {
            // Try ALL Master keys
            for id in &self.master_identities {
                if let Ok(plaintext) = self.decrypt_v2_with_identity(encrypted_data, id) {
                    return Ok(plaintext);
                }
            }
            // Try Imported
            for id in &self.imported_identities {
                if let Ok(plaintext) = self.decrypt_v2_with_identity(encrypted_data, id) {
                    return Ok(plaintext);
                }
            }
        }

        // 2. Try RepoKey (V1) format - if we have a RepoKey
        if let Ok(repo_key) = self.load_repo_key() {
            if let Ok(plaintext) = self.decrypt_with_repo_key(&repo_key, encrypted_data) {
                return Ok(plaintext);
            }
            if let Ok(plaintext) = self.decrypt_git_seal(&repo_key, encrypted_data) {
                return Ok(plaintext);
            }
        }

        // 3. Drunk guy with keychain (brute force all keys in ~/demon/keys)
        if let Some(plaintext) = self.try_keychain_bruteforce(encrypted_data) {
            return Ok(plaintext);
        }

        Err(anyhow::anyhow!(
            "Decryption failed after trying all keys (V2 + V1 + Keychain). Magic: {:?}, Len: {}",
            &encrypted_data.get(0..20).unwrap_or(&[]),
            encrypted_data.len()
        ))
    }

    fn decrypt_v2_with_identity(
        &self,
        encrypted_data: &[u8],
        identity: &x25519::Identity,
    ) -> Result<Vec<u8>> {
        let decryptor = age::Decryptor::new(std::io::Cursor::new(encrypted_data))?;
        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase not supported"))
            }
        };
        let mut plaintext = Vec::new();
        reader.read_to_end(&mut plaintext)?;
        Ok(plaintext)
    }

    /// Helper for encrypting data to all known recipients.
    pub fn encrypt_v2_for_all(&self, data: &[u8]) -> Result<Vec<u8>> {
        let recipients = self.gather_all_recipients()?;
        let age_recipients: Vec<Box<dyn age::Recipient + Send>> = recipients
            .into_iter()
            .map(|r| Box::new(r) as Box<dyn age::Recipient + Send>)
            .collect();
        self.encrypt_v2(data, age_recipients)
    }

    /// Encrypt data for a specific node (runner) + master keys
    pub fn encrypt_for_node(&self, data: &[u8], node_recipient: &str) -> Result<Vec<u8>> {
        let mut recipients = Vec::new();

        // 1. Add node recipient
        let node: x25519::Recipient = node_recipient
            .parse::<x25519::Recipient>()
            .map_err(|e| anyhow::anyhow!("Invalid node recipient: {}", e))?;
        recipients.push(node);

        // 2. Add master keys (so Director/User can still recover/debug)
        let master_ids = self.load_master_identities()?;
        for id in master_ids {
            recipients.push(id.to_public());
        }

        let age_recipients: Vec<Box<dyn age::Recipient + Send>> = recipients
            .into_iter()
            .map(|r| Box::new(r) as Box<dyn age::Recipient + Send>)
            .collect();

        self.encrypt_v2(data, age_recipients)
    }

    /// Ensure the current user's public key is present in the repo keys.
    /// This prevents "lockout" by ensuring we can always decrypt what we encrypt.
    pub fn ensure_current_user_key(&self) -> Result<()> {
        let repo_root = match self.get_repo_root() {
            Ok(r) => r,
            Err(_) => return Ok(()), // Not in a repo, skip
        };

        let identity = self
            .master_identities
            .first()
            .context("No master identity found")?;
        let pub_key = identity.to_public();
        let pub_key_str = pub_key.to_string();

        // Use a short hash of the key for the filename to allow multiple owners
        // without conflict or overwriting.
        let safe_id = pub_key_str.chars().take(8).collect::<String>();
        let filename = format!("owner_{}.pub", safe_id);

        let keys_dir = repo_root.join(".demon").join("data").join("keys");
        fs::create_dir_all(&keys_dir)?;

        let key_path = keys_dir.join(&filename);
        if !key_path.exists() {
            // also check if we are already in there under a different name?
            // actually, writing it again is cheap and safe.
            fs::write(&key_path, &pub_key_str)?;
            // eprintln!("🔑 Auto-added public key to repo: {}", filename);
        }
        Ok(())
    }

    /// Create a secure backup of a file before modification.
    /// The backup is encrypted with all known keys and stored in ~/demon/backups/
    /// Returns the path to the backup file.
    pub fn backup_file(&self, file_path: &Path, content: &[u8]) -> Result<PathBuf> {
        let path_str = file_path.to_string_lossy();
        if path_str.contains("demon/backups") || path_str.contains("arcane/backups") {
            return Err(anyhow::anyhow!(
                "Recursion guard: Skipping backup of backup file"
            ));
        }

        // Auto-ensure our key is in the repo before we do anything that might rely on it later

        let home = self.get_home()?;
        let backup_dir = home.join(".demon").join("backups");
        fs::create_dir_all(&backup_dir)?;

        // Hash the path to create a deterministic but safe filename
        let mut hasher = Sha256::new();
        hasher.update(file_path.to_string_lossy().as_bytes());
        let hash = hasher.finalize();
        let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

        // Timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Filename: <hash>_<timestamp>.age
        let filename = format!("{}_{}.age", hash_hex, timestamp);
        let backup_path = backup_dir.join(filename);

        // Encrypt logic (using encrypt_v2_for_all)
        let encrypted = self.encrypt_v2_for_all(content)?;

        fs::write(&backup_path, encrypted)?;
        Ok(backup_path)
    }

    /// Restore a file from the latest secure backup.
    /// Finds the backup matching the file path hash and decrypts it to the target path.
    pub fn restore_file(&self, file_path: &Path) -> Result<PathBuf> {
        let home = self.get_home()?;
        let backup_dir = home.join(".demon").join("backups");

        if !backup_dir.exists() {
            return Err(anyhow::anyhow!(
                "No backups found for file: {:?}",
                file_path
            ));
        }

        // Hash the path to find matching backups
        let mut hasher = Sha256::new();
        hasher.update(file_path.to_string_lossy().as_bytes());
        let hash = hasher.finalize();
        let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

        // Find matching backup files
        let mut backups = Vec::new();
        if backup_dir.exists() {
            for entry in fs::read_dir(&backup_dir)? {
                let entry = entry?;
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Check if file starts with the hash
                    if name.starts_with(&hash_hex) && name.ends_with(".age") {
                        backups.push(path);
                    }
                }
            }
        }

        if backups.is_empty() {
            return Err(anyhow::anyhow!(
                "No backups found for file: {:?}",
                file_path
            ));
        }

        // Sort to get the latest
        backups.sort();
        let latest_backup = backups.last().expect("List should not be empty");

        // Decrypt
        let encrypted_content = fs::read(latest_backup)?;
        let decrypted_content = self.unlock_payload(&encrypted_content)?;

        // Write back
        fs::write(file_path, decrypted_content)?;

        Ok(latest_backup.clone())
    }

    /// List all available backups for a given file path, sorted by timestamp (newest first).
    pub fn list_backups(&self, file_path: &Path) -> Result<Vec<PathBuf>> {
        let home = self.get_home()?;
        let backup_dir = home.join(".demon").join("backups");

        if !backup_dir.exists() {
            return Ok(Vec::new());
        }

        // Hash the path to find matching backups
        let mut hasher = Sha256::new();
        hasher.update(file_path.to_string_lossy().as_bytes());
        let hash = hasher.finalize();
        let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

        // Find matching backup files
        let mut backups = Vec::new();
        for entry in fs::read_dir(&backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Check if file starts with the hash
                if name.starts_with(&hash_hex) && name.ends_with(".age") {
                    backups.push(path);
                }
            }
        }

        // Sort reverse (newest first)
        backups.sort_by(|a, b| b.cmp(a));

        Ok(backups)
    }

    /// In-situ Clean: Scan for secrets and replace with REDACTED_REGEX tags.
    pub fn smart_clean(&self, content: &str) -> Result<String> {
        let scanner = SecretScanner::new();
        self.smart_clean_with_scanner(content, &scanner)
    }

    /// In-situ Clean with a specific scanner (allows filtering patterns)
    fn smart_clean_with_scanner(&self, content: &str, scanner: &SecretScanner) -> Result<String> {
        let mut had_error = false;
        let mut last_err: String = String::new();
        let cleaned = scanner.scan_and_replace(content, |_, secret| {
            match self.encrypt_v2_for_all(secret.as_bytes()) {
                Ok(encrypted) => {
                    let b64 = general_purpose::STANDARD.encode(encrypted);
                    format!("[{}:{}]", self.secret_marker, b64)
                }
                Err(e) => {
                    last_err = e.to_string();
                    had_error = true;
                    secret.to_string()
                }
            }
        });
        if had_error {
            Err(anyhow::anyhow!("smart_clean: encryption failed for one or more secrets: {}", last_err))
        } else {
            Ok(cleaned)
        }
    }

    /// Smart Clean with Path Context:
    /// If the path is in a sensitive directory (e.g. .ssh, .aws) OR force_encrypt is true, encrypt the ENTIRE file.
    /// Otherwise, use regex-based in-situ encryption (if text).
    pub fn smart_clean_with_path(&self, content: &[u8], path_str: &str) -> Result<Vec<u8>> {
        // 1. Definition of Sensitive Paths (Still used for binary detection)
        let sensitive_dirs = [
            ".ssh",
            "demon/keys",
            "demon/secrets",
            ".aws",
            ".kube",
            ".gnupg",
            ".azure",
            ".config/gcloud",
        ];

        let sensitive_exts = [
            ".age", ".key", ".p12", ".pfx", ".pem", ".crt", ".der", ".asc", ".zip", ".tar", ".gz",
            ".bz2", ".7z", ".rar", ".tgz", ".xz", ".tar.gz", ".tar.bz2", ".tar.xz", ".sqlite",
            ".sqlite3", ".db", ".vmdk", ".img", ".qcow2", ".vdi", ".iso", ".docker", ".oci",
            ".xlsx", ".csv", ".ods", ".kdbx", ".1pif", ".sql", ".apk", ".aab", ".dmg", ".pcap",
            ".pcapng", ".ovpn", ".tfstate", ".tfplan", ".tfvars",
        ];

        let sensitive_filenames = [
            "id_rsa",
            "id_ed25519",
            "id_ecdsa",
            "id_dsa",
            "id_xmss",
            "master.age",
            "identity.age",
            "owner.age",
            "demon-key",
            "id_rsa.pub",
            "id_ed25519.pub",
            "credentials",
            ".bash_history",
            ".zsh_history",
            ".sh_history",
            "core",
            "known_hosts",
            "vault.yml",
            ".terraform.lock.hcl",
            "terraform.tfvars",
            ".env",
            ".env.local",
            ".env.production",
            ".env.development",
            ".env.staging",
            ".npmrc",
            ".pypirc",
            "netrc",
            ".pgpass",
            ".my.cnf",
        ];

        let filename = std::path::Path::new(path_str)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let is_sensitive_location = sensitive_dirs.iter().any(|dir| path_str.contains(dir))
            || sensitive_exts.iter().any(|ext| path_str.ends_with(ext))
            || sensitive_filenames.contains(&filename)
            || sensitive_filenames.iter().any(|p| filename.starts_with(p))
            || self
                .managed_patterns
                .iter()
                .any(|p| filename == p || path_str.contains(p));

        // 2. Process based on content type
        match std::str::from_utf8(content) {
            Ok(text_content) => {
                // Full encryption for .env files and other sensitive files that shouldn't leak structure
                let is_full_encrypt = is_env_file(filename)
                    || (is_sensitive_location
                        && (filename == "credentials"
                            || filename.starts_with(".bash_history")
                            || filename.starts_with(".zsh_history")
                            || filename.starts_with(".sh_history")
                            || filename == "vault.yml"));
                if is_full_encrypt {
                    // Don't double-encrypt
                    if content.starts_with(HEADER_V2_MAGIC)
                        || self.starts_with_any_secret_tag(content)
                    {
                        return Ok(content.to_vec());
                    }
                    // Add/increment version header for .env files to track changes
                    let content_to_encrypt = if is_env_file(filename) {
                        // Check if this is already a warden-managed file by looking for our marker
                        if text_content.contains("Dracon Warden") {
                            // Remove old header and add new one with incremented version
                            let stripped = strip_env_version_header(text_content);
                            format!(
                                "{}\n{}",
                                make_env_version_header(text_content),
                                stripped.trim()
                            )
                        } else {
                            // First time encryption - add v1 header
                            format!(
                                "{}\n{}",
                                make_env_version_header(text_content),
                                text_content
                            )
                        }
                    } else {
                        text_content.to_string()
                    };
                    return self.encrypt_v2_to_b64_tag(content_to_encrypt.as_bytes());
                }
                // Eager encryption: scan for and encrypt all detected secrets.
                // This means we may encrypt non-secret content, but we won't miss secrets.
                let cleaned = self.smart_clean(text_content)?;
                Ok(cleaned.into_bytes())
            }
            Err(_) => {
                // Binary Data: Only encrypt if it is in a sensitive location
                if is_sensitive_location {
                    // Don't double-encrypt
                    if content.starts_with(HEADER_V2_MAGIC)
                        || self.starts_with_any_secret_tag(content)
                    {
                        return Ok(content.to_vec());
                    }
                    self.encrypt_v2_to_b64_tag(content)
                } else {
                    // Normal binary path -> Passthrough (preserves images, etc)
                    Ok(content.to_vec())
                }
            }
        }
    }

    fn encrypt_v2_to_b64_tag(&self, content: &[u8]) -> Result<Vec<u8>> {
        match self.encrypt_v2_for_all(content) {
            Ok(encrypted) => {
                let b64 = general_purpose::STANDARD.encode(encrypted);
                Ok(format!("[{}:{}]", self.secret_marker, b64).into_bytes())
            }
            Err(e) => Err(anyhow::anyhow!("Failed to encrypt sensitive file: {}", e)),
        }
    }

    /// In-situ Smudge: Decrypt REDACTED_REGEX tags back to plaintext.
    pub fn smart_smudge(&self, content: &str) -> Result<String> {
        let markers = self.secret_tag_prefixes();
        let mut result = String::new();
        let mut last_end = 0;

        while last_end < content.len() {
            let mut next: Option<(usize, usize)> = None;
            for marker in &markers {
                if let Some(start_idx) = content[last_end..].find(marker) {
                    let absolute_start = last_end + start_idx;
                    let marker_len = marker.len();
                    if next
                        .map(|(best_idx, _)| absolute_start < best_idx)
                        .unwrap_or(true)
                    {
                        next = Some((absolute_start, marker_len));
                    }
                }
            }

            let Some((absolute_start, marker_len)) = next else {
                break;
            };

            result.push_str(&content[last_end..absolute_start]);

            // Find closing bracket
            if let Some(end_offset) = content[absolute_start..].find(']') {
                let absolute_end = absolute_start + end_offset + 1;
                let b64 = &content[absolute_start + marker_len..absolute_end - 1];

                match general_purpose::STANDARD.decode(b64.trim()) {
                    Ok(encrypted) => match self.unlock_payload(&encrypted) {
                        Ok(plaintext) => {
                            result.push_str(&String::from_utf8_lossy(&plaintext));
                        }
                        Err(_) => result.push_str(&content[absolute_start..absolute_end]),
                    },
                    Err(_) => result.push_str(&content[absolute_start..absolute_end]),
                }
                last_end = absolute_end;
            } else {
                // No closing bracket found, treat as normal text
                result.push_str(&content[absolute_start..]);
                last_end = content.len();
            }
        }

        result.push_str(&content[last_end..]);
        Ok(result)
    }

    /// Git Clean Filter: Encrypt stdin -> stdout
    /// V2 Upgrade: Encrypts to ALL known public keys (User + Machines + Teams)
    pub fn seal_clean(&self, file_path: Option<&str>) -> Result<()> {
        use std::io::{Read, Write};

        // 1. Read plaintext from stdin
        let mut buffer = Vec::new();
        std::io::stdin().read_to_end(&mut buffer)?;

        // Auto-add key to avoid lockout (Ensure keys folder exists)
        if let Err(e) = self.ensure_current_user_key() {
            eprintln!("⚠️ failed to ensure user key: {}", e);
        }

        // 3. Backup (Safety Net) - must happen before buffer is potentially moved
        if let Some(path) = file_path {
            if path.contains(".env") {
                if let Err(e) = self.backup_secret(path, &buffer) {
                    eprintln!("⚠️ failed to backup .env file: {}", e);
                }
            }
        }

        // 4. Smart Clean: Targeted encryption only to preserve Git diffs.
        // Every file (UTF-8) is scanned for secrets.
        // Binary files are passed through untouched to preserve Git diffs.
        let output = if let Ok(text_content) = std::str::from_utf8(&buffer) {
            self.smart_clean(text_content)?.into_bytes()
        } else {
            buffer
        };

        // 5. Write to stdout
        std::io::stdout().write_all(&output)?;

        Ok(())
    }

    /// Recursive disk-wide decryption: Replaces all [*_SECRET:...] tags with plaintext in-place.
    pub fn decrypt_path(&self, root: &Path, recursive: bool, dry_run: bool) -> Result<usize> {
        let mut total_restored = 0;

        if !root.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {:?}", root));
        }

        if root.is_file() {
            return self.decrypt_file(root, dry_run);
        }

        // Use WalkDir for efficient recursion
        let walker = walkdir::WalkDir::new(root)
            .max_depth(if recursive { usize::MAX } else { 1 })
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                // Allow the root even if it starts with a dot
                if e.path() == root {
                    return true;
                }
                // Skip common noise and git internals unless explicit
                !name.starts_with('.') || name == ".env"
            });

        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(count) = self.decrypt_file(entry.path(), dry_run) {
                    total_restored += count;
                }
            }
        }

        Ok(total_restored)
    }

    fn decrypt_file(&self, path: &Path, dry_run: bool) -> Result<usize> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Ok(0),
        };

        if !self.contains_any_secret_tag(&content) {
            return Ok(0);
        }

        let smudged = self.smart_smudge(&content)?;
        if smudged == content {
            return Ok(0);
        }

        // Count how many tags were replaced
        let tag_count = self.count_secret_tags(&content);

        if !dry_run {
            // Write back to disk
            std::fs::write(path, smudged)?;
            println!("  🔓 Restored {} secrets in {:?}", tag_count, path);
        } else {
            println!("  🔍 Would restore {} secrets in {:?}", tag_count, path);
        }

        Ok(tag_count)
    }

    /// Migrate secret marker prefixes in-place without touching encrypted payload bytes.
    /// Example: `[DEMON_SECRET:...]` -> `[DRACON_SECRET:...]`.
    pub fn migrate_markers_in_path(
        &self,
        root: &Path,
        recursive: bool,
        dry_run: bool,
        from_marker: &str,
        to_marker: &str,
    ) -> Result<MarkerMigrationStats> {
        let from = normalize_secret_marker(from_marker)
            .ok_or_else(|| anyhow::anyhow!("Invalid source marker: {}", from_marker))?;
        let to = normalize_secret_marker(to_marker)
            .ok_or_else(|| anyhow::anyhow!("Invalid target marker: {}", to_marker))?;

        let from_prefix = format!("[{}:", from);
        let to_prefix = format!("[{}:", to);
        let mut stats = MarkerMigrationStats::default();

        if !root.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {:?}", root));
        }

        let mut process_file = |path: &Path| -> Result<()> {
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => return Ok(()),
            };
            stats.files_scanned += 1;

            let count = content.matches(&from_prefix).count();
            if count == 0 {
                return Ok(());
            }

            let migrated = content.replace(&from_prefix, &to_prefix);
            if !dry_run {
                fs::write(path, migrated)?;
            }

            stats.files_changed += 1;
            stats.markers_changed += count;
            Ok(())
        };

        if root.is_file() {
            process_file(root)?;
            return Ok(stats);
        }

        let walker = walkdir::WalkDir::new(root)
            .max_depth(if recursive { usize::MAX } else { 1 })
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                if e.path() == root {
                    return true;
                }
                !name.starts_with('.') || name == ".env"
            });

        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Err(e) = process_file(entry.path()) {
                    eprintln!("⚠️ failed to process {}: {}", entry.path().display(), e);
                }
            }
        }

        Ok(stats)
    }

    /// Git Smudge Filter: Decrypt stdin/file -> stdout
    /// Gracefully handles: V2 (Direct), V1 (RepoKey), Plaintext, REDACTED_REGEX wrapped
    pub fn seal_smudge(&self, file_path: Option<&str>) -> Result<()> {
        use std::io::{Read, Write};

        // 1. Read content
        let mut buffer = Vec::new();
        if let Some(path) = file_path {
            let mut file = fs::File::open(path)?;
            file.read_to_end(&mut buffer)?;
        } else {
            std::io::stdin().read_to_end(&mut buffer)?;
        }

        // 2. Check for V2 (Age) Header
        if buffer.starts_with(HEADER_V2_MAGIC) {
            match self.unlock_payload(&buffer) {
                Ok(plaintext) => {
                    std::io::stdout().write_all(&plaintext)?;
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("⚠️ V2 Decryption Failed: {}", e);
                    // Fallthrough to pass raw (might be intended?)
                }
            }
        }

        // 3. Check for *_SECRET text wrapper format
        if let Ok(text) = std::str::from_utf8(&buffer) {
            if self.contains_any_secret_tag(text) {
                let smudged = self.smart_smudge(text)?;
                std::io::stdout().write_all(smudged.as_bytes())?;
                return Ok(());
            }
        }

        // 4. Fallback: Pass raw buffer (Plaintext or already decrypted)
        std::io::stdout().write_all(&buffer)?;
        Ok(())
    }

    pub fn encrypt_with_repo_key(&self, repo_key: &RepoKey, plaintext: &[u8]) -> Result<Vec<u8>> {
        let key = Key::<Aes256Gcm>::from_slice(&repo_key.0);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("Encryption failure: {}", e))?;

        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypt data using the repo key
    pub fn decrypt_with_repo_key(
        &self,
        repo_key: &RepoKey,
        encrypted_data: &[u8],
    ) -> Result<Vec<u8>> {
        if encrypted_data.len() < 12 {
            // Graceful fallback: If data is too short, it might be plain text or empty.
            // For filter, error to be safe.
            return Err(anyhow::anyhow!("Invalid ciphertext length"));
        }

        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];

        let key = Key::<Aes256Gcm>::from_slice(&repo_key.0);
        let cipher = Aes256Gcm::new(key);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failure: {}", e))?;

        Ok(plaintext)
    }

    /// Decrypt data using the legacy Git Seal V1 format (AES-256-CFB with derived IV).
    /// WARNING: This format uses a deterministic IV derived from the key, which violates
    /// AES-CFB security requirements. Calls to this function are logged as security events.
    /// If you have ciphertexts created with this format, consider re-encrypting with a
    /// modern AEAD (AES-256-GCM with random nonce) when possible.
    pub fn decrypt_git_seal(&self, repo_key: &RepoKey, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let mut hasher = Sha256::new();
        hasher.update(&repo_key.0);
        let key_hash = hasher.finalize();

        let key = &key_hash[..32];
        let iv = &key_hash[..16];

        // Using dynamic dispatch or just specific type
        // Using specific Decryptor type from cfb_mode
        let cipher = cfb_mode::Decryptor::<aes::Aes256>::new_from_slices(key, iv)
            .map_err(|e| anyhow::anyhow!("CFB init error: {}", e))?;

        let mut plaintext = ciphertext.to_vec();
        cipher.decrypt(&mut plaintext);

        // Simple heuristic: check if result is likely plaintext
        // If it's garbage, it was probably not encrypted this way
        let is_likely_plaintext = plaintext
            .iter()
            .take(20)
            .all(|&b| b.is_ascii() && (b.is_ascii_graphic() || b.is_ascii_whitespace() || b == 0));

        if !is_likely_plaintext {
            return Err(anyhow::anyhow!(
                "Git Seal decryption produced binary garbage"
            ));
        }

        Ok(plaintext)
    }

    /// "Drunk guy with keychain" - try all keys from ~/demon/keys/
    fn try_keychain_bruteforce(&self, ciphertext: &[u8]) -> Option<Vec<u8>> {
        let home = match std::env::var("HOME") {
            Ok(h) => PathBuf::from(h),
            Err(_) => return None,
        };

        // Check demon key directories
        let keychain_dirs = vec![
            home.join(".demon").join("keys"), // key storage
            home.join(".arcane").join("keys"),
        ];

        for keys_dir in &keychain_dirs {
            if !keys_dir.exists() {
                continue;
            }

            // Collect all key files
            let entries = match fs::read_dir(keys_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                // Try to read as raw key (32 bytes)
                if let Ok(key_bytes) = fs::read(&path) {
                    if key_bytes.len() == 32 {
                        let repo_key = RepoKey(key_bytes);

                        // Try AES-GCM
                        if let Ok(plaintext) = self.decrypt_with_repo_key(&repo_key, ciphertext) {
                            eprintln!(
                                "🔓 Decrypted with keychain key (AES-GCM): {:?}",
                                path.file_name()
                            );
                            return Some(plaintext);
                        }

                        // Try AES-CFB (git-seal style)
                        if let Ok(plaintext) = self.decrypt_git_seal(&repo_key, ciphertext) {
                            eprintln!(
                                "🔓 Decrypted with keychain key (AES-CFB): {:?}",
                                path.file_name()
                            );
                            return Some(plaintext);
                        }
                    }
                }
            }
        } // end for keys_dir

        None
    }

    /// Recursively list files and check for ignored files (using whitelist)
    pub fn scan_dir(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    // Skip hidden directories (except .git? no, skip .git)
                    if path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .starts_with('.')
                    {
                        continue;
                    }
                    if let Ok(sub) = self.scan_dir(&path) {
                        files.extend(sub);
                    }
                } else {
                    files.push(path);
                }
            }
        }
        Ok(files)
    }

    fn get_registries_path(&self) -> Result<PathBuf> {
        let home = self.get_home()?;
        Ok(home.join(".demon").join("registries.age"))
    }

    pub fn load_registry_credentials(&self) -> Result<Vec<RegistryCredential>> {
        let path = self.get_registries_path()?;
        if !path.exists() {
            return Ok(Vec::new());
        }

        let encrypted_bytes = fs::read(&path)?;
        let identity = self
            .master_identities
            .first()
            .context("Master identity required to unlock registry credentials")?;

        let decryptor = age::Decryptor::new(std::io::Cursor::new(&encrypted_bytes))?;
        let mut reader = match decryptor {
            age::Decryptor::Recipients(d) => d
                .decrypt(std::iter::once(identity as &dyn age::Identity))
                .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?,
            age::Decryptor::Passphrase(_) => {
                return Err(anyhow::anyhow!("Passphrase encryption not supported"))
            }
        };

        let mut json_bytes = Vec::new();
        reader.read_to_end(&mut json_bytes)?;

        let creds: Vec<RegistryCredential> = serde_json::from_slice(&json_bytes)?;
        Ok(creds)
    }

    pub fn save_registry_credential(&self, cred: RegistryCredential) -> Result<()> {
        let mut creds = self.load_registry_credentials().unwrap_or_default();

        // Upsert logic
        if let Some(existing) = creds.iter_mut().find(|c| c.registry == cred.registry) {
            existing.username = cred.username;
            existing.password = cred.password;
        } else {
            creds.push(cred);
        }

        self.save_registry_credentials_list(&creds)
    }

    fn save_registry_credentials_list(&self, creds: &[RegistryCredential]) -> Result<()> {
        let path = self.get_registries_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json_bytes = serde_json::to_vec(creds)?;

        let master = self
            .master_identities
            .first()
            .context("Master identity required to encrypt registry credentials")?;
        let recipient = master.to_public();

        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(recipient)];
        let encryptor = age::Encryptor::with_recipients(recipients)
            .context("Failed to create encryptor for registry credentials")?;

        let mut file = fs::File::create(&path)?;
        let mut writer = encryptor.wrap_output(&mut file)?;
        writer.write_all(&json_bytes)?;
        writer.finish()?;

        Ok(())
    }
}

pub struct Warden;

impl Warden {
    pub fn new() -> Result<Self> {
        Ok(Warden)
    }

    pub fn smudge(&self, bytes: &[u8], _path: Option<&str>) -> Result<Vec<u8>> {
        let content = String::from_utf8_lossy(bytes);
        let smudged = DemonSecurity::new(None)?.smart_smudge(&content)?;
        Ok(smudged.into_bytes())
    }
}

pub struct DraconWarden;

impl DraconWarden {
    pub fn new() -> Result<Self> {
        Ok(DraconWarden)
    }

    pub fn smudge(&self, bytes: &[u8], path: Option<&str>) -> Result<Vec<u8>> {
        let path_str = path.unwrap_or("");
        let filename = std::path::Path::new(path_str)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let content = String::from_utf8_lossy(bytes);
        let smudged = DemonSecurity::new(None)?.smart_smudge(&content)?;
        let final_content = if is_env_file(filename) && !smudged.contains("Dracon Warden") {
            format!("{}\n{}", make_env_version_header(&smudged), smudged)
        } else {
            smudged
        };
        Ok(final_content.into_bytes())
    }

    pub fn clean(&self, bytes: &[u8], path: Option<&str>) -> Result<Vec<u8>> {
        let cleaned = DemonSecurity::new(None)?.smart_clean_with_path(bytes, path.unwrap_or(""))?;
        Ok(cleaned)
    }
}

fn is_env_file(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    path_lower.ends_with(".env")
        || path_lower.contains(".env.")
        || path_lower.ends_with(".envrc")
        || path_lower.ends_with("/.env")
        || path_lower.ends_with("/.envrc")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smudge_robustness() {
        let security = DemonSecurity::new(None).unwrap();
        assert_eq!(
            security.smart_smudge("[DRACON_SECRET:]").unwrap(),
            "[DRACON_SECRET:]"
        );
        let long_junk = "A".repeat(5000);
        let tag = format!("[DRACON_SECRET:{}]", long_junk);
        let input = format!("before {} after", tag);
        let output = security.smart_smudge(&input).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn test_protection_exemptions() {
        let scanner = SecretScanner::new_without_age_keys();
        let patterns = scanner.patterns.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>();
        eprintln!(
            "Patterns in scanner (excluding age keys): {}",
            patterns.len()
        );
        eprintln!(
            "Age Secret Key in patterns: {}",
            patterns.contains(&"Age Secret Key".to_string())
        );

        let content = "AGE-SECRET-KEY-142MYS9ZZPE0Q0CFSU4D3WTMMXRN5EN89U83TUSKGZVACLCE0A37SN5NENW";
        let scanned = scanner.scan_and_replace(content, |name, secret| {
            eprintln!("Match found: {} -> {}", name, secret);
            format!("[MATCHED:{}]", name)
        });
        eprintln!("Scanned result: {}", scanned);

        let security = DemonSecurity::new(None).unwrap();
        let result = security
            .smart_clean_with_path(content.as_bytes(), "master.age")
            .unwrap();
        let result_str = String::from_utf8_lossy(&result);
        assert!(
            !result_str.contains("AGE-SECRET-KEY"),
            "Age Secret Key pattern was not excluded from scanning! Result: {}",
            &result_str[..result_str.len().min(500)]
        );
    }

    #[test]
    fn test_legacy_marker_compatibility() {
        let security = DemonSecurity::new(None).unwrap();
        let input = "prefix [DEMON_SECRET:not-base64] suffix";
        let output = security.smart_smudge(input).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn test_marker_migration_in_place() {
        let security = DemonSecurity::new(None).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("sample.env");
        std::fs::write(
            &file,
            "A=[DEMON_SECRET:abc]\nB=[DEMON_SECRET:def]\nC=plain\n",
        )
        .unwrap();

        let stats = security
            .migrate_markers_in_path(dir.path(), true, false, "DEMON_SECRET", "DRACON_SECRET")
            .unwrap();
        assert_eq!(stats.files_changed, 1);
        assert_eq!(stats.markers_changed, 2);

        let migrated = std::fs::read_to_string(file).unwrap();
        assert!(migrated.contains("[DRACON_SECRET:abc]"));
        assert!(migrated.contains("[DRACON_SECRET:def]"));
        assert!(!migrated.contains("[DEMON_SECRET:"));
    }

    #[test]
    fn test_get_env_version_extracts_version() {
        let v1_content = r#"# =============================================================================
# Dracon Warden Encrypted Environment File
# Version: 1
# =============================================================================
API_KEY=secret"#;
        assert_eq!(get_env_version(v1_content), 1);

        let v5_content = r#"# =============================================================================
# Dracon Warden Encrypted Environment File
# Version: 5
# =============================================================================
API_KEY=secret"#;
        assert_eq!(get_env_version(v5_content), 5);

        let no_version = r#"API_KEY=secret"#;
        assert_eq!(get_env_version(no_version), 0);
    }

    #[test]
    fn test_make_env_version_header_increments_version() {
        let v1_content = r#"# Version: 1
API_KEY=secret"#;
        let header = make_env_version_header(v1_content);
        assert!(header.contains("Version: 2"));

        let v0_content = r#"API_KEY=secret"#;
        let header = make_env_version_header(v0_content);
        assert!(header.contains("Version: 1"));
    }

    #[test]
    fn test_strip_env_version_header_removes_header() {
        let with_header = r#"# =============================================================================
# Dracon Warden Encrypted Environment File
# Version: 1
# =============================================================================
API_KEY=secret"#;
        let stripped = strip_env_version_header(with_header);
        assert!(!stripped.contains("Dracon Warden"));
        assert!(stripped.contains("API_KEY=secret"));
        assert!(stripped.starts_with("API_KEY=secret"));
    }

    #[test]
    fn test_strip_env_version_header_passthrough_when_no_header() {
        let no_header = "API_KEY=secret";
        let stripped = strip_env_version_header(no_header);
        assert_eq!(stripped, no_header);
    }

    #[test]
    fn test_env_versioning_increment_flow() {
        let security = DemonSecurity::new(None).unwrap();

        let v1_content = r#"# =============================================================================
# Dracon Warden Encrypted Environment File
# Version: 1
# =============================================================================
API_KEY=original"#;

        let encrypted = security
            .smart_clean_with_path(v1_content.as_bytes(), ".env.local")
            .unwrap();
        let encrypted_str = String::from_utf8_lossy(&encrypted);

        let decrypted = security.smart_smudge(&encrypted_str).unwrap();
        assert!(
            decrypted.contains("Version: 2"),
            "Version should increment to 2, got:\n{}",
            decrypted
        );
        assert!(
            decrypted.contains("API_KEY=original"),
            "Content should be preserved"
        );
    }
}
