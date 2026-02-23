use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::ArtifactError;

/// Deterministic directory layout for a single run's artifacts.
///
/// Structure:
/// ```text
/// .hydra/runs/<run_id>/
///   manifest.json
///   events.jsonl
///   agents/<agent_key>/
///     stdout.log
///     stderr.log
///     diff.patch
///     score.json
/// ```
#[derive(Debug, Clone)]
pub struct RunLayout {
    run_id: Uuid,
    base_dir: PathBuf,
}

impl RunLayout {
    pub fn new(hydra_root: &Path, run_id: Uuid) -> Self {
        let base_dir = hydra_root.join("runs").join(run_id.to_string());
        Self { run_id, base_dir }
    }

    pub fn run_id(&self) -> Uuid {
        self.run_id
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.base_dir.join("manifest.json")
    }

    pub fn events_path(&self) -> PathBuf {
        self.base_dir.join("events.jsonl")
    }

    pub fn agent_dir(&self, agent_key: &str) -> PathBuf {
        self.base_dir.join("agents").join(agent_key)
    }

    pub fn agent_stdout(&self, agent_key: &str) -> PathBuf {
        self.agent_dir(agent_key).join("stdout.log")
    }

    pub fn agent_stderr(&self, agent_key: &str) -> PathBuf {
        self.agent_dir(agent_key).join("stderr.log")
    }

    pub fn agent_diff(&self, agent_key: &str) -> PathBuf {
        self.agent_dir(agent_key).join("diff.patch")
    }

    pub fn agent_score(&self, agent_key: &str) -> PathBuf {
        self.agent_dir(agent_key).join("score.json")
    }

    pub fn baseline_dir(&self) -> PathBuf {
        self.base_dir.join("baseline")
    }

    pub fn baseline_build_log(&self) -> PathBuf {
        self.baseline_dir().join("build.log")
    }

    pub fn baseline_test_log(&self) -> PathBuf {
        self.baseline_dir().join("test.log")
    }

    pub fn baseline_lint_log(&self) -> PathBuf {
        self.baseline_dir().join("lint.log")
    }

    pub fn baseline_result(&self) -> PathBuf {
        self.baseline_dir().join("baseline.json")
    }

    /// Create the full directory tree for this run.
    pub fn create_dirs(&self, agent_keys: &[&str]) -> Result<(), ArtifactError> {
        if self.base_dir.exists() {
            return Err(ArtifactError::RunAlreadyExists {
                path: self.base_dir.display().to_string(),
            });
        }

        std::fs::create_dir_all(&self.base_dir)?;

        for key in agent_keys {
            std::fs::create_dir_all(self.agent_dir(key))?;
        }

        Ok(())
    }

    /// List all existing run IDs under the hydra root.
    pub fn list_runs(hydra_root: &Path) -> Result<Vec<Uuid>, ArtifactError> {
        let runs_dir = hydra_root.join("runs");
        if !runs_dir.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        for entry in std::fs::read_dir(runs_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Ok(id) = Uuid::parse_str(name) {
                        ids.push(id);
                    }
                }
            }
        }

        Ok(ids)
    }

    /// Remove this run's directory tree.
    pub fn cleanup(&self) -> Result<(), ArtifactError> {
        if self.base_dir.exists() {
            std::fs::remove_dir_all(&self.base_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn layout_paths_are_deterministic() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let layout = RunLayout::new(Path::new("/tmp/.hydra"), id);

        assert_eq!(
            layout.base_dir(),
            Path::new("/tmp/.hydra/runs/550e8400-e29b-41d4-a716-446655440000")
        );
        assert!(layout.manifest_path().ends_with("manifest.json"));
        assert!(layout.events_path().ends_with("events.jsonl"));
        assert!(layout
            .agent_stdout("claude")
            .ends_with("agents/claude/stdout.log"));
    }

    #[test]
    fn create_and_cleanup_dirs() {
        let tmp = TempDir::new().unwrap();
        let hydra_root = tmp.path().join(".hydra");
        let id = Uuid::new_v4();
        let layout = RunLayout::new(&hydra_root, id);

        layout.create_dirs(&["claude", "codex"]).unwrap();

        assert!(layout.base_dir().exists());
        assert!(layout.agent_dir("claude").exists());
        assert!(layout.agent_dir("codex").exists());

        layout.cleanup().unwrap();
        assert!(!layout.base_dir().exists());
    }

    #[test]
    fn create_dirs_fails_if_exists() {
        let tmp = TempDir::new().unwrap();
        let hydra_root = tmp.path().join(".hydra");
        let id = Uuid::new_v4();
        let layout = RunLayout::new(&hydra_root, id);

        layout.create_dirs(&[]).unwrap();

        let result = layout.create_dirs(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn list_runs_returns_existing() {
        let tmp = TempDir::new().unwrap();
        let hydra_root = tmp.path().join(".hydra");

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        RunLayout::new(&hydra_root, id1).create_dirs(&[]).unwrap();
        RunLayout::new(&hydra_root, id2).create_dirs(&[]).unwrap();

        let mut runs = RunLayout::list_runs(&hydra_root).unwrap();
        runs.sort();
        let mut expected = vec![id1, id2];
        expected.sort();
        assert_eq!(runs, expected);
    }

    #[test]
    fn list_runs_empty_when_no_dir() {
        let tmp = TempDir::new().unwrap();
        let hydra_root = tmp.path().join(".hydra");
        let runs = RunLayout::list_runs(&hydra_root).unwrap();
        assert!(runs.is_empty());
    }

    #[test]
    fn paths_use_forward_separators_in_components() {
        let id = Uuid::new_v4();
        let layout = RunLayout::new(Path::new("/project/.hydra"), id);
        let agent_path = layout.agent_dir("codex");
        assert!(agent_path.to_string_lossy().contains("agents"));
        assert!(agent_path.to_string_lossy().contains("codex"));
    }
}
