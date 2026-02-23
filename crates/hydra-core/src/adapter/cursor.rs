use tracing::{debug, warn};

use super::probe::{
    which_binary, AdapterProbe, AdapterTier, CapabilitySet, CommandRunner, Confidence, ProbeResult,
    ProbeStatus,
};
use super::runtime::{AdapterRuntime, AgentEvent, AgentEventType, SpawnRequest};
use crate::supervisor::AgentCommand;

/// Cursor agent adapter (Experimental).
///
/// Stub runtime implementation. The Cursor adapter is experimental and
/// its CLI output format is not yet stabilized.
pub struct CursorAdapter;

impl AdapterRuntime for CursorAdapter {
    fn build_command(&self, req: &SpawnRequest) -> AgentCommand {
        let mut args = vec![req.task_prompt.clone()];

        if req.output_json_stream {
            args.push("--json".to_string());
        }

        AgentCommand {
            program: "cursor-agent".to_string(),
            args,
            env: vec![],
            cwd: req.worktree_path.clone(),
        }
    }

    fn parse_line(&self, line: &str) -> Option<AgentEvent> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Best-effort: attempt JSON parse, fall back to plain text message.
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(value) => {
                let event_type = match value.get("type").and_then(|t| t.as_str()) {
                    Some("message") => AgentEventType::Message,
                    Some("tool_call") => AgentEventType::ToolCall,
                    Some("tool_result") => AgentEventType::ToolResult,
                    Some("completed") | Some("done") => AgentEventType::Completed,
                    Some("error") => AgentEventType::Failed,
                    _ => AgentEventType::Unknown,
                };
                Some(AgentEvent {
                    event_type,
                    data: value,
                    raw_line: Some(line.to_string()),
                })
            }
            Err(_) => Some(AgentEvent {
                event_type: AgentEventType::Message,
                data: serde_json::json!({"text": trimmed}),
                raw_line: Some(line.to_string()),
            }),
        }
    }

    fn parse_raw(&self, chunk: &[u8]) -> Vec<AgentEvent> {
        String::from_utf8_lossy(chunk)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| self.parse_line(line))
            .collect()
    }
}

/// Binary names to try when searching for cursor, in priority order.
const CURSOR_CANDIDATES: &[&str] = &["cursor-agent", "cursor"];

/// Probe for the Cursor agent adapter (Experimental).
///
/// This adapter is ALWAYS classified as Experimental and will never be
/// promoted to Tier1, regardless of detected capabilities.
pub struct CursorProbe<'a> {
    runner: &'a dyn CommandRunner,
    /// Optional user-configured binary path. Tried first before PATH search.
    configured_path: Option<String>,
}

impl<'a> CursorProbe<'a> {
    pub fn new(runner: &'a dyn CommandRunner) -> Self {
        Self {
            runner,
            configured_path: None,
        }
    }

    pub fn with_configured_path(mut self, path: Option<String>) -> Self {
        self.configured_path = path;
        self
    }

    /// Discover the cursor binary. Tries (in order):
    /// 1. User-configured path
    /// 2. `cursor-agent` in PATH
    /// 3. `cursor` in PATH
    fn discover_binary(&self) -> Option<(String, std::path::PathBuf)> {
        // Try configured path first
        if let Some(ref configured) = self.configured_path {
            if let Ok(output) = self.runner.run(configured, &["--help"]) {
                if output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty()
                {
                    debug!(path = %configured, "using configured cursor path");
                    return Some((configured.clone(), std::path::PathBuf::from(configured)));
                }
            }
        }

        // Try candidates in PATH
        for candidate in CURSOR_CANDIDATES {
            if let Some(path) = which_binary(self.runner, candidate) {
                debug!(binary = %candidate, path = %path.display(), "found cursor binary");
                return Some(((*candidate).to_string(), path));
            }
        }

        None
    }
}

