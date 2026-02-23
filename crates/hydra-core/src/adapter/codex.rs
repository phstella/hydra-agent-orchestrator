use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use super::error::AdapterError;
use super::types::*;
use super::{parse_version_string, resolve_binary, AgentAdapter};

/// OpenAI Codex adapter: probe + runtime implementation.
pub struct CodexAdapter {
    configured_path: Option<String>,
}

impl CodexAdapter {
    pub fn new(configured_path: Option<String>) -> Self {
        Self { configured_path }
    }

    fn probe_help(binary: &PathBuf) -> Result<String, String> {
        let output = Command::new(binary)
            .args(["exec", "--help"])
            .output()
            .map_err(|e| format!("failed to run exec --help: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "exec --help exited with status {}",
                output
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string())
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(format!("{stdout}{stderr}"))
    }

    fn probe_top_help(binary: &PathBuf) -> Result<String, String> {
        let output = Command::new(binary)
            .arg("--help")
            .output()
            .map_err(|e| format!("failed to run --help: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "--help exited with status {}",
                output
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string())
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(format!("{stdout}{stderr}"))
    }

    fn probe_version(binary: &PathBuf) -> Option<String> {
        let output = Command::new(binary).arg("--version").output().ok()?;
        parse_version_string(&String::from_utf8_lossy(&output.stdout))
    }

    /// Parse help text to determine supported flags.
    pub fn parse_help_flags(top_help: &str, exec_help: &str) -> Vec<String> {
        let mut flags = Vec::new();
        let combined = format!("{top_help}\n{exec_help}");

        if combined.contains("exec") {
            flags.push("exec".to_string());
        }
        if combined.contains("--json") {
            flags.push("--json".to_string());
        }
        if combined.contains("--full-auto") {
            flags.push("--full-auto".to_string());
        }
        if combined.contains("--sandbox") {
            flags.push("--sandbox".to_string());
        }
        if combined.contains("--ask-for-approval") {
            flags.push("--ask-for-approval".to_string());
        }
        if combined.contains("--dangerously-bypass")
            || combined.contains("dangerously-bypass-approvals-and-sandbox")
        {
            flags.push("--dangerously-bypass-approvals-and-sandbox".to_string());
        }
        if combined.contains("-C") || combined.contains("--cd") {
            flags.push("--cd".to_string());
        }

        flags
    }

    /// Parse a single line of Codex `--json` JSONL output into an `AgentEvent`.
    ///
    /// Codex JSONL event types:
    /// - `start` -> Progress
    /// - `message` -> Message
    /// - `tool_call` -> ToolCall
    /// - `tool_result` -> ToolResult
    /// - `completed` -> Completed + optional Usage
    pub fn parse_json_line(line: &str) -> Option<AgentEvent> {
        let v: serde_json::Value = serde_json::from_str(line).ok()?;
        let obj = v.as_object()?;

        match obj.get("type")?.as_str()? {
            "start" => {
                let task = obj
                    .get("task")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                let model = obj
                    .get("model")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown");
                Some(AgentEvent::Progress {
                    message: format!("started: {task} (model: {model})"),
                    percent: Some(0.0),
                })
            }
            "message" => {
                let content = obj
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                if content.is_empty() {
                    return None;
                }
                Some(AgentEvent::Message { content })
            }
            "tool_call" => {
                let tool = obj
                    .get("tool")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let input = obj.get("input").cloned().unwrap_or(serde_json::Value::Null);
                Some(AgentEvent::ToolCall { tool, input })
            }
            "tool_result" => {
                let tool = obj
                    .get("tool")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let output = obj
                    .get("output")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                Some(AgentEvent::ToolResult { tool, output })
            }
            "completed" => {
                if let Some(usage) = obj.get("usage").and_then(|u| u.as_object()) {
                    let input_tokens = usage
                        .get("input_tokens")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0);
                    let output_tokens = usage
                        .get("output_tokens")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0);
                    let mut extra = HashMap::new();
                    if let Some(cost) = obj.get("cost_usd").and_then(|c| c.as_f64()) {
                        extra.insert("cost_usd".to_string(), serde_json::Value::from(cost));
                    }
                    return Some(AgentEvent::Usage {
                        input_tokens,
                        output_tokens,
                        extra,
                    });
                }
                let summary = obj
                    .get("cost_usd")
                    .and_then(|c| c.as_f64())
                    .map(|c| format!("cost: ${c:.4}"));
                Some(AgentEvent::Completed { summary })
            }
            _ => None,
        }
    }
}

