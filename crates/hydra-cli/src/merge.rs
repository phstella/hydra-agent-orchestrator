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

enum MergeInput {
    Branch { branch: String },
    DiffPatch { branch: String, patch_path: PathBuf },
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

    let branch = entry.branch.clone();

    // Pre-flight safety checks
    check_not_in_merge_state(&repo_root)?;
    check_clean_working_tree(&repo_root)?;
    let merge_input = resolve_merge_input(&repo_root, &layout, &agent_key, &branch)?;

    if opts.dry_run {
        return run_dry_merge(&repo_root, &merge_input, &layout, &agent_key, opts.json);
    }

    if !opts.confirm {
        bail!("merge requires --confirm flag (or use --dry-run to preview)");
    }

    run_real_merge(
        &repo_root,
        &merge_input,
        &layout,
        opts.run_id,
        &agent_key,
        opts.json,
    )
}

fn resolve_merge_input(
    repo_root: &Path,
    layout: &RunLayout,
    agent_key: &str,
    branch: &str,
) -> Result<MergeInput> {
    if branch_exists(repo_root, branch)? {
        return Ok(MergeInput::Branch {
            branch: branch.to_string(),
        });
    }

    let diff_path = layout.agent_diff(agent_key);
    if diff_path.exists() {
        return Ok(MergeInput::DiffPatch {
            branch: branch.to_string(),
            patch_path: diff_path,
        });
    }

    bail!(
        "branch '{}' does not exist and no persisted diff artifact found at {}. \
         The worktree may have been cleaned up already",
        branch,
        layout.agent_diff(agent_key).display()
    );
}

fn run_dry_merge(
    repo_root: &Path,
    input: &MergeInput,
    layout: &RunLayout,
    agent_key: &str,
    json: bool,
) -> Result<()> {
    let (branch, merge_stdout, merge_stderr, has_conflicts, source) = match input {
        MergeInput::Branch { branch } => {
            let had_merge_state_before = merge_head_exists(repo_root)?;

            let merge_output = std::process::Command::new("git")
                .args(["merge", "--no-commit", "--no-ff", branch])
                .current_dir(repo_root)
                .output()
                .context("failed to run git merge --no-commit")?;

            let merge_stdout = String::from_utf8_lossy(&merge_output.stdout).to_string();
            let merge_stderr = String::from_utf8_lossy(&merge_output.stderr).to_string();
            let has_conflicts = !merge_output.status.success();

            let has_merge_state_after = merge_head_exists(repo_root)?;

            // Abort only when this dry-run actually opened a merge state.
            if !had_merge_state_before && has_merge_state_after {
                let abort_output = std::process::Command::new("git")
                    .args(["merge", "--abort"])
                    .current_dir(repo_root)
                    .output()
                    .context("failed to run git merge --abort after dry-run")?;
                if !abort_output.status.success() {
                    let stderr = String::from_utf8_lossy(&abort_output.stderr)
                        .trim()
                        .to_string();
                    bail!(
                        "dry-run merge created merge state but failed to abort{}",
                        if stderr.is_empty() {
                            String::new()
                        } else {
                            format!(": {stderr}")
                        }
                    );
                }
            }

            (
                branch.as_str(),
                merge_stdout,
                merge_stderr,
                has_conflicts,
                "branch".to_string(),
            )
        }
        MergeInput::DiffPatch { branch, patch_path } => {
            let patch_arg = patch_path.to_string_lossy().to_string();
            let merge_output = std::process::Command::new("git")
                .args(["apply", "--check", "--3way", &patch_arg])
                .current_dir(repo_root)
                .output()
                .context("failed to run git apply --check")?;

            (
                branch.as_str(),
                String::from_utf8_lossy(&merge_output.stdout).to_string(),
                String::from_utf8_lossy(&merge_output.stderr).to_string(),
                !merge_output.status.success(),
                "patch".to_string(),
            )
        }
    };

    let report = MergeReport {
        agent: agent_key.to_string(),
        branch: branch.to_string(),
        dry_run: true,
        success: !has_conflicts,
        has_conflicts,
        stdout: merge_stdout.clone(),
        stderr: merge_stderr.clone(),
        source: source.clone(),
    };
    let (report_path, report_json) = write_merge_report(layout, &report)?;

    if json {
        println!("{report_json}");
    } else if has_conflicts {
        println!(
            "Dry-run merge of '{agent_key}' source '{source}' targeting '{branch}': CONFLICTS DETECTED"
        );
        println!();
        if !merge_stderr.is_empty() {
            println!("{merge_stderr}");
        }
        println!("Report saved to: {}", report_path.display());
        std::process::exit(1);
    } else {
        println!(
            "Dry-run merge of '{agent_key}' source '{source}' targeting '{branch}': clean merge (no conflicts)"
        );
        println!("Report saved to: {}", report_path.display());
    }

    Ok(())
}

