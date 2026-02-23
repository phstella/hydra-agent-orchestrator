use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use hydra_core::adapter::{AdapterRegistry, ProbeReport, ProbeRunner};
use hydra_core::config::HydraConfig;

use crate::ipc_types::{AgentStreamEvent, RaceResult};

const EVENT_CHANNEL_CAPACITY: usize = 4096;
const MAX_STORED_EVENTS_PER_RUN: usize = 10_000;

#[derive(Debug, Clone)]
pub struct RaceRuntime {
    pub status: String,
    pub events: Vec<AgentStreamEvent>,
    pub result: Option<RaceResult>,
    pub error: Option<String>,
}

impl RaceRuntime {
    fn running() -> Self {
        Self {
            status: "running".to_string(),
            events: Vec::new(),
            result: None,
            error: None,
        }
    }
}

#[derive(Clone)]
pub struct AppStateHandle {
    pub races: Arc<Mutex<HashMap<String, RaceRuntime>>>,
    pub event_tx: broadcast::Sender<AgentStreamEvent>,
}

impl AppStateHandle {
    pub async fn register_race(&self, run_id: &str) {
        let mut races = self.races.lock().await;
        races.insert(run_id.to_string(), RaceRuntime::running());
    }

    pub async fn append_event(&self, run_id: &str, event: AgentStreamEvent) {
        let mut races = self.races.lock().await;
        if let Some(race) = races.get_mut(run_id) {
            race.events.push(event.clone());
            if race.events.len() > MAX_STORED_EVENTS_PER_RUN {
                let overflow = race.events.len() - MAX_STORED_EVENTS_PER_RUN;
                race.events.drain(0..overflow);
            }
        }
        let _ = self.event_tx.send(event);
    }

    pub async fn mark_completed(&self, run_id: &str, result: RaceResult) {
        let mut races = self.races.lock().await;
        if let Some(race) = races.get_mut(run_id) {
            race.status = "completed".to_string();
            race.result = Some(result);
            race.error = None;
        }
    }

    pub async fn mark_failed(&self, run_id: &str, error: impl Into<String>) {
        let mut races = self.races.lock().await;
        let error = error.into();
        let entry = races
            .entry(run_id.to_string())
            .or_insert_with(RaceRuntime::running);
        entry.status = "failed".to_string();
        entry.error = Some(error.clone());
        if entry.result.is_none() {
            entry.result = Some(RaceResult {
                run_id: run_id.to_string(),
                status: "failed".to_string(),
                agents: Vec::new(),
                duration_ms: None,
                total_cost: None,
            });
        }
    }

    pub async fn race_result(&self, run_id: &str) -> Option<RaceResult> {
        let races = self.races.lock().await;
        races.get(run_id).and_then(|r| r.result.clone())
    }

    pub async fn poll_events(
        &self,
        run_id: &str,
        cursor: usize,
        max_batch_size: usize,
    ) -> Option<(Vec<AgentStreamEvent>, usize, bool, String, Option<String>)> {
        let races = self.races.lock().await;
        let race = races.get(run_id)?;
        let start = cursor.min(race.events.len());
        let end = (start + max_batch_size).min(race.events.len());
        let batch = race.events[start..end].to_vec();
        let done = race.status != "running";
        Some((batch, end, done, race.status.clone(), race.error.clone()))
    }
}

pub struct AppState {
    pub config: Arc<Mutex<HydraConfig>>,
    pub last_probe_report: Arc<Mutex<Option<ProbeReport>>>,
    pub races: Arc<Mutex<HashMap<String, RaceRuntime>>>,
    pub event_tx: broadcast::Sender<AgentStreamEvent>,
}

impl AppState {
    pub fn new(config: HydraConfig) -> Self {
        let (event_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self {
            config: Arc::new(Mutex::new(config)),
            last_probe_report: Arc::new(Mutex::new(None)),
            races: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
        }
    }

    pub fn handle(&self) -> AppStateHandle {
        AppStateHandle {
            races: Arc::clone(&self.races),
            event_tx: self.event_tx.clone(),
        }
    }

    pub async fn run_probes(&self) -> ProbeReport {
        let config = self.config.lock().await;
        let registry = AdapterRegistry::from_config(&config.adapters);

        let adapters: Vec<Box<dyn hydra_core::adapter::AgentAdapter>> = registry
            .known_keys()
            .into_iter()
            .filter_map(|key| {
                registry.resolve(key, true).ok().map(
                    |arc| -> Box<dyn hydra_core::adapter::AgentAdapter> {
                        Box::new(ArcAdapterWrapper(arc))
                    },
                )
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
