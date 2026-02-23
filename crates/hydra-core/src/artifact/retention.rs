use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{HydraError, Result};

use super::run_dir::RunDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetentionPolicy {
    /// Delete all run artifacts after merge.
    None,
    /// Keep only failed runs.
    Failed,
    /// Keep everything.
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    pub policy: RetentionPolicy,
    pub max_age_days: Option<u64>,
}

/// Enforce the retention policy on all runs under `repo_root/.hydra/runs/`.
///
/// Scans each run directory, reads its manifest, and removes runs that do not
/// satisfy the given policy or that exceed `max_age_days`.
pub fn cleanup(repo_root: &Path, config: &RetentionConfig) -> Result<u64> {
    let runs_dir = repo_root.join(".hydra").join("runs");
    if !runs_dir.exists() {
        debug!(path = %runs_dir.display(), "no runs directory found, nothing to clean");
        return Ok(0);
    }

    let entries = std::fs::read_dir(&runs_dir)
        .map_err(|e| HydraError::Artifact(format!("failed to read runs directory: {e}")))?;

    let now = Utc::now();
    let mut removed: u64 = 0;

    for entry in entries {
        let entry = entry
            .map_err(|e| HydraError::Artifact(format!("failed to read directory entry: {e}")))?;

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let run_dir = RunDir::open(&path);
        let manifest = match run_dir.read_manifest() {
            Ok(m) => m,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "skipping run with unreadable manifest");
                continue;
            }
        };

        let mut should_remove = false;

        // Check policy
        match config.policy {
            RetentionPolicy::None => {
                should_remove = true;
            }
            RetentionPolicy::Failed => {
                use super::manifest::RunStatus;
                if manifest.status != RunStatus::Failed {
                    should_remove = true;
                }
            }
            RetentionPolicy::All => {}
        }

        // Check max age
        if !should_remove {
            if let Some(max_days) = config.max_age_days {
                let age = now.signed_duration_since(manifest.started_at);
                if age.num_days() > max_days as i64 {
                    should_remove = true;
                }
            }
        }

        if should_remove {
            info!(run_id = %manifest.run_id, path = %path.display(), "removing run artifacts");
            std::fs::remove_dir_all(&path).map_err(|e| {
                HydraError::Artifact(format!(
                    "failed to remove run directory {}: {e}",
                    path.display()
                ))
            })?;
            removed += 1;
        }
    }

    info!(removed, "retention cleanup complete");
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::manifest::{RunManifest, RunStatus};
    use crate::artifact::run_dir::RunDir;
    use chrono::Utc;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn make_manifest(status: RunStatus, started_at: chrono::DateTime<Utc>) -> RunManifest {
        RunManifest {
            schema_version: "1.0.0".into(),
            run_id: Uuid::new_v4(),
            repo_root: PathBuf::from("/tmp/fake"),
            base_ref: "main".into(),
            task_prompt_hash: "abc123".into(),
            started_at,
            completed_at: Some(Utc::now()),
            status,
            agents: vec![],
        }
    }

    #[test]
    fn cleanup_policy_none_removes_all() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();

        // Create two runs
        for _ in 0..2 {
            let run_id = Uuid::new_v4();
            let rd = RunDir::create(repo, run_id).unwrap();
            let manifest = make_manifest(RunStatus::Completed, Utc::now());
            rd.write_manifest(&manifest).unwrap();
        }

        let config = RetentionConfig {
            policy: RetentionPolicy::None,
            max_age_days: None,
        };

        let removed = cleanup(repo, &config).unwrap();
        assert_eq!(removed, 2);
        assert!(repo
            .join(".hydra")
            .join("runs")
            .read_dir()
            .unwrap()
            .next()
            .is_none());
    }

    #[test]
    fn cleanup_policy_failed_keeps_failures() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();

        let ok_id = Uuid::new_v4();
        let fail_id = Uuid::new_v4();

        let rd = RunDir::create(repo, ok_id).unwrap();
        rd.write_manifest(&make_manifest(RunStatus::Completed, Utc::now()))
            .unwrap();

        let rd = RunDir::create(repo, fail_id).unwrap();
        rd.write_manifest(&make_manifest(RunStatus::Failed, Utc::now()))
            .unwrap();

        let config = RetentionConfig {
            policy: RetentionPolicy::Failed,
            max_age_days: None,
        };

        let removed = cleanup(repo, &config).unwrap();
        assert_eq!(removed, 1);

        // The failed run should still exist
        assert!(repo
            .join(".hydra")
            .join("runs")
            .join(fail_id.to_string())
            .exists());
        assert!(!repo
            .join(".hydra")
            .join("runs")
            .join(ok_id.to_string())
            .exists());
    }

    #[test]
    fn cleanup_policy_all_with_max_age() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();

        let old_id = Uuid::new_v4();
        let new_id = Uuid::new_v4();

        let old_time = Utc::now() - chrono::Duration::days(100);
        let rd = RunDir::create(repo, old_id).unwrap();
        rd.write_manifest(&make_manifest(RunStatus::Completed, old_time))
            .unwrap();

        let rd = RunDir::create(repo, new_id).unwrap();
        rd.write_manifest(&make_manifest(RunStatus::Completed, Utc::now()))
            .unwrap();

        let config = RetentionConfig {
            policy: RetentionPolicy::All,
            max_age_days: Some(30),
        };

        let removed = cleanup(repo, &config).unwrap();
        assert_eq!(removed, 1);
        assert!(!repo
            .join(".hydra")
            .join("runs")
            .join(old_id.to_string())
            .exists());
        assert!(repo
            .join(".hydra")
            .join("runs")
            .join(new_id.to_string())
            .exists());
    }

    #[test]
    fn cleanup_no_runs_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let config = RetentionConfig {
            policy: RetentionPolicy::None,
            max_age_days: None,
        };
        let removed = cleanup(tmp.path(), &config).unwrap();
        assert_eq!(removed, 0);
    }
}