impl AgentAdapter for CodexAdapter {
    fn key(&self) -> &'static str {
        "codex"
    }

    fn tier(&self) -> AdapterTier {
        AdapterTier::Tier1
    }

    fn detect(&self) -> DetectResult {
        let binary = resolve_binary(self.configured_path.as_deref(), &["codex"]);

        let Some(binary_path) = binary else {
            return DetectResult {
                status: DetectStatus::Missing,
                binary_path: None,
                version: None,
                supported_flags: vec![],
                confidence: CapabilityConfidence::Verified,
                error: Some("codex binary not found in PATH".to_string()),
            };
        };

        let version = Self::probe_version(&binary_path);

        let top_help = match Self::probe_top_help(&binary_path) {
            Ok(text) => text,
            Err(e) => {
                return DetectResult {
                    status: DetectStatus::Blocked,
                    binary_path: Some(binary_path),
                    version,
                    supported_flags: vec![],
                    confidence: CapabilityConfidence::Unknown,
                    error: Some(e),
                };
            }
        };

        let exec_help = match Self::probe_help(&binary_path) {
            Ok(text) => text,
            Err(e) => {
                return DetectResult {
                    status: DetectStatus::Blocked,
                    binary_path: Some(binary_path),
                    version,
                    supported_flags: vec![],
                    confidence: CapabilityConfidence::Unknown,
                    error: Some(e),
                };
            }
        };
        let flags = Self::parse_help_flags(&top_help, &exec_help);

        let has_exec = flags.iter().any(|f| f == "exec");
        let has_json = flags.iter().any(|f| f == "--json");

        let status = if has_exec && has_json {
            DetectStatus::Ready
        } else {
            DetectStatus::Blocked
        };

        let error = if status == DetectStatus::Blocked {
            let mut missing = Vec::new();
            if !has_exec {
                missing.push("exec subcommand");
            }
            if !has_json {
                missing.push("--json flag");
            }
            Some(format!("missing required: {}", missing.join(", ")))
        } else {
            None
        };

        DetectResult {
            status,
            binary_path: Some(binary_path),
            version,
            supported_flags: flags,
            confidence: CapabilityConfidence::Verified,
            error,
        }
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            json_stream: CapabilityEntry::verified(true),
            plain_text: CapabilityEntry::verified(true),
            force_edit_mode: CapabilityEntry::verified(true),
            sandbox_controls: CapabilityEntry::verified(true),
            approval_controls: CapabilityEntry::verified(true),
            session_resume: CapabilityEntry::unknown(),
            emits_usage: CapabilityEntry::verified(true),
        }
    }

    fn build_command(&self, req: &SpawnRequest) -> Result<BuiltCommand, AdapterError> {
        let binary = resolve_binary(self.configured_path.as_deref(), &["codex"]).ok_or(
            AdapterError::BinaryMissing {
                adapter: "codex".to_string(),
            },
        )?;

        let mut args = vec![
            "exec".to_string(),
            req.task_prompt.clone(),
            "--json".to_string(),
        ];

        if req.force_edit {
            args.push("--full-auto".to_string());
        }

        Ok(BuiltCommand {
            program: binary.display().to_string(),
            args,
            env: vec![],
            cwd: req.worktree_path.clone(),
        })
    }

    fn parse_line(&self, line: &str) -> Option<AgentEvent> {
        Self::parse_json_line(line)
    }

    fn parse_raw(&self, chunk: &[u8]) -> Vec<AgentEvent> {
        let text = match std::str::from_utf8(chunk) {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };
        text.lines().filter_map(Self::parse_json_line).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const FIXTURE_HELP: &str = include_str!("../../tests/fixtures/adapters/codex/help.txt");
    const FIXTURE_JSON: &str =
        include_str!("../../tests/fixtures/adapters/codex/exec-json.ok.jsonl");

    #[test]
    fn parse_help_finds_exec_and_json() {
        let flags = CodexAdapter::parse_help_flags(FIXTURE_HELP, FIXTURE_HELP);
        assert!(flags.contains(&"exec".to_string()));
        assert!(flags.contains(&"--json".to_string()));
    }

    #[test]
    fn parse_help_finds_approval_flags() {
        let flags = CodexAdapter::parse_help_flags(FIXTURE_HELP, FIXTURE_HELP);
        assert!(flags.contains(&"--full-auto".to_string()));
        assert!(flags.contains(&"--sandbox".to_string()));
        assert!(flags.contains(&"--ask-for-approval".to_string()));
        assert!(flags.contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));
    }

    #[test]
    fn parse_help_empty_returns_empty() {
        let flags = CodexAdapter::parse_help_flags("", "");
        assert!(flags.is_empty());
    }

    #[test]
    fn codex_adapter_is_tier1() {
        let adapter = CodexAdapter::new(None);
        assert_eq!(adapter.tier(), AdapterTier::Tier1);
        assert_eq!(adapter.key(), "codex");
    }

    #[test]
    fn codex_capabilities_sandbox_verified() {
        let adapter = CodexAdapter::new(None);
        let caps = adapter.capabilities();
        assert!(caps.sandbox_controls.supported);
        assert_eq!(
            caps.sandbox_controls.confidence,
            CapabilityConfidence::Verified
        );
    }

    #[test]
    fn detect_returns_missing_when_binary_absent() {
        let adapter = CodexAdapter::new(Some("/nonexistent/codex".to_string()));
        let result = adapter.detect();
        assert_eq!(result.status, DetectStatus::Missing);
    }

    #[test]
    fn parse_version_codex_format() {
        assert_eq!(
            parse_version_string("codex v0.1.2025062"),
            Some("0.1.2025062".to_string())
        );
    }

    // --- M1.6: build_command tests ---

    #[test]
    fn build_command_produces_correct_flags() {
        let adapter = CodexAdapter::new(Some("/usr/bin/echo".to_string()));
        let req = SpawnRequest {
            task_prompt: "Fix the bug in main.rs".to_string(),
            worktree_path: PathBuf::from("/tmp/wt"),
            timeout_seconds: 300,
            allow_network: false,
            force_edit: true,
            output_json_stream: true,
        };
        let cmd = adapter.build_command(&req).unwrap();
        assert_eq!(cmd.program, "/usr/bin/echo");
        assert_eq!(cmd.args[0], "exec");
        assert_eq!(cmd.args[1], "Fix the bug in main.rs");
        assert!(cmd.args.contains(&"--json".to_string()));
        assert!(cmd.args.contains(&"--full-auto".to_string()));
        assert_eq!(cmd.cwd, PathBuf::from("/tmp/wt"));
    }

    #[test]
    fn build_command_omits_full_auto_when_not_force_edit() {
        let adapter = CodexAdapter::new(Some("/usr/bin/echo".to_string()));
        let req = SpawnRequest {
            task_prompt: "describe".to_string(),
            worktree_path: PathBuf::from("/tmp/wt"),
            timeout_seconds: 300,
            allow_network: false,
            force_edit: false,
            output_json_stream: true,
        };
        let cmd = adapter.build_command(&req).unwrap();
        assert!(!cmd.args.contains(&"--full-auto".to_string()));
    }

    #[test]
    fn build_command_fails_when_binary_missing() {
        let adapter = CodexAdapter::new(Some("/nonexistent/codex".to_string()));
        let req = SpawnRequest {
            task_prompt: "test".to_string(),
            worktree_path: PathBuf::from("/tmp/wt"),
            timeout_seconds: 300,
            allow_network: false,
            force_edit: true,
            output_json_stream: true,
        };
        let err = adapter.build_command(&req).unwrap_err();
        assert!(matches!(err, AdapterError::BinaryMissing { .. }));
    }

    // --- M1.6: parse_line / parse_raw tests ---

    #[test]
    fn parse_line_start_event() {
        let line = r#"{"type":"start","task":"Fix the bug in main.rs","model":"o4-mini"}"#;
        let evt = CodexAdapter::parse_json_line(line).unwrap();
        match evt {
            AgentEvent::Progress { message, percent } => {
                assert!(message.contains("Fix the bug"));
                assert!(message.contains("o4-mini"));
                assert_eq!(percent, Some(0.0));
            }
            other => panic!("expected Progress, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_message_event() {
        let line =
            r#"{"type":"message","role":"assistant","content":"I'll fix the bug in main.rs."}"#;
        let evt = CodexAdapter::parse_json_line(line).unwrap();
        match evt {
            AgentEvent::Message { content } => {
                assert_eq!(content, "I'll fix the bug in main.rs.");
            }
            other => panic!("expected Message, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_tool_call_event() {
        let line = r#"{"type":"tool_call","tool":"shell","input":{"command":"cat src/main.rs"}}"#;
        let evt = CodexAdapter::parse_json_line(line).unwrap();
        match evt {
            AgentEvent::ToolCall { tool, input } => {
                assert_eq!(tool, "shell");
                assert!(input["command"].as_str().unwrap().contains("main.rs"));
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_tool_result_event() {
        let line = r#"{"type":"tool_result","tool":"shell","output":"fn main() {\n    println!(\"Hello\");\n}"}"#;
        let evt = CodexAdapter::parse_json_line(line).unwrap();
        match evt {
            AgentEvent::ToolResult { tool, output } => {
                assert_eq!(tool, "shell");
                assert!(output.as_str().unwrap().contains("fn main"));
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_completed_with_usage() {
        let line = r#"{"type":"completed","usage":{"input_tokens":800,"output_tokens":320},"cost_usd":0.002}"#;
        let evt = CodexAdapter::parse_json_line(line).unwrap();
        match evt {
            AgentEvent::Usage {
                input_tokens,
                output_tokens,
                extra,
            } => {
                assert_eq!(input_tokens, 800);
                assert_eq!(output_tokens, 320);
                assert!(extra.contains_key("cost_usd"));
            }
            other => panic!("expected Usage, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_completed_without_usage() {
        let line = r#"{"type":"completed"}"#;
        let evt = CodexAdapter::parse_json_line(line).unwrap();
        assert!(matches!(evt, AgentEvent::Completed { .. }));
    }

    #[test]
    fn parse_line_ignores_unknown_type() {
        let line = r#"{"type":"future_event","data":{}}"#;
        assert!(CodexAdapter::parse_json_line(line).is_none());
    }

    #[test]
    fn parse_line_handles_invalid_json() {
        assert!(CodexAdapter::parse_json_line("not json").is_none());
        assert!(CodexAdapter::parse_json_line("").is_none());
    }

    #[test]
    fn parse_line_empty_message_returns_none() {
        let line = r#"{"type":"message","role":"assistant","content":""}"#;
        assert!(CodexAdapter::parse_json_line(line).is_none());
    }

    #[test]
    fn parse_raw_processes_fixture_lines() {
        let adapter = CodexAdapter::new(None);
        let events = adapter.parse_raw(FIXTURE_JSON.as_bytes());
        assert!(
            events.len() >= 3,
            "fixture should produce at least 3 events, got {}",
            events.len()
        );

        let has_message = events
            .iter()
            .any(|e| matches!(e, AgentEvent::Message { .. }));
        let has_tool_call = events
            .iter()
            .any(|e| matches!(e, AgentEvent::ToolCall { .. }));
        let has_usage = events.iter().any(|e| matches!(e, AgentEvent::Usage { .. }));
        assert!(has_message, "should parse at least one Message");
        assert!(has_tool_call, "should parse at least one ToolCall");
        assert!(has_usage, "should parse Usage from completed event");
    }

    #[test]
    fn parse_line_fixture_lines_individually() {
        let adapter = CodexAdapter::new(None);
        for line in FIXTURE_JSON.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let _ = adapter.parse_line(line);
        }
    }
}
