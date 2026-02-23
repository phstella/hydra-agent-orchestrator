use std::path::Path;
use std::time::Duration;

use hydra_core::adapter::BuiltCommand;
use hydra_core::artifact::{EventKind, EventReader, EventWriter, RunEvent, RunLayout};
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

fn test_cwd() -> std::path::PathBuf {
    std::env::temp_dir()
}

/// Verify that force_cleanup after a cancelled supervised process
/// leaves no orphan worktrees or branches.
#[tokio::test]
async fn cancel_during_execution_cleans_up_worktree() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let wt_base = tmp.path().join("worktrees");
    let svc = WorktreeService::new(repo.clone(), wt_base);
    let run_id = Uuid::new_v4();

    let wt_info = svc.create(run_id, "claude", "HEAD").await.unwrap();
    assert!(wt_info.path.exists());

    let cmd = BuiltCommand {
        program: "sleep".to_string(),
        args: vec!["60".to_string()],
        env: vec![],
        cwd: wt_info.path.clone(),
    };

    let (tx, mut rx) = mpsc::channel(64);
    let policy = SupervisorPolicy {
        hard_timeout: Duration::from_secs(120),
        idle_timeout: Duration::from_secs(120),
        ..Default::default()
    };

    let handle = supervise(cmd, policy, tx, |_| None).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;
    handle.cancel().await;

    while let Some(evt) = rx.recv().await {
        if matches!(
            evt,
            SupervisorEvent::Failed { .. } | SupervisorEvent::Completed { .. }
        ) {
            break;
        }
    }

    svc.force_cleanup(&wt_info).await.unwrap();

    assert!(
        !wt_info.path.exists(),
        "worktree directory should be removed after cleanup"
    );

    let branch_check = std::process::Command::new("git")
        .args(["branch", "--list", &wt_info.branch])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&branch_check.stdout)
            .trim()
            .is_empty(),
        "branch should be deleted after cleanup"
    );
}

/// Verify that a crashing agent process (non-zero exit) still allows
/// proper worktree cleanup with no orphans.
#[tokio::test]
async fn agent_crash_cleanup_no_orphan_worktrees() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let wt_base = tmp.path().join("worktrees");
    let svc = WorktreeService::new(repo.clone(), wt_base);
    let run_id = Uuid::new_v4();

    let wt_info = svc.create(run_id, "codex", "HEAD").await.unwrap();
    assert!(wt_info.path.exists());

    let cmd = BuiltCommand {
        program: "sh".to_string(),
        args: vec!["-c".to_string(), "echo 'starting'; exit 137".to_string()],
        env: vec![],
        cwd: wt_info.path.clone(),
    };

    let (tx, mut rx) = mpsc::channel(64);
    let policy = SupervisorPolicy::default();

    let _handle = supervise(cmd, policy, tx, |_| None).await.unwrap();

    let mut saw_failure = false;
    while let Some(evt) = rx.recv().await {
        if let SupervisorEvent::Failed { error, .. } = &evt {
            assert!(error.contains("137"));
            saw_failure = true;
            break;
        }
    }
    assert!(saw_failure, "should report failure for crashed agent");

    svc.force_cleanup(&wt_info).await.unwrap();

    assert!(!wt_info.path.exists(), "worktree should be removed");

    let entries = svc.list().await.unwrap();
    let orphans: Vec<_> = entries
        .iter()
        .filter(|e| e.branch.contains(&run_id.to_string()))
        .collect();
    assert!(orphans.is_empty(), "no orphan worktrees should remain");
}

/// Verify that a timed-out agent still produces usable partial artifacts.
#[tokio::test]
async fn timeout_produces_partial_artifacts() {
    let tmp = TempDir::new().unwrap();
    let hydra_root = tmp.path().join(".hydra");
    let run_id = Uuid::new_v4();
    let layout = RunLayout::new(&hydra_root, run_id);
    layout.create_dirs(&["claude"]).unwrap();

    let mut event_writer = EventWriter::create(&layout.events_path()).unwrap();

    event_writer
        .write_event(&RunEvent::new(
            EventKind::RunStarted,
            None,
            serde_json::json!({"run_id": run_id.to_string()}),
        ))
        .unwrap();

    let cmd = BuiltCommand {
        program: "sh".to_string(),
        args: vec![
            "-c".to_string(),
            "echo line1; echo line2; sleep 60".to_string(),
        ],
        env: vec![],
        cwd: test_cwd(),
    };

    let (tx, mut rx) = mpsc::channel(64);
    let policy = SupervisorPolicy {
        hard_timeout: Duration::from_millis(300),
        idle_timeout: Duration::from_secs(120),
        ..Default::default()
    };

    let _handle = supervise(cmd, policy, tx, |_| None).await.unwrap();

    let mut saw_timeout = false;
    let mut stdout_lines = Vec::new();
    while let Some(evt) = rx.recv().await {
        match &evt {
            SupervisorEvent::Stdout(line) => {
                event_writer
                    .write_event(&RunEvent::new(
                        EventKind::AgentStdout,
                        Some("claude".to_string()),
                        serde_json::json!({"line": line}),
                    ))
                    .unwrap();
                stdout_lines.push(line.clone());
            }
            SupervisorEvent::TimedOut { .. } => {
                event_writer
                    .write_event(&RunEvent::new(
                        EventKind::AgentFailed,
                        Some("claude".to_string()),
                        serde_json::json!({"error": "timed out"}),
                    ))
                    .unwrap();
                saw_timeout = true;
                break;
            }
            _ => {}
        }
    }
    assert!(saw_timeout, "should report timeout");
    assert!(
        !stdout_lines.is_empty(),
        "should have captured partial stdout before timeout"
    );

    event_writer
        .write_event(&RunEvent::new(
            EventKind::RunFailed,
            None,
            serde_json::json!({"reason": "timeout"}),
        ))
        .unwrap();
    drop(event_writer);

    let events = EventReader::read_all(&layout.events_path()).unwrap();
    assert!(
        events.len() >= 3,
        "should have at least RunStarted + stdout + RunFailed events"
    );
    assert_eq!(events.first().unwrap().kind, EventKind::RunStarted);
    assert_eq!(events.last().unwrap().kind, EventKind::RunFailed);

    let stdout_events: Vec<_> = events
        .iter()
        .filter(|e| e.kind == EventKind::AgentStdout)
        .collect();
    assert!(
        !stdout_events.is_empty(),
        "partial artifacts should contain stdout events captured before timeout"
    );
}

