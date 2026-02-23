use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;
use uuid::Uuid;

const HYDRA_BIN: &str = env!("CARGO_BIN_EXE_hydra");

fn run_git(dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute git")
}

fn run_git_ok(dir: &Path, args: &[&str]) -> Output {
    let out = run_git(dir, args);
    assert!(
        out.status.success(),
        "git {} failed\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn run_hydra(dir: &Path, args: &[&str]) -> Output {
    Command::new(HYDRA_BIN)
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute hydra")
}

fn output_text(out: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn merge_head_exists(dir: &Path) -> bool {
    run_git(dir, &["rev-parse", "-q", "--verify", "MERGE_HEAD"])
        .status
        .success()
}

fn init_test_repo(dir: &Path) {
    run_git_ok(dir, &["init", "-b", "main"]);
    run_git_ok(dir, &["config", "user.email", "test@hydra.dev"]);
    run_git_ok(dir, &["config", "user.name", "Hydra Test"]);
    std::fs::write(dir.join("README.md"), "# test\n").unwrap();
    std::fs::write(dir.join(".gitignore"), ".hydra/\n").unwrap();
    run_git_ok(dir, &["add", "README.md", ".gitignore"]);
    run_git_ok(dir, &["commit", "-m", "init"]);
}

fn create_agent_branch(
    repo: &Path,
    base_branch: &str,
    run_id: Uuid,
    agent_key: &str,
    file_path: &str,
    content: &str,
) -> String {
    let branch = format!("hydra/{run_id}/agent/{agent_key}");
    run_git_ok(repo, &["checkout", base_branch]);
    run_git_ok(repo, &["checkout", "-b", &branch]);
    let path = repo.join(file_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
    run_git_ok(repo, &["add", file_path]);
    run_git_ok(repo, &["commit", "-m", &format!("{agent_key} changes")]);
    run_git_ok(repo, &["checkout", base_branch]);
    branch
}

#[derive(Clone)]
struct AgentSpec {
    key: String,
    branch: String,
    mergeable: bool,
    composite: f64,
}

fn write_run_artifacts(repo: &Path, run_id: Uuid, base_ref: &str, agents: &[AgentSpec]) -> PathBuf {
    let run_dir = repo.join(".hydra/runs").join(run_id.to_string());

    for agent in agents {
        let agent_dir = run_dir.join("agents").join(&agent.key);
        std::fs::create_dir_all(&agent_dir).unwrap();

        let score = serde_json::json!({
            "agent_key": agent.key,
            "dimensions": [
                {
                    "name": "build",
                    "score": if agent.mergeable { 100.0 } else { 0.0 },
                    "evidence": {}
                },
                {
                    "name": "tests",
                    "score": 80.0,
                    "evidence": {"regression": 0, "baseline_passed": 10}
                }
            ],
            "composite": agent.composite,
            "mergeable": agent.mergeable,
            "gate_failures": if agent.mergeable {
                Vec::<String>::new()
            } else {
                vec!["build failed".to_string()]
            }
        });

        std::fs::write(
            agent_dir.join("score.json"),
            serde_json::to_string_pretty(&score).unwrap(),
        )
        .unwrap();
    }

    let manifest_agents: Vec<serde_json::Value> = agents
        .iter()
        .map(|agent| {
            serde_json::json!({
                "agent_key": agent.key,
                "tier": "tier-1",
                "branch": agent.branch,
                "worktree_path": null
            })
        })
        .collect();

    let manifest = serde_json::json!({
        "schema_version": 2,
        "event_schema_version": 1,
        "run_id": run_id.to_string(),
        "repo_root": repo.display().to_string(),
        "base_ref": base_ref,
        "task_prompt_hash": "abcd1234",
        "started_at": "2026-02-23T00:00:00Z",
        "completed_at": "2026-02-23T00:01:00Z",
        "status": "completed",
        "agents": manifest_agents
    });

    std::fs::create_dir_all(&run_dir).unwrap();
    std::fs::write(
        run_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(run_dir.join("events.jsonl"), "").unwrap();

    run_dir
}

#[test]
fn dry_run_clean_merge_writes_report_and_cleans_merge_state() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let branch = create_agent_branch(
        &repo,
        "main",
        run_id,
        "claude",
        "claude_output.txt",
        "claude was here\n",
    );
    let run_dir = write_run_artifacts(
        &repo,
        run_id,
        "main",
        &[AgentSpec {
            key: "claude".to_string(),
            branch,
            mergeable: true,
            composite: 90.0,
        }],
    );

    let run_id_arg = run_id.to_string();
    let out = run_hydra(
        &repo,
        &[
            "merge",
            "--run-id",
            &run_id_arg,
            "--agent",
            "claude",
            "--dry-run",
            "--json",
        ],
    );

    assert!(out.status.success(), "{}", output_text(&out));

    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["agent"], "claude");
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["success"], true);
    assert_eq!(report["has_conflicts"], false);

    let report_path = run_dir.join("merge_report.json");
    assert!(report_path.exists());
    assert!(!merge_head_exists(&repo), "merge state should be cleaned");
}

#[test]
fn dry_run_conflict_reports_failure_and_cleans_merge_state() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let branch = create_agent_branch(
        &repo,
        "main",
        run_id,
        "claude",
        "README.md",
        "# branch version\n",
    );

    std::fs::write(repo.join("README.md"), "# main version\n").unwrap();
    run_git_ok(&repo, &["add", "README.md"]);
    run_git_ok(&repo, &["commit", "-m", "main change"]);

    let run_dir = write_run_artifacts(
        &repo,
        run_id,
        "main",
        &[AgentSpec {
            key: "claude".to_string(),
            branch,
            mergeable: true,
            composite: 90.0,
        }],
    );

    let run_id_arg = run_id.to_string();
    let out = run_hydra(
        &repo,
        &[
            "merge",
            "--run-id",
            &run_id_arg,
            "--agent",
            "claude",
            "--dry-run",
        ],
    );

    assert!(!out.status.success(), "expected conflict failure");
    assert!(
        output_text(&out).contains("CONFLICTS DETECTED"),
        "{}",
        output_text(&out)
    );

    let report_path = run_dir.join("merge_report.json");
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(report_path).unwrap()).unwrap();
    assert_eq!(report["success"], false);
    assert_eq!(report["has_conflicts"], true);

    assert!(!merge_head_exists(&repo), "merge state should be cleaned");
}

