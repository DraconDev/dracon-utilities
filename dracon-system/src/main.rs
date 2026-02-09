use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::process::Command;

use dracon_system_lib::analyze_workspace_storage;

#[derive(Parser, Debug)]
#[command(name = "dracon-system")]
#[command(about = "Deterministic system utility (no AI)")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show core path and service status.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Run deterministic diagnostics for canonical dracon setup.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Analyze storage hotspots and optionally clean safe build/cache dirs.
    Storage {
        root: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        cleanup: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        allow_tracked: bool,
        #[arg(long)]
        min_size_mb: Option<u64>,
        #[arg(long)]
        kinds: Option<String>,
    },
}

#[derive(Debug, Serialize)]
struct StatusReport {
    system_root: String,
    nixos_root: String,
    sync_policy: String,
    system_policy: String,
    sync_service_active: bool,
    warden_service_active: bool,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    system_root_exists: bool,
    nixos_root_exists: bool,
    canonical_libs_exists: bool,
    canonical_utils_exists: bool,
    sync_policy_exists: bool,
    sync_service_active: bool,
    warden_service_active: bool,
}

#[derive(Debug, Clone)]
struct CleanupConfig {
    apply: bool,
    allow_tracked: bool,
    min_size_mb: u64,
    kinds: HashSet<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct SystemPolicy {
    #[serde(default)]
    storage: StoragePolicy,
}

#[derive(Debug, Clone, Deserialize)]
struct StoragePolicy {
    #[serde(default)]
    default_root: String,
    #[serde(default = "default_min_size_mb")]
    min_size_mb: u64,
    #[serde(default = "default_kinds")]
    kinds: String,
}

impl Default for StoragePolicy {
    fn default() -> Self {
        Self {
            default_root: String::new(),
            min_size_mb: default_min_size_mb(),
            kinds: default_kinds(),
        }
    }
}

fn default_min_size_mb() -> u64 {
    512
}

fn default_kinds() -> String {
    "rust-build,node-deps,build-output,cache".to_string()
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut idx = 0usize;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    format!("{value:.1} {}", UNITS[idx])
}

fn canonical_system_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join("dracon")
}

fn parse_kinds(csv: &str) -> HashSet<String> {
    csv.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn resolve_system_policy_path() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("DRACON_SYSTEM_POLICY") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Some(p);
        }
    }

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    let candidates = [
        home.join("dracon/utilities/system/dracon-system.toml"),
        home.join("dracon/utilities/system/config.toml"),
        home.join("dracon/system/dracon-system.toml"),
        home.join("dracon/system/config.toml"),
    ];

    candidates.into_iter().find(|p| p.exists())
}

fn load_system_policy() -> (Option<PathBuf>, SystemPolicy) {
    let Some(path) = resolve_system_policy_path() else {
        return (None, SystemPolicy::default());
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return (Some(path), SystemPolicy::default()),
    };
    let parsed: SystemPolicy = toml::from_str(&content).unwrap_or_default();
    (Some(path), parsed)
}

async fn is_user_service_active(service: &str) -> bool {
    let output = Command::new("systemctl")
        .args(["--user", "is-active", service])
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim() == "active"
        }
        _ => false,
    }
}

async fn build_status_report() -> StatusReport {
    let root = canonical_system_root();
    let (system_policy_path, _) = load_system_policy();
    StatusReport {
        system_root: root.display().to_string(),
        nixos_root: root.join("nixos").display().to_string(),
        sync_policy: root
            .join("utilities/sync/dracon-sync.toml")
            .display()
            .to_string(),
        system_policy: system_policy_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<default>".to_string()),
        sync_service_active: is_user_service_active("dracon-sync.service").await,
        warden_service_active: is_user_service_active("dracon-warden.service").await,
    }
}

async fn build_doctor_report() -> DoctorReport {
    let root = canonical_system_root();
    let nixos = root.join("nixos");
    let libs = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join("Dev/dracon-libs");
    let utils = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join("Dev/dracon-utilities");
    let policy = root.join("utilities/sync/dracon-sync.toml");

    DoctorReport {
        system_root_exists: root.exists(),
        nixos_root_exists: nixos.exists(),
        canonical_libs_exists: libs.exists(),
        canonical_utils_exists: utils.exists(),
        sync_policy_exists: policy.exists(),
        sync_service_active: is_user_service_active("dracon-sync.service").await,
        warden_service_active: is_user_service_active("dracon-warden.service").await,
    }
}

