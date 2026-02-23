use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use hydra_core::adapter::probe::{AdapterTier, ProbeStatus};
use hydra_core::artifact::manifest::AgentStatus;
use hydra_core::config::HydraConfig;
use hydra_core::doctor::DoctorReport;
use hydra_core::merge::MergeService;
use hydra_core::orchestrator::Orchestrator;

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
    Race {
        /// Agent to use (claude, codex).
        #[arg(long, default_value = "claude")]
        agents: String,
        /// Task prompt describing what the agent should do.
        #[arg(short, long)]
        prompt: String,
        /// Path to hydra.toml config file.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Merge a completed run result into the main branch.
    Merge {
        /// Source branch to merge from (e.g., hydra/<run_id>/agent/claude).
        #[arg(long)]
        source: String,
        /// Target branch (default: current branch).
        #[arg(long)]
        target: Option<String>,
        /// Dry-run only (default: true).
        #[arg(long, default_value_t = true)]
        dry_run: bool,
        /// Actually perform the merge (overrides --dry-run).
        #[arg(long)]
        confirm: bool,
        /// Output results as JSON.
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<ExitCode> {
    hydra_core::init_tracing();

    let cli = Cli::parse();

    match cli.command {
        Some(Command::Doctor { json }) => Ok(run_doctor(json)),
        Some(Command::Race {
            agents,
            prompt,
            config,
        }) => run_race(&agents, &prompt, config.as_deref()).await,
        Some(Command::Merge {
            source,
            target,
            dry_run,
            confirm,
            json,
        }) => run_merge(&source, target.as_deref(), dry_run, confirm, json).await,
        None => {
            println!("hydra v0.1.0");
            Ok(ExitCode::SUCCESS)
        }
    }
}

async fn run_race(
    agent_key: &str,
    prompt: &str,
    config_path: Option<&std::path::Path>,
) -> Result<ExitCode> {
    // Load config.
    let config = match config_path {
        Some(path) => HydraConfig::load(path)
            .context(format!("failed to load config from {}", path.display()))?,
        None => HydraConfig::load_or_default(),
    };

    // Detect repo root.
    let repo_root = detect_repo_root()
        .await
        .context("failed to detect git repository root")?;

    // Register cleanup handler for Ctrl+C.
    hydra_core::worktree::register_cleanup_handler(repo_root.clone());

    let orchestrator = Orchestrator::new(config, repo_root);

    println!("Starting race with agent: {agent_key}");
    println!("Prompt: {prompt}");
    println!();

    let result = orchestrator
        .race_single(agent_key, prompt)
        .await
        .context("race failed")?;

    // Print summary.
    println!();
    println!("Race Summary");
    println!("============");
    println!("Run ID:       {}", result.run_id);
    println!("Artifact dir: {}", result.artifact_dir.display());

    let mut all_success = true;
    for agent in &result.agents {
        let status_str = match &agent.status {
            AgentStatus::Completed => "completed",
            AgentStatus::Failed => "failed",
            AgentStatus::TimedOut => "timed out",
            AgentStatus::Cancelled => "cancelled",
            AgentStatus::Running => "running",
        };
        let exit_str = agent
            .exit_code
            .map(|c| format!(" (exit code: {c})"))
            .unwrap_or_default();

        println!("Agent:        {} - {status_str}{exit_str}", agent.agent_key);
        println!("Branch:       {}", agent.branch);
        println!("Worktree:     {}", agent.worktree_path.display());

        if agent.status != AgentStatus::Completed {
            all_success = false;
        }
    }

    if all_success {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

/// Detect the git repository root via `git rev-parse --show-toplevel`.
async fn detect_repo_root() -> Result<PathBuf> {
    let output = tokio::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .await
        .context("failed to run git rev-parse")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("not inside a git repository: {stderr}");
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

async fn run_merge(
    source: &str,
    target: Option<&str>,
    dry_run: bool,
    confirm: bool,
    json_output: bool,
) -> Result<ExitCode> {
    let repo_root = detect_repo_root()
        .await
        .context("failed to detect git repository root")?;

    let target_branch = match target {
        Some(t) => t.to_string(),
        None => {
            // Use current branch as target
            let output = tokio::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&repo_root)
                .output()
                .await
                .context("failed to get current branch")?;
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
    };

    let svc = MergeService::new(repo_root);

    let is_dry_run = dry_run && !confirm;

    let report = if is_dry_run {
        svc.dry_run(source, &target_branch)
            .await
            .context("merge dry-run failed")?
    } else {
        svc.merge(source, &target_branch)
            .await
            .context("merge failed")?
    };

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("serialize MergeReport")
        );
    } else {
        print_merge_report(&report);
    }

    if report.can_merge {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

fn print_merge_report(report: &hydra_core::merge::MergeReport) {
    let mode = if report.dry_run { "DRY-RUN" } else { "MERGE" };
    println!("Hydra Merge Report ({mode})");
    println!("=========================");
    println!();
    println!("Source: {}", report.source_branch);
    println!("Target: {}", report.target_branch);
    println!();

    if report.can_merge {
        println!("Status: CAN MERGE");
    } else {
        println!("Status: CONFLICTS DETECTED");
    }

    println!(
        "Changes: {} files changed, {} insertions(+), {} deletions(-)",
        report.files_changed, report.insertions, report.deletions
    );

    if !report.conflicts.is_empty() {
        println!();
        println!("Conflicts:");
        for conflict in &report.conflicts {
            println!("  - {} ({})", conflict.path, conflict.conflict_type);
        }
    }

    if report.dry_run && report.can_merge {
        println!();
        println!("To perform the merge, run again with --confirm");
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