#[test]
fn non_mergeable_agent_is_blocked_without_force() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let branch = create_agent_branch(
        &repo,
        "main",
        run_id,
        "claude",
        "claude_output.txt",
        "claude was here\n",
    );
    write_run_artifacts(
        &repo,
        run_id,
        "main",
        &[AgentSpec {
            key: "claude".to_string(),
            branch,
            mergeable: false,
            composite: 50.0,
        }],
    );

    let run_id_arg = run_id.to_string();
    let out = run_hydra(
        &repo,
        &[
            "merge",
            "--run-id",
            &run_id_arg,
            "--agent",
            "claude",
            "--confirm",
        ],
    );

    assert!(!out.status.success());
    assert!(
        output_text(&out).contains("is not mergeable"),
        "{}",
        output_text(&out)
    );
}

#[test]
fn missing_branch_fails_preflight() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let branch = create_agent_branch(
        &repo,
        "main",
        run_id,
        "claude",
        "claude_output.txt",
        "claude was here\n",
    );
    write_run_artifacts(
        &repo,
        run_id,
        "main",
        &[AgentSpec {
            key: "claude".to_string(),
            branch: branch.clone(),
            mergeable: true,
            composite: 90.0,
        }],
    );

    run_git_ok(&repo, &["branch", "-D", &branch]);

    let run_id_arg = run_id.to_string();
    let out = run_hydra(
        &repo,
        &[
            "merge",
            "--run-id",
            &run_id_arg,
            "--agent",
            "claude",
            "--dry-run",
        ],
    );

    assert!(!out.status.success());
    assert!(
        output_text(&out).contains("does not exist"),
        "{}",
        output_text(&out)
    );
}

#[test]
fn dirty_working_tree_fails_preflight() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let branch = create_agent_branch(
        &repo,
        "main",
        run_id,
        "claude",
        "claude_output.txt",
        "claude was here\n",
    );
    write_run_artifacts(
        &repo,
        run_id,
        "main",
        &[AgentSpec {
            key: "claude".to_string(),
            branch,
            mergeable: true,
            composite: 90.0,
        }],
    );

    std::fs::write(repo.join("dirty.txt"), "uncommitted\n").unwrap();

    let run_id_arg = run_id.to_string();
    let out = run_hydra(
        &repo,
        &[
            "merge",
            "--run-id",
            &run_id_arg,
            "--agent",
            "claude",
            "--dry-run",
        ],
    );

    assert!(!out.status.success());
    assert!(
        output_text(&out).contains("uncommitted changes"),
        "{}",
        output_text(&out)
    );
}

