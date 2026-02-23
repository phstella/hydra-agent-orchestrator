use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use hydra_core::adapter::claude::ClaudeAdapter;
use hydra_core::adapter::codex::CodexAdapter;
use hydra_core::adapter::cursor::CursorAdapter;
use hydra_core::adapter::{AgentAdapter, ProbeRunner};

mod doctor;

#[derive(Parser)]
#[command(name = "hydra", about = "Multi-agent orchestration control center")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check adapter readiness and system health
    Doctor {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor { json } => {
            let adapter_paths = doctor::load_adapter_path_overrides();
            let adapters: Vec<Box<dyn AgentAdapter>> = vec![
                Box::new(ClaudeAdapter::new(adapter_paths.claude)),
                Box::new(CodexAdapter::new(adapter_paths.codex)),
                Box::new(CursorAdapter::new(adapter_paths.cursor)),
            ];

            let runner = ProbeRunner::new(adapters);
            let probe_report = runner.run();
            let git_checks = doctor::check_git_repo();
            let report = doctor::DoctorReport::new(probe_report, git_checks);

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                doctor::print_human_report(&report);
            }

            if !report.healthy() {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
