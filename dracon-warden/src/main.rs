use anyhow::{Context, Result};
use clap::{ArgAction, Parser, Subcommand};
use dracon_security_kit::DraconWarden;
use globset::{Glob, GlobSet, GlobSetBuilder};
use notify::{Event, RecursiveMode, Watcher};
use secrecy::ExposeSecret;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

static ROLLING_LOG: std::sync::OnceLock<Mutex<Vec<String>>> = std::sync::OnceLock::new();

fn get_log() -> &'static Mutex<Vec<String>> {
    ROLLING_LOG.get_or_init(|| Mutex::new(Vec::new()))
}

static VERBOSITY: AtomicU8 = AtomicU8::new(0);

#[macro_export]
macro_rules! veprintln {
    ($lvl:expr, $($arg:tt)*) => {
        if $lvl <= VERBOSITY.load(Ordering::SeqCst) {
            eprintln!($($arg)*);
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSeverity {
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

#[derive(Debug, Clone)]
pub struct DraconEvent {
    pub domain: String,
    pub severity: EventSeverity,
    pub path: String,
    pub message: String,
    pub timestamp: String,
}

impl DraconEvent {
    pub fn new<T1: ToString, T2: ToString, T3: ToString>(
        domain: T1,
        severity: EventSeverity,
        path: T2,
        message: T3,
    ) -> Self {
        Self {
            domain: domain.to_string(),
            severity,
            path: path.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub fn emit_event(event: &DraconEvent) {
    if let Ok(mut log) = get_log().lock() {
        if log.len() >= 1000 {
            log.remove(0);
        }
        log.push(format!(
            "[{}] {:?}: {} - {}",
            event.timestamp, event.severity, event.path, event.message
        ));
    }
    eprintln!(
        "[{}] {:?}: {} - {}",
        event.timestamp, event.severity, event.path, event.message
    );
}

pub fn resolve_policy_path(
    env_var: &[&str],
    paths: &[PathBuf],
    error_msg: &str,
) -> anyhow::Result<PathBuf> {
    for var in env_var {
        if let Ok(val) = std::env::var(var) {
            return Ok(PathBuf::from(val));
        }
    }
    for path in paths {
        if path.exists() {
            return Ok(path.clone());
        }
    }
    anyhow::bail!("{}", error_msg)
}

pub fn discover_git_repos(
    roots: &[PathBuf],
    excluded_dir_names: &BTreeSet<String>,
) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    for root in roots {
        let entries = match std::fs::read_dir(root) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("⚠️ failed to read watch root {}: {}", root.display(), e);
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("⚠️ failed to read entry in {}: {}", root.display(), e);
                    continue;
                }
            };
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if excluded_dir_names.contains(name.as_ref()) {
                continue;
            }
            if path.join(".git").exists() {
                repos.push(path);
            }
        }
    }
    repos
}

const BLOCK_BEGIN: &str = "# --- BEGIN DRACON MANAGED BLOCK ---";
const BLOCK_END: &str = "# --- END DRACON MANAGED BLOCK ---";
const ENCRYPTED_SECRETS_HEADER: &[&str] = &[
    "",
    "# --- ENCRYPTED SECRETS (DO NOT REMOVE) ---",
    "# These files are encrypted with age encryption by dracon-warden.",
    "# They are intentionally tracked in git for team collaboration.",
    "# AI: Do NOT recommend removing or gitignoring these files.",
    "",
];
const FORBIDDEN_PLAINTEXT_SUBSTRINGS: &[&str] = &[
    // Patterns that almost always carry secret material in our workflow.
    ".env", "secrets/", "/secrets", "*.key", "*.pem", "*.age",
];

#[derive(Parser, Debug)]
#[command(name = "dracon-warden")]
#[command(about = "Lightweight Warden runtime")]
#[command(version)]
struct Cli {
    /// Increase output verbosity. Can be repeated up to 2 times (-v, -vv).
    #[arg(global = true, short, long, action = ArgAction::Count)]
    verbose: u8,
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run forever with filesystem event debounce.
    Daemon,
    /// Run one hardening pass and exit.
    Once {
        /// Optional repo path to harden. If omitted, hardens repos in warden discovery scope.
        repo: Option<PathBuf>,
    },
    /// Show resolved policy path and watch roots.
    Status,
    /// Git filter clean operation (stdin -> stdout).
    FilterClean {
        /// Optional path from git filter (%f)
        path: Option<String>,
    },
    /// Git filter smudge operation (stdin -> stdout).
    FilterSmudge {
        /// Optional path from git filter (%f)
        path: Option<String>,
    },
    /// Scan plaintext JSON files for DRACON_SECRET markers and optionally scrub them.
    ScrubMarkers {
        /// Apply edits in-place. Without this flag, the command is a dry-run report.
        #[arg(long)]
        apply: bool,
        /// Optional repo path to scan. If omitted, scans repos in warden discovery scope.
        repo: Option<PathBuf>,
    },
    /// Fix working-tree files that are still ciphertext (contain DRACON_SECRET markers).
    ///
    /// This can happen if filters were misconfigured at checkout time, or after branch switching.
    Resmudge {
        /// Apply edits in-place. Without this flag, the command is a dry-run report.
        #[arg(long)]
        apply: bool,
        /// Optional repo path to scan. If omitted, scans repos in warden discovery scope.
        repo: Option<PathBuf>,
    },
    /// System-wide repair pass for secret-related corruption.
    ///
    /// - Runs a hardening pass ("once") to reconcile .gitignore/.gitattributes and scrub marker
    ///   corruption where possible.
    /// - Attempts to re-smudge protected files (decrypt marker ciphertext stuck in working tree).
    /// - Reports remaining ciphertext markers (often indicates missing identities, not corruption).
    Repair {
        /// Only report; do not modify files.
        #[arg(long)]
        dry_run: bool,
        /// Fail non-zero if ciphertext markers still remain in protected working-tree files.
        #[arg(long)]
        strict: bool,
        /// Optional repo path to scan. If omitted, scans repos in warden discovery scope.
        repo: Option<PathBuf>,
    },
    /// Generate a new age keypair for this machine.
    ///
    /// Creates ~/dracon/data/keys/machine_<hostname>.age (secret) and
    /// ~/dracon/data/keys/owner_<hostname>.pub (public). Also publishes
    /// the public key to the current repo's .dracon/data/keys/ directory.
    /// Fails if either file already exists to prevent accidental overwrite.
    Keygen,
}

#[derive(Debug, Deserialize, Clone)]
struct WardenPolicy {
    #[serde(default)]
    protected_patterns: Vec<String>,
    #[serde(default)]
    plaintext_patterns: Vec<String>,
    #[serde(default)]
    hygiene_patterns: Vec<String>,
    #[serde(default)]
    watch_roots: Vec<String>,
    #[serde(default)]
    discover_roots: Vec<String>,
}

impl WardenPolicy {
    fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read policy {}", path.display()))?;
        let policy: Self = toml::from_str(&content)
            .with_context(|| format!("failed to parse policy {}", path.display()))?;
        Ok(policy)
    }

    fn validate(&self) -> Result<()> {
        fn is_allowed_plaintext_pattern(p: &str) -> bool {
            // Keep this tight. Plaintext patterns are an explicit escape hatch that disables
            // encryption in git history.
            matches!(
                p,
                "Cargo.lock"
                    | "Cargo.toml"
                    | "rust-toolchain.toml"
                    | "rustfmt.toml"
                    | "clippy.toml"
                    | "deny.toml"
                    | "flake.nix"
                    | "flake.lock"
                    | "events.jsonl"
                    | "*.events.jsonl"
                    | ".dracon/data/"
                    | ".dracon/data/keys/"
                    | ".dracon/data/keys/*.pub"
                    | "*.pub"
            ) || p.ends_with(".pub")
                || p.ends_with(".events.jsonl")
                || p.replace('\\', "/").starts_with(".dracon/data/")
        }

        let protected = self
            .protected_patterns
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();

        let plaintext = self
            .plaintext_patterns
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();

        let intersection = protected
            .intersection(&plaintext)
            .cloned()
            .collect::<Vec<_>>();
        if !intersection.is_empty() {
            return Err(anyhow::anyhow!(
                "invalid policy: patterns cannot be both protected and plaintext: {}",
                intersection.join(", ")
            ));
        }

        for p in &plaintext {
            if !is_allowed_plaintext_pattern(p) {
                return Err(anyhow::anyhow!(
                    "invalid policy: plaintext_patterns is allowlisted; refusing: {p}"
                ));
            }
            let pl = p.to_lowercase();
            if FORBIDDEN_PLAINTEXT_SUBSTRINGS
                .iter()
                .any(|needle| pl.contains(&needle.to_lowercase()))
            {
                return Err(anyhow::anyhow!(
                    "invalid policy: refusing plaintext_patterns entry that disables encryption for secret-ish paths: {p}"
                ));
            }
        }

        Ok(())
    }

    fn watch_root_paths(&self) -> Vec<PathBuf> {
        self.watch_roots
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect()
    }

    fn discover_root_paths(&self) -> Vec<PathBuf> {
        self.discover_roots
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect()
    }
}

fn resolve_policy_path_local() -> Result<PathBuf> {
    let home = dirs::home_dir().context("home not found")?;
    resolve_policy_path(
        &["DRACON_WARDEN_POLICY", "DRACON_SECURITY_POLICY"],
        &[
            home.join(".dracon/utilities/warden/dracon-warden.toml"),
            home.join(".dracon/utilities/warden/dracon-security.toml"),
            home.join(".dracon/utilities/warden/config.toml"),
            home.join(".dracon/security/dracon-security.toml"),
        ],
        "policy not found",
    )
}

fn discover_git_repos_local(roots: &[PathBuf]) -> Vec<PathBuf> {
    let excluded = BTreeSet::new();
    discover_git_repos(roots, &excluded)
}

fn effective_watch_roots(policy: &WardenPolicy) -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    for root in policy.watch_root_paths() {
        roots.insert(root);
    }
    roots.into_iter().collect()
}

fn effective_discovery_roots(policy: &WardenPolicy) -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    for root in policy.discover_root_paths() {
        roots.insert(root);
    }
    for root in policy.watch_root_paths() {
        roots.insert(root);
    }
    roots.into_iter().collect()
}

