use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("artifact I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("artifact serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("run directory already exists: {path}")]
    RunAlreadyExists { path: String },

    #[error("run directory not found: {path}")]
    RunNotFound { path: String },

    #[error("manifest not found at {path}")]
    ManifestNotFound { path: String },
}
