//! Observability contract for Hydra run events and health metrics.
//!
//! Defines the canonical event schema and provides run health
//! metrics computable from captured artifacts.

use serde::{Deserialize, Serialize};

use crate::artifact::events::{EventType, RunEvent};

/// Schema version for events and manifests.
pub const SCHEMA_VERSION: &str = "1.0.0";

/// Event schema definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSchema {
    pub version: String,
    pub event_types: Vec<EventTypeDefinition>,
}

/// Definition of a single event type in the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTypeDefinition {
    pub name: String,
    pub description: String,
    pub required_fields: Vec<String>,
}

/// Run health metrics computable from artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunHealthMetrics {
    pub success_rate: f64,
    pub orchestration_overhead_ms: u64,
    pub adapter_error_count: u32,
    pub total_events: u32,
    pub agents_completed: u32,
    pub agents_failed: u32,
}

impl RunHealthMetrics {
    /// Compute health metrics from a run's events.
    pub fn from_events(events: &[RunEvent]) -> Self {
        let total_events = events.len() as u32;

        let agents_completed = events
            .iter()
            .filter(|e| e.event_type == EventType::AgentCompleted)
            .count() as u32;

        let agents_failed = events
            .iter()
            .filter(|e| e.event_type == EventType::AgentFailed)
            .count() as u32;

        let adapter_error_count = events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::AgentFailed | EventType::RunFailed))
            .count() as u32;

        let total_agents = agents_completed + agents_failed;
        let success_rate = if total_agents > 0 {
            agents_completed as f64 / total_agents as f64
        } else {
            0.0
        };

        // Compute orchestration overhead: time between RunStarted and first AgentStarted,
        // plus time between last AgentCompleted/AgentFailed and RunCompleted.
        let orchestration_overhead_ms = compute_overhead(events);

        Self {
            success_rate,
            orchestration_overhead_ms,
            adapter_error_count,
            total_events,
            agents_completed,
            agents_failed,
        }
    }
}

/// Compute orchestration overhead from event timestamps.
fn compute_overhead(events: &[RunEvent]) -> u64 {
    let run_started = events
        .iter()
        .find(|e| e.event_type == EventType::RunStarted)
        .map(|e| e.timestamp);

    let first_agent_started = events
        .iter()
        .find(|e| e.event_type == EventType::AgentStarted)
        .map(|e| e.timestamp);

    let last_agent_done = events
        .iter()
        .filter(|e| {
            matches!(
                e.event_type,
                EventType::AgentCompleted | EventType::AgentFailed
            )
        })
        .max_by_key(|e| e.timestamp)
        .map(|e| e.timestamp);

    let run_completed = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::RunCompleted | EventType::RunFailed))
        .map(|e| e.timestamp);

    let mut overhead_ms = 0u64;

    // Startup overhead: RunStarted -> first AgentStarted
    if let (Some(start), Some(agent_start)) = (run_started, first_agent_started) {
        let diff = agent_start.signed_duration_since(start);
        overhead_ms += diff.num_milliseconds().unsigned_abs();
    }

    // Teardown overhead: last AgentDone -> RunCompleted
    if let (Some(last_done), Some(end)) = (last_agent_done, run_completed) {
        let diff = end.signed_duration_since(last_done);
        overhead_ms += diff.num_milliseconds().unsigned_abs();
    }

    overhead_ms
}