fn run_real_merge(
    repo_root: &Path,
    input: &MergeInput,
    layout: &RunLayout,
    run_id: Uuid,
    agent_key: &str,
    json: bool,
) -> Result<()> {
    match input {
        MergeInput::Branch { branch } => {
            run_real_branch_merge(repo_root, branch, layout, run_id, agent_key, json)
        }
        MergeInput::DiffPatch { branch, patch_path } => run_real_patch_merge(
            repo_root, branch, patch_path, layout, run_id, agent_key, json,
        ),
    }
}

fn run_real_branch_merge(
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
            source: "branch".to_string(),
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

fn run_real_patch_merge(
    repo_root: &Path,
    branch: &str,
    patch_path: &Path,
    layout: &RunLayout,
    run_id: Uuid,
    agent_key: &str,
    json: bool,
) -> Result<()> {
    let patch_arg = patch_path.to_string_lossy().to_string();

    let check_output = std::process::Command::new("git")
        .args(["apply", "--check", "--3way", &patch_arg])
        .current_dir(repo_root)
        .output()
        .context("failed to run git apply --check before merge")?;

    if !check_output.status.success() {
        let stdout = String::from_utf8_lossy(&check_output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&check_output.stderr).to_string();
        let report = MergeReport {
            agent: agent_key.to_string(),
            branch: branch.to_string(),
            dry_run: false,
            success: false,
            has_conflicts: true,
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            source: "patch".to_string(),
        };
        let (_report_path, report_json) = write_merge_report(layout, &report)?;

        if json {
            println!("{report_json}");
        } else {
            eprintln!("Merge failed: {}", stderr.trim());
        }
        std::process::exit(1);
    }

    let apply_output = std::process::Command::new("git")
        .args(["apply", "--index", "--3way", &patch_arg])
        .current_dir(repo_root)
        .output()
        .context("failed to run git apply --index")?;

    if !apply_output.status.success() {
        let stdout = String::from_utf8_lossy(&apply_output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&apply_output.stderr).to_string();
        let report = MergeReport {
            agent: agent_key.to_string(),
            branch: branch.to_string(),
            dry_run: false,
            success: false,
            has_conflicts: true,
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            source: "patch".to_string(),
        };
        let (_report_path, report_json) = write_merge_report(layout, &report)?;

        if json {
            println!("{report_json}");
        } else {
            eprintln!("Merge failed: {}", stderr.trim());
        }
        std::process::exit(1);
    }

    let staged_status = std::process::Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(repo_root)
        .status()
        .context("failed to inspect staged changes")?;
    let has_staged_changes = !staged_status.success();

    let message = format!("hydra: merge {agent_key} from run {run_id}");
    if has_staged_changes {
        let commit_output = std::process::Command::new("git")
            .args(["commit", "-m", &message])
            .current_dir(repo_root)
            .output()
            .context("failed to create merge commit from patch")?;
        if !commit_output.status.success() {
            let stdout = String::from_utf8_lossy(&commit_output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&commit_output.stderr).to_string();
            let report = MergeReport {
                agent: agent_key.to_string(),
                branch: branch.to_string(),
                dry_run: false,
                success: false,
                has_conflicts: false,
                stdout: stdout.clone(),
                stderr: stderr.clone(),
                source: "patch".to_string(),
            };
            let (_report_path, report_json) = write_merge_report(layout, &report)?;

            if json {
                println!("{report_json}");
            } else {
                eprintln!("Merge commit failed: {}", stderr.trim());
            }
            std::process::exit(1);
        }
    }

    if json {
        let report = serde_json::json!({
            "agent": agent_key,
            "branch": branch,
            "success": true,
            "message": if has_staged_changes {
                message.clone()
            } else {
                format!("No changes to apply for '{}' (patch already present)", agent_key)
            },
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if has_staged_changes {
        println!(
            "Merged '{agent_key}' by applying persisted patch artifact (branch '{branch}' was unavailable)"
        );
    } else {
        println!(
            "No changes applied for '{agent_key}' because the patch already matches the current tree"
        );
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
    if merge_head_exists(repo_root)? {
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

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!(
            "git status failed{}",
            if stderr.is_empty() {
                String::new()
            } else {
                format!(": {stderr}")
            }
        );
    }

    let dirty_files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_porcelain_path)
        .filter(|path| !is_hydra_artifact_path(path))
        .collect();

    if !dirty_files.is_empty() {
        let shown: Vec<&str> = dirty_files.iter().take(5).map(|s| s.as_str()).collect();
        let extra = dirty_files.len().saturating_sub(shown.len());
        let suffix = if extra > 0 {
            format!(" (+{extra} more)")
        } else {
            String::new()
        };
        bail!(
            "working tree has uncommitted changes in: {}{}. \
             Commit or stash changes before running hydra merge",
            shown.join(", "),
            suffix
        );
    }
    Ok(())
}

fn parse_porcelain_path(line: &str) -> Option<String> {
    let path = line.get(3..)?.trim();
    if path.is_empty() {
        return None;
    }
    if let Some((_, new_path)) = path.rsplit_once(" -> ") {
        Some(new_path.to_string())
    } else {
        Some(path.to_string())
    }
}

fn is_hydra_artifact_path(path: &str) -> bool {
    let normalized = path.trim().trim_start_matches("./").replace('\\', "/");
    normalized == ".hydra" || normalized.starts_with(".hydra/")
}

fn branch_exists(repo_root: &Path, branch: &str) -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--verify", &format!("refs/heads/{branch}")])
        .current_dir(repo_root)
        .output()
        .context("failed to verify branch")?;

    if !output.status.success() {
        return Ok(false);
    }
    Ok(true)
}

fn merge_head_exists(repo_root: &Path) -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "-q", "--verify", "MERGE_HEAD"])
        .current_dir(repo_root)
        .output()
        .context("failed to inspect merge state")?;

    if output.status.success() {
        return Ok(true);
    }

    if output.status.code() == Some(1) {
        return Ok(false);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    bail!(
        "failed to inspect merge state{}",
        if stderr.is_empty() {
            String::new()
        } else {
            format!(": {stderr}")
        }
    )
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
    source: String,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    use tempfile::tempdir;

    use super::{check_clean_working_tree, is_hydra_artifact_path, parse_porcelain_path};

    #[test]
    fn parses_porcelain_paths_for_renames() {
        assert_eq!(
            parse_porcelain_path("R  src/old.rs -> src/new.rs"),
            Some("src/new.rs".to_string())
        );
    }

    #[test]
    fn matches_only_hydra_artifact_paths() {
        assert!(is_hydra_artifact_path(".hydra"));
        assert!(is_hydra_artifact_path(".hydra/runs/test/events.jsonl"));
        assert!(is_hydra_artifact_path("./.hydra/worktrees/run/claude"));
        assert!(!is_hydra_artifact_path("src/.hydra-note.md"));
        assert!(!is_hydra_artifact_path("src/main.rs"));
    }

    #[test]
    fn clean_check_ignores_hydra_artifacts_but_flags_real_changes() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path();

        let status = Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .status()
            .expect("git init");
        assert!(status.success());

        let hydra_events = repo.join(".hydra/runs/test/events.jsonl");
        fs::create_dir_all(hydra_events.parent().expect("parent"))
            .expect("mkdir -p .hydra/runs/test");
        fs::write(&hydra_events, "{}\n").expect("write events");
        check_clean_working_tree(repo).expect("hydra artifacts should not block merge");

        let src_file = repo.join("src/main.rs");
        fs::create_dir_all(src_file.parent().expect("parent")).expect("mkdir -p src");
        fs::write(&src_file, "fn main() {}\n").expect("write src");
        let err = check_clean_working_tree(repo).expect_err("real source changes must block merge");
        let msg = err.to_string();
        assert!(msg.contains("working tree has uncommitted changes in"));
        assert!(msg.contains("src"));
    }
}
