//! Orchestrator manages a complete run lifecycle.
//!
//! For M1.7 scope, only single-agent races are supported via [`Orchestrator::race_single`].

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::adapter::{AdapterRuntime, ClaudeAdapter, CodexAdapter, SpawnRequest};
use crate::artifact::events::{EventType, RunEvent};
use crate::artifact::manifest::{AgentManifest, AgentStatus, RunManifest, RunStatus};
use crate::artifact::run_dir::RunDir;
use crate::config::HydraConfig;
use crate::supervisor::{ProcessSupervisor, SupervisorConfig, SupervisorEvent};
use crate::worktree::WorktreeService;
use crate::{HydraError, Result};

/// Orchestrator manages a complete run lifecycle.
pub struct Orchestrator {
    config: HydraConfig,
    repo_root: PathBuf,
}

/// Result of a completed race.
#[derive(Debug)]
pub struct RaceResult {
    pub run_id: Uuid,
    pub agents: Vec<AgentRunResult>,
    pub artifact_dir: PathBuf,
}

/// Per-agent result from a race.
#[derive(Debug)]
pub struct AgentRunResult {
    pub agent_key: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub status: AgentStatus,
    pub exit_code: Option<i32>,
}

impl Orchestrator {
    pub fn new(config: HydraConfig, repo_root: PathBuf) -> Self {
        Self { config, repo_root }
    }

