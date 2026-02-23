use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::{HydraError, Result};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Configuration for supervising an agent process.
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    pub run_id: Uuid,
    pub agent_key: String,
    pub idle_timeout: Duration,
    pub hard_timeout: Duration,
    /// Maximum bytes of combined stdout/stderr to retain in memory.
    pub max_output_bytes: usize,
}

/// A command ready to be executed by the supervisor.
#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
}

/// Lifecycle events emitted by the supervisor.
#[derive(Debug, Clone)]
pub enum SupervisorEvent {
    Started {
        agent_key: String,
        pid: u32,
        started_at: DateTime<Utc>,
    },
    Stdout {
        agent_key: String,
        line: String,
    },
    Stderr {
        agent_key: String,
        line: String,
    },
    Completed {
        agent_key: String,
        exit_code: i32,
        completed_at: DateTime<Utc>,
    },
    Failed {
        agent_key: String,
        error: String,
        failed_at: DateTime<Utc>,
    },
    TimedOut {
        agent_key: String,
        reason: TimeoutReason,
        timed_out_at: DateTime<Utc>,
    },
    Cancelled {
        agent_key: String,
        cancelled_at: DateTime<Utc>,
    },
}

/// Reason a timeout was triggered.
#[derive(Debug, Clone, PartialEq)]
pub enum TimeoutReason {
    Idle,
    Hard,
}

/// Final result of a supervised process execution.
#[derive(Debug)]
pub struct SupervisorResult {
    pub agent_key: String,
    pub exit_code: Option<i32>,
    pub status: ProcessStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub stdout_lines: Vec<String>,
    pub stderr_lines: Vec<String>,
}

/// Terminal status of a supervised process.
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStatus {
    Completed,
    Failed,
    TimedOut(TimeoutReason),
    Cancelled,
}

/// Handle allowing the caller to cancel a running supervised process.
pub struct SupervisorHandle {
    cancel_tx: tokio::sync::oneshot::Sender<()>,
}

impl SupervisorHandle {
    /// Send a cancellation signal to the supervised process.
    pub fn cancel(self) {
        // Ignoring error: receiver may already be dropped if process exited.
        let _ = self.cancel_tx.send(());
    }
}

/// The process supervisor manages a single agent process.
pub struct ProcessSupervisor {
    config: SupervisorConfig,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

/// Grace period after SIGTERM before escalating to SIGKILL.
const SIGTERM_GRACE: Duration = Duration::from_secs(3);

impl ProcessSupervisor {
    pub fn new(config: SupervisorConfig) -> Self {
        Self { config }
    }

    /// Spawn and supervise an agent process.
    ///
    /// Returns an event receiver and a handle for cancellation.
    pub async fn spawn(
        &self,
        command: AgentCommand,
    ) -> Result<(mpsc::Receiver<SupervisorEvent>, SupervisorHandle)> {
        let (event_tx, event_rx) = mpsc::channel(256);
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();

        let config = self.config.clone();

        let mut child = Command::new(&command.program)
            .args(&command.args)
            .envs(command.env.iter().map(|(k, v)| (k, v)))
            .current_dir(&command.cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| HydraError::Process(format!("failed to spawn process: {e}")))?;

        let pid = child
            .id()
            .ok_or_else(|| HydraError::Process("process exited before pid was read".into()))?;

        let started_at = Utc::now();
        let _ = event_tx
            .send(SupervisorEvent::Started {
                agent_key: config.agent_key.clone(),
                pid,
                started_at,
            })
            .await;

        // Take ownership of stdout/stderr handles.
        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        tokio::spawn(async move {
            supervise_loop(config, child, stdout, stderr, event_tx, cancel_rx).await;
        });

        let handle = SupervisorHandle { cancel_tx };
        Ok((event_rx, handle))
    }

