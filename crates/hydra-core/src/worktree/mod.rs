use std::path::{Path, PathBuf};

use thiserror::Error;
use tokio::process::Command;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum WorktreeError {
    #[error("git command failed: {detail}")]
    GitFailed { detail: String },

    #[error("worktree already exists at '{path}'")]
    AlreadyExists { path: String },

    #[error("worktree not found at '{path}'")]
    NotFound { path: String },

    #[error("not inside a git repository")]
    NotARepo,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Metadata about a created worktree.
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub run_id: Uuid,
    pub agent_key: String,
}

/// Metadata returned by `list()` for existing worktrees.
#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    pub path: PathBuf,
    pub branch: String,
    pub head_commit: String,
}

/// Service for managing git worktrees used for agent isolation.
///
/// Each agent in a run gets its own worktree branched from the base ref.
/// The service provides create/list/remove with cleanup-safe semantics.
pub struct WorktreeService {
    repo_root: PathBuf,
    base_dir: PathBuf,
}

impl WorktreeService {
    pub fn new(repo_root: PathBuf, base_dir: PathBuf) -> Self {
        Self {
            repo_root,
            base_dir,
        }
    }

    /// Create a new worktree for an agent run.
    ///
    /// Branch: `hydra/<run_id>/agent/<agent_key>`
    /// Path:   `<base_dir>/<run_id>/<agent_key>/`
    pub async fn create(
        &self,
        run_id: Uuid,
        agent_key: &str,
        base_ref: &str,
    ) -> Result<WorktreeInfo, WorktreeError> {
        let branch = format!("hydra/{run_id}/agent/{agent_key}");
        let wt_path = self.base_dir.join(run_id.to_string()).join(agent_key);

        if wt_path.exists() {
            return Err(WorktreeError::AlreadyExists {
                path: wt_path.display().to_string(),
            });
        }

        if let Some(parent) = wt_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                &branch,
                &wt_path.display().to_string(),
                base_ref,
            ])
            .current_dir(&self.repo_root)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::GitFailed {
                detail: stderr.trim().to_string(),
            });
        }

        tracing::info!(
            run_id = %run_id,
            agent = agent_key,
            path = %wt_path.display(),
            branch = %branch,
            "created worktree"
        );

        Ok(WorktreeInfo {
            path: wt_path,
            branch,
            run_id,
            agent_key: agent_key.to_string(),
        })
    }

    /// List all worktrees known to git in this repo.
    pub async fn list(&self) -> Result<Vec<WorktreeEntry>, WorktreeError> {
        let output = Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .current_dir(&self.repo_root)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::GitFailed {
                detail: stderr.trim().to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_worktree_list_porcelain(&stdout))
    }

    /// Remove a worktree by path. Cleans up both the directory and the git reference.
    pub async fn remove(&self, wt_path: &Path, force: bool) -> Result<(), WorktreeError> {
        if !wt_path.exists() {
            return Err(WorktreeError::NotFound {
                path: wt_path.display().to_string(),
            });
        }

        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        let wt_str = wt_path.display().to_string();
        args.push(&wt_str);

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.repo_root)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::GitFailed {
                detail: stderr.trim().to_string(),
            });
        }

        tracing::info!(path = %wt_path.display(), "removed worktree");
        Ok(())
    }

    /// Force-remove a worktree and delete its associated branch.
    /// Used for cleanup on interrupt or failure.
    pub async fn force_cleanup(&self, info: &WorktreeInfo) -> Result<(), WorktreeError> {
        if info.path.exists() {
            if let Err(e) = self.remove(&info.path, true).await {
                tracing::warn!(
                    path = %info.path.display(),
                    error = %e,
                    "force cleanup: worktree remove failed, removing directory manually"
                );
                let _ = tokio::fs::remove_dir_all(&info.path).await;

                let _ = Command::new("git")
                    .args(["worktree", "prune"])
                    .current_dir(&self.repo_root)
                    .output()
                    .await;
            }
        }

        let _ = Command::new("git")
            .args(["branch", "-D", &info.branch])
            .current_dir(&self.repo_root)
            .output()
            .await;

        tracing::info!(
            branch = %info.branch,
            path = %info.path.display(),
            "force cleanup complete"
        );
        Ok(())
    }
}

