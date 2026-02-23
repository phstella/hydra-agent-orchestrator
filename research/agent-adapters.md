# Agent Adapters — Headless API Reference

Each CLI agent Hydra supports at launch exposes a non-interactive mode suitable for programmatic orchestration. This document captures the exact invocation flags, output formats, and known limitations for each, providing the specification that each Rust adapter must implement.

---

## Common Adapter Interface

Every adapter translates between Hydra's internal `AgentAdapter` trait and the CLI agent's specific flags:

```
AgentAdapter.spawn(task, worktree, config)
    → builds a command string
    → spawns it as a child process in the worktree directory
    → returns a handle for streaming and interruption
```

Key responsibilities:
1. Set the working directory to the agent's worktree path
2. Pass the task prompt via the appropriate flag
3. Enable streaming JSON output where available
4. Auto-approve tool usage to avoid interactive prompts
5. Parse agent-specific output into normalized `AgentEvent` values

---

## Claude Code

### Binary

```
claude
```

### Headless Invocation

```bash
claude -p "your task prompt" \
  --output-format stream-json \
  --allowedTools "Edit,Write,Bash" \
  --verbose
```

### Flags Reference

| Flag | Purpose | Required |
|---|---|---|
| `-p <prompt>` | Print mode — disables interactive UI, returns result to stdout | Yes |
| `--output-format stream-json` | Newline-delimited JSON events streamed in real time | Yes |
| `--output-format json` | Single JSON blob after completion (alternative) | No |
| `--output-format text` | Plain text output (default) | No |
| `--allowedTools <tools>` | Pre-approve specific tools to avoid permission prompts | Recommended |
| `--verbose` | Include additional metadata in output | Optional |
| `--max-turns <n>` | Limit the number of agentic turns | Optional |

### Streaming JSON Event Format

Each line is a self-contained JSON object. Key event types:

```jsonl
{"type":"system","message":"Starting session...","timestamp":"..."}
{"type":"assistant","message":"I'll implement the auth module...","timestamp":"..."}
{"type":"tool_use","tool":"Edit","input":{"file":"src/auth.rs","content":"..."},"timestamp":"..."}
{"type":"tool_result","tool":"Edit","output":"File edited successfully","timestamp":"..."}
{"type":"result","result":"Task completed. Created auth.rs with JWT validation.","session_id":"...","cost":{"input_tokens":1200,"output_tokens":800}}
```

### Exit Codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General error or API failure |
| Non-zero | Tool execution failure |

### Adapter Implementation Notes

- The `stream-json` format is preferred for real-time UI updates. Parse each line as JSON and map `type` to `AgentEvent` variants.
- Use `--allowedTools` to prevent the agent from blocking on permission prompts. A sensible default set: `Edit,Write,Bash,Read`.
- Claude Code respects the working directory — spawn the child process with `cwd` set to the worktree path.
- Cost data (`input_tokens`, `output_tokens`) is included in the final `result` event and should be captured for cost tracking.
- Session continuation is possible via `--session-id` but is not needed for race mode (each race is a fresh session).

### Known Issues

- None critical for headless mode as of Feb 2026. Claude Code's headless mode is the most mature of the three agents.

---

## OpenAI Codex CLI

### Binary

```
codex
```

### Headless Invocation

```bash
codex exec "your task prompt" \
  --json \
  --full-auto \
  --sandbox danger-full-access
```

### Flags Reference

| Flag | Purpose | Required |
|---|---|---|
| `exec <prompt>` | Non-interactive execution mode | Yes |
| `--json` | JSONL event stream output | Yes |
| `--full-auto` | Allow file edits without confirmation | Recommended |
| `--sandbox danger-full-access` | Unrestricted filesystem/network access | Situational |
| `--sandbox read-only` | Read-only sandbox (default) | No |
| `--ephemeral` | Don't persist session files | Optional |
| `-o <file>` / `--output-last-message` | Write final message to a file | Optional |
| `--output-schema <schema>` | Structured JSON output conforming to a schema | Optional |

### JSONL Event Stream Format

Progress streams to stderr, structured events to stdout:

```jsonl
{"type":"thread.started","thread_id":"th_abc123"}
{"type":"turn.started","turn_id":"tu_def456"}
{"type":"item.created","item":{"type":"message","role":"assistant","content":"I'll create the auth module..."}}
{"type":"item.created","item":{"type":"tool_call","name":"write_file","arguments":{"path":"src/auth.rs","content":"..."}}}
{"type":"item.created","item":{"type":"tool_result","output":"File written"}}
{"type":"turn.completed","turn_id":"tu_def456","usage":{"input_tokens":900,"output_tokens":600}}
{"type":"thread.completed","thread_id":"th_abc123"}
```

### Exit Codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General error or API failure |
| 42 | Input error |
| 53 | Turn limit exceeded |

