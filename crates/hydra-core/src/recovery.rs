//! Crash recovery and stale-state cleanup.
//!
//! When Hydra exits unexpectedly, worktrees and partial run data may be
//! left behind. This module provides tools to detect and clean up that
//! state.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::artifact::manifest::{RunManifest, RunStatus};
use crate::worktree::WorktreeService;
use crate::{HydraError, Result};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Metadata about a run that may need recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryMetadata {
    pub run_id: Uuid,
    pub state: RecoveryState,
    pub worktrees: Vec<WorktreeRecord>,
    pub last_checkpoint: DateTime<Utc>,
}

/// State of a run from the recovery perspective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryState {
    InProgress,
    Interrupted,
    CleanupNeeded,
    Recovered,
}

/// Record of a worktree associated with a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeRecord {
    pub path: PathBuf,
    pub branch: String,
    pub agent_key: String,
    pub cleaned_up: bool,
}

/// Summary of a cleanup operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupReport {
    pub runs_cleaned: u32,
    pub worktrees_removed: u32,
    pub branches_deleted: u32,
    pub errors: Vec<String>,
}

const RECOVERY_FILE: &str = "recovery.json";

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Service for detecting and cleaning up stale Hydra state.
pub struct RecoveryService {
    hydra_dir: PathBuf,
    repo_root: PathBuf,
}

impl RecoveryService {
    pub fn new(repo_root: &Path) -> Self {
        Self {
            hydra_dir: repo_root.join(".hydra"),
            repo_root: repo_root.to_path_buf(),
        }
    }

    /// Scan for interrupted or stale runs.
    ///
    /// A run is considered stale if its manifest status is `Running` but no
    /// process appears to be active, or if a recovery checkpoint indicates
    /// an interrupted state.
    pub async fn scan_stale_runs(&self) -> Result<Vec<RecoveryMetadata>> {
        let runs_dir = self.hydra_dir.join("runs");
        if !runs_dir.exists() {
            return Ok(vec![]);
        }

        let mut stale = Vec::new();

        let mut entries = tokio::fs::read_dir(&runs_dir)
            .await
            .map_err(|e| HydraError::Artifact(format!("failed to read runs directory: {e}")))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| HydraError::Artifact(format!("failed to read directory entry: {e}")))?
        {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            // Check for recovery checkpoint first.
            let recovery_path = path.join(RECOVERY_FILE);
            if recovery_path.exists() {
                match self.read_checkpoint(&recovery_path).await {
                    Ok(meta) if meta.state != RecoveryState::Recovered => {
                        stale.push(meta);
                        continue;
                    }
                    _ => {}
                }
            }

            // Check manifest for stale Running status.
            let manifest_path = path.join("manifest.json");
            if manifest_path.exists() {
                match self.read_manifest(&manifest_path).await {
                    Ok(manifest) if manifest.status == RunStatus::Running => {
                        let run_id = manifest.run_id;
                        let worktrees = manifest
                            .agents
                            .iter()
                            .map(|a| WorktreeRecord {
                                path: a.worktree_path.clone(),
                                branch: a.branch.clone(),
                                agent_key: a.agent_key.clone(),
                                cleaned_up: false,
                            })
                            .collect();

                        stale.push(RecoveryMetadata {
                            run_id,
                            state: RecoveryState::Interrupted,
                            worktrees,
                            last_checkpoint: manifest.started_at,
                        });
                    }
                    _ => {}
                }
            }
        }

