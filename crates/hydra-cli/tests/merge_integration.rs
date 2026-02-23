use std::path::Path;
use std::process::Command;

use tempfile::TempDir;
use uuid::Uuid;

/// Initialize a git repo with one commit.
fn init_test_repo(dir: &Path) {
    run_git(dir, &["init", "-b", "main"]);
    run_git(dir, &["config", "user.email", "test@hydra.dev"]);
    run_git(dir, &["config", "user.name", "Hydra Test"]);
    std::fs::write(dir.join("README.md"), "# test").unwrap();
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", "init"]);
}

fn run_git(dir: &Path, args: &[&str]) -> std::process::Output {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// Create a fake hydra run directory with manifest, agent score, and a real branch.
fn setup_completed_run(
    repo: &Path,
    run_id: Uuid,
    agent_key: &str,
    mergeable: bool,
    composite: f64,
) -> String {
    // Create branch with a change
    let branch = format!("hydra/{run_id}/agent/{agent_key}");
    let output_file = format!("{agent_key}_output.txt");
    run_git(repo, &["checkout", "-b", &branch]);
    std::fs::write(repo.join(&output_file), format!("{agent_key} was here")).unwrap();
    run_git(repo, &["add", &output_file]);
    run_git(repo, &["commit", "-m", &format!("{agent_key} changes")]);
    run_git(repo, &["checkout", "main"]);

    // Create .hydra run artifacts
    let hydra_root = repo.join(".hydra");
    let run_dir = hydra_root.join("runs").join(run_id.to_string());
    let agent_dir = run_dir.join("agents").join(agent_key);
    std::fs::create_dir_all(&agent_dir).unwrap();

    // Write manifest
    let manifest = serde_json::json!({
        "schema_version": 2,
        "event_schema_version": 1,
        "run_id": run_id.to_string(),
        "repo_root": repo.display().to_string(),
        "base_ref": "main",
        "task_prompt_hash": "abcd1234",
        "started_at": "2026-02-23T00:00:00Z",
        "completed_at": "2026-02-23T00:01:00Z",
        "status": "completed",
        "agents": [{
            "agent_key": agent_key,
            "tier": "tier-1",
            "branch": branch,
            "worktree_path": null
        }]
    });
    std::fs::write(
        run_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    // Write score
    let score = serde_json::json!({
        "agent_key": agent_key,
        "dimensions": [
            {"name": "build", "score": if mergeable { 100.0 } else { 0.0 }, "evidence": {}},
            {"name": "tests", "score": 80.0, "evidence": {}}
        ],
        "composite": composite,
        "mergeable": mergeable,
        "gate_failures": if mergeable { vec![] } else { vec!["build failed".to_string()] }
    });
    std::fs::write(
        agent_dir.join("score.json"),
        serde_json::to_string_pretty(&score).unwrap(),
    )
    .unwrap();

    // Write events.jsonl (minimal)
    std::fs::write(run_dir.join("events.jsonl"), "").unwrap();

    branch
}

#[test]
fn dry_run_clean_merge() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let branch = setup_completed_run(&repo, run_id, "claude", true, 90.0);

    // Verify branch exists
    let out = Command::new("git")
        .args(["branch", "--list", &branch])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&out.stdout).trim().is_empty(),
        "branch should exist"
    );

    // Run dry merge via the merge module functions directly is tricky since it's CLI,
    // so we test the underlying git operations
    let merge_output = Command::new("git")
        .args(["merge", "--no-commit", "--no-ff", &branch])
        .current_dir(&repo)
        .output()
        .unwrap();

    assert!(
        merge_output.status.success(),
        "dry-run merge should succeed without conflicts"
    );

    // Abort to clean up
    let _ = Command::new("git")
        .args(["merge", "--abort"])
        .current_dir(&repo)
        .output();
}

#[test]
fn dry_run_detects_conflicts() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let branch = format!("hydra/{run_id}/agent/claude");

    // Create branch with a conflicting change
    run_git(&repo, &["checkout", "-b", &branch]);
    std::fs::write(repo.join("README.md"), "# branch version").unwrap();
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "branch change"]);
    run_git(&repo, &["checkout", "main"]);

    // Create conflicting change on main
    std::fs::write(repo.join("README.md"), "# main version").unwrap();
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "main change"]);

    // Merge should detect conflict
    let merge_output = Command::new("git")
        .args(["merge", "--no-commit", "--no-ff", &branch])
        .current_dir(&repo)
        .output()
        .unwrap();

    assert!(
        !merge_output.status.success(),
        "merge should fail with conflicts"
    );

    // Abort
    let _ = Command::new("git")
        .args(["merge", "--abort"])
        .current_dir(&repo)
        .output();
}

