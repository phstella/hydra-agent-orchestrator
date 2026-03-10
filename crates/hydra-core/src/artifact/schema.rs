use serde::{Deserialize, Serialize};

use super::events::{EventKind, RunEvent};

/// Well-known error message fragments used to classify agent failure causes.
/// Use these instead of bare string literals to prevent silent classification drift.
const CANCELLED_INDICATOR: &str = "cancelled";
const TIMED_OUT_INDICATOR: &str = "timed out";

/// Versioned event schema definition.
/// All EventKind variants are enumerated here for stability guarantees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSchemaDefinition {
    pub version: u32,
    pub event_kinds: Vec<String>,
}

impl EventSchemaDefinition {
    pub fn current() -> Self {
        Self {
            version: 1,
            event_kinds: vec![
                "run_started".to_string(),
                "run_completed".to_string(),
                "run_failed".to_string(),
                "agent_started".to_string(),
                "agent_completed".to_string(),
                "agent_failed".to_string(),
                "agent_stdout".to_string(),
                "agent_stderr".to_string(),
                "score_started".to_string(),
                "score_finished".to_string(),
                "merge_ready".to_string(),
                "merge_succeeded".to_string(),
                "merge_conflict".to_string(),
            ],
        }
    }
}

/// Run health metrics computable from persisted artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunHealthMetrics {
    pub total_agents: u32,
    pub agents_completed: u32,
    pub agents_failed: u32,
    pub success_rate: f64,
    pub total_events: u32,
    pub orchestration_overhead_ms: Option<u64>,
    pub adapter_errors: u32,
}

impl RunHealthMetrics {
    /// Compute health metrics from a list of events.
    pub fn from_events(events: &[RunEvent]) -> Self {
        let total_events = events.len() as u32;

        let agents_started = events
            .iter()
            .filter(|e| e.kind == EventKind::AgentStarted)
            .count() as u32;

        let agents_completed = events
            .iter()
            .filter(|e| e.kind == EventKind::AgentCompleted)
            .count() as u32;

        let agents_failed = events
            .iter()
            .filter(|e| e.kind == EventKind::AgentFailed)
            .count() as u32;

        let total_agents = agents_started;

        let success_rate = if total_agents == 0 {
            0.0
        } else {
            agents_completed as f64 / total_agents as f64
        };

        let orchestration_overhead_ms = compute_overhead(events);

        let adapter_errors = events
            .iter()
            .filter(|e| {
                e.kind == EventKind::AgentFailed
                    && e.data
                        .get("error")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| {
                            !s.contains(CANCELLED_INDICATOR) && !s.contains(TIMED_OUT_INDICATOR)
                        })
            })
            .count() as u32;

        RunHealthMetrics {
            total_agents,
            agents_completed,
            agents_failed,
            success_rate,
            total_events,
            orchestration_overhead_ms,
            adapter_errors,
        }
    }
}

fn compute_overhead(events: &[RunEvent]) -> Option<u64> {
    let run_started = events.iter().find(|e| e.kind == EventKind::RunStarted)?;
    let first_agent = events.iter().find(|e| e.kind == EventKind::AgentStarted)?;

    let overhead = first_agent
        .timestamp
        .signed_duration_since(run_started.timestamp);
    Some(overhead.num_milliseconds().unsigned_abs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_event(kind: EventKind, agent_key: Option<&str>) -> RunEvent {
        RunEvent {
            timestamp: Utc::now(),
            kind,
            agent_key: agent_key.map(|s| s.to_string()),
            data: serde_json::json!({}),
        }
    }

    #[test]
    fn schema_definition_lists_all_event_kinds() {
        let schema = EventSchemaDefinition::current();
        assert_eq!(schema.version, 1);
        assert!(schema.event_kinds.contains(&"run_started".to_string()));
        assert!(schema.event_kinds.contains(&"agent_completed".to_string()));
        assert!(schema.event_kinds.contains(&"merge_conflict".to_string()));
        assert_eq!(schema.event_kinds.len(), 13);
    }

    #[test]
    fn health_metrics_from_successful_run() {
        let events = vec![
            make_event(EventKind::RunStarted, None),
            make_event(EventKind::AgentStarted, Some("claude")),
            make_event(EventKind::AgentStarted, Some("codex")),
            make_event(EventKind::AgentCompleted, Some("claude")),
            make_event(EventKind::AgentCompleted, Some("codex")),
            make_event(EventKind::RunCompleted, None),
        ];

        let metrics = RunHealthMetrics::from_events(&events);
        assert_eq!(metrics.total_agents, 2);
        assert_eq!(metrics.agents_completed, 2);
        assert_eq!(metrics.agents_failed, 0);
        assert!((metrics.success_rate - 1.0).abs() < 0.01);
        assert_eq!(metrics.adapter_errors, 0);
    }

    #[test]
    fn health_metrics_with_one_failure() {
        let events = vec![
            make_event(EventKind::RunStarted, None),
            make_event(EventKind::AgentStarted, Some("claude")),
            make_event(EventKind::AgentStarted, Some("codex")),
            make_event(EventKind::AgentCompleted, Some("claude")),
            RunEvent {
                timestamp: Utc::now(),
                kind: EventKind::AgentFailed,
                agent_key: Some("codex".to_string()),
                data: serde_json::json!({"error": "process crashed"}),
            },
            make_event(EventKind::RunCompleted, None),
        ];

        let metrics = RunHealthMetrics::from_events(&events);
        assert_eq!(metrics.total_agents, 2);
        assert_eq!(metrics.agents_completed, 1);
        assert_eq!(metrics.agents_failed, 1);
        assert!((metrics.success_rate - 0.5).abs() < 0.01);
        assert_eq!(metrics.adapter_errors, 1);
    }

    #[test]
    fn health_metrics_empty_events() {
        let metrics = RunHealthMetrics::from_events(&[]);
        assert_eq!(metrics.total_agents, 0);
        assert_eq!(metrics.success_rate, 0.0);
        assert!(metrics.orchestration_overhead_ms.is_none());
    }

    #[test]
    fn schema_definition_roundtrip() {
        let schema = EventSchemaDefinition::current();
        let json = serde_json::to_string(&schema).unwrap();
        let loaded: EventSchemaDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.version, schema.version);
        assert_eq!(loaded.event_kinds.len(), schema.event_kinds.len());
    }
}
