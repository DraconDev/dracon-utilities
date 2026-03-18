mod policy;
mod exclude;
mod git;
mod bump;
mod scribe;
mod report;
mod daemon;
mod sync;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use policy::{resolve_policy_path, SyncPolicy};
use policy::freeze_reason;
use exclude::excluded_dir_names_set;
use report::{ConcernRepairFilter, RepoFilter, push_large_blob_threshold_bytes, run_repair_concerns, run_repair_warns, run_repos_report};
use daemon::{run_once, run_daemon};
use sync::sync_repo;

#[derive(Parser, Debug)]
#[command(name = "dracon-sync")]
#[command(about = "Dracon sync runtime")]
struct Cli {
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
    Daemon,
    /// Sync a specific repository now.
    SyncNow { repo: PathBuf },
    /// Open sync policy in the system editor.
    EditConfig,
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
    let policy_path = resolve_policy_path()?;

    match cli.cmd {
        Command::Status { json } => {
            let policy = SyncPolicy::load(&policy_path)?;
            let roots = policy.watch_root_paths();
            let excluded_dir_names = excluded_dir_names_set(&policy);
            let repos = git::discover_git_repos(&roots, &excluded_dir_names);
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
                    extra_remotes: policy.extra_remotes.len(),
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
                println!("🌐 EXTRA_REMOTES: {}", policy.extra_remotes.len());
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
        Command::Daemon => {
            run_daemon(policy_path).await?;
        }
        Command::SyncNow { repo } => {
            if let Some(reason) = freeze_reason(&policy_path) {
                println!("⏸️ sync frozen ({})", reason);
                return Ok(());
            }
            let policy = SyncPolicy::load(&policy_path)?;
            let excluded_dir_names = excluded_dir_names_set(&policy);
            if sync_repo(&repo, &policy, &excluded_dir_names, 0).await? {
                println!("🔁 synced {}", repo.display());
            } else {
                println!("✅ no sync changes {}", repo.display());
            }
        }
        Command::EditConfig => {
            policy::open_policy_in_editor(&policy_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Tests from the original main.rs will be added here as needed
}
// test