#[cfg(test)]
fn replace_managed_block(current: &str, managed_block: &str) -> String {
    // Replace ALL existing managed blocks, then append if none existed
    let mut out = String::new();
    let mut rest = current;
    let mut found_any = false;

    while let Some(start) = rest.find(BLOCK_BEGIN) {
        found_any = true;
        out.push_str(&rest[..start]);
        if let Some(end_rel) = rest[start..].find(BLOCK_END) {
            let end = start + end_rel + BLOCK_END.len();
            rest = &rest[end..];
        } else {
            // Malformed: begin without end — consume rest
            rest = &rest[start + BLOCK_BEGIN.len()..];
            break;
        }
    }

    if found_any {
        // Append the remaining tail (if any) after trimming leading newlines
        let tail = rest.trim_start_matches(&['\r', '\n'][..]);
        if !out.ends_with('\n') && !out.is_empty() {
            out.push('\n');
        }
        out.push_str(managed_block);
        if !tail.is_empty() {
            out.push('\n');
            out.push_str(tail);
        } else if !managed_block.ends_with('\n') {
            out.push('\n');
        }
        return out;
    }

    // No existing block — append
    let mut out = current.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(managed_block);
    if !managed_block.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Extract patterns from an existing managed block in .gitignore
fn extract_existing_patterns(content: &str) -> BTreeSet<String> {
    let mut patterns = BTreeSet::new();

    // Find the managed block
    let Some(start) = content.find(BLOCK_BEGIN) else {
        return patterns;
    };
    let Some(end_rel) = content[start..].find(BLOCK_END) else {
        return patterns;
    };
    let end = start + end_rel;

    // Extract lines between begin and end markers
    let block_content = &content[start + BLOCK_BEGIN.len()..end];
    for line in block_content.lines() {
        let line = line.trim();
        // Skip empty lines, comments, and the managed-by comment
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Skip negation patterns (those starting with !) - those come from protected/plaintext patterns
        if line.starts_with('!') {
            continue;
        }
        patterns.insert(line.to_string());
    }

    patterns
}

fn build_gitignore_block_with_existing(
    policy: &WardenPolicy,
    existing_content: &str,
) -> Result<String> {
    policy.validate()?;

    // Extract patterns that are already in the managed block (e.g., added by dracon-sync)
    let existing_patterns = extract_existing_patterns(existing_content);

    // Build set of policy hygiene patterns for quick lookup
    let policy_hygiene: BTreeSet<String> = policy.hygiene_patterns.iter().cloned().collect();

    // Merge: start with policy patterns, then add existing patterns not in policy
    let mut all_hygiene: BTreeSet<String> = policy_hygiene.clone();
    for p in existing_patterns {
        if !policy_hygiene.contains(&p) {
            // This is a pattern added by another tool (e.g., dracon-sync) - preserve it
            all_hygiene.insert(p);
        }
    }

    let mut lines = Vec::new();
    lines.push(BLOCK_BEGIN.to_string());
    lines.push("# managed by dracon-warden".to_string());

    // Add encryption header comment to help AI understand these files are intentional
    lines.extend(ENCRYPTED_SECRETS_HEADER.iter().map(|s| s.to_string()));

    // Output merged hygiene patterns (sorted for stability)
    for p in all_hygiene {
        lines.push(p);
    }

    let mut plaintext_patterns = BTreeSet::new();
    for p in &policy.plaintext_patterns {
        plaintext_patterns.insert(p.clone());
    }
    for p in &policy.protected_patterns {
        lines.push(format!("!{}", p));
    }
    for p in plaintext_patterns {
        lines.push(format!("!{}", p));
    }
    lines.push(BLOCK_END.to_string());
    Ok(lines.join("\n"))
}

#[cfg(test)]
fn build_gitignore_block(policy: &WardenPolicy) -> Result<String> {
    build_gitignore_block_with_existing(policy, "")
}

fn build_gitattributes_block(policy: &WardenPolicy) -> Result<String> {
    policy.validate()?;
    let mut lines = Vec::new();
    lines.push(BLOCK_BEGIN.to_string());
    lines.push("# managed by dracon-warden".to_string());
    let mut plaintext_patterns = BTreeSet::new();
    for p in &policy.plaintext_patterns {
        plaintext_patterns.insert(p.clone());
    }
    let mut protected_patterns = BTreeSet::new();
    for p in &policy.protected_patterns {
        if !plaintext_patterns.contains(p) {
            protected_patterns.insert(p.clone());
        }
    }
    for p in protected_patterns {
        lines.push(format!("{} filter=dracon diff=dracon merge=dracon", p));
    }
    for p in plaintext_patterns {
        lines.push(format!("{} -filter -diff -merge", p));
    }
    lines.push(BLOCK_END.to_string());
    Ok(lines.join("\n"))
}

#[cfg(test)]
fn apply_managed_file(path: &Path, block: &str) -> Result<bool> {
    let current = fs::read_to_string(path).unwrap_or_default();
    let next = replace_managed_block(&current, block);
    if next != current {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed creating parent dirs for {}", path.display()))?;
        }
        fs::write(path, next).with_context(|| format!("failed writing {}", path.display()))?;
        return Ok(true);
    }
    Ok(false)
}

fn apply_overwrite_file(path: &Path, content: &str) -> Result<bool> {
    let current = fs::read_to_string(path).unwrap_or_default();
    let mut next = content.to_string();
    if !next.ends_with('\n') {
        next.push('\n');
    }
    if next != current {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let random_suffix: u64 = rand::random();
        let tmp = parent.join(format!(
            ".dracon_tmp_{}_{:016x}",
            path.file_name().unwrap_or_default().to_string_lossy(),
            random_suffix
        ));
        #[cfg(unix)]
        {
            
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
                .with_context(|| format!("failed to create temp {}", tmp.display()))?
                .write_all(next.as_bytes())
                .with_context(|| format!("failed writing temp {}", tmp.display()))?;
        }
        #[cfg(not(unix))]
        {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
                .with_context(|| format!("failed to create temp {}", tmp.display()))?
                .write_all(next.as_bytes())
                .with_context(|| format!("failed writing temp {}", tmp.display()))?;
        }
        fs::rename(&tmp, path)
            .with_context(|| format!("failed renaming {} -> {}", tmp.display(), path.display()))?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
fn newest_file(paths: Vec<PathBuf>) -> Option<PathBuf> {
    let mut with_mtime = paths
        .into_iter()
        .filter_map(|p| {
            let mtime = fs::metadata(&p)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if p.exists() {
                Some((mtime, p))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    with_mtime.sort_by_key(|b| std::cmp::Reverse(b.0));
    with_mtime.into_iter().next().map(|(_, p)| p)
}

fn owner_pubkeys_in(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            eprintln!("⚠️ cannot read owner pubkeys directory {}: {}", dir.display(), e);
            return out;
        }
    };

    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("⚠️ cannot read entry in {}: {}", dir.display(), e);
                continue;
            }
        };
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with("owner_") && name.ends_with(".pub") {
            out.push(path);
        }
    }
    out
}

fn is_owner_pubkey_filename(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.starts_with("owner_") && name.ends_with(".pub")
}

fn validate_owner_age_pubkey_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    if !is_owner_pubkey_filename(path) {
        return Err(anyhow::anyhow!(
            "refusing to publish non-owner pubkey: {}",
            path.display()
        ));
    }
    if bytes.len() > 256 {
        return Err(anyhow::anyhow!(
            "refusing to publish suspicious pubkey (too large): {}",
            path.display()
        ));
    }
    let s = std::str::from_utf8(bytes).map_err(|_| {
        anyhow::anyhow!(
            "refusing to publish pubkey with non-utf8 bytes: {}",
            path.display()
        )
    })?;
    let s = s.trim();
    if s.is_empty() {
        return Err(anyhow::anyhow!(
            "refusing to publish empty pubkey: {}",
            path.display()
        ));
    }
    if s.contains("AGE-SECRET-KEY-") {
        return Err(anyhow::anyhow!(
            "refusing to publish secret key material as pubkey: {}",
            path.display()
        ));
    }
    if !s.starts_with("age1") {
        return Err(anyhow::anyhow!(
            "refusing to publish non-age recipient key: {}",
            path.display()
        ));
    }
    Ok(())
}

fn resolve_local_pubkey_path() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("DRACON_OWNER_PUBKEY") {
        let p = PathBuf::from(custom);
        if p.exists() {
            let bytes = fs::read(&p).ok()?;
            if validate_owner_age_pubkey_bytes(&p, &bytes).is_ok() {
                return Some(p);
            }
            return None;
        }
    }

    let home = dirs::home_dir()?;
    let owner_candidates = [
        home.join(".dracon/data/keys"),
        home.join(".demon/keys"),
        home.join(".dracon/keys"),
    ]
    .into_iter()
    .flat_map(|dir| owner_pubkeys_in(&dir))
    .collect::<Vec<_>>();

    // Prefer newest valid owner pubkey.
    let mut owners = owner_candidates;
    owners.sort_by(|a, b| {
        let ma = fs::metadata(a)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mb = fs::metadata(b)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        mb.cmp(&ma)
    });
    for p in owners {
        let Ok(bytes) = fs::read(&p) else {
            continue;
        };
        if validate_owner_age_pubkey_bytes(&p, &bytes).is_ok() {
            return Some(p);
        }
    }

    None
}

