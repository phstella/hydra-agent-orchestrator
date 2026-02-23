use std::path::PathBuf;
use std::process::Command;

use super::types::*;
use super::{parse_version_string, resolve_binary, AgentAdapter};

/// Claude Code adapter probe implementation.
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

        let status = if has_print && has_output_format {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_HELP: &str = include_str!("../../tests/fixtures/adapters/claude/help.txt");

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
}
