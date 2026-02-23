use std::path::PathBuf;

use hydra_core::adapter::{
    AdapterRuntime, AgentEventType, ClaudeAdapter, CodexAdapter, CursorAdapter, SpawnRequest,
};

fn sample_request() -> SpawnRequest {
    SpawnRequest {
        task_prompt: "Fix the bug in main.rs".to_string(),
        worktree_path: PathBuf::from("/tmp/hydra/worktree-abc"),
        timeout_seconds: 300,
        force_edit: false,
        output_json_stream: true,
    }
}

// ---------------------------------------------------------------------------
// Claude adapter: build_command
// ---------------------------------------------------------------------------

#[test]
fn claude_build_command_has_correct_program() {
    let adapter = ClaudeAdapter;
    let cmd = adapter.build_command(&sample_request());
    assert_eq!(cmd.program, "claude");
}

#[test]
fn claude_build_command_has_print_flag() {
    let adapter = ClaudeAdapter;
    let cmd = adapter.build_command(&sample_request());
    assert!(cmd.args.contains(&"-p".to_string()));
    assert!(cmd.args.contains(&"Fix the bug in main.rs".to_string()));
}

#[test]
fn claude_build_command_has_stream_json() {
    let adapter = ClaudeAdapter;
    let cmd = adapter.build_command(&sample_request());
    let idx = cmd
        .args
        .iter()
        .position(|a| a == "--output-format")
        .unwrap();
    assert_eq!(cmd.args[idx + 1], "stream-json");
}

#[test]
fn claude_build_command_has_bypass_permissions() {
    let adapter = ClaudeAdapter;
    let cmd = adapter.build_command(&sample_request());
    let idx = cmd
        .args
        .iter()
        .position(|a| a == "--permission-mode")
        .unwrap();
    assert_eq!(cmd.args[idx + 1], "bypassPermissions");
}

#[test]
fn claude_build_command_sets_cwd() {
    let adapter = ClaudeAdapter;
    let cmd = adapter.build_command(&sample_request());
    assert_eq!(cmd.cwd, PathBuf::from("/tmp/hydra/worktree-abc"));
}

// ---------------------------------------------------------------------------
// Claude adapter: parse_line
// ---------------------------------------------------------------------------

#[test]
fn claude_parse_line_message() {
    let adapter = ClaudeAdapter;
    let line = r#"{"type":"message","id":"msg_01","content":[{"type":"text","text":"hello"}]}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Message);
    assert!(event.raw_line.as_ref().unwrap().contains("msg_01"));
}

#[test]
fn claude_parse_line_assistant() {
    let adapter = ClaudeAdapter;
    let line = r#"{"type":"assistant","content":[]}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Message);
}

#[test]
fn claude_parse_line_tool_use() {
    let adapter = ClaudeAdapter;
    let line = r#"{"type":"tool_use","id":"tool_01","name":"bash","input":{}}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::ToolCall);
}

#[test]
fn claude_parse_line_tool_result() {
    let adapter = ClaudeAdapter;
    let line = r#"{"type":"tool_result","tool_use_id":"tool_01","content":"ok"}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::ToolResult);
}

#[test]
fn claude_parse_line_result_completed() {
    let adapter = ClaudeAdapter;
    let line = r#"{"type":"result","result":"done","is_error":false}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Completed);
}

#[test]
fn claude_parse_line_content_block_stop_completed() {
    let adapter = ClaudeAdapter;
    let line = r#"{"type":"content_block_stop","index":0}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Completed);
}

#[test]
fn claude_parse_line_error() {
    let adapter = ClaudeAdapter;
    let line = r#"{"type":"error","error":{"message":"bad request"}}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Failed);
}

#[test]
fn claude_parse_line_usage_with_usage_field() {
    let adapter = ClaudeAdapter;
    let line = r#"{"type":"usage","usage":{"input_tokens":100,"output_tokens":50}}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Usage);
}

#[test]
fn claude_parse_line_message_delta_with_usage() {
    let adapter = ClaudeAdapter;
    let line = r#"{"type":"message_delta","usage":{"output_tokens":50}}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Usage);
}

#[test]
fn claude_parse_line_message_delta_without_usage() {
    let adapter = ClaudeAdapter;
    let line = r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Progress);
}

#[test]
fn claude_parse_line_unknown_type() {
    let adapter = ClaudeAdapter;
    let line = r#"{"type":"future_event","data":"stuff"}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Unknown);
}

#[test]
fn claude_parse_line_empty_returns_none() {
    let adapter = ClaudeAdapter;
    assert!(adapter.parse_line("").is_none());
    assert!(adapter.parse_line("   ").is_none());
}

#[test]
fn claude_parse_line_non_json_returns_none() {
    let adapter = ClaudeAdapter;
    assert!(adapter.parse_line("not json at all").is_none());
}

