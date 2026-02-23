//! End-to-end race integration tests (M2.10).
//!
//! Uses mock agents (simple shell scripts) to test the full multi-agent
//! race flow: worktree creation, parallel execution, event writing,
//! baseline capture, scoring, and artifact completeness.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use hydra_core::artifact::RunLayout;
use hydra_core::config::ScoringConfig;
use hydra_core::scoring::baseline::{
    capture_baseline, persist_baseline, BaselineResult, CommandResult, TestResult,
};
use hydra_core::scoring::build::score_build;
use hydra_core::scoring::ranking::{rank_agents, AgentScore};
use hydra_core::scoring::tests::score_tests;
use hydra_core::supervisor::{supervise, SupervisorEvent, SupervisorPolicy};
use hydra_core::worktree::WorktreeService;
use tempfile::TempDir;
use tokio::sync::mpsc;
use uuid::Uuid;

fn init_test_repo(dir: &Path) {
    use std::process::Command;
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@hydra.dev"])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Hydra Test"])
        .current_dir(dir)
        .output()
        .unwrap();
    std::fs::write(dir.join("README.md"), "# test").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir)
        .output()
        .unwrap();
}

#[tokio::test]
async fn two_agents_run_concurrently_both_complete() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let wt_base = tmp.path().join("worktrees");
    let wt_service = WorktreeService::new(repo.clone(), wt_base);
    let run_id = Uuid::new_v4();

    let wt1 = wt_service.create(run_id, "agent-a", "HEAD").await.unwrap();
    let wt2 = wt_service.create(run_id, "agent-b", "HEAD").await.unwrap();

    assert!(wt1.path.exists());
    assert!(wt2.path.exists());
    assert_ne!(wt1.path, wt2.path);

    let cmd1 = hydra_core::adapter::BuiltCommand {
        program: "echo".to_string(),
        args: vec!["agent-a done".to_string()],
        env: vec![],
        cwd: wt1.path.clone(),
    };
    let cmd2 = hydra_core::adapter::BuiltCommand {
        program: "echo".to_string(),
        args: vec!["agent-b done".to_string()],
        env: vec![],
        cwd: wt2.path.clone(),
    };

    let policy = SupervisorPolicy {
        hard_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(5),
        output_buffer_bytes: 1024,
    };

    let (tx1, mut rx1) = mpsc::channel(64);
    let (tx2, mut rx2) = mpsc::channel(64);

    let _h1 = supervise(cmd1, policy.clone(), tx1, |_| None)
        .await
        .unwrap();
    let _h2 = supervise(cmd2, policy, tx2, |_| None).await.unwrap();

    let mut a_completed = false;
    let mut b_completed = false;

    while let Some(evt) = rx1.recv().await {
        if matches!(evt, SupervisorEvent::Completed { .. }) {
            a_completed = true;
            break;
        }
    }
    while let Some(evt) = rx2.recv().await {
        if matches!(evt, SupervisorEvent::Completed { .. }) {
            b_completed = true;
            break;
        }
    }

    assert!(a_completed, "agent-a should complete");
    assert!(b_completed, "agent-b should complete");

    wt_service.force_cleanup(&wt1).await.unwrap();
    wt_service.force_cleanup(&wt2).await.unwrap();
}

#[tokio::test]
async fn one_agent_failure_does_not_kill_other() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let wt_base = tmp.path().join("worktrees");
    let wt_service = WorktreeService::new(repo.clone(), wt_base);
    let run_id = Uuid::new_v4();

    let wt_good = wt_service.create(run_id, "good", "HEAD").await.unwrap();
    let wt_bad = wt_service.create(run_id, "bad", "HEAD").await.unwrap();

    let cmd_good = hydra_core::adapter::BuiltCommand {
        program: "sh".to_string(),
        args: vec!["-c".to_string(), "sleep 0.1 && echo success".to_string()],
        env: vec![],
        cwd: wt_good.path.clone(),
    };
    let cmd_bad = hydra_core::adapter::BuiltCommand {
        program: "sh".to_string(),
        args: vec!["-c".to_string(), "exit 1".to_string()],
        env: vec![],
        cwd: wt_bad.path.clone(),
    };

    let policy = SupervisorPolicy {
        hard_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(5),
        output_buffer_bytes: 1024,
    };

    let (tx_good, mut rx_good) = mpsc::channel(64);
    let (tx_bad, mut rx_bad) = mpsc::channel(64);

    let _h_good = supervise(cmd_good, policy.clone(), tx_good, |_| None)
        .await
        .unwrap();
    let _h_bad = supervise(cmd_bad, policy, tx_bad, |_| None).await.unwrap();

    let mut bad_failed = false;
    while let Some(evt) = rx_bad.recv().await {
        if matches!(evt, SupervisorEvent::Failed { .. }) {
            bad_failed = true;
            break;
        }
    }

    let mut good_completed = false;
    while let Some(evt) = rx_good.recv().await {
        if matches!(evt, SupervisorEvent::Completed { .. }) {
            good_completed = true;
            break;
        }
    }

    assert!(bad_failed, "bad agent should fail");
    assert!(
        good_completed,
        "good agent should still complete despite bad agent failure"
    );

    wt_service.force_cleanup(&wt_good).await.unwrap();
    wt_service.force_cleanup(&wt_bad).await.unwrap();
}

