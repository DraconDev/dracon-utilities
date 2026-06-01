//! Git clean/smudge filter pipeline for encryption.

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use crate::make_env_version_header;
use crate::strip_env_version_header;
use crate::normalize_secret_marker;
use crate::DemonSecurity;
use crate::MarkerMigrationStats;
use crate::SecretScanner;

const HEADER_V2_MAGIC: &[u8] = b"age-encryption.org/v1";

impl DemonSecurity {
    pub fn smart_clean(&self, content: &str) -> Result<String> {
        let scanner = SecretScanner::new()?;
        self.smart_clean_with_scanner(content, &scanner)
    }

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

        // Check if any path component exactly matches a sensitive directory name.
        // Using component-level matching avoids false positives like "my.ssh.config"
        // matching ".ssh" via substring contains.
        let path_components: Vec<&str> = std::path::Path::new(path_str)
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        // Single-component matching
        let has_single_component = sensitive_dirs
            .iter()
            .any(|dir| !dir.contains('/') && path_components.contains(dir));
        // Multi-component sequence matching (e.g. ".config/gcloud")
        let has_multi_component = sensitive_dirs.iter().any(|dir| {
            let parts: Vec<&str> = dir.split('/').collect();
            if parts.len() < 2 {
                return false;
            }
            path_components
                .windows(parts.len())
                .any(|window| window == parts.as_slice())
        });

        let is_sensitive_location = has_single_component
            || has_multi_component
            || sensitive_exts.iter().any(|ext| path_str.ends_with(ext))
            || sensitive_filenames.contains(&filename)
            || sensitive_filenames
                .iter()
                .any(|p| filename == *p || filename.starts_with(&format!("{}.", p)))
            || self
                .managed_patterns
                .iter()
                .any(|p| filename == p || path_str.contains(p));

        // 2. Process based on content type
        match std::str::from_utf8(content) {
            Ok(text_content) => {
                // Full encryption for sensitive files that shouldn't leak structure
                let is_full_encrypt = is_sensitive_location
                    && (filename.starts_with(".env")
                        || filename == "credentials"
                        || filename.starts_with(".bash_history")
                        || filename.starts_with(".zsh_history")
                        || filename.starts_with(".sh_history")
                        || filename == "vault.yml");
                if is_full_encrypt {
                    // Don't double-encrypt
                    if content.starts_with(HEADER_V2_MAGIC)
                        || self.starts_with_any_secret_tag(content)
                    {
                        return Ok(content.to_vec());
                    }
                    // Add/increment version header for .env files to track changes
                    let content_to_encrypt = if filename.starts_with(".env") {
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
                // For identity files (master.age, identity.age), use a scanner that
                // skips age key patterns to avoid encrypting the identity itself,
                // but still catches other embedded secrets like API keys.
                let is_identity_file = filename == "master.age" || filename == "identity.age";
                let cleaned = if is_identity_file {
                    let scanner = SecretScanner::new_without_age_keys()?;
                    self.smart_clean_with_scanner(text_content, &scanner)?
                } else {
                    self.smart_clean(text_content)?
                };
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

    pub fn decrypt_path(&self, root: &Path, recursive: bool, dry_run: bool) -> Result<usize> {
        let mut total_restored = 0;
        let mut walk_errors = 0;

        if !root.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {:?}", root));
        }

        if root.is_file() {
            return self.decrypt_file(root, dry_run);
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

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!(
                        "⚠️ walk error during secret restore at {}: {}",
                        root.display(),
                        e
                    );
                    walk_errors += 1;
                    continue;
                }
            };
            if entry.file_type().is_file() {
                if let Ok(count) = self.decrypt_file(entry.path(), dry_run) {
                    total_restored += count;
                }
            }
        }

        if walk_errors > 0 {
            return Err(anyhow::anyhow!(
                "decrypt_path completed with {} walk error(s)",
                walk_errors
            ));
        }

        Ok(total_restored)
    }

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

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!(
                        "⚠️ walk error during marker scan at {}: {}",
                        root.display(),
                        e
                    );
                    stats.walk_errors += 1;
                    continue;
                }
            };
            if entry.file_type().is_file() {
                if let Err(e) = process_file(entry.path()) {
                    eprintln!("⚠️ failed to process {}: {}", entry.path().display(), e);
                }
            }
        }

        if stats.walk_errors > 0 {
            return Err(anyhow::anyhow!(
                "migrate_markers_in_path completed with {} walk error(s)",
                stats.walk_errors
            ));
        }

        Ok(stats)
    }

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

}
