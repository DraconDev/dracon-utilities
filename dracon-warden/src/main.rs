use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dracon_security_kit::{DraconWarden, Warden};
use notify::{Event, RecursiveMode, Watcher};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const BLOCK_BEGIN: &str = "# --- BEGIN DRACON MANAGED BLOCK ---";
const BLOCK_END: &str = "# --- END DRACON MANAGED BLOCK ---";
const DEFAULT_PLAINTEXT_PATTERNS: &[&str] = &[
    "config/envs/*.env",
    "config/licenses.json",
    "config/licenses.test.json",
    "config/services.json",
    "config/services.test.json",
    "plan/pages/snapshots/*.json",
    "plan/pages/templates/*.json",
];

#[derive(Parser, Debug)]
#[command(name = "dracon-warden")]
#[command(about = "Lightweight Warden runtime")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run forever with filesystem event debounce.
    Daemon,
    /// Run one hardening pass and exit.
    Once,
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
}

#[derive(Debug, Deserialize, Default, Clone)]
struct SyncRootsPolicy {
    #[serde(default)]
    watch_roots: Vec<String>,
}

impl WardenPolicy {
    fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read policy {}", path.display()))?;
        let policy: Self = toml::from_str(&content)
            .with_context(|| format!("failed to parse policy {}", path.display()))?;
        Ok(policy)
    }

    fn watch_root_paths(&self) -> Vec<PathBuf> {
        self.watch_roots
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect()
    }
}

fn resolve_policy_path() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("DRACON_WARDEN_POLICY") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Ok(p);
        }
    }

    if let Ok(custom) = std::env::var("DRACON_SECURITY_POLICY") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Ok(p);
        }
    }

    let home = dirs::home_dir().context("home not found")?;
    let candidates = [
        home.join("dracon/utilities/warden/dracon-warden.toml"),
        home.join("dracon/utilities/warden/dracon-security.toml"),
        home.join("dracon/utilities/warden/config.toml"),
        home.join("dracon/security/dracon-security.toml"),
    ];

    for p in &candidates {
        if p.exists() {
            return Ok(p.clone());
        }
    }

    Err(anyhow::anyhow!(
        "policy not found. checked: {} (or DRACON_WARDEN_POLICY/DRACON_SECURITY_POLICY)",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn discover_git_repos(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut repos = BTreeSet::new();

    for root in roots {
        if root.join(".git").exists() {
            repos.insert(root.clone());
        }

        let walker = walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                if e.depth() == 0 {
                    return true;
                }
                if name == "target"
                    || name == "node_modules"
                    || name == ".cache"
                    || name == ".direnv"
                {
                    return false;
                }
                true
            });

        for entry in walker.filter_map(|e| e.ok()) {
            if !entry.file_type().is_dir() {
                continue;
            }
            if entry.file_name() == ".git" {
                if let Some(parent) = entry.path().parent() {
                    repos.insert(parent.to_path_buf());
                }
            }
        }
    }

    repos.into_iter().collect()
}

