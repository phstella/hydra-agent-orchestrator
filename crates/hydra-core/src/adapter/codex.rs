use std::path::PathBuf;
use std::process::Command;

use super::types::*;
use super::{parse_version_string, resolve_binary, AgentAdapter};

/// OpenAI Codex adapter probe implementation.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_HELP: &str = include_str!("../../tests/fixtures/adapters/codex/help.txt");

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
}
