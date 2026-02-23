use std::path::PathBuf;
use std::process::Command;

use super::types::*;
use super::{resolve_binary, AgentAdapter};

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

    fn probe_help(binary: &PathBuf) -> Result<String, String> {
        let output = Command::new(binary)
            .arg("--help")
            .output()
            .map_err(|e| format!("failed to run --help: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(format!("{stdout}{stderr}"))
    }

    fn probe_version(binary: &PathBuf) -> Option<String> {
        let output = Command::new(binary).arg("--version").output().ok()?;
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        parse_version_string(&text)
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
}

fn parse_version_string(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        for word in line.split_whitespace() {
            let w = word.strip_prefix('v').unwrap_or(word);
            if w.contains('.') && w.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return Some(w.to_string());
            }
        }
    }
    None
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
}
