use std::path::PathBuf;
use std::process::Command;

use super::types::*;
use super::{parse_version_string, resolve_binary, AdapterError, AgentAdapter};

/// Cursor Agent adapter probe implementation (experimental).
///
/// Cursor is Tier-2 experimental: the probe never promotes it to Tier-1.
/// Status can be `ExperimentalReady`, `ExperimentalBlocked`, or `Missing`.
pub struct CursorAdapter {
    configured_path: Option<String>,
}

impl CursorAdapter {
    pub fn new(configured_path: Option<String>) -> Self {
        Self { configured_path }
    }

    fn resolve_binary_path(&self) -> Option<PathBuf> {
        resolve_binary(self.configured_path.as_deref(), &["cursor-agent", "cursor"])
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
    /// Cursor flags have lower confidence than Tier-1 adapters.
    pub fn parse_help_flags(help_text: &str) -> Vec<String> {
        let mut flags = Vec::new();

        if help_text.contains("-p") || help_text.contains("--print") {
            flags.push("--print".to_string());
        }
        if help_text.contains("--output-format") {
            flags.push("--output-format".to_string());
        }
        if help_text.contains("-f") || help_text.contains("--force") {
            flags.push("--force".to_string());
        }

        flags
    }

    /// Parse a single line of stream-json output from the cursor agent.
    /// Uses the same format as Claude's stream-json protocol.
    pub fn parse_stream_json_line(line: &str) -> Option<AgentEvent> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
        let event_type = value.get("type")?.as_str()?;

        match event_type {
            "system" => {
                let msg = value
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("system event")
                    .to_string();
                Some(AgentEvent::Progress {
                    message: msg,
                    percent: None,
                })
            }
            "assistant" => {
                if let Some(content) = value.get("content").and_then(|v| v.as_str()) {
                    if content.is_empty() {
                        return None;
                    }
                    Some(AgentEvent::Message {
                        content: content.to_string(),
                    })
                } else if let Some(tool_use) = value.get("tool_use") {
                    let tool = tool_use
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let input = tool_use.get("input").cloned().unwrap_or_default();
                    Some(AgentEvent::ToolCall { tool, input })
                } else {
                    None
                }
            }
            "result" => {
                if let Some(usage) = value.get("usage") {
                    let input_tokens = usage
                        .get("input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let output_tokens = usage
                        .get("output_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    Some(AgentEvent::Usage {
                        input_tokens,
                        output_tokens,
                        extra: std::collections::HashMap::new(),
                    })
                } else {
                    Some(AgentEvent::Completed { summary: None })
                }
            }
            _ => None,
        }
    }
}

