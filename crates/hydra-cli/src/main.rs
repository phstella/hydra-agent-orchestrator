use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use hydra_core::adapter::claude::ClaudeAdapter;
use hydra_core::adapter::codex::CodexAdapter;
use hydra_core::adapter::cursor::CursorAdapter;
use hydra_core::adapter::{AgentAdapter, ProbeRunner};

mod doctor;
mod merge;
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
    /// Run agents on a task in isolated worktrees
    Race {
        /// Agents to run (comma-separated, e.g. "claude,codex")
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,

        /// Task prompt for the agents
        #[arg(long, short = 'p')]
        prompt: String,

        /// Base git ref to branch from (default: HEAD)
        #[arg(long, default_value = "HEAD")]
        base_ref: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Allow agent execution outside strict worktree sandbox controls
        #[arg(long = "unsafe")]
        unsafe_mode: bool,

        /// Allow experimental (non-Tier-1) adapters to participate in the race
        #[arg(long)]
        allow_experimental_adapters: bool,
    },
    /// Merge an agent's branch from a completed race run
    Merge {
        /// Run ID to merge from
        #[arg(long)]
        run_id: uuid::Uuid,

        /// Specific agent to merge (defaults to highest-scoring mergeable)
        #[arg(long)]
        agent: Option<String>,

        /// Preview merge without modifying working tree
        #[arg(long)]
        dry_run: bool,

        /// Confirm and execute the merge
        #[arg(long)]
        confirm: bool,

        /// Force merge even if agent is not marked mergeable
        #[arg(long)]
        force: bool,

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
            unsafe_mode,
            allow_experimental_adapters,
        } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(race::run_race(race::RaceOpts {
                agents,
                prompt,
                base_ref,
                json,
                unsafe_mode,
                allow_experimental_adapters,
            }))?;
        }
        Commands::Merge {
            run_id,
            agent,
            dry_run,
            confirm,
            force,
            json,
        } => {
            merge::run_merge(merge::MergeOpts {
                run_id,
                agent,
                dry_run,
                confirm,
                force,
                json,
            })?;
        }
    }

    Ok(())
}
