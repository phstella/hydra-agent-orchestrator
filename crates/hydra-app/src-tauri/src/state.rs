use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use hydra_core::config::HydraConfig;

/// Shared application state accessible from Tauri commands.
pub struct AppState {
    pub config: HydraConfig,
    pub repo_root: Arc<Mutex<Option<PathBuf>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let config = HydraConfig::load_or_default();
        Self {
            config,
            repo_root: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the repo root, defaulting to current directory.
    pub async fn repo_root(&self) -> PathBuf {
        let guard = self.repo_root.lock().await;
        guard
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}
