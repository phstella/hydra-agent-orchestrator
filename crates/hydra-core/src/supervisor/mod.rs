pub mod pty;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::adapter::{AgentEvent, BuiltCommand};

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("failed to spawn process: {0}")]
    SpawnFailed(std::io::Error),

    #[error("process timed out after {seconds}s ({kind})")]
    TimedOut { seconds: u64, kind: TimeoutKind },

    #[error("process was cancelled")]
    Cancelled,

    #[error("process exited with code {code}")]
    NonZeroExit { code: i32 },

    #[error("I/O error during supervision: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutKind {
    Hard,
    Idle,
}

impl std::fmt::Display for TimeoutKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutKind::Hard => write!(f, "hard"),
            TimeoutKind::Idle => write!(f, "idle"),
        }
    }
}

/// Lifecycle event emitted by the supervisor for each agent process.
#[derive(Debug, Clone)]
pub enum SupervisorEvent {
    Started {
        pid: u32,
    },
    Stdout(String),
    Stderr(String),
    AgentEvent(AgentEvent),
    Completed {
        exit_code: i32,
        duration: Duration,
    },
    Failed {
        error: String,
        duration: Duration,
    },
    TimedOut {
        kind: TimeoutKind,
        duration: Duration,
    },
}

/// Runtime policy for supervisor timeout and buffer limits.
///
/// Distinct from `config::SupervisorConfig` which is the TOML-deserialized schema type.
#[derive(Debug, Clone)]
pub struct SupervisorPolicy {
    pub hard_timeout: Duration,
    pub idle_timeout: Duration,
    pub output_buffer_bytes: usize,
}

impl Default for SupervisorPolicy {
    fn default() -> Self {
        Self {
            hard_timeout: Duration::from_secs(1800),
            idle_timeout: Duration::from_secs(300),
            output_buffer_bytes: 10 * 1024 * 1024,
        }
    }
}

impl SupervisorPolicy {
    pub fn from_hydra_config(cfg: &crate::config::SupervisorConfig) -> Self {
        Self {
            hard_timeout: Duration::from_secs(cfg.hard_timeout_seconds),
            idle_timeout: Duration::from_secs(cfg.idle_timeout_seconds),
            output_buffer_bytes: cfg.output_buffer_bytes,
        }
    }
}

/// Handle to a running supervised process, used for cancellation.
pub struct SupervisorHandle {
    cancel_tx: mpsc::Sender<()>,
}

impl SupervisorHandle {
    /// Request cancellation of the supervised process.
    pub async fn cancel(&self) {
        let _ = self.cancel_tx.send(()).await;
    }
}