#[tokio::test]
async fn scoring_produces_correct_ranking() {
    let build_pass = CommandResult {
        command: "cargo build".to_string(),
        success: true,
        exit_code: 0,
        stdout: String::new(),
        stderr: String::new(),
        duration_ms: 100,
    };
    let build_fail = CommandResult {
        command: "cargo build".to_string(),
        success: false,
        exit_code: 1,
        stdout: String::new(),
        stderr: "error".to_string(),
        duration_ms: 50,
    };

    let test_baseline = TestResult {
        command_result: build_pass.clone(),
        passed: 10,
        failed: 0,
        total: 10,
    };
    let test_good = TestResult {
        command_result: build_pass.clone(),
        passed: 10,
        failed: 0,
        total: 10,
    };
    let test_bad = TestResult {
        command_result: build_fail.clone(),
        passed: 5,
        failed: 5,
        total: 10,
    };

    let build_score_good = score_build(None, &build_pass);
    let build_score_bad = score_build(None, &build_fail);
    let test_score_good = score_tests(Some(&test_baseline), &test_good);
    let test_score_bad = score_tests(Some(&test_baseline), &test_bad);

    let agents = vec![
        (
            "good-agent".to_string(),
            vec![build_score_good, test_score_good],
        ),
        (
            "bad-agent".to_string(),
            vec![build_score_bad, test_score_bad],
        ),
    ];

    let weights = hydra_core::config::WeightsConfig::default();
    let gates = hydra_core::config::GatesConfig::default();
    let durations = HashMap::new();

    let ranked = rank_agents(agents, &weights, &gates, &durations);

    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].agent_key, "good-agent");
    assert_eq!(ranked[1].agent_key, "bad-agent");
    assert!(ranked[0].mergeable);
    assert!(!ranked[1].mergeable);
    assert!(ranked[0].composite > ranked[1].composite);
}

#[tokio::test]
async fn scoring_reproducible_from_artifacts() {
    let build_result = CommandResult {
        command: "cargo build".to_string(),
        success: true,
        exit_code: 0,
        stdout: "ok".to_string(),
        stderr: String::new(),
        duration_ms: 100,
    };
    let test_result = TestResult {
        command_result: build_result.clone(),
        passed: 20,
        failed: 1,
        total: 21,
    };

    let build_score = score_build(None, &build_result);
    let test_score = score_tests(None, &test_result);

    let agents = vec![(
        "agent-a".to_string(),
        vec![build_score.clone(), test_score.clone()],
    )];

    let weights = hydra_core::config::WeightsConfig::default();
    let gates = hydra_core::config::GatesConfig::default();
    let durations = HashMap::new();

    let ranked1 = rank_agents(agents.clone(), &weights, &gates, &durations);

    // Serialize and deserialize the scores to simulate re-scoring from artifacts
    let serialized = serde_json::to_string(&ranked1[0]).unwrap();
    let deserialized: AgentScore = serde_json::from_str(&serialized).unwrap();

    assert!((ranked1[0].composite - deserialized.composite).abs() < 0.001);
    assert_eq!(ranked1[0].mergeable, deserialized.mergeable);
    assert_eq!(ranked1[0].dimensions.len(), deserialized.dimensions.len());
}

#[tokio::test]
async fn baseline_capture_and_persist_roundtrip() {
    let config = ScoringConfig {
        commands: hydra_core::config::CommandsConfig {
            build: Some("echo build-ok".to_string()),
            test: Some("echo 'test result: ok. 10 passed; 0 failed; 0 ignored'".to_string()),
            lint: Some("echo clean".to_string()),
        },
        ..ScoringConfig::default()
    };

    let tmp = TempDir::new().unwrap();
    let result = capture_baseline(tmp.path(), &config).await.unwrap();

    assert!(result.build.as_ref().unwrap().success);
    assert_eq!(result.test.as_ref().unwrap().passed, 10);

    let baseline_path = tmp.path().join("baseline.json");
    persist_baseline(&result, &baseline_path).unwrap();

    let data = std::fs::read_to_string(&baseline_path).unwrap();
    let loaded: BaselineResult = serde_json::from_str(&data).unwrap();
    assert!(loaded.build.unwrap().success);
    assert_eq!(loaded.test.unwrap().passed, 10);
}

#[tokio::test]
async fn artifact_layout_has_all_expected_paths() {
    let tmp = TempDir::new().unwrap();
    let run_id = Uuid::new_v4();
    let layout = RunLayout::new(tmp.path(), run_id);

    layout.create_dirs(&["claude", "codex"]).unwrap();

    assert!(layout.base_dir().exists());
    assert!(layout.agent_dir("claude").exists());
    assert!(layout.agent_dir("codex").exists());

    // Paths are deterministic
    assert!(layout.manifest_path().ends_with("manifest.json"));
    assert!(layout.events_path().ends_with("events.jsonl"));
    assert!(layout
        .agent_score("claude")
        .ends_with("agents/claude/score.json"));
    assert!(layout.baseline_dir().ends_with("baseline"));

    layout.cleanup().unwrap();
}