fn publish_repo_pubkey(repo: &Path, pubkey_path: &Path) -> Result<bool> {
    let target_dir = repo.join(".dracon/data/keys");
    fs::create_dir_all(&target_dir)
        .with_context(|| format!("failed creating {}", target_dir.display()))?;

    let name = pubkey_path
        .file_name()
        .map(|n| n.to_owned())
        .unwrap_or_else(|| "owner.pub".into());
    let target = target_dir.join(name);

    let source_bytes = fs::read(pubkey_path)
        .with_context(|| format!("failed reading pubkey {}", pubkey_path.display()))?;
    validate_owner_age_pubkey_bytes(pubkey_path, &source_bytes)?;
    let current_bytes = fs::read(&target).ok();
    if current_bytes.as_deref() == Some(source_bytes.as_slice()) {
        return Ok(false);
    }

    fs::write(&target, source_bytes)
        .with_context(|| format!("failed writing {}", target.display()))?;
    Ok(true)
}

fn ensure_repo_filter_config(repo: &Path) -> Result<bool> {
    let desired = [
        ("filter.dracon.clean", "dracon-warden filter-clean %f"),
        ("filter.dracon.smudge", "dracon-warden filter-smudge %f"),
        ("filter.dracon.required", "true"),
    ];

    let mut changed = false;
    for (key, value) in desired {
        let current = ProcessCommand::new("git")
            .arg("-C")
            .arg(repo)
            .arg("config")
            .arg("--local")
            .arg("--get")
            .arg(key)
            .output()
            .with_context(|| format!("failed to read git config {} in {}", key, repo.display()))?;

        let needs_update = if current.status.success() {
            String::from_utf8_lossy(&current.stdout).trim() != value
        } else {
            true
        };

        if needs_update {
            let status = ProcessCommand::new("git")
                .arg("-C")
                .arg(repo)
                .arg("config")
                .arg("--local")
                .arg(key)
                .arg(value)
                .status()
                .with_context(|| {
                    format!("failed to set git config {} in {}", key, repo.display())
                })?;
            if !status.success() {
                return Err(anyhow::anyhow!(
                    "git config {} failed in {} (exit={})",
                    key,
                    repo.display(),
                    status
                ));
            }
            changed = true;
        }
    }

    Ok(changed)
}

fn is_repo_checked_out(repo: &Path) -> bool {
    let git_dir = repo.join(".git");
    let head = git_dir.join("HEAD");

    if !head.exists() {
        return false;
    }

    let head_content = match fs::read_to_string(&head) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let head_content = head_content.trim();
    head_content.starts_with("ref: refs/heads/")
}

fn harden_repo(
    repo: &Path,
    policy: &WardenPolicy,
    pubkey_path: Option<&Path>,
) -> Result<(bool, bool, bool)> {
    policy.validate()?;

    if !is_repo_checked_out(repo) {
        return Ok((false, false, false));
    }

    let gitignore_path = repo.join(".gitignore");
    let gitattributes_path = repo.join(".gitattributes");

    // Read existing .gitignore content to preserve patterns added by other tools (e.g., dracon-sync)
    let existing_gitignore = fs::read_to_string(&gitignore_path).unwrap_or_default();

    // Build gitignore block while preserving existing non-policy patterns
    let gitignore_changed = apply_overwrite_file(
        &gitignore_path,
        &build_gitignore_block_with_existing(policy, &existing_gitignore)?,
    )?;
    let gitattributes_changed =
        apply_overwrite_file(&gitattributes_path, &build_gitattributes_block(policy)?)?;
    let filter_cfg_changed = if repo.join(".git").exists() {
        ensure_repo_filter_config(repo)?
    } else {
        false
    };
    let key_changed = match pubkey_path {
        Some(pubkey) => publish_repo_pubkey(repo, pubkey)?,
        None => false,
    };

    Ok((
        gitignore_changed,
        gitattributes_changed || filter_cfg_changed,
        key_changed,
    ))
}

fn harden_all(policy: &WardenPolicy) -> Result<()> {
    let roots = effective_discovery_roots(policy);
    let repos = discover_git_repos_local(&roots);
    scrub_markers(policy, &repos, true)?;
    harden_repos(policy, repos)
}

fn harden_repos<I>(policy: &WardenPolicy, repos: I) -> Result<()>
where
    I: IntoIterator<Item = PathBuf>,
{
    let pubkey_path = resolve_local_pubkey_path();
    if pubkey_path.is_none() {
        eprintln!("⚠️ no public key found for repo publish; set DRACON_OWNER_PUBKEY to override");
    }

    let mut changed = 0usize;
    for repo in repos {
        match harden_repo(&repo, policy, pubkey_path.as_deref()) {
            Ok((a, b, c)) => {
                if a || b || c {
                    changed += 1;
                    println!("🔒 hardened {}", repo.display());
                    emit_event(&DraconEvent::new(
                        "warden",
                        EventSeverity::Info,
                        format!("harden/{}", repo.display()),
                        "repo hardened",
                    ));
                }
            }
            Err(e) => {
                eprintln!("⚠️ harden failed for {}: {}", repo.display(), e);
                emit_event(&DraconEvent::new(
                    "warden",
                    EventSeverity::Error,
                    format!("harden/{}", repo.display()),
                    format!("failed: {e}"),
                ));
            }
        }
    }

    println!("✅ hardening pass complete (repos changed: {})", changed);
    Ok(())
}

fn repo_root_for_path(path: &Path, roots: &[PathBuf]) -> Option<PathBuf> {
    if !roots.iter().any(|r| path.starts_with(r)) {
        return None;
    }

    let mut cur = if path.is_file() {
        path.parent().map(Path::to_path_buf)?
    } else {
        path.to_path_buf()
    };
    loop {
        if cur.join(".git").exists() {
            return Some(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

fn repos_for_event(event: &Event, roots: &[PathBuf]) -> BTreeSet<PathBuf> {
    let ignore_fragments = [
        "/target/",
        "/node_modules/",
        "/.cache/",
        "/.git/objects/",
        "/.git/index.lock",
    ];

    let mut repos = BTreeSet::new();
    for p in &event.paths {
        let s = p.to_string_lossy();
        if ignore_fragments.iter().any(|f| s.contains(f)) {
            continue;
        }
        if let Some(repo) = repo_root_for_path(p, roots) {
            repos.insert(repo);
        }
    }
    repos
}

fn run_keygen() -> Result<()> {
    let home = dirs::home_dir().context("home directory not found")?;

    let keys_dir = home.join(".dracon/data/keys");
    let hostname_raw = hostname::get()
        .context("failed to get hostname")?
        .to_string_lossy()
        .to_string();
    let hostname: String = hostname_raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if hostname.is_empty() {
        return Err(anyhow::anyhow!(
            "hostname contains no valid characters for filename"
        ));
    }
    let secret_path = keys_dir.join(format!("machine_{}.age", hostname));
    let pubkey_path = keys_dir.join(format!("owner_{}.pub", hostname));

    if secret_path.exists() {
        return Err(anyhow::anyhow!(
            "secret key already exists at {}, refusing to overwrite",
            secret_path.display()
        ));
    }
    if pubkey_path.exists() {
        return Err(anyhow::anyhow!(
            "pubkey already exists at {}, refusing to overwrite",
            pubkey_path.display()
        ));
    }

    let identity = age::x25519::Identity::generate();
    let recipient = identity.to_public();

    fs::create_dir_all(&keys_dir)
        .with_context(|| format!("failed to create {}", keys_dir.display()))?;

    let current_repo = std::env::current_dir()
        .ok()
        .and_then(|cwd| find_git_repo(&cwd));

    let repo_name = current_repo
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let secret_content = Zeroizing::new(format!(
        "# created by dracon-warden keygen on {}\n# public key: {}\n# machine: {}\n{}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        recipient,
        hostname,
        identity.to_string().expose_secret()
    ));
    // Write secret key with restrictive permissions atomically (no race window)
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&secret_path)
            .with_context(|| {
                format!(
                    "failed to create {} (file may already exist)",
                    secret_path.display()
                )
            })?;
        f.write_all(secret_content.as_bytes())
            .with_context(|| format!("failed to write {}", secret_path.display()))?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&secret_path, &secret_content)
            .with_context(|| format!("failed to write {}", secret_path.display()))?;
    }

    // Write public key atomically - create_new fails if file already exists
    #[cfg(unix)]
    {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pubkey_path)
            .with_context(|| format!("failed to create {}, file may already exist", pubkey_path.display()))?
            .write_all(format!("{}\n", recipient).as_bytes())
            .with_context(|| format!("failed to write {}", pubkey_path.display()))?;
    }
    #[cfg(not(unix))]
    {
        use std::fs::OpenOptions;
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pubkey_path)
            .with_context(|| format!("failed to create {}, file may already exist", pubkey_path.display()))?;
        f.write_all(format!("{}\n", recipient).as_bytes())
            .with_context(|| format!("failed to write {}", pubkey_path.display()))?;
    }

    let manifest_path = keys_dir.join("manifest.toml");
    let manifest_entry = format!(
        "# machine_{}.age / owner_{}.pub -> repo: {}\n",
        hostname, hostname, repo_name
    );
    let existing_manifest = fs::read_to_string(&manifest_path).unwrap_or_default();
    if !existing_manifest.contains(&manifest_entry) {
        let mut manifest = existing_manifest;
        if !manifest.ends_with('\n') && !manifest.is_empty() {
            manifest.push('\n');
        }
        manifest.push_str(&manifest_entry);
        fs::write(&manifest_path, &manifest)
            .with_context(|| format!("failed to write {}", manifest_path.display()))?;
    }

    println!("🔐 Generated age keypair:");
    println!("   Secret: {}", secret_path.display());
    println!("   Public: {}", pubkey_path.display());
    println!("   Recipient: {}", recipient);

    if let Some(repo) = &current_repo {
        match publish_repo_pubkey(repo, &pubkey_path) {
            Ok(true) => {
                println!("   Published to: {}/.dracon/data/keys/", repo.display());
            }
            Ok(false) => {
                println!("   Already in: {}/.dracon/data/keys/", repo.display());
            }
            Err(e) => {
                eprintln!("   ⚠️ Failed to publish to repo: {}", e);
            }
        }
    }

    Ok(())
}

