use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Tier classification for adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterTier {
    Tier1,
    Experimental,
}

impl std::fmt::Display for AdapterTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterTier::Tier1 => write!(f, "tier-1"),
            AdapterTier::Experimental => write!(f, "experimental"),
        }
    }
}

/// Confidence level for a given capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityConfidence {
    Verified,
    Observed,
    Unknown,
}

/// Capabilities reported by an adapter probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub json_stream: CapabilityEntry,
    pub plain_text: CapabilityEntry,
    pub force_edit_mode: CapabilityEntry,
    pub sandbox_controls: CapabilityEntry,
    pub approval_controls: CapabilityEntry,
    pub session_resume: CapabilityEntry,
    pub emits_usage: CapabilityEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityEntry {
    pub supported: bool,
    pub confidence: CapabilityConfidence,
}

impl CapabilityEntry {
    pub fn verified(supported: bool) -> Self {
        Self {
            supported,
            confidence: CapabilityConfidence::Verified,
        }
    }

    pub fn observed(supported: bool) -> Self {
        Self {
            supported,
            confidence: CapabilityConfidence::Observed,
        }
    }

    pub fn unknown() -> Self {
        Self {
            supported: false,
            confidence: CapabilityConfidence::Unknown,
        }
    }
}

/// Result of an adapter probe (binary discovery + version + flags).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectResult {
    pub status: DetectStatus,
    pub binary_path: Option<PathBuf>,
    pub version: Option<String>,
    pub supported_flags: Vec<String>,
    pub confidence: CapabilityConfidence,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectStatus {
    Ready,
    Blocked,
    ExperimentalReady,
    ExperimentalBlocked,
    Missing,
}

impl DetectStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Ready | Self::ExperimentalReady)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::ExperimentalReady => "experimental-ready",
            Self::ExperimentalBlocked => "experimental-blocked",
            Self::Missing => "missing",
        }
    }
}

impl DetectResult {
    pub fn status_label(&self) -> &'static str {
        self.status.label()
    }
}

/// Individual probe result contributed to the overall report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub adapter_key: String,
    pub tier: AdapterTier,
    pub detect: DetectResult,
    pub capabilities: CapabilitySet,
}

/// Normalized agent event types emitted by adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    Message {
        content: String,
    },
    ToolCall {
        tool: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool: String,
        output: serde_json::Value,
    },
    Progress {
        message: String,
        percent: Option<f64>,
    },
    Completed {
        summary: Option<String>,
    },
    Failed {
        error: String,
    },
    Usage {
        input_tokens: u64,
        output_tokens: u64,
        extra: HashMap<String, serde_json::Value>,
    },
}

/// Request passed to an adapter to build a spawn command.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub task_prompt: String,
    pub worktree_path: PathBuf,
    pub timeout_seconds: u64,
    pub allow_network: bool,
    pub force_edit: bool,
    pub output_json_stream: bool,
    pub unsafe_mode: bool,
    pub supported_flags: Vec<String>,
}

/// Command built by an adapter for the process supervisor.
#[derive(Debug, Clone)]
pub struct BuiltCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
}