fn resolve_sync_policy_path() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("DRACON_SYNC_POLICY") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Some(p);
        }
    }

    let home = dirs::home_dir()?;
    let candidates = [
        home.join("dracon/utilities/sync/dracon-sync.toml"),
        home.join("dracon/utilities/sync/config.toml"),
        home.join("dracon/git/dracon-git.toml"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

fn load_sync_watch_roots() -> Vec<PathBuf> {
    let Some(path) = resolve_sync_policy_path() else {
        return Vec::new();
    };
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(policy) = toml::from_str::<SyncRootsPolicy>(&content) else {
        return Vec::new();
    };
    policy
        .watch_roots
        .into_iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect()
}

fn effective_watch_roots(policy: &WardenPolicy) -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    for root in policy.watch_root_paths() {
        roots.insert(root);
    }
    for root in load_sync_watch_roots() {
        roots.insert(root);
    }
    roots.into_iter().collect()
}

fn replace_managed_block(current: &str, managed_block: &str) -> String {
    if let Some(start) = current.find(BLOCK_BEGIN) {
        if let Some(end_rel) = current[start..].find(BLOCK_END) {
            let end = start + end_rel + BLOCK_END.len();
            let tail = current[end..].trim_start_matches(&['\r', '\n'][..]);
            let mut out = String::new();
            out.push_str(&current[..start]);
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
    }

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

fn build_gitignore_block(policy: &WardenPolicy) -> String {
    let mut lines = Vec::new();
    lines.push(BLOCK_BEGIN.to_string());
    lines.push("# managed by dracon-warden".to_string());
    for p in &policy.hygiene_patterns {
        lines.push(p.clone());
    }
    let mut plaintext_patterns = BTreeSet::new();
    for p in &policy.plaintext_patterns {
        plaintext_patterns.insert(p.clone());
    }
    for p in DEFAULT_PLAINTEXT_PATTERNS {
        plaintext_patterns.insert((*p).to_string());
    }
    for p in &policy.protected_patterns {
        lines.push(format!("!{}", p));
    }
    for p in plaintext_patterns {
        lines.push(format!("!{}", p));
    }
    lines.push(BLOCK_END.to_string());
    lines.join("\n")
}

fn build_gitattributes_block(policy: &WardenPolicy) -> String {
    let mut lines = Vec::new();
    lines.push(BLOCK_BEGIN.to_string());
    lines.push("# managed by dracon-warden".to_string());
    let mut plaintext_patterns = BTreeSet::new();
    for p in &policy.plaintext_patterns {
        plaintext_patterns.insert(p.clone());
    }
    for p in DEFAULT_PLAINTEXT_PATTERNS {
        plaintext_patterns.insert((*p).to_string());
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
    lines.join("\n")
}

fn normalize_filter_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn should_passthrough_filter_path(path: Option<&str>) -> bool {
    let Some(path) = path else {
        return false;
    };
    let normalized = normalize_filter_path(path);
    normalized.starts_with("config/envs/")
}

fn apply_managed_file(path: &Path, block: &str) -> Result<bool> {
    let current = fs::read_to_string(path).unwrap_or_default();
    let next = replace_managed_block(&current, block);
    if next != current {
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
        fs::write(path, next).with_context(|| format!("failed writing {}", path.display()))?;
        return Ok(true);
    }
    Ok(false)
}

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
    with_mtime.sort_by(|a, b| b.0.cmp(&a.0));
    with_mtime.into_iter().next().map(|(_, p)| p)
}

fn owner_pubkeys_in(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return out,
    };

    for entry in read_dir.flatten() {
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

fn resolve_local_pubkey_path() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("DRACON_OWNER_PUBKEY") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Some(p);
        }
    }

    let home = dirs::home_dir()?;
    let owner_candidates = [
        home.join("dracon/data/keys"),
        home.join(".demon/keys"),
        home.join("dracon/keys"),
    ]
    .into_iter()
    .flat_map(|dir| owner_pubkeys_in(&dir))
    .collect::<Vec<_>>();

    if let Some(newest_owner) = newest_file(owner_candidates) {
        return Some(newest_owner);
    }

    let identity_candidates = vec![
        home.join("dracon/identity.pub"),
        home.join(".demon/identity.pub"),
    ];
    newest_file(identity_candidates)
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
    let current_bytes = fs::read(&target).ok();
    if current_bytes.as_deref() == Some(source_bytes.as_slice()) {
        return Ok(false);
    }

    fs::write(&target, source_bytes)
        .with_context(|| format!("failed writing {}", target.display()))?;
    Ok(true)
}

fn harden_repo(
    repo: &Path,
    policy: &WardenPolicy,
    pubkey_path: Option<&Path>,
) -> Result<(bool, bool, bool)> {
    let gitignore_path = repo.join(".gitignore");
    let gitattributes_path = repo.join(".gitattributes");

    let gitignore_changed = apply_managed_file(&gitignore_path, &build_gitignore_block(policy))?;
    let gitattributes_changed =
        apply_overwrite_file(&gitattributes_path, &build_gitattributes_block(policy))?;
    let key_changed = match pubkey_path {
        Some(pubkey) => publish_repo_pubkey(repo, pubkey)?,
        None => false,
    };

    Ok((gitignore_changed, gitattributes_changed, key_changed))
}

fn harden_all(policy: &WardenPolicy) -> Result<()> {
    let roots = effective_watch_roots(policy);
    let repos = discover_git_repos(&roots);
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
                }
            }
            Err(e) => eprintln!("⚠️ harden failed for {}: {}", repo.display(), e),
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