// ---------------------------------------------------------------------------
// Claude adapter: parse_raw
// ---------------------------------------------------------------------------

#[test]
fn claude_parse_raw_skips_empty_lines() {
    let adapter = ClaudeAdapter;
    let chunk = b"\n\n{\"type\":\"message\",\"content\":\"hi\"}\n\n";
    let events = adapter.parse_raw(chunk);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, AgentEventType::Message);
}

#[test]
fn claude_parse_raw_non_json_fallback_to_message() {
    let adapter = ClaudeAdapter;
    let chunk = b"some plain text output\n";
    let events = adapter.parse_raw(chunk);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, AgentEventType::Message);
    assert!(events[0].data.get("text").is_some());
}

#[test]
fn claude_parse_raw_fixture_file() {
    let fixture = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/adapters/claude/stream-json.ok.jsonl"
    ))
    .unwrap();

    let adapter = ClaudeAdapter;
    let events = adapter.parse_raw(&fixture);

    // The fixture has 10 non-empty lines.
    assert_eq!(events.len(), 10);

    // Spot-check event types in order.
    assert_eq!(events[0].event_type, AgentEventType::Message); // message
    assert_eq!(events[1].event_type, AgentEventType::ToolCall); // tool_use
    assert_eq!(events[2].event_type, AgentEventType::ToolResult); // tool_result
    assert_eq!(events[3].event_type, AgentEventType::Usage); // message_delta with usage
    assert_eq!(events[4].event_type, AgentEventType::Message); // assistant
    assert_eq!(events[5].event_type, AgentEventType::Completed); // content_block_stop
    assert_eq!(events[6].event_type, AgentEventType::Completed); // result
    assert_eq!(events[7].event_type, AgentEventType::Usage); // usage
    assert_eq!(events[8].event_type, AgentEventType::Failed); // error
    assert_eq!(events[9].event_type, AgentEventType::Unknown); // unknown_future_type
}

// ---------------------------------------------------------------------------
// Codex adapter: build_command
// ---------------------------------------------------------------------------

#[test]
fn codex_build_command_has_correct_program() {
    let adapter = CodexAdapter;
    let cmd = adapter.build_command(&sample_request());
    assert_eq!(cmd.program, "codex");
}

#[test]
fn codex_build_command_has_exec_subcommand() {
    let adapter = CodexAdapter;
    let cmd = adapter.build_command(&sample_request());
    assert_eq!(cmd.args[0], "exec");
}

#[test]
fn codex_build_command_has_task_prompt() {
    let adapter = CodexAdapter;
    let cmd = adapter.build_command(&sample_request());
    assert_eq!(cmd.args[1], "Fix the bug in main.rs");
}

#[test]
fn codex_build_command_has_json_and_full_auto() {
    let adapter = CodexAdapter;
    let cmd = adapter.build_command(&sample_request());
    assert!(cmd.args.contains(&"--json".to_string()));
    assert!(cmd.args.contains(&"--full-auto".to_string()));
}

#[test]
fn codex_build_command_sets_cwd() {
    let adapter = CodexAdapter;
    let cmd = adapter.build_command(&sample_request());
    assert_eq!(cmd.cwd, PathBuf::from("/tmp/hydra/worktree-abc"));
}

// ---------------------------------------------------------------------------
// Codex adapter: parse_line
// ---------------------------------------------------------------------------

#[test]
fn codex_parse_line_message() {
    let adapter = CodexAdapter;
    let line = r#"{"type":"message","content":"hello"}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Message);
}

#[test]
fn codex_parse_line_function_call() {
    let adapter = CodexAdapter;
    let line = r#"{"type":"function_call","name":"write_file","arguments":{}}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::ToolCall);
}

#[test]
fn codex_parse_line_tool_call() {
    let adapter = CodexAdapter;
    let line = r#"{"type":"tool_call","name":"run","arguments":{}}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::ToolCall);
}

#[test]
fn codex_parse_line_function_call_output() {
    let adapter = CodexAdapter;
    let line = r#"{"type":"function_call_output","output":"ok"}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::ToolResult);
}

#[test]
fn codex_parse_line_tool_result() {
    let adapter = CodexAdapter;
    let line = r#"{"type":"tool_result","output":"ok"}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::ToolResult);
}

#[test]
fn codex_parse_line_completed() {
    let adapter = CodexAdapter;
    let line = r#"{"type":"completed","summary":"done"}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Completed);
}

#[test]
fn codex_parse_line_done() {
    let adapter = CodexAdapter;
    let line = r#"{"type":"done","result":"ok"}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Completed);
}

#[test]
fn codex_parse_line_error() {
    let adapter = CodexAdapter;
    let line = r#"{"type":"error","message":"something went wrong"}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Failed);
}

