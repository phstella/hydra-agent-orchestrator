use std::path::Path;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{ScoringConfig, ScoringProfile};

#[derive(Debug, Error)]
pub enum BaselineError {
    #[error("command timed out after {seconds}s: {command}")]
    TimedOut { command: String, seconds: u64 },

    #[error("I/O error running baseline command: {0}")]
    Io(#[from] std::io::Error),
}

/// Result of running a single shell command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub command: String,
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Parsed test output with pass/fail counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub command_result: CommandResult,
    pub passed: u32,
    pub failed: u32,
    pub total: u32,
}

/// Parsed lint output with error/warning counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub command_result: CommandResult,
    pub errors: u32,
    pub warnings: u32,
}

/// Aggregated baseline capture for build/test/lint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineResult {
    pub build: Option<CommandResult>,
    pub test: Option<TestResult>,
    pub lint: Option<LintResult>,
}

/// Resolve commands from profile + explicit overrides.
pub fn resolve_commands(config: &ScoringConfig) -> ResolvedCommands {
    let profile_cmds = config.profile.map(profile_defaults);
    let cmds = &config.commands;

    ResolvedCommands {
        build: cmds
            .build
            .clone()
            .or_else(|| profile_cmds.as_ref().map(|p| p.build.clone())),
        test: cmds
            .test
            .clone()
            .or_else(|| profile_cmds.as_ref().map(|p| p.test.clone())),
        lint: cmds
            .lint
            .clone()
            .or_else(|| profile_cmds.as_ref().and_then(|p| p.lint.clone())),
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedCommands {
    pub build: Option<String>,
    pub test: Option<String>,
    pub lint: Option<String>,
}

fn profile_defaults(profile: ScoringProfile) -> ProfileCommands {
    match profile {
        ScoringProfile::Rust => ProfileCommands {
            build: "cargo build --all-targets".to_string(),
            test: "cargo test".to_string(),
            lint: Some("cargo clippy --all-targets -- -D warnings".to_string()),
        },
        ScoringProfile::JsNode => ProfileCommands {
            build: "npm run build".to_string(),
            test: "npm test".to_string(),
            lint: Some("npm run lint".to_string()),
        },
        ScoringProfile::Python => ProfileCommands {
            build: "true".to_string(),
            test: "pytest -q".to_string(),
            lint: Some("ruff check .".to_string()),
        },
    }
}

struct ProfileCommands {
    build: String,
    test: String,
    lint: Option<String>,
}

/// Run a shell command with a timeout, returning the result.
pub async fn run_command(
    command: &str,
    cwd: &Path,
    timeout_seconds: u64,
) -> Result<CommandResult, BaselineError> {
    let start = Instant::now();

    let child = tokio::process::Command::new("sh")
        .args(["-c", command])
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let timeout = Duration::from_secs(timeout_seconds);
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let duration = start.elapsed();
            let exit_code = output.status.code().unwrap_or(-1);
            Ok(CommandResult {
                command: command.to_string(),
                success: output.status.success(),
                exit_code,
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                duration_ms: duration.as_millis() as u64,
            })
        }
        Ok(Err(e)) => Err(BaselineError::Io(e)),
        Err(_) => Err(BaselineError::TimedOut {
            command: command.to_string(),
            seconds: timeout_seconds,
        }),
    }
}

/// Parse test output to extract pass/fail/total counts.
/// Supports common patterns from cargo test, pytest, jest/mocha.
pub fn parse_test_output(result: &CommandResult) -> TestResult {
    let combined = format!("{}\n{}", result.stdout, result.stderr);
    let (passed, failed, total) =
        parse_test_counts(&combined).unwrap_or(if result.success { (1, 0, 1) } else { (0, 1, 1) });

    TestResult {
        command_result: result.clone(),
        passed,
        failed,
        total,
    }
}

fn parse_test_counts(output: &str) -> Option<(u32, u32, u32)> {
    // cargo test: "test result: ok. X passed; Y failed; Z ignored; ..."
    let cargo_re = regex::Regex::new(r"test result:.*?(\d+)\s+passed;\s+(\d+)\s+failed").ok()?;
    if let Some(caps) = cargo_re.captures(output) {
        let passed: u32 = caps[1].parse().ok()?;
        let failed: u32 = caps[2].parse().ok()?;
        return Some((passed, failed, passed + failed));
    }

    // pytest: "X passed, Y failed" or "X passed"
    let pytest_re = regex::Regex::new(r"(\d+)\s+passed(?:,\s+(\d+)\s+failed)?").ok()?;
    if let Some(caps) = pytest_re.captures(output) {
        let passed: u32 = caps[1].parse().ok()?;
        let failed: u32 = caps
            .get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);
        return Some((passed, failed, passed + failed));
    }

    // jest/mocha: "Tests: X passed, Y failed, Z total"
    let jest_re =
        regex::Regex::new(r"Tests:\s+(\d+)\s+passed(?:,\s+(\d+)\s+failed)?,\s+(\d+)\s+total")
            .ok()?;
    if let Some(caps) = jest_re.captures(output) {
        let passed: u32 = caps[1].parse().ok()?;
        let failed: u32 = caps
            .get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(0);
        let total: u32 = caps[3].parse().ok()?;
        return Some((passed, failed, total));
    }

    None
}

