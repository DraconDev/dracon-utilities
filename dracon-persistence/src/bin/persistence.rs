use anyhow::Result;
use clap::{Parser, Subcommand};
use dracon_persistence_kit::{repository::GitSyncer, pulse::PersistencePulse, RepositorySyncer};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "dracon-persistence", about = "Dracon Persistence - Autonomous Repository Manager", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the autonomous persistence daemon
    Daemon {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        #[arg(short, long, default_value = "300")]
        interval: u64,
    },
    /// Perform a single sync right now
    Now {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Commands::Daemon { path, interval } => {
            let syncer = Arc::new(GitSyncer::new(path));
            let pulse = PersistencePulse::new(syncer, interval);
            pulse.run().await;
        }
        Commands::Now { path } => {
            let syncer = GitSyncer::new(path);
            let status = syncer.status().await?;
            println!("Branch: {} (Clean: {})", status.branch, status.is_clean);
            syncer.sync().await?;
        }
    }

    Ok(())
}