    /// Run a single-agent race (M1.7 scope).
    pub async fn race_single(&self, agent_key: &str, task_prompt: &str) -> Result<RaceResult> {
        let run_id = Uuid::new_v4();
        let started_at = Utc::now();

        info!(%run_id, agent_key, "starting single-agent race");

        // 1. Create RunDir and write initial manifest.
        let run_dir = RunDir::create(&self.repo_root, run_id)?;

        let prompt_hash = hash_prompt(task_prompt);
        let manifest = RunManifest {
            schema_version: "1.0.0".into(),
            run_id,
            repo_root: self.repo_root.clone(),
            base_ref: "HEAD".into(),
            task_prompt_hash: prompt_hash,
            started_at,
            completed_at: None,
            status: RunStatus::Running,
            agents: vec![],
        };
        run_dir.write_manifest(&manifest)?;

        // 2. Log RunStarted event.
        run_dir.append_event(&RunEvent {
            timestamp: started_at,
            run_id,
            event_type: EventType::RunStarted,
            agent_key: None,
            data: serde_json::json!({ "task_prompt": task_prompt }),
        })?;

        // 3. Create worktree.
        let wt_svc = WorktreeService::new(self.repo_root.clone());
        let worktree = match wt_svc.create(run_id, agent_key, "HEAD").await {
            Ok(wt) => wt,
            Err(e) => {
                self.finalize_run_failed(&run_dir, &manifest, &e.to_string())?;
                return Err(e);
            }
        };

        // 4. Build agent command via adapter.
        let adapter = get_adapter(agent_key)?;
        let spawn_req = SpawnRequest {
            task_prompt: task_prompt.to_string(),
            worktree_path: worktree.path.clone(),
            timeout_seconds: self.config.general.default_timeout_seconds,
            force_edit: false,
            output_json_stream: true,
        };
        let command = adapter.build_command(&spawn_req);

        // 5. Configure and run supervisor.
        let sup_config = SupervisorConfig {
            run_id,
            agent_key: agent_key.to_string(),
            idle_timeout: Duration::from_secs(self.config.general.idle_timeout_seconds),
            hard_timeout: Duration::from_secs(self.config.general.hard_timeout_seconds),
            max_output_bytes: 10 * 1024 * 1024, // 10 MiB
        };

        let supervisor = ProcessSupervisor::new(sup_config);

        // Log AgentStarted.
        run_dir.append_event(&RunEvent {
            timestamp: Utc::now(),
            run_id,
            event_type: EventType::AgentStarted,
            agent_key: Some(agent_key.to_string()),
            data: serde_json::json!({
                "program": command.program,
                "worktree": worktree.path.display().to_string(),
                "branch": worktree.branch,
            }),
        })?;

        // 6. Spawn and stream events.
        let (mut rx, _handle) = match supervisor.spawn(command).await {
            Ok(pair) => pair,
            Err(e) => {
                warn!(%run_id, error = %e, "failed to spawn agent process");
                // Attempt worktree cleanup.
                let _ = wt_svc.remove(&worktree).await;
                self.finalize_run_failed(&run_dir, &manifest, &e.to_string())?;
                return Err(e);
            }
        };

        let mut exit_code: Option<i32> = None;
        let mut agent_status = AgentStatus::Running;

        while let Some(event) = rx.recv().await {
            // Stream each supervisor event to the run dir.
            let (event_type, agent_key_opt, data) = supervisor_event_to_run_event(&event);
            let _ = run_dir.append_event(&RunEvent {
                timestamp: Utc::now(),
                run_id,
                event_type,
                agent_key: agent_key_opt,
                data,
            });

            match &event {
                SupervisorEvent::Completed {
                    exit_code: code, ..
                } => {
                    exit_code = Some(*code);
                    agent_status = if *code == 0 {
                        AgentStatus::Completed
                    } else {
                        AgentStatus::Failed
                    };
                }
                SupervisorEvent::Failed { .. } => {
                    agent_status = AgentStatus::Failed;
                }
                SupervisorEvent::TimedOut { .. } => {
                    agent_status = AgentStatus::TimedOut;
                }
                SupervisorEvent::Cancelled { .. } => {
                    agent_status = AgentStatus::Cancelled;
                }
                _ => {}
            }
        }

        // 7. Finalize manifest.
        let completed_at = Utc::now();
        let run_status = match agent_status {
            AgentStatus::Completed => RunStatus::Completed,
            AgentStatus::TimedOut => RunStatus::TimedOut,
            AgentStatus::Cancelled => RunStatus::Cancelled,
            _ => RunStatus::Failed,
        };

        let final_manifest = RunManifest {
            completed_at: Some(completed_at),
            status: run_status,
            agents: vec![AgentManifest {
                agent_key: agent_key.to_string(),
                adapter_version: None,
                worktree_path: worktree.path.clone(),
                branch: worktree.branch.clone(),
                started_at,
                completed_at: Some(completed_at),
                status: agent_status.clone(),
                token_usage: None,
                cost_estimate_usd: None,
            }],
            ..manifest
        };
        run_dir.write_manifest(&final_manifest)?;

        // 8. Log terminal event.
        let terminal_event_type = match &agent_status {
            AgentStatus::Completed => EventType::RunCompleted,
            _ => EventType::RunFailed,
        };
        run_dir.append_event(&RunEvent {
            timestamp: completed_at,
            run_id,
            event_type: terminal_event_type,
            agent_key: None,
            data: serde_json::json!({
                "status": format!("{:?}", agent_status),
                "exit_code": exit_code,
            }),
        })?;

        info!(
            %run_id,
            %agent_key,
            ?agent_status,
            ?exit_code,
            "single-agent race completed"
        );

        // 9. On failure, attempt worktree cleanup.
        if agent_status != AgentStatus::Completed {
            info!(%run_id, "cleaning up worktree after non-success");
            if let Err(e) = wt_svc.remove(&worktree).await {
                warn!(%run_id, error = %e, "worktree cleanup failed");
            }
        }

        Ok(RaceResult {
            run_id,
            agents: vec![AgentRunResult {
                agent_key: agent_key.to_string(),
                branch: worktree.branch.clone(),
                worktree_path: worktree.path.clone(),
                status: agent_status,
                exit_code,
            }],
            artifact_dir: run_dir.path().to_path_buf(),
        })
    }