/// Parse lint output to extract error/warning counts.
pub fn parse_lint_output(result: &CommandResult) -> LintResult {
    let combined = format!("{}\n{}", result.stdout, result.stderr);
    let (errors, warnings) =
        parse_lint_counts(&combined).unwrap_or(if result.success { (0, 0) } else { (1, 0) });

    LintResult {
        command_result: result.clone(),
        errors,
        warnings,
    }
}

fn parse_lint_counts(output: &str) -> Option<(u32, u32)> {
    // clippy: "warning: ... generated X warning(s)" / "error[...]"
    let error_count = regex::Regex::new(r"(?m)^error")
        .ok()
        .map(|re| re.find_iter(output).count() as u32)
        .unwrap_or(0);
    let warning_count = regex::Regex::new(r"(?m)^warning")
        .ok()
        .map(|re| re.find_iter(output).count() as u32)
        .unwrap_or(0);

    // eslint: "X problems (Y errors, Z warnings)"
    if let Some(caps) = regex::Regex::new(r"(\d+)\s+errors?,\s+(\d+)\s+warnings?")
        .ok()
        .and_then(|re| re.captures(output))
    {
        let errors: u32 = caps[1].parse().ok()?;
        let warnings: u32 = caps[2].parse().ok()?;
        return Some((errors, warnings));
    }

    if error_count > 0 || warning_count > 0 {
        return Some((error_count, warning_count));
    }

    None
}

/// Capture baseline build/test/lint on the given working directory.
pub async fn capture_baseline(
    cwd: &Path,
    config: &ScoringConfig,
) -> Result<BaselineResult, BaselineError> {
    let commands = resolve_commands(config);
    let timeout = config.timeout_per_check_seconds;

    let build = match &commands.build {
        Some(cmd) => {
            tracing::info!(command = cmd, "capturing baseline build");
            Some(run_command(cmd, cwd, timeout).await?)
        }
        None => None,
    };

    let test = match &commands.test {
        Some(cmd) => {
            tracing::info!(command = cmd, "capturing baseline tests");
            let result = run_command(cmd, cwd, timeout).await?;
            Some(parse_test_output(&result))
        }
        None => None,
    };

    let lint = match &commands.lint {
        Some(cmd) => {
            tracing::info!(command = cmd, "capturing baseline lint");
            let result = run_command(cmd, cwd, timeout).await?;
            Some(parse_lint_output(&result))
        }
        None => None,
    };

    Ok(BaselineResult { build, test, lint })
}

