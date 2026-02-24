use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ArtifactError;
use crate::security::SecretRedactor;

/// Deterministic directory layout for a single interactive session's artifacts.
///
/// Structure:
/// ```text
/// .hydra/sessions/<session_id>/
///   session.json
///   events.jsonl
///   transcript.ansi.log
///   summary.json
/// ```
#[derive(Debug, Clone)]
pub struct SessionLayout {
    session_id: String,
    base_dir: PathBuf,
}

impl SessionLayout {
    pub fn new(hydra_root: &Path, session_id: &str) -> Self {
        let base_dir = hydra_root.join("sessions").join(session_id);
        Self {
            session_id: session_id.to_string(),
            base_dir,
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn session_json_path(&self) -> PathBuf {
        self.base_dir.join("session.json")
    }

    pub fn events_path(&self) -> PathBuf {
        self.base_dir.join("events.jsonl")
    }

    pub fn transcript_path(&self) -> PathBuf {
        self.base_dir.join("transcript.ansi.log")
    }

    pub fn summary_path(&self) -> PathBuf {
        self.base_dir.join("summary.json")
    }

    pub fn create_dirs(&self) -> Result<(), ArtifactError> {
        std::fs::create_dir_all(&self.base_dir)?;
        Ok(())
    }

    pub fn list_sessions(hydra_root: &Path) -> Result<Vec<String>, ArtifactError> {
        let sessions_dir = hydra_root.join("sessions");
        if !sessions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        for entry in std::fs::read_dir(sessions_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    ids.push(name.to_string());
                }
            }
        }
        Ok(ids)
    }

    pub fn cleanup(&self) -> Result<(), ArtifactError> {
        if self.base_dir.exists() {
            std::fs::remove_dir_all(&self.base_dir)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Session metadata (session.json)
// ---------------------------------------------------------------------------

const SESSION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub schema_version: u32,
    pub session_id: String,
    pub agent_key: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: String,
    pub cwd: String,
    pub unsafe_mode: bool,
    pub experimental: bool,
}

impl SessionMetadata {
    pub fn new(
        session_id: &str,
        agent_key: &str,
        started_at: &str,
        cwd: &str,
        unsafe_mode: bool,
        experimental: bool,
    ) -> Self {
        Self {
            schema_version: SESSION_SCHEMA_VERSION,
            session_id: session_id.to_string(),
            agent_key: agent_key.to_string(),
            started_at: started_at.to_string(),
            ended_at: None,
            status: "running".to_string(),
            cwd: cwd.to_string(),
            unsafe_mode,
            experimental,
        }
    }

    pub fn write_to(&self, path: &Path) -> Result<(), ArtifactError> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn read_from(path: &Path) -> Result<Self, ArtifactError> {
        let data = std::fs::read_to_string(path)?;
        let meta: Self = serde_json::from_str(&data)?;
        Ok(meta)
    }
}

// ---------------------------------------------------------------------------
// Session event JSONL writer (events.jsonl)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub data: serde_json::Value,
}

impl SessionEvent {
    pub fn new(event_type: &str, data: serde_json::Value) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            data,
        }
    }
}

pub struct SessionEventWriter {
    file: std::fs::File,
    redactor: SecretRedactor,
}

impl SessionEventWriter {
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

    pub fn write_event(&mut self, event: &SessionEvent) -> Result<(), ArtifactError> {
        let line = serde_json::to_string(event)?;
        let redacted = self.redactor.redact_line(&line);
        writeln!(self.file, "{}", redacted)?;
        self.file.flush()?;
        Ok(())
    }
}

pub struct SessionEventReader;

impl SessionEventReader {
    pub fn read_all(path: &Path) -> Result<Vec<SessionEvent>, ArtifactError> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let mut events = Vec::new();
        use std::io::BufRead;
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let event: SessionEvent = serde_json::from_str(&line)?;
            events.push(event);
        }
        Ok(events)
    }
}