/// Verify that force_cleanup is fully idempotent: calling it multiple times
/// after the worktree and branch are already gone does not error.
#[tokio::test]
async fn force_cleanup_idempotent_after_interrupt() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let wt_base = tmp.path().join("worktrees");
    let svc = WorktreeService::new(repo.clone(), wt_base);
    let run_id = Uuid::new_v4();

    let wt_info = svc.create(run_id, "claude", "HEAD").await.unwrap();
    assert!(wt_info.path.exists());

    svc.force_cleanup(&wt_info).await.unwrap();
    assert!(!wt_info.path.exists());

    // Second cleanup should succeed without errors (idempotent)
    svc.force_cleanup(&wt_info).await.unwrap();

    // Third cleanup should also succeed
    svc.force_cleanup(&wt_info).await.unwrap();
}

/// Verify that multiple concurrent worktrees can all be cleaned up
/// independently without interference.
#[tokio::test]
async fn concurrent_worktree_cleanup_no_interference() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let wt_base = tmp.path().join("worktrees");
    let svc = WorktreeService::new(repo.clone(), wt_base);
    let run_id = Uuid::new_v4();

    let wt1 = svc.create(run_id, "claude", "HEAD").await.unwrap();
    let wt2 = svc.create(run_id, "codex", "HEAD").await.unwrap();
    assert!(wt1.path.exists());
    assert!(wt2.path.exists());

    svc.force_cleanup(&wt1).await.unwrap();
    assert!(!wt1.path.exists());
    assert!(wt2.path.exists(), "cleaning up wt1 should not affect wt2");

    svc.force_cleanup(&wt2).await.unwrap();
    assert!(!wt2.path.exists());

    let entries = svc.list().await.unwrap();
    let orphans: Vec<_> = entries
        .iter()
        .filter(|e| e.branch.contains(&run_id.to_string()))
        .collect();
    assert!(orphans.is_empty(), "no orphan worktrees from run");
}

/// Simulate Ctrl+C during a race: supervisor is cancelled, then all
/// worktree cleanup runs. Verify end-to-end no orphans remain.
#[cfg(unix)]
#[tokio::test]
async fn simulated_ctrlc_full_cleanup() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    init_test_repo(&repo);

    let wt_base = tmp.path().join("worktrees");
    let svc = WorktreeService::new(repo.clone(), wt_base);
    let run_id = Uuid::new_v4();
    let wt_info = svc.create(run_id, "claude", "HEAD").await.unwrap();

    // Spawn a long-running process with a background child
    let cmd = BuiltCommand {
        program: "sh".to_string(),
        args: vec![
            "-c".to_string(),
            "sleep 60 & echo child:$!; wait".to_string(),
        ],
        env: vec![],
        cwd: wt_info.path.clone(),
    };

    let (tx, mut rx) = mpsc::channel(64);
    let policy = SupervisorPolicy {
        hard_timeout: Duration::from_secs(120),
        idle_timeout: Duration::from_secs(120),
        ..Default::default()
    };

    let handle = supervise(cmd, policy, tx, |_| None).await.unwrap();

    let child_pid: i32 = loop {
        tokio::select! {
            Some(evt) = rx.recv() => {
                if let SupervisorEvent::Stdout(line) = &evt {
                    if let Some(pid_str) = line.strip_prefix("child:") {
                        if let Ok(pid) = pid_str.trim().parse::<i32>() {
                            break pid;
                        }
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                panic!("timed out waiting for child pid");
            }
        }
    };

    handle.cancel().await;

    while let Some(evt) = rx.recv().await {
        if matches!(
            evt,
            SupervisorEvent::Failed { .. } | SupervisorEvent::Completed { .. }
        ) {
            break;
        }
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    let exists = unsafe { libc::kill(child_pid, 0) == 0 };
    assert!(
        !exists,
        "background child should be killed via process group"
    );

    svc.force_cleanup(&wt_info).await.unwrap();
    assert!(!wt_info.path.exists(), "worktree should be removed");

    let entries = svc.list().await.unwrap();
    let orphans: Vec<_> = entries
        .iter()
        .filter(|e| e.branch.contains(&run_id.to_string()))
        .collect();
    assert!(
        orphans.is_empty(),
        "no orphan worktrees after simulated Ctrl+C"
    );
}
