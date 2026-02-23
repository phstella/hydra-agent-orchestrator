use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

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
            let runner = hydra_core::adapter::ProbeRunner::new(vec![]);
            let report = runner.run();

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("Hydra Doctor Report");
                println!("===================");
                println!(
                    "All Tier-1 adapters ready: {}",
                    if report.all_tier1_ready { "yes" } else { "NO" }
                );
                println!();
                if report.results.is_empty() {
                    println!("No adapters registered.");
                }
                for r in &report.results {
                    println!(
                        "  [{}] {} ({}): {:?}",
                        r.tier,
                        r.adapter_key,
                        r.detect.status_label(),
                        r.detect.status
                    );
                    if let Some(path) = &r.detect.binary_path {
                        println!("    binary: {}", path.display());
                    }
                    if let Some(v) = &r.detect.version {
                        println!("    version: {}", v);
                    }
                    if let Some(err) = &r.detect.error {
                        println!("    error: {}", err);
                    }
                }
            }

            if !report.all_tier1_ready {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
