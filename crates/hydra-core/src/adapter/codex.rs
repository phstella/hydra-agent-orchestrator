use tracing::{debug, warn};

use super::probe::{
    missing_result, which_binary, AdapterProbe, AdapterTier, CapabilitySet, CommandRunner,
    Confidence, ProbeResult, ProbeStatus,
};
use super::runtime::{AdapterRuntime, AgentEvent, AgentEventType, SpawnRequest};
use crate::supervisor::AgentCommand;

/// Codex CLI adapter (Tier 1).
///
/// Handles both probing (via [`CodexProbe`]) and runtime execution.
pub struct CodexAdapter;

impl AdapterRuntime for CodexAdapter {
    fn build_command(&self, req: &SpawnRequest) -> AgentCommand {
        let args = vec![
            "exec".to_string(),
            req.task_prompt.clone(),
            "--json".to_string(),
            "--full-auto".to_string(),
        ];

        AgentCommand {
            program: "codex".to_string(),
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
            Some("message") => AgentEventType::Message,
            Some("function_call") | Some("tool_call") => AgentEventType::ToolCall,
            Some("function_call_output") | Some("tool_result") => AgentEventType::ToolResult,
            Some("completed") | Some("done") => AgentEventType::Completed,
            Some("error") => AgentEventType::Failed,
            _ => {
                if value.get("usage").is_some() {
                    AgentEventType::Usage
                } else {
                    AgentEventType::Unknown
                }
            }
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
            .filter_map(|line| self.parse_line(line))
            .collect()
    }
}

/// Probe for the Codex CLI adapter (Tier 1).
pub struct CodexProbe<'a> {
    runner: &'a dyn CommandRunner,
}

impl<'a> CodexProbe<'a> {
    pub fn new(runner: &'a dyn CommandRunner) -> Self {
        Self { runner }
    }
}

impl AdapterProbe for CodexProbe<'_> {
    fn key(&self) -> &'static str {
        "codex"
    }

    fn tier(&self) -> AdapterTier {
        AdapterTier::Tier1
    }

    fn probe(&self) -> ProbeResult {
        let binary_path = match which_binary(self.runner, "codex") {
            Some(p) => p,
            None => return missing_result(self.key(), self.tier()),
        };

        debug!(path = %binary_path.display(), "found codex binary");

        // Get version
        let version = self
            .runner
            .run("codex", &["--version"])
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        // Check exec subcommand help for --json flag
        let exec_help = match self.runner.run("codex", &["exec", "--help"]) {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                format!("{stdout}\n{stderr}")
            }
            Err(e) => {
                warn!(error = %e, "failed to run codex exec --help");
                return ProbeResult {
                    adapter_key: self.key().to_string(),
                    tier: self.tier(),
                    status: ProbeStatus::Blocked,
                    binary_path: Some(binary_path),
                    version,
                    capabilities: CapabilitySet::default(),
                    confidence: Confidence::Unknown,
                    message: Some(format!("failed to run exec --help: {e}")),
                };
            }
        };

        let has_exec = !exec_help.is_empty();
        let has_json = exec_help.contains("--json");

        let has_sandbox = exec_help.contains("--sandbox");

        let capabilities = CapabilitySet {
            json_stream: has_json,
            plain_text: true,
            sandbox_controls: has_sandbox,
            ..Default::default()
        };

        let mut blocked_reasons = Vec::new();
        if !has_exec {
            blocked_reasons.push("exec subcommand not available");
        }
        if !has_json {
            blocked_reasons.push("missing --json flag on exec subcommand");
        }

        let (status, confidence, message) = if blocked_reasons.is_empty() {
            (ProbeStatus::Ready, Confidence::Verified, None)
        } else {
            let msg = blocked_reasons.join("; ");
            warn!(reason = %msg, "codex adapter blocked");
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
    fn codex_missing_binary() {
        let mut mock = MockRunner::new();
        mock.register(
            "which codex",
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            )),
        );
        let probe = CodexProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::Missing);
        assert_eq!(result.adapter_key, "codex");
    }

    #[test]
    fn codex_ready() {
        let mut mock = MockRunner::new();
        mock.register("which codex", Ok(success_output("/usr/bin/codex\n")));
        mock.register("codex --version", Ok(success_output("codex 0.5.0\n")));
        mock.register(
            "codex exec --help",
            Ok(success_output(
                "Usage: codex exec [options]\n  --json     Output JSON\n  --sandbox  Enable sandbox\n",
            )),
        );
        let probe = CodexProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::Ready);
        assert_eq!(result.tier, AdapterTier::Tier1);
        assert!(result.capabilities.json_stream);
        assert!(result.capabilities.plain_text);
        assert!(result.capabilities.sandbox_controls);
        assert_eq!(result.confidence, Confidence::Verified);
        assert!(result.version.unwrap().contains("0.5.0"));
    }

    #[test]
    fn codex_blocked_no_json() {
        let mut mock = MockRunner::new();
        mock.register("which codex", Ok(success_output("/usr/bin/codex\n")));
        mock.register("codex --version", Ok(success_output("codex 0.1.0\n")));
        mock.register(
            "codex exec --help",
            Ok(success_output("Usage: codex exec [options]\n  --verbose\n")),
        );
        let probe = CodexProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::Blocked);
        assert!(result.message.unwrap().contains("--json"));
    }

    #[test]
    fn codex_unknown_fields_dont_crash() {
        let mut mock = MockRunner::new();
        mock.register("which codex", Ok(success_output("/usr/bin/codex\n")));
        mock.register("codex --version", Ok(success_output("codex 1.0.0\n")));
        mock.register(
            "codex exec --help",
            Ok(success_output(
                "Usage: codex exec [options]\n  --json     JSON output\n  --future-flag  Something new\n  --another-one  Yep\n",
            )),
        );
        let probe = CodexProbe::new(&mock);
        let result = probe.probe();
        assert_eq!(result.status, ProbeStatus::Ready);
    }
}