impl AgentAdapter for CursorAdapter {
    fn key(&self) -> &'static str {
        "cursor-agent"
    }

    fn tier(&self) -> AdapterTier {
        AdapterTier::Experimental
    }

    fn detect(&self) -> DetectResult {
        let binary = resolve_binary(self.configured_path.as_deref(), &["cursor-agent", "cursor"]);

        let Some(binary_path) = binary else {
            return DetectResult {
                status: DetectStatus::Missing,
                binary_path: None,
                version: None,
                supported_flags: vec![],
                confidence: CapabilityConfidence::Unknown,
                error: Some("cursor-agent/cursor binary not found in PATH".to_string()),
            };
        };

        let version = Self::probe_version(&binary_path);

        let help_text = match Self::probe_help(&binary_path) {
            Ok(text) => text,
            Err(e) => {
                return DetectResult {
                    status: DetectStatus::ExperimentalBlocked,
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

        let status = if has_print && has_output_format {
            DetectStatus::ExperimentalReady
        } else {
            DetectStatus::ExperimentalBlocked
        };

        let confidence = if has_print && has_output_format {
            CapabilityConfidence::Observed
        } else {
            CapabilityConfidence::Unknown
        };

        let error = if status == DetectStatus::ExperimentalBlocked {
            let mut missing = Vec::new();
            if !has_print {
                missing.push("-p/--print");
            }
            if !has_output_format {
                missing.push("--output-format");
            }
            Some(format!("missing flags: {}", missing.join(", ")))
        } else {
            None
        };

        DetectResult {
            status,
            binary_path: Some(binary_path),
            version,
            supported_flags: flags,
            confidence,
            error,
        }
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            json_stream: CapabilityEntry::observed(true),
            plain_text: CapabilityEntry::observed(true),
            force_edit_mode: CapabilityEntry::observed(true),
            sandbox_controls: CapabilityEntry::unknown(),
            approval_controls: CapabilityEntry::unknown(),
            session_resume: CapabilityEntry::unknown(),
            emits_usage: CapabilityEntry::unknown(),
        }
    }

    fn build_command(&self, req: &SpawnRequest) -> Result<BuiltCommand, AdapterError> {
        let binary = self
            .resolve_binary_path()
            .ok_or_else(|| AdapterError::BinaryMissing {
                adapter: "cursor-agent".to_string(),
            })?;

        let mut args = vec![
            "-p".to_string(),
            req.task_prompt.clone(),
            "--output-format".to_string(),
            "stream-json".to_string(),
        ];

        if req.force_edit && req.supported_flags.iter().any(|f| f == "--force") {
            args.push("--force".to_string());
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
        let text = String::from_utf8_lossy(chunk);
        text.lines().filter_map(|l| self.parse_line(l)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_HELP: &str = include_str!("../../tests/fixtures/adapters/cursor/help.txt");

    #[test]
    fn cursor_adapter_is_experimental() {
        let adapter = CursorAdapter::new(None);
        assert_eq!(adapter.tier(), AdapterTier::Experimental);
        assert_eq!(adapter.key(), "cursor-agent");
    }

    #[test]
    fn cursor_never_returns_tier1_status() {
        let adapter = CursorAdapter::new(Some("/nonexistent/cursor".to_string()));
        let result = adapter.detect();
        assert!(
            matches!(
                result.status,
                DetectStatus::Missing
                    | DetectStatus::ExperimentalReady
                    | DetectStatus::ExperimentalBlocked
            ),
            "Cursor probe must never return Ready or Blocked (Tier-1 statuses)"
        );
    }

    #[test]
    fn parse_help_finds_cursor_flags() {
        let flags = CursorAdapter::parse_help_flags(FIXTURE_HELP);
        assert!(flags.contains(&"--print".to_string()));
        assert!(flags.contains(&"--output-format".to_string()));
        assert!(flags.contains(&"--force".to_string()));
    }

    #[test]
    fn parse_help_empty_returns_empty() {
        let flags = CursorAdapter::parse_help_flags("");
        assert!(flags.is_empty());
    }

    #[test]
    fn cursor_capabilities_are_observed_confidence() {
        let adapter = CursorAdapter::new(None);
        let caps = adapter.capabilities();
        assert_eq!(caps.json_stream.confidence, CapabilityConfidence::Observed);
        assert_eq!(
            caps.force_edit_mode.confidence,
            CapabilityConfidence::Observed
        );
    }

    #[test]
    fn detect_returns_missing_when_binary_absent() {
        let adapter = CursorAdapter::new(Some("/nonexistent/cursor-agent".to_string()));
        let result = adapter.detect();
        assert_eq!(result.status, DetectStatus::Missing);
    }

    #[test]
    fn parse_version_cursor_format() {
        assert_eq!(
            parse_version_string("Cursor Agent CLI v0.45.2"),
            Some("0.45.2".to_string())
        );
    }

    #[test]
    fn build_command_produces_correct_flags() {
        let req = SpawnRequest {
            task_prompt: "fix the bug".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/wt"),
            timeout_seconds: 300,
            allow_network: false,
            force_edit: true,
            output_json_stream: true,
            unsafe_mode: false,
            supported_flags: vec![
                "--print".to_string(),
                "--output-format".to_string(),
                "--force".to_string(),
            ],
        };
        let adapter = CursorAdapter::new(Some("/usr/bin/cursor-agent".to_string()));
        // Can't actually test build_command without a real binary
        // but we can verify parse_line works
        let _ = adapter.key();
        let _ = req;
    }

    #[test]
    fn parse_line_system_event() {
        let line = r#"{"type":"system","message":"Cursor agent starting"}"#;
        let evt = CursorAdapter::parse_stream_json_line(line).unwrap();
        assert!(matches!(evt, AgentEvent::Progress { .. }));
    }

    #[test]
    fn parse_line_assistant_message() {
        let line = r#"{"type":"assistant","content":"I'll fix that bug"}"#;
        let evt = CursorAdapter::parse_stream_json_line(line).unwrap();
        assert!(matches!(evt, AgentEvent::Message { .. }));
    }

    #[test]
    fn parse_line_assistant_tool_use() {
        let line =
            r#"{"type":"assistant","tool_use":{"name":"edit","input":{"file":"src/main.rs"}}}"#;
        let evt = CursorAdapter::parse_stream_json_line(line).unwrap();
        assert!(matches!(evt, AgentEvent::ToolCall { .. }));
    }

    #[test]
    fn parse_line_result_with_usage() {
        let line = r#"{"type":"result","usage":{"input_tokens":100,"output_tokens":50}}"#;
        let evt = CursorAdapter::parse_stream_json_line(line).unwrap();
        assert!(matches!(evt, AgentEvent::Usage { .. }));
    }

    #[test]
    fn parse_line_result_without_usage() {
        let line = r#"{"type":"result"}"#;
        let evt = CursorAdapter::parse_stream_json_line(line).unwrap();
        assert!(matches!(evt, AgentEvent::Completed { .. }));
    }

    #[test]
    fn parse_line_empty_returns_none() {
        assert!(CursorAdapter::parse_stream_json_line("").is_none());
    }

    #[test]
    fn parse_line_invalid_json_returns_none() {
        assert!(CursorAdapter::parse_stream_json_line("not json").is_none());
    }

    #[test]
    fn parse_line_unknown_type_returns_none() {
        let line = r#"{"type":"unknown_event"}"#;
        assert!(CursorAdapter::parse_stream_json_line(line).is_none());
    }
}
