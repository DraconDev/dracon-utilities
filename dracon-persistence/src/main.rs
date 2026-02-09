use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use std::{fs, path::{Path, PathBuf}, process::Command, io::{self, Write}};
use notify::{Watcher, RecursiveMode, Event, EventKind};
use tokio::sync::mpsc;

mod config;
use config::{PersistencePolicy, BackupPolicy};

#[derive(Parser)]
#[command(
    name = "dracon-persistence",
    about = "Legacy state linker (sync responsibilities moved to dracon-sync)",
    version
)]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Cmd {
    /// 🛠️  Legacy setup (sync moved to dracon-sync)
    Install,
    /// 📡 Deprecated: use dracon-sync daemon
    Daemon,
    /// 🏗️  Deprecated: use dracon-sync sync-now <repo>
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
            println!("⚠️  dracon-persistence install is deprecated.");
            println!("Use dracon-sync + dracon-warden for runtime automation.");
            Ok(())
        },
        Cmd::Daemon => {
            println!("⚠️  dracon-persistence daemon is deprecated.");
            println!("Use: dracon-sync daemon");
            Ok(())
        },
        Cmd::SyncNow { path } => {
            println!("⚠️  dracon-persistence sync-now is deprecated: {}", path.display());
            println!("Use: dracon-sync sync-now {}", path.display());
            Ok(())
        },
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