    /// Run an agent to completion, collecting all events into a `SupervisorResult`.
    pub async fn run_to_completion(&self, command: AgentCommand) -> Result<SupervisorResult> {
        let (mut rx, _handle) = self.spawn(command).await?;

        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();
        let mut exit_code: Option<i32> = None;
        let mut status = ProcessStatus::Failed;
        let mut started_at = Utc::now();
        let mut completed_at = started_at;

        while let Some(event) = rx.recv().await {
            match event {
                SupervisorEvent::Started { started_at: ts, .. } => {
                    started_at = ts;
                }
                SupervisorEvent::Stdout { line, .. } => {
                    stdout_lines.push(line);
                }
                SupervisorEvent::Stderr { line, .. } => {
                    stderr_lines.push(line);
                }
                SupervisorEvent::Completed {
                    exit_code: code,
                    completed_at: ts,
                    ..
                } => {
                    exit_code = Some(code);
                    completed_at = ts;
                    status = if code == 0 {
                        ProcessStatus::Completed
                    } else {
                        ProcessStatus::Failed
                    };
                }
                SupervisorEvent::Failed { failed_at: ts, .. } => {
                    completed_at = ts;
                    status = ProcessStatus::Failed;
                }
                SupervisorEvent::TimedOut {
                    reason,
                    timed_out_at: ts,
                    ..
                } => {
                    completed_at = ts;
                    status = ProcessStatus::TimedOut(reason);
                }
                SupervisorEvent::Cancelled {
                    cancelled_at: ts, ..
                } => {
                    completed_at = ts;
                    status = ProcessStatus::Cancelled;
                }
            }
        }

        Ok(SupervisorResult {
            agent_key: self.config.agent_key.clone(),
            exit_code,
            status,
            started_at,
            completed_at,
            stdout_lines,
            stderr_lines,
        })
    }
}

// ---------------------------------------------------------------------------
// Core supervision loop
// ---------------------------------------------------------------------------

/// Internal enum for merging stdout/stderr streams.
enum OutputLine {
    Stdout(String),
    Stderr(String),
}

async fn supervise_loop(
    config: SupervisorConfig,
    mut child: tokio::process::Child,
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    event_tx: mpsc::Sender<SupervisorEvent>,
    cancel_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let agent_key = &config.agent_key;

    // Merge stdout and stderr into a single stream via a local channel.
    let (line_tx, mut line_rx) = mpsc::channel::<OutputLine>(256);

    let stdout_tx = line_tx.clone();
    let stderr_tx = line_tx;

    // Spawn reader tasks for stdout and stderr.
    let stdout_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if stdout_tx.send(OutputLine::Stdout(line)).await.is_err() {
                break;
            }
        }
    });

    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if stderr_tx.send(OutputLine::Stderr(line)).await.is_err() {
                break;
            }
        }
    });

    // Bounded output tracking.
    let mut total_bytes: usize = 0;
    let mut truncated = false;
    let max_bytes = config.max_output_bytes;

    // Ring buffer to keep the most recent lines when over the byte limit.
    let mut ring: VecDeque<OutputLine> = VecDeque::new();
    let mut ring_bytes: usize = 0;

    let hard_deadline = tokio::time::Instant::now() + config.hard_timeout;
    let idle_dur = config.idle_timeout;

    tokio::pin!(cancel_rx);

    loop {
        let idle_timeout = tokio::time::sleep(idle_dur);
        let hard_timeout = tokio::time::sleep_until(hard_deadline);

        tokio::select! {
            biased;

            // Cancellation signal.
            _ = &mut cancel_rx => {
                debug!(agent_key, "cancel signal received");
                terminate_child(&mut child).await;
                let _ = event_tx.send(SupervisorEvent::Cancelled {
                    agent_key: agent_key.clone(),
                    cancelled_at: Utc::now(),
                }).await;
                break;
            }

            // Hard timeout.
            _ = hard_timeout => {
                warn!(agent_key, elapsed_secs = config.hard_timeout.as_secs(), "hard timeout reached");
                terminate_child(&mut child).await;
                let _ = event_tx.send(SupervisorEvent::TimedOut {
                    agent_key: agent_key.clone(),
                    reason: TimeoutReason::Hard,
                    timed_out_at: Utc::now(),
                }).await;
                break;
            }

            // Idle timeout.
            _ = idle_timeout => {
                warn!(agent_key, idle_secs = idle_dur.as_secs(), "idle timeout reached");
                terminate_child(&mut child).await;
                let _ = event_tx.send(SupervisorEvent::TimedOut {
                    agent_key: agent_key.clone(),
                    reason: TimeoutReason::Idle,
                    timed_out_at: Utc::now(),
                }).await;
                break;
            }

            // Output from the process.
            line = line_rx.recv() => {
                match line {
                    Some(output) => {
                        let line_bytes = match &output {
                            OutputLine::Stdout(l) | OutputLine::Stderr(l) => l.len(),
                        };
                        total_bytes += line_bytes;

                        // If over the limit, switch to ring-buffer mode.
                        if total_bytes > max_bytes {
                            if !truncated {
                                warn!(
                                    agent_key,
                                    total_bytes,
                                    max_bytes,
                                    "output exceeds max_output_bytes; truncating older lines"
                                );
                                truncated = true;
                            }

                            ring_bytes += line_bytes;
                            ring.push_back(output);

                            // Evict oldest lines until we are back within budget.
                            while ring_bytes > max_bytes {
                                if let Some(old) = ring.pop_front() {
                                    let old_bytes = match &old {
                                        OutputLine::Stdout(l) | OutputLine::Stderr(l) => l.len(),
                                    };
                                    ring_bytes -= old_bytes;
                                }
                            }
                        } else {
                            // Still under the limit: emit events immediately.
                            match output {
                                OutputLine::Stdout(l) => {
                                    let _ = event_tx.send(SupervisorEvent::Stdout {
                                        agent_key: agent_key.clone(),
                                        line: l,
                                    }).await;
                                }
                                OutputLine::Stderr(l) => {
                                    let _ = event_tx.send(SupervisorEvent::Stderr {
                                        agent_key: agent_key.clone(),
                                        line: l,
                                    }).await;
                                }
                            }
                        }
                    }
                    None => {
                        // Both stdout and stderr closed; wait for process exit.
                        match child.wait().await {
                            Ok(exit_status) => {
                                let code = exit_status.code().unwrap_or(-1);
                                debug!(agent_key, code, "process exited");

                                // Drain any remaining ring-buffer lines.
                                drain_ring(&ring, agent_key, &event_tx).await;

                                let _ = event_tx
                                    .send(SupervisorEvent::Completed {
                                        agent_key: agent_key.clone(),
                                        exit_code: code,
                                        completed_at: Utc::now(),
                                    })
                                    .await;
                            }
                            Err(e) => {
                                // Drain any remaining ring-buffer lines.
                                drain_ring(&ring, agent_key, &event_tx).await;

                                let _ = event_tx.send(SupervisorEvent::Failed {
                                    agent_key: agent_key.clone(),
                                    error: e.to_string(),
                                    failed_at: Utc::now(),
                                }).await;
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    // Ensure reader tasks are cleaned up.
    stdout_task.abort();
    stderr_task.abort();
}

/// Drain remaining ring-buffer lines as events.
async fn drain_ring(
    ring: &VecDeque<OutputLine>,
    agent_key: &str,
    event_tx: &mpsc::Sender<SupervisorEvent>,
) {
    for item in ring {
        match item {
            OutputLine::Stdout(l) => {
                let _ = event_tx
                    .send(SupervisorEvent::Stdout {
                        agent_key: agent_key.to_owned(),
                        line: l.clone(),
                    })
                    .await;
            }
            OutputLine::Stderr(l) => {
                let _ = event_tx
                    .send(SupervisorEvent::Stderr {
                        agent_key: agent_key.to_owned(),
                        line: l.clone(),
                    })
                    .await;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Platform-specific process termination
// ---------------------------------------------------------------------------

#[cfg(unix)]
async fn terminate_child(child: &mut tokio::process::Child) {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;

    if let Some(pid) = child.id() {
        // Send SIGTERM first.
        let pid = Pid::from_raw(pid as i32);
        let _ = signal::kill(pid, Signal::SIGTERM);

        // Wait for graceful exit up to SIGTERM_GRACE.
        match tokio::time::timeout(SIGTERM_GRACE, child.wait()).await {
            Ok(_) => (),
            Err(_) => {
                warn!(?pid, "process did not exit after SIGTERM; sending SIGKILL");
                let _ = signal::kill(pid, Signal::SIGKILL);
                let _ = child.wait().await;
            }
        }
    } else {
        // Process already exited.
        let _ = child.wait().await;
    }
}

#[cfg(windows)]
async fn terminate_child(child: &mut tokio::process::Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_config(agent_key: &str) -> SupervisorConfig {
        SupervisorConfig {
            run_id: Uuid::new_v4(),
            agent_key: agent_key.to_string(),
            idle_timeout: Duration::from_secs(10),
            hard_timeout: Duration::from_secs(30),
            max_output_bytes: 1024 * 1024,
        }
    }

    fn echo_command(msg: &str) -> AgentCommand {
        AgentCommand {
            program: "echo".to_string(),
            args: vec![msg.to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
        }
    }

    #[tokio::test]
    async fn test_successful_completion() {
        let supervisor = ProcessSupervisor::new(test_config("test-agent"));
        let result = supervisor
            .run_to_completion(echo_command("hello"))
            .await
            .unwrap();

        assert_eq!(result.status, ProcessStatus::Completed);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout_lines.contains(&"hello".to_string()));
    }

    #[tokio::test]
    async fn test_process_failure() {
        let supervisor = ProcessSupervisor::new(test_config("fail-agent"));
        let cmd = AgentCommand {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), "exit 42".to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
        };
        let result = supervisor.run_to_completion(cmd).await.unwrap();

        assert_eq!(result.status, ProcessStatus::Failed);
        assert_eq!(result.exit_code, Some(42));
    }

    #[tokio::test]
    async fn test_hard_timeout() {
        let mut config = test_config("timeout-agent");
        config.hard_timeout = Duration::from_millis(200);
        config.idle_timeout = Duration::from_secs(60); // won't trigger

        let supervisor = ProcessSupervisor::new(config);
        let cmd = AgentCommand {
            program: "sleep".to_string(),
            args: vec!["999".to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
        };
        let result = supervisor.run_to_completion(cmd).await.unwrap();

        assert_eq!(result.status, ProcessStatus::TimedOut(TimeoutReason::Hard));
    }

    #[tokio::test]
    async fn test_idle_timeout() {
        let mut config = test_config("idle-agent");
        config.idle_timeout = Duration::from_millis(200);
        config.hard_timeout = Duration::from_secs(60);

        let supervisor = ProcessSupervisor::new(config);
        // Write one line then hang forever -- idle timeout fires after no further output.
        let cmd = AgentCommand {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), "echo start && sleep 999".to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
        };
        let result = supervisor.run_to_completion(cmd).await.unwrap();

        assert_eq!(result.status, ProcessStatus::TimedOut(TimeoutReason::Idle));
    }

    #[tokio::test]
    async fn test_cancellation() {
        let mut config = test_config("cancel-agent");
        config.hard_timeout = Duration::from_secs(60);
        config.idle_timeout = Duration::from_secs(60);

        let supervisor = ProcessSupervisor::new(config);
        let cmd = AgentCommand {
            program: "sleep".to_string(),
            args: vec!["999".to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
        };

        let (mut rx, handle) = supervisor.spawn(cmd).await.unwrap();

        // Wait for Started event.
        let first = rx.recv().await.unwrap();
        assert!(matches!(first, SupervisorEvent::Started { .. }));

        // Cancel.
        handle.cancel();

        // Drain events until we see Cancelled.
        let mut saw_cancelled = false;
        while let Some(event) = rx.recv().await {
            if matches!(event, SupervisorEvent::Cancelled { .. }) {
                saw_cancelled = true;
            }
        }
        assert!(saw_cancelled, "expected a Cancelled event");
    }

    #[tokio::test]
    async fn test_bounded_output_buffering() {
        let mut config = test_config("buffer-agent");
        // Very small limit so we trigger truncation quickly.
        config.max_output_bytes = 50;

        let supervisor = ProcessSupervisor::new(config);
        // Generate many lines of output.
        let cmd = AgentCommand {
            program: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "for i in $(seq 1 100); do echo \"line-$i\"; done".to_string(),
            ],
            env: vec![],
            cwd: std::env::temp_dir(),
        };

        let result = supervisor.run_to_completion(cmd).await.unwrap();

        assert_eq!(result.status, ProcessStatus::Completed);
        // We should have fewer than 100 stdout lines because old ones were evicted.
        assert!(
            result.stdout_lines.len() < 100,
            "expected truncated output, got {} lines",
            result.stdout_lines.len()
        );
        // The last line should still be present.
        assert!(
            result.stdout_lines.last().is_some_and(|l| l == "line-100"),
            "expected last line to be line-100, got {:?}",
            result.stdout_lines.last()
        );
    }

    #[tokio::test]
    async fn test_event_ordering() {
        let supervisor = ProcessSupervisor::new(test_config("order-agent"));
        let cmd = AgentCommand {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), "echo hello && echo world".to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
        };

        let (mut rx, _handle) = supervisor.spawn(cmd).await.unwrap();

        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
        }

        // First event must be Started.
        assert!(
            matches!(events.first(), Some(SupervisorEvent::Started { .. })),
            "first event should be Started"
        );

        // Last event must be a terminal event.
        let last = events.last().unwrap();
        assert!(
            matches!(
                last,
                SupervisorEvent::Completed { .. }
                    | SupervisorEvent::Failed { .. }
                    | SupervisorEvent::TimedOut { .. }
                    | SupervisorEvent::Cancelled { .. }
            ),
            "last event should be terminal"
        );
    }

    #[tokio::test]
    async fn test_stderr_captured() {
        let supervisor = ProcessSupervisor::new(test_config("stderr-agent"));
        let cmd = AgentCommand {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), "echo errline >&2".to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
        };
        let result = supervisor.run_to_completion(cmd).await.unwrap();

        assert_eq!(result.status, ProcessStatus::Completed);
        assert!(
            result.stderr_lines.contains(&"errline".to_string()),
            "expected stderr to contain 'errline', got {:?}",
            result.stderr_lines
        );
    }

    #[tokio::test]
    async fn test_env_and_cwd_passed() {
        let supervisor = ProcessSupervisor::new(test_config("env-agent"));
        let cmd = AgentCommand {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), "echo $HYDRA_TEST_VAR".to_string()],
            env: vec![("HYDRA_TEST_VAR".to_string(), "hello-hydra".to_string())],
            cwd: std::env::temp_dir(),
        };
        let result = supervisor.run_to_completion(cmd).await.unwrap();

        assert_eq!(result.status, ProcessStatus::Completed);
        assert!(
            result.stdout_lines.contains(&"hello-hydra".to_string()),
            "expected env var in output, got {:?}",
            result.stdout_lines
        );
    }
}
