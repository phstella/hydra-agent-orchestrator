use tracing::{debug, warn};

use super::probe::{
    missing_result, which_binary, AdapterProbe, AdapterTier, CapabilitySet, CommandRunner,
    Confidence, ProbeResult, ProbeStatus,
};
use super::runtime::{AdapterRuntime, AgentEvent, AgentEventType, SpawnRequest};
use crate::supervisor::AgentCommand;

/// Claude CLI adapter (Tier 1).
///
/// Handles both probing (via [`ClaudeProbe`]) and runtime execution.
pub struct ClaudeAdapter;

impl AdapterRuntime for ClaudeAdapter {
    fn build_command(&self, req: &SpawnRequest) -> AgentCommand {
        let args = vec![
            "-p".to_string(),
            req.task_prompt.clone(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--permission-mode".to_string(),
            "bypassPermissions".to_string(),
        ];

        AgentCommand {
            program: "claude".to_string(),
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

        let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;

        let event_type = match value.get("type").and_then(|t| t.as_str()) {
            Some("assistant") | Some("message") => AgentEventType::Message,
            Some("tool_use") => AgentEventType::ToolCall,
            Some("tool_result") => AgentEventType::ToolResult,
            Some("result") | Some("content_block_stop") => AgentEventType::Completed,
            Some("error") => AgentEventType::Failed,
            Some("usage") | Some("message_delta") => {
                if value.get("usage").is_some() {
                    AgentEventType::Usage
                } else {
                    AgentEventType::Progress
                }
            }
            _ => AgentEventType::Unknown,
        };

        Some(AgentEvent {
            event_type,
            data: value,
            raw_line: Some(line.to_string()),
        })
    }

    fn parse_raw(&self, chunk: &[u8]) -> Vec<AgentEvent> {
        String::from_utf8_lossy(chunk)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                self.parse_line(line).unwrap_or(AgentEvent {
                    event_type: AgentEventType::Message,
                    data: serde_json::json!({"text": line}),
                    raw_line: Some(line.to_string()),
                })
            })
            .collect()
    }
}

/// Probe for the Claude CLI adapter (Tier 1).
pub struct ClaudeProbe<'a> {
    runner: &'a dyn CommandRunner,
}

impl<'a> ClaudeProbe<'a> {
    pub fn new(runner: &'a dyn CommandRunner) -> Self {
        Self { runner }
    }
}

impl AdapterProbe for ClaudeProbe<'_> {
    fn key(&self) -> &'static str {
        "claude"
    }

    fn tier(&self) -> AdapterTier {
        AdapterTier::Tier1
    }

    fn probe(&self) -> ProbeResult {
        let binary_path = match which_binary(self.runner, "claude") {
            Some(p) => p,
            None => return missing_result(self.key(), self.tier()),
        };

        debug!(path = %binary_path.display(), "found claude binary");

        // Get version
        let version = self
            .runner
            .run("claude", &["--version"])
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        // Check help output for required flags
        let help_output = match self.runner.run("claude", &["--help"]) {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                format!("{stdout}\n{stderr}")
            }
            Err(e) => {
                warn!(error = %e, "failed to run claude --help");
                return ProbeResult {
                    adapter_key: self.key().to_string(),
                    tier: self.tier(),
                    status: ProbeStatus::Blocked,
                    binary_path: Some(binary_path),
                    version,
                    capabilities: CapabilitySet::default(),
                    confidence: Confidence::Unknown,
                    message: Some(format!("failed to run --help: {e}")),
                };
            }
        };

        let has_print = help_output.contains("-p") || help_output.contains("--print");
        let has_output_format = help_output.contains("--output-format");

        let has_resume = help_output.contains("--resume");

        let capabilities = CapabilitySet {
            plain_text: has_print,
            json_stream: has_output_format,
            session_resume: has_resume,
            ..Default::default()
        };

        let mut blocked_reasons = Vec::new();
        if !has_print {
            blocked_reasons.push("missing -p/--print flag");
        }
        if !has_output_format {
            blocked_reasons.push("missing --output-format flag");
        }

        let (status, confidence, message) = if blocked_reasons.is_empty() {
            (ProbeStatus::Ready, Confidence::Verified, None)
        } else {
            let msg = blocked_reasons.join("; ");
            warn!(reason = %msg, "claude adapter blocked");
            (ProbeStatus::Blocked, Confidence::Observed, Some(msg))
        };

        ProbeResult {
            adapter_key: self.key().to_string(),
            tier: self.tier(),
            status,
            binary_path: Some(binary_path),
            version,
            capabilities,
            confidence,
            message,
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
    fn claude_missing_binary() {
        let mut mock = MockRunner::new();
        mock.register(
            "which claude",
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            )),
        );
        let probe = ClaudeProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::Missing);
        assert_eq!(result.adapter_key, "claude");
    }

    #[test]
    fn claude_ready() {
        let mut mock = MockRunner::new();
        mock.register("which claude", Ok(success_output("/usr/bin/claude\n")));
        mock.register("claude --version", Ok(success_output("claude 1.2.3\n")));
        mock.register(
            "claude --help",
            Ok(success_output(
                "Usage: claude [options]\n  -p, --print    Print mode\n  --output-format json|text\n  --resume       Resume session\n",
            )),
        );
        let probe = ClaudeProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::Ready);
        assert_eq!(result.tier, AdapterTier::Tier1);
        assert!(result.capabilities.plain_text);
        assert!(result.capabilities.json_stream);
        assert!(result.capabilities.session_resume);
        assert_eq!(result.confidence, Confidence::Verified);
        assert!(result.binary_path.is_some());
        assert!(result.version.unwrap().contains("1.2.3"));
    }

    #[test]
    fn claude_blocked_missing_flags() {
        let mut mock = MockRunner::new();
        mock.register("which claude", Ok(success_output("/usr/bin/claude\n")));
        mock.register("claude --version", Ok(success_output("claude 0.1.0\n")));
        mock.register(
            "claude --help",
            Ok(success_output("Usage: claude [options]\n  --verbose\n")),
        );
        let probe = ClaudeProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::Blocked);
        assert!(result.message.unwrap().contains("missing"));
    }

    #[test]
    fn claude_blocked_partial_flags() {
        let mut mock = MockRunner::new();
        mock.register("which claude", Ok(success_output("/usr/bin/claude\n")));
        mock.register("claude --version", Ok(success_output("claude 1.0.0\n")));
        mock.register(
            "claude --help",
            Ok(success_output(
                "Usage: claude [options]\n  -p, --print    Print mode\n",
            )),
        );
        let probe = ClaudeProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::Blocked);
        let msg = result.message.unwrap();
        assert!(msg.contains("--output-format"));
        assert!(!msg.contains("--print"));
    }

    #[test]
    fn claude_unknown_help_fields_dont_crash() {
        let mut mock = MockRunner::new();
        mock.register("which claude", Ok(success_output("/usr/bin/claude\n")));
        mock.register("claude --version", Ok(success_output("claude 2.0.0\n")));
        mock.register(
            "claude --help",
            Ok(success_output(
                "Usage: claude [options]\n  -p, --print    Print\n  --output-format json\n  --unknown-flag  Something new\n  --another-new  Another\n",
            )),
        );
        let probe = ClaudeProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::Ready);
    }
}
