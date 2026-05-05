mod policy;
mod exclude;
mod git;
mod bump;
mod secrets;
mod simple_ai;
#[cfg(feature = "scribe")]
mod scribe;
mod report;
mod daemon;
mod sync;
mod test_helpers;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use policy::{resolve_policy_path, SyncPolicy, timestamp_secs};
use policy::freeze_reason;
use exclude::excluded_dir_names_set;
use report::{ConcernRepairFilter, RepoFilter, push_large_blob_threshold_bytes, run_repair_concerns, run_repair_warns, run_repos_report};
use daemon::{run_once, run_daemon, unstuck_repo, list_stuck_repos};
use git::{has_both_main_and_master, consolidate_to_main, detect_orphan_origin, fix_orphan_origin};
use sync::sync_repo;

#[derive(Parser, Debug)]
#[command(name = "dracon-sync")]
#[command(about = "Dracon sync runtime")]
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
    /// Show resolved policy path and sync scope.
    Status {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Validate the sync policy for errors and warnings.
    ValidateConfig,
    /// One-off report across discovered repositories.
    Repos {
        /// Show only concern repos.
        #[arg(long)]
        only_concern: bool,
        /// Show only warn repos.
        #[arg(long, conflicts_with = "only_concern")]
        only_warn: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Repair concern repos (dry-run by default; use --apply to execute).
    RepairConcerns {
        /// Execute git operations to repair concerns.
        #[arg(long)]
        apply: bool,
        /// Only repair this repository path.
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Override push timeout seconds for this run.
        #[arg(long)]
        push_timeout_secs: Option<u64>,
        /// Retry count for push operations.
        #[arg(long, default_value_t = 3)]
        push_retries: u32,
        /// Allow rewrite of large blobs even when paths are outside excluded dirs.
        #[arg(long)]
        rewrite_large_any: bool,
        /// Only repair stuck push concerns.
        #[arg(long, conflicts_with = "only_stuck_pull")]
        only_stuck_push: bool,
        /// Only repair stuck pull concerns.
        #[arg(long, conflicts_with = "only_stuck_push")]
        only_stuck_pull: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Repair warn repos (dirty-only triage; dry-run by default).
    RepairWarns {
        /// Execute git operations to repair warns.
        #[arg(long)]
        apply: bool,
        /// Only repair this repository path.
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Run one sync pass.
    Once,
    /// Run continuous sync loop.
    Daemon {
        /// Override the policy scan interval (seconds). Defaults to policy value.
        #[arg(long)]
        interval_secs: Option<u64>,
    },
    /// Sync a specific repository now.
    SyncNow {
        /// The repository path to sync immediately.
        repo: PathBuf,
    },
    /// Open sync policy in the system editor.
    EditConfig,
    /// Pause sync (creates freeze marker).
    Pause,
    /// Resume sync (removes freeze marker).
    Resume,
    /// Test AI providers connectivity.
    TestAi {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Manage repos permanently stuck on push.
    Stuck {
        #[command(subcommand)]
        cmd: StuckCommands,
    },
    /// Manage repos that have both main and master branches.
    DualBranch {
        #[command(subcommand)]
        cmd: DualBranchCommands,
    },
    /// Detect and repair origin URLs pointing to orphan -N suffixed repos.
    RepairOrigins {
        /// Execute git operations to repair origins.
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Subcommand, Debug)]
enum StuckCommands {
    /// List repos that are permanently stuck on push.
    List,
    /// Unstuck a repo that was marked as permanently stuck.
    Unstuck {
        /// The repository path to unstuck.
        repo: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum DualBranchCommands {
    /// List repos that have both main and master branches.
    List,
    /// Consolidate a repo with both main and master to main only.
    Repair {
        /// The repository path to consolidate.
        repo: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // If output is piped (e.g. `dracon-sync repos | head`), stdout can become a broken pipe.
    // Rust's default printing panics on write errors; convert that specific panic into a clean exit.
    let default_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.to_string();
        if msg.contains("Broken pipe") {
            std::process::exit(0);
        }
        default_panic_hook(info);
    }));

    let cli = Cli::parse();
    daemon::VERBOSITY.store(cli.verbose, Ordering::SeqCst);
    let policy_path = resolve_policy_path()?;

    match cli.cmd {
        Command::Status { json } => {
            let policy = SyncPolicy::load(&policy_path)?;
            let roots = policy.watch_root_paths();
            let excluded_dir_names = excluded_dir_names_set(&policy);
            let repos = git::discover_git_repos(&roots, &excluded_dir_names, &policy.exclude_repos, Some(&policy.system_repo));
            let freeze = freeze_reason(&policy_path);
            if json {
                let payload = report::StatusJson {
                    policy: policy_path.display().to_string(),
                    roots: roots.iter().map(|p| p.display().to_string()).collect(),
                    repos_discovered: repos.len(),
                    pulse_interval_secs: policy.pulse_interval_secs,
                    inactivity_push_delay_secs: policy.inactivity_push_delay_secs,
                    freeze: freeze
                        .map(|r| format!("ON ({})", r))
                        .unwrap_or_else(|| "OFF".to_string()),
                    auto_commit: policy.auto_commit,
                    auto_pull: policy.auto_pull,
                    auto_push: policy.auto_push,
                    auto_bump_versions: policy.auto_bump_versions,
                    auto_repair_concerns: policy.auto_repair_concerns,
                    auto_repair_warns: policy.auto_repair_warns,
                    auto_rewrite_large_blobs: policy.auto_rewrite_large_blobs,
                    max_stage_file_bytes: policy.max_stage_file_bytes,
                    push_blob_threshold_bytes: push_large_blob_threshold_bytes(&policy),
                    exclude_dirs: policy.exclude_dir_names.clone(),
                    exclude_file_patterns: policy.exclude_file_patterns.clone(),
                    pull_op_timeout_secs: policy.pull_op_timeout_secs,
                    push_op_timeout_secs: policy.push_op_timeout_secs,
                    repo_sync_timeout_secs: policy.repo_sync_timeout_secs,
                    push_retries: policy.push_retries,
                    repair_cooldown_secs: policy.repair_cooldown_secs,
                    incident_ledger_max_lines: policy.incident_ledger_max_lines,
                    incident_ledger_max_age_days: policy.incident_ledger_max_age_days,
                    system_repo: policy.system_repo.clone(),
                    backup_policy: policy.backup_policy.clone(),
                    backup_dir: policy.backup_dir.clone(),
                    remotes: policy.remotes.len(),
                    remote_configs: policy.remotes.iter().map(|r| report::RemoteStatus {
                        name: r.name.clone(),
                        auth_type: format!("{:?}", r.auth_type).to_lowercase(),
                        auto_create: r.auto_create,
                        priority: r.priority,
                    }).collect(),
                };
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("📜 POLICY: {}", policy_path.display());
                println!("🔁 ROOTS: {:?}", roots);
                println!("📦 REPOS_DISCOVERED: {}", repos.len());
                println!("⏱️ PULSE: {}s", policy.pulse_interval_secs);
                println!(
                    "⏳ INACTIVITY_PUSH_DELAY: {}s",
                    policy.inactivity_push_delay_secs
                );
                println!(
                    "⏸️ FREEZE: {}",
                    freeze
                        .map(|r| format!("ON ({})", r))
                        .unwrap_or_else(|| "OFF".to_string())
                );
                println!(
                    "⚙️ FLAGS: auto_commit={} auto_pull={} auto_push={} auto_bump_versions={} auto_repair_concerns={} auto_repair_warns={} auto_rewrite_large_blobs={}",
                    policy.auto_commit,
                    policy.auto_pull,
                    policy.auto_push,
                    policy.auto_bump_versions,
                    policy.auto_repair_concerns,
                    policy.auto_repair_warns,
                    policy.auto_rewrite_large_blobs
                );
                println!("📏 MAX_STAGE_FILE_BYTES: {}", policy.max_stage_file_bytes);
                println!(
                    "🧱 PUSH_BLOB_THRESHOLD_BYTES: {}",
                    push_large_blob_threshold_bytes(&policy)
                );
                println!("🚫 EXCLUDE_DIRS: {:?}", policy.exclude_dir_names);
                println!("🚫 EXCLUDE_FILE_PATTERNS: {:?}", policy.exclude_file_patterns);
                println!(
                    "⏱️ TIMEOUTS: pull={}s push={}s repo={}s retries={}",
                    policy.pull_op_timeout_secs,
                    policy.push_op_timeout_secs,
                    policy.repo_sync_timeout_secs,
                    policy.push_retries
                );
                println!(
                    "🧯 REPAIR: cooldown={}s ledger_max_lines={} ledger_max_age_days={}",
                    policy.repair_cooldown_secs,
                    policy.incident_ledger_max_lines,
                    policy.incident_ledger_max_age_days
                );
                if !policy.system_repo.is_empty() {
                    println!("🏛️ SYSTEM_REPO: {}", policy.system_repo);
                }
                if !policy.backup_policy.is_empty() || !policy.backup_dir.is_empty() {
                    println!(
                        "🧰 BACKUP: policy={} dir={}",
                        policy.backup_policy, policy.backup_dir
                    );
                }
                println!("🌐 REMOTES: {}", policy.remotes.len());
            }
        }
        Command::ValidateConfig => {
            let result = policy::validate_config(&policy_path);
            if result.is_valid() {
                println!("✅ Policy is valid");
            } else {
                println!("❌ Policy has errors:");
                for e in &result.errors {
                    println!("  ERROR: {}", e);
                }
                if !result.warnings.is_empty() {
                    println!("\n⚠️  Warnings:");
                    for w in &result.warnings {
                        println!("  WARNING: {}", w);
                    }
                }
                std::process::exit(1);
            }
            if !result.warnings.is_empty() {
                println!("⚠️  Policy has warnings:");
                for w in &result.warnings {
                    println!("  WARNING: {}", w);
                }
            }
        }
        Command::Pause => {
            if let Some(home) = dirs::home_dir() {
                let marker = home.join(".dracon").join("dracon-sync.freeze");
                std::fs::write(&marker, format!("paused at {}\n", timestamp_secs()))?;
                println!("⏸️  Sync paused (freeze marker: {})", marker.display());
            } else {
                anyhow::bail!("cannot determine home directory");
            }
        }
        Command::Resume => {
            if let Some(home) = dirs::home_dir() {
                let marker = home.join(".dracon").join("dracon-sync.freeze");
                if marker.exists() {
                    std::fs::remove_file(&marker)?;
                    println!("▶️  Sync resumed (freeze marker removed)");
                } else {
                    println!("ℹ️  No freeze marker found — sync was not paused");
                }
            } else {
                anyhow::bail!("cannot determine home directory");
            }
        }
        Command::Repos {
            only_concern,
            only_warn,
            json,
        } => {
            let filter = if only_concern {
                RepoFilter::Concern
            } else if only_warn {
                RepoFilter::Warn
            } else {
                RepoFilter::All
            };
            run_repos_report(&policy_path, filter, json).await?;
        }
        Command::RepairConcerns {
            apply,
            repo,
            push_timeout_secs,
            push_retries,
            rewrite_large_any,
            only_stuck_push,
            only_stuck_pull,
            json,
        } => {
            let filter = if only_stuck_push {
                ConcernRepairFilter::StuckPush
            } else if only_stuck_pull {
                ConcernRepairFilter::StuckPull
            } else {
                ConcernRepairFilter::All
            };
            run_repair_concerns(
                &policy_path,
                apply,
                repo,
                push_timeout_secs,
                push_retries,
                rewrite_large_any,
                filter,
                json,
            )
            .await?;
        }
        Command::RepairWarns { apply, repo, json } => {
            run_repair_warns(&policy_path, apply, repo, json).await?;
        }
        Command::Once => {
            run_once(&policy_path).await?;
        }
        Command::Daemon { interval_secs } => {
            run_daemon(policy_path, interval_secs).await?;
        }
        Command::SyncNow { repo } => {
            if let Some(reason) = freeze_reason(&policy_path) {
                println!("⏸️ sync frozen ({})", reason);
                return Ok(());
            }
            if daemon::is_repo_stuck(&repo) {
                println!("🔒 {} is stuck on push. Run 'dracon-sync stuck unstuck {}' first.", repo.display(), repo.display());
                return Ok(());
            }
            let policy = SyncPolicy::load(&policy_path)?;
            let excluded_dir_names = excluded_dir_names_set(&policy);
            if sync_repo(&repo, &policy, &excluded_dir_names, 0, None, false).await? {
                println!("🔁 synced {}", repo.display());
            } else {
                println!("✅ no sync changes {}", repo.display());
            }
        }
        Command::EditConfig => {
            policy::open_policy_in_editor(&policy_path)?;
        }
        Command::TestAi { json } => {
            use simple_ai::SimpleAiService;

            let service = SimpleAiService::new();
            if service.is_empty() {
                if json {
                    println!(r#"{{"providers":[],"all_ok":false,"error":"no providers configured"}}"#);
                } else {
                    println!("❌ No AI providers configured");
                    println!("   Add providers to ~/.dracon/utilities/sync/ai.toml");
                }
                return Ok(());
            }

            SimpleAiService::reset_health().await;

            let providers = service.provider_names();

            #[derive(serde::Serialize)]
            struct ProviderResult {
                name: String,
                status: String,
                latency_ms: Option<u64>,
                error: Option<String>,
            }

            let mut results: Vec<ProviderResult> = Vec::new();
            let mut all_ok = true;
            let mut working_provider = None;

            for name in &providers {
                if json {
                    print!("Testing {}... ", name);
                } else {
                    print!("   Testing {}... ", name);
                }
                match service.test_provider(name).await {
                    Ok((true, resp)) => {
                        if resp.trim().to_uppercase().contains("OK") {
                            if json {
                                println!("ok");
                            } else {
                                println!("✅");
                            }
                            working_provider = Some(name.clone());
                            results.push(ProviderResult {
                                name: name.clone(),
                                status: "ok".to_string(),
                                latency_ms: None,
                                error: None,
                            });
                        } else {
                            if json {
                                println!("warn");
                            } else {
                                println!("⚠️  (unexpected response: {}...)", resp.chars().take(20).collect::<String>());
                            }
                            working_provider = Some(name.clone());
                            results.push(ProviderResult {
                                name: name.clone(),
                                status: "warn".to_string(),
                                latency_ms: None,
                                error: Some(resp.chars().take(50).collect()),
                            });
                        }
                    }
                    Ok((false, err)) => {
                        let err_lower = err.to_lowercase();
                        if err_lower.contains("429") || err_lower.contains("rate limit") {
                            if json {
                                println!("rate_limited");
                            } else {
                                println!("⏳ rate limited");
                            }
                            all_ok = false;
                            results.push(ProviderResult {
                                name: name.clone(),
                                status: "rate_limited".to_string(),
                                latency_ms: None,
                                error: Some(err.to_string()),
                            });
                        } else if err_lower.contains("401") || err_lower.contains("unauthorized") || err_lower.contains("api key") {
                            if json {
                                println!("auth_error");
                            } else {
                                println!("🔑 auth error (check API key)");
                            }
                            all_ok = false;
                            results.push(ProviderResult {
                                name: name.clone(),
                                status: "auth_error".to_string(),
                                latency_ms: None,
                                error: Some(err.to_string()),
                            });
                        } else {
                            if json {
                                println!("error");
                            } else {
                                println!("❌ {}", err.chars().take(40).collect::<String>());
                            }
                            all_ok = false;
                            results.push(ProviderResult {
                                name: name.clone(),
                                status: "error".to_string(),
                                latency_ms: None,
                                error: Some(err.to_string()),
                            });
                        }
                    }
                    Err(e) => {
                        if json {
                            println!("error");
                        } else {
                            println!("❌ {}", e.to_string().chars().take(40).collect::<String>());
                        }
                        all_ok = false;
                        results.push(ProviderResult {
                            name: name.clone(),
                            status: "error".to_string(),
                            latency_ms: None,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }

            if json {
                #[derive(serde::Serialize)]
                struct JsonOutput {
                    providers: Vec<ProviderResult>,
                    all_ok: bool,
                    working_provider: Option<String>,
                }
                let output = JsonOutput {
                    providers: results,
                    all_ok,
                    working_provider,
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!();
                if all_ok {
                    println!("✅ All AI providers ready");
                } else if working_provider.is_some() {
                    println!("⚠️  Some providers failed but fallback available");
                } else {
                    println!("❌ All AI providers failed");
                }

                if let Some(ref wp) = working_provider {
                    println!("   Using: {} (fallback order: {:?})", wp, providers);
                }
            }
        }
        Command::Stuck { cmd } => match cmd {
            StuckCommands::List => {
                list_stuck_repos();
            }
            StuckCommands::Unstuck { repo } => {
                unstuck_repo(&repo);
            }
        },
        Command::DualBranch { cmd } => match cmd {
            DualBranchCommands::List => {
                let policy = SyncPolicy::load(&policy_path)?;
                let roots = policy.watch_root_paths();
                let excluded_dir_names = excluded_dir_names_set(&policy);
                let repos = git::discover_git_repos(&roots, &excluded_dir_names, &policy.exclude_repos, Some(&policy.system_repo));
                let mut found = 0;
                for repo in repos {
                    if has_both_main_and_master(&repo) {
                        let branch = git::current_branch(&repo).unwrap_or_else(|| "unknown".to_string());
                        println!("   {} (currently on {})", repo.display(), branch);
                        found += 1;
                    }
                }
                if found == 0 {
                    println!("✅ no repos with both main and master");
                } else {
                    println!("\n🔧 Run 'dracon-sync dual-branch repair <path>' to consolidate to main");
                }
            }
            DualBranchCommands::Repair { repo } => {
                if !has_both_main_and_master(&repo) {
                    println!("ℹ️ {} does not have both main and master", repo.display());
                    return Ok(());
                }
                println!("🔧 Consolidating {} to main...", repo.display());
                match consolidate_to_main(&repo).await {
                    Ok(()) => println!("✅ consolidated to main"),
                    Err(e) => {
                        eprintln!("❌ failed: {}", e);
                        return Err(e);
                    }
                }
            }
        },
        Command::RepairOrigins { apply } => {
            let policy = SyncPolicy::load(&policy_path)?;
            let roots = policy.watch_root_paths();
            let excluded_dir_names = excluded_dir_names_set(&policy);
            let repos = git::discover_git_repos(&roots, &excluded_dir_names, &policy.exclude_repos, Some(&policy.system_repo));
            let mut found = 0;
            for repo in repos {
                if let Some((current, canonical)) = detect_orphan_origin(&repo) {
                    println!("   {}: {} -> {}", repo.display(), current, canonical);
                    found += 1;
                    if apply {
                        if let Err(e) = fix_orphan_origin(&repo, &canonical) {
                            eprintln!("❌ failed to fix origin for {}: {}", repo.display(), e);
                        } else {
                            println!("✅ fixed origin for {}", repo.display());
                        }
                    }
                }
            }
            if found == 0 {
                println!("✅ no orphan origins found");
            } else if !apply {
                println!("\n🔧 Run 'dracon-sync repair-origins --apply' to fix them");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Tests from the original main.rs will be added here as needed
}