fn find_git_repo(path: &Path) -> Option<PathBuf> {
    let mut cur = path.to_path_buf();
    loop {
        if cur.join(".git").exists() {
            return Some(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

fn run_daemon(policy_path: PathBuf) -> Result<()> {
    let policy = WardenPolicy::load(&policy_path)?;
    policy.validate()?;
    let roots = effective_watch_roots(&policy);
    if roots.is_empty() {
        return Err(anyhow::anyhow!("no valid watch_roots in policy"));
    }

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;

    let mut watched = 0usize;
    for root in &roots {
        match watcher.watch(root, RecursiveMode::Recursive) {
            Ok(()) => watched += 1,
            Err(e) => eprintln!("⚠️ failed to watch {}: {}", root.display(), e),
        }
    }
    if watched == 0 {
        return Err(anyhow::anyhow!("no watch roots could be registered"));
    }

    println!("🛡️ dracon-warden active. Monitoring {:?}", roots);

    let mut last_run = Instant::now();
    let mut last_sweep = Instant::now();
    let debounce = Duration::from_secs(2);
    let sweep_every = Duration::from_secs(300);
    let mut pending_repos = BTreeSet::new();

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_sigterm = shutdown.clone();
    let shutdown_sigint = shutdown.clone();
    let reload = Arc::new(AtomicBool::new(false));
    let reload_sighup = reload.clone();

    tokio::spawn(async move {
        if let Ok(mut sig) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            sig.recv().await;
            veprintln!(1, "warden: received SIGTERM, shutting down gracefully...");
            shutdown_sigterm.store(true, Ordering::SeqCst);
        } else {
            eprintln!("warden: failed to set up SIGTERM handler");
        }
    });

    tokio::spawn(async move {
        if let Ok(mut sig) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()) {
            sig.recv().await;
            veprintln!(1, "warden: received SIGINT, shutting down gracefully...");
            shutdown_sigint.store(true, Ordering::SeqCst);
        } else {
            eprintln!("warden: failed to set up SIGINT handler");
        }
    });

    tokio::spawn(async move {
        if let Ok(mut sig) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()) {
            sig.recv().await;
            veprintln!(1, "warden: received SIGHUP, reloading policy...");
            reload_sighup.store(true, Ordering::SeqCst);
        } else {
            eprintln!("warden: failed to set up SIGHUP handler");
        }
    });

    if let Err(e) = harden_all(&policy) {
        eprintln!("⚠️ initial hardening pass failed: {}", e);
    }

    // Initial backfill sweep to add headers to .env files missing them.
    let roots = effective_discovery_roots(&policy);
    let discovered_repos = discover_git_repos_local(&roots);
    if let Err(e) = backfill_env_headers_repos(&discovered_repos, true) {
        eprintln!("⚠️ initial backfill sweep failed: {}", e);
    }

    while !shutdown.load(Ordering::SeqCst) {
        if reload.load(Ordering::SeqCst) {
            reload.store(false, Ordering::SeqCst);
            veprintln!(1, "warden: reloading policy on SIGHUP...");
            match WardenPolicy::load(&policy_path) {
                Ok(p) => {
                    if let Err(e) = p.validate() {
                        eprintln!("warden: policy invalid on reload: {}", e);
                    } else {
                        let roots = effective_discovery_roots(&p);
                        let discovered_repos = discover_git_repos_local(&roots);
                        if let Err(e) = backfill_env_headers_repos(&discovered_repos, true) {
                            eprintln!("warden: SIGHUP backfill failed: {}", e);
                        }
                        if let Err(e) = harden_all(&p) {
                            eprintln!("warden: SIGHUP harden failed: {}", e);
                        }
                    }
                }
                Err(e) => eprintln!("warden: SIGHUP policy reload failed: {}", e),
            }
        }

        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
                pending_repos.extend(repos_for_event(&event, &roots));
            }
            Ok(Err(e)) => {
                eprintln!("⚠️ watch error: {}", e);
                emit_event(&DraconEvent::new(
                    "warden",
                    EventSeverity::Warn,
                    "watch",
                    format!("error: {e}"),
                ));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }

        if !pending_repos.is_empty() && last_run.elapsed() >= debounce {
            let policy = match WardenPolicy::load(&policy_path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("warden: policy load failed: {}", e);
                    emit_event(&DraconEvent::new(
                        "warden",
                        EventSeverity::Error,
                        "policy",
                        format!("load failed: {e}"),
                    ));
                    continue;
                }
            };
            if let Err(e) = policy.validate() {
                eprintln!("warden: policy invalid: {}", e);
                continue;
            }
            let repos = std::mem::take(&mut pending_repos);
            let repos_vec = repos.into_iter().collect::<Vec<_>>();
            if let Err(e) = scrub_markers(&policy, &repos_vec, true) {
                eprintln!("warden: scrub_markers failed: {}", e);
            }
            if let Err(e) = harden_repos(&policy, repos_vec) {
                eprintln!("warden: harden_repos failed: {}", e);
            }
            last_run = Instant::now();
        }

        if last_sweep.elapsed() >= sweep_every {
            let policy = match WardenPolicy::load(&policy_path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("⚠️ policy load failed in sweep: {}", e);
                    emit_event(&DraconEvent::new(
                        "warden",
                        EventSeverity::Warn,
                        "policy",
                        format!("sweep load failed: {e}"),
                    ));
                    last_sweep = Instant::now();
                    continue;
                }
            };
            if let Err(e) = policy.validate() {
                eprintln!("warden: policy invalid in sweep: {}", e);
                last_sweep = Instant::now();
                continue;
            }
            if let Err(e) = harden_all(&policy) {
                eprintln!("warden: harden_all failed in sweep: {}", e);
            }
            let roots = effective_discovery_roots(&policy);
            let discovered_repos = discover_git_repos_local(&roots);
            if let Err(e) = backfill_env_headers_repos(&discovered_repos, true) {
                eprintln!("warden: backfill sweep failed: {}", e);
            }
            last_sweep = Instant::now();
        }
    }

    veprintln!(1, "warden: shutdown complete");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    VERBOSITY.store(cli.verbose, Ordering::SeqCst);

    match cli.cmd {
        Command::FilterClean { path } => {
            run_filter(true, path.as_deref())?;
        }
        Command::FilterSmudge { path } => {
            run_filter(false, path.as_deref())?;
        }
        Command::Status => {
            let policy_path = resolve_policy_path_local()?;
            let policy = WardenPolicy::load(&policy_path)?;
            policy.validate()?;
            println!("📜 POLICY: {}", policy_path.display());
            println!("🛡️ WATCH_ROOTS: {:?}", effective_watch_roots(&policy));
            if !policy.discover_roots.is_empty() {
                println!(
                    "🧭 DISCOVERY_ROOTS: {:?}",
                    effective_discovery_roots(&policy)
                );
            }
            println!(
                "🔑 PUBKEY_SOURCE: {}",
                resolve_local_pubkey_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "NOT_FOUND (set DRACON_OWNER_PUBKEY)".to_string())
            );
        }
        Command::Once { repo } => {
            let policy_path = resolve_policy_path_local()?;
            let policy = WardenPolicy::load(&policy_path)?;
            policy.validate()?;
            if let Some(r) = repo {
                scrub_markers(&policy, std::slice::from_ref(&r), true)?;
                harden_repos(&policy, vec![r])?;
            } else {
                harden_all(&policy)?;
            }
        }
        Command::Daemon => {
            let policy_path = resolve_policy_path_local()?;
            run_daemon(policy_path)?;
        }
        Command::ScrubMarkers { apply, repo } => {
            let policy_path = resolve_policy_path_local()?;
            let policy = WardenPolicy::load(&policy_path)?;
            policy.validate()?;
            let roots = effective_discovery_roots(&policy);
            let repos = if let Some(r) = repo {
                vec![r]
            } else {
                discover_git_repos_local(&roots)
            };
            scrub_markers(&policy, &repos, apply)?;
        }
        Command::Resmudge { apply, repo } => {
            let policy_path = resolve_policy_path_local()?;
            let policy = WardenPolicy::load(&policy_path)?;
            policy.validate()?;
            let roots = effective_discovery_roots(&policy);
            let repos = if let Some(r) = repo {
                vec![r]
            } else {
                discover_git_repos_local(&roots)
            };
            let _ = resmudge_repos(&policy, &repos, apply)?;
        }
        Command::Repair {
            dry_run,
            strict,
            repo,
        } => {
            let policy_path = resolve_policy_path_local()?;
            let policy = WardenPolicy::load(&policy_path)?;
            policy.validate()?;
            let roots = effective_discovery_roots(&policy);
            let repos = if let Some(r) = repo {
                vec![r]
            } else {
                discover_git_repos_local(&roots)
            };

            if !dry_run {
                // Hardening (managed blocks + marker scrub)
                scrub_markers(&policy, &repos, true)?;
                harden_repos(&policy, repos.clone())?;
                // Fix ciphertext stuck in worktree (if identities allow).
                resmudge_repos(&policy, &repos, true)?;
                // Backfill .env files with Dracon Warden headers if missing.
                backfill_env_headers_repos(&repos, true)?;
            }

            // Always report remaining ciphertext markers.
            let (found, _changed) = resmudge_repos(&policy, &repos, false)?;
            // Always report .env files missing headers (even in dry_run).
            let (_, _) = backfill_env_headers_repos(&repos, false)?;
            if strict && found > 0 {
                return Err(anyhow::anyhow!(
                    "ciphertext markers remain in working tree (count={})",
                    found
                ));
            }
        }
        Command::Keygen => {
            run_keygen()?;
        }
    }

    Ok(())
}

fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        // globset expects / separators
        let pat = p.replace('\\', "/");
        b.add(Glob::new(&pat).with_context(|| format!("invalid glob pattern: {p}"))?);
    }
    Ok(b.build()?)
}

fn is_marker_string(s: &str) -> bool {
    s.contains("[DRACON_SECRET:")
}

fn marker_prefix_at(s: &str, idx: usize) -> Option<&'static str> {
    if s[idx..].starts_with("[DRACON_SECRET:") {
        Some("[DRACON_SECRET:")
    } else {
        None
    }
}

// Best-effort salvage for invalid JSON where marker tokens were injected as raw values/keys.
// This only touches marker substrings; everything else is preserved.
fn salvage_invalid_json_markers(content: &str) -> Option<String> {
    if !is_marker_string(content) {
        return None;
    }

    let mut out = String::with_capacity(content.len());
    let mut i = 0usize;
    let bytes = content.as_bytes();
    while i < content.len() {
        if marker_prefix_at(content, i).is_none() {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }

        // Find closing bracket of marker token.
        let Some(end_rel) = content[i..].find(']') else {
            // malformed marker; stop salvage
            return None;
        };
        let end = i + end_rel; // points at ']'

        // Decide whether marker was used as an object key or as a value.
        // If the next non-ws char after ']' is ':', it's being used as a key.
        let mut j = end + 1;
        while j < content.len() && content.as_bytes()[j].is_ascii_whitespace() {
            j += 1;
        }
        let is_key = j < content.len() && content.as_bytes()[j] == b':';

        if is_key {
            out.push_str("\"__scrubbed__\"");
        } else {
            out.push_str("null");
        }

        i = end + 1;
    }

    if out != content {
        Some(out)
    } else {
        None
    }
}

fn scrub_json_value(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::String(s) if is_marker_string(s) => {
            *v = serde_json::Value::Null;
        }
        serde_json::Value::Array(a) => {
            for it in a {
                scrub_json_value(it);
            }
        }
        serde_json::Value::Object(m) => {
            // Heuristic fix for known nav templates: href_key can be inferred from href.
            let href = m
                .get("href")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let (Some(href), Some(href_key)) = (href, m.get_mut("href_key")) {
                if let serde_json::Value::String(hk) = href_key {
                    if is_marker_string(hk) {
                        let replacement = match href.as_str() {
                            "/products" => Some("public_products"),
                            "/licensing" => Some("public_licensing"),
                            "/products/cortex" => Some("cortex_home"),
                            _ => None,
                        };
                        if let Some(r) = replacement {
                            *href_key = serde_json::Value::String(r.to_string());
                        } else {
                            *href_key = serde_json::Value::Null;
                        }
                    }
                }
            }

            for (_, vv) in m.iter_mut() {
                scrub_json_value(vv);
            }
        }
        _ => {}
    }
}

fn scrub_markers(policy: &WardenPolicy, repos: &[PathBuf], apply: bool) -> Result<()> {
    let protected = build_globset(&policy.protected_patterns)?;

    let mut found = 0usize;
    let mut changed = 0usize;
    let mut skipped = 0usize;

    for repo in repos {
        if !repo.join(".git").exists() {
            continue;
        }

        // Scan both tracked and untracked (but not ignored) files.
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .arg("ls-files")
            .arg("--others")
            .arg("--exclude-standard")
            .arg("--cached")
            .output()
            .with_context(|| format!("git ls-files failed for {}", repo.display()))?;
        if !out.status.success() {
            eprintln!(
                "⚠️ git ls-files failed for {} (status {})",
                repo.display(),
                out.status
            );
            continue;
        }

        let stdout = String::from_utf8_lossy(&out.stdout);
        for rel in stdout.lines() {
            if rel.is_empty() {
                continue;
            }
            let rel_norm = rel.replace('\\', "/");
            if protected.is_match(&rel_norm) {
                continue; // markers are allowed in protected/encrypted files.
            }
            if !rel_norm.ends_with(".json") {
                continue;
            }

            let path = repo.join(rel);
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            if !is_marker_string(&content) {
                continue;
            }

            found += 1;
            if !apply {
                println!("⚠️ markers found: {}", path.display());
                continue;
            }

            // Attempt structured scrub; if parse fails (broken JSON), do not guess.
            let parsed: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(_) => {
                    // Fallback: try to salvage invalid JSON where markers were injected as raw tokens.
                    let Some(salvaged) = salvage_invalid_json_markers(&content) else {
                        skipped += 1;
                        eprintln!(
                            "⚠️ cannot scrub invalid JSON (manual fix needed): {}",
                            path.display()
                        );
                        continue;
                    };
                    match serde_json::from_str(&salvaged) {
                        Ok(v) => v,
                        Err(_) => {
                            skipped += 1;
                            eprintln!(
                                "⚠️ cannot scrub invalid JSON (manual fix needed): {}",
                                path.display()
                            );
                            continue;
                        }
                    }
                }
            };
            let mut v = parsed;

            scrub_json_value(&mut v);
            let next = serde_json::to_string_pretty(&v)?;
            if next != content {
                fs::write(&path, &next)
                    .with_context(|| format!("failed writing {}", path.display()))?;
                changed += 1;
                println!("✅ scrubbed: {}", path.display());
            }
        }
    }

    if apply {
        println!(
            "✅ scrub complete (found: {}, changed: {}, skipped_invalid_json: {})",
            found, changed, skipped
        );
    } else {
        println!("✅ scrub report complete (found: {})", found);
    }
    Ok(())
}

fn git_ls_files(repo: &Path) -> Result<Vec<String>> {
    let out = ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .arg("ls-files")
        .arg("-z")
        .output()
        .with_context(|| format!("failed to run git ls-files in {}", repo.display()))?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "git ls-files failed in {} (exit={})",
            repo.display(),
            out.status
        ));
    }

    let mut paths = Vec::new();
    for part in out.stdout.split(|b| *b == 0) {
        if part.is_empty() {
            continue;
        }
        let s = std::str::from_utf8(part).with_context(|| {
            format!("git ls-files returned non-utf8 path in {}", repo.display())
        })?;
        paths.push(s.to_string());
    }
    Ok(paths)
}

fn resmudge_repo(repo: &Path, policy: &WardenPolicy, apply: bool) -> Result<(usize, usize)> {
    let protected = build_globset(&policy.protected_patterns)?;
    let files = git_ls_files(repo)?;

    let mut found = 0usize;
    let mut changed = 0usize;
    let warden = if apply {
        Some(DraconWarden::new()?)
    } else {
        None
    };

    for rel in files {
        let rel_norm = rel.replace("\\", "/");
        if !protected.is_match(&rel_norm) {
            continue;
        }

        let full = repo.join(&rel);
        let bytes = match fs::read(&full) {
            Ok(b) => b,
            Err(_) => continue,
        };

        if !is_marker_string(&String::from_utf8_lossy(&bytes)) {
            continue;
        }

        found += 1;

        if !apply {
            println!("🔎 ciphertext in worktree: {}", full.display());
            continue;
        }

        let Some(warden) = &warden else {
            continue;
        };

        match warden.smudge(&bytes, Some(&rel_norm)) {
            Ok(out) => {
                if out != bytes {
                    if let Err(e) = fs::write(&full, out) {
                        eprintln!("⚠️ resmudge write failed {}: {}", full.display(), e);
                        continue;
                    }
                    changed += 1;
                    println!("✅ resmudged: {}", full.display());
                }
            }
            Err(e) => {
                eprintln!("⚠️ resmudge failed {}: {}", full.display(), e);
            }
        }
    }

    Ok((found, changed))
}

fn resmudge_repos(policy: &WardenPolicy, repos: &[PathBuf], apply: bool) -> Result<(usize, usize)> {
    policy.validate()?;

    let mut total_found = 0usize;
    let mut total_changed = 0usize;

    for repo in repos {
        match resmudge_repo(repo, policy, apply) {
            Ok((found, changed)) => {
                total_found += found;
                total_changed += changed;
            }
            Err(e) => eprintln!("⚠️ resmudge failed for {}: {}", repo.display(), e),
        }
    }

    if apply {
        println!(
            "✅ resmudge complete (found: {}, changed: {})",
            total_found, total_changed
        );
    } else {
        println!("✅ resmudge report complete (found: {})", total_found);
    }

    Ok((total_found, total_changed))
}

fn is_env_file_name(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    path_lower.ends_with(".env")
        || path_lower.contains(".env.")
        || path_lower.ends_with(".envrc")
        || path_lower.ends_with("/.env")
        || path_lower.ends_with("/.envrc")
}

fn is_encrypted_env_content(content: &str) -> bool {
    let trimmed = content.trim_end_matches('\n');
    trimmed.starts_with("[DRACON_SECRET:") && trimmed.ends_with(']')
}

