use std::path::Path;
use std::time::Duration;

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::{HydraError, Result};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Result of running baseline capture commands against the base ref.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineResult {
    pub build: Option<CommandResult>,
    pub test: Option<TestResult>,
    pub lint: Option<LintResult>,
}

/// Raw result of executing a shell command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub exit_code: i32,
    pub duration: Duration,
    pub stdout: String,
    pub stderr: String,
}

/// Structured test execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub passed: u32,
    pub failed: u32,
    pub total: u32,
    pub raw_output: String,
}

/// Structured lint execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub errors: u32,
    pub warnings: u32,
    pub raw_output: String,
}

// ---------------------------------------------------------------------------
// Command execution
// ---------------------------------------------------------------------------

/// Run a single shell command in `working_dir` with a timeout.
///
/// Returns `Ok(CommandResult)` even when the command fails (non-zero exit);
/// returns `Err` only for OS-level spawn failures.
async fn run_command(
    working_dir: &Path,
    command: &str,
    timeout: Duration,
) -> Result<CommandResult> {
    let start = std::time::Instant::now();

    let child = tokio::process::Command::new("sh")
        .args(["-c", command])
        .current_dir(working_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| HydraError::Scoring(format!("failed to spawn command '{command}': {e}")))?;

    // wait_with_output consumes the child, so we can't kill it on timeout.
    // Instead, wrap the whole thing in a timeout and use `id()` to kill via signal.
    let child_id = child.id();
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let duration = start.elapsed();
            let exit_code = output.status.code().unwrap_or(-1);
            Ok(CommandResult {
                success: output.status.success(),
                exit_code,
                duration,
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        }
        Ok(Err(e)) => Err(HydraError::Scoring(format!(
            "command '{command}' failed: {e}"
        ))),
        Err(_) => {
            // Timeout -- attempt to kill via pid
            if let Some(pid) = child_id {
                #[cfg(unix)]
                {
                    let _ = nix::sys::signal::kill(
                        nix::unistd::Pid::from_raw(pid as i32),
                        nix::sys::signal::Signal::SIGKILL,
                    );
                }
                #[cfg(not(unix))]
                {
                    let _ = pid; // suppress unused warning
                }
            }
            let duration = start.elapsed();
            warn!(command, ?timeout, "command timed out");
            Ok(CommandResult {
                success: false,
                exit_code: -1,
                duration,
                stdout: String::new(),
                stderr: format!("TIMEOUT after {timeout:?}"),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Output parsers
// ---------------------------------------------------------------------------

/// Parse test output into a `TestResult`.
///
/// Tries several common formats (cargo test, pytest, jest/mocha).
/// Falls back to exit-code heuristic if no pattern matches.
fn parse_test_output(cmd_result: &CommandResult) -> TestResult {
    let combined = format!("{}\n{}", cmd_result.stdout, cmd_result.stderr);

    // Cargo test: "test result: ok. 12 passed; 1 failed; 0 ignored; ..."
    if let Some((passed, failed)) = parse_cargo_test(&combined) {
        return TestResult {
            passed,
            failed,
            total: passed + failed,
            raw_output: combined,
        };
    }

    // Pytest: "5 passed, 2 failed" or "5 passed"
    if let Some((passed, failed)) = parse_pytest(&combined) {
        return TestResult {
            passed,
            failed,
            total: passed + failed,
            raw_output: combined,
        };
    }

    // Jest/Mocha: "Tests: N passed, M failed, T total"
    if let Some((passed, failed, total)) = parse_jest(&combined) {
        return TestResult {
            passed,
            failed,
            total,
            raw_output: combined,
        };
    }

    // Fallback: exit code
    debug!("no test parser matched, using exit-code fallback");
    if cmd_result.success {
        TestResult {
            passed: 1,
            failed: 0,
            total: 1,
            raw_output: combined,
        }
    } else {
        TestResult {
            passed: 0,
            failed: 1,
            total: 1,
            raw_output: combined,
        }
    }
}

fn parse_cargo_test(output: &str) -> Option<(u32, u32)> {
    let re = Regex::new(r"test result: \S+\.\s+(\d+) passed;\s+(\d+) failed").ok()?;
    let caps = re.captures(output)?;
    let passed: u32 = caps[1].parse().ok()?;
    let failed: u32 = caps[2].parse().ok()?;
    Some((passed, failed))
}

fn parse_pytest(output: &str) -> Option<(u32, u32)> {
    // "5 passed, 2 failed" or "5 passed" (no failures)
    let passed_re = Regex::new(r"(\d+) passed").ok()?;
    let failed_re = Regex::new(r"(\d+) failed").ok()?;
    let passed: u32 = passed_re.captures(output)?.get(1)?.as_str().parse().ok()?;
    let failed: u32 = failed_re
        .captures(output)
        .and_then(|c| c.get(1)?.as_str().parse().ok())
        .unwrap_or(0);
    Some((passed, failed))
}

fn parse_jest(output: &str) -> Option<(u32, u32, u32)> {
    // "Tests:  2 failed, 5 passed, 7 total" or "Tests:  5 passed, 5 total"
    let re = Regex::new(r"Tests:\s+(?:(\d+) failed,\s+)?(\d+) passed,\s+(\d+) total").ok()?;
    let caps = re.captures(output)?;
    let failed: u32 = caps
        .get(1)
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(0);
    let passed: u32 = caps[2].parse().ok()?;
    let total: u32 = caps[3].parse().ok()?;
    Some((passed, failed, total))
}

/// Parse lint output into a `LintResult`.
///
/// Counts occurrences of "error" and "warning" patterns. Falls back to
/// exit-code heuristic.
fn parse_lint_output(cmd_result: &CommandResult) -> LintResult {
    let combined = format!("{}\n{}", cmd_result.stdout, cmd_result.stderr);

    // Clippy/rustc: "error[E0123]:" or "error:" patterns
    let error_re = Regex::new(r"(?mi)^error(\[E\d+\])?:").unwrap();
    let warning_re = Regex::new(r"(?mi)^warning(\[[\w\d]+\])?:").unwrap();

    let errors = error_re.find_iter(&combined).count() as u32;
    let warnings = warning_re.find_iter(&combined).count() as u32;

    if errors > 0 || warnings > 0 {
        return LintResult {
            errors,
            warnings,
            raw_output: combined,
        };
    }

    // ESLint/ruff style: "N errors and M warnings" or "N problems (X errors, Y warnings)"
    if let Some((e, w)) = parse_eslint_summary(&combined) {
        return LintResult {
            errors: e,
            warnings: w,
            raw_output: combined,
        };
    }

    // Fallback: exit code
    if cmd_result.success {
        LintResult {
            errors: 0,
            warnings: 0,
            raw_output: combined,
        }
    } else {
        LintResult {
            errors: 1,
            warnings: 0,
            raw_output: combined,
        }
    }
}

fn parse_eslint_summary(output: &str) -> Option<(u32, u32)> {
    // "X problems (E errors, W warnings)"
    let re = Regex::new(r"(\d+) problems?\s*\((\d+) errors?,\s*(\d+) warnings?\)").ok()?;
    if let Some(caps) = re.captures(output) {
        let errors: u32 = caps[2].parse().ok()?;
        let warnings: u32 = caps[3].parse().ok()?;
        return Some((errors, warnings));
    }
    None
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Capture baseline metrics by running build/test/lint on the base ref.
///
/// Each command is optional. Missing commands produce `None` for that field.
/// Timeouts are handled gracefully (marked as failed).
pub async fn capture_baseline(
    working_dir: &Path,
    build_cmd: Option<&str>,
    test_cmd: Option<&str>,
    lint_cmd: Option<&str>,
    timeout: Duration,
) -> Result<BaselineResult> {
    let build = match build_cmd {
        Some(cmd) => {
            debug!(cmd, "running baseline build");
            let result = run_command(working_dir, cmd, timeout).await?;
            Some(result)
        }
        None => None,
    };

    let test = match test_cmd {
        Some(cmd) => {
            debug!(cmd, "running baseline tests");
            let cmd_result = run_command(working_dir, cmd, timeout).await?;
            Some(parse_test_output(&cmd_result))
        }
        None => None,
    };

    let lint = match lint_cmd {
        Some(cmd) => {
            debug!(cmd, "running baseline lint");
            let cmd_result = run_command(working_dir, cmd, timeout).await?;
            Some(parse_lint_output(&cmd_result))
        }
        None => None,
    };

    Ok(BaselineResult { build, test, lint })
}

/// Run a single command and return structured test results.
///
/// Used for capturing agent test output (same parsing as baseline).
pub async fn run_and_parse_tests(
    working_dir: &Path,
    cmd: &str,
    timeout: Duration,
) -> Result<TestResult> {
    let cmd_result = run_command(working_dir, cmd, timeout).await?;
    Ok(parse_test_output(&cmd_result))
}

/// Run a single command and return structured lint results.
pub async fn run_and_parse_lint(
    working_dir: &Path,
    cmd: &str,
    timeout: Duration,
) -> Result<LintResult> {
    let cmd_result = run_command(working_dir, cmd, timeout).await?;
    Ok(parse_lint_output(&cmd_result))
}

/// Run a single command and return the raw `CommandResult`.
pub async fn run_and_capture(
    working_dir: &Path,
    cmd: &str,
    timeout: Duration,
) -> Result<CommandResult> {
    run_command(working_dir, cmd, timeout).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test_parsers {
    use super::*;

    #[test]
    fn cargo_test_output() {
        let output = "running 15 tests\n...\ntest result: ok. 14 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out";
        let (p, f) = parse_cargo_test(output).unwrap();
        assert_eq!(p, 14);
        assert_eq!(f, 1);
    }

    #[test]
    fn pytest_output_with_failures() {
        let output = "======== 5 passed, 2 failed in 1.23s ========";
        let (p, f) = parse_pytest(output).unwrap();
        assert_eq!(p, 5);
        assert_eq!(f, 2);
    }

    #[test]
    fn pytest_output_no_failures() {
        let output = "======== 10 passed in 0.50s ========";
        let (p, f) = parse_pytest(output).unwrap();
        assert_eq!(p, 10);
        assert_eq!(f, 0);
    }

    #[test]
    fn jest_output_with_failures() {
        let output = "Tests:  2 failed, 5 passed, 7 total";
        let (p, f, t) = parse_jest(output).unwrap();
        assert_eq!(p, 5);
        assert_eq!(f, 2);
        assert_eq!(t, 7);
    }

    #[test]
    fn jest_output_no_failures() {
        let output = "Tests:  5 passed, 5 total";
        let (p, f, t) = parse_jest(output).unwrap();
        assert_eq!(p, 5);
        assert_eq!(f, 0);
        assert_eq!(t, 5);
    }

    #[test]
    fn lint_clippy_errors() {
        let output = "error[E0308]: mismatched types\nerror: aborting due to previous error\nwarning: unused variable\n";
        let cmd = CommandResult {
            success: false,
            exit_code: 1,
            duration: Duration::from_secs(1),
            stdout: output.to_string(),
            stderr: String::new(),
        };
        let result = parse_lint_output(&cmd);
        assert_eq!(result.errors, 2);
        assert_eq!(result.warnings, 1);
    }

    #[test]
    fn lint_eslint_summary() {
        let output = "10 problems (3 errors, 7 warnings)";
        let (e, w) = parse_eslint_summary(output).unwrap();
        assert_eq!(e, 3);
        assert_eq!(w, 7);
    }

    #[test]
    fn lint_clean_output() {
        let cmd = CommandResult {
            success: true,
            exit_code: 0,
            duration: Duration::from_secs(1),
            stdout: String::new(),
            stderr: String::new(),
        };
        let result = parse_lint_output(&cmd);
        assert_eq!(result.errors, 0);
        assert_eq!(result.warnings, 0);
    }

    #[test]
    fn test_exit_code_fallback_success() {
        let cmd = CommandResult {
            success: true,
            exit_code: 0,
            duration: Duration::from_secs(1),
            stdout: "all good".to_string(),
            stderr: String::new(),
        };
        let result = parse_test_output(&cmd);
        assert_eq!(result.passed, 1);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_exit_code_fallback_failure() {
        let cmd = CommandResult {
            success: false,
            exit_code: 1,
            duration: Duration::from_secs(1),
            stdout: "something broke".to_string(),
            stderr: String::new(),
        };
        let result = parse_test_output(&cmd);
        assert_eq!(result.passed, 0);
        assert_eq!(result.failed, 1);
    }

    #[tokio::test]
    async fn capture_baseline_missing_commands() {
        let result = capture_baseline(Path::new("/tmp"), None, None, None, Duration::from_secs(10))
            .await
            .unwrap();
        assert!(result.build.is_none());
        assert!(result.test.is_none());
        assert!(result.lint.is_none());
    }

    #[tokio::test]
    async fn capture_baseline_echo_command() {
        let result = capture_baseline(
            Path::new("/tmp"),
            Some("echo build-ok"),
            Some("echo 'test result: ok. 3 passed; 0 failed; 0 ignored'"),
            Some("echo lint-clean"),
            Duration::from_secs(10),
        )
        .await
        .unwrap();

        let build = result.build.unwrap();
        assert!(build.success);
        assert_eq!(build.exit_code, 0);

        let test = result.test.unwrap();
        assert_eq!(test.passed, 3);
        assert_eq!(test.failed, 0);

        let lint = result.lint.unwrap();
        assert_eq!(lint.errors, 0);
        assert_eq!(lint.warnings, 0);
    }

    #[tokio::test]
    async fn capture_baseline_timeout() {
        let result = capture_baseline(
            Path::new("/tmp"),
            Some("sleep 10"),
            None,
            None,
            Duration::from_millis(100),
        )
        .await
        .unwrap();

        let build = result.build.unwrap();
        assert!(!build.success);
        assert_eq!(build.exit_code, -1);
        assert!(build.stderr.contains("TIMEOUT"));
    }
}
