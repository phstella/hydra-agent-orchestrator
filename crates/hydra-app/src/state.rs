use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};

use hydra_core::adapter::{AdapterRegistry, ProbeReport, ProbeRunner};
use hydra_core::config::HydraConfig;
use hydra_core::supervisor::pty::{PtyEvent, PtySession};

use crate::ipc_types::{AgentStreamEvent, InteractiveStreamEvent, RaceResult};

const EVENT_CHANNEL_CAPACITY: usize = 4096;
const MAX_STORED_EVENTS_PER_RUN: usize = 10_000;
const MAX_INTERACTIVE_EVENTS: usize = 50_000;

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

// ---------------------------------------------------------------------------
// Interactive session runtime (M4.2)
// ---------------------------------------------------------------------------

pub struct InteractiveSessionRuntime {
    pub session_id: String,
    pub agent_key: String,
    pub status: String,
    pub started_at: String,
    pub events: Vec<InteractiveStreamEvent>,
    pub error: Option<String>,
    pub pty_session: Option<PtySession>,
}

impl InteractiveSessionRuntime {
    fn new(session_id: String, agent_key: String, started_at: String) -> Self {
        Self {
            session_id,
            agent_key,
            status: "running".to_string(),
            started_at,
            events: Vec::new(),
            error: None,
            pty_session: None,
        }
    }
}

#[derive(Clone)]
pub struct InteractiveStateHandle {
    pub sessions: Arc<Mutex<HashMap<String, InteractiveSessionRuntime>>>,
}

impl InteractiveStateHandle {
    pub async fn register_session(
        &self,
        session_id: &str,
        agent_key: &str,
        started_at: &str,
        pty_session: PtySession,
    ) {
        let mut sessions = self.sessions.lock().await;
        let mut runtime = InteractiveSessionRuntime::new(
            session_id.to_string(),
            agent_key.to_string(),
            started_at.to_string(),
        );
        runtime.pty_session = Some(pty_session);
        sessions.insert(session_id.to_string(), runtime);
    }

    pub async fn append_event(&self, session_id: &str, event: InteractiveStreamEvent) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.events.push(event);
            if session.events.len() > MAX_INTERACTIVE_EVENTS {
                let overflow = session.events.len() - MAX_INTERACTIVE_EVENTS;
                session.events.drain(0..overflow);
            }
        }
    }

    pub async fn poll_events(
        &self,
        session_id: &str,
        cursor: usize,
        max_batch: usize,
    ) -> Option<(
        Vec<InteractiveStreamEvent>,
        usize,
        bool,
        String,
        Option<String>,
    )> {
        let sessions = self.sessions.lock().await;
        let session = sessions.get(session_id)?;
        let start = cursor.min(session.events.len());
        let end = (start + max_batch).min(session.events.len());
        let batch = session.events[start..end].to_vec();
        let done = session.status != "running";
        Some((
            batch,
            end,
            done,
            session.status.clone(),
            session.error.clone(),
        ))
    }

    pub async fn mark_completed(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = "completed".to_string();
        }
    }

    pub async fn mark_failed(&self, session_id: &str, error: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = "failed".to_string();
            session.error = Some(error.to_string());
        }
    }

    pub async fn mark_stopped(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = "stopped".to_string();
        }
    }

    pub async fn get_status(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.lock().await;
        sessions.get(session_id).map(|s| s.status.clone())
    }

    pub async fn write_input(&self, session_id: &str, data: &[u8]) -> Result<(), String> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| "session not found".to_string())?;
        if session.status != "running" {
            return Err(format!("session is {}, not running", session.status));
        }
        let pty = session
            .pty_session
            .as_ref()
            .ok_or_else(|| "PTY not available".to_string())?;
        pty.write_input(data)
            .await
            .map_err(|e| format!("write failed: {e}"))
    }

    pub async fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| "session not found".to_string())?;
        if session.status != "running" {
            return Err(format!("session is {}, not running", session.status));
        }
        let pty = session
            .pty_session
            .as_ref()
            .ok_or_else(|| "PTY not available".to_string())?;
        pty.resize(cols, rows)
            .await
            .map_err(|e| format!("resize failed: {e}"))
    }

    /// Stop a session. Returns (was_running, current_status). Idempotent.
    pub async fn stop_session(&self, session_id: &str) -> Result<(bool, String), String> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| "session not found".to_string())?;

        let was_running = session.status == "running";
        if was_running {
            if let Some(ref pty) = session.pty_session {
                pty.stop().await;
            }
            session.status = "stopped".to_string();
        }
        let status = session.status.clone();
        Ok((was_running, status))
    }

    pub async fn list_sessions(&self) -> Vec<(String, String, String, String, usize)> {
        let sessions = self.sessions.lock().await;
        sessions
            .values()
            .map(|s| {
                (
                    s.session_id.clone(),
                    s.agent_key.clone(),
                    s.status.clone(),
                    s.started_at.clone(),
                    s.events.len(),
                )
            })
            .collect()
    }

    /// Shutdown all running sessions. Called on app exit.
    pub async fn shutdown_all(&self) {
        let mut sessions = self.sessions.lock().await;
        for session in sessions.values_mut() {
            if session.status == "running" {
                if let Some(ref pty) = session.pty_session {
                    pty.stop().await;
                }
                session.status = "stopped".to_string();
            }
        }
    }
}

