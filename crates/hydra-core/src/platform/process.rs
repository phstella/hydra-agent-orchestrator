//! Platform-aware process management utilities.
//!
//! Provides cross-platform process termination, orphan detection, and cleanup.

use std::time::Duration;

use tracing::{debug, warn};

use crate::Result;

/// Grace period after initial termination signal before force-killing.
const TERMINATE_GRACE: Duration = Duration::from_secs(2);

/// Send a termination signal to a process by PID.
///
/// On Unix: sends SIGTERM, waits [`TERMINATE_GRACE`], then SIGKILL.
/// On Windows: uses `taskkill /PID <pid> /F`.
pub async fn terminate_process(pid: u32) -> Result<()> {
    debug!(pid, "terminating process");

    #[cfg(unix)]
    {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;

        let nix_pid = Pid::from_raw(pid as i32);

        // SIGTERM for graceful shutdown.
        if let Err(e) = signal::kill(nix_pid, Signal::SIGTERM) {
            debug!(pid, error = %e, "SIGTERM failed (process may have already exited)");
            return Ok(());
        }

        tokio::time::sleep(TERMINATE_GRACE).await;

        // SIGKILL as fallback.
        if let Err(e) = signal::kill(nix_pid, Signal::SIGKILL) {
            debug!(pid, error = %e, "SIGKILL failed (process may have already exited)");
        }
    }

    #[cfg(windows)]
    {
        let output = tokio::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output()
            .await;

        match output {
            Ok(o) if !o.status.success() => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                debug!(pid, %stderr, "taskkill failed (process may have already exited)");
            }
            Err(e) => {
                debug!(pid, error = %e, "failed to execute taskkill");
            }
            _ => {}
        }
    }

    Ok(())
}

/// Check for orphan processes from a specific Hydra run.
///
/// Searches for processes whose working directory or environment indicates
/// they belong to the given run. Returns a list of PIDs.
pub async fn check_orphan_processes(run_id: &str) -> Vec<u32> {
    debug!(run_id, "checking for orphan processes");

    #[cfg(unix)]
    {
        // Use `pgrep` to find processes with the run_id in their command line.
        let output = tokio::process::Command::new("pgrep")
            .args(["-f", run_id])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout
                    .lines()
                    .filter_map(|line| line.trim().parse::<u32>().ok())
                    .collect()
            }
            _ => vec![],
        }
    }

    #[cfg(windows)]
    {
        // On Windows, use WMIC to find processes.
        let output = tokio::process::Command::new("wmic")
            .args([
                "process",
                "where",
                &format!("CommandLine like '%{run_id}%'"),
                "get",
                "ProcessId",
                "/format:list",
            ])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout
                    .lines()
                    .filter_map(|line| {
                        line.strip_prefix("ProcessId=")
                            .and_then(|s| s.trim().parse::<u32>().ok())
                    })
                    .collect()
            }
            _ => vec![],
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        vec![]
    }
}

/// Kill a list of orphan processes.
pub async fn kill_orphans(pids: &[u32]) -> Result<()> {
    for &pid in pids {
        if let Err(e) = terminate_process(pid).await {
            warn!(pid, error = %e, "failed to kill orphan process");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn terminate_nonexistent_process_is_ok() {
        // PID 999999999 almost certainly does not exist.
        let result = terminate_process(999_999_999).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn check_orphans_for_nonexistent_run() {
        let pids = check_orphan_processes("nonexistent-run-id-12345").await;
        // Should return empty, not error.
        assert!(pids.is_empty());
    }

    #[tokio::test]
    async fn kill_orphans_empty_list() {
        let result = kill_orphans(&[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn kill_orphans_with_bad_pids() {
        let result = kill_orphans(&[999_999_999]).await;
        assert!(result.is_ok());
    }
}