async fn is_git_tracked_dir(path: &Path) -> Result<bool> {
    let parent = match path.parent() {
        Some(p) => p,
        None => return Ok(false),
    };
    let name = match path.file_name() {
        Some(n) => n.to_string_lossy().to_string(),
        None => return Ok(false),
    };

    let top_out = Command::new("git")
        .arg("-C")
        .arg(parent)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .await?;
    if !top_out.status.success() {
        return Ok(false);
    }

    let repo_root = String::from_utf8_lossy(&top_out.stdout).trim().to_string();
    if repo_root.is_empty() {
        return Ok(false);
    }

    let ls_out = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["ls-files", "--", &name])
        .output()
        .await?;
    if !ls_out.status.success() {
        return Ok(false);
    }

    Ok(!String::from_utf8_lossy(&ls_out.stdout).trim().is_empty())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Commands::Status { json } => {
            let report = build_status_report().await;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("system_root: {}", report.system_root);
                println!("nixos_root: {}", report.nixos_root);
                println!("sync_policy: {}", report.sync_policy);
                println!("system_policy: {}", report.system_policy);
                println!("sync_service_active: {}", report.sync_service_active);
                println!("warden_service_active: {}", report.warden_service_active);
            }
        }
        Commands::Doctor { json } => {
            let report = build_doctor_report().await;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("system_root_exists: {}", report.system_root_exists);
                println!("nixos_root_exists: {}", report.nixos_root_exists);
                println!("canonical_libs_exists: {}", report.canonical_libs_exists);
                println!("canonical_utils_exists: {}", report.canonical_utils_exists);
                println!("sync_policy_exists: {}", report.sync_policy_exists);
                println!("sync_service_active: {}", report.sync_service_active);
                println!("warden_service_active: {}", report.warden_service_active);
            }
        }
        Commands::Storage {
            root,
            json,
            cleanup,
            apply,
            allow_tracked,
            min_size_mb,
            kinds,
        } => {
            let (_, policy) = load_system_policy();
            let root = root.unwrap_or_else(|| {
                if !policy.storage.default_root.trim().is_empty() {
                    return PathBuf::from(policy.storage.default_root.clone());
                }
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/home"))
                    .join("Dev")
            });
            let min_size_mb = min_size_mb.unwrap_or(policy.storage.min_size_mb);
            let kinds = kinds.unwrap_or_else(|| policy.storage.kinds.clone());

            let report = analyze_workspace_storage(&root, 15, 25).await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
                return Ok(());
            }

            println!("Workspace: {}", report.root.display());
            println!();
            println!("Top projects:");
            for item in &report.top_projects {
                println!("  {:>10}  {}", human_bytes(item.bytes), item.path.display());
            }

            println!();
            println!("Top hotspots:");
            for item in &report.top_hotspots {
                println!(
                    "  {:>10}  {:<12} {}",
                    human_bytes(item.bytes),
                    item.kind,
                    item.path.display()
                );
            }

            if cleanup {
                let cfg = CleanupConfig {
                    apply,
                    allow_tracked,
                    min_size_mb,
                    kinds: parse_kinds(&kinds),
                };
                let threshold = cfg.min_size_mb.saturating_mul(1024 * 1024);
                let selected: Vec<_> = report
                    .top_hotspots
                    .iter()
                    .filter(|h| cfg.kinds.contains(&h.kind) && h.bytes >= threshold)
                    .collect();

                println!();
                println!(
                    "Cleanup mode: {}",
                    if cfg.apply { "APPLY" } else { "DRY-RUN" }
                );
                println!(
                    "Kinds: {}",
                    {
                        let mut v: Vec<_> = cfg.kinds.iter().cloned().collect();
                        v.sort();
                        v.join(",")
                    }
                );
                println!("Min size: {} MiB", cfg.min_size_mb);
                println!("Allow tracked: {}", cfg.allow_tracked);
                println!("Selected paths: {}", selected.len());

                let mut total = 0u64;
                let mut actionable = Vec::new();
                for item in selected {
                    let tracked = is_git_tracked_dir(&item.path).await.unwrap_or(false);
                    if tracked && !cfg.allow_tracked {
                        println!(
                            "  {:>10}  {:<12} {}  [SKIP tracked]",
                            human_bytes(item.bytes),
                            item.kind,
                            item.path.display()
                        );
                        continue;
                    }
                    total += item.bytes;
                    println!(
                        "  {:>10}  {:<12} {}{}",
                        human_bytes(item.bytes),
                        item.kind,
                        item.path.display(),
                        if tracked { "  [tracked]" } else { "" }
                    );
                    actionable.push(item.path.clone());
                }
                println!("Estimated reclaimed: {}", human_bytes(total));

                if cfg.apply {
                    for path in actionable {
                        if path.exists() {
                            println!("Deleting {}", path.display());
                            tokio::fs::remove_dir_all(path).await?;
                        }
                    }
                } else {
                    println!("No changes made. Re-run with --apply to execute cleanup.");
                }
            }
        }
    }

    Ok(())
}
