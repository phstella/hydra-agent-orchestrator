//! Git merge service for agent run results.
//!
//! Provides dry-run merge checking and actual merge execution for
//! bringing agent worktree branches into a target branch.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::{HydraError, Result};

/// Report produced by a merge dry-run or actual merge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeReport {
    pub source_branch: String,
    pub target_branch: String,
    pub dry_run: bool,
    pub can_merge: bool,
    pub conflicts: Vec<ConflictFile>,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

/// A file that has a merge conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictFile {
    pub path: String,
    /// Type of conflict: "content", "rename", "delete"
    pub conflict_type: String,
}

/// Service for performing git merges of agent results.
pub struct MergeService {
    repo_root: PathBuf,
}

impl MergeService {
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    /// Dry-run merge: check for conflicts without modifying working tree.
    ///
    /// Performs `git merge --no-commit --no-ff <source>` then immediately
    /// aborts to leave the tree clean.
    pub async fn dry_run(&self, source_branch: &str, target_branch: &str) -> Result<MergeReport> {
        info!(source_branch, target_branch, "performing merge dry-run");

        // Get diff stats first (before attempting merge)
        let (files_changed, insertions, deletions) =
            self.diff_stats(source_branch, target_branch).await?;

        // Attempt the merge without committing
        let output = Command::new("git")
            .args(["merge", "--no-commit", "--no-ff", source_branch])
            .current_dir(&self.repo_root)
            .output()
            .await
            .map_err(|e| HydraError::Git(format!("failed to execute git merge: {e}")))?;

        let merge_succeeded = output.status.success();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        // Parse conflicts if merge failed
        let conflicts = if !merge_succeeded {
            self.parse_conflicts(&stdout, &stderr).await
        } else {
            Vec::new()
        };

        // Always abort the merge to leave tree clean
        let _ = self.abort_merge().await;

        let report = MergeReport {
            source_branch: source_branch.to_string(),
            target_branch: target_branch.to_string(),
            dry_run: true,
            can_merge: merge_succeeded,
            conflicts,
            files_changed,
            insertions,
            deletions,
        };

        debug!(?report, "dry-run complete");
        Ok(report)
    }

    /// Perform actual merge with `--no-ff`.
    pub async fn merge(&self, source_branch: &str, target_branch: &str) -> Result<MergeReport> {
        info!(source_branch, target_branch, "performing merge");

        let (files_changed, insertions, deletions) =
            self.diff_stats(source_branch, target_branch).await?;

        let output = Command::new("git")
            .args([
                "merge",
                "--no-ff",
                "-m",
                &format!("hydra: merge {source_branch} into {target_branch}"),
                source_branch,
            ])
            .current_dir(&self.repo_root)
            .output()
            .await
            .map_err(|e| HydraError::Git(format!("failed to execute git merge: {e}")))?;

        let merge_succeeded = output.status.success();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if !merge_succeeded {
            let conflicts = self.parse_conflicts(&stdout, &stderr).await;

            // Abort the failed merge
            let _ = self.abort_merge().await;

            let report = MergeReport {
                source_branch: source_branch.to_string(),
                target_branch: target_branch.to_string(),
                dry_run: false,
                can_merge: false,
                conflicts,
                files_changed,
                insertions,
                deletions,
            };

            return Ok(report);
        }

        Ok(MergeReport {
            source_branch: source_branch.to_string(),
            target_branch: target_branch.to_string(),
            dry_run: false,
            can_merge: true,
            conflicts: Vec::new(),
            files_changed,
            insertions,
            deletions,
        })
    }

    /// Get diff stats between two branches.
    ///
    /// Returns (files_changed, insertions, deletions).
    pub async fn diff_stats(
        &self,
        source_branch: &str,
        target_branch: &str,
    ) -> Result<(u32, u32, u32)> {
        let output = git_command(
            &self.repo_root,
            &[
                "diff",
                "--stat",
                &format!("{target_branch}...{source_branch}"),
            ],
        )
        .await;

        match output {
            Ok(stat_output) => Ok(parse_diff_stat(&stat_output)),
            Err(_) => {
                // If diff fails (e.g., branches don't share history), return zeros
                warn!(
                    source_branch,
                    target_branch, "diff --stat failed, returning zeros"
                );
                Ok((0, 0, 0))
            }
        }
    }

