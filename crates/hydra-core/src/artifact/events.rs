use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use super::ArtifactError;
use crate::security::SecretRedactor;

/// Normalized event kinds persisted to events.jsonl.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    RunStarted,
    RunCompleted,
    RunFailed,
    AgentStarted,
    AgentCompleted,
    AgentFailed,
    AgentStdout,
    AgentStderr,
    ScoreStarted,
    ScoreFinished,
    MergeReady,
    MergeSucceeded,
    MergeConflict,
}

/// A single event line in `events.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub timestamp: DateTime<Utc>,
    pub kind: EventKind,
    pub agent_key: Option<String>,
    pub data: serde_json::Value,
}

impl RunEvent {
    pub fn new(kind: EventKind, agent_key: Option<String>, data: serde_json::Value) -> Self {
        Self {
            timestamp: Utc::now(),
            kind,
            agent_key,
            data,
        }
    }
}

/// Append-only writer for events.jsonl.
pub struct EventWriter {
    file: std::fs::File,
    redactor: SecretRedactor,
}

impl EventWriter {
    pub fn create(path: &Path) -> Result<Self, ArtifactError> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            file,
            redactor: SecretRedactor::new(),
        })
    }

    pub fn write_event(&mut self, event: &RunEvent) -> Result<(), ArtifactError> {
        let line = serde_json::to_string(event)?;
        let redacted = self.redactor.redact_line(&line);
        writeln!(self.file, "{}", redacted)?;
        self.file.flush()?;
        Ok(())
    }
}

/// Reader for replaying events from events.jsonl.
pub struct EventReader;

impl EventReader {
    pub fn read_all(path: &Path) -> Result<Vec<RunEvent>, ArtifactError> {
        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let event: RunEvent = serde_json::from_str(&line)?;
            events.push(event);
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn event_write_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        let mut writer = EventWriter::create(&path).unwrap();

        let e1 = RunEvent::new(
            EventKind::RunStarted,
            None,
            serde_json::json!({"task": "test"}),
        );
        let e2 = RunEvent::new(
            EventKind::AgentStarted,
            Some("claude".to_string()),
            serde_json::json!({}),
        );
        let e3 = RunEvent::new(
            EventKind::RunCompleted,
            None,
            serde_json::json!({"ok": true}),
        );

        writer.write_event(&e1).unwrap();
        writer.write_event(&e2).unwrap();
        writer.write_event(&e3).unwrap();
        drop(writer);

        let events = EventReader::read_all(&path).unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].kind, EventKind::RunStarted);
        assert_eq!(events[1].kind, EventKind::AgentStarted);
        assert_eq!(events[1].agent_key.as_deref(), Some("claude"));
        assert_eq!(events[2].kind, EventKind::RunCompleted);
    }

    #[test]
    fn event_is_one_line_per_entry() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        let mut writer = EventWriter::create(&path).unwrap();
        writer
            .write_event(&RunEvent::new(
                EventKind::RunStarted,
                None,
                serde_json::json!({}),
            ))
            .unwrap();
        writer
            .write_event(&RunEvent::new(
                EventKind::RunCompleted,
                None,
                serde_json::json!({}),
            ))
            .unwrap();
        drop(writer);

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        for line in &lines {
            let _: RunEvent = serde_json::from_str(line).unwrap();
        }
    }

    #[test]
    fn empty_events_file_reads_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");
        std::fs::write(&path, "").unwrap();

        let events = EventReader::read_all(&path).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn all_event_kinds_serialize() {
        let kinds = vec![
            EventKind::RunStarted,
            EventKind::RunCompleted,
            EventKind::RunFailed,
            EventKind::AgentStarted,
            EventKind::AgentCompleted,
            EventKind::AgentFailed,
            EventKind::AgentStdout,
            EventKind::AgentStderr,
            EventKind::ScoreStarted,
            EventKind::ScoreFinished,
            EventKind::MergeReady,
            EventKind::MergeSucceeded,
            EventKind::MergeConflict,
        ];

        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let back: EventKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, kind);
        }
    }

    #[test]
    fn event_writer_redacts_secrets_before_persisting() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");
        let mut writer = EventWriter::create(&path).unwrap();

        writer
            .write_event(&RunEvent::new(
                EventKind::AgentStdout,
                Some("claude".to_string()),
                serde_json::json!({
                    "line": "OPENAI_API_KEY=sk-proj-super-secret"
                }),
            ))
            .unwrap();
        drop(writer);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("sk-proj-"));
        assert!(content.contains("[REDACTED:OPENAI_KEY]"));

        let events = EventReader::read_all(&path).unwrap();
        let line = events[0].data["line"].as_str().unwrap();
        assert!(line.contains("[REDACTED:OPENAI_KEY]"));
    }
}