/// Spawn a background task that reads PTY events and forwards them to the session registry.
pub fn spawn_pty_event_bridge(
    session_id: String,
    agent_key: String,
    mut event_rx: mpsc::Receiver<PtyEvent>,
    state: InteractiveStateHandle,
) {
    tokio::spawn(async move {
        while let Some(evt) = event_rx.recv().await {
            let now = chrono::Utc::now().to_rfc3339();
            let (event_type, data) = match &evt {
                PtyEvent::Started => ("session_started".to_string(), serde_json::json!({})),
                PtyEvent::Output(bytes) => {
                    let text = String::from_utf8_lossy(bytes);
                    ("output".to_string(), serde_json::json!({ "text": text }))
                }
                PtyEvent::Completed {
                    exit_code,
                    duration,
                } => {
                    state.mark_completed(&session_id).await;
                    (
                        "session_completed".to_string(),
                        serde_json::json!({
                            "exitCode": exit_code,
                            "durationMs": duration.as_millis() as u64,
                        }),
                    )
                }
                PtyEvent::Failed { error, duration } => {
                    state.mark_failed(&session_id, error).await;
                    (
                        "session_failed".to_string(),
                        serde_json::json!({
                            "error": error,
                            "durationMs": duration.as_millis() as u64,
                        }),
                    )
                }
                PtyEvent::Stopped { duration } => {
                    state.mark_stopped(&session_id).await;
                    (
                        "session_stopped".to_string(),
                        serde_json::json!({
                            "durationMs": duration.as_millis() as u64,
                        }),
                    )
                }
            };

            let stream_event = InteractiveStreamEvent {
                session_id: session_id.clone(),
                agent_key: agent_key.clone(),
                event_type,
                data,
                timestamp: now,
            };
            state.append_event(&session_id, stream_event).await;
        }
    });
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
    pub interactive: InteractiveStateHandle,
}

