use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use std::{fs, path::{Path, PathBuf}, process::Command, io::{self, Write}};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use tokio::sync::mpsc;

mod config;
use config::{PersistencePolicy, BackupPolicy};

#[derive(Parser)]
#[command(name = "dracon-persistence", about = "Persistence Manager - Autonomous Git & Symmetry", version)]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Cmd {
    /// 🛠️  Perform guided initial setup
    Install,
    /// 📡 Start background persistence daemon
    Daemon,
    /// 🏗️  Manually trigger synchronization for a specific repository
    SyncNow { path: PathBuf },
    /// 🚚 Relocate a folder to persistent storage and link it back
    RelocateState { path: String, #[arg(short, long)] target: Option<String> },
    /// 🩹 Repair all configured symmetry links and ingest changes
    RepairLinks,
    /// 📜 Show persistence status and active symmetry
    Status,
    /// ⚙️ Open the persistence policy in your editor
    Edit,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Install => {
            println!("🛠️  DRACON PERSISTENCE GUIDED SETUP\n");
            let mut policy = PersistencePolicy::load()?;
            print!("Enter primary persistent repo path (default: {:?}): ", policy.system_repo);
            io::stdout().flush()?;
            let mut input = String::new(); io::stdin().read_line(&mut input)?;
            let input = input.trim();
            if !input.is_empty() { policy.system_repo = PathBuf::from(input); }
            policy.save()?;
            println!("\n✅ Persistence environment initialized."); Ok(())
        },
        Cmd::Daemon => {
            let _ = kill_previous("dracon-persistence");
            let mut policy = PersistencePolicy::load()?;
            println!("🔥 Persistence Daemon active. Monitoring {:?}...", policy.watch_roots);
            let (tx, mut rx) = mpsc::unbounded_channel();
            let mut watcher = notify::recommended_watcher(move |res| { if let Ok(e) = res { let _ = tx.send(e); } })?;
            for root in &policy.watch_roots { if root.exists() { let _ = watcher.watch(root, RecursiveMode::Recursive); } }
            if let Ok(cp) = PersistencePolicy::path() { let _ = watcher.watch(&cp, RecursiveMode::NonRecursive); }
            
            loop {
                // Execute sync pulse
                for repo in discover_repos(policy.watch_roots.clone()) { let _ = sync_repo(&repo, &policy); }
                if let Ok(rx_res) = tokio::time::timeout(tokio::time::Duration::from_secs(policy.pulse_interval_secs), rx.recv()).await {
                    if let Some(event) = rx_res {
                        if event.paths.iter().any(|p| p.to_string_lossy().contains("dracon-persistence.toml")) {
                            if let Ok(new_policy) = PersistencePolicy::load() { policy = new_policy; println!("🔄 Policy reloaded."); }
                        }
                    }
                }
            }
        },
        Cmd::SyncNow { path } => { sync_repo(&path, &PersistencePolicy::load()?) },
        Cmd::RelocateState { path, target } => {
            let mut policy = PersistencePolicy::load()?;
            let source_abs = expand_home(&path)?;
            if !source_abs.exists() { return Err(anyhow!("Source path missing")); }
            let target_rel = target.unwrap_or_else(|| source_abs.file_name().unwrap().to_string_lossy().to_string());
            let target_abs = policy.system_repo.join("state").join(&target_rel);
            println!("🚚 Relocating {:?} to {:?}", source_abs, target_abs);
            fs::create_dir_all(target_abs.parent().unwrap())?;
            if source_abs.is_dir() {
                Command::new("cp").args(["-r", source_abs.to_str().unwrap(), target_abs.to_str().unwrap()]).status()?;
                fs::remove_dir_all(&source_abs)?;
            } else { fs::rename(&source_abs, &target_abs)?; }
            #[cfg(unix)] std::os::unix::fs::symlink(&target_abs, &source_abs)?;
            policy.symmetry.insert(path, format!("state/{}", target_rel));
            policy.save()?; Ok(())
        },
        Cmd::RepairLinks => {
            let policy = PersistencePolicy::load()?;
            repair_symmetry(&policy)
        },
        Cmd::Status => {
            let policy = PersistencePolicy::load()?;
            println!("📜 POLICY: {:?}", PersistencePolicy::path()?);
            println!("🏛️  CORE:   {:?}", policy.system_repo);
            println!("🔗 LINKS:  {}", policy.symmetry.len()); Ok(())
        },
        Cmd::Edit => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
            Command::new(editor).arg(PersistencePolicy::path()?).status()?; Ok(())
        }
    }
}

fn sync_repo(repo: &Path, policy: &PersistencePolicy) -> Result<()> {
    if !repo.join(".git").exists() { return Ok(()); }
    if policy.backup_policy == BackupPolicy::Bundle {
        let name = repo.file_name().unwrap().to_string_lossy();
        let bundle = policy.backup_dir.join(format!("{}_{}.bundle", name, chrono::Local::now().format("%Y-%m-%d")));
        let _ = fs::create_dir_all(&policy.backup_dir);
        let _ = Command::new("git").arg("-C").arg(repo).args(["bundle", "create", bundle.to_str().unwrap(), "HEAD"]).status();
    }
    if policy.auto_pull { let _ = Command::new("git").arg("-C").arg(repo).args(["pull", "--rebase", "--autostash"]).status(); }
    if policy.auto_commit {
        let _ = Command::new("git").arg("-C").arg(repo).arg("add").arg(".").status();
        let _ = Command::new("git").arg("-C").arg(repo).args(["commit", "-m", "chore: persistence pulse"]).status();
    }
    if policy.auto_push { let _ = Command::new("git").arg("-C").arg(repo).arg("push").status(); }
    if repo == policy.system_repo { let _ = repair_symmetry(policy); }
    Ok(())
}

fn repair_symmetry(policy: &PersistencePolicy) -> Result<()> {
    for (live_raw, target_rel) in &policy.symmetry {
        let live_abs = expand_home(live_raw)?;
        let target_abs = policy.system_repo.join(target_rel);
        if !target_abs.exists() { continue; }
        let is_symlink = fs::symlink_metadata(&live_abs).map(|m| m.file_type().is_symlink()).unwrap_or(false);
        if !is_symlink {
            if live_abs.exists() {
                if live_abs.is_dir() { Command::new("cp").args(["-rn", live_abs.to_str().unwrap(), target_abs.to_str().unwrap()]).status()?; fs::remove_dir_all(&live_abs)?; }
                else { fs::copy(&live_abs, &target_abs)?; fs::remove_file(&live_abs)?; }
            }
            #[cfg(unix)] std::os::unix::fs::symlink(&target_abs, &live_abs)?;
        }
    }
    Ok(())
}

fn discover_repos(roots: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut disc = Vec::new();
    for r in roots { for e in walkdir::WalkDir::new(r).into_iter().filter_map(|e| e.ok()) { if e.file_name() == ".git" { if let Some(p) = e.path().parent() { disc.push(p.to_path_buf()); } } } }
    disc
}

fn kill_previous(name: &str) -> Result<()> {
    let out = Command::new("pgrep").arg("-f").arg(name).output()?;
    let my_pid = std::process::id();
    for pid_str in String::from_utf8_lossy(&out.stdout).split_whitespace() {
        if let Ok(pid) = pid_str.parse::<u32>() { if pid != my_pid { let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status(); } }
    }
    Ok(())
}

fn expand_home(path: &str) -> Result<PathBuf> {
    if path.starts_with("~") {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("HOME not set"))?;
        Ok(home.join(path.trim_start_matches("~/")))
    } else { Ok(PathBuf::from(path)) }
}