/// Spawn and supervise a single agent process.
///
/// Returns a handle for cancellation and streams events via the provided channel.
/// The line_parser closure converts raw stdout lines into optional `AgentEvent`s.
pub async fn supervise<F>(
    cmd: BuiltCommand,
    policy: SupervisorPolicy,
    event_tx: mpsc::Sender<SupervisorEvent>,
    line_parser: F,
) -> Result<SupervisorHandle, SupervisorError>
where
    F: Fn(&str) -> Option<AgentEvent> + Send + 'static,
{
    let mut child = build_process(&cmd)?;

    let pid = child.id().unwrap_or(0);
    let _ = event_tx.send(SupervisorEvent::Started { pid }).await;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| SupervisorError::Io(std::io::Error::other("stdout not captured")))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| SupervisorError::Io(std::io::Error::other("stderr not captured")))?;

    let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
    let handle = SupervisorHandle {
        cancel_tx: cancel_tx.clone(),
    };

    let hard_timeout = policy.hard_timeout;
    let idle_timeout = policy.idle_timeout;
    let max_buffer = policy.output_buffer_bytes;

    tokio::spawn(async move {
        let start = Instant::now();

        let stdout_tx = event_tx.clone();
        let stderr_tx = event_tx.clone();

        let (idle_reset_tx, mut idle_reset_rx) = mpsc::channel::<()>(32);

        let stdout_idle_tx = idle_reset_tx.clone();
        let stderr_idle_tx = idle_reset_tx;

        let stdout_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let mut total_bytes: usize = 0;

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(n) => {
                        total_bytes = total_bytes.saturating_add(n);
                        let _ = stdout_idle_tx.send(()).await;

                        if total_bytes <= max_buffer {
                            let trimmed = line.trim_end();
                            if let Some(evt) = line_parser(trimmed) {
                                let _ = stdout_tx.send(SupervisorEvent::AgentEvent(evt)).await;
                            }
                            let _ = stdout_tx
                                .send(SupervisorEvent::Stdout(trimmed.to_string()))
                                .await;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let stderr_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let _ = stderr_idle_tx.send(()).await;
                        let _ = stderr_tx
                            .send(SupervisorEvent::Stderr(line.trim_end().to_string()))
                            .await;
                    }
                    Err(_) => break,
                }
            }
        });

        tokio::select! {
            status = child.wait() => {
                match status {
                    Ok(s) => {
                        let code = s.code().unwrap_or(-1);
                        let duration = start.elapsed();
                        if code == 0 {
                            let _ = event_tx.send(SupervisorEvent::Completed {
                                exit_code: code,
                                duration,
                            }).await;
                        } else {
                            let _ = event_tx.send(SupervisorEvent::Failed {
                                error: format!("exited with code {code}"),
                                duration,
                            }).await;
                        }
                    }
                    Err(e) => {
                        let _ = event_tx.send(SupervisorEvent::Failed {
                            error: e.to_string(),
                            duration: start.elapsed(),
                        }).await;
                    }
                }
            }
            _ = tokio::time::sleep(hard_timeout) => {
                terminate_process(&mut child).await;
                let _ = event_tx.send(SupervisorEvent::TimedOut {
                    kind: TimeoutKind::Hard,
                    duration: start.elapsed(),
                }).await;
            }
            _ = idle_timeout_watch(idle_timeout, &mut idle_reset_rx) => {
                terminate_process(&mut child).await;
                let _ = event_tx.send(SupervisorEvent::TimedOut {
                    kind: TimeoutKind::Idle,
                    duration: start.elapsed(),
                }).await;
            }
            _ = cancel_rx.recv() => {
                terminate_process(&mut child).await;
                let _ = event_tx.send(SupervisorEvent::Failed {
                    error: "cancelled".to_string(),
                    duration: start.elapsed(),
                }).await;
            }
        };

        let _ = stdout_task.await;
        let _ = stderr_task.await;
    });

    Ok(handle)
}

