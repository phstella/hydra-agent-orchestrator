use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use uuid::Uuid;

use hydra_core::artifact::{RunLayout, RunManifest};
use hydra_core::scoring::ranking::AgentScore;

#[derive(Subcommand)]
pub enum RunCommand {
    /// Show summary details for a stored run
    Show {
        /// Explicit run ID to inspect
        #[arg(long)]
        run_id: Option<Uuid>,

        /// Inspect the most recently modified run directory
        #[arg(long)]
        latest: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

pub fn run_command(command: RunCommand) -> Result<()> {
    match command {
        RunCommand::Show {
            run_id,
            latest,
            json,
        } => run_show(run_id, latest, json),
    }
}

fn run_show(run_id: Option<Uuid>, latest: bool, json: bool) -> Result<()> {
    let repo_root = discover_repo_root()?;
    let hydra_root = repo_root.join(".hydra");
    let selected_run_id = resolve_run_id(&hydra_root, run_id, latest)?;
    let layout = RunLayout::new(&hydra_root, selected_run_id);

    if !layout.base_dir().exists() {
        bail!(
            "run {} not found at {}",
            selected_run_id,
            layout.base_dir().display()
        );
    }

    let manifest =
        RunManifest::read_from(&layout.manifest_path()).context("failed to read run manifest")?;

    let mut scores_by_agent: HashMap<String, AgentScore> = HashMap::new();
    for agent in &manifest.agents {
        if let Some(score) = load_agent_score(&layout, &agent.agent_key)? {
            scores_by_agent.insert(agent.agent_key.clone(), score);
        }
    }

    let mut rankings: Vec<AgentScore> = scores_by_agent.values().cloned().collect();
    rankings.sort_by(|a, b| {
        b.composite
            .partial_cmp(&a.composite)
            .unwrap_or(Ordering::Equal)
    });

    if json {
        let winner = rankings
            .iter()
            .find(|score| score.mergeable)
            .map(|score| score.agent_key.clone());

        let agents = manifest
            .agents
            .iter()
            .map(|agent| {
                let score = scores_by_agent.get(&agent.agent_key);
                serde_json::json!({
                    "agent_key": agent.agent_key,
                    "tier": agent.tier,
                    "branch": agent.branch,
                    "worktree_path": agent.worktree_path,
                    "score": score,
                })
            })
            .collect::<Vec<_>>();

        let output = serde_json::json!({
            "run_id": manifest.run_id,
            "status": manifest.status,
            "started_at": manifest.started_at,
            "completed_at": manifest.completed_at,
            "manifest_path": layout.manifest_path(),
            "artifacts_path": layout.base_dir(),
            "winner": winner,
            "rankings": rankings,
            "agents": agents,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Run Summary");
    println!("===========");
    println!("  Run ID:    {}", manifest.run_id);
    println!("  Status:    {:?}", manifest.status);
    println!("  Started:   {}", manifest.started_at);
    if let Some(completed_at) = manifest.completed_at {
        println!("  Completed: {}", completed_at);
    }
    println!("  Artifacts: {}", layout.base_dir().display());
    println!("  Manifest:  {}", layout.manifest_path().display());
    println!();

    for agent in &manifest.agents {
        println!("  Agent:     {}", agent.agent_key);
        println!("    Tier:      {}", agent.tier);
        println!("    Branch:    {}", agent.branch);
        if let Some(score) = scores_by_agent.get(&agent.agent_key) {
            println!(
                "    Score:     {:.1} ({})",
                score.composite,
                if score.mergeable {
                    "mergeable"
                } else {
                    "not mergeable"
                }
            );
            if !score.gate_failures.is_empty() {
                println!("    Gates:     {}", score.gate_failures.join("; "));
            }
        } else {
            println!("    Score:     unavailable");
        }
        println!();
    }

    println!("  Rankings:");
    if rankings.is_empty() {
        println!("    (none)");
    } else {
        for (idx, score) in rankings.iter().enumerate() {
            println!(
                "    {}. {} {:.1} {}",
                idx + 1,
                score.agent_key,
                score.composite,
                if score.mergeable {
                    "(mergeable)"
                } else {
                    "(not mergeable)"
                }
            );
        }
    }

    Ok(())
}

fn resolve_run_id(hydra_root: &Path, run_id: Option<Uuid>, latest: bool) -> Result<Uuid> {
    if run_id.is_some() && latest {
        bail!("use either --run-id or --latest, not both");
    }

    match run_id {
        Some(id) => Ok(id),
        None => latest_run_id(hydra_root),
    }
}

fn latest_run_id(hydra_root: &Path) -> Result<Uuid> {
    let runs_dir = hydra_root.join("runs");
    if !runs_dir.exists() {
        bail!("no runs found at {}", runs_dir.display());
    }

    let mut candidates: Vec<(Uuid, u128)> = Vec::new();
    for entry in std::fs::read_dir(&runs_dir).context("failed to read runs directory")? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        let Ok(id) = Uuid::parse_str(name) else {
            continue;
        };

        let modified_ns = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        candidates.push((id, modified_ns));
    }

    candidates
        .into_iter()
        .max_by_key(|(_, modified_ns)| *modified_ns)
        .map(|(id, _)| id)
        .ok_or_else(|| anyhow::anyhow!("no valid run directories found at {}", runs_dir.display()))
}

fn load_agent_score(layout: &RunLayout, agent_key: &str) -> Result<Option<AgentScore>> {
    let score_path = layout.agent_score(agent_key);
    if !score_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&score_path).with_context(|| {
        format!(
            "failed to read score file for agent '{}' at {}",
            agent_key,
            score_path.display()
        )
    })?;
    let score: AgentScore = serde_json::from_str(&content).with_context(|| {
        format!(
            "failed to parse score file for agent '{}' at {}",
            agent_key,
            score_path.display()
        )
    })?;
    Ok(Some(score))
}

fn discover_repo_root() -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git rev-parse")?;

    if !output.status.success() {
        bail!("not inside a git repository");
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolve_run_id_rejects_conflicting_flags() {
        let hydra_root = PathBuf::from("/tmp/does-not-matter");
        let run_id = Uuid::new_v4();
        let result = resolve_run_id(&hydra_root, Some(run_id), true);
        assert!(result.is_err());
    }

    #[test]
    fn latest_run_id_selects_newest_directory() {
        let tmp = TempDir::new().unwrap();
        let hydra_root = tmp.path().join(".hydra");
        let runs_dir = hydra_root.join("runs");
        std::fs::create_dir_all(&runs_dir).unwrap();

        let id_old = Uuid::new_v4();
        let id_new = Uuid::new_v4();
        std::fs::create_dir_all(runs_dir.join(id_old.to_string())).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        std::fs::create_dir_all(runs_dir.join(id_new.to_string())).unwrap();

        let selected = latest_run_id(&hydra_root).unwrap();
        assert_eq!(selected, id_new);
    }
}