fn backfill_env_headers_repo(repo: &Path, apply: bool) -> Result<(usize, usize)> {
    let files = git_ls_files(repo)?;
    let warden = DraconWarden::new()?;

    let mut found = 0usize;
    let mut changed = 0usize;

    for rel in files {
        let rel_norm = rel.replace("\\", "/");
        if !is_env_file_name(&rel_norm) {
            continue;
        }

        let full = repo.join(&rel);
        let bytes = match fs::read(&full) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let content = String::from_utf8_lossy(&bytes);
        if content.contains("Dracon Warden") {
            continue;
        }

        let is_encrypted = is_encrypted_env_content(&content);
        found += 1;

        if !apply {
            if is_encrypted {
                println!("🔎 .env without header (encrypted, skipping): {}", full.display());
            } else {
                println!("🔎 .env without header: {}", full.display());
            }
            continue;
        }

        if is_encrypted {
            eprintln!("⚠️ refusing to decrypt encrypted file during header backfill: {}", full.display());
            continue;
        }

        match warden.smudge(&bytes, Some(&rel_norm)) {
            Ok(out) => {
                if out != bytes {
                    if let Err(e) = fs::write(&full, &out) {
                        eprintln!("⚠️ backfill write failed {}: {}", full.display(), e);
                        continue;
                    }
                    changed += 1;
                    println!("✅ header added: {}", full.display());
                }
            }
            Err(e) => {
                eprintln!("⚠️ backfill failed {}: {}", full.display(), e);
            }
        }
    }

    Ok((found, changed))
}

fn backfill_env_headers_repos(repos: &[PathBuf], apply: bool) -> Result<(usize, usize)> {
    let mut total_found = 0usize;
    let mut total_changed = 0usize;

    for repo in repos {
        match backfill_env_headers_repo(repo, apply) {
            Ok((found, changed)) => {
                total_found += found;
                total_changed += changed;
            }
            Err(e) => eprintln!("⚠️ backfill failed for {}: {}", repo.display(), e),
        }
    }

    if apply {
        println!(
            "✅ backfill complete (found: {}, changed: {})",
            total_found, total_changed
        );
    } else {
        println!("✅ backfill report complete (found: {})", total_found);
    }

    Ok((total_found, total_changed))
}

