mod bump;
mod daemon;
mod exclude;
mod git;
mod helpers;
mod log;
mod nix;
mod policy;
mod release;
mod report;
mod scribe;
mod secrets;
mod simple_ai;
mod standard_files;
mod sync;
mod test_helpers;
mod visibility;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use daemon::{list_stuck_repos, run_daemon, run_once, unstuck_repo};
use exclude::excluded_dir_names_set;
use git::{consolidate_to_main, detect_orphan_origin, fix_orphan_origin, has_both_main_and_master};
use helpers::{is_auth_error, is_rate_limited};
use policy::freeze_reason;
use policy::{resolve_policy_path, timestamp_secs, SyncPolicy};
use report::{
    push_large_blob_threshold_bytes, run_repair_concerns, run_repair_warns, run_repos_report,
    ConcernRepairFilter, RepoFilter,
};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use sync::sync_repo;

#[derive(Parser, Debug)]
#[command(name = "dracon-sync")]
#[command(about = "Git sync automation — auto-commit, push, and mirror your repos")]
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
        /// Sort repos by field: updated, name, modified, ahead, behind.
        #[arg(long, default_value = "updated")]
        sort: String,
        /// Filter repos by name (substring match).
        #[arg(long)]
        filter: Option<String>,
        /// Show full repo paths instead of short names.
        #[arg(long)]
        full_path: bool,
    },
    /// Check daemon health (policy valid, daemon responsive, repos healthy).
    Health {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Print Prometheus-style metrics.
    Metrics,
    /// Run one sync pass.
    Once,
    /// Run continuous sync loop.
    Daemon {
        /// Override the policy scan interval (seconds). Defaults to policy value.
        #[arg(long)]
        interval_secs: Option<u64>,
    },
    /// Sync one or more repositories now.
    SyncNow {
        /// The repository path(s) to sync immediately.
        repos: Vec<PathBuf>,
        /// Preview what would be done without making any changes.
        #[arg(long)]
        dry_run: bool,
        /// Bypass safety guards (e.g. mass-deletion prevention) for intentional operations.
        #[arg(long)]
        force: bool,
    },
    /// Pause sync (creates freeze marker).
    Pause,
    /// Resume sync (removes freeze marker).
    Resume,
    /// Open sync policy in the system editor.
    EditConfig,
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
    /// Detect and repair origin URLs pointing to orphan -N suffixed repos.
    RepairOrigins {
        /// Execute git operations to repair origins.
        #[arg(long)]
        apply: bool,
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
    /// Publish a repository to configured package registries.
    Publish {
        /// The repository path to publish.
        repo: PathBuf,
        /// Only publish to these target names (defaults to all configured).
        #[arg(long)]
        targets: Vec<String>,
        /// Skip the dry-run check and publish directly.
        #[arg(long)]
        skip_dry_run: bool,
    },
    /// Show publish status for a repository across configured registries.
    PublishStatus {
        /// The repository path to check.
        repo: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Scaffold standard files (LICENSE, CLA, etc.) into repositories.
    Scaffold {
        /// Repository path to scaffold. Defaults to all discovered repos.
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Only scaffold these files (by target name, e.g. LICENSE, CLA.md).
        #[arg(long)]
        files: Vec<String>,
        /// Overwrite existing files with template versions.
        #[arg(long)]
        overwrite: bool,
        /// Preview what would be done without making any changes.
        #[arg(long)]
        dry_run: bool,
    },
    /// Validate the sync policy for errors and warnings.
    ValidateConfig,
    /// Test AI providers connectivity.
    TestAi {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
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
            let repos = git::discover_git_repos(
                &roots,
                &excluded_dir_names,
                &policy.exclude_repos,
                Some(&policy.system_repo),
            );
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
                    remote_configs: policy
                        .remotes
                        .iter()
                        .map(|r| report::RemoteStatus {
                            name: r.name.clone(),
                            auth_type: format!("{:?}", r.auth_type).to_lowercase(),
                            auto_create: r.auto_create,
                            priority: r.priority,
                        })
                        .collect(),
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
                println!(
                    "🚫 EXCLUDE_FILE_PATTERNS: {:?}",
                    policy.exclude_file_patterns
                );
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
            sort,
            filter: filter_name,
            full_path,
        } => {
            let filter = if only_concern {
                RepoFilter::Concern
            } else if only_warn {
                RepoFilter::Warn
            } else {
                RepoFilter::All
            };
            run_repos_report(&policy_path, filter, json, &sort, filter_name.as_deref(), full_path).await?;
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
        Command::SyncNow {
            repos,
            dry_run,
            force,
        } => {
            if let Some(reason) = freeze_reason(&policy_path) {
                println!("⏸️ sync frozen ({})", reason);
                return Ok(());
            }
            let policy = SyncPolicy::load(&policy_path)?;
            let excluded_dir_names = excluded_dir_names_set(&policy);
            for repo in repos {
                if daemon::is_repo_stuck(&repo) {
                    println!(
                        "🔒 {} is stuck on push. Run 'dracon-sync stuck unstuck {}' first.",
                        repo.display(),
                        repo.display()
                    );
                    continue;
                }
                match sync_repo(
                    &repo,
                    &policy,
                    &excluded_dir_names,
                    0,
                    None,
                    dry_run,
                    Some(&policy_path),
                    force,
                )
                .await
                {
                    Ok(crate::sync::SyncOutcome::Synced) => {
                        if dry_run {
                            println!("✅ dry-run complete for {}", repo.display());
                        } else {
                            println!("🔁 synced {}", repo.display());
                        }
                    }
                    Ok(crate::sync::SyncOutcome::NothingToDo) => {
                        if dry_run {
                            println!("✅ no sync changes needed for {}", repo.display());
                        } else {
                            println!("✅ no sync changes {}", repo.display());
                        }
                    }
                    Ok(crate::sync::SyncOutcome::Blocked) => {
                        println!(
                            "⏸️  sync blocked for {} (guard or manual intervention required)",
                            repo.display()
                        );
                    }
                    Err(e) => {
                        eprintln!("❌ error syncing {}: {}", repo.display(), e);
                    }
                }
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
                    println!(
                        r#"{{"providers":[],"all_ok":false,"error":"no providers configured"}}"#
                    );
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
                                println!(
                                    "⚠️  (unexpected response: {}...)",
                                    resp.chars().take(20).collect::<String>()
                                );
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
                        if is_rate_limited(&err_lower) {
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
                        } else if is_auth_error(&err_lower) {
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
        Command::DualBranch { cmd } => {
            match cmd {
                DualBranchCommands::List => {
                    let policy = SyncPolicy::load(&policy_path)?;
                    let roots = policy.watch_root_paths();
                    let excluded_dir_names = excluded_dir_names_set(&policy);
                    let repos = git::discover_git_repos(
                        &roots,
                        &excluded_dir_names,
                        &policy.exclude_repos,
                        Some(&policy.system_repo),
                    );
                    let mut found = 0;
                    for repo in repos {
                        if has_both_main_and_master(&repo) {
                            let branch =
                                git::current_branch(&repo).unwrap_or_else(|| "unknown".to_string());
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
            }
        }
        Command::RepairOrigins { apply } => {
            let policy = SyncPolicy::load(&policy_path)?;
            let roots = policy.watch_root_paths();
            let excluded_dir_names = excluded_dir_names_set(&policy);
            let repos = git::discover_git_repos(
                &roots,
                &excluded_dir_names,
                &policy.exclude_repos,
                Some(&policy.system_repo),
            );
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
        Command::Health { json } => {
            let policy = SyncPolicy::load(&policy_path)?;
            let validate_result = policy::validate_config(&policy_path);
            let roots = policy.watch_root_paths();
            let excluded_dir_names = excluded_dir_names_set(&policy);
            let repos = git::discover_git_repos(
                &roots,
                &excluded_dir_names,
                &policy.exclude_repos,
                Some(&policy.system_repo),
            );
            let freeze = freeze_reason(&policy_path);

            let frozen = freeze.is_some();
            let policy_ok = validate_result.is_valid();
            let daemon_ok = true;

            let status = if frozen || !policy_ok {
                "unhealthy"
            } else {
                "healthy"
            };

            if json {
                #[derive(serde::Serialize)]
                struct HealthJson<'a> {
                    status: &'a str,
                    frozen: bool,
                    freeze_reason: Option<&'a str>,
                    policy_valid: bool,
                    policy_errors: Vec<String>,
                    policy_warnings: Vec<String>,
                    daemon_running: bool,
                    roots: usize,
                    repos_discovered: usize,
                }
                let payload = HealthJson {
                    status,
                    frozen,
                    freeze_reason: freeze.as_deref(),
                    policy_valid: policy_ok,
                    policy_errors: validate_result.errors,
                    policy_warnings: validate_result.warnings,
                    daemon_running: daemon_ok,
                    roots: roots.len(),
                    repos_discovered: repos.len(),
                };
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("🏥 Health Check");
                println!(
                    "   Status: {}",
                    if status == "healthy" {
                        "✅ healthy"
                    } else {
                        "❌ unhealthy"
                    }
                );
                println!(
                    "   Daemon: {}",
                    if daemon_ok {
                        "✅ running"
                    } else {
                        "❌ not running"
                    }
                );
                if let Some(reason) = &freeze {
                    println!("   Freeze: ⏸️ {}", reason);
                } else {
                    println!("   Freeze: off");
                }
                println!(
                    "   Policy: {}",
                    if policy_ok {
                        "✅ valid"
                    } else {
                        "❌ invalid"
                    }
                );
                for e in &validate_result.errors {
                    println!("      ERROR: {}", e);
                }
                for w in &validate_result.warnings {
                    println!("      WARNING: {}", w);
                }
                println!(
                    "   Repos: {} discovered across {} roots",
                    repos.len(),
                    roots.len()
                );
            }
        }
        Command::Metrics => {
            let policy = SyncPolicy::load(&policy_path)?;
            let roots = policy.watch_root_paths();
            let excluded_dir_names = excluded_dir_names_set(&policy);
            let repos = git::discover_git_repos(
                &roots,
                &excluded_dir_names,
                &policy.exclude_repos,
                Some(&policy.system_repo),
            );
            let freeze = freeze_reason(&policy_path);
            let frozen = freeze.is_some();

            println!("# HELP dracon_sync_info Dracon sync daemon info");
            println!("# TYPE dracon_sync_info gauge");
            println!(
                "dracon_sync_info{{version=\"{}\"}} 1",
                env!("CARGO_PKG_VERSION")
            );

            println!(
                "# HELP dracon_sync_repos_discovered_total Number of git repositories discovered"
            );
            println!("# TYPE dracon_sync_repos_discovered_total gauge");
            println!("dracon_sync_repos_discovered_total {}", repos.len());

            println!("# HELP dracon_sync_watch_roots_total Number of configured watch roots");
            println!("# TYPE dracon_sync_watch_roots_total gauge");
            println!("dracon_sync_watch_roots_total {}", roots.len());

            println!("# HELP dracon_sync_remotes_total Number of configured remotes");
            println!("# TYPE dracon_sync_remotes_total gauge");
            println!("dracon_sync_remotes_total {}", policy.remotes.len());

            println!("# HELP dracon_sync_freeze_state Whether sync is currently frozen (1=frozen, 0=active)");
            println!("# TYPE dracon_sync_freeze_state gauge");
            println!("dracon_sync_freeze_state {}", if frozen { 1 } else { 0 });

            println!("# HELP dracon_sync_policy_auto_commit Whether auto-commit is enabled");
            println!("# TYPE dracon_sync_policy_auto_commit gauge");
            println!(
                "dracon_sync_policy_auto_commit {}",
                if policy.auto_commit { 1 } else { 0 }
            );

            println!("# HELP dracon_sync_policy_auto_push Whether auto-push is enabled");
            println!("# TYPE dracon_sync_policy_auto_push gauge");
            println!(
                "dracon_sync_policy_auto_push {}",
                if policy.auto_push { 1 } else { 0 }
            );

            println!("# HELP dracon_sync_policy_auto_pull Whether auto-pull is enabled");
            println!("# TYPE dracon_sync_policy_auto_pull gauge");
            println!(
                "dracon_sync_policy_auto_pull {}",
                if policy.auto_pull { 1 } else { 0 }
            );

            println!("# HELP dracon_sync_policy_auto_repair_concerns Whether auto-repair concerns is enabled");
            println!("# TYPE dracon_sync_policy_auto_repair_concerns gauge");
            println!(
                "dracon_sync_policy_auto_repair_concerns {}",
                if policy.auto_repair_concerns { 1 } else { 0 }
            );

            println!("# HELP dracon_sync_incident_ledger_max_lines Incident ledger max lines");
            println!("# TYPE dracon_sync_incident_ledger_max_lines gauge");
            println!(
                "dracon_sync_incident_ledger_max_lines {}",
                policy.incident_ledger_max_lines
            );

            let incident_path = report::incident_ledger_path(&policy_path);
            if incident_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&incident_path) {
                    let lines = content.lines().filter(|l| !l.trim().is_empty()).count();
                    println!("# HELP dracon_sync_incident_ledger_lines_current Current number of lines in incident ledger");
                    println!("# TYPE dracon_sync_incident_ledger_lines_current gauge");
                    println!("dracon_sync_incident_ledger_lines_current {}", lines);
                }
            }

            if let Some(home) = dirs::home_dir() {
                let stuck_path = home.join(".local/state/dracon/dracon-sync-stuck-push-repos.json");
                if stuck_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&stuck_path) {
                        if let Ok(stuck) = serde_json::from_str::<Vec<serde_json::Value>>(&content)
                        {
                            println!("# HELP dracon_sync_stuck_repos_total Number of repos permanently stuck on push");
                            println!("# TYPE dracon_sync_stuck_repos_total gauge");
                            println!("dracon_sync_stuck_repos_total {}", stuck.len());
                        }
                    }
                }
            }

            println!("# HELP dracon_sync_push_retries Default push retry count");
            println!("# TYPE dracon_sync_push_retries gauge");
            println!("dracon_sync_push_retries {}", policy.push_retries);

            println!("# HELP dracon_sync_pulse_interval_secs Sync pulse interval in seconds");
            println!("# TYPE dracon_sync_pulse_interval_secs gauge");
            println!(
                "dracon_sync_pulse_interval_secs {}",
                policy.pulse_interval_secs
            );

            let blocked =
                crate::sync::MASS_DELETION_GUARD_BLOCKED.load(std::sync::atomic::Ordering::Relaxed);
            println!("# HELP dracon_sync_mass_deletion_guard_blocked_total Mass deletions blocked by safety guard");
            println!("# TYPE dracon_sync_mass_deletion_guard_blocked_total counter");
            println!("dracon_sync_mass_deletion_guard_blocked_total {}", blocked);
        }
        Command::Publish {
            repo,
            targets,
            skip_dry_run: _,
        } => {
            let policy = SyncPolicy::load(&policy_path)?;
            if !policy.auto_publish {
                anyhow::bail!("auto_publish is disabled in config. Enable it or use `dracon-sync publish` with --force.");
            }
            let repo_targets = if targets.is_empty() {
                policy
                    .publish_targets
                    .iter()
                    .map(|t| t.name.clone())
                    .collect::<Vec<_>>()
            } else {
                targets
            };
            let version = release::detect_project_version(&repo)
                .map(|(v, _)| v)
                .unwrap_or_else(|| "unknown".to_string());
            println!(
                "Publishing {} (v{}) to: {}",
                repo.display(),
                version,
                repo_targets.join(", ")
            );
            let steps = release::run_release_pipeline(
                &repo,
                "",
                &version,
                "patch", // Default to patch for manual publish
                &policy,
                true,  // auto_tag: always tag for manual publish
                false, // auto_release: don't create GitHub release for manual publish
                &repo_targets,
                false, // nix_auto_update: disabled for manual publish
            )
            .await;
            for step in &steps {
                match step {
                    release::ReleaseStep::TagCreated(tag) => println!("  Tag: {tag}"),
                    release::ReleaseStep::GitHubReleaseCreated(tag) => println!("  Release: {tag}"),
                    release::ReleaseStep::Published { registry, version } => {
                        println!("  Published: {registry} v{version}")
                    }
                    release::ReleaseStep::NixFlakePRCreated(url) => {
                        println!("  Nix flake PR: {url}")
                    }
                    release::ReleaseStep::Skipped(reason) => println!("  Skipped: {reason}"),
                    release::ReleaseStep::Failed { step: s, error } => {
                        eprintln!("  Failed: {s} — {error}")
                    }
                }
            }
        }
        Command::PublishStatus { repo, json } => {
            let policy = SyncPolicy::load(&policy_path)?;
            let version = release::detect_project_version(&repo)
                .map(|(v, _)| v)
                .unwrap_or_else(|| "unknown".to_string());
            let mut statuses = Vec::new();
            for target in &policy.publish_targets {
                match release::extract_package_name(&repo, target.registry) {
                    Ok(pkg_name) => {
                        let exists = release::version_exists_on_registry(
                            target.registry,
                            &pkg_name,
                            &version,
                        )
                        .await;
                        statuses.push(serde_json::json!({
                            "target": target.name,
                            "registry": target.registry.as_str(),
                            "package": pkg_name,
                            "version": version,
                            "published": exists.unwrap_or(false),
                        }));
                    }
                    Err(e) => statuses.push(serde_json::json!({
                        "target": target.name,
                        "registry": target.registry.as_str(),
                        "version": version,
                        "error": e.to_string(),
                    })),
                }
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&statuses)?);
            } else {
                println!("Publish status for {} (v{}):", repo.display(), version);
                for s in &statuses {
                    let target = s["target"].as_str().unwrap_or("?");
                    let published = s["published"].as_bool().unwrap_or(false);
                    let status_str = if published {
                        "published"
                    } else {
                        "not published"
                    };
                    println!("  {target}: {status_str}");
                }
            }
        }
        Command::Scaffold {
            repo,
            files,
            overwrite,
            dry_run,
        } => {
            cmd_scaffold(&policy_path, repo, files, overwrite, dry_run).await?;
        }
    }

    Ok(())
}

async fn cmd_scaffold(
    policy_path: &std::path::Path,
    repo: Option<PathBuf>,
    files: Vec<String>,
    overwrite: bool,
    dry_run: bool,
) -> Result<()> {
    use anyhow::Context;
    use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};
    let policy = SyncPolicy::load(policy_path)?;

    if policy.standard_files.is_empty() {
        println!("No standard files configured in policy.");
        println!("Add [[standard_files]] entries to {}", policy_path.display());
        return Ok(());
    }

    let filtered_configs: Vec<_> = if files.is_empty() {
        policy.standard_files.clone()
    } else {
        policy
            .standard_files
            .iter()
            .filter(|c| files.contains(&c.target))
            .cloned()
            .collect()
    };

    if filtered_configs.is_empty() {
        println!("No matching standard files found.");
        return Ok(());
    }

    let repos = if let Some(repo_path) = repo {
        vec![repo_path]
    } else {
        let roots: Vec<PathBuf> = policy.watch_roots.iter().map(PathBuf::from).collect();
        let excluded: std::collections::BTreeSet<String> =
            policy.exclude_dir_names.iter().cloned().collect();
        git::discover_git_repos(&roots, &excluded, &policy.exclude_repos, None)
    };

    let policy_base = policy_path.parent().unwrap_or(policy_path);
    let mut results: Vec<(String, String, String)> = Vec::new();
    let mut total_copied = 0usize;

    for repo_path in &repos {
        let repo_override = policy::load_repo_override(repo_path);
        let repo_name = repo_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| repo_path.display().to_string());

        for cfg in &filtered_configs {
            if repo_override.skip_standard_files.contains(&cfg.target) {
                continue;
            }

            let target_path = repo_path.join(&cfg.target);
            if target_path.exists() && !overwrite && !cfg.overwrite {
                continue;
            }

            let source_path = cfg.source_path(policy_base);
            if !source_path.exists() {
                results.push((repo_name.clone(), cfg.target.clone(), "template missing".to_string()));
                continue;
            }

            if dry_run {
                results.push((repo_name.clone(), cfg.target.clone(), "would copy".to_string()));
                total_copied += 1;
                continue;
            }

            if target_path.exists() && (overwrite || cfg.overwrite) {
                if target_path.is_dir() {
                    std::fs::remove_dir_all(&target_path)
                        .with_context(|| format!("failed to remove {}", cfg.target))?;
                } else {
                    std::fs::remove_file(&target_path)
                        .with_context(|| format!("failed to remove {}", cfg.target))?;
                }
            }

            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }

            match std::fs::copy(&source_path, &target_path) {
                Ok(_) => {
                    results.push((repo_name.clone(), cfg.target.clone(), "copied".to_string()));
                    total_copied += 1;
                }
                Err(e) => {
                    results.push((repo_name.clone(), cfg.target.clone(), format!("error: {e}")));
                }
            }
        }
    }

    if results.is_empty() {
        println!("No standard files to scaffold (all repos already have them).");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("REPO"),
            Cell::new("FILE"),
            Cell::new("STATUS"),
        ]);

    for (repo_name, file, status) in &results {
        let (status_str, color) = match status.as_str() {
            "copied" => ("\u{2705} copied", Color::Green),
            "would copy" => ("\u{1f4dd} would copy", Color::Yellow),
            "template missing" => ("\u{274c} template missing", Color::Red),
            s if s.starts_with("error:") => ("\u{274c} error", Color::Red),
            _ => (status.as_str(), Color::White),
        };
        table.add_row(vec![
            Cell::new(repo_name),
            Cell::new(file),
            Cell::new(status_str).fg(color),
        ]);
    }

    println!("{table}");
    let mode = if dry_run { "DRY-RUN" } else { "APPLIED" };
    println!("{mode}: {total_copied} files scaffolded across {} repos", repos.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    fn temp_policy(repos: Vec<&str>) -> TempDir {
        let tmp = TempDir::new().unwrap();
        let content = format!(
            r#"
auto_github_private = false
auto_commit = true
auto_pull = true
auto_push = true
auto_bump_versions = false
watch_roots = {:?}
remotes = []
"#,
            repos
        );
        std::fs::write(tmp.path().join("policy.toml"), content).unwrap();
        tmp
    }

    #[test]
    fn test_freeze_reason_none_when_no_marker() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".dracon")).unwrap();

        let policy_tmp = temp_policy(vec!["/dev/null"]);
        let policy_path = policy_tmp.path().join("policy.toml");

        let result = crate::policy::freeze_reason(&policy_path);
        assert!(result.is_none(), "no freeze marker should return None");
    }

    #[test]
    fn test_freeze_marker_paths() {
        let paths = crate::policy::freeze_marker_paths(std::path::Path::new("/fake.toml"));
        assert!(!paths.is_empty());
        assert!(paths
            .iter()
            .any(|p| p.to_string_lossy().contains(".dracon")));
        assert!(paths.iter().any(|p| p.to_string_lossy().contains("freeze")));
    }

    #[test]
    fn test_env_freeze_takes_precedence() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".dracon")).unwrap();

        let policy_tmp = temp_policy(vec!["/dev/null"]);
        let policy_path = policy_tmp.path().join("policy.toml");

        let _guard = crate::test_helpers::EnvRestorer::new("DRACON_SYNC_FREEZE", "1");
        let result = crate::policy::freeze_reason(&policy_path);

        assert!(
            result.is_some(),
            "env freeze should override missing marker"
        );
        assert!(result.unwrap().contains("env DRACON_SYNC_FREEZE"));
    }

    #[test]
    fn test_metrics_output_has_expected_format() {
        let lines = vec![
            "# HELP dracon_sync_info Dracon sync daemon info".to_string(),
            "# TYPE dracon_sync_info gauge".to_string(),
            format!(
                "dracon_sync_info{{version=\"{}\"}} 1",
                env!("CARGO_PKG_VERSION")
            ),
            "dracon_sync_repos_discovered_total 20".to_string(),
            "# HELP dracon_sync_freeze_state gauge".to_string(),
            "dracon_sync_freeze_state 0".to_string(),
        ];

        let mut found_version_line = false;
        for line in &lines {
            if line.starts_with('#') {
                assert!(
                    line.contains(" HELP ") || line.contains(" TYPE "),
                    "comment line should be HELP or TYPE: {}",
                    line
                );
            } else {
                assert!(
                    line.contains("dracon_sync"),
                    "metric line should contain metric name: {}",
                    line
                );
                if line.contains("version=") {
                    found_version_line = true;
                }
            }
        }
        assert!(found_version_line, "version metric line should be present");
    }

    #[test]
    fn test_metrics_contains_all_expected_metrics() {
        let expected_metrics = vec![
            "dracon_sync_info",
            "dracon_sync_repos_discovered_total",
            "dracon_sync_watch_roots_total",
            "dracon_sync_remotes_total",
            "dracon_sync_freeze_state",
            "dracon_sync_policy_auto_commit",
            "dracon_sync_policy_auto_push",
            "dracon_sync_policy_auto_pull",
            "dracon_sync_push_retries",
            "dracon_sync_pulse_interval_secs",
        ];

        for metric in &expected_metrics {
            assert!(
                metric.starts_with("dracon_sync_"),
                "metric name should start with dracon_sync_: {}",
                metric
            );
        }
    }
}