### Adapter Implementation Notes

- `codex exec` is the correct subcommand — not `codex run` or bare `codex`.
- `--full-auto` is essential to avoid the agent blocking on edit confirmations.
- `--sandbox danger-full-access` may be needed for tasks that require network access or writing outside the worktree. For most code generation tasks, `--full-auto` alone suffices.
- The `--json` flag produces JSONL on stdout. Parse line-by-line and map `type` fields to `AgentEvent` variants.
- Token usage is reported in `turn.completed` events.
- Working directory is respected — set `cwd` to the worktree path.
- `--ephemeral` is recommended for race mode to avoid polluting the filesystem with session artifacts.

### Known Issues

- The `codex exec fork` subcommand (for session forking) is still under development as of Feb 2026 — not yet usable for collaboration workflows.

---

## Cursor CLI

### Binary

```
cursor
```

### Headless Invocation

```bash
cursor --print "your task prompt" \
  --force \
  --output-format stream-json
```

### Flags Reference

| Flag | Purpose | Required |
|---|---|---|
| `--print` / `-p` | Print mode — non-interactive, output to stdout | Yes |
| `--force` / `--yolo` | Allow actual file modifications (without this, changes are only proposed) | Yes |
| `--output-format stream-json` | Streaming JSONL output | Recommended |
| `--output-format json` | Single JSON result | Alternative |
| `--output-format text` | Plain text (default) | No |

### Environment

| Variable | Purpose |
|---|---|
| `CURSOR_API_KEY` | Required for authentication in headless mode |

### Output Format

Cursor CLI's streaming JSON follows a similar structure to Claude Code (both are derived from similar patterns):

```jsonl
{"type":"system","message":"Starting..."}
{"type":"assistant","message":"Implementing the feature..."}
{"type":"tool_use","tool":"edit","input":{"file":"src/main.rs","content":"..."}}
{"type":"result","result":"Done.","session_id":"..."}
```

### Exit Codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Error |

### Adapter Implementation Notes

- **`--force` is mandatory** for race mode. Without it, Cursor only proposes changes without writing them to disk.
- `CURSOR_API_KEY` must be set in the environment before spawning. The adapter should validate this on initialization and return a clear error if missing.
- Working directory is respected — set `cwd` to the worktree path.
- Cursor CLI's `--print` mode has known stability issues: the process can hang indefinitely after completing its response. **The adapter must implement a watchdog timer** (configurable via `hydra.toml`, default 300s) that kills the process if no output is received for a sustained period.

### Known Issues (Critical)

1. **Hanging after completion**: The CLI sometimes hangs after responding, even in `--print` mode. Mitigation: implement an inactivity timeout that detects when no new output has been received for N seconds after the `result` event, then sends SIGTERM.
2. **MCP server approval**: MCP servers require initial interactive approval and do not work in headless mode without prior setup. Mitigation: document that users must run Cursor interactively once to approve MCP servers before using Hydra.
3. **stdin limitations**: No practical way to read prompts from files or stdin directly. The prompt must be passed as a CLI argument.
4. **`ls` subcommand ignores headless params**: The `ls` command enters interactive mode regardless of `--print`. Not relevant for race mode, but blocks any session-listing functionality.

---

## Adapter Mapping: CLI Output → AgentEvent

Each adapter maps the agent's output into normalized `AgentEvent` values:

| Agent Output Type | AgentEvent Variant | Notes |
|---|---|---|
| System/init messages | `AgentEvent::Info` | Startup messages, non-actionable |
| Assistant text | `AgentEvent::Message` | Model's natural language response |
| Tool invocation | `AgentEvent::ToolUse { tool, input }` | File edit, command execution, etc. |
| Tool result | `AgentEvent::ToolResult { tool, output }` | Success/failure of tool call |
| Final result | `AgentEvent::Completed { summary, tokens }` | Task done — triggers scoring |
| Error | `AgentEvent::Failed { error }` | Agent crashed or API error |

```rust
pub enum AgentEvent {
    Info { message: String },
    Message { content: String },
    ToolUse { tool: String, input: serde_json::Value },
    ToolResult { tool: String, output: String, success: bool },
    Completed { summary: String, tokens: TokenUsage },
    Failed { error: String },
}

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}
```

---

## Adding New Agents

To add support for a new CLI agent (e.g., Aider, Gemini CLI):

1. Create a new file in `crates/hydra-core/src/adapter/` (e.g., `aider.rs`)
2. Implement the `AgentAdapter` trait
3. Map the agent's output format to `AgentEvent` in the `stream_events` implementation
4. Register the adapter in `adapter/mod.rs`
5. Add default configuration in `config.rs`
6. Add the agent name to the CLI's `--agents` flag options

The adapter pattern ensures that adding a new agent never requires changes to the scoring engine, event bus, worktree manager, or UI.