impl AdapterProbe for CursorProbe<'_> {
    fn key(&self) -> &'static str {
        "cursor-agent"
    }

    fn tier(&self) -> AdapterTier {
        AdapterTier::Experimental
    }

    fn probe(&self) -> ProbeResult {
        let (binary_name, binary_path) = match self.discover_binary() {
            Some(found) => found,
            None => {
                return ProbeResult {
                    adapter_key: self.key().to_string(),
                    tier: self.tier(),
                    status: ProbeStatus::Missing,
                    binary_path: None,
                    version: None,
                    capabilities: CapabilitySet::default(),
                    confidence: Confidence::Unknown,
                    message: Some("cursor-agent/cursor binary not found in PATH".to_string()),
                };
            }
        };

        // Attempt version detection
        let version = self
            .runner
            .run(&binary_name, &["--version"])
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        // Check help output for capability detection
        let help_output = match self.runner.run(&binary_name, &["--help"]) {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                format!("{stdout}\n{stderr}")
            }
            Err(e) => {
                warn!(error = %e, "failed to run cursor --help");
                return ProbeResult {
                    adapter_key: self.key().to_string(),
                    tier: self.tier(),
                    status: ProbeStatus::ExperimentalBlocked,
                    binary_path: Some(binary_path),
                    version,
                    capabilities: CapabilitySet::default(),
                    confidence: Confidence::Unknown,
                    message: Some(format!("failed to run --help: {e}")),
                };
            }
        };

        let has_json = help_output.contains("--json") || help_output.contains("json");
        let has_edit = help_output.contains("--edit") || help_output.contains("edit mode");

        let capabilities = CapabilitySet {
            plain_text: true,
            json_stream: has_json,
            force_edit_mode: has_edit,
            ..Default::default()
        };

        // Cursor is always experimental - determine ready vs blocked
        let (status, confidence) = if help_output.is_empty() {
            (ProbeStatus::ExperimentalBlocked, Confidence::Unknown)
        } else {
            (ProbeStatus::ExperimentalReady, Confidence::Observed)
        };

        ProbeResult {
            adapter_key: self.key().to_string(),
            tier: self.tier(),
            status,
            binary_path: Some(binary_path),
            version,
            capabilities,
            confidence,
            message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::probe::CommandRunner;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};

    struct MockRunner {
        responses: std::collections::HashMap<String, std::io::Result<Output>>,
    }

    impl MockRunner {
        fn new() -> Self {
            Self {
                responses: std::collections::HashMap::new(),
            }
        }

        fn register(&mut self, cmd: &str, result: std::io::Result<Output>) {
            self.responses.insert(cmd.to_string(), result);
        }
    }

    impl CommandRunner for MockRunner {
        fn run(&self, program: &str, args: &[&str]) -> std::io::Result<Output> {
            let key = format!("{program} {}", args.join(" "));
            match self.responses.get(&key) {
                Some(Ok(output)) => Ok(output.clone()),
                Some(Err(_)) => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "mock error",
                )),
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("no mock for: {key}"),
                )),
            }
        }
    }

    fn success_output(stdout: &str) -> Output {
        Output {
            status: ExitStatus::from_raw(0),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    #[test]
    fn cursor_missing_all_binaries() {
        let mock = MockRunner::new();
        // No binaries registered, all will fail
        let probe = CursorProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::Missing);
        assert_eq!(result.tier, AdapterTier::Experimental);
    }

    #[test]
    fn cursor_found_via_cursor_agent() {
        let mut mock = MockRunner::new();
        mock.register(
            "which cursor-agent",
            Ok(success_output("/usr/bin/cursor-agent\n")),
        );
        mock.register(
            "cursor-agent --version",
            Ok(success_output("cursor-agent 0.3.0\n")),
        );
        mock.register(
            "cursor-agent --help",
            Ok(success_output(
                "Usage: cursor-agent [options]\n  --json  JSON output\n  --edit  Edit mode\n",
            )),
        );
        let probe = CursorProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::ExperimentalReady);
        assert_eq!(result.tier, AdapterTier::Experimental);
        assert!(result.capabilities.json_stream);
        assert!(result.capabilities.force_edit_mode);
        assert!(result.version.unwrap().contains("0.3.0"));
    }

    #[test]
    fn cursor_found_via_cursor_fallback() {
        let mut mock = MockRunner::new();
        // cursor-agent not found, but cursor is
        mock.register(
            "which cursor-agent",
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            )),
        );
        mock.register("which cursor", Ok(success_output("/usr/bin/cursor\n")));
        mock.register("cursor --version", Ok(success_output("cursor 1.0.0\n")));
        mock.register(
            "cursor --help",
            Ok(success_output("Usage: cursor [options]\n  --verbose\n")),
        );
        let probe = CursorProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::ExperimentalReady);
        assert_eq!(result.tier, AdapterTier::Experimental);
        // No json detected in help
        assert!(!result.capabilities.json_stream);
    }

    #[test]
    fn cursor_configured_path() {
        let mut mock = MockRunner::new();
        mock.register(
            "/opt/cursor/bin/cursor-agent --help",
            Ok(success_output("Usage: cursor-agent\n  --json\n")),
        );
        mock.register(
            "/opt/cursor/bin/cursor-agent --version",
            Ok(success_output("cursor-agent 0.2.0\n")),
        );
        let probe = CursorProbe::new(&mock)
            .with_configured_path(Some("/opt/cursor/bin/cursor-agent".into()));
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::ExperimentalReady);
        assert!(result.capabilities.json_stream);
    }

    #[test]
    fn cursor_never_returns_tier1() {
        let mut mock = MockRunner::new();
        mock.register(
            "which cursor-agent",
            Ok(success_output("/usr/bin/cursor-agent\n")),
        );
        mock.register(
            "cursor-agent --version",
            Ok(success_output("cursor-agent 99.0.0\n")),
        );
        mock.register(
            "cursor-agent --help",
            Ok(success_output(
                "Usage: cursor-agent [options]\n  --json\n  --edit\n  --sandbox\n  --everything\n",
            )),
        );
        let probe = CursorProbe::new(&mock);
        let result = probe.probe();

        // Even with all capabilities, must remain Experimental
        assert_eq!(result.tier, AdapterTier::Experimental);
        assert_ne!(result.status, ProbeStatus::Ready);
        assert!(
            result.status == ProbeStatus::ExperimentalReady
                || result.status == ProbeStatus::ExperimentalBlocked
        );
    }

    #[test]
    fn cursor_unknown_help_fields_dont_crash() {
        let mut mock = MockRunner::new();
        mock.register(
            "which cursor-agent",
            Ok(success_output("/usr/bin/cursor-agent\n")),
        );
        mock.register(
            "cursor-agent --version",
            Ok(success_output("cursor-agent 1.0.0\n")),
        );
        mock.register(
            "cursor-agent --help",
            Ok(success_output(
                "Usage: cursor-agent\n  --weird-flag  Weird\n  --another  Another\n  --json  JSON\n",
            )),
        );
        let probe = CursorProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::ExperimentalReady);
    }
}