#[test]
fn merge_state_is_detected_in_linked_worktree_and_preserved() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    run_git_ok(&repo, &["worktree", "add", "-b", "wt-main", "wt", "main"]);
    let wt = repo.join("wt");

    let run_id = Uuid::new_v4();
    let branch = create_agent_branch(
        &wt,
        "wt-main",
        run_id,
        "claude",
        "claude_output.txt",
        "claude was here\n",
    );
    write_run_artifacts(
        &wt,
        run_id,
        "wt-main",
        &[AgentSpec {
            key: "claude".to_string(),
            branch,
            mergeable: true,
            composite: 90.0,
        }],
    );

    run_git_ok(&wt, &["checkout", "-b", "conflict-branch"]);
    std::fs::write(wt.join("README.md"), "# conflict\n").unwrap();
    run_git_ok(&wt, &["add", "README.md"]);
    run_git_ok(&wt, &["commit", "-m", "conflict change"]);

    run_git_ok(&wt, &["checkout", "wt-main"]);
    std::fs::write(wt.join("README.md"), "# main conflict\n").unwrap();
    run_git_ok(&wt, &["add", "README.md"]);
    run_git_ok(&wt, &["commit", "-m", "main conflict"]);

    let conflict_merge = run_git(&wt, &["merge", "--no-ff", "conflict-branch"]);
    assert!(!conflict_merge.status.success());
    assert!(merge_head_exists(&wt));

    let run_id_arg = run_id.to_string();
    let out = run_hydra(
        &wt,
        &[
            "merge",
            "--run-id",
            &run_id_arg,
            "--agent",
            "claude",
            "--dry-run",
        ],
    );

    assert!(!out.status.success());
    assert!(
        output_text(&out).contains("already in a merge state"),
        "{}",
        output_text(&out)
    );
    assert!(
        merge_head_exists(&wt),
        "existing merge state in worktree must be preserved"
    );

    run_git_ok(&wt, &["merge", "--abort"]);
}

#[test]
fn merge_requires_confirm_when_not_dry_run() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let branch = create_agent_branch(
        &repo,
        "main",
        run_id,
        "claude",
        "claude_output.txt",
        "claude was here\n",
    );
    write_run_artifacts(
        &repo,
        run_id,
        "main",
        &[AgentSpec {
            key: "claude".to_string(),
            branch,
            mergeable: true,
            composite: 90.0,
        }],
    );

    let run_id_arg = run_id.to_string();
    let out = run_hydra(
        &repo,
        &["merge", "--run-id", &run_id_arg, "--agent", "claude"],
    );

    assert!(!out.status.success());
    assert!(
        output_text(&out).contains("requires --confirm"),
        "{}",
        output_text(&out)
    );
}

#[test]
fn merge_with_confirm_succeeds() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let branch = create_agent_branch(
        &repo,
        "main",
        run_id,
        "claude",
        "claude_output.txt",
        "claude was here\n",
    );
    write_run_artifacts(
        &repo,
        run_id,
        "main",
        &[AgentSpec {
            key: "claude".to_string(),
            branch,
            mergeable: true,
            composite: 90.0,
        }],
    );

    let run_id_arg = run_id.to_string();
    let out = run_hydra(
        &repo,
        &[
            "merge",
            "--run-id",
            &run_id_arg,
            "--agent",
            "claude",
            "--confirm",
        ],
    );

    assert!(out.status.success(), "{}", output_text(&out));

    let log_out = run_git_ok(&repo, &["log", "--oneline", "-1"]);
    let log_line = String::from_utf8_lossy(&log_out.stdout);
    assert!(log_line.contains("hydra: merge claude"));
    assert!(repo.join("claude_output.txt").exists());
}

#[test]
fn pick_winner_prefers_highest_mergeable_agent() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let run_id = Uuid::new_v4();
    let claude_branch = create_agent_branch(
        &repo,
        "main",
        run_id,
        "claude",
        "claude_output.txt",
        "claude was here\n",
    );
    let codex_branch = create_agent_branch(
        &repo,
        "main",
        run_id,
        "codex",
        "codex_output.txt",
        "codex was here\n",
    );

    write_run_artifacts(
        &repo,
        run_id,
        "main",
        &[
            AgentSpec {
                key: "claude".to_string(),
                branch: claude_branch,
                mergeable: true,
                composite: 85.0,
            },
            AgentSpec {
                key: "codex".to_string(),
                branch: codex_branch,
                mergeable: false,
                composite: 95.0,
            },
        ],
    );

    let run_id_arg = run_id.to_string();
    let out = run_hydra(&repo, &["merge", "--run-id", &run_id_arg, "--confirm"]);

    assert!(out.status.success(), "{}", output_text(&out));

    let log_out = run_git_ok(&repo, &["log", "--oneline", "-1"]);
    let log_line = String::from_utf8_lossy(&log_out.stdout);
    assert!(
        log_line.contains("hydra: merge claude"),
        "unexpected merge winner: {}",
        log_line
    );
}
