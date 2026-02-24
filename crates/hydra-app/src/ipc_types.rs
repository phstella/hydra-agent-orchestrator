use serde::{Deserialize, Serialize};

use hydra_core::adapter::{
    AdapterTier, CapabilityConfidence, CapabilitySet, DetectStatus, ProbeResult,
};

// ---------------------------------------------------------------------------
// Doctor / Preflight types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticCheck {
    pub name: String,
    pub description: String,
    pub status: CheckStatus,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Passed,
    Failed,
    Warning,
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterInfo {
    pub key: String,
    pub tier: AdapterTier,
    pub status: DetectStatus,
    pub version: Option<String>,
    pub confidence: CapabilityConfidence,
    pub capabilities: CapabilitySet,
}

impl From<&ProbeResult> for AdapterInfo {
    fn from(pr: &ProbeResult) -> Self {
        Self {
            key: pr.adapter_key.clone(),
            tier: pr.tier,
            status: pr.detect.status.clone(),
            version: pr.detect.version.clone(),
            confidence: pr.detect.confidence,
            capabilities: pr.capabilities.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightResult {
    pub system_ready: bool,
    pub all_tier1_ready: bool,
    pub passed_count: u32,
    pub failed_count: u32,
    pub total_count: u32,
    pub health_score: f64,
    pub checks: Vec<DiagnosticCheck>,
    pub adapters: Vec<AdapterInfo>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Race IPC types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaceRequest {
    pub task_prompt: String,
    pub agents: Vec<String>,
    pub allow_experimental: bool,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaceStarted {
    pub run_id: String,
    pub agents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStreamEvent {
    pub run_id: String,
    pub agent_key: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionScoreIpc {
    pub name: String,
    pub score: f64,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaceResult {
    pub run_id: String,
    pub status: String,
    pub agents: Vec<AgentResult>,
    pub duration_ms: Option<u64>,
    pub total_cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaceEventBatch {
    pub run_id: String,
    pub events: Vec<AgentStreamEvent>,
    pub next_cursor: u64,
    pub done: bool,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkingTreeStatus {
    pub clean: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentResult {
    pub agent_key: String,
    pub status: String,
    pub duration_ms: Option<u64>,
    pub score: Option<f64>,
    pub mergeable: Option<bool>,
    pub gate_failures: Vec<String>,
    pub dimensions: Vec<DimensionScoreIpc>,
}

// ---------------------------------------------------------------------------
// Interactive session types (M4.2)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveSessionRequest {
    pub agent_key: String,
    pub task_prompt: String,
    pub allow_experimental: bool,
    pub unsafe_mode: bool,
    pub cwd: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveSessionStarted {
    pub session_id: String,
    pub agent_key: String,
    pub status: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveStreamEvent {
    pub session_id: String,
    pub agent_key: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveEventBatch {
    pub session_id: String,
    pub events: Vec<InteractiveStreamEvent>,
    pub next_cursor: u64,
    pub done: bool,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveWriteAck {
    pub session_id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveResizeAck {
    pub session_id: String,
    pub success: bool,
    pub cols: u16,
    pub rows: u16,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveStopResult {
    pub session_id: String,
    pub status: String,
    pub was_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveSessionSummary {
    pub session_id: String,
    pub agent_key: String,
    pub status: String,
    pub started_at: String,
    pub event_count: u64,
}

// ---------------------------------------------------------------------------
// IPC Error wrapper
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl IpcError {
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: "internal_error".to_string(),
            message: msg.into(),
            details: None,
        }
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        Self {
            code: "validation_error".to_string(),
            message: msg.into(),
            details: None,
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: "not_found".to_string(),
            message: msg.into(),
            details: None,
        }
    }

    pub fn adapter_error(msg: impl Into<String>) -> Self {
        Self {
            code: "adapter_error".to_string(),
            message: msg.into(),
            details: None,
        }
    }

    pub fn safety_gate(msg: impl Into<String>) -> Self {
        Self {
            code: "safety_gate".to_string(),
            message: msg.into(),
            details: None,
        }
    }

    pub fn experimental_blocked(msg: impl Into<String>) -> Self {
        Self {
            code: "experimental_blocked".to_string(),
            message: msg.into(),
            details: None,
        }
    }

    pub fn dirty_worktree(msg: impl Into<String>) -> Self {
        Self {
            code: "dirty_worktree".to_string(),
            message: msg.into(),
            details: None,
        }
    }

    pub fn unsafe_blocked(msg: impl Into<String>) -> Self {
        Self {
            code: "unsafe_blocked".to_string(),
            message: msg.into(),
            details: None,
        }
    }
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

// ---------------------------------------------------------------------------
// Diff / Merge types (P3-UI-05)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffFile {
    pub path: String,
    pub added: u64,
    pub removed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDiffPayload {
    pub run_id: String,
    pub agent_key: String,
    pub base_ref: String,
    pub branch: Option<String>,
    pub mergeable: Option<bool>,
    pub gate_failures: Vec<String>,
    pub diff_text: String,
    pub files: Vec<DiffFile>,
    pub diff_available: bool,
    pub source: String,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergePreviewPayload {
    pub agent_key: String,
    pub branch: String,
    pub success: bool,
    pub has_conflicts: bool,
    pub stdout: String,
    pub stderr: String,
    pub report_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeExecutionPayload {
    pub agent_key: String,
    pub branch: String,
    pub success: bool,
    pub message: String,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}
