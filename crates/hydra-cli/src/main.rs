use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use hydra_core::adapter::claude::ClaudeAdapter;
use hydra_core::adapter::codex::CodexAdapter;
use hydra_core::adapter::cursor::CursorAdapter;
use hydra_core::adapter::{AgentAdapter, ProbeRunner};

mod doctor;
mod race;

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
    /// Run an agent on a task in an isolated worktree
    Race {
        /// Agent to run (claude, codex)
        #[arg(long)]
        agents: String,

        /// Task prompt for the agent
        #[arg(long, short = 'p')]
        prompt: String,

        /// Base git ref to branch from (default: HEAD)
        #[arg(long, default_value = "HEAD")]
        base_ref: String,

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
            let adapter_cfg = doctor::load_adapter_config();
            let adapters: Vec<Box<dyn AgentAdapter>> = vec![
                Box::new(ClaudeAdapter::new(adapter_cfg.claude)),
                Box::new(CodexAdapter::new(adapter_cfg.codex)),
                Box::new(CursorAdapter::new(adapter_cfg.cursor)),
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
        Commands::Race {
            agents,
            prompt,
            base_ref,
            json,
        } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(race::run_race(race::RaceOpts {
                agent: agents,
                prompt,
                base_ref,
                json,
            }))?;
        }
    }

    Ok(())
}