fn run_daemon(policy_path: PathBuf) -> Result<()> {
    let policy = WardenPolicy::load(&policy_path)?;
    let roots = effective_watch_roots(&policy);
    if roots.is_empty() {
        return Err(anyhow::anyhow!("no valid watch_roots in policy"));
    }

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;

    for root in &roots {
        watcher.watch(root, RecursiveMode::Recursive)?;
    }

    println!("🛡️ dracon-warden active. Monitoring {:?}", roots);

    let mut last_run = Instant::now();
    let debounce = Duration::from_secs(2);
    let mut pending_repos = BTreeSet::new();

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
                pending_repos.extend(repos_for_event(&event, &roots));
            }
            Ok(Err(e)) => {
                eprintln!("⚠️ watch error: {}", e);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(anyhow::anyhow!("watch channel disconnected"));
            }
        }

        if !pending_repos.is_empty() && last_run.elapsed() >= debounce {
            let policy = WardenPolicy::load(&policy_path)?;
            let repos = std::mem::take(&mut pending_repos);
            harden_repos(&policy, repos)?;
            last_run = Instant::now();
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::FilterClean { path } => {
            run_filter(true, path.as_deref())?;
        }
        Command::FilterSmudge { path } => {
            run_filter(false, path.as_deref())?;
        }
        Command::Status => {
            let policy_path = resolve_policy_path()?;
            let policy = WardenPolicy::load(&policy_path)?;
            println!("📜 POLICY: {}", policy_path.display());
            println!("🛡️ ROOTS: {:?}", effective_watch_roots(&policy));
            println!(
                "🔑 PUBKEY_SOURCE: {}",
                resolve_local_pubkey_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "NOT_FOUND (set DRACON_OWNER_PUBKEY)".to_string())
            );
        }
        Command::Once => {
            let policy_path = resolve_policy_path()?;
            let policy = WardenPolicy::load(&policy_path)?;
            harden_all(&policy)?;
        }
        Command::Daemon => {
            let policy_path = resolve_policy_path()?;
            run_daemon(policy_path)?;
        }
    }

    Ok(())
}

