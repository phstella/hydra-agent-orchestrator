use std::path::{Path, PathBuf};

/// Sandbox enforcement mode for agent processes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxMode {
    /// Agent writes are confined to its assigned worktree.
    Strict,
    /// Agent has unrestricted filesystem access (requires explicit opt-in).
    Unsafe,
}

/// Policy for sandboxing agent worktree writes.
pub struct SandboxPolicy {
    mode: SandboxMode,
    allowed_root: PathBuf,
}

impl SandboxPolicy {
    pub fn strict(worktree_root: PathBuf) -> Self {
        Self {
            mode: SandboxMode::Strict,
            allowed_root: worktree_root,
        }
    }

    pub fn unsafe_mode(worktree_root: PathBuf) -> Self {
        tracing::warn!(
            worktree = %worktree_root.display(),
            "Unsafe sandbox mode enabled — agent can write outside worktree"
        );
        Self {
            mode: SandboxMode::Unsafe,
            allowed_root: worktree_root,
        }
    }

    pub fn mode(&self) -> &SandboxMode {
        &self.mode
    }

    pub fn allowed_root(&self) -> &Path {
        &self.allowed_root
    }

    /// Check whether a given path is allowed under this policy.
    /// In strict mode, the path must be under the allowed root.
    /// In unsafe mode, all paths are allowed.
    pub fn check_path(&self, target: &Path) -> SandboxResult {
        if self.mode == SandboxMode::Unsafe {
            return SandboxResult::Allowed;
        }

        let root = self
            .allowed_root
            .canonicalize()
            .unwrap_or_else(|_| normalize_abs_path(&self.allowed_root));

        let resolved_target = target
            .canonicalize()
            .unwrap_or_else(|_| normalize_abs_path(target));

        if resolved_target.starts_with(&root) {
            SandboxResult::Allowed
        } else {
            SandboxResult::Blocked {
                path: target.to_path_buf(),
                allowed_root: self.allowed_root.clone(),
            }
        }
    }
}

fn normalize_abs_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };

    let mut normalized = PathBuf::new();
    let is_absolute = absolute.is_absolute();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() && !is_absolute {
                    normalized.push("..");
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

#[derive(Debug, PartialEq, Eq)]
pub enum SandboxResult {
    Allowed,
    Blocked {
        path: PathBuf,
        allowed_root: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn strict_allows_path_under_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let child = root.join("src/main.rs");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(&child, "fn main() {}").unwrap();

        let policy = SandboxPolicy::strict(root);
        assert_eq!(policy.check_path(&child), SandboxResult::Allowed);
    }

    #[test]
    fn strict_blocks_path_outside_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("worktree");
        std::fs::create_dir_all(&root).unwrap();

        let outside = tmp.path().join("other/secret.txt");
        std::fs::create_dir_all(tmp.path().join("other")).unwrap();
        std::fs::write(&outside, "secret").unwrap();

        let policy = SandboxPolicy::strict(root.clone());
        assert!(matches!(
            policy.check_path(&outside),
            SandboxResult::Blocked { .. }
        ));
    }

    #[test]
    fn unsafe_allows_any_path() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("worktree");
        std::fs::create_dir_all(&root).unwrap();

        let outside = tmp.path().join("elsewhere/file.txt");
        std::fs::create_dir_all(tmp.path().join("elsewhere")).unwrap();
        std::fs::write(&outside, "data").unwrap();

        let policy = SandboxPolicy::unsafe_mode(root);
        assert_eq!(policy.check_path(&outside), SandboxResult::Allowed);
    }

    #[test]
    fn strict_mode_identifier() {
        let policy = SandboxPolicy::strict(PathBuf::from("/tmp/test"));
        assert_eq!(*policy.mode(), SandboxMode::Strict);
    }

    #[test]
    fn unsafe_mode_identifier() {
        let policy = SandboxPolicy::unsafe_mode(PathBuf::from("/tmp/test"));
        assert_eq!(*policy.mode(), SandboxMode::Unsafe);
    }

    #[test]
    fn nonexistent_path_under_root_allowed() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let nonexistent = root.join("does/not/exist.rs");

        let policy = SandboxPolicy::strict(root);
        assert_eq!(policy.check_path(&nonexistent), SandboxResult::Allowed);
    }

    #[test]
    fn nonexistent_path_outside_root_blocked() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("worktree");
        std::fs::create_dir_all(&root).unwrap();

        let outside = PathBuf::from("/somewhere/else/file.txt");

        let policy = SandboxPolicy::strict(root);
        assert!(matches!(
            policy.check_path(&outside),
            SandboxResult::Blocked { .. }
        ));
    }

    #[test]
    fn nonexistent_path_with_similar_prefix_is_blocked() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("worktree");
        std::fs::create_dir_all(&root).unwrap();

        let lookalike = tmp.path().join("worktree-evil/file.txt");
        let policy = SandboxPolicy::strict(root);

        assert!(matches!(
            policy.check_path(&lookalike),
            SandboxResult::Blocked { .. }
        ));
    }

    #[test]
    fn path_traversal_with_dotdot_is_blocked() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("worktree");
        std::fs::create_dir_all(&root).unwrap();

        let escaped = root.join("../outside/file.txt");
        let policy = SandboxPolicy::strict(root);

        assert!(matches!(
            policy.check_path(&escaped),
            SandboxResult::Blocked { .. }
        ));
    }
}