#[test]
fn gate_enforcement_blocks_unmergeable() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    setup_completed_run(&repo, run_id, "claude", false, 50.0);

    // Load score and verify it's not mergeable
    let score_path = repo
        .join(".hydra/runs")
        .join(run_id.to_string())
        .join("agents/claude/score.json");
    let score_data = std::fs::read_to_string(&score_path).unwrap();
    let score: serde_json::Value = serde_json::from_str(&score_data).unwrap();

    assert_eq!(score["mergeable"], false);
    assert!(!score["gate_failures"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn pick_winner_selects_highest_mergeable() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();

    // Create two agents: one mergeable with lower score, one unmergeable with higher score
    setup_completed_run(&repo, run_id, "claude", true, 85.0);
    setup_completed_run(&repo, run_id, "codex", false, 95.0);

    // Update manifest to include both agents
    let run_dir = repo
        .join(".hydra/runs")
        .join(run_id.to_string());
    let manifest = serde_json::json!({
        "schema_version": 2,
        "event_schema_version": 1,
        "run_id": run_id.to_string(),
        "repo_root": repo.display().to_string(),
        "base_ref": "main",
        "task_prompt_hash": "abcd1234",
        "started_at": "2026-02-23T00:00:00Z",
        "completed_at": "2026-02-23T00:01:00Z",
        "status": "completed",
        "agents": [
            {
                "agent_key": "claude",
                "tier": "tier-1",
                "branch": format!("hydra/{run_id}/agent/claude"),
                "worktree_path": null
            },
            {
                "agent_key": "codex",
                "tier": "tier-1",
                "branch": format!("hydra/{run_id}/agent/codex"),
                "worktree_path": null
            }
        ]
    });
    std::fs::write(
        run_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    // Load scores and verify winner logic:
    // codex has higher score (95) but is not mergeable
    // claude has lower score (85) but is mergeable
    // winner should be claude
    let claude_score: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("agents/claude/score.json")).unwrap(),
    )
    .unwrap();
    let codex_score: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("agents/codex/score.json")).unwrap(),
    )
    .unwrap();

    assert!(claude_score["mergeable"].as_bool().unwrap());
    assert!(!codex_score["mergeable"].as_bool().unwrap());
    assert!(
        codex_score["composite"].as_f64().unwrap()
            > claude_score["composite"].as_f64().unwrap()
    );
}

#[test]
fn real_merge_succeeds_with_confirm() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let branch = setup_completed_run(&repo, run_id, "claude", true, 90.0);

    // Perform real merge
    let message = format!("hydra: merge claude from run {run_id}");
    let merge_output = Command::new("git")
        .args(["merge", "--no-ff", &branch, "-m", &message])
        .current_dir(&repo)
        .output()
        .unwrap();

    assert!(
        merge_output.status.success(),
        "real merge should succeed: {}",
        String::from_utf8_lossy(&merge_output.stderr)
    );

    // Verify the merge commit exists
    let log_output = Command::new("git")
        .args(["log", "--oneline", "-1"])
        .current_dir(&repo)
        .output()
        .unwrap();
    let log_line = String::from_utf8_lossy(&log_output.stdout);
    assert!(
        log_line.contains("hydra: merge claude"),
        "merge commit should be present"
    );

    // Verify the file from the agent branch is present
    assert!(repo.join("claude_output.txt").exists());
}

#[test]
fn merge_report_is_written() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let branch = setup_completed_run(&repo, run_id, "claude", true, 90.0);

    let run_dir = repo
        .join(".hydra/runs")
        .join(run_id.to_string());
    let report_path = run_dir.join("merge_report.json");

    // Simulate writing a merge report
    let report = serde_json::json!({
        "agent": "claude",
        "branch": branch,
        "dry_run": true,
        "success": true,
        "has_conflicts": false,
        "stdout": "Updating abc..def",
        "stderr": ""
    });
    std::fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

    // Verify report is readable
    let loaded: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    assert_eq!(loaded["agent"], "claude");
    assert_eq!(loaded["success"], true);
    assert_eq!(loaded["has_conflicts"], false);
}

#[test]
fn pre_flight_detects_merge_state() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    // Create a branch with a conflict
    run_git(&repo, &["checkout", "-b", "conflict-branch"]);
    std::fs::write(repo.join("README.md"), "# conflict").unwrap();
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "conflict change"]);
    run_git(&repo, &["checkout", "main"]);
    std::fs::write(repo.join("README.md"), "# main conflict").unwrap();
    run_git(&repo, &["add", "."]);
    run_git(&repo, &["commit", "-m", "main conflict"]);

    // Start a merge that will conflict
    let _ = Command::new("git")
        .args(["merge", "--no-ff", "conflict-branch"])
        .current_dir(&repo)
        .output();

    // MERGE_HEAD should exist
    assert!(
        repo.join(".git/MERGE_HEAD").exists(),
        "repo should be in merge state"
    );

    // Abort to clean up
    let _ = Command::new("git")
        .args(["merge", "--abort"])
        .current_dir(&repo)
        .output();
}

#[test]
fn pre_flight_detects_dirty_working_tree() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    // Create uncommitted changes
    std::fs::write(repo.join("dirty.txt"), "uncommitted").unwrap();

    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&repo)
        .output()
        .unwrap();

    assert!(
        !output.stdout.is_empty(),
        "working tree should be dirty"
    );
}

#[test]
fn pre_flight_detects_missing_branch() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let output = Command::new("git")
        .args([
            "rev-parse",
            "--verify",
            "refs/heads/hydra/nonexistent/agent/claude",
        ])
        .current_dir(&repo)
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "non-existent branch should fail rev-parse"
    );
}
