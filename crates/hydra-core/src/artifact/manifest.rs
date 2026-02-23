use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use super::ArtifactError;

/// Top-level manifest written to `manifest.json` for every run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema_version: u32,
    pub event_schema_version: u32,
    pub run_id: Uuid,
    pub repo_root: String,
    pub base_ref: String,
    pub task_prompt_hash: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: RunStatus,
    pub agents: Vec<AgentEntry>,
}

impl RunManifest {
    pub const CURRENT_SCHEMA_VERSION: u32 = 2;
    pub const CURRENT_EVENT_SCHEMA_VERSION: u32 = 1;

    pub fn new(
        run_id: Uuid,
        repo_root: String,
        base_ref: String,
        task_prompt_hash: String,
        agents: Vec<AgentEntry>,
    ) -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            event_schema_version: Self::CURRENT_EVENT_SCHEMA_VERSION,
            run_id,
            repo_root,
            base_ref,
            task_prompt_hash,
            started_at: Utc::now(),
            completed_at: None,
            status: RunStatus::Running,
            agents,
        }
    }

    pub fn write_to(&self, path: &Path) -> Result<(), ArtifactError> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn read_from(path: &Path) -> Result<Self, ArtifactError> {
        if !path.exists() {
            return Err(ArtifactError::ManifestNotFound {
                path: path.display().to_string(),
            });
        }
        let data = std::fs::read_to_string(path)?;
        let manifest: Self = serde_json::from_str(&data)?;
        Ok(manifest)
    }

    pub fn mark_completed(&mut self, status: RunStatus) {
        self.completed_at = Some(Utc::now());
        self.status = status;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    TimedOut,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEntry {
    pub agent_key: String,
    pub tier: String,
    pub branch: String,
    pub worktree_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_manifest() -> RunManifest {
        RunManifest::new(
            Uuid::new_v4(),
            "/home/user/project".to_string(),
            "main".to_string(),
            "abc123".to_string(),
            vec![AgentEntry {
                agent_key: "claude".to_string(),
                tier: "tier-1".to_string(),
                branch: "hydra/test-run/agent/claude".to_string(),
                worktree_path: None,
            }],
        )
    }

    #[test]
    fn manifest_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("manifest.json");

        let original = make_test_manifest();
        original.write_to(&path).unwrap();

        let loaded = RunManifest::read_from(&path).unwrap();
        assert_eq!(loaded.run_id, original.run_id);
        assert_eq!(loaded.schema_version, RunManifest::CURRENT_SCHEMA_VERSION);
        assert_eq!(loaded.status, RunStatus::Running);
        assert_eq!(loaded.agents.len(), 1);
        assert_eq!(loaded.agents[0].agent_key, "claude");
    }

    #[test]
    fn manifest_mark_completed() {
        let mut manifest = make_test_manifest();
        assert!(manifest.completed_at.is_none());

        manifest.mark_completed(RunStatus::Completed);
        assert!(manifest.completed_at.is_some());
        assert_eq!(manifest.status, RunStatus::Completed);
    }

    #[test]
    fn manifest_read_missing_file_returns_error() {
        let result = RunManifest::read_from(Path::new("/nonexistent/manifest.json"));
        assert!(result.is_err());
    }

    #[test]
    fn schema_version_is_present_in_json() {
        let manifest = make_test_manifest();
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("\"schema_version\":2"));
        assert!(json.contains("\"event_schema_version\":1"));
    }

    #[test]
    fn manifest_includes_all_statuses() {
        for status in [
            RunStatus::Running,
            RunStatus::Completed,
            RunStatus::Failed,
            RunStatus::TimedOut,
            RunStatus::Interrupted,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: RunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }
}
