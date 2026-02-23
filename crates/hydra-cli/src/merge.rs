use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use uuid::Uuid;

use hydra_core::artifact::{RunLayout, RunManifest, RunStatus};
use hydra_core::scoring::ranking::AgentScore;

pub struct MergeOpts {
    pub run_id: Uuid,
    pub agent: Option<String>,
    pub dry_run: bool,
    pub confirm: bool,
    pub force: bool,
    pub json: bool,
}

pub fn run_merge(opts: MergeOpts) -> Result<()> {
    let repo_root = discover_repo_root()?;
    let hydra_root = repo_root.join(".hydra");
    let layout = RunLayout::new(&hydra_root, opts.run_id);

    if !layout.base_dir().exists() {
        bail!(
            "run {} not found at {}",
            opts.run_id,
            layout.base_dir().display()
        );
    }

    let manifest =
        RunManifest::read_from(&layout.manifest_path()).context("failed to read manifest")?;

    if manifest.status != RunStatus::Completed {
        bail!(
            "run {} has status {:?}, not Completed",
            opts.run_id,
            manifest.status
        );
    }

    let agent_key = match &opts.agent {
        Some(key) => key.clone(),
        None => pick_winner(&layout, &manifest)?,
    };

    let entry = manifest
        .agents
        .iter()
        .find(|a| a.agent_key == agent_key)
        .ok_or_else(|| anyhow::anyhow!("agent '{}' not found in run {}", agent_key, opts.run_id))?;

    if !opts.force {
        if let Ok(score) = load_agent_score(&layout, &agent_key) {
            if !score.mergeable {
                bail!(
                    "agent '{}' is not mergeable (gate failures: {}). Use --force to override",
                    agent_key,
                    score.gate_failures.join(", ")
                );
            }
        }
    }

    let branch = &entry.branch;

    // Pre-flight safety checks
    check_not_in_merge_state(&repo_root)?;
    check_clean_working_tree(&repo_root)?;
    check_branch_exists(&repo_root, branch)?;

    if opts.dry_run {
        return run_dry_merge(&repo_root, branch, &layout, &agent_key, opts.json);
    }

    if !opts.confirm {
        bail!("merge requires --confirm flag (or use --dry-run to preview)");
    }

    run_real_merge(
        &repo_root,
        branch,
        &layout,
        opts.run_id,
        &agent_key,
        opts.json,
    )
}

fn run_dry_merge(
    repo_root: &Path,
    branch: &str,
    layout: &RunLayout,
    agent_key: &str,
    json: bool,
) -> Result<()> {
    let merge_output = std::process::Command::new("git")
        .args(["merge", "--no-commit", "--no-ff", branch])
        .current_dir(repo_root)
        .output()
        .context("failed to run git merge --no-commit")?;

    let merge_stdout = String::from_utf8_lossy(&merge_output.stdout).to_string();
    let merge_stderr = String::from_utf8_lossy(&merge_output.stderr).to_string();
    let has_conflicts = !merge_output.status.success();

    // Always abort the merge to restore working tree
    let _ = std::process::Command::new("git")
        .args(["merge", "--abort"])
        .current_dir(repo_root)
        .output();

    let report = MergeReport {
        agent: agent_key.to_string(),
        branch: branch.to_string(),
        dry_run: true,
        success: !has_conflicts,
        has_conflicts,
        stdout: merge_stdout.clone(),
        stderr: merge_stderr.clone(),
    };
    let (report_path, report_json) = write_merge_report(layout, &report)?;

    if json {
        println!("{report_json}");
    } else if has_conflicts {
        println!("Dry-run merge of '{agent_key}' branch '{branch}': CONFLICTS DETECTED");
        println!();
        if !merge_stderr.is_empty() {
            println!("{merge_stderr}");
        }
        println!("Report saved to: {}", report_path.display());
        std::process::exit(1);
    } else {
        println!("Dry-run merge of '{agent_key}' branch '{branch}': clean merge (no conflicts)");
        println!("Report saved to: {}", report_path.display());
    }

    Ok(())
}