// ---------------------------------------------------------------------------
// Transcript writer (transcript.ansi.log)
// ---------------------------------------------------------------------------

pub struct TranscriptWriter {
    file: std::fs::File,
    path: PathBuf,
    redactor: SecretRedactor,
}

impl TranscriptWriter {
    pub fn create(path: &Path) -> Result<Self, ArtifactError> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            redactor: SecretRedactor::new(),
        })
    }

    pub fn append_output(&mut self, raw_bytes: &[u8]) -> Result<(), ArtifactError> {
        let text = String::from_utf8_lossy(raw_bytes);
        let redacted = self.redactor.redact_line(&text);
        self.file.write_all(redacted.as_bytes())?;
        self.file.flush()?;
        Ok(())
    }

    pub fn append_user_input(&mut self, input: &str) -> Result<(), ArtifactError> {
        let marker = format!("\n--- USER INPUT ---\n{}\n--- END INPUT ---\n", input);
        let redacted = self.redactor.redact_line(&marker);
        self.file.write_all(redacted.as_bytes())?;
        self.file.flush()?;
        Ok(())
    }

    pub fn rewrite_with_full_redaction(&mut self) -> Result<(), ArtifactError> {
        self.file.flush()?;
        let content = std::fs::read_to_string(&self.path)?;
        let redacted = self.redactor.redact(&content).value;

        if redacted != content {
            std::fs::write(&self.path, redacted)?;
            self.file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Session summary (summary.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub schema_version: u32,
    pub session_id: String,
    pub agent_key: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: u64,
    pub event_count: u64,
    pub output_bytes: u64,
    pub user_input_count: u64,
}

impl SessionSummary {
    pub fn write_to(&self, path: &Path) -> Result<(), ArtifactError> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn read_from(path: &Path) -> Result<Self, ArtifactError> {
        let data = std::fs::read_to_string(path)?;
        let summary: Self = serde_json::from_str(&data)?;
        Ok(summary)
    }
}

// ---------------------------------------------------------------------------
// Aggregate writer that manages all artifacts for one session
// ---------------------------------------------------------------------------

pub struct SessionArtifactWriter {
    layout: SessionLayout,
    metadata: SessionMetadata,
    event_writer: SessionEventWriter,
    transcript_writer: TranscriptWriter,
    event_count: u64,
    output_bytes: u64,
    user_input_count: u64,
}

impl SessionArtifactWriter {
    pub fn init(
        hydra_root: &Path,
        session_id: &str,
        agent_key: &str,
        started_at: &str,
        cwd: &str,
        unsafe_mode: bool,
        experimental: bool,
    ) -> Result<Self, ArtifactError> {
        let layout = SessionLayout::new(hydra_root, session_id);
        layout.create_dirs()?;

        let metadata = SessionMetadata::new(
            session_id,
            agent_key,
            started_at,
            cwd,
            unsafe_mode,
            experimental,
        );
        metadata.write_to(&layout.session_json_path())?;

        let event_writer = SessionEventWriter::create(&layout.events_path())?;
        let transcript_writer = TranscriptWriter::create(&layout.transcript_path())?;

        let mut writer = Self {
            layout,
            metadata,
            event_writer,
            transcript_writer,
            event_count: 0,
            output_bytes: 0,
            user_input_count: 0,
        };

        writer.event_writer.write_event(&SessionEvent::new(
            "session_started",
            serde_json::json!({
                "agent_key": agent_key,
                "cwd": cwd,
            }),
        ))?;
        writer.event_count += 1;

        Ok(writer)
    }

    pub fn record_output(&mut self, raw_bytes: &[u8]) -> Result<(), ArtifactError> {
        let text = String::from_utf8_lossy(raw_bytes);
        self.event_writer.write_event(&SessionEvent::new(
            "output",
            serde_json::json!({ "length": raw_bytes.len() }),
        ))?;
        self.transcript_writer.append_output(raw_bytes)?;
        self.event_count += 1;
        self.output_bytes += raw_bytes.len() as u64;
        let _ = text;
        Ok(())
    }

    pub fn record_user_input(&mut self, input: &str) -> Result<(), ArtifactError> {
        self.event_writer.write_event(&SessionEvent::new(
            "user_input",
            serde_json::json!({ "input": input }),
        ))?;
        self.transcript_writer.append_user_input(input)?;
        self.event_count += 1;
        self.user_input_count += 1;
        Ok(())
    }

    pub fn record_event(
        &mut self,
        event_type: &str,
        data: serde_json::Value,
    ) -> Result<(), ArtifactError> {
        self.event_writer
            .write_event(&SessionEvent::new(event_type, data))?;
        self.event_count += 1;
        Ok(())
    }

    pub fn finalize(
        &mut self,
        status: &str,
        ended_at: &str,
        duration_ms: u64,
    ) -> Result<(), ArtifactError> {
        self.transcript_writer.rewrite_with_full_redaction()?;

        self.event_writer.write_event(&SessionEvent::new(
            &format!("session_{status}"),
            serde_json::json!({ "duration_ms": duration_ms }),
        ))?;
        self.event_count += 1;

        self.metadata.status = status.to_string();
        self.metadata.ended_at = Some(ended_at.to_string());
        self.metadata.write_to(&self.layout.session_json_path())?;

        let summary = SessionSummary {
            schema_version: SESSION_SCHEMA_VERSION,
            session_id: self.metadata.session_id.clone(),
            agent_key: self.metadata.agent_key.clone(),
            status: status.to_string(),
            started_at: self.metadata.started_at.clone(),
            ended_at: ended_at.to_string(),
            duration_ms,
            event_count: self.event_count,
            output_bytes: self.output_bytes,
            user_input_count: self.user_input_count,
        };
        summary.write_to(&self.layout.summary_path())?;

        Ok(())
    }

    pub fn layout(&self) -> &SessionLayout {
        &self.layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn session_layout_paths_are_deterministic() {
        let layout = SessionLayout::new(Path::new("/tmp/.hydra"), "sess-001");
        assert_eq!(
            layout.base_dir(),
            Path::new("/tmp/.hydra/sessions/sess-001")
        );
        assert!(layout.session_json_path().ends_with("session.json"));
        assert!(layout.events_path().ends_with("events.jsonl"));
        assert!(layout.transcript_path().ends_with("transcript.ansi.log"));
        assert!(layout.summary_path().ends_with("summary.json"));
    }

    #[test]
    fn session_layout_create_and_cleanup() {
        let tmp = TempDir::new().unwrap();
        let hydra_root = tmp.path().join(".hydra");
        let layout = SessionLayout::new(&hydra_root, "test-session");

        layout.create_dirs().unwrap();
        assert!(layout.base_dir().exists());

        layout.cleanup().unwrap();
        assert!(!layout.base_dir().exists());
    }

    #[test]
    fn session_layout_list_sessions() {
        let tmp = TempDir::new().unwrap();
        let hydra_root = tmp.path().join(".hydra");

        SessionLayout::new(&hydra_root, "s1").create_dirs().unwrap();
        SessionLayout::new(&hydra_root, "s2").create_dirs().unwrap();

        let mut sessions = SessionLayout::list_sessions(&hydra_root).unwrap();
        sessions.sort();
        assert_eq!(sessions, vec!["s1", "s2"]);
    }

    #[test]
    fn session_layout_list_empty_when_no_dir() {
        let tmp = TempDir::new().unwrap();
        let sessions = SessionLayout::list_sessions(&tmp.path().join(".hydra")).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn session_metadata_write_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("session.json");

        let meta = SessionMetadata::new(
            "s1",
            "claude",
            "2026-02-24T00:00:00Z",
            "/repo",
            false,
            false,
        );
        meta.write_to(&path).unwrap();

        let read_back = SessionMetadata::read_from(&path).unwrap();
        assert_eq!(read_back.schema_version, 1);
        assert_eq!(read_back.session_id, "s1");
        assert_eq!(read_back.agent_key, "claude");
        assert_eq!(read_back.status, "running");
        assert!(read_back.ended_at.is_none());
        assert!(!read_back.unsafe_mode);
        assert!(!read_back.experimental);
    }

    #[test]
    fn session_event_write_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        let mut writer = SessionEventWriter::create(&path).unwrap();
        writer
            .write_event(&SessionEvent::new("session_started", serde_json::json!({})))
            .unwrap();
        writer
            .write_event(&SessionEvent::new(
                "output",
                serde_json::json!({ "length": 42 }),
            ))
            .unwrap();
        writer
            .write_event(&SessionEvent::new(
                "session_completed",
                serde_json::json!({ "duration_ms": 5000 }),
            ))
            .unwrap();
        drop(writer);

        let events = SessionEventReader::read_all(&path).unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, "session_started");
        assert_eq!(events[1].event_type, "output");
        assert_eq!(events[2].event_type, "session_completed");
    }

    #[test]
    fn session_event_writer_redacts_secrets() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        let mut writer = SessionEventWriter::create(&path).unwrap();
        writer
            .write_event(&SessionEvent::new(
                "output",
                serde_json::json!({ "text": "key=sk-proj-super-secret" }),
            ))
            .unwrap();
        drop(writer);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("sk-proj-"), "secret should be redacted");
        assert!(content.contains("[REDACTED:OPENAI_KEY]"));
    }

    #[test]
    fn transcript_writer_redacts_secrets() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("transcript.ansi.log");

        let mut writer = TranscriptWriter::create(&path).unwrap();
        writer.append_output(b"token=ghp_abc123secret").unwrap();
        writer.append_user_input("my key is sk-ant-secret").unwrap();
        drop(writer);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            !content.contains("ghp_abc123"),
            "GitHub PAT should be redacted"
        );
        assert!(
            !content.contains("sk-ant-secret"),
            "Anthropic key should be redacted"
        );
    }

    #[test]
    fn session_summary_write_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("summary.json");

        let summary = SessionSummary {
            schema_version: 1,
            session_id: "s1".to_string(),
            agent_key: "claude".to_string(),
            status: "completed".to_string(),
            started_at: "2026-02-24T00:00:00Z".to_string(),
            ended_at: "2026-02-24T00:05:00Z".to_string(),
            duration_ms: 300_000,
            event_count: 42,
            output_bytes: 10_000,
            user_input_count: 3,
        };
        summary.write_to(&path).unwrap();

        let read_back = SessionSummary::read_from(&path).unwrap();
        assert_eq!(read_back.session_id, "s1");
        assert_eq!(read_back.status, "completed");
        assert_eq!(read_back.duration_ms, 300_000);
        assert_eq!(read_back.event_count, 42);
        assert_eq!(read_back.output_bytes, 10_000);
        assert_eq!(read_back.user_input_count, 3);
    }

    #[test]
    fn session_artifact_writer_full_lifecycle() {
        let tmp = TempDir::new().unwrap();
        let hydra_root = tmp.path().join(".hydra");

        let mut writer = SessionArtifactWriter::init(
            &hydra_root,
            "lifecycle-test",
            "claude",
            "2026-02-24T00:00:00Z",
            "/repo",
            false,
            false,
        )
        .unwrap();

        // session.json created on init
        let meta = SessionMetadata::read_from(&writer.layout().session_json_path()).unwrap();
        assert_eq!(meta.status, "running");

        writer.record_output(b"hello from agent\n").unwrap();
        writer.record_user_input("fix the bug").unwrap();
        writer.record_output(b"fixing...\n").unwrap();

        writer
            .finalize("completed", "2026-02-24T00:05:00Z", 300_000)
            .unwrap();

        // Verify session.json updated
        let meta_final = SessionMetadata::read_from(&writer.layout().session_json_path()).unwrap();
        assert_eq!(meta_final.status, "completed");
        assert_eq!(meta_final.ended_at.as_deref(), Some("2026-02-24T00:05:00Z"));

        // Verify events.jsonl
        let events = SessionEventReader::read_all(&writer.layout().events_path()).unwrap();
        assert!(
            events.len() >= 5,
            "expected start + 2 output + 1 input + finalize, got {}",
            events.len()
        );
        assert_eq!(events[0].event_type, "session_started");
        assert_eq!(events[1].event_type, "output");
        assert_eq!(events[2].event_type, "user_input");
        assert_eq!(events.last().unwrap().event_type, "session_completed");

        // Verify transcript.ansi.log has agent output and user input markers
        let transcript = std::fs::read_to_string(writer.layout().transcript_path()).unwrap();
        assert!(transcript.contains("hello from agent"));
        assert!(transcript.contains("USER INPUT"));
        assert!(transcript.contains("fix the bug"));

        // Verify summary.json
        let summary = SessionSummary::read_from(&writer.layout().summary_path()).unwrap();
        assert_eq!(summary.status, "completed");
        assert_eq!(summary.duration_ms, 300_000);
        assert_eq!(summary.output_bytes, 27); // "hello from agent\n" + "fixing...\n"
        assert_eq!(summary.user_input_count, 1);
    }

    #[test]
    fn session_artifact_writer_finalize_failed() {
        let tmp = TempDir::new().unwrap();
        let hydra_root = tmp.path().join(".hydra");

        let mut writer = SessionArtifactWriter::init(
            &hydra_root,
            "fail-test",
            "codex",
            "2026-02-24T00:00:00Z",
            "/repo",
            false,
            true,
        )
        .unwrap();

        writer.record_output(b"partial output").unwrap();
        writer
            .finalize("failed", "2026-02-24T00:01:00Z", 60_000)
            .unwrap();

        let meta = SessionMetadata::read_from(&writer.layout().session_json_path()).unwrap();
        assert_eq!(meta.status, "failed");
        assert!(meta.experimental);

        let summary = SessionSummary::read_from(&writer.layout().summary_path()).unwrap();
        assert_eq!(summary.status, "failed");
    }

    #[test]
    fn session_artifact_writer_finalize_stopped() {
        let tmp = TempDir::new().unwrap();
        let hydra_root = tmp.path().join(".hydra");

        let mut writer = SessionArtifactWriter::init(
            &hydra_root,
            "stop-test",
            "claude",
            "2026-02-24T00:00:00Z",
            "/repo",
            true,
            false,
        )
        .unwrap();

        writer
            .finalize("stopped", "2026-02-24T00:00:30Z", 30_000)
            .unwrap();

        let meta = SessionMetadata::read_from(&writer.layout().session_json_path()).unwrap();
        assert_eq!(meta.status, "stopped");
        assert!(meta.unsafe_mode);

        let summary = SessionSummary::read_from(&writer.layout().summary_path()).unwrap();
        assert_eq!(summary.status, "stopped");
        assert_eq!(summary.duration_ms, 30_000);
    }

    #[test]
    fn session_artifact_writer_redacts_secret_spanning_output_chunks_on_finalize() {
        let tmp = TempDir::new().unwrap();
        let hydra_root = tmp.path().join(".hydra");

        let mut writer = SessionArtifactWriter::init(
            &hydra_root,
            "chunk-redact",
            "claude",
            "2026-02-24T00:00:00Z",
            "/repo",
            false,
            false,
        )
        .unwrap();

        writer
            .record_output(b"OPENAI_API_KEY=sk-proj-partial")
            .unwrap();
        writer.record_output(b"secretvalue\n").unwrap();
        writer
            .finalize("completed", "2026-02-24T00:00:05Z", 5_000)
            .unwrap();

        let transcript = std::fs::read_to_string(writer.layout().transcript_path()).unwrap();
        assert!(
            !transcript.contains("sk-proj-"),
            "full transcript redaction should catch cross-chunk secrets"
        );
        assert!(transcript.contains("[REDACTED:OPENAI_KEY]"));
    }
}
