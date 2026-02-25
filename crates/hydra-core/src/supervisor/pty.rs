use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use thiserror::Error;
use tokio::sync::{mpsc, Mutex, Notify};

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("failed to open PTY pair: {0}")]
    OpenFailed(String),

    #[error("failed to spawn process in PTY: {0}")]
    SpawnFailed(String),

    #[error("session has already been stopped")]
    AlreadyStopped,

    #[error("write to PTY failed: {0}")]
    WriteFailed(String),

    #[error("resize failed: {0}")]
    ResizeFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtySessionStatus {
    Running,
    Completed { exit_code: Option<u32> },
    Failed { error: String },
    Stopped,
}

#[derive(Debug, Clone)]
pub enum PtyEvent {
    Started,
    Output(Vec<u8>),
    Completed {
        exit_code: Option<u32>,
        duration: Duration,
    },
    Failed {
        error: String,
        duration: Duration,
    },
    Stopped {
        duration: Duration,
    },
}

pub struct PtySessionConfig {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
    pub initial_cols: u16,
    pub initial_rows: u16,
}

impl Default for PtySessionConfig {
    fn default() -> Self {
        Self {
            program: String::new(),
            args: Vec::new(),
            env: Vec::new(),
            cwd: PathBuf::from("."),
            initial_cols: 120,
            initial_rows: 40,
        }
    }
}

struct PtyInner {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send>,
}

const CHILD_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const CHILD_EXIT_POLL_ATTEMPTS: usize = 20;

pub struct PtySession {
    inner: Arc<Mutex<Option<PtyInner>>>,
    status: Arc<Mutex<PtySessionStatus>>,
    stop_notify: Arc<Notify>,
}

impl PtySession {
    /// Spawn a PTY-backed process and begin streaming output via `event_tx`.
    pub fn spawn(
        config: PtySessionConfig,
        event_tx: mpsc::Sender<PtyEvent>,
    ) -> Result<Self, PtyError> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: config.initial_rows,
                cols: config.initial_cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::OpenFailed(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&config.program);
        cmd.args(&config.args);
        cmd.cwd(&config.cwd);

        for (key, val) in &config.env {
            cmd.env(key, val);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::SpawnFailed(e.to_string()))?;