async fn idle_timeout_watch(timeout: Duration, reset_rx: &mut mpsc::Receiver<()>) {
    loop {
        tokio::select! {
            _ = tokio::time::sleep(timeout) => {
                return;
            }
            msg = reset_rx.recv() => {
                match msg {
                    Some(()) => {
                        // Activity detected, restart the idle timer
                    }
                    None => {
                        // All senders dropped (stdout/stderr finished) — process is
                        // exiting normally; yield forever so child.wait() wins the
                        // outer select.
                        std::future::pending::<()>().await;
                    }
                }
            }
        }
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillErrorKind {
    NoSuchProcess,
    PermissionDenied,
    Other(i32),
    Unknown,
}

#[cfg(unix)]
fn classify_kill_error(errno: Option<i32>) -> KillErrorKind {
    match errno {
        Some(code) if code == libc::ESRCH => KillErrorKind::NoSuchProcess,
        Some(code) if code == libc::EPERM => KillErrorKind::PermissionDenied,
        Some(code) => KillErrorKind::Other(code),
        None => KillErrorKind::Unknown,
    }
}

async fn terminate_process(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    {
        let Some(pid_u32) = child.id() else {
            let _ = child.kill().await;
            return;
        };

        let pid = pid_u32 as i32;
        let pgid = -pid;

        let term_result = unsafe { libc::kill(pgid, libc::SIGTERM) };
        if term_result != 0 {
            match classify_kill_error(std::io::Error::last_os_error().raw_os_error()) {
                KillErrorKind::NoSuchProcess => {
                    tracing::debug!(pid, "process group already exited before SIGTERM");
                    return;
                }
                KillErrorKind::PermissionDenied => {
                    tracing::warn!(pid, "permission denied sending SIGTERM to process group");
                    let _ = child.kill().await;
                    return;
                }
                KillErrorKind::Other(errno) => {
                    tracing::warn!(
                        pid,
                        errno,
                        "failed sending SIGTERM to process group; falling back to child.kill"
                    );
                    let _ = child.kill().await;
                    return;
                }
                KillErrorKind::Unknown => {
                    tracing::warn!(
                        pid,
                        "failed sending SIGTERM to process group with unknown errno; falling back"
                    );
                    let _ = child.kill().await;
                    return;
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(300)).await;

        if let Ok(Some(_)) = child.try_wait() {
            return;
        }

        let kill_result = unsafe { libc::kill(pgid, libc::SIGKILL) };
        if kill_result != 0 {
            match classify_kill_error(std::io::Error::last_os_error().raw_os_error()) {
                KillErrorKind::NoSuchProcess => {
                    tracing::debug!(pid, "process group exited before SIGKILL escalation");
                    return;
                }
                KillErrorKind::PermissionDenied => {
                    tracing::warn!(pid, "permission denied sending SIGKILL to process group");
                }
                KillErrorKind::Other(errno) => {
                    tracing::warn!(
                        pid,
                        errno,
                        "failed sending SIGKILL to process group; falling back to child.kill"
                    );
                }
                KillErrorKind::Unknown => {
                    tracing::warn!(
                        pid,
                        "failed sending SIGKILL to process group with unknown errno; falling back"
                    );
                }
            }
        }
        let _ = child.kill().await;
    }

    #[cfg(not(unix))]
    {
        let _ = child.kill().await;
    }
}

fn build_process(cmd: &BuiltCommand) -> Result<tokio::process::Child, SupervisorError> {
    let mut command = Command::new(&cmd.program);
    command
        .args(&cmd.args)
        .current_dir(&cmd.cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null());

    for (key, val) in &cmd.env {
        command.env(key, val);
    }

    #[cfg(unix)]
    {
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    command.spawn().map_err(SupervisorError::SpawnFailed)
}

/// Resolve the working directory for a command, defaulting to the given fallback.
pub fn resolve_cwd(cmd_cwd: &Path, fallback: &Path) -> PathBuf {
    if cmd_cwd.as_os_str().is_empty() {
        fallback.to_path_buf()
    } else {
        cmd_cwd.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::BuiltCommand;
    use std::path::PathBuf;

    fn test_cwd() -> PathBuf {
        std::env::temp_dir()
    }

    fn echo_command(msg: &str) -> BuiltCommand {
        #[cfg(unix)]
        let (program, args) = ("echo".to_string(), vec![msg.to_string()]);
        #[cfg(windows)]
        let (program, args) = (
            "cmd".to_string(),
            vec!["/C".to_string(), format!("echo {msg}")],
        );

        BuiltCommand {
            program,
            args,
            env: vec![],
            cwd: test_cwd(),
        }
    }

    fn sleep_command(seconds: f64) -> BuiltCommand {
        #[cfg(unix)]
        let (program, args) = ("sleep".to_string(), vec![seconds.to_string()]);
        #[cfg(windows)]
        let (program, args) = (
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                format!("Start-Sleep -Seconds {}", seconds.ceil() as u64),
            ],
        );

        BuiltCommand {
            program,
            args,
            env: vec![],
            cwd: test_cwd(),
        }
    }

    fn failing_command() -> BuiltCommand {
        #[cfg(unix)]
        let (program, args) = (
            "sh".to_string(),
            vec!["-c".to_string(), "exit 42".to_string()],
        );
        #[cfg(windows)]
        let (program, args) = (
            "cmd".to_string(),
            vec!["/C".to_string(), "exit 42".to_string()],
        );

        BuiltCommand {
            program,
            args,
            env: vec![],
            cwd: test_cwd(),
        }
    }

    fn multiline_command() -> BuiltCommand {
        #[cfg(unix)]
        let (program, args) = (
            "sh".to_string(),
            vec![
                "-c".to_string(),
                "echo line1; echo line2; echo line3".to_string(),
            ],
        );
        #[cfg(windows)]
        let (program, args) = (
            "cmd".to_string(),
            vec![
                "/C".to_string(),
                "echo line1 && echo line2 && echo line3".to_string(),
            ],
        );

        BuiltCommand {
            program,
            args,
            env: vec![],
            cwd: test_cwd(),
        }
    }

    #[cfg(unix)]
    fn background_child_command() -> BuiltCommand {
        BuiltCommand {
            program: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "sleep 60 & echo child:$!; wait".to_string(),
            ],
            env: vec![],
            cwd: test_cwd(),
        }
    }

    #[cfg(unix)]
    fn process_exists(pid: i32) -> bool {
        unsafe {
            if libc::kill(pid, 0) == 0 {
                true
            } else {
                matches!(
                    std::io::Error::last_os_error().raw_os_error(),
                    Some(libc::EPERM)
                )
            }
        }
    }

    #[tokio::test]
    async fn supervise_echo_completes_successfully() {
        let (tx, mut rx) = mpsc::channel(64);
        let cmd = echo_command("hello hydra");
        let config = SupervisorPolicy {
            hard_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(5),
            output_buffer_bytes: 1024,
        };

        let _handle = supervise(cmd, config, tx, |_| None).await.unwrap();

        let mut saw_started = false;
        let mut saw_stdout = false;
        let mut saw_completed = false;

        while let Some(evt) = rx.recv().await {
            match evt {
                SupervisorEvent::Started { .. } => saw_started = true,
                SupervisorEvent::Stdout(line) if line.contains("hello hydra") => saw_stdout = true,
                SupervisorEvent::Completed { exit_code, .. } => {
                    assert_eq!(exit_code, 0);
                    saw_completed = true;
                    break;
                }
                _ => {}
            }
        }

        assert!(saw_started, "should emit Started event");
        assert!(saw_stdout, "should capture stdout");
        assert!(saw_completed, "should emit Completed event");
    }

    #[tokio::test]
    async fn supervise_failing_command_reports_failure() {
        let (tx, mut rx) = mpsc::channel(64);
        let cmd = failing_command();
        let config = SupervisorPolicy::default();

        let _handle = supervise(cmd, config, tx, |_| None).await.unwrap();

        let mut saw_failure = false;
        while let Some(evt) = rx.recv().await {
            if let SupervisorEvent::Failed { error, .. } = evt {
                assert!(error.contains("42"));
                saw_failure = true;
                break;
            }
        }
        assert!(saw_failure);
    }

    #[tokio::test]
    async fn supervise_cancellation() {
        let (tx, mut rx) = mpsc::channel(64);
        let cmd = sleep_command(60.0);
        let config = SupervisorPolicy {
            hard_timeout: Duration::from_secs(120),
            idle_timeout: Duration::from_secs(120),
            ..Default::default()
        };

        let handle = supervise(cmd, config, tx, |_| None).await.unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;
        handle.cancel().await;

        let mut saw_cancel = false;
        while let Some(evt) = rx.recv().await {
            if let SupervisorEvent::Failed { error, .. } = evt {
                if error.contains("cancelled") {
                    saw_cancel = true;
                    break;
                }
            }
        }
        assert!(saw_cancel, "should report cancellation");
    }

    #[tokio::test]
    async fn supervise_hard_timeout() {
        let (tx, mut rx) = mpsc::channel(64);
        let cmd = sleep_command(60.0);
        let config = SupervisorPolicy {
            hard_timeout: Duration::from_millis(200),
            idle_timeout: Duration::from_secs(120),
            ..Default::default()
        };

        let _handle = supervise(cmd, config, tx, |_| None).await.unwrap();

        let mut saw_timeout = false;
        while let Some(evt) = rx.recv().await {
            if let SupervisorEvent::TimedOut { kind, .. } = evt {
                assert_eq!(kind, TimeoutKind::Hard);
                saw_timeout = true;
                break;
            }
        }
        assert!(saw_timeout, "should report hard timeout");
    }

    #[tokio::test]
    async fn supervise_captures_multiline_stdout() {
        let (tx, mut rx) = mpsc::channel(64);
        let cmd = multiline_command();
        let config = SupervisorPolicy::default();

        let _handle = supervise(cmd, config, tx, |_| None).await.unwrap();

        let mut lines = Vec::new();
        while let Some(evt) = rx.recv().await {
            match evt {
                SupervisorEvent::Stdout(line) => lines.push(line),
                SupervisorEvent::Completed { .. } => break,
                SupervisorEvent::Failed { .. } => break,
                _ => {}
            }
        }

        assert!(lines.contains(&"line1".to_string()));
        assert!(lines.contains(&"line2".to_string()));
        assert!(lines.contains(&"line3".to_string()));
    }

    #[tokio::test]
    async fn supervise_with_line_parser() {
        let (tx, mut rx) = mpsc::channel(64);
        let cmd = echo_command(r#"{"type":"message","content":"hello"}"#);
        let config = SupervisorPolicy::default();

        let _handle = supervise(cmd, config, tx, |line| {
            if line.contains("hello") {
                Some(AgentEvent::Message {
                    content: "hello".to_string(),
                })
            } else {
                None
            }
        })
        .await
        .unwrap();

        let mut saw_agent_event = false;
        while let Some(evt) = rx.recv().await {
            if let SupervisorEvent::AgentEvent(AgentEvent::Message { ref content }) = evt {
                assert_eq!(content, "hello");
                saw_agent_event = true;
            }
            if matches!(
                evt,
                SupervisorEvent::Completed { .. } | SupervisorEvent::Failed { .. }
            ) {
                break;
            }
        }
        assert!(saw_agent_event, "should emit parsed agent event");
    }

    #[tokio::test]
    async fn supervise_nonexistent_binary_fails() {
        let (tx, _rx) = mpsc::channel(64);
        let cmd = BuiltCommand {
            program: if cfg!(windows) {
                r"Z:\definitely\missing\hydra.exe".to_string()
            } else {
                "/nonexistent/binary".to_string()
            },
            args: vec![],
            env: vec![],
            cwd: test_cwd(),
        };
        let config = SupervisorPolicy::default();

        let result = supervise(cmd, config, tx, |_| None).await;
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn hard_timeout_kills_background_child_process() {
        let (tx, mut rx) = mpsc::channel(64);
        let cmd = background_child_command();
        let config = SupervisorPolicy {
            hard_timeout: Duration::from_millis(200),
            idle_timeout: Duration::from_secs(30),
            ..Default::default()
        };

        let _handle = supervise(cmd, config, tx, |_| None).await.unwrap();

        let mut child_pid: Option<i32> = None;
        let mut saw_timeout = false;
        while let Some(evt) = rx.recv().await {
            match evt {
                SupervisorEvent::Stdout(line) => {
                    if let Some(pid) = line.strip_prefix("child:") {
                        child_pid = pid.trim().parse::<i32>().ok();
                    }
                }
                SupervisorEvent::TimedOut {
                    kind: TimeoutKind::Hard,
                    ..
                } => {
                    saw_timeout = true;
                    break;
                }
                _ => {}
            }
        }

        assert!(saw_timeout, "expected hard timeout");
        let pid = child_pid.expect("expected background child pid in stdout");

        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            !process_exists(pid),
            "background child process should be terminated with process group"
        );
    }

    #[cfg(unix)]
    #[test]
    fn classify_kill_error_distinguishes_known_errno() {
        assert_eq!(
            classify_kill_error(Some(libc::ESRCH)),
            KillErrorKind::NoSuchProcess
        );
        assert_eq!(
            classify_kill_error(Some(libc::EPERM)),
            KillErrorKind::PermissionDenied
        );
        assert_eq!(
            classify_kill_error(Some(libc::EINVAL)),
            KillErrorKind::Other(libc::EINVAL)
        );
        assert_eq!(classify_kill_error(None), KillErrorKind::Unknown);
    }
}