    /// Parse conflict information from git merge output.
    async fn parse_conflicts(&self, stdout: &str, stderr: &str) -> Vec<ConflictFile> {
        let mut conflicts = Vec::new();
        let combined = format!("{stdout}\n{stderr}");

        for line in combined.lines() {
            if let Some(path) = line.strip_prefix("CONFLICT (content): Merge conflict in ") {
                conflicts.push(ConflictFile {
                    path: path.trim().to_string(),
                    conflict_type: "content".to_string(),
                });
            } else if line.starts_with("CONFLICT (rename/delete)") {
                // Extract file path from rename/delete conflicts
                let path = line
                    .split_whitespace()
                    .last()
                    .unwrap_or("unknown")
                    .to_string();
                conflicts.push(ConflictFile {
                    path,
                    conflict_type: "rename".to_string(),
                });
            } else if line.starts_with("CONFLICT (modify/delete)") {
                let path = line
                    .split_whitespace()
                    .last()
                    .unwrap_or("unknown")
                    .to_string();
                conflicts.push(ConflictFile {
                    path,
                    conflict_type: "delete".to_string(),
                });
            }
        }

        // Also try to get unmerged files from git status
        if conflicts.is_empty() {
            if let Ok(status_output) =
                git_command(&self.repo_root, &["diff", "--name-only", "--diff-filter=U"]).await
            {
                for path in status_output.lines() {
                    let path = path.trim();
                    if !path.is_empty() {
                        conflicts.push(ConflictFile {
                            path: path.to_string(),
                            conflict_type: "content".to_string(),
                        });
                    }
                }
            }
        }

        conflicts
    }

    /// Abort an in-progress merge.
    async fn abort_merge(&self) -> Result<()> {
        let _ = git_command(&self.repo_root, &["merge", "--abort"]).await;
        Ok(())
    }
}

/// Run a git command and return stdout.
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

