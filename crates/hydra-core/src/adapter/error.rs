use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("binary not found for adapter '{adapter}'")]
    BinaryMissing { adapter: String },

    #[error("authentication missing for adapter '{adapter}': {detail}")]
    AuthMissing { adapter: String, detail: String },

    #[error("unsupported version for adapter '{adapter}': got {version}")]
    UnsupportedVersion { adapter: String, version: String },

    #[error("unsupported flag '{flag}' for adapter '{adapter}'")]
    UnsupportedFlag { adapter: String, flag: String },

    #[error("spawn failed for adapter '{adapter}': {detail}")]
    SpawnFailed { adapter: String, detail: String },

    #[error("stream parse error for adapter '{adapter}': {detail}")]
    StreamParseError { adapter: String, detail: String },

    #[error("adapter '{adapter}' timed out after {seconds}s")]
    TimedOut { adapter: String, seconds: u64 },

    #[error("adapter '{adapter}' was interrupted")]
    Interrupted { adapter: String },

    #[error("feature '{feature}' not yet implemented for adapter '{adapter}'")]
    NotImplemented { adapter: String, feature: String },

    #[error("probe failed for adapter '{adapter}': {detail}")]
    ProbeFailed { adapter: String, detail: String },
}
