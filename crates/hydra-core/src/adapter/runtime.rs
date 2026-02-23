use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::supervisor::AgentCommand;

/// Normalized event type from agent output streams.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventType {
    Message,
    ToolCall,
    ToolResult,
    Progress,
    Completed,
    Failed,
    Usage,
    Unknown,
}

/// A single normalized event parsed from adapter output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub event_type: AgentEventType,
    pub data: serde_json::Value,
    pub raw_line: Option<String>,
}

/// Request to spawn an agent process.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub task_prompt: String,
    pub worktree_path: PathBuf,
    pub timeout_seconds: u64,
    pub force_edit: bool,
    pub output_json_stream: bool,
}

/// Runtime adapter trait - extends probes with execution capability.
///
/// Each adapter implements this to translate between Hydra's normalized
/// event model and the adapter's native CLI output format.
pub trait AdapterRuntime: Send + Sync {
    /// Build the [`AgentCommand`] to execute this adapter.
    fn build_command(&self, req: &SpawnRequest) -> AgentCommand;

    /// Parse a single line of output into a normalized event.
    ///
    /// Returns `None` if the line cannot be parsed (e.g. empty or non-JSON).
    fn parse_line(&self, line: &str) -> Option<AgentEvent>;

    /// Fallback: parse raw bytes into zero or more events.
    fn parse_raw(&self, chunk: &[u8]) -> Vec<AgentEvent>;
}