fn run_real_merge(
    repo_root: &Path,
    branch: &str,
    layout: &RunLayout,
    run_id: Uuid,
    agent_key: &str,
    json: bool,
) -> Result<()> {
    let message = format!("hydra: merge {agent_key} from run {run_id}");

    let output = std::process::Command::new("git")
        .args(["merge", "--no-ff", branch, "-m", &message])
        .current_dir(repo_root)
        .output()
        .context("failed to run git merge")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let _ = std::process::Command::new("git")
            .args(["merge", "--abort"])
            .current_dir(repo_root)
            .output();

        let report = MergeReport {
            agent: agent_key.to_string(),
            branch: branch.to_string(),
            dry_run: false,
            success: false,
            has_conflicts: true,
            stdout: stdout.clone(),
            stderr: stderr.clone(),
        };
        let (_report_path, report_json) = write_merge_report(layout, &report)?;

        if json {
            println!("{report_json}");
        } else {
            eprintln!("Merge failed: {}", stderr.trim());
        }
        std::process::exit(1);
    }

    if json {
        let report = serde_json::json!({
            "agent": agent_key,
            "branch": branch,
            "success": true,
            "message": message,
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Merged '{agent_key}' branch '{branch}'");
        if !stdout.is_empty() {
            println!("{stdout}");
        }
    }

    Ok(())
}

fn write_merge_report(layout: &RunLayout, report: &MergeReport) -> Result<(PathBuf, String)> {
    let report_path = layout.base_dir().join("merge_report.json");
    let report_json =
        serde_json::to_string_pretty(report).context("failed to serialize merge report")?;
    std::fs::write(&report_path, &report_json).context("failed to write merge report")?;
    Ok((report_path, report_json))
}

fn pick_winner(layout: &RunLayout, manifest: &RunManifest) -> Result<String> {
    let mut best: Option<(String, f64)> = None;

    for agent in &manifest.agents {
        if let Ok(score) = load_agent_score(layout, &agent.agent_key) {
            if score.mergeable {
                match &best {
                    None => best = Some((agent.agent_key.clone(), score.composite)),
                    Some((_, best_score)) if score.composite > *best_score => {
                        best = Some((agent.agent_key.clone(), score.composite));
                    }
                    _ => {}
                }
            }
        }
    }

    best.map(|(k, _)| k).ok_or_else(|| {
        anyhow::anyhow!("no mergeable agent found in run. Use --agent to specify explicitly")
    })
}

fn load_agent_score(layout: &RunLayout, agent_key: &str) -> Result<AgentScore> {
    let path = layout.agent_score(agent_key);
    if !path.exists() {
        bail!("score file not found for agent '{agent_key}'");
    }
    let data = std::fs::read_to_string(&path)?;
    let score: AgentScore = serde_json::from_str(&data)?;
    Ok(score)
}

fn check_not_in_merge_state(repo_root: &Path) -> Result<()> {
    let merge_head = repo_root.join(".git/MERGE_HEAD");
    if merge_head.exists() {
        bail!(
            "repository is already in a merge state. \
             Resolve or abort the current merge before running hydra merge"
        );
    }
    Ok(())
}

fn check_clean_working_tree(repo_root: &Path) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output()
        .context("failed to run git status")?;

    if output.status.success() && !output.stdout.is_empty() {
        bail!(
            "working tree has uncommitted changes. \
             Commit or stash changes before running hydra merge"
        );
    }
    Ok(())
}

fn check_branch_exists(repo_root: &Path, branch: &str) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--verify", &format!("refs/heads/{branch}")])
        .current_dir(repo_root)
        .output()
        .context("failed to verify branch")?;

    if !output.status.success() {
        bail!(
            "branch '{}' does not exist. \
             The worktree may have been cleaned up already",
            branch
        );
    }
    Ok(())
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

#[derive(serde::Serialize)]
struct MergeReport {
    agent: String,
    branch: String,
    dry_run: bool,
    success: bool,
    has_conflicts: bool,
    stdout: String,
    stderr: String,
}