fn run_filter(is_clean: bool, path: Option<&str>) -> Result<()> {
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input)?;
    if should_passthrough_filter_path(path) {
        std::io::stdout().write_all(&input)?;
        return Ok(());
    }
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
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let unique = format!(
                "{}_{}_{}",
                prefix,
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time")
                    .as_nanos()
            );
            let path = std::env::temp_dir().join(unique);
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
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
        let block = build_gitignore_block(&sample_policy());
        assert!(block.contains(BLOCK_BEGIN));
        assert!(block.contains("target/"));
        assert!(block.contains("!*.env"));
        assert!(block.contains("!secrets/**"));
        assert!(block.contains("!*.pub"));
        assert!(block.contains("!config/envs/*.env"));
        assert!(block.contains("!config/licenses.json"));
        assert!(block.contains("!config/services.test.json"));
        assert!(block.contains("!plan/pages/templates/*.json"));
        assert!(block.contains(BLOCK_END));
    }

    #[test]
    fn build_gitattributes_block_includes_expected_lines() {
        let block = build_gitattributes_block(&sample_policy());
        assert!(block.contains("*.env filter=dracon"));
        assert!(block.contains("secrets/** filter=dracon"));
        assert!(block.contains("*.pub -filter -diff -merge"));
        assert!(block.contains("config/envs/*.env -filter -diff -merge"));
        assert!(block.contains("config/licenses.json -filter -diff -merge"));
        assert!(block.contains("config/services.test.json -filter -diff -merge"));
        assert!(block.contains("plan/pages/templates/*.json -filter -diff -merge"));
    }

    #[test]
    fn plaintext_overrides_protected_without_duplicate_filter_rule() {
        let policy = WardenPolicy {
            protected_patterns: vec!["config/envs/*.env".into(), "*.env".into()],
            plaintext_patterns: vec!["config/envs/*.env".into()],
            hygiene_patterns: vec![],
            watch_roots: vec![],
        };
        let block = build_gitattributes_block(&policy);
        let filter_rule = "config/envs/*.env filter=dracon diff=dracon merge=dracon";
        let plaintext_rule = "config/envs/*.env -filter -diff -merge";
        assert!(!block.contains(filter_rule));
        assert!(block.contains(plaintext_rule));
    }

    #[test]
    fn passthrough_filter_path_matches_config_envs() {
        assert!(should_passthrough_filter_path(Some("config/envs/local.env")));
        assert!(should_passthrough_filter_path(Some("./config/envs/local.env")));
        assert!(should_passthrough_filter_path(Some("config\\envs\\local.env")));
        assert!(!should_passthrough_filter_path(Some(".env")));
        assert!(!should_passthrough_filter_path(None));
    }

    #[test]
    fn repos_for_event_ignores_target_and_maps_repo_root() {
        let td = TempDir::new("warden_event_repo_root");
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
        let td = TempDir::new("warden_owner_pubkeys");
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
        let td = TempDir::new("warden_newest");
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
        let td = TempDir::new("warden_publish_key");
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
        let td = TempDir::new("warden_harden_repo");
        let repo = td.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");
        let key = td.path().join("owner_test.pub");
        fs::write(&key, "age1yyy").expect("key");

        let (a, b, c) = harden_repo(&repo, &sample_policy(), Some(&key)).expect("harden");
        assert!(a);
        assert!(b);
        assert!(c);
        assert!(repo.join(".gitignore").exists());
        assert!(repo.join(".gitattributes").exists());
        assert!(repo.join(".dracon/data/keys/owner_test.pub").exists());
    }

    #[test]
    fn load_sync_watch_roots_reads_override_policy() {
        let _guard = env_lock().lock().expect("env lock");
        let td = TempDir::new("warden_sync_roots");
        let a = td.path().join("a");
        let b = td.path().join("b");
        fs::create_dir_all(&a).expect("a");
        fs::create_dir_all(&b).expect("b");
        let sync_policy = td.path().join("sync.toml");
        fs::write(
            &sync_policy,
            format!(
                "watch_roots = [\"{}\", \"{}\"]\n",
                a.display(),
                b.display()
            ),
        )
        .expect("write sync policy");
        std::env::set_var("DRACON_SYNC_POLICY", &sync_policy);

        let roots = load_sync_watch_roots();
        assert!(roots.contains(&a));
        assert!(roots.contains(&b));

        std::env::remove_var("DRACON_SYNC_POLICY");
    }

    #[test]
    fn effective_watch_roots_merges_and_dedupes() {
        let _guard = env_lock().lock().expect("env lock");
        let td = TempDir::new("warden_effective_roots");
        let p1 = td.path().join("one");
        let p2 = td.path().join("two");
        fs::create_dir_all(&p1).expect("p1");
        fs::create_dir_all(&p2).expect("p2");

        let sync_policy = td.path().join("sync.toml");
        fs::write(
            &sync_policy,
            format!(
                "watch_roots = [\"{}\", \"{}\"]\n",
                p1.display(),
                p2.display()
            ),
        )
        .expect("sync policy");
        std::env::set_var("DRACON_SYNC_POLICY", &sync_policy);

        let policy = WardenPolicy {
            protected_patterns: vec![],
            plaintext_patterns: vec![],
            hygiene_patterns: vec![],
            watch_roots: vec![p1.display().to_string()],
        };
        let merged = effective_watch_roots(&policy);
        assert_eq!(merged.len(), 2);
        assert!(merged.contains(&p1));
        assert!(merged.contains(&p2));

        std::env::remove_var("DRACON_SYNC_POLICY");
    }

    #[test]
    fn apply_managed_file_detects_noop_second_write() {
        let td = TempDir::new("warden_apply_noop");
        let file = td.path().join(".gitignore");
        let block = format!("{BLOCK_BEGIN}\nfoo\n{BLOCK_END}");
        assert!(apply_managed_file(&file, &block).expect("first"));
        assert!(!apply_managed_file(&file, &block).expect("second"));
    }

    #[test]
    fn apply_overwrite_file_detects_noop_second_write() {
        let td = TempDir::new("warden_apply_overwrite_noop");
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
}
