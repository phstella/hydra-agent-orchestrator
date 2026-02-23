//! Git worktree lifecycle management for agent isolation.
//!
//! Each agent run gets its own worktree under `.hydra/worktrees/<run_id>/<agent_key>/`
//! with a branch named `hydra/<run_id_short>/agent/<agent_key>`.

use std::path::{Path, PathBuf};

use tokio::process::Command;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{HydraError, Result};

/// Represents a managed worktree tied to a specific agent run.
#[derive(Debug, Clone)]
pub struct ManagedWorktree {
    pub run_id: Uuid,
    pub agent_key: String,
    pub path: PathBuf,
    pub branch: String,
}

/// Service for creating, listing, and tearing down git worktrees.
pub struct WorktreeService {
    repo_root: PathBuf,
}

impl WorktreeService {
    /// Create a new service rooted at the given repository path.
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    /// Return the base directory where all hydra worktrees live.
    fn worktrees_dir(&self) -> PathBuf {
        self.repo_root.join(".hydra").join("worktrees")
    }

    /// Build the canonical branch name for an agent run.
    fn branch_name(run_id: Uuid, agent_key: &str) -> String {
        let short = &run_id.to_string()[..8];
        format!("hydra/{short}/agent/{agent_key}")
    }

    /// Build the canonical worktree path for an agent run.
    fn worktree_path(&self, run_id: Uuid, agent_key: &str) -> PathBuf {
        self.worktrees_dir()
            .join(run_id.to_string())
            .join(agent_key)
    }

    /// Validate that `path` is inside `repo_root` (prevents path traversal).
    fn validate_path(&self, path: &Path) -> Result<()> {
        let canon_root = self
            .repo_root
            .canonicalize()
            .map_err(|e| HydraError::Worktree(format!("cannot canonicalize repo root: {e}")))?;
        // The path might not exist yet, so canonicalize its parent.
        let parent = path
            .parent()
            .ok_or_else(|| HydraError::Worktree("worktree path has no parent".into()))?;
        let canon_parent = parent.canonicalize().map_err(|e| {
            HydraError::Worktree(format!("cannot canonicalize worktree parent: {e}"))
        })?;
        if !canon_parent.starts_with(&canon_root) {
            return Err(HydraError::Worktree(format!(
                "worktree path {path:?} escapes repo root {canon_root:?}"
            )));
        }
        Ok(())
    }

    /// Create a worktree for an agent run.
    ///
    /// Creates a new branch from `base_ref` and adds a git worktree.
    pub async fn create(
        &self,
        run_id: Uuid,
        agent_key: &str,
        base_ref: &str,
    ) -> Result<ManagedWorktree> {
        let branch = Self::branch_name(run_id, agent_key);
        let wt_path = self.worktree_path(run_id, agent_key);

        // Ensure parent directories exist.
        let parent = wt_path
            .parent()
            .ok_or_else(|| HydraError::Worktree("worktree path has no parent".into()))?;
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| HydraError::Worktree(format!("failed to create directories: {e}")))?;

        self.validate_path(&wt_path)?;

        info!(%run_id, agent_key, %branch, path = %wt_path.display(), "creating worktree");

        create_branch(&self.repo_root, &branch, base_ref).await?;
        add_worktree(&self.repo_root, &wt_path, &branch).await?;