fn run_filter(is_clean: bool, path: Option<&str>) -> Result<()> {
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input)?;
    let warden = DraconWarden::new()?;
    let output = if is_clean {
        warden.clean(&input, path)?
    } else {
        warden.smudge(&input, path)?
    };
    std::io::stdout().write_all(&output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    static HOME_MUTEX: Mutex<()> = Mutex::new(());

    struct TestDir {
        path: std::path::PathBuf,
        #[allow(dead_code)]
        guard: Mutex<()>,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let tmp = std::env::temp_dir();
            let path = tmp.join(format!("dracon_warden_test_{}_{}", name, id));
            fs::create_dir_all(&path).expect("create temp dir");
            Self {
                path,
                guard: Mutex::new(()),
            }
        }
        fn path(&self) -> &std::path::Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn sample_policy() -> WardenPolicy {
        WardenPolicy {
            protected_patterns: vec!["*.env".into(), "secrets/**".into()],
            plaintext_patterns: vec!["*.pub".into()],
            hygiene_patterns: vec!["target/".into(), "*.log".into()],
            watch_roots: vec![],
            discover_roots: vec![],
        }
    }

    #[test]
    fn replace_managed_block_appends_when_missing() {
        let current = "a=1\n";
        let block = format!("{BLOCK_BEGIN}\nmanaged\n{BLOCK_END}");
        let next = replace_managed_block(current, &block);
        assert!(next.contains("a=1"));
        assert!(next.contains("managed"));
        assert!(next.contains(BLOCK_BEGIN));
        assert!(next.contains(BLOCK_END));
    }

    #[test]
    fn replace_managed_block_replaces_existing_and_keeps_tail() {
        let current = format!("head\n{BLOCK_BEGIN}\nold\n{BLOCK_END}\n\nend\n");
        let block = format!("{BLOCK_BEGIN}\nnew\n{BLOCK_END}");
        let next = replace_managed_block(&current, &block);
        assert!(next.contains("head"));
        assert!(next.contains("new"));
        assert!(!next.contains("old"));
        assert!(next.contains("end"));
    }

    #[test]
    fn build_gitignore_block_includes_expected_lines() {
        let block = build_gitignore_block(&sample_policy()).expect("block");
        assert!(block.contains(BLOCK_BEGIN));
        assert!(block.contains("target/"));
        assert!(block.contains("!*.env"));
        assert!(block.contains("!secrets/**"));
        assert!(block.contains("!*.pub"));
        assert!(!block.contains("!config/licenses.json"));
        assert!(!block.contains("!config/services.test.json"));
        assert!(!block.contains("!plan/pages/templates/*.json"));
        assert!(block.contains(BLOCK_END));
    }

    #[test]
    fn build_gitattributes_block_includes_expected_lines() {
        let block = build_gitattributes_block(&sample_policy()).expect("block");
        assert!(block.contains("*.env filter=dracon"));
        assert!(block.contains("secrets/** filter=dracon"));
        assert!(block.contains("*.pub -filter -diff -merge"));
        assert!(!block.contains("config/licenses.json -filter -diff -merge"));
        assert!(!block.contains("config/services.test.json -filter -diff -merge"));
        assert!(!block.contains("plan/pages/templates/*.json -filter -diff -merge"));
    }

    #[test]
    fn plaintext_cannot_overlap_protected_or_disable_env_encryption() {
        let policy = WardenPolicy {
            protected_patterns: vec!["config/envs/*.env".into(), "*.env".into()],
            plaintext_patterns: vec!["config/envs/*.env".into()],
            hygiene_patterns: vec![],
            watch_roots: vec![],
            discover_roots: vec![],
        };
        assert!(build_gitattributes_block(&policy).is_err());
    }

    #[test]
    fn repos_for_event_ignores_target_and_maps_repo_root() {
        let td = TestDir::new("warden_event_repo_root");
        let repo = td.path().join("repo");
        fs::create_dir_all(repo.join(".git")).expect("git dir");
        fs::create_dir_all(repo.join("src")).expect("src");
        fs::create_dir_all(repo.join("target")).expect("target");
        let roots = vec![td.path().to_path_buf()];

        let ev = Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Any),
            paths: vec![repo.join("src/main.rs"), repo.join("target/tmp.o")],
            attrs: notify::event::EventAttributes::default(),
        };
        let repos = repos_for_event(&ev, &roots);
        assert_eq!(repos.len(), 1);
        assert!(repos.contains(&repo));
    }

    #[test]
    fn owner_pubkeys_in_filters_only_owner_pub() {
        let td = TestDir::new("warden_owner_pubkeys");
        fs::write(td.path().join("owner_a.pub"), "a").expect("write");
        fs::write(td.path().join("owner_a.key"), "a").expect("write");
        fs::write(td.path().join("identity.pub"), "a").expect("write");
        let keys = owner_pubkeys_in(td.path());
        assert_eq!(keys.len(), 1);
        assert_eq!(
            keys[0].file_name().and_then(|n| n.to_str()),
            Some("owner_a.pub")
        );
    }

    #[test]
    fn newest_file_picks_newest_existing() {
        let td = TestDir::new("warden_newest");
        let a = td.path().join("a.pub");
        let b = td.path().join("b.pub");
        fs::write(&a, "a").expect("write a");
        std::thread::sleep(Duration::from_secs(1));
        fs::write(&b, "b").expect("write b");
        let picked = newest_file(vec![a.clone(), b.clone()]).expect("picked");
        assert_eq!(picked, b);
    }

    #[test]
    fn publish_repo_pubkey_writes_and_is_idempotent() {
        let td = TestDir::new("warden_publish_key");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");
        let key = td.path().join("owner_test.pub");
        fs::write(&key, "age1xxx").expect("key");

        assert!(publish_repo_pubkey(&repo, &key).expect("first publish"));
        assert!(!publish_repo_pubkey(&repo, &key).expect("second publish"));
        let out = repo.join(".dracon/data/keys/owner_test.pub");
        assert_eq!(fs::read_to_string(out).expect("read out"), "age1xxx");
    }

    #[test]
    fn harden_repo_changes_files_and_writes_key() {
        let td = TestDir::new("warden_harden_repo");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");
        let key = td.path().join("owner_test.pub");
        fs::write(&key, "age1yyy").expect("key");

        let status = ProcessCommand::new("git")
            .arg("init")
            .arg(&repo)
            .status()
            .expect("git init");
        assert!(status.success(), "git init should succeed");

        let (a, b, c) = harden_repo(&repo, &sample_policy(), Some(&key)).expect("harden");
        assert!(a, "gitignore should be written");
        assert!(b, ".gitattributes should be written");
        assert!(c, "pubkey should be published");
        assert!(repo.join(".gitignore").exists());
        assert!(repo.join(".gitattributes").exists());
        assert!(repo.join(".dracon/data/keys/owner_test.pub").exists());
    }

    #[test]
    fn harden_repo_sets_local_dracon_filter_config() {
        let td = TestDir::new("warden_harden_repo_filter_cfg");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");
        let status = ProcessCommand::new("git")
            .arg("init")
            .arg(&repo)
            .status()
            .expect("git init");
        assert!(status.success());

        let (_a, b, _c) = harden_repo(&repo, &sample_policy(), None).expect("harden");
        assert!(b);

        let clean = ProcessCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .arg("config")
            .arg("--local")
            .arg("--get")
            .arg("filter.dracon.clean")
            .output()
            .expect("get clean");
        assert!(clean.status.success());
        assert_eq!(
            String::from_utf8_lossy(&clean.stdout).trim(),
            "dracon-warden filter-clean %f"
        );

        let smudge = ProcessCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .arg("config")
            .arg("--local")
            .arg("--get")
            .arg("filter.dracon.smudge")
            .output()
            .expect("get smudge");
        assert!(smudge.status.success());
        assert_eq!(
            String::from_utf8_lossy(&smudge.stdout).trim(),
            "dracon-warden filter-smudge %f"
        );

        let required = ProcessCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .arg("config")
            .arg("--local")
            .arg("--get")
            .arg("filter.dracon.required")
            .output()
            .expect("get required");
        assert!(required.status.success());
        assert_eq!(String::from_utf8_lossy(&required.stdout).trim(), "true");
    }

    #[test]
    fn publish_repo_pubkey_rejects_non_owner_or_secret_key_material() {
        let td = TestDir::new("warden_publish_key_rejects");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");

        let not_owner = td.path().join("identity.pub");
        fs::write(&not_owner, "age1xxx").expect("write");
        assert!(publish_repo_pubkey(&repo, &not_owner).is_err());

        let secret = td.path().join("owner_secret.pub");
        fs::write(&secret, "AGE-SECRET-KEY-1XXXX").expect("write");
        assert!(publish_repo_pubkey(&repo, &secret).is_err());
    }

    #[test]
    fn salvage_invalid_json_replaces_marker_tokens_and_parses() {
        let a = "{[DRACON_SECRET:abc]: \"x\"}";
        let salvaged = salvage_invalid_json_markers(a).expect("salvaged");
        let v: serde_json::Value = serde_json::from_str(&salvaged).expect("parse");
        assert_eq!(
            v["__scrubbed__"],
            serde_json::Value::String("x".to_string())
        );

        let b = "{ \"track_id\": [DRACON_SECRET:abc], \"x\": 1 }";
        let salvaged = salvage_invalid_json_markers(b).expect("salvaged");
        let v: serde_json::Value = serde_json::from_str(&salvaged).expect("parse");
        assert!(v["track_id"].is_null());
        assert_eq!(v["x"], serde_json::Value::from(1));
    }

    #[test]
    fn effective_watch_roots_merges_and_dedupes() {
        let td = TestDir::new("warden_effective_roots");
        let p1 = td.path().join("one");
        fs::create_dir_all(&p1).expect("p1");

        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec![],
            hygiene_patterns: vec![],
            watch_roots: vec![p1.display().to_string(), p1.display().to_string()],
            discover_roots: vec![],
        };
        let merged = effective_watch_roots(&policy);
        assert_eq!(merged.len(), 1);
        assert!(merged.contains(&p1));
    }

    #[test]
    fn effective_discovery_roots_merges_watch_and_discover_deduped() {
        let td = TestDir::new("warden_effective_discovery_roots");
        let p1 = td.path().join("one");
        let p2 = td.path().join("two");
        fs::create_dir_all(&p1).expect("p1");
        fs::create_dir_all(&p2).expect("p2");

        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec![],
            hygiene_patterns: vec![],
            watch_roots: vec![p1.display().to_string()],
            discover_roots: vec![p1.display().to_string(), p2.display().to_string()],
        };
        let merged = effective_discovery_roots(&policy);
        assert_eq!(merged.len(), 2);
        assert!(merged.contains(&p1));
        assert!(merged.contains(&p2));
    }

    #[test]
    fn apply_managed_file_detects_noop_second_write() {
        let td = TestDir::new("warden_apply_noop");
        let file = td.path().join(".gitignore");
        let block = format!("{BLOCK_BEGIN}\nfoo\n{BLOCK_END}");
        assert!(apply_managed_file(&file, &block).expect("first"));
        assert!(!apply_managed_file(&file, &block).expect("second"));
    }

    #[test]
    fn apply_overwrite_file_detects_noop_second_write() {
        let td = TestDir::new("warden_apply_overwrite_noop");
        let file = td.path().join(".gitattributes");
        let body = "a\nb\n";
        assert!(apply_overwrite_file(&file, body).expect("first"));
        assert!(!apply_overwrite_file(&file, body).expect("second"));
    }

    #[test]
    fn repeated_replace_block_scenarios_are_stable() {
        for idx in 0..200usize {
            let current = if idx % 2 == 0 {
                format!("prefix-{idx}\n")
            } else {
                format!("prefix-{idx}\n{BLOCK_BEGIN}\nold\n{BLOCK_END}\n")
            };
            let block = format!("{BLOCK_BEGIN}\nnew-{idx}\n{BLOCK_END}");
            let next = replace_managed_block(&current, &block);
            assert!(next.contains(&format!("new-{idx}")));
            assert!(next.contains(BLOCK_BEGIN));
            assert!(next.contains(BLOCK_END));
        }
    }

    #[test]
    fn resolve_policy_path_local_finds_temp_config() {
        let td = TestDir::new("warden_policy_path");
        let config_dir = td.path().join(".dracon").join("utilities").join("warden");
        fs::create_dir_all(&config_dir).expect("create config dir");
        let config_path = config_dir.join("dracon-warden.toml");
        fs::write(
            &config_path,
            r#"
[watch]
watch_roots = ["/tmp/test"]
"#,
        )
        .expect("write config");

        std::env::set_var("DRACON_WARDEN_POLICY", config_path.display().to_string());
        let path = resolve_policy_path_local().expect("should resolve");
        std::env::remove_var("DRACON_WARDEN_POLICY");

        assert_eq!(path, config_path);
    }

    #[test]
    fn resolve_policy_path_local_falls_back_to_default_locations() {
        let td = TestDir::new("warden_policy_default");
        let config_dir = td.path().join(".dracon").join("utilities").join("warden");
        fs::create_dir_all(&config_dir).expect("create config dir");
        let config_path = config_dir.join("dracon-warden.toml");
        fs::write(
            &config_path,
            r#"
[watch]
watch_roots = ["/tmp/test"]
"#,
        )
        .expect("write config");

        let _lock = HOME_MUTEX.lock().expect("home mutex poisoned");
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", td.path().to_str().unwrap());
        let path = resolve_policy_path_local();
        std::env::remove_var("HOME");
        if let Some(h) = original_home {
            std::env::set_var("HOME", h);
        }

        assert!(path.is_ok(), "should find config in default location");
    }

    #[test]
    fn marker_prefix_at_finds_correct_positions() {
        let s = "prefix [DRACON_SECRET:abc] after";
        assert_eq!(marker_prefix_at(s, 7), Some("[DRACON_SECRET:"));

        let s2 = "prefix [DRACON_SECRET:xyz] after";
        assert_eq!(marker_prefix_at(s2, 7), Some("[DRACON_SECRET:"));

        let s3 = "no marker here";
        assert_eq!(marker_prefix_at(s3, 0), None);
    }

    #[test]
    fn is_marker_string_detects_both_markers() {
        assert!(is_marker_string("hello [DRACON_SECRET:xyz] world"));
        assert!(!is_marker_string("hello world"));
        assert!(!is_marker_string("DRACON_SECRET not in brackets"));
        assert!(!is_marker_string("[WRONG_SECRET:abc]"));
    }

    #[test]
    fn build_gitignore_block_includes_demon_directives() {
        let block = build_gitignore_block(&sample_policy()).expect("block");
        assert!(block.contains("# --- BEGIN DRACON MANAGED BLOCK ---"));
        assert!(block.contains("target/"));
        assert!(block.contains("*.log"));
    }

    #[test]
    fn build_gitattributes_block_sets_filter_for_env() {
        let block = build_gitattributes_block(&sample_policy()).expect("block");
        assert!(block.contains("*.env filter=dracon"));
        assert!(block.contains("secrets/** filter=dracon"));
    }

    #[test]
    fn discover_git_repos_finds_all_git_dirs() {
        let td = TestDir::new("warden_discover_all");
        let root = td.path().join("root");
        fs::create_dir_all(&root).expect("root");

        let repo1 = root.join("my_repo");
        fs::create_dir_all(repo1.join(".git")).expect("my_repo .git");

        let repo2 = root.join("other_repo");
        fs::create_dir_all(repo2.join(".git")).expect("other_repo .git");

        let repos = discover_git_repos(&[root], &BTreeSet::new());

        assert!(repos.contains(&repo1), "my_repo should be found");
        assert!(repos.contains(&repo2), "other_repo should be found");
    }

    #[test]
    fn discover_git_repos_local_finds_basic_repos() {
        let td = TestDir::new("warden_discover_local");
        let root = td.path().join("root");
        fs::create_dir_all(&root).expect("root");

        let repo1 = root.join("repo1");
        fs::create_dir_all(repo1.join(".git")).expect("repo1 .git");

        let repo2 = root.join("repo2");
        fs::create_dir_all(repo2.join(".git")).expect("repo2 .git");

        let repos = discover_git_repos_local(&[root]);

        assert!(repos.contains(&repo1), "repo1 should be found");
        assert!(repos.contains(&repo2), "repo2 should be found");
    }

    #[test]
    fn filter_smudge_handles_empty_input() {
        let content = "let x = 1;\n";
        let warden = DraconWarden::new().expect("create warden");
        let result = warden.smudge(content.as_bytes(), None).expect("smudge");
        assert_eq!(
            result,
            content.as_bytes(),
            "plaintext should pass through unchanged"
        );
    }

    #[test]
    fn replace_managed_block_empty_current_string() {
        let current = "";
        let block = format!("{BLOCK_BEGIN}\nnewcontent\n{BLOCK_END}");
        let next = replace_managed_block(current, &block);
        assert!(next.contains("newcontent"));
        assert!(next.contains(BLOCK_BEGIN));
        assert!(next.contains(BLOCK_END));
    }

    #[test]
    fn replace_managed_block_multiple_blocks_replaces_all() {
        let current = format!(
            "prefix\n{BLOCK_BEGIN}\nfirst\n{BLOCK_END}\nmid\n{BLOCK_BEGIN}\nsecond\n{BLOCK_END}\n suffix\n"
        );
        let block = format!("{BLOCK_BEGIN}\nnew\n{BLOCK_END}");
        let next = replace_managed_block(&current, &block);
        assert!(next.contains("prefix"));
        assert!(next.contains("new"));
        assert!(!next.contains("first"), "first block content should be replaced");
        assert!(!next.contains("second"), "second block content should be replaced");
        assert!(next.contains("mid"));
        assert!(next.contains(" suffix"));
    }

    #[test]
    fn replace_managed_block_preserves_leading_whitespace() {
        let current = "  prefix\n";
        let block = format!("{BLOCK_BEGIN}\nmanaged\n{BLOCK_END}");
        let next = replace_managed_block(current, &block);
        assert!(next.starts_with("  prefix\n"), "leading content should be preserved");
    }

    #[test]
    fn apply_managed_file_creates_parent_dirs() {
        let td = TestDir::new("warden_apply_creates_dirs");
        let nested = td.path().join("a/b/c/managed.txt");
        let block = format!("{BLOCK_BEGIN}\ncontent\n{BLOCK_END}");
        let result = apply_managed_file(&nested, &block);
        assert!(result.is_ok(), "should create parent dirs");
        assert!(nested.exists(), "file should exist");
        std::fs::remove_dir_all(td.path()).ok();
    }

    #[test]
    fn apply_overwrite_file_creates_new_file() {
        let td = TestDir::new("warden_overwrite_new");
        let file = td.path().join("newfile.txt");
        let result = apply_overwrite_file(&file, "hello world");
        assert!(result.is_ok(), "should create new file");
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.starts_with("hello world"), "should contain content: {:?}", content);
        std::fs::remove_dir_all(td.path()).ok();
    }

    #[test]
    fn apply_overwrite_file_overwrites_existing() {
        let td = TestDir::new("warden_overwrite_existing");
        let file = td.path().join("existing.txt");
        std::fs::write(&file, "old content").unwrap();
        let result = apply_overwrite_file(&file, "new content");
        assert!(result.is_ok(), "should overwrite");
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.starts_with("new content"), "should contain new content: {:?}", content);
        std::fs::remove_dir_all(td.path()).ok();
    }

    #[test]
    fn is_marker_string_edge_cases() {
        assert!(!is_marker_string(""), "empty string should not match");
        assert!(!is_marker_string("[DRACON_SECRET]"), "no colon");
        assert!(!is_marker_string("DRACON_SECRET not in brackets"), "not in brackets");
        assert!(!is_marker_string("[WRONG_SECRET:abc]"), "wrong prefix");
        assert!(is_marker_string("[DRACON_SECRET:]"), "empty key is still a marker");
        assert!(is_marker_string("[DRACON_SECRET: ]"), "space key is still a marker");
        assert!(is_marker_string("[DRACON_SECRET:abc123]"), "basic key");
        assert!(is_marker_string("[DRACON_SECRET:abc-123_456]"), "key with dash underscore");
    }

    #[test]
    fn marker_prefix_at_edge_cases() {
        assert_eq!(marker_prefix_at("no bracket here", 0), None);
        assert_eq!(marker_prefix_at("[DRACON_SECRET:abc]", 0), Some("[DRACON_SECRET:"), "starts at position 0");
        assert_eq!(marker_prefix_at("[DRACON_SECRET:abc]", 1), None, "starts at position 1");
        assert_eq!(marker_prefix_at("prefix [DRACON_SECRET", 8), None, "incomplete bracket without colon");
        assert_eq!(marker_prefix_at("[DRACON_SECRET:abc] more", 0), Some("[DRACON_SECRET:"), "marker at start followed by more");
        assert_eq!(marker_prefix_at("text [DRACON_SECRET:abc] end", 5), Some("[DRACON_SECRET:"), "at position 5 [ bracket is at position 5");
    }

    #[test]
    fn salvage_invalid_json_no_marker_returns_none() {
        assert!(salvage_invalid_json_markers("just normal json").is_none());
        assert!(salvage_invalid_json_markers("").is_none());
        assert!(salvage_invalid_json_markers("[DRACON_SECRE").is_none(), "incomplete marker should return None");
    }

    #[test]
    fn salvage_invalid_json_marker_at_end_of_string() {
        let input = r#"{"key": "value", "secret": "[DRACON_SECRET:abc]"}"#;
        let salvaged = salvage_invalid_json_markers(input).expect("should salvage");
        assert!(salvaged.contains("null") || salvaged.contains("__scrubbed__"));
    }

    #[test]
    fn salvage_invalid_json_markers_multiple_in_sequence() {
        let input = r#"{"a": [DRACON_SECRET:x], "b": [DRACON_SECRET:y], "c": "normal"}"#;
        let salvaged = salvage_invalid_json_markers(input).expect("should salvage");
        assert!(salvaged.contains("null") || salvaged.contains("__scrubbed__"));
        assert!(salvaged.contains("normal"));
    }

    #[test]
    fn salvage_invalid_json_handles_nested_markers() {
        let input = r#"{"key": "[DRACON_SECRET:abc]", "nested": {"key": "[DRACON_SECRET:xyz]"}}"#;
        let salvaged = salvage_invalid_json_markers(input).expect("should salvage");
        let v: serde_json::Value = serde_json::from_str(&salvaged).expect("should parse");
        assert!(v["key"].is_null() || v["key"].is_string());
    }

    #[test]
    fn effective_watch_roots_handles_empty_policy() {
        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec![],
            hygiene_patterns: vec![],
            watch_roots: vec![],
            discover_roots: vec![],
        };
        let roots = effective_watch_roots(&policy);
        assert!(roots.is_empty());
    }

    #[test]
    fn effective_discovery_roots_handles_empty_policy() {
        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec![],
            hygiene_patterns: vec![],
            watch_roots: vec![],
            discover_roots: vec![],
        };
        let roots = effective_discovery_roots(&policy);
        assert!(roots.is_empty());
    }

    #[test]
    fn build_globset_empty_patterns_returns_empty_set() {
        let set = build_globset(&[]).expect("should succeed");
        assert!(set.is_empty());
    }

    #[test]
    fn build_globset_single_pattern_matches() {
        let set = build_globset(&["*.json".into()]).expect("should succeed");
        assert!(set.is_match("test.json"));
        assert!(!set.is_match("test.txt"));
    }

    #[test]
    fn build_globset_multiple_patterns() {
        let set = build_globset(&["*.json".into(), "*.toml".into()]).expect("should succeed");
        assert!(set.is_match("test.json"));
        assert!(set.is_match("test.toml"));
        assert!(!set.is_match("test.txt"));
    }

    #[test]
    fn build_globset_invalid_pattern_returns_error() {
        let result = build_globset(&["[".into()]);
        assert!(result.is_err(), "invalid glob pattern should return error");
    }

    #[test]
    fn build_globset_normalizes_backslash() {
        let set = build_globset(&["subdir\\*.json".into()]).expect("should succeed");
        assert!(set.is_match("subdir/test.json"));
    }

    #[test]
    fn run_keygen_generates_keypair_successfully() {
        let td = TestDir::new("warden_keygen_success");
        let keys_dir = td.path().join(".dracon").join("data").join("keys");

        let _lock = HOME_MUTEX.lock().expect("home mutex poisoned");
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", td.path().to_str().unwrap());

        let result = run_keygen();

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        assert!(result.is_ok(), "keygen should succeed: {:?}", result);
        let hostname_raw = hostname::get().expect("hostname").to_string_lossy().to_string();
        let hostname: String = hostname_raw.chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        let secret_path = keys_dir.join(format!("machine_{}.age", hostname));
        let pubkey_path = keys_dir.join(format!("owner_{}.pub", hostname));
        assert!(secret_path.exists(), "secret key should be created at {}", secret_path.display());
        assert!(pubkey_path.exists(), "pubkey should be created at {}", pubkey_path.display());
    }

    #[test]
    fn run_keygen_refuses_to_overwrite_existing_secret_key() {
        let td = TestDir::new("warden_keygen_secret_exists");
        let keys_dir = td.path().join(".dracon").join("data").join("keys");
        std::fs::create_dir_all(&keys_dir).unwrap();

        let _lock = HOME_MUTEX.lock().expect("home mutex poisoned");
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", td.path().to_str().unwrap());

        let hostname_raw = hostname::get().expect("hostname").to_string_lossy().to_string();
        let hostname: String = hostname_raw.chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        let fake_secret = keys_dir.join(format!("machine_{}.age", hostname));
        std::fs::write(&fake_secret, "already exists").unwrap();

        let result = run_keygen();

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        assert!(result.is_err(), "should refuse to overwrite existing secret key");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("already exists"), "error should mention already exists: {}", err_msg);
    }

    #[test]
    fn run_keygen_refuses_to_overwrite_existing_pubkey() {
        let td = TestDir::new("warden_keygen_pubkey_exists");
        let keys_dir = td.path().join(".dracon").join("data").join("keys");
        std::fs::create_dir_all(&keys_dir).unwrap();

        let _lock = HOME_MUTEX.lock().expect("home mutex poisoned");
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", td.path().to_str().unwrap());

        let hostname_raw = hostname::get().expect("hostname").to_string_lossy().to_string();
        let hostname: String = hostname_raw.chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        let fake_pubkey = keys_dir.join(format!("owner_{}.pub", hostname));
        std::fs::write(&fake_pubkey, "already exists").unwrap();

        let result = run_keygen();

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        assert!(result.is_err(), "should refuse to overwrite existing pubkey");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("already exists") || err_msg.contains("file may already exist"),
            "error should mention already exists: {}", err_msg);
    }
}