        info!(count = stale.len(), "scanned for stale runs");
        Ok(stale)
    }

    /// Clean up a specific stale run.
    pub async fn cleanup_run(&self, run_id: Uuid) -> Result<()> {
        info!(%run_id, "cleaning up stale run");

        // Remove worktrees.
        let wt_svc = WorktreeService::new(self.repo_root.clone());
        if let Err(e) = wt_svc.cleanup_run(run_id).await {
            warn!(%run_id, error = %e, "worktree cleanup encountered errors");
        }

        // Update the manifest to reflect the failed state.
        let run_dir = self.hydra_dir.join("runs").join(run_id.to_string());
        let manifest_path = run_dir.join("manifest.json");
        if manifest_path.exists() {
            if let Ok(mut manifest) = self.read_manifest(&manifest_path).await {
                manifest.status = RunStatus::Failed;
                manifest.completed_at = Some(Utc::now());
                let json = serde_json::to_string_pretty(&manifest).map_err(|e| {
                    HydraError::Artifact(format!("failed to serialize manifest: {e}"))
                })?;
                tokio::fs::write(&manifest_path, json)
                    .await
                    .map_err(|e| HydraError::Artifact(format!("failed to update manifest: {e}")))?;
            }
        }

        // Write recovered checkpoint.
        let recovery_path = run_dir.join(RECOVERY_FILE);
        let checkpoint = RecoveryMetadata {
            run_id,
            state: RecoveryState::Recovered,
            worktrees: vec![],
            last_checkpoint: Utc::now(),
        };
        let json = serde_json::to_string_pretty(&checkpoint).map_err(|e| {
            HydraError::Artifact(format!("failed to serialize recovery checkpoint: {e}"))
        })?;
        tokio::fs::write(&recovery_path, json).await.map_err(|e| {
            HydraError::Artifact(format!("failed to write recovery checkpoint: {e}"))
        })?;

        Ok(())
    }

    /// Clean up all stale state and return a summary report.
    pub async fn cleanup_all(&self) -> Result<CleanupReport> {
        let stale = self.scan_stale_runs().await?;
        let mut report = CleanupReport {
            runs_cleaned: 0,
            worktrees_removed: 0,
            branches_deleted: 0,
            errors: Vec::new(),
        };

        for meta in &stale {
            report.worktrees_removed += meta.worktrees.len() as u32;
            report.branches_deleted += meta.worktrees.len() as u32;

            match self.cleanup_run(meta.run_id).await {
                Ok(()) => {
                    report.runs_cleaned += 1;
                }
                Err(e) => {
                    report.errors.push(format!("run {}: {e}", meta.run_id));
                }
            }
        }

        info!(
            runs_cleaned = report.runs_cleaned,
            worktrees_removed = report.worktrees_removed,
            errors = report.errors.len(),
            "cleanup complete"
        );

        Ok(report)
    }

    /// Write a recovery checkpoint during an active run.
    pub fn write_checkpoint(&self, metadata: &RecoveryMetadata) -> Result<()> {
        let run_dir = self
            .hydra_dir
            .join("runs")
            .join(metadata.run_id.to_string());

        if !run_dir.exists() {
            std::fs::create_dir_all(&run_dir).map_err(|e| {
                HydraError::Artifact(format!("failed to create run directory: {e}"))
            })?;
        }

        let recovery_path = run_dir.join(RECOVERY_FILE);
        let json = serde_json::to_string_pretty(metadata).map_err(|e| {
            HydraError::Artifact(format!("failed to serialize recovery checkpoint: {e}"))
        })?;
        std::fs::write(&recovery_path, json).map_err(|e| {
            HydraError::Artifact(format!("failed to write recovery checkpoint: {e}"))
        })?;

        debug!(run_id = %metadata.run_id, "wrote recovery checkpoint");
        Ok(())
    }

    /// Read a recovery checkpoint from disk.
    async fn read_checkpoint(&self, path: &Path) -> Result<RecoveryMetadata> {
        let data = tokio::fs::read_to_string(path).await.map_err(|e| {
            HydraError::Artifact(format!("failed to read recovery checkpoint: {e}"))
        })?;
        serde_json::from_str(&data)
            .map_err(|e| HydraError::Artifact(format!("failed to parse recovery checkpoint: {e}")))
    }

    /// Read a run manifest from disk.
    async fn read_manifest(&self, path: &Path) -> Result<RunManifest> {
        let data = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| HydraError::Artifact(format!("failed to read manifest: {e}")))?;
        serde_json::from_str(&data)
            .map_err(|e| HydraError::Artifact(format!("failed to parse manifest: {e}")))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_metadata_serde_round_trip() {
        let meta = RecoveryMetadata {
            run_id: Uuid::new_v4(),
            state: RecoveryState::Interrupted,
            worktrees: vec![WorktreeRecord {
                path: PathBuf::from("/tmp/wt"),
                branch: "hydra/abc/agent/claude".into(),
                agent_key: "claude".into(),
                cleaned_up: false,
            }],
            last_checkpoint: Utc::now(),
        };

        let json = serde_json::to_string(&meta).unwrap();
        let deser: RecoveryMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deser.run_id, meta.run_id);
        assert_eq!(deser.state, RecoveryState::Interrupted);
        assert_eq!(deser.worktrees.len(), 1);
        assert_eq!(deser.worktrees[0].agent_key, "claude");
    }

    #[test]
    fn cleanup_report_serde_round_trip() {
        let report = CleanupReport {
            runs_cleaned: 3,
            worktrees_removed: 6,
            branches_deleted: 6,
            errors: vec!["some error".into()],
        };

        let json = serde_json::to_string(&report).unwrap();
        let deser: CleanupReport = serde_json::from_str(&json).unwrap();

        assert_eq!(deser.runs_cleaned, 3);
        assert_eq!(deser.worktrees_removed, 6);
        assert_eq!(deser.errors.len(), 1);
    }

    #[test]
    fn recovery_state_variants_serialize_correctly() {
        let states = vec![
            (RecoveryState::InProgress, "\"in_progress\""),
            (RecoveryState::Interrupted, "\"interrupted\""),
            (RecoveryState::CleanupNeeded, "\"cleanup_needed\""),
            (RecoveryState::Recovered, "\"recovered\""),
        ];
        for (state, expected) in states {
            let json = serde_json::to_string(&state).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn write_and_read_checkpoint() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = RecoveryService::new(tmp.path());
        let run_id = Uuid::new_v4();

        let meta = RecoveryMetadata {
            run_id,
            state: RecoveryState::InProgress,
            worktrees: vec![],
            last_checkpoint: Utc::now(),
        };

        svc.write_checkpoint(&meta).unwrap();

        // Verify the file was written.
        let recovery_path = tmp
            .path()
            .join(".hydra")
            .join("runs")
            .join(run_id.to_string())
            .join(RECOVERY_FILE);
        assert!(recovery_path.exists());

        let contents = std::fs::read_to_string(&recovery_path).unwrap();
        let deser: RecoveryMetadata = serde_json::from_str(&contents).unwrap();
        assert_eq!(deser.run_id, run_id);
        assert_eq!(deser.state, RecoveryState::InProgress);
    }

    #[tokio::test]
    async fn scan_empty_repo_returns_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = RecoveryService::new(tmp.path());
        let stale = svc.scan_stale_runs().await.unwrap();
        assert!(stale.is_empty());
    }

    #[tokio::test]
    async fn scan_finds_stale_running_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let run_id = Uuid::new_v4();
        let run_dir = tmp
            .path()
            .join(".hydra")
            .join("runs")
            .join(run_id.to_string());
        tokio::fs::create_dir_all(&run_dir).await.unwrap();

        let manifest = RunManifest {
            schema_version: "1.0.0".into(),
            run_id,
            repo_root: tmp.path().to_path_buf(),
            base_ref: "HEAD".into(),
            task_prompt_hash: "abc".into(),
            started_at: Utc::now(),
            completed_at: None,
            status: RunStatus::Running,
            agents: vec![],
        };
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        tokio::fs::write(run_dir.join("manifest.json"), json)
            .await
            .unwrap();

        let svc = RecoveryService::new(tmp.path());
        let stale = svc.scan_stale_runs().await.unwrap();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].run_id, run_id);
        assert_eq!(stale[0].state, RecoveryState::Interrupted);
    }

    #[tokio::test]
    async fn cleanup_all_on_empty_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = RecoveryService::new(tmp.path());
        let report = svc.cleanup_all().await.unwrap();
        assert_eq!(report.runs_cleaned, 0);
        assert_eq!(report.worktrees_removed, 0);
        assert!(report.errors.is_empty());
    }
}