    /// Write a failed manifest when the run cannot even start properly.
    fn finalize_run_failed(
        &self,
        run_dir: &RunDir,
        base_manifest: &RunManifest,
        error_msg: &str,
    ) -> Result<()> {
        let now = Utc::now();
        let failed_manifest = RunManifest {
            completed_at: Some(now),
            status: RunStatus::Failed,
            ..base_manifest.clone()
        };
        run_dir.write_manifest(&failed_manifest)?;
        run_dir.append_event(&RunEvent {
            timestamp: now,
            run_id: base_manifest.run_id,
            event_type: EventType::RunFailed,
            agent_key: None,
            data: serde_json::json!({ "error": error_msg }),
        })?;
        Ok(())
    }
}

/// Resolve an adapter runtime by key.
fn get_adapter(agent_key: &str) -> Result<Box<dyn AdapterRuntime>> {
    match agent_key {
        "claude" => Ok(Box::new(ClaudeAdapter)),
        "codex" => Ok(Box::new(CodexAdapter)),
        other => Err(HydraError::Adapter(format!(
            "unknown agent key '{other}'; supported: claude, codex"
        ))),
    }
}

/// Map a supervisor event to a (EventType, optional agent_key, data) tuple.
fn supervisor_event_to_run_event(
    event: &SupervisorEvent,
) -> (EventType, Option<String>, serde_json::Value) {
    match event {
        SupervisorEvent::Started { agent_key, pid, .. } => (
            EventType::AgentStarted,
            Some(agent_key.clone()),
            serde_json::json!({ "pid": pid }),
        ),
        SupervisorEvent::Stdout { agent_key, line } => (
            EventType::AgentStdout,
            Some(agent_key.clone()),
            serde_json::json!({ "line": line }),
        ),
        SupervisorEvent::Stderr { agent_key, line } => (
            EventType::AgentStderr,
            Some(agent_key.clone()),
            serde_json::json!({ "line": line }),
        ),
        SupervisorEvent::Completed {
            agent_key,
            exit_code,
            ..
        } => (
            EventType::AgentCompleted,
            Some(agent_key.clone()),
            serde_json::json!({ "exit_code": exit_code }),
        ),
        SupervisorEvent::Failed {
            agent_key, error, ..
        } => (
            EventType::AgentFailed,
            Some(agent_key.clone()),
            serde_json::json!({ "error": error }),
        ),
        SupervisorEvent::TimedOut {
            agent_key, reason, ..
        } => (
            EventType::AgentFailed,
            Some(agent_key.clone()),
            serde_json::json!({ "reason": format!("{:?}", reason) }),
        ),
        SupervisorEvent::Cancelled { agent_key, .. } => (
            EventType::AgentFailed,
            Some(agent_key.clone()),
            serde_json::json!({ "reason": "cancelled" }),
        ),
    }
}

/// Compute a hex-encoded hash of the given input.
///
/// Uses the standard library's `DefaultHasher` to avoid pulling in a crypto
/// dependency. The prompt hash is used for deduplication, not security.
fn hash_prompt(input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_adapter_claude() {
        let adapter = get_adapter("claude");
        assert!(adapter.is_ok());
    }

    #[test]
    fn get_adapter_codex() {
        let adapter = get_adapter("codex");
        assert!(adapter.is_ok());
    }

    #[test]
    fn get_adapter_unknown() {
        let result = get_adapter("gpt-pilot");
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("unknown agent key"));
    }

    #[test]
    fn hash_prompt_deterministic() {
        let h1 = hash_prompt("hello world");
        let h2 = hash_prompt("hello world");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16); // 64-bit hash = 16 hex chars
    }

    #[test]
    fn supervisor_event_mapping() {
        let ev = SupervisorEvent::Stdout {
            agent_key: "claude".into(),
            line: "hello".into(),
        };
        let (et, ak, data) = supervisor_event_to_run_event(&ev);
        assert_eq!(et, EventType::AgentStdout);
        assert_eq!(ak, Some("claude".into()));
        assert_eq!(data["line"], "hello");
    }
}
