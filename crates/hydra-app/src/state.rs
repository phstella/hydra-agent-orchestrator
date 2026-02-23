use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use hydra_core::adapter::{AdapterRegistry, ProbeReport, ProbeRunner};
use hydra_core::config::HydraConfig;

use crate::ipc_types::AgentStreamEvent;

const EVENT_CHANNEL_CAPACITY: usize = 4096;

pub struct AppState {
    pub config: Arc<Mutex<HydraConfig>>,
    pub last_probe_report: Arc<Mutex<Option<ProbeReport>>>,
    pub event_tx: broadcast::Sender<AgentStreamEvent>,
}

impl AppState {
    pub fn new(config: HydraConfig) -> Self {
        let (event_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self {
            config: Arc::new(Mutex::new(config)),
            last_probe_report: Arc::new(Mutex::new(None)),
            event_tx,
        }
    }

    pub async fn run_probes(&self) -> ProbeReport {
        let config = self.config.lock().await;
        let registry = AdapterRegistry::from_config(&config.adapters);

        let adapters: Vec<Box<dyn hydra_core::adapter::AgentAdapter>> = registry
            .known_keys()
            .into_iter()
            .filter_map(|key| {
                registry
                    .resolve(key, true)
                    .ok()
                    .map(|arc| -> Box<dyn hydra_core::adapter::AgentAdapter> {
                        Box::new(ArcAdapterWrapper(arc))
                    })
            })
            .collect();

        let runner = ProbeRunner::new(adapters);
        let report = runner.run();

        *self.last_probe_report.lock().await = Some(report.clone());
        report
    }
}

/// Wraps an `Arc<dyn AgentAdapter>` to satisfy `ProbeRunner`'s `Box<dyn AgentAdapter>` requirement.
struct ArcAdapterWrapper(Arc<dyn hydra_core::adapter::AgentAdapter>);

impl hydra_core::adapter::AgentAdapter for ArcAdapterWrapper {
    fn key(&self) -> &'static str {
        self.0.key()
    }

    fn tier(&self) -> hydra_core::adapter::AdapterTier {
        self.0.tier()
    }

    fn detect(&self) -> hydra_core::adapter::DetectResult {
        self.0.detect()
    }

    fn capabilities(&self) -> hydra_core::adapter::CapabilitySet {
        self.0.capabilities()
    }
}