        drop(pair.slave);

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Io(std::io::Error::other(e.to_string())))?;

        let inner = PtyInner {
            master: pair.master,
            writer,
            child,
        };

        let session = Self {
            inner: Arc::new(Mutex::new(Some(inner))),
            status: Arc::new(Mutex::new(PtySessionStatus::Running)),
            stop_notify: Arc::new(Notify::new()),
        };

        let inner_ref = Arc::clone(&session.inner);
        let status_ref = Arc::clone(&session.status);
        let stop_notify_ref = Arc::clone(&session.stop_notify);

        let _ = event_tx.try_send(PtyEvent::Started);

        tokio::spawn(async move {
            Self::run_loop(inner_ref, status_ref, stop_notify_ref, event_tx).await;
        });

        Ok(session)
    }

    pub async fn write_input(&self, data: &[u8]) -> Result<(), PtyError> {
        let mut guard = self.inner.lock().await;
        let inner = guard.as_mut().ok_or(PtyError::AlreadyStopped)?;
        inner
            .writer
            .write_all(data)
            .map_err(|e| PtyError::WriteFailed(e.to_string()))?;
        inner
            .writer
            .flush()
            .map_err(|e| PtyError::WriteFailed(e.to_string()))?;
        Ok(())
    }

    pub async fn resize(&self, cols: u16, rows: u16) -> Result<(), PtyError> {
        let guard = self.inner.lock().await;
        let inner = guard.as_ref().ok_or(PtyError::AlreadyStopped)?;
        inner
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::ResizeFailed(e.to_string()))
    }

    /// Request graceful termination. Idempotent.
    pub async fn stop(&self) {
        self.stop_notify.notify_one();
    }

    pub async fn status(&self) -> PtySessionStatus {
        self.status.lock().await.clone()
    }

    async fn run_loop(
        inner: Arc<Mutex<Option<PtyInner>>>,
        status: Arc<Mutex<PtySessionStatus>>,
        stop_notify: Arc<Notify>,
        event_tx: mpsc::Sender<PtyEvent>,
    ) {
        let start = Instant::now();

        let reader = {
            let guard = inner.lock().await;
            if let Some(i) = guard.as_ref() {
                i.master.try_clone_reader().ok()
            } else {
                None
            }
        };

        let Some(mut reader) = reader else {
            let _ = event_tx
                .send(PtyEvent::Failed {
                    error: "failed to clone PTY reader".to_string(),
                    duration: start.elapsed(),
                })
                .await;
            *status.lock().await = PtySessionStatus::Failed {
                error: "failed to clone PTY reader".to_string(),
            };
            return;
        };

        let (output_tx, mut output_rx) = mpsc::channel::<Result<Vec<u8>, String>>(256);

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if output_tx.blocking_send(Ok(buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = output_tx.blocking_send(Err(e.to_string()));
                        break;
                    }
                }
            }
        });

        let final_status;

        loop {
            tokio::select! {
                biased;
                _ = stop_notify.notified() => {
                    Self::kill_child(&inner).await;
                    let _ = event_tx
                        .send(PtyEvent::Stopped { duration: start.elapsed() })
                        .await;
                    final_status = PtySessionStatus::Stopped;
                    break;
                }
                chunk = output_rx.recv() => {
                    match chunk {
                        Some(Ok(data)) => {
                            let _ = event_tx.send(PtyEvent::Output(data)).await;
                        }
                        Some(Err(err)) => {
                            let _ = event_tx
                                .send(PtyEvent::Failed {
                                    error: err.clone(),
                                    duration: start.elapsed(),
                                })
                                .await;
                            final_status = PtySessionStatus::Failed { error: err };
                            break;
                        }
                        None => {
                            let exit_code = Self::wait_child(&inner).await;
                            let _ = event_tx
                                .send(PtyEvent::Completed {
                                    exit_code,
                                    duration: start.elapsed(),
                                })
                                .await;
                            final_status = PtySessionStatus::Completed { exit_code };
                            break;
                        }
                    }
                }
            }
        }

        *status.lock().await = final_status;
        Self::cleanup(&inner).await;
    }

    async fn kill_child(inner: &Arc<Mutex<Option<PtyInner>>>) {
        let mut guard = inner.lock().await;
        if let Some(ref mut i) = *guard {
            let _ = terminate_child_with_retry(i.child.as_mut());
        }
    }

    async fn wait_child(inner: &Arc<Mutex<Option<PtyInner>>>) -> Option<u32> {
        let mut guard = inner.lock().await;
        if let Some(ref mut i) = *guard {
            match i.child.wait() {
                Ok(status) => Some(status.exit_code()),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    async fn cleanup(inner: &Arc<Mutex<Option<PtyInner>>>) {
        let mut guard = inner.lock().await;
        if let Some(mut i) = guard.take() {
            let _ = terminate_child_with_retry(i.child.as_mut());
        }
    }
}

fn wait_for_child_exit_with_retry(child: &mut (dyn Child + Send), attempts: usize) -> Option<u32> {
    for attempt in 0..attempts {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status.exit_code()),
            Ok(None) => {
                if attempt + 1 < attempts {
                    std::thread::sleep(CHILD_EXIT_POLL_INTERVAL);
                }
            }
            Err(_) => return None,
        }
    }

    None
}

