use anyhow::Result;
use clap::{Parser, Subcommand};
use dracon_security_kit::{DraconWarden, Warden};
use std::io::{self, Read, Write};

#[derive(Parser)]
#[command(name = "dracon-security", about = "Dracon Security - Autonomous Secret Protection", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encrypt sensitive patterns from stdin to stdout
    Clean {
        #[arg(long)]
        path: Option<String>,
    },
    /// Decrypt sensitive tags from stdin to stdout
    Smudge {
        #[arg(long)]
        path: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let warden = DraconWarden::new()?;

    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer)?;

    match cli.cmd {
        Commands::Clean { path } => {
            let output = warden.clean(&buffer, path.as_deref())?;
            io::stdout().write_all(&output)?;
        }
        Commands::Smudge { path } => {
            let output = warden.smudge(&buffer, path.as_deref())?;
            io::stdout().write_all(&output)?;
        }
    }

    Ok(())
}
