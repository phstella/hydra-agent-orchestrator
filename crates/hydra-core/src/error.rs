use thiserror::Error;

/// Top-level error type for the hydra-core library.
#[derive(Debug, Error)]
pub enum HydraError {
    /// Configuration parse or validation error.
    #[error("config error: {0}")]
    Config(String),

    /// Adapter probe or execution error.
    #[error("adapter error: {0}")]
    Adapter(String),

    /// Git worktree lifecycle error.
    #[error("worktree error: {0}")]
    Worktree(String),

    /// Agent process supervision error.
    #[error("process error: {0}")]
    Process(String),

    /// Scoring engine error.
    #[error("scoring error: {0}")]
    Scoring(String),

    /// Artifact read/write error.
    #[error("artifact error: {0}")]
    Artifact(String),

    /// Wraps `std::io::Error`.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Git command error.
    #[error("git error: {0}")]
    Git(String),
}

/// Convenience alias used throughout the library.
pub type Result<T> = std::result::Result<T, HydraError>;
