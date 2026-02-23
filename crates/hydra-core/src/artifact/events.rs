use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single event in a run's JSONL event log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub timestamp: DateTime<Utc>,
    pub run_id: Uuid,
    pub event_type: EventType,
    pub agent_key: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    RunStarted,
    RunCompleted,
    RunFailed,
    AgentStarted,
    AgentCompleted,
    AgentFailed,
    AgentStdout,
    AgentStderr,
    AgentMessage,
    AgentToolCall,
    AgentToolResult,
    AgentProgress,
    AgentUsage,
    ScoreStarted,
    ScoreFinished,
    MergeReady,
    MergeSucceeded,
    MergeConflict,
}