#[test]
fn codex_parse_line_usage_detected() {
    let adapter = CodexAdapter;
    let line = r#"{"usage":{"prompt_tokens":500,"completion_tokens":200}}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Usage);
}

#[test]
fn codex_parse_line_unknown_type() {
    let adapter = CodexAdapter;
    let line = r#"{"type":"new_codex_event","data":"abc"}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Unknown);
}

#[test]
fn codex_parse_line_empty_returns_none() {
    let adapter = CodexAdapter;
    assert!(adapter.parse_line("").is_none());
    assert!(adapter.parse_line("  \t  ").is_none());
}

#[test]
fn codex_parse_line_non_json_returns_none() {
    let adapter = CodexAdapter;
    assert!(adapter.parse_line("this is not json").is_none());
}

// ---------------------------------------------------------------------------
// Codex adapter: parse_raw
// ---------------------------------------------------------------------------

#[test]
fn codex_parse_raw_skips_empty_lines() {
    let adapter = CodexAdapter;
    let chunk = b"\n{\"type\":\"message\",\"content\":\"hi\"}\n\n";
    let events = adapter.parse_raw(chunk);
    assert_eq!(events.len(), 1);
}

#[test]
fn codex_parse_raw_non_json_skipped() {
    let adapter = CodexAdapter;
    let chunk = b"plain text\n";
    let events = adapter.parse_raw(chunk);
    // Codex parse_raw uses filter_map, so non-JSON lines are dropped.
    assert!(events.is_empty());
}

#[test]
fn codex_parse_raw_fixture_file() {
    let fixture = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/adapters/codex/exec-json.ok.jsonl"
    ))
    .unwrap();

    let adapter = CodexAdapter;
    let events = adapter.parse_raw(&fixture);

    // The fixture has 10 non-empty lines.
    assert_eq!(events.len(), 10);

    assert_eq!(events[0].event_type, AgentEventType::Message); // message
    assert_eq!(events[1].event_type, AgentEventType::ToolCall); // function_call
    assert_eq!(events[2].event_type, AgentEventType::ToolResult); // function_call_output
    assert_eq!(events[3].event_type, AgentEventType::ToolCall); // tool_call
    assert_eq!(events[4].event_type, AgentEventType::ToolResult); // tool_result
    assert_eq!(events[5].event_type, AgentEventType::Completed); // completed
    assert_eq!(events[6].event_type, AgentEventType::Completed); // done
    assert_eq!(events[7].event_type, AgentEventType::Failed); // error
    assert_eq!(events[8].event_type, AgentEventType::Usage); // usage (no type field)
    assert_eq!(events[9].event_type, AgentEventType::Unknown); // unknown_codex_type
}

// ---------------------------------------------------------------------------
// Cursor adapter: build_command (stub)
// ---------------------------------------------------------------------------

#[test]
fn cursor_build_command_has_correct_program() {
    let adapter = CursorAdapter;
    let cmd = adapter.build_command(&sample_request());
    assert_eq!(cmd.program, "cursor-agent");
}

#[test]
fn cursor_build_command_includes_json_flag_when_requested() {
    let adapter = CursorAdapter;
    let req = sample_request(); // output_json_stream = true
    let cmd = adapter.build_command(&req);
    assert!(cmd.args.contains(&"--json".to_string()));
}

#[test]
fn cursor_build_command_no_json_flag_when_not_requested() {
    let adapter = CursorAdapter;
    let mut req = sample_request();
    req.output_json_stream = false;
    let cmd = adapter.build_command(&req);
    assert!(!cmd.args.contains(&"--json".to_string()));
}

#[test]
fn cursor_parse_line_json() {
    let adapter = CursorAdapter;
    let line = r#"{"type":"message","text":"hello"}"#;
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Message);
}

#[test]
fn cursor_parse_line_plain_text_fallback() {
    let adapter = CursorAdapter;
    let line = "some plain output from cursor";
    let event = adapter.parse_line(line).unwrap();
    assert_eq!(event.event_type, AgentEventType::Message);
    assert!(event.data.get("text").is_some());
}

#[test]
fn cursor_parse_line_empty_returns_none() {
    let adapter = CursorAdapter;
    assert!(adapter.parse_line("").is_none());
}

// ---------------------------------------------------------------------------
// Cross-adapter: AgentEvent serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn agent_event_serde_round_trip() {
    use hydra_core::adapter::AgentEvent;

    let event = AgentEvent {
        event_type: AgentEventType::ToolCall,
        data: serde_json::json!({"name": "bash", "input": {"command": "ls"}}),
        raw_line: Some("original line".to_string()),
    };

    let json = serde_json::to_string(&event).unwrap();
    let deser: AgentEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(deser.event_type, AgentEventType::ToolCall);
    assert_eq!(deser.raw_line.unwrap(), "original line");
}