fn terminate_child_with_retry(child: &mut (dyn Child + Send)) -> Option<u32> {
    if let Some(exit_code) = wait_for_child_exit_with_retry(child, 1) {
        return Some(exit_code);
    }

    let _ = child.kill();
    if let Some(exit_code) = wait_for_child_exit_with_retry(child, CHILD_EXIT_POLL_ATTEMPTS) {
        return Some(exit_code);
    }

    // Retry a second hard kill in case the first signal raced with process shutdown.
    let _ = child.kill();
    wait_for_child_exit_with_retry(child, CHILD_EXIT_POLL_ATTEMPTS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_echo_config(msg: &str) -> PtySessionConfig {
        PtySessionConfig {
            #[cfg(unix)]
            program: "echo".to_string(),
            #[cfg(windows)]
            program: "cmd".to_string(),
            #[cfg(unix)]
            args: vec![msg.to_string()],
            #[cfg(windows)]
            args: vec!["/C".to_string(), format!("echo {msg}")],
            env: vec![],
            cwd: std::env::temp_dir(),
            initial_cols: 80,
            initial_rows: 24,
        }
    }

    fn test_cat_config() -> PtySessionConfig {
        PtySessionConfig {
            #[cfg(unix)]
            program: "cat".to_string(),
            #[cfg(windows)]
            program: "cmd".to_string(),
            #[cfg(unix)]
            args: vec![],
            #[cfg(windows)]
            args: vec!["/K".to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
            initial_cols: 80,
            initial_rows: 24,
        }
    }

    fn test_sleep_config(secs: u64) -> PtySessionConfig {
        PtySessionConfig {
            #[cfg(unix)]
            program: "sleep".to_string(),
            #[cfg(windows)]
            program: "powershell".to_string(),
            #[cfg(unix)]
            args: vec![secs.to_string()],
            #[cfg(windows)]
            args: vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                format!("Start-Sleep -Seconds {secs}"),
            ],
            env: vec![],
            cwd: std::env::temp_dir(),
            initial_cols: 80,
            initial_rows: 24,
        }
    }

    #[tokio::test]
    async fn pty_spawn_echo_streams_output() {
        let (tx, mut rx) = mpsc::channel(64);
        let _session = PtySession::spawn(test_echo_config("hello pty"), tx).unwrap();

        let mut saw_output = false;
        let mut saw_completed = false;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                evt = rx.recv() => {
                    match evt {
                        Some(PtyEvent::Output(data)) => {
                            let text = String::from_utf8_lossy(&data);
                            if text.contains("hello pty") {
                                saw_output = true;
                            }
                        }
                        Some(PtyEvent::Completed { .. }) => {
                            saw_completed = true;
                            break;
                        }
                        Some(PtyEvent::Failed { .. }) | None => break,
                        _ => {}
                    }
                }
            }
        }

        assert!(saw_output, "should see echo output");
        assert!(saw_completed, "should see completed event");
    }

    #[tokio::test]
    async fn pty_write_input_reaches_process() {
        let (tx, mut rx) = mpsc::channel(128);
        let session = PtySession::spawn(test_cat_config(), tx).unwrap();

        tokio::time::sleep(Duration::from_millis(200)).await;

        session.write_input(b"test input\n").await.unwrap();

        let mut saw_echo = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                evt = rx.recv() => {
                    match evt {
                        Some(PtyEvent::Output(data)) => {
                            let text = String::from_utf8_lossy(&data);
                            if text.contains("test input") {
                                saw_echo = true;
                                break;
                            }
                        }
                        None => break,
                        _ => {}
                    }
                }
            }
        }

        session.stop().await;
        assert!(saw_echo, "cat should echo back input");
    }

    #[tokio::test]
    async fn pty_resize_propagates_without_error() {
        let (tx, _rx) = mpsc::channel(64);
        let session = PtySession::spawn(test_sleep_config(30), tx).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        let result = session.resize(132, 50).await;
        assert!(result.is_ok(), "resize should succeed on running session");
        session.stop().await;
    }

    #[tokio::test]
    async fn pty_stop_terminates_process() {
        let (tx, mut rx) = mpsc::channel(64);
        let session = PtySession::spawn(test_sleep_config(60), tx).unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        session.stop().await;

        let mut saw_stopped = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                evt = rx.recv() => {
                    match evt {
                        Some(PtyEvent::Stopped { .. }) => {
                            saw_stopped = true;
                            break;
                        }
                        None => break,
                        _ => {}
                    }
                }
            }
        }

        assert!(saw_stopped, "should emit Stopped event");
        assert_eq!(session.status().await, PtySessionStatus::Stopped);
    }

    #[tokio::test]
    async fn pty_write_after_stop_returns_error() {
        let (tx, mut rx) = mpsc::channel(64);
        let session = PtySession::spawn(test_sleep_config(60), tx).unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        session.stop().await;

        // Drain events until stopped
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                evt = rx.recv() => {
                    match evt {
                        Some(PtyEvent::Stopped { .. }) | None => break,
                        _ => {}
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        let result = session.write_input(b"too late\n").await;
        assert!(result.is_err(), "write after stop should fail");
    }

    #[tokio::test]
    async fn pty_resize_after_stop_returns_error() {
        let (tx, mut rx) = mpsc::channel(64);
        let session = PtySession::spawn(test_sleep_config(60), tx).unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        session.stop().await;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                evt = rx.recv() => {
                    match evt {
                        Some(PtyEvent::Stopped { .. }) | None => break,
                        _ => {}
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        let result = session.resize(80, 24).await;
        assert!(result.is_err(), "resize after stop should fail");
    }

    #[tokio::test]
    async fn pty_spawn_nonexistent_binary_fails() {
        let config = PtySessionConfig {
            program: "/nonexistent/hydra_test_binary".to_string(),
            args: vec![],
            env: vec![],
            cwd: std::env::temp_dir(),
            initial_cols: 80,
            initial_rows: 24,
        };
        let (tx, _rx) = mpsc::channel(64);
        let result = PtySession::spawn(config, tx);
        assert!(result.is_err(), "spawn of nonexistent binary should fail");
    }

    #[tokio::test]
    async fn pty_idempotent_stop() {
        let (tx, mut rx) = mpsc::channel(64);
        let session = PtySession::spawn(test_sleep_config(60), tx).unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        session.stop().await;
        // Second stop should not panic or deadlock
        session.stop().await;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                evt = rx.recv() => {
                    match evt {
                        Some(PtyEvent::Stopped { .. }) | None => break,
                        _ => {}
                    }
                }
            }
        }

        assert_eq!(session.status().await, PtySessionStatus::Stopped);
    }
}
