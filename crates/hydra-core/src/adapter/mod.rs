pub mod claude;
pub mod codex;
pub mod cursor;
mod error;
mod probe;
mod types;

pub use error::AdapterError;
pub use probe::{ProbeReport, ProbeRunner};
pub use types::{
    AdapterTier, AgentEvent, BuiltCommand, CapabilityConfidence, CapabilitySet, DetectResult,
    DetectStatus, ProbeResult, SpawnRequest,
};

use std::path::PathBuf;

/// Core trait that every agent adapter must implement.
///
/// Phase 0 focuses on `detect()` and `capabilities()`.
/// `build_command()`, `parse_line()`, and `parse_raw()` are wired in Phase 1.
pub trait AgentAdapter: Send + Sync {
    fn key(&self) -> &'static str;
    fn tier(&self) -> AdapterTier;
    fn detect(&self) -> DetectResult;
    fn capabilities(&self) -> CapabilitySet;

    fn build_command(&self, _req: &SpawnRequest) -> Result<BuiltCommand, AdapterError> {
        Err(AdapterError::NotImplemented {
            adapter: self.key().to_string(),
            feature: "build_command".to_string(),
        })
    }

    fn parse_line(&self, _line: &str) -> Option<AgentEvent> {
        None
    }

    fn parse_raw(&self, _chunk: &[u8]) -> Vec<AgentEvent> {
        Vec::new()
    }
}

/// Resolve binary by checking configured path, then `$PATH` candidates.
///
/// If a configured path is provided but doesn't exist, returns `None`
/// without falling back to PATH discovery (explicit config takes precedence).
pub fn resolve_binary(configured: Option<&str>, candidates: &[&str]) -> Option<PathBuf> {
    if let Some(path) = configured {
        let p = PathBuf::from(path);
        return if p.exists() { Some(p) } else { None };
    }
    for name in candidates {
        if let Ok(p) = which::which(name) {
            return Some(p);
        }
    }
    None
}