/// Parse `git worktree list --porcelain` output into entries.
fn parse_worktree_list_porcelain(output: &str) -> Vec<WorktreeEntry> {
    let mut entries = Vec::new();
    let mut path: Option<PathBuf> = None;
    let mut head = String::new();
    let mut branch = String::new();

    for line in output.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            path = Some(PathBuf::from(p));
        } else if let Some(h) = line.strip_prefix("HEAD ") {
            head = h.to_string();
        } else if let Some(b) = line.strip_prefix("branch ") {
            branch = b.strip_prefix("refs/heads/").unwrap_or(b).to_string();
        } else if line.is_empty() {
            if let Some(p) = path.take() {
                entries.push(WorktreeEntry {
                    path: p,
                    branch: std::mem::take(&mut branch),
                    head_commit: std::mem::take(&mut head),
                });
            }
        }
    }

    if let Some(p) = path.take() {
        entries.push(WorktreeEntry {
            path: p,
            branch,
            head_commit: head,
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_test_repo(dir: &Path) {
        use std::process::Command as StdCommand;
        StdCommand::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["config", "user.email", "test@hydra.dev"])
            .current_dir(dir)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["config", "user.name", "Hydra Test"])
            .current_dir(dir)
            .output()
            .unwrap();
        std::fs::write(dir.join("README.md"), "# test").unwrap();
        StdCommand::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn parse_porcelain_output() {
        let output = "\
worktree /home/user/repo
HEAD abc123def456
branch refs/heads/main

worktree /home/user/repo/.hydra/worktrees/run1/claude
HEAD def789abc012
branch refs/heads/hydra/run1/agent/claude

";
        let entries = parse_worktree_list_porcelain(output);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].branch, "main");
        assert_eq!(entries[1].branch, "hydra/run1/agent/claude");
        assert_eq!(entries[1].head_commit, "def789abc012");
    }

    #[test]
    fn parse_porcelain_with_bare_worktree() {
        let output = "\
worktree /home/user/repo
HEAD abc123
branch refs/heads/main

worktree /tmp/wt
HEAD 000000
bare

";
        let entries = parse_worktree_list_porcelain(output);
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn create_and_remove_worktree() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_test_repo(&repo);

        let wt_base = tmp.path().join("worktrees");
        let svc = WorktreeService::new(repo.clone(), wt_base);

        let run_id = Uuid::new_v4();
        let info = svc.create(run_id, "claude", "HEAD").await.unwrap();

        assert!(info.path.exists());
        assert!(info.branch.contains("claude"));
        assert!(info.path.join("README.md").exists());

        let entries = svc.list().await.unwrap();
        assert!(entries.len() >= 2); // main + new worktree

        svc.remove(&info.path, false).await.unwrap();
        assert!(!info.path.exists());
    }

    #[tokio::test]
    async fn create_duplicate_worktree_fails() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_test_repo(&repo);

        let wt_base = tmp.path().join("worktrees");
        let svc = WorktreeService::new(repo.clone(), wt_base);
        let run_id = Uuid::new_v4();

        let info = svc.create(run_id, "codex", "HEAD").await.unwrap();
        let result = svc.create(run_id, "codex", "HEAD").await;
        assert!(result.is_err());

        svc.force_cleanup(&info).await.unwrap();
    }

    #[tokio::test]
    async fn force_cleanup_removes_worktree_and_branch() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_test_repo(&repo);

        let wt_base = tmp.path().join("worktrees");
        let svc = WorktreeService::new(repo.clone(), wt_base);
        let run_id = Uuid::new_v4();

        let info = svc.create(run_id, "claude", "HEAD").await.unwrap();
        assert!(info.path.exists());

        svc.force_cleanup(&info).await.unwrap();
        assert!(!info.path.exists());

        let branch_check = std::process::Command::new("git")
            .args(["branch", "--list", &info.branch])
            .current_dir(&repo)
            .output()
            .unwrap();
        let branch_output = String::from_utf8_lossy(&branch_check.stdout);
        assert!(
            branch_output.trim().is_empty(),
            "branch should be deleted after cleanup"
        );
    }

    #[tokio::test]
    async fn remove_nonexistent_worktree_fails() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_test_repo(&repo);

        let svc = WorktreeService::new(repo, tmp.path().join("worktrees"));
        let result = svc.remove(Path::new("/tmp/nonexistent-wt"), false).await;
        assert!(result.is_err());
    }
}
