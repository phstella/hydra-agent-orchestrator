use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hydra", version, about = "Hydra agent orchestrator")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Check that required agents and tools are available.
    Doctor,
    /// Start an agent race on the current task.
    Race,
    /// Merge a completed run result into the main branch.
    Merge,
}

#[tokio::main]
async fn main() -> Result<()> {
    hydra_core::init_tracing();

    let cli = Cli::parse();

    match cli.command {
        Some(Command::Doctor) => {
            tracing::info!("doctor subcommand (stub)");
        }
        Some(Command::Race) => {
            tracing::info!("race subcommand (stub)");
        }
        Some(Command::Merge) => {
            tracing::info!("merge subcommand (stub)");
        }
        None => {
            println!("hydra v0.1.0");
        }
    }

    Ok(())
}
