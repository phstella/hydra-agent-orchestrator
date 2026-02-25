use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use super::error::AdapterError;
use super::types::*;
use super::{parse_version_string, resolve_binary, AgentAdapter};

/// Claude Code adapter: probe + runtime implementation.
pub struct ClaudeAdapter {
    configured_path: Option<String>,
}

impl ClaudeAdapter {
    pub fn new(configured_path: Option<String>) -> Self {
        Self { configured_path }
    }

    fn probe_help(binary: &PathBuf) -> Result<String, String> {
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
    pub fn parse_help_flags(help_text: &str) -> Vec<String> {
        let mut flags = Vec::new();
        let required = [
            ("-p", "--print"),
            ("--output-format", "--output-format"),
            ("--permission-mode", "--permission-mode"),
        ];

        for (short, long) in &required {
            if Self::contains_flag_token(help_text, short)
                || Self::contains_flag_token(help_text, long)
            {
                flags.push(long.to_string());
            }
        }

        let optional = [
            "--allowedTools",
            "--disallowedTools",
            "--max-turns",
            "--input-format",
            "--verbose",
        ];
        for flag in &optional {
            if Self::contains_flag_token(help_text, flag) {
                flags.push(flag.to_string());
            }
        }

        flags
    }

    fn contains_flag_token(help_text: &str, flag: &str) -> bool {
        help_text.lines().any(|line| {
            line.split(|c: char| c.is_whitespace() || c == ',' || c == ':' || c == '(' || c == ')')
                .any(|token| token == flag)
        })
    }

    /// Parse a single line of Claude `stream-json` output into an `AgentEvent`.
    ///
    /// Claude stream-json emits one JSON object per line. Known types:
    /// - `system` (init) -> Progress
    /// - `assistant` with message content -> Message
    /// - `assistant` with tool_use -> ToolCall
    /// - `result` with subtype `tool_result` -> ToolResult
    /// - `result` with subtype `success` -> Completed + optional Usage
    pub fn parse_stream_json_line(line: &str) -> Option<AgentEvent> {
        let v: serde_json::Value = serde_json::from_str(line).ok()?;
        let obj = v.as_object()?;

        match obj.get("type")?.as_str()? {
            "system" => Some(AgentEvent::Progress {
                message: format!(
                    "session init: {}",
                    obj.get("subtype")
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown")
                ),
                percent: None,
            }),
            "assistant" => {
                let msg = obj.get("message")?.as_object()?;
                if let Some(tool_use) = msg.get("tool_use").and_then(|t| t.as_object()) {
                    let tool = tool_use
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let input = tool_use
                        .get("input")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    return Some(AgentEvent::ToolCall { tool, input });
                }
                let content = msg
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                if content.is_empty() {
                    return None;
                }
                Some(AgentEvent::Message { content })
            }
            "result" => {
                let subtype = obj.get("subtype").and_then(|s| s.as_str()).unwrap_or("");
                match subtype {
                    "tool_result" => {
                        let tool_id = obj
                            .get("tool_use_id")
                            .and_then(|t| t.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let content = obj
                            .get("content")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                        Some(AgentEvent::ToolResult {
                            tool: tool_id,
                            output: content,
                        })
                    }
                    "success" => {
                        let summary = obj
                            .get("cost_usd")
                            .and_then(|c| c.as_f64())
                            .map(|c| format!("cost: ${c:.4}"));
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
                            if let Some(dur) = obj.get("duration_ms").and_then(|d| d.as_u64()) {
                                extra.insert(
                                    "duration_ms".to_string(),
                                    serde_json::Value::from(dur),
                                );
                            }
                            return Some(AgentEvent::Usage {
                                input_tokens,
                                output_tokens,
                                extra,
                            });
                        }
                        Some(AgentEvent::Completed { summary })
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl AgentAdapter for ClaudeAdapter {
    fn key(&self) -> &'static str {
        "claude"
    }

    fn tier(&self) -> AdapterTier {
        AdapterTier::Tier1
    }

    fn detect(&self) -> DetectResult {
        let binary = resolve_binary(self.configured_path.as_deref(), &["claude"]);

        let Some(binary_path) = binary else {
            return DetectResult {
                status: DetectStatus::Missing,
                binary_path: None,
                version: None,
                supported_flags: vec![],
                confidence: CapabilityConfidence::Verified,
                error: Some("claude binary not found in PATH".to_string()),
            };
        };

        let version = Self::probe_version(&binary_path);

        let help_text = match Self::probe_help(&binary_path) {
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

        let flags = Self::parse_help_flags(&help_text);

        let has_print = flags.iter().any(|f| f == "--print");
        let has_output_format = flags.iter().any(|f| f == "--output-format");
        let has_permission_mode = flags.iter().any(|f| f == "--permission-mode");

        let status = if has_print && has_output_format && has_permission_mode {
            DetectStatus::Ready
        } else {
            DetectStatus::Blocked
        };

        let error = if status == DetectStatus::Blocked {
            let mut missing = Vec::new();
            if !has_print {
                missing.push("-p/--print");
            }
            if !has_output_format {
                missing.push("--output-format");
            }
            if !has_permission_mode {
                missing.push("--permission-mode");
            }
            Some(format!("missing required flags: {}", missing.join(", ")))
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
            sandbox_controls: CapabilityEntry::observed(false),
            approval_controls: CapabilityEntry::verified(true),
            session_resume: CapabilityEntry::unknown(),
            emits_usage: CapabilityEntry::verified(true),
        }
    }

    fn build_command(&self, req: &SpawnRequest) -> Result<BuiltCommand, AdapterError> {
        let binary = resolve_binary(self.configured_path.as_deref(), &["claude"]).ok_or(
            AdapterError::BinaryMissing {
                adapter: "claude".to_string(),
            },
        )?;

        let mut args = vec![
            "-p".to_string(),
            req.task_prompt.clone(),
            "--output-format".to_string(),
            "stream-json".to_string(),
        ];

        if req.supported_flags.iter().any(|f| f == "--verbose") {
            args.push("--verbose".to_string());
        }

        if req.force_edit {
            args.push("--permission-mode".to_string());
            args.push("bypassPermissions".to_string());
        }

        Ok(BuiltCommand {
            program: binary.display().to_string(),
            args,
            env: vec![],
            cwd: req.worktree_path.clone(),
        })
    }

    fn parse_line(&self, line: &str) -> Option<AgentEvent> {
        Self::parse_stream_json_line(line)
    }

    fn parse_raw(&self, chunk: &[u8]) -> Vec<AgentEvent> {
        let text = match std::str::from_utf8(chunk) {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };
        text.lines()
            .filter_map(Self::parse_stream_json_line)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const FIXTURE_HELP: &str = include_str!("../../tests/fixtures/adapters/claude/help.txt");
    const FIXTURE_STREAM: &str =
        include_str!("../../tests/fixtures/adapters/claude/stream-json.ok.jsonl");

    #[test]
    fn parse_help_finds_required_flags() {
        let flags = ClaudeAdapter::parse_help_flags(FIXTURE_HELP);
        assert!(flags.contains(&"--print".to_string()));
        assert!(flags.contains(&"--output-format".to_string()));
        assert!(flags.contains(&"--permission-mode".to_string()));
    }

    #[test]
    fn parse_help_finds_optional_flags() {
        let flags = ClaudeAdapter::parse_help_flags(FIXTURE_HELP);
        assert!(flags.contains(&"--allowedTools".to_string()));
        assert!(flags.contains(&"--disallowedTools".to_string()));
        assert!(flags.contains(&"--max-turns".to_string()));
    }

    #[test]
    fn parse_help_detects_verbose_flag() {
        let help = "Options:\n  --verbose  Enables verbose logging";
        let flags = ClaudeAdapter::parse_help_flags(help);
        assert!(flags.contains(&"--verbose".to_string()));
    }

    #[test]
    fn parse_help_missing_flags_returns_empty_for_blank() {
        let flags = ClaudeAdapter::parse_help_flags("");
        assert!(flags.is_empty());
    }

    #[test]
    fn parse_help_does_not_false_positive_print_from_permission_mode() {
        let help = "Options:\n  --permission-mode <mode>  Controls approvals";
        let flags = ClaudeAdapter::parse_help_flags(help);
        assert!(!flags.contains(&"--print".to_string()));
        assert!(flags.contains(&"--permission-mode".to_string()));
    }

    #[test]
    fn parse_version_extracts_semver() {
        assert_eq!(
            parse_version_string("Claude Code CLI v1.0.42"),
            Some("1.0.42".to_string())
        );
        assert_eq!(parse_version_string("1.0.42"), Some("1.0.42".to_string()));
        assert_eq!(parse_version_string(""), None);
        assert_eq!(parse_version_string("no version here"), None);
    }

    #[test]
    fn claude_adapter_is_tier1() {
        let adapter = ClaudeAdapter::new(None);
        assert_eq!(adapter.tier(), AdapterTier::Tier1);
        assert_eq!(adapter.key(), "claude");
    }

    #[test]
    fn claude_capabilities_json_stream_verified() {
        let adapter = ClaudeAdapter::new(None);
        let caps = adapter.capabilities();
        assert!(caps.json_stream.supported);
        assert_eq!(caps.json_stream.confidence, CapabilityConfidence::Verified);
    }

    #[test]
    fn detect_returns_missing_when_binary_absent() {
        let adapter = ClaudeAdapter::new(Some("/nonexistent/claude".to_string()));
        let result = adapter.detect();
        assert_eq!(result.status, DetectStatus::Missing);
        assert!(result.error.is_some());
    }

    // --- M1.5: build_command tests ---

    #[test]
    fn build_command_produces_correct_flags() {
        let adapter = ClaudeAdapter::new(Some("/usr/bin/echo".to_string()));
        let req = SpawnRequest {
            task_prompt: "fix the bug".to_string(),
            worktree_path: PathBuf::from("/tmp/wt"),
            timeout_seconds: 300,
            allow_network: false,
            force_edit: true,
            output_json_stream: true,
            unsafe_mode: false,
            supported_flags: vec![],
        };
        let cmd = adapter.build_command(&req).unwrap();
        assert_eq!(cmd.program, "/usr/bin/echo");
        assert!(cmd.args.contains(&"-p".to_string()));
        assert!(cmd.args.contains(&"fix the bug".to_string()));
        assert!(cmd.args.contains(&"--output-format".to_string()));
        assert!(cmd.args.contains(&"stream-json".to_string()));
        assert!(cmd.args.contains(&"--permission-mode".to_string()));
        assert!(cmd.args.contains(&"bypassPermissions".to_string()));
        assert_eq!(cmd.cwd, PathBuf::from("/tmp/wt"));
    }

    #[test]
    fn build_command_omits_permission_mode_when_not_force_edit() {
        let adapter = ClaudeAdapter::new(Some("/usr/bin/echo".to_string()));
        let req = SpawnRequest {
            task_prompt: "describe the code".to_string(),
            worktree_path: PathBuf::from("/tmp/wt"),
            timeout_seconds: 300,
            allow_network: false,
            force_edit: false,
            output_json_stream: true,
            unsafe_mode: false,
            supported_flags: vec![],
        };
        let cmd = adapter.build_command(&req).unwrap();
        assert!(!cmd.args.contains(&"--permission-mode".to_string()));
    }

    #[test]
    fn build_command_includes_verbose_when_supported() {
        let adapter = ClaudeAdapter::new(Some("/usr/bin/echo".to_string()));
        let req = SpawnRequest {
            task_prompt: "describe the code".to_string(),
            worktree_path: PathBuf::from("/tmp/wt"),
            timeout_seconds: 300,
            allow_network: false,
            force_edit: true,
            output_json_stream: true,
            unsafe_mode: false,
            supported_flags: vec!["--verbose".to_string()],
        };
        let cmd = adapter.build_command(&req).unwrap();
        assert!(cmd.args.contains(&"--verbose".to_string()));
    }

    #[test]
    fn build_command_fails_when_binary_missing() {
        let adapter = ClaudeAdapter::new(Some("/nonexistent/claude".to_string()));
        let req = SpawnRequest {
            task_prompt: "test".to_string(),
            worktree_path: PathBuf::from("/tmp/wt"),
            timeout_seconds: 300,
            allow_network: false,
            force_edit: true,
            output_json_stream: true,
            unsafe_mode: false,
            supported_flags: vec![],
        };
        let err = adapter.build_command(&req).unwrap_err();
        assert!(matches!(err, AdapterError::BinaryMissing { .. }));
    }

    // --- M1.5: parse_line / parse_raw tests ---

    #[test]
    fn parse_line_system_init() {
        let line =
            r#"{"type":"system","subtype":"init","session_id":"abc123","tools":["Read","Write"]}"#;
        let evt = ClaudeAdapter::parse_stream_json_line(line)
            .expect("system init fixture should parse into an AgentEvent");
        match evt {
            AgentEvent::Progress { message, .. } => {
                assert!(message.contains("init"));
            }
            other => assert!(
                matches!(other, AgentEvent::Progress { .. }),
                "expected Progress, got {other:?}"
            ),
        }
    }

    #[test]
    fn parse_line_assistant_message() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":"I'll help with that task."},"session_id":"abc123"}"#;
        let evt = ClaudeAdapter::parse_stream_json_line(line)
            .expect("assistant fixture should parse into an AgentEvent");
        match evt {
            AgentEvent::Message { content } => {
                assert_eq!(content, "I'll help with that task.");
            }
            other => assert!(
                matches!(other, AgentEvent::Message { .. }),
                "expected Message, got {other:?}"
            ),
        }
    }

    #[test]
    fn parse_line_assistant_tool_use() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":"","tool_use":{"name":"Read","input":{"file_path":"src/main.rs"}}},"session_id":"abc123"}"#;
        let evt = ClaudeAdapter::parse_stream_json_line(line)
            .expect("assistant tool_use fixture should parse into an AgentEvent");
        match evt {
            AgentEvent::ToolCall { tool, input } => {
                assert_eq!(tool, "Read");
                let file_path = input["file_path"]
                    .as_str()
                    .expect("tool_use input file_path should be a string");
                assert!(file_path.contains("main.rs"));
            }
            other => assert!(
                matches!(other, AgentEvent::ToolCall { .. }),
                "expected ToolCall, got {other:?}"
            ),
        }
    }

    #[test]
    fn parse_line_result_tool_result() {
        let line = r#"{"type":"result","subtype":"tool_result","tool_use_id":"t1","content":"fn main() {}","session_id":"abc123"}"#;
        let evt = ClaudeAdapter::parse_stream_json_line(line)
            .expect("result tool_result fixture should parse into an AgentEvent");
        match evt {
            AgentEvent::ToolResult { tool, output } => {
                assert_eq!(tool, "t1");
                let text = output
                    .as_str()
                    .expect("tool_result output should be a string");
                assert!(text.contains("fn main"));
            }
            other => assert!(
                matches!(other, AgentEvent::ToolResult { .. }),
                "expected ToolResult, got {other:?}"
            ),
        }
    }

    #[test]
    fn parse_line_result_success_with_usage() {
        let line = r#"{"type":"result","subtype":"success","cost_usd":0.003,"duration_ms":2500,"session_id":"abc123","usage":{"input_tokens":1200,"output_tokens":450}}"#;
        let evt = ClaudeAdapter::parse_stream_json_line(line)
            .expect("result success fixture should parse into an AgentEvent");
        match evt {
            AgentEvent::Usage {
                input_tokens,
                output_tokens,
                extra,
            } => {
                assert_eq!(input_tokens, 1200);
                assert_eq!(output_tokens, 450);
                assert!(extra.contains_key("cost_usd"));
                assert!(extra.contains_key("duration_ms"));
            }
            other => assert!(
                matches!(other, AgentEvent::Usage { .. }),
                "expected Usage, got {other:?}"
            ),
        }
    }

    #[test]
    fn parse_line_ignores_unknown_type() {
        let line = r#"{"type":"unknown_future_type","data":{}}"#;
        assert!(ClaudeAdapter::parse_stream_json_line(line).is_none());
    }

    #[test]
    fn parse_line_handles_invalid_json() {
        assert!(ClaudeAdapter::parse_stream_json_line("not json at all").is_none());
        assert!(ClaudeAdapter::parse_stream_json_line("").is_none());
    }

    #[test]
    fn parse_line_empty_assistant_content_returns_none() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":""},"session_id":"abc123"}"#;
        assert!(ClaudeAdapter::parse_stream_json_line(line).is_none());
    }

    #[test]
    fn parse_raw_processes_multiple_lines() {
        let adapter = ClaudeAdapter::new(None);
        let events = adapter.parse_raw(FIXTURE_STREAM.as_bytes());
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
        assert!(has_usage, "should parse Usage from success result");
    }

    #[test]
    fn parse_line_fixture_lines_individually() {
        let adapter = ClaudeAdapter::new(None);
        for line in FIXTURE_STREAM.lines() {
            if line.trim().is_empty() {
                continue;
            }
            // Should not panic on any fixture line
            let _ = adapter.parse_line(line);
        }
    }
}
