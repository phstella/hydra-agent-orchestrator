use std::path::Path;
use std::time::Duration;

use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

pub const DEFAULT_GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug)]
pub struct GitCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Error)]
pub enum GitExecError {
    #[error("git command timed out after {timeout_secs}s: {command}")]
    TimedOut { command: String, timeout_secs: u64 },

    #[error("git command failed with exit code {code:?}: {command}; stderr: {stderr}")]
    NonZeroExit {
        command: String,
        code: Option<i32>,
        stdout: String,
        stderr: String,
    },

    #[error("failed to execute git command: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn run_git(args: &[&str], cwd: &Path) -> Result<GitCommandOutput, GitExecError> {
    run_git_program_with_timeout("git", args, cwd, DEFAULT_GIT_COMMAND_TIMEOUT).await
}

pub async fn run_git_program_with_timeout(
    program: &str,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> Result<GitCommandOutput, GitExecError> {
    let command = render_command(program, args);
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let mut stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("stdout pipe missing"))?;
    let mut stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| std::io::Error::other("stderr pipe missing"))?;

    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stdout_pipe.read_to_end(&mut buf).await?;
        Ok::<Vec<u8>, std::io::Error>(buf)
    });
    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stderr_pipe.read_to_end(&mut buf).await?;
        Ok::<Vec<u8>, std::io::Error>(buf)
    });

    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => return Err(GitExecError::Io(e)),
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            return Err(GitExecError::TimedOut {
                command,
                timeout_secs: timeout.as_secs(),
            });
        }
    };

    let stdout = stdout_task
        .await
        .map_err(|e| std::io::Error::other(e.to_string()))??;
    let stderr = stderr_task
        .await
        .map_err(|e| std::io::Error::other(e.to_string()))??;
    let stdout = String::from_utf8_lossy(&stdout).to_string();
    let stderr = String::from_utf8_lossy(&stderr).to_string();
    let exit_code = status.code();

    if !status.success() {
        return Err(GitExecError::NonZeroExit {
            command,
            code: exit_code,
            stdout,
            stderr,
        });
    }

    Ok(GitCommandOutput {
        stdout,
        stderr,
        exit_code,
    })
}

fn render_command(program: &str, args: &[&str]) -> String {
    if args.is_empty() {
        return program.to_string();
    }
    format!("{program} {}", args.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn timeout_path_is_reported() {
        let tmp = tempfile::tempdir().unwrap();

        let err = run_git_program_with_timeout(
            "sh",
            &["-c", "sleep 2"],
            tmp.path(),
            Duration::from_millis(50),
        )
        .await
        .expect_err("fake git should time out");

        assert!(matches!(err, GitExecError::TimedOut { .. }));
    }
}