        Ok(ManagedWorktree {
            run_id,
            agent_key: agent_key.to_string(),
            path: wt_path,
            branch,
        })
    }

    /// List all hydra-managed worktrees found in the repo.
    pub async fn list(&self) -> Result<Vec<ManagedWorktree>> {
        let raw = list_worktrees(&self.repo_root).await?;
        let mut managed = Vec::new();
        for (path, branch) in raw {
            if let Some(wt) = Self::parse_managed(&path, &branch) {
                managed.push(wt);
            }
        }
        Ok(managed)
    }

    /// Remove a specific worktree and delete its branch.
    pub async fn remove(&self, worktree: &ManagedWorktree) -> Result<()> {
        info!(
            run_id = %worktree.run_id,
            agent_key = %worktree.agent_key,
            "removing worktree"
        );
        remove_worktree(&self.repo_root, &worktree.path).await?;
        delete_branch(&self.repo_root, &worktree.branch).await?;

        // Clean up empty parent directories.
        Self::cleanup_empty_dirs(&worktree.path).await;
        Ok(())
    }

    /// Remove all worktrees belonging to a specific run.
    pub async fn cleanup_run(&self, run_id: Uuid) -> Result<()> {
        info!(%run_id, "cleaning up all worktrees for run");
        let worktrees = self.list().await?;
        for wt in worktrees {
            if wt.run_id == run_id {
                if let Err(e) = self.remove(&wt).await {
                    warn!(
                        run_id = %wt.run_id,
                        agent_key = %wt.agent_key,
                        error = %e,
                        "failed to remove worktree during run cleanup, attempting force"
                    );
                    self.force_remove(&wt).await;
                }
            }
        }
        Ok(())
    }

    /// Emergency cleanup: remove every hydra-managed worktree.
    pub async fn cleanup_all(&self) -> Result<()> {
        info!("emergency cleanup: removing all hydra worktrees");
        let worktrees = self.list().await?;
        for wt in worktrees {
            if let Err(e) = self.remove(&wt).await {
                warn!(
                    run_id = %wt.run_id,
                    agent_key = %wt.agent_key,
                    error = %e,
                    "failed to remove worktree during full cleanup, attempting force"
                );
                self.force_remove(&wt).await;
            }
        }
        Ok(())
    }

    /// Force-remove a worktree. Used as a fallback when normal removal fails.
    async fn force_remove(&self, worktree: &ManagedWorktree) {
        debug!(path = %worktree.path.display(), "force-removing worktree");
        let _ = force_remove_worktree(&self.repo_root, &worktree.path).await;
        let _ = delete_branch(&self.repo_root, &worktree.branch).await;
        Self::cleanup_empty_dirs(&worktree.path).await;
    }

    /// Try to remove empty parent directories up to the .hydra/worktrees level.
    async fn cleanup_empty_dirs(path: &Path) {
        let mut current = path.to_path_buf();
        // Walk up at most 2 levels (agent_key dir, run_id dir).
        for _ in 0..2 {
            if let Some(parent) = current.parent() {
                // remove_dir only succeeds if the directory is empty.
                if tokio::fs::remove_dir(&current).await.is_err() {
                    break;
                }
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
    }

    /// Parse a worktree entry into a `ManagedWorktree` if it matches the hydra pattern.
    ///
    /// Branch pattern: `hydra/<uuid_short>/agent/<agent_key>`
    fn parse_managed(path: &Path, branch: &str) -> Option<ManagedWorktree> {
        let parts: Vec<&str> = branch.split('/').collect();
        // Expected: ["hydra", "<uuid_short>", "agent", "<agent_key>"]
        if parts.len() != 4 || parts[0] != "hydra" || parts[2] != "agent" {
            return None;
        }
        let agent_key = parts[3].to_string();

        // Try to extract the full UUID from the path.
        // Path pattern: .../worktrees/<full_uuid>/<agent_key>
        let run_id = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|name| name.to_str())
            .and_then(|s| Uuid::parse_str(s).ok())?;

        Some(ManagedWorktree {
            run_id,
            agent_key,
            path: path.to_path_buf(),
            branch: branch.to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// Git CLI helpers
// ---------------------------------------------------------------------------

/// Run a git command in the context of `repo_root` and return its stdout.
async fn git_command(repo_root: &Path, args: &[&str]) -> Result<String> {
    debug!(cwd = %repo_root.display(), ?args, "running git command");
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .await
        .map_err(|e| HydraError::Git(format!("failed to execute git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(HydraError::Git(format!("git {args:?} failed: {stderr}")));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Create a new branch from `base_ref`.
async fn create_branch(repo_root: &Path, branch: &str, base_ref: &str) -> Result<()> {
    git_command(repo_root, &["branch", branch, base_ref]).await?;
    Ok(())
}

/// Add a git worktree at `path` for `branch`.
async fn add_worktree(repo_root: &Path, path: &Path, branch: &str) -> Result<()> {
    let path_str = path
        .to_str()
        .ok_or_else(|| HydraError::Worktree("worktree path is not valid UTF-8".into()))?;
    git_command(repo_root, &["worktree", "add", path_str, branch]).await?;
    Ok(())
}

/// Remove a git worktree.
async fn remove_worktree(repo_root: &Path, path: &Path) -> Result<()> {
    let path_str = path
        .to_str()
        .ok_or_else(|| HydraError::Worktree("worktree path is not valid UTF-8".into()))?;
    git_command(repo_root, &["worktree", "remove", path_str]).await?;
    Ok(())
}

/// Force-remove a git worktree (used when normal remove fails).
async fn force_remove_worktree(repo_root: &Path, path: &Path) -> Result<()> {
    let path_str = path
        .to_str()
        .ok_or_else(|| HydraError::Worktree("worktree path is not valid UTF-8".into()))?;
    git_command(repo_root, &["worktree", "remove", "--force", path_str]).await?;
    Ok(())
}

/// Delete a local branch. Ignores errors (branch may already be gone).
async fn delete_branch(repo_root: &Path, branch: &str) -> Result<()> {
    // Use -D to force-delete even if not fully merged.
    match git_command(repo_root, &["branch", "-D", branch]).await {
        Ok(_) => Ok(()),
        Err(e) => {
            debug!(branch, error = %e, "branch deletion failed (may already be gone)");
            Ok(())
        }
    }
}

/// List all git worktrees with their paths and branch names.
async fn list_worktrees(repo_root: &Path) -> Result<Vec<(PathBuf, String)>> {
    let output = git_command(repo_root, &["worktree", "list", "--porcelain"]).await?;
    let mut result = Vec::new();
    let mut current_path: Option<PathBuf> = None;

    for line in output.lines() {
        if let Some(path_str) = line.strip_prefix("worktree ") {
            current_path = Some(PathBuf::from(path_str));
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            if let Some(path) = current_path.take() {
                // branch_ref looks like "refs/heads/hydra/..."
                let branch = branch_ref
                    .strip_prefix("refs/heads/")
                    .unwrap_or(branch_ref)
                    .to_string();
                result.push((path, branch));
            }
        } else if line.is_empty() {
            current_path = None;
        }
    }

    Ok(result)
}

/// Register a Ctrl+C handler that cleans up all hydra worktrees.
///
/// Call once at program start. The handler is best-effort: cleanup failures
/// are logged but do not prevent shutdown.
pub fn register_cleanup_handler(repo_root: PathBuf) {
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            warn!("received Ctrl+C, cleaning up hydra worktrees");
            let svc = WorktreeService::new(repo_root);
            if let Err(e) = svc.cleanup_all().await {
                warn!(error = %e, "cleanup on interrupt failed");
            }
            std::process::exit(130);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a temporary bare-ish git repo with an initial commit so we have
    /// a valid base ref to branch from.
    async fn setup_test_repo() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let root = tmp.path().to_path_buf();

        git_command(&root, &["init"]).await.unwrap();
        git_command(&root, &["config", "user.email", "test@hydra.dev"])
            .await
            .unwrap();
        git_command(&root, &["config", "user.name", "Hydra Test"])
            .await
            .unwrap();

        // Create an initial commit so HEAD is valid.
        let placeholder = root.join("README.md");
        tokio::fs::write(&placeholder, "# test repo\n")
            .await
            .unwrap();
        git_command(&root, &["add", "."]).await.unwrap();
        git_command(&root, &["commit", "-m", "initial commit"])
            .await
            .unwrap();

        (tmp, root)
    }

    #[tokio::test]
    async fn create_list_remove_lifecycle() {
        let (_tmp, root) = setup_test_repo().await;
        let svc = WorktreeService::new(root);
        let run_id = Uuid::new_v4();

        // Create
        let wt = svc.create(run_id, "claude", "HEAD").await.unwrap();
        assert!(wt.path.exists());
        assert!(wt.branch.contains("agent/claude"));

        // List
        let listed = svc.list().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].run_id, run_id);
        assert_eq!(listed[0].agent_key, "claude");

        // Remove
        svc.remove(&wt).await.unwrap();
        assert!(!wt.path.exists());

        let listed = svc.list().await.unwrap();
        assert_eq!(listed.len(), 0);
    }

    #[tokio::test]
    async fn cleanup_run_removes_all_agent_worktrees() {
        let (_tmp, root) = setup_test_repo().await;
        let svc = WorktreeService::new(root);
        let run_id = Uuid::new_v4();

        svc.create(run_id, "claude", "HEAD").await.unwrap();
        svc.create(run_id, "codex", "HEAD").await.unwrap();

        let listed = svc.list().await.unwrap();
        assert_eq!(listed.len(), 2);

        svc.cleanup_run(run_id).await.unwrap();

        let listed = svc.list().await.unwrap();
        assert_eq!(listed.len(), 0);
    }

    #[tokio::test]
    async fn cleanup_all_removes_everything() {
        let (_tmp, root) = setup_test_repo().await;
        let svc = WorktreeService::new(root);

        let run1 = Uuid::new_v4();
        let run2 = Uuid::new_v4();

        svc.create(run1, "claude", "HEAD").await.unwrap();
        svc.create(run2, "codex", "HEAD").await.unwrap();

        let listed = svc.list().await.unwrap();
        assert_eq!(listed.len(), 2);

        svc.cleanup_all().await.unwrap();

        let listed = svc.list().await.unwrap();
        assert_eq!(listed.len(), 0);
    }

    #[tokio::test]
    async fn idempotent_cleanup() {
        let (_tmp, root) = setup_test_repo().await;
        let svc = WorktreeService::new(root);
        let run_id = Uuid::new_v4();

        let wt = svc.create(run_id, "claude", "HEAD").await.unwrap();
        svc.remove(&wt).await.unwrap();

        // Second remove should not error (idempotent cleanup_run).
        svc.cleanup_run(run_id).await.unwrap();
        svc.cleanup_all().await.unwrap();
    }

    #[tokio::test]
    async fn path_validation_rejects_traversal() {
        let (_tmp, root) = setup_test_repo().await;
        let svc = WorktreeService::new(root.clone());

        // Build a path that escapes the repo root via enough ".." components.
        // The parent of the bad_path must exist for canonicalize to work,
        // so we use the temp dir's parent (which always exists).
        let bad_path = root.join("..").join("escape_target");
        // Ensure the parent exists for canonicalize.
        let _ = tokio::fs::create_dir_all(bad_path.parent().unwrap()).await;
        let result = svc.validate_path(&bad_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("escapes repo root"));
    }

    #[tokio::test]
    async fn branch_name_uses_short_uuid() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let branch = WorktreeService::branch_name(id, "claude");
        assert_eq!(branch, "hydra/550e8400/agent/claude");
    }
}
