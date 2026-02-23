use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::error::{HydraError, Result};

use super::events::RunEvent;
use super::manifest::RunManifest;

const MANIFEST_FILE: &str = "manifest.json";
const EVENTS_FILE: &str = "events.jsonl";

/// Manages a single run's artifact directory at `.hydra/runs/<run_id>/`.
#[derive(Debug, Clone)]
pub struct RunDir {
    path: PathBuf,
}

impl RunDir {
    /// Create a new run directory under `repo_root/.hydra/runs/<run_id>/`.
    ///
    /// Creates all intermediate directories as needed.
    pub fn create(repo_root: &Path, run_id: Uuid) -> Result<Self> {
        let path = repo_root
            .join(".hydra")
            .join("runs")
            .join(run_id.to_string());

        fs::create_dir_all(&path).map_err(|e| {
            HydraError::Artifact(format!(
                "failed to create run directory {}: {e}",
                path.display()
            ))
        })?;

        Ok(Self { path })
    }

    /// Open an existing run directory without creating it.
    pub fn open(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    /// Returns the root path of this run directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write (or overwrite) the run manifest.
    pub fn write_manifest(&self, manifest: &RunManifest) -> Result<()> {
        let manifest_path = self.path.join(MANIFEST_FILE);
        let json = serde_json::to_string_pretty(manifest)
            .map_err(|e| HydraError::Artifact(format!("failed to serialize manifest: {e}")))?;
        fs::write(&manifest_path, json).map_err(|e| {
            HydraError::Artifact(format!(
                "failed to write manifest {}: {e}",
                manifest_path.display()
            ))
        })
    }

    /// Read the run manifest from disk.
    pub fn read_manifest(&self) -> Result<RunManifest> {
        let manifest_path = self.path.join(MANIFEST_FILE);
        let data = fs::read_to_string(&manifest_path).map_err(|e| {
            HydraError::Artifact(format!(
                "failed to read manifest {}: {e}",
                manifest_path.display()
            ))
        })?;
        serde_json::from_str(&data)
            .map_err(|e| HydraError::Artifact(format!("failed to parse manifest: {e}")))
    }

    /// Append a single event as a JSONL line to `events.jsonl`.
    pub fn append_event(&self, event: &RunEvent) -> Result<()> {
        let events_path = self.path.join(EVENTS_FILE);
        let mut line = serde_json::to_string(event)
            .map_err(|e| HydraError::Artifact(format!("failed to serialize event: {e}")))?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)
            .map_err(|e| {
                HydraError::Artifact(format!(
                    "failed to open events file {}: {e}",
                    events_path.display()
                ))
            })?;

        file.write_all(line.as_bytes())
            .map_err(|e| HydraError::Artifact(format!("failed to write event: {e}")))
    }

    /// Read all events from the JSONL file.
    ///
    /// Returns an empty vec if the events file does not exist.
    pub fn read_events(&self) -> Result<Vec<RunEvent>> {
        let events_path = self.path.join(EVENTS_FILE);
        if !events_path.exists() {
            return Ok(vec![]);
        }

        let data = fs::read_to_string(&events_path).map_err(|e| {
            HydraError::Artifact(format!(
                "failed to read events file {}: {e}",
                events_path.display()
            ))
        })?;

        let mut events = Vec::new();
        for (i, line) in data.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let event: RunEvent = serde_json::from_str(line).map_err(|e| {
                HydraError::Artifact(format!("failed to parse event at line {}: {e}", i + 1))
            })?;
            events.push(event);
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::events::EventType;
    use crate::artifact::manifest::{RunManifest, RunStatus};
    use chrono::Utc;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn sample_manifest(run_id: Uuid) -> RunManifest {
        RunManifest {
            schema_version: "1.0.0".into(),
            run_id,
            repo_root: PathBuf::from("/tmp/test-repo"),
            base_ref: "main".into(),
            task_prompt_hash: "deadbeef".into(),
            started_at: Utc::now(),
            completed_at: None,
            status: RunStatus::Running,
            agents: vec![],
        }
    }

    fn sample_event(run_id: Uuid, event_type: EventType) -> RunEvent {
        RunEvent {
            timestamp: Utc::now(),
            run_id,
            event_type,
            agent_key: None,
            data: serde_json::json!({}),
        }
    }

    #[test]
    fn create_and_read_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let run_id = Uuid::new_v4();

        let rd = RunDir::create(tmp.path(), run_id).unwrap();
        let manifest = sample_manifest(run_id);
        rd.write_manifest(&manifest).unwrap();

        let read_back = rd.read_manifest().unwrap();
        assert_eq!(read_back.run_id, run_id);
        assert_eq!(read_back.schema_version, "1.0.0");
        assert_eq!(read_back.status, RunStatus::Running);
    }

    #[test]
    fn append_and_read_events() {
        let tmp = tempfile::tempdir().unwrap();
        let run_id = Uuid::new_v4();

        let rd = RunDir::create(tmp.path(), run_id).unwrap();

        // Initially empty
        let events = rd.read_events().unwrap();
        assert!(events.is_empty());

        // Append some events
        rd.append_event(&sample_event(run_id, EventType::RunStarted))
            .unwrap();
        rd.append_event(&sample_event(run_id, EventType::AgentStarted))
            .unwrap();
        rd.append_event(&sample_event(run_id, EventType::RunCompleted))
            .unwrap();

        let events = rd.read_events().unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, EventType::RunStarted);
        assert_eq!(events[1].event_type, EventType::AgentStarted);
        assert_eq!(events[2].event_type, EventType::RunCompleted);
    }

    #[test]
    fn run_dir_path_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let run_id = Uuid::new_v4();

        let rd = RunDir::create(tmp.path(), run_id).unwrap();
        let expected = tmp
            .path()
            .join(".hydra")
            .join("runs")
            .join(run_id.to_string());
        assert_eq!(rd.path(), expected);
        assert!(rd.path().is_dir());
    }

    #[test]
    fn open_existing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let run_id = Uuid::new_v4();

        let rd = RunDir::create(tmp.path(), run_id).unwrap();
        let manifest = sample_manifest(run_id);
        rd.write_manifest(&manifest).unwrap();

        // Open via path directly
        let rd2 = RunDir::open(rd.path());
        let read_back = rd2.read_manifest().unwrap();
        assert_eq!(read_back.run_id, run_id);
    }
}
