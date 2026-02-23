use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::error::{HydraError, Result};

/// Controls what file-system writes are permitted during a run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Allow writes to paths outside the agent's worktree.
    pub allow_writes_outside_worktree: bool,
    /// Disable all sandbox checks. Requires explicit per-run opt-in.
    pub unsafe_mode: bool,
}

/// Validate that `target_path` is inside `worktree_root`, subject to `policy`.
///
/// - If `policy.unsafe_mode` is true, all paths are allowed but a warning is
///   emitted via `tracing`.
/// - If `policy.allow_writes_outside_worktree` is true, all paths are allowed.
/// - Otherwise, `target_path` must be a descendant of `worktree_root`.
pub fn validate_path(
    policy: &SandboxPolicy,
    worktree_root: &Path,
    target_path: &Path,
) -> Result<()> {
    if policy.unsafe_mode {
        warn!(
            target_path = %target_path.display(),
            worktree_root = %worktree_root.display(),
            "unsafe mode: allowing write outside sandbox"
        );
        return Ok(());
    }

    if policy.allow_writes_outside_worktree {
        return Ok(());
    }

    // Canonicalize both paths to resolve symlinks and relative components.
    // If canonicalize fails (e.g. path doesn't exist yet), fall back to
    // checking the parent directory.
    let canonical_root = worktree_root.canonicalize().map_err(|e| {
        HydraError::Artifact(format!(
            "failed to canonicalize worktree root {}: {e}",
            worktree_root.display()
        ))
    })?;

    let canonical_target = if target_path.exists() {
        target_path.canonicalize().map_err(|e| {
            HydraError::Artifact(format!(
                "failed to canonicalize target path {}: {e}",
                target_path.display()
            ))
        })?
    } else {
        // For paths that don't exist yet, canonicalize the parent.
        let parent = target_path.parent().ok_or_else(|| {
            HydraError::Artifact(format!(
                "target path has no parent: {}",
                target_path.display()
            ))
        })?;
        let canonical_parent = parent.canonicalize().map_err(|e| {
            HydraError::Artifact(format!(
                "failed to canonicalize parent of target path {}: {e}",
                parent.display()
            ))
        })?;
        canonical_parent.join(target_path.file_name().unwrap_or_default())
    };

    if canonical_target.starts_with(&canonical_root) {
        Ok(())
    } else {
        Err(HydraError::Artifact(format!(
            "path {} is outside worktree root {}",
            canonical_target.display(),
            canonical_root.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_path_inside_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path();
        let target = worktree.join("src").join("main.rs");
        std::fs::create_dir_all(worktree.join("src")).unwrap();
        std::fs::write(&target, "fn main() {}").unwrap();

        let policy = SandboxPolicy::default();
        assert!(validate_path(&policy, worktree, &target).is_ok());
    }

    #[test]
    fn rejects_path_outside_worktree() {
        let worktree_dir = tempfile::tempdir().unwrap();
        let outside_dir = tempfile::tempdir().unwrap();
        let target = outside_dir.path().join("evil.sh");
        std::fs::write(&target, "#!/bin/bash").unwrap();

        let policy = SandboxPolicy::default();
        let result = validate_path(&policy, worktree_dir.path(), &target);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("outside worktree root"));
    }

    #[test]
    fn unsafe_mode_allows_outside_path() {
        let worktree_dir = tempfile::tempdir().unwrap();
        let outside_dir = tempfile::tempdir().unwrap();
        let target = outside_dir.path().join("allowed.txt");
        std::fs::write(&target, "data").unwrap();

        let policy = SandboxPolicy {
            allow_writes_outside_worktree: false,
            unsafe_mode: true,
        };
        assert!(validate_path(&policy, worktree_dir.path(), &target).is_ok());
    }

    #[test]
    fn allow_writes_outside_flag() {
        let worktree_dir = tempfile::tempdir().unwrap();
        let outside_dir = tempfile::tempdir().unwrap();
        let target = outside_dir.path().join("allowed.txt");
        std::fs::write(&target, "data").unwrap();

        let policy = SandboxPolicy {
            allow_writes_outside_worktree: true,
            unsafe_mode: false,
        };
        assert!(validate_path(&policy, worktree_dir.path(), &target).is_ok());
    }

    #[test]
    fn allows_nonexistent_file_inside_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let worktree = tmp.path();
        std::fs::create_dir_all(worktree.join("src")).unwrap();
        let target = worktree.join("src").join("new_file.rs");

        let policy = SandboxPolicy::default();
        assert!(validate_path(&policy, worktree, &target).is_ok());
    }

    #[test]
    fn default_policy_is_restrictive() {
        let policy = SandboxPolicy::default();
        assert!(!policy.allow_writes_outside_worktree);
        assert!(!policy.unsafe_mode);
    }
}