impl AppState {
    pub fn new(config: HydraConfig) -> Self {
        let (event_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self {
            config: Arc::new(Mutex::new(config)),
            last_probe_report: Arc::new(Mutex::new(None)),
            races: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            interactive: InteractiveStateHandle {
                sessions: Arc::new(Mutex::new(HashMap::new())),
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_core::supervisor::pty::PtySessionConfig;
    use std::time::Duration;

    fn echo_pty_config(msg: &str) -> PtySessionConfig {
        PtySessionConfig {
            program: "echo".to_string(),
            args: vec![msg.to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
            initial_cols: 80,
            initial_rows: 24,
        }
    }

    fn sleep_pty_config(secs: u64) -> PtySessionConfig {
        PtySessionConfig {
            program: "sleep".to_string(),
            args: vec![secs.to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
            initial_cols: 80,
            initial_rows: 24,
        }
    }

    fn cat_pty_config() -> PtySessionConfig {
        PtySessionConfig {
            program: "cat".to_string(),
            args: vec![],
            env: vec![],
            cwd: std::env::temp_dir(),
            initial_cols: 80,
            initial_rows: 24,
        }
    }

    fn new_interactive_state() -> InteractiveStateHandle {
        InteractiveStateHandle {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[tokio::test]
    async fn interactive_start_stream_input_resize_stop() {
        let state = new_interactive_state();

        let (tx, rx) = mpsc::channel(256);
        let session = PtySession::spawn(cat_pty_config(), tx).unwrap();
        state
            .register_session("s1", "claude", "2026-02-24T00:00:00Z", session)
            .await;

        spawn_pty_event_bridge("s1".to_string(), "claude".to_string(), rx, state.clone());

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Write input
        state.write_input("s1", b"hello\n").await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Resize
        state.resize("s1", 132, 50).await.unwrap();

        // Poll events
        let (events, _next, _done, status, _err) = state.poll_events("s1", 0, 512).await.unwrap();
        assert!(!events.is_empty(), "should have events after input");
        assert_eq!(status, "running");

        // Stop
        let (was_running, final_status) = state.stop_session("s1").await.unwrap();
        assert!(was_running);
        assert_eq!(final_status, "stopped");
    }

    #[tokio::test]
    async fn interactive_invalid_session_id_returns_error() {
        let state = new_interactive_state();

        assert!(state.poll_events("nonexistent", 0, 100).await.is_none());
        assert!(state.write_input("nonexistent", b"data").await.is_err());
        assert!(state.resize("nonexistent", 80, 24).await.is_err());
        assert!(state.stop_session("nonexistent").await.is_err());
    }

    #[tokio::test]
    async fn interactive_write_after_stop_returns_error() {
        let state = new_interactive_state();

        let (tx, rx) = mpsc::channel(256);
        let session = PtySession::spawn(sleep_pty_config(60), tx).unwrap();
        state
            .register_session("s2", "codex", "2026-02-24T00:00:00Z", session)
            .await;

        spawn_pty_event_bridge("s2".to_string(), "codex".to_string(), rx, state.clone());
        tokio::time::sleep(Duration::from_millis(200)).await;

        state.stop_session("s2").await.unwrap();

        let result = state.write_input("s2", b"late write").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not running"));
    }

    #[tokio::test]
    async fn interactive_resize_after_stop_returns_error() {
        let state = new_interactive_state();

        let (tx, rx) = mpsc::channel(256);
        let session = PtySession::spawn(sleep_pty_config(60), tx).unwrap();
        state
            .register_session("s3", "claude", "2026-02-24T00:00:00Z", session)
            .await;

        spawn_pty_event_bridge("s3".to_string(), "claude".to_string(), rx, state.clone());
        tokio::time::sleep(Duration::from_millis(200)).await;

        state.stop_session("s3").await.unwrap();

        let result = state.resize("s3", 80, 24).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not running"));
    }

    #[tokio::test]
    async fn interactive_idempotent_stop() {
        let state = new_interactive_state();

        let (tx, rx) = mpsc::channel(256);
        let session = PtySession::spawn(sleep_pty_config(60), tx).unwrap();
        state
            .register_session("s4", "claude", "2026-02-24T00:00:00Z", session)
            .await;

        spawn_pty_event_bridge("s4".to_string(), "claude".to_string(), rx, state.clone());
        tokio::time::sleep(Duration::from_millis(200)).await;

        let (was_running_1, _) = state.stop_session("s4").await.unwrap();
        assert!(was_running_1);

        let (was_running_2, status_2) = state.stop_session("s4").await.unwrap();
        assert!(
            !was_running_2,
            "second stop should report was_running=false"
        );
        assert_eq!(status_2, "stopped");
    }

    #[tokio::test]
    async fn interactive_multiple_sessions_isolated() {
        let state = new_interactive_state();

        let (tx1, rx1) = mpsc::channel(256);
        let s1 = PtySession::spawn(sleep_pty_config(60), tx1).unwrap();
        state
            .register_session("sa", "claude", "2026-02-24T00:00:00Z", s1)
            .await;
        spawn_pty_event_bridge("sa".to_string(), "claude".to_string(), rx1, state.clone());

        let (tx2, rx2) = mpsc::channel(256);
        let s2 = PtySession::spawn(sleep_pty_config(60), tx2).unwrap();
        state
            .register_session("sb", "codex", "2026-02-24T00:00:01Z", s2)
            .await;
        spawn_pty_event_bridge("sb".to_string(), "codex".to_string(), rx2, state.clone());

        tokio::time::sleep(Duration::from_millis(200)).await;

        let list = state.list_sessions().await;
        assert_eq!(list.len(), 2);

        // Stop one, other remains running
        state.stop_session("sa").await.unwrap();
        assert_eq!(state.get_status("sa").await, Some("stopped".to_string()));
        assert_eq!(state.get_status("sb").await, Some("running".to_string()));

        state.stop_session("sb").await.unwrap();
    }

    #[tokio::test]
    async fn interactive_shutdown_all_stops_running_sessions() {
        let state = new_interactive_state();

        let (tx1, rx1) = mpsc::channel(256);
        let s1 = PtySession::spawn(sleep_pty_config(60), tx1).unwrap();
        state
            .register_session("x1", "claude", "2026-02-24T00:00:00Z", s1)
            .await;
        spawn_pty_event_bridge("x1".to_string(), "claude".to_string(), rx1, state.clone());

        let (tx2, rx2) = mpsc::channel(256);
        let s2 = PtySession::spawn(sleep_pty_config(60), tx2).unwrap();
        state
            .register_session("x2", "codex", "2026-02-24T00:00:01Z", s2)
            .await;
        spawn_pty_event_bridge("x2".to_string(), "codex".to_string(), rx2, state.clone());

        tokio::time::sleep(Duration::from_millis(200)).await;

        state.shutdown_all().await;

        assert_eq!(state.get_status("x1").await, Some("stopped".to_string()));
        assert_eq!(state.get_status("x2").await, Some("stopped".to_string()));
    }

    #[tokio::test]
    async fn interactive_event_bridge_populates_events() {
        let state = new_interactive_state();

        let (tx, rx) = mpsc::channel(256);
        let session = PtySession::spawn(echo_pty_config("bridge-test"), tx).unwrap();
        state
            .register_session("eb", "claude", "2026-02-24T00:00:00Z", session)
            .await;

        spawn_pty_event_bridge("eb".to_string(), "claude".to_string(), rx, state.clone());

        // Wait for process to complete and bridge to flush
        tokio::time::sleep(Duration::from_millis(1000)).await;

        let (events, _, _, _, _) = state.poll_events("eb", 0, 1000).await.unwrap();
        assert!(!events.is_empty(), "bridge should have forwarded events");

        let has_output = events.iter().any(|e| e.event_type == "output");
        assert!(has_output, "should have output events from echo");
    }

    #[tokio::test]
    async fn interactive_session_request_serde_roundtrip() {
        let json = r#"{"agentKey":"claude","taskPrompt":"fix bug","allowExperimental":false,"unsafeMode":false,"cwd":null,"cols":120,"rows":40}"#;
        let req: crate::ipc_types::InteractiveSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.agent_key, "claude");
        assert_eq!(req.task_prompt, "fix bug");
        assert_eq!(req.cols, Some(120));
        assert_eq!(req.rows, Some(40));
        assert!(!req.allow_experimental);
        assert!(!req.unsafe_mode);
        assert!(req.cwd.is_none());
    }

    #[tokio::test]
    async fn interactive_event_batch_serde_roundtrip() {
        let batch = crate::ipc_types::InteractiveEventBatch {
            session_id: "s1".to_string(),
            events: vec![crate::ipc_types::InteractiveStreamEvent {
                session_id: "s1".to_string(),
                agent_key: "claude".to_string(),
                event_type: "output".to_string(),
                data: serde_json::json!({ "text": "hello" }),
                timestamp: "2026-02-24T00:00:00Z".to_string(),
            }],
            next_cursor: 1,
            done: false,
            status: "running".to_string(),
            error: None,
        };
        let json = serde_json::to_string(&batch).unwrap();
        let back: crate::ipc_types::InteractiveEventBatch = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, "s1");
        assert_eq!(back.events.len(), 1);
        assert_eq!(back.next_cursor, 1);
        assert!(!back.done);
    }

    #[tokio::test]
    async fn interactive_poll_after_completion_returns_done() {
        let state = new_interactive_state();

        let (tx, rx) = mpsc::channel(256);
        let session = PtySession::spawn(echo_pty_config("done-test"), tx).unwrap();
        state
            .register_session("dc", "claude", "2026-02-24T00:00:00Z", session)
            .await;

        spawn_pty_event_bridge("dc".to_string(), "claude".to_string(), rx, state.clone());

        // Wait for echo to complete
        tokio::time::sleep(Duration::from_millis(1000)).await;

        let (_, _, done, status, _) = state.poll_events("dc", 0, 1000).await.unwrap();
        assert!(done, "poll after process exit should report done=true");
        assert_ne!(status, "running");
    }
}