/// Persist baseline results as a JSON artifact.
pub fn persist_baseline(result: &BaselineResult, path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(result).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScoringConfig;

    #[test]
    fn resolve_commands_from_rust_profile() {
        let config = ScoringConfig {
            profile: Some(ScoringProfile::Rust),
            ..ScoringConfig::default()
        };
        let cmds = resolve_commands(&config);
        assert_eq!(cmds.build.as_deref(), Some("cargo build --all-targets"));
        assert_eq!(cmds.test.as_deref(), Some("cargo test"));
        assert!(cmds.lint.as_deref().unwrap().contains("clippy"));
    }

    #[test]
    fn resolve_commands_from_js_node_profile() {
        let config = ScoringConfig {
            profile: Some(ScoringProfile::JsNode),
            ..ScoringConfig::default()
        };
        let cmds = resolve_commands(&config);
        assert_eq!(cmds.build.as_deref(), Some("npm run build"));
        assert_eq!(cmds.test.as_deref(), Some("npm test"));
        assert_eq!(cmds.lint.as_deref(), Some("npm run lint"));
    }

    #[test]
    fn resolve_commands_from_python_profile() {
        let config = ScoringConfig {
            profile: Some(ScoringProfile::Python),
            ..ScoringConfig::default()
        };
        let cmds = resolve_commands(&config);
        assert_eq!(cmds.build.as_deref(), Some("true"));
        assert_eq!(cmds.test.as_deref(), Some("pytest -q"));
        assert_eq!(cmds.lint.as_deref(), Some("ruff check ."));
    }

    #[test]
    fn explicit_commands_override_profile() {
        let config = ScoringConfig {
            profile: Some(ScoringProfile::Rust),
            commands: crate::config::CommandsConfig {
                build: Some("make build".to_string()),
                ..Default::default()
            },
            ..ScoringConfig::default()
        };
        let cmds = resolve_commands(&config);
        assert_eq!(cmds.build.as_deref(), Some("make build"));
        assert_eq!(cmds.test.as_deref(), Some("cargo test"));
    }

    #[test]
    fn no_profile_no_commands_returns_none() {
        let config = ScoringConfig::default();
        let cmds = resolve_commands(&config);
        assert!(cmds.build.is_none());
        assert!(cmds.test.is_none());
        assert!(cmds.lint.is_none());
    }

    #[test]
    fn parse_cargo_test_output() {
        let result = CommandResult {
            command: "cargo test".to_string(),
            success: true,
            exit_code: 0,
            stdout: "test result: ok. 42 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out\n"
                .to_string(),
            stderr: String::new(),
            duration_ms: 100,
        };
        let tr = parse_test_output(&result);
        assert_eq!(tr.passed, 42);
        assert_eq!(tr.failed, 3);
        assert_eq!(tr.total, 45);
    }

    #[test]
    fn parse_pytest_output() {
        let result = CommandResult {
            command: "pytest".to_string(),
            success: true,
            exit_code: 0,
            stdout: "===== 15 passed, 2 failed in 1.23s =====\n".to_string(),
            stderr: String::new(),
            duration_ms: 100,
        };
        let tr = parse_test_output(&result);
        assert_eq!(tr.passed, 15);
        assert_eq!(tr.failed, 2);
        assert_eq!(tr.total, 17);
    }

    #[test]
    fn parse_test_output_fallback_to_exit_code() {
        let result = CommandResult {
            command: "run-tests".to_string(),
            success: false,
            exit_code: 1,
            stdout: "some random output".to_string(),
            stderr: String::new(),
            duration_ms: 100,
        };
        let tr = parse_test_output(&result);
        assert_eq!(tr.passed, 0);
        assert_eq!(tr.failed, 1);
        assert_eq!(tr.total, 1);
    }

    #[test]
    fn parse_lint_counts_clippy_format() {
        let output = "warning: unused variable\nwarning: unused import\nerror: mismatched types\n";
        let (errors, warnings) = parse_lint_counts(output).unwrap();
        assert_eq!(errors, 1);
        assert_eq!(warnings, 2);
    }

    #[test]
    fn parse_lint_counts_eslint_format() {
        let output = "✖ 15 problems (3 errors, 12 warnings)\n";
        let (errors, warnings) = parse_lint_counts(output).unwrap();
        assert_eq!(errors, 3);
        assert_eq!(warnings, 12);
    }

    #[test]
    fn parse_lint_counts_clean_returns_none() {
        let output = "All checks passed!\n";
        assert!(parse_lint_counts(output).is_none());
    }

    #[tokio::test]
    async fn run_command_echo_succeeds() {
        let result = run_command("echo hello", std::env::temp_dir().as_path(), 10)
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn run_command_failure_captured() {
        let result = run_command("exit 42", std::env::temp_dir().as_path(), 10)
            .await
            .unwrap();
        assert!(!result.success);
        assert_eq!(result.exit_code, 42);
    }

    #[tokio::test]
    async fn run_command_timeout() {
        let err = run_command("sleep 60", std::env::temp_dir().as_path(), 1)
            .await
            .unwrap_err();
        assert!(matches!(err, BaselineError::TimedOut { .. }));
    }

    #[tokio::test]
    async fn capture_baseline_with_no_config_returns_all_none() {
        let config = ScoringConfig::default();
        let result = capture_baseline(std::env::temp_dir().as_path(), &config)
            .await
            .unwrap();
        assert!(result.build.is_none());
        assert!(result.test.is_none());
        assert!(result.lint.is_none());
    }

    #[tokio::test]
    async fn capture_baseline_with_commands() {
        let config = ScoringConfig {
            commands: crate::config::CommandsConfig {
                build: Some("echo build-ok".to_string()),
                test: Some("echo 'test result: ok. 5 passed; 0 failed; 0 ignored'".to_string()),
                lint: Some("echo lint-clean".to_string()),
            },
            ..ScoringConfig::default()
        };

        let result = capture_baseline(std::env::temp_dir().as_path(), &config)
            .await
            .unwrap();
        assert!(result.build.as_ref().unwrap().success);
        assert_eq!(result.test.as_ref().unwrap().passed, 5);
        assert_eq!(result.lint.as_ref().unwrap().errors, 0);
    }

    #[test]
    fn persist_and_read_baseline() {
        let result = BaselineResult {
            build: Some(CommandResult {
                command: "echo ok".to_string(),
                success: true,
                exit_code: 0,
                stdout: "ok\n".to_string(),
                stderr: String::new(),
                duration_ms: 10,
            }),
            test: None,
            lint: None,
        };
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("baseline.json");
        persist_baseline(&result, &path).unwrap();
        let data = std::fs::read_to_string(&path).unwrap();
        let loaded: BaselineResult = serde_json::from_str(&data).unwrap();
        assert!(loaded.build.unwrap().success);
    }
}
