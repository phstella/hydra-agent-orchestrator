use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

use hydra_core::adapter::probe::{AdapterTier, ProbeStatus};
use hydra_core::doctor::DoctorReport;

#[derive(Parser)]
#[command(name = "hydra", version, about = "Hydra agent orchestrator")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Check that required agents and tools are available.
    Doctor {
        /// Output results as JSON instead of human-readable text.
        #[arg(long)]
        json: bool,
    },
    /// Start an agent race on the current task.
    Race,
    /// Merge a completed run result into the main branch.
    Merge,
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    hydra_core::init_tracing();

    let cli = Cli::parse();

    match cli.command {
        Some(Command::Doctor { json }) => Ok(run_doctor(json)),
        Some(Command::Race) => {
            tracing::info!("race subcommand (stub)");
            Ok(ExitCode::SUCCESS)
        }
        Some(Command::Merge) => {
            tracing::info!("merge subcommand (stub)");
            Ok(ExitCode::SUCCESS)
        }
        None => {
            println!("hydra v0.1.0");
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn run_doctor(json_output: bool) -> ExitCode {
    let report = DoctorReport::run(None);

    if json_output {
        // Safe: DoctorReport derives Serialize so this won't fail.
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("serialize DoctorReport")
        );
    } else {
        print_human_report(&report);
    }

    if report.overall_ready {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn print_human_report(report: &DoctorReport) {
    println!("Hydra Doctor Report");
    println!("===================");
    println!();

    // Git section
    println!("Git:");
    if report.git.git_available {
        let ver = report.git.git_version.as_deref().unwrap_or("unknown");
        println!("  \u{2713} git available (version {ver})");
    } else {
        println!("  \u{2717} git not available");
    }

    if report.git.in_git_repo {
        println!("  \u{2713} inside git repository");
    } else {
        println!("  \u{2717} not inside a git repository");
    }

    println!();

    // Adapters section
    println!("Adapters:");
    for adapter in &report.adapters.adapters {
        let tier_label = match adapter.tier {
            AdapterTier::Tier1 => "Tier-1",
            AdapterTier::Experimental => "Experimental",
        };

        let (icon, status_text) = match adapter.status {
            ProbeStatus::Ready => ("\u{2713}", "ready"),
            ProbeStatus::Blocked => ("\u{2717}", "blocked"),
            ProbeStatus::Missing => ("\u{2717}", "missing"),
            ProbeStatus::ExperimentalReady => ("\u{2713}", "ready"),
            ProbeStatus::ExperimentalBlocked => ("\u{2717}", "blocked"),
        };

        // Use '-' prefix for missing experimental adapters (they are optional)
        let icon = if adapter.tier == AdapterTier::Experimental
            && adapter.status == ProbeStatus::Missing
        {
            "-"
        } else {
            icon
        };

        let version_part = adapter
            .version
            .as_ref()
            .map(|v| format!(" ({v})"))
            .unwrap_or_default();

        println!(
            "  {icon} {}: {status_text}{version_part} [{tier_label}]",
            adapter.adapter_key
        );
    }

    println!();

    // Overall
    if report.overall_ready {
        println!("Overall: READY");
    } else {
        let reasons = report.not_ready_reasons();
        let reason_text = if reasons.is_empty() {
            String::new()
        } else {
            format!(" ({})", reasons.join("; "))
        };
        println!("Overall: NOT READY{reason_text}");
    }
}