/// Parse the summary line from `git diff --stat` output.
///
/// Typical format: ` N files changed, M insertions(+), K deletions(-)`
fn parse_diff_stat(output: &str) -> (u32, u32, u32) {
    let mut files_changed = 0u32;
    let mut insertions = 0u32;
    let mut deletions = 0u32;

    // The summary line is typically the last non-empty line
    for line in output.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Match patterns like "N file(s) changed"
        for part in line.split(',') {
            let part = part.trim();
            if part.contains("changed") {
                if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                    files_changed = n;
                }
            } else if part.contains("insertion") {
                if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                    insertions = n;
                }
            } else if part.contains("deletion") {
                if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                    deletions = n;
                }
            }
        }

        // Only parse the first matching line (from bottom)
        if files_changed > 0 || insertions > 0 || deletions > 0 {
            break;
        }
    }

    (files_changed, insertions, deletions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_diff_stat_full() {
        let output = " src/main.rs | 10 ++++------\n src/lib.rs  |  5 +++--\n 2 files changed, 7 insertions(+), 8 deletions(-)\n";
        let (files, ins, del) = parse_diff_stat(output);
        assert_eq!(files, 2);
        assert_eq!(ins, 7);
        assert_eq!(del, 8);
    }

    #[test]
    fn parse_diff_stat_insertions_only() {
        let output = " 1 file changed, 3 insertions(+)\n";
        let (files, ins, del) = parse_diff_stat(output);
        assert_eq!(files, 1);
        assert_eq!(ins, 3);
        assert_eq!(del, 0);
    }

    #[test]
    fn parse_diff_stat_deletions_only() {
        let output = " 1 file changed, 5 deletions(-)\n";
        let (files, ins, del) = parse_diff_stat(output);
        assert_eq!(files, 1);
        assert_eq!(ins, 0);
        assert_eq!(del, 5);
    }

    #[test]
    fn parse_diff_stat_empty() {
        let (files, ins, del) = parse_diff_stat("");
        assert_eq!(files, 0);
        assert_eq!(ins, 0);
        assert_eq!(del, 0);
    }

    #[test]
    fn merge_report_serialization() {
        let report = MergeReport {
            source_branch: "feature".to_string(),
            target_branch: "main".to_string(),
            dry_run: true,
            can_merge: false,
            conflicts: vec![ConflictFile {
                path: "src/main.rs".to_string(),
                conflict_type: "content".to_string(),
            }],
            files_changed: 3,
            insertions: 10,
            deletions: 5,
        };
        let json = serde_json::to_string(&report).expect("serialize");
        let deser: MergeReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deser.source_branch, "feature");
        assert!(!deser.can_merge);
        assert_eq!(deser.conflicts.len(), 1);
        assert_eq!(deser.conflicts[0].path, "src/main.rs");
    }

    #[tokio::test]
    async fn dry_run_on_real_repo() {
        // Create a temp git repo with two branches
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();

        // Init repo
        git_command(&root, &["init"]).await.unwrap();
        git_command(&root, &["config", "user.email", "test@hydra.dev"])
            .await
            .unwrap();
        git_command(&root, &["config", "user.name", "Hydra Test"])
            .await
            .unwrap();

        // Initial commit on main
        tokio::fs::write(root.join("file.txt"), "initial\n")
            .await
            .unwrap();
        git_command(&root, &["add", "."]).await.unwrap();
        git_command(&root, &["commit", "-m", "initial"])
            .await
            .unwrap();

        // Create feature branch with changes
        git_command(&root, &["checkout", "-b", "feature"])
            .await
            .unwrap();
        tokio::fs::write(root.join("file.txt"), "modified\n")
            .await
            .unwrap();
        git_command(&root, &["add", "."]).await.unwrap();
        git_command(&root, &["commit", "-m", "feature change"])
            .await
            .unwrap();

        // Switch back to main
        // Try "master" first; fall back to "main" for newer git defaults.
        if git_command(&root, &["checkout", "master"]).await.is_err() {
            git_command(&root, &["checkout", "main"]).await.unwrap();
        }

        let svc = MergeService::new(root);
        let report = svc.dry_run("feature", "HEAD").await.unwrap();

        assert!(report.dry_run);
        assert!(report.can_merge);
        assert!(report.conflicts.is_empty());
        assert!(report.files_changed > 0);
    }

    #[tokio::test]
    async fn merge_conflict_detection() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();

        // Init repo
        git_command(&root, &["init"]).await.unwrap();
        git_command(&root, &["config", "user.email", "test@hydra.dev"])
            .await
            .unwrap();
        git_command(&root, &["config", "user.name", "Hydra Test"])
            .await
            .unwrap();

        // Initial commit
        tokio::fs::write(root.join("file.txt"), "line1\nline2\nline3\n")
            .await
            .unwrap();
        git_command(&root, &["add", "."]).await.unwrap();
        git_command(&root, &["commit", "-m", "initial"])
            .await
            .unwrap();

        // Create feature branch with conflicting changes
        git_command(&root, &["checkout", "-b", "feature"])
            .await
            .unwrap();
        tokio::fs::write(root.join("file.txt"), "feature-line1\nline2\nline3\n")
            .await
            .unwrap();
        git_command(&root, &["add", "."]).await.unwrap();
        git_command(&root, &["commit", "-m", "feature change"])
            .await
            .unwrap();

        // Go back to default branch and make conflicting change
        // Try "master" first; fall back to "main" for newer git defaults.
        if git_command(&root, &["checkout", "master"]).await.is_err() {
            git_command(&root, &["checkout", "main"]).await.unwrap();
        }
        tokio::fs::write(root.join("file.txt"), "main-line1\nline2\nline3\n")
            .await
            .unwrap();
        git_command(&root, &["add", "."]).await.unwrap();
        git_command(&root, &["commit", "-m", "main change"])
            .await
            .unwrap();

        let svc = MergeService::new(root);
        let report = svc.dry_run("feature", "HEAD").await.unwrap();

        assert!(report.dry_run);
        assert!(!report.can_merge);
        assert!(!report.conflicts.is_empty());
    }

    #[tokio::test]
    async fn actual_merge_succeeds() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();

        // Init repo
        git_command(&root, &["init"]).await.unwrap();
        git_command(&root, &["config", "user.email", "test@hydra.dev"])
            .await
            .unwrap();
        git_command(&root, &["config", "user.name", "Hydra Test"])
            .await
            .unwrap();

        // Initial commit
        tokio::fs::write(root.join("file.txt"), "initial\n")
            .await
            .unwrap();
        git_command(&root, &["add", "."]).await.unwrap();
        git_command(&root, &["commit", "-m", "initial"])
            .await
            .unwrap();

        // Create feature branch
        git_command(&root, &["checkout", "-b", "feature"])
            .await
            .unwrap();
        tokio::fs::write(root.join("new_file.txt"), "new content\n")
            .await
            .unwrap();
        git_command(&root, &["add", "."]).await.unwrap();
        git_command(&root, &["commit", "-m", "add new file"])
            .await
            .unwrap();

        // Switch back to default branch
        // Try "master" first; fall back to "main" for newer git defaults.
        if git_command(&root, &["checkout", "master"]).await.is_err() {
            git_command(&root, &["checkout", "main"]).await.unwrap();
        }

        let svc = MergeService::new(root.clone());
        let report = svc.merge("feature", "HEAD").await.unwrap();

        assert!(!report.dry_run);
        assert!(report.can_merge);
        assert!(report.conflicts.is_empty());

        // Verify the file exists after merge
        assert!(root.join("new_file.txt").exists());
    }
}