/// Get the canonical event schema definition.
pub fn event_schema() -> EventSchema {
    EventSchema {
        version: SCHEMA_VERSION.to_string(),
        event_types: vec![
            EventTypeDefinition {
                name: "run_started".to_string(),
                description: "Emitted when a run begins".to_string(),
                required_fields: vec!["run_id".to_string(), "timestamp".to_string()],
            },
            EventTypeDefinition {
                name: "run_completed".to_string(),
                description: "Emitted when a run finishes successfully".to_string(),
                required_fields: vec!["run_id".to_string(), "timestamp".to_string()],
            },
            EventTypeDefinition {
                name: "run_failed".to_string(),
                description: "Emitted when a run fails".to_string(),
                required_fields: vec![
                    "run_id".to_string(),
                    "timestamp".to_string(),
                    "data".to_string(),
                ],
            },
            EventTypeDefinition {
                name: "agent_started".to_string(),
                description: "Emitted when an agent process begins".to_string(),
                required_fields: vec![
                    "run_id".to_string(),
                    "timestamp".to_string(),
                    "agent_key".to_string(),
                ],
            },
            EventTypeDefinition {
                name: "agent_completed".to_string(),
                description: "Emitted when an agent process finishes successfully".to_string(),
                required_fields: vec![
                    "run_id".to_string(),
                    "timestamp".to_string(),
                    "agent_key".to_string(),
                ],
            },
            EventTypeDefinition {
                name: "agent_failed".to_string(),
                description: "Emitted when an agent process fails".to_string(),
                required_fields: vec![
                    "run_id".to_string(),
                    "timestamp".to_string(),
                    "agent_key".to_string(),
                    "data".to_string(),
                ],
            },
            EventTypeDefinition {
                name: "agent_usage".to_string(),
                description: "Token usage report from an agent".to_string(),
                required_fields: vec![
                    "run_id".to_string(),
                    "timestamp".to_string(),
                    "agent_key".to_string(),
                    "data".to_string(),
                ],
            },
            EventTypeDefinition {
                name: "score_started".to_string(),
                description: "Emitted when scoring begins for an agent".to_string(),
                required_fields: vec![
                    "run_id".to_string(),
                    "timestamp".to_string(),
                    "agent_key".to_string(),
                ],
            },
            EventTypeDefinition {
                name: "score_finished".to_string(),
                description: "Emitted when scoring completes for an agent".to_string(),
                required_fields: vec![
                    "run_id".to_string(),
                    "timestamp".to_string(),
                    "agent_key".to_string(),
                    "data".to_string(),
                ],
            },
            EventTypeDefinition {
                name: "merge_ready".to_string(),
                description: "Emitted when an agent result is ready for merge".to_string(),
                required_fields: vec![
                    "run_id".to_string(),
                    "timestamp".to_string(),
                    "agent_key".to_string(),
                ],
            },
            EventTypeDefinition {
                name: "merge_succeeded".to_string(),
                description: "Emitted when a merge completes successfully".to_string(),
                required_fields: vec![
                    "run_id".to_string(),
                    "timestamp".to_string(),
                    "agent_key".to_string(),
                ],
            },
            EventTypeDefinition {
                name: "merge_conflict".to_string(),
                description: "Emitted when a merge encounters conflicts".to_string(),
                required_fields: vec![
                    "run_id".to_string(),
                    "timestamp".to_string(),
                    "agent_key".to_string(),
                    "data".to_string(),
                ],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    fn make_event(event_type: EventType, agent_key: Option<&str>, offset_ms: i64) -> RunEvent {
        RunEvent {
            timestamp: Utc::now() + Duration::milliseconds(offset_ms),
            run_id: Uuid::nil(),
            event_type,
            agent_key: agent_key.map(|s| s.to_string()),
            data: serde_json::Value::Null,
        }
    }

    #[test]
    fn schema_version_is_set() {
        assert_eq!(SCHEMA_VERSION, "1.0.0");
    }

    #[test]
    fn event_schema_has_types() {
        let schema = event_schema();
        assert_eq!(schema.version, "1.0.0");
        assert!(!schema.event_types.is_empty());

        // Check that key event types are defined
        let names: Vec<&str> = schema.event_types.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"run_started"));
        assert!(names.contains(&"run_completed"));
        assert!(names.contains(&"agent_started"));
        assert!(names.contains(&"agent_completed"));
        assert!(names.contains(&"agent_failed"));
        assert!(names.contains(&"agent_usage"));
    }

    #[test]
    fn event_types_have_required_fields() {
        let schema = event_schema();
        for et in &schema.event_types {
            assert!(
                !et.required_fields.is_empty(),
                "event type {} has no required fields",
                et.name
            );
            assert!(
                et.required_fields.contains(&"run_id".to_string()),
                "event type {} missing run_id",
                et.name
            );
            assert!(
                et.required_fields.contains(&"timestamp".to_string()),
                "event type {} missing timestamp",
                et.name
            );
        }
    }

    #[test]
    fn health_metrics_empty_events() {
        let metrics = RunHealthMetrics::from_events(&[]);
        assert_eq!(metrics.total_events, 0);
        assert_eq!(metrics.agents_completed, 0);
        assert_eq!(metrics.agents_failed, 0);
        assert_eq!(metrics.success_rate, 0.0);
        assert_eq!(metrics.adapter_error_count, 0);
    }

    #[test]
    fn health_metrics_all_success() {
        let events = vec![
            make_event(EventType::RunStarted, None, 0),
            make_event(EventType::AgentStarted, Some("claude"), 100),
            make_event(EventType::AgentStarted, Some("codex"), 150),
            make_event(EventType::AgentCompleted, Some("claude"), 5000),
            make_event(EventType::AgentCompleted, Some("codex"), 6000),
            make_event(EventType::RunCompleted, None, 6500),
        ];

        let metrics = RunHealthMetrics::from_events(&events);
        assert_eq!(metrics.total_events, 6);
        assert_eq!(metrics.agents_completed, 2);
        assert_eq!(metrics.agents_failed, 0);
        assert!((metrics.success_rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(metrics.adapter_error_count, 0);
    }

    #[test]
    fn health_metrics_partial_failure() {
        let events = vec![
            make_event(EventType::RunStarted, None, 0),
            make_event(EventType::AgentStarted, Some("claude"), 100),
            make_event(EventType::AgentStarted, Some("codex"), 150),
            make_event(EventType::AgentCompleted, Some("claude"), 5000),
            make_event(EventType::AgentFailed, Some("codex"), 3000),
            make_event(EventType::RunCompleted, None, 5500),
        ];

        let metrics = RunHealthMetrics::from_events(&events);
        assert_eq!(metrics.agents_completed, 1);
        assert_eq!(metrics.agents_failed, 1);
        assert!((metrics.success_rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(metrics.adapter_error_count, 1);
    }

    #[test]
    fn health_metrics_all_failed() {
        let events = vec![
            make_event(EventType::RunStarted, None, 0),
            make_event(EventType::AgentStarted, Some("claude"), 100),
            make_event(EventType::AgentFailed, Some("claude"), 1000),
            make_event(EventType::RunFailed, None, 1500),
        ];

        let metrics = RunHealthMetrics::from_events(&events);
        assert_eq!(metrics.agents_completed, 0);
        assert_eq!(metrics.agents_failed, 1);
        assert_eq!(metrics.success_rate, 0.0);
        // AgentFailed + RunFailed
        assert_eq!(metrics.adapter_error_count, 2);
    }

    #[test]
    fn orchestration_overhead_computed() {
        let events = vec![
            make_event(EventType::RunStarted, None, 0),
            make_event(EventType::AgentStarted, Some("claude"), 200),
            make_event(EventType::AgentCompleted, Some("claude"), 5000),
            make_event(EventType::RunCompleted, None, 5300),
        ];

        let metrics = RunHealthMetrics::from_events(&events);
        // Startup: 200ms, Teardown: 300ms = 500ms total
        assert_eq!(metrics.orchestration_overhead_ms, 500);
    }

    #[test]
    fn schema_serialization() {
        let schema = event_schema();
        let json = serde_json::to_string(&schema).expect("serialize");
        let deser: EventSchema = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deser.version, "1.0.0");
        assert_eq!(deser.event_types.len(), schema.event_types.len());
    }

    #[test]
    fn health_metrics_serialization() {
        let metrics = RunHealthMetrics {
            success_rate: 0.75,
            orchestration_overhead_ms: 500,
            adapter_error_count: 1,
            total_events: 10,
            agents_completed: 3,
            agents_failed: 1,
        };
        let json = serde_json::to_string(&metrics).expect("serialize");
        let deser: RunHealthMetrics = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deser.agents_completed, 3);
        assert!((deser.success_rate - 0.75).abs() < f64::EPSILON);
    }
}
