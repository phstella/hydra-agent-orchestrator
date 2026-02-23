# Agent Adapters

Last updated: 2026-02-23

## 1. Purpose

Hydra adapters translate vendor-specific CLI behavior into one normalized runtime contract. They must be resilient to flag drift, output-format changes, and partial failures.

This document separates:
- **Verified behavior**: backed by current docs/source
- **Inferred behavior**: plausible but not fully confirmed in docs

## 2. Adapter Contract

Each adapter implements:

```rust
trait AgentAdapter {
    fn key(&self) -> &'static str;            // e.g. "claude", "codex", "cursor-agent"
    fn detect(&self) -> DetectResult;         // binary + version + feature probes
    fn capabilities(&self) -> CapabilitySet;  // json_stream, force_edit, session_resume, etc.
    fn build_command(&self, req: SpawnRequest) -> BuiltCommand;
    fn parse_line(&self, line: &str) -> Option<AgentEvent>; // semantic events
    fn parse_raw(&self, chunk: &[u8]) -> Vec<AgentEvent>;   // fallback parser
}
```

Normalized events:
- `message`
- `tool_call`
- `tool_result`
- `progress`
- `completed`
- `failed`
- `usage`

## 3. Capability Model

```rust
struct CapabilitySet {
    json_stream: bool,
    plain_text: bool,
    force_edit_mode: bool,
    sandbox_controls: bool,
    approval_controls: bool,
    session_resume: bool,
    emits_usage: bool,
}
```

Confidence tags:
- `verified`: present in official docs/source now
- `observed`: seen in practical runs, not clearly documented
- `unknown`: cannot reliably verify yet

## 4. Claude Code Adapter (`claude`)

### 4.1 Verified CLI surface

Official docs include:
- print/headless mode via `-p` / `--print`
- output mode via `--output-format` with `text|json|stream-json`
- input mode via `--input-format` with `text|stream-json`
- tool controls via `--allowedTools`, `--disallowedTools`
- approval controls via `--permission-mode`

### 4.2 Recommended Hydra invocation (race mode)

```bash
claude -p "$TASK_PROMPT" \
  --output-format stream-json \
  --permission-mode bypassPermissions
```

Optional hardening:
- add `--allowedTools` to narrow tool surface
- add `--max-turns` for bounded runs

### 4.3 Parsing strategy

Primary parser:
- line-oriented JSON parse in `stream-json` mode

Fallback parser:
- raw text stream to `message` events if JSON parse fails repeatedly

### 4.4 Known integration risks

- Output event schemas can evolve; parser must ignore unknown fields.
- Permission defaults vary by user config; adapter must always pass explicit mode.

## 5. OpenAI Codex Adapter (`codex`)

### 5.1 Verified CLI surface

Official docs/source indicate:
- non-interactive mode: `codex exec PROMPT`
- JSON streaming mode: `--json`
- execution root override: `-C, --cd`
- sandbox controls: `--sandbox <mode>`
- approval controls: `--ask-for-approval <policy>`
- shortcuts:
  - `--full-auto` (auto approvals + sandboxed)
  - `--dangerously-bypass-approvals-and-sandbox`

### 5.2 Recommended Hydra invocation (race mode)

```bash
codex exec "$TASK_PROMPT" \
  --json \
  --full-auto
```

When task needs unrestricted host access (opt-in only):

```bash
codex exec "$TASK_PROMPT" --json --dangerously-bypass-approvals-and-sandbox
```

### 5.3 Parsing strategy

- Parse JSONL event stream from stdout in `--json` mode.
- Keep raw events for future schema migration.
- Extract usage when present and normalize into Hydra `usage` event.

### 5.4 Known integration risks

- Approval/sandbox flags have changed across releases.
- Adapter should perform startup probe: `codex exec --help` and map accepted flags dynamically when possible.

## 6. Cursor Adapter (`cursor-agent` / `cursor`)

### 6.1 Current status

Cursor documentation references **Cursor Agent CLI** and a headless `--print` mode. Public docs and ecosystem references suggest command names and options can vary between versions (`cursor-agent` vs `cursor`).

### 6.2 Operational recommendation

Use runtime binary discovery order:
1. configured path (`hydra.toml`)
2. `cursor-agent`
3. `cursor`

Probe each candidate with `--help` and enable adapter only on successful capability match.

### 6.3 Likely flags (partially verified)

Commonly referenced:
- `-p` / `--print` for headless prompt execution
- output-format option (`text|json|stream-json`)
- `-f` / `--force` to apply changes without approval

### 6.4 Integration guardrails

- Treat Cursor adapter as `observed` confidence until probe passes.
- Require explicit warning in UI when adapter is using inferred flags.
- Enforce idle timeout due known hang reports in headless operation.

## 7. Standard SpawnRequest and BuiltCommand

```rust
struct SpawnRequest {
    task_prompt: String,
    worktree_path: PathBuf,
    timeout_seconds: u64,
    allow_network: bool,
    force_edit: bool,
    output_json_stream: bool,
}

struct BuiltCommand {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    cwd: PathBuf,
}
```

## 8. Error Taxonomy

- `BinaryMissing`
- `AuthMissing`
- `UnsupportedVersion`
- `UnsupportedFlag`
- `SpawnFailed`
- `StreamParseError`
- `TimedOut`
- `Interrupted`

Each error should include:
- adapter key
- attempted command
- stderr excerpt
- recovery hint

## 9. Conformance Tests

Each adapter needs contract tests in CI:

1. `detect()` succeeds with mocked `--version` output.
2. `build_command()` emits expected flags for safe/default mode.
3. parser handles known event fixtures and unknown fields.
4. timeout and interruption behavior produce deterministic statuses.
5. command probe downgrades gracefully when a flag is unavailable.

Fixture layout:

```text
crates/hydra-core/tests/fixtures/adapters/
  claude/
    help.txt
    stream-json.ok.jsonl
  codex/
    help.txt
    exec-json.ok.jsonl
  cursor/
    help.txt
    stream-json.sample.jsonl
```

## 10. Adapter Confidence Matrix (as of 2026-02-23)

| Adapter | Headless mode | JSON stream | Force edit | Confidence |
|---|---|---|---|---|
| Claude Code | Verified | Verified (`stream-json`) | Verified (permission mode / tool flags) | High |
| OpenAI Codex | Verified (`exec`) | Verified (`--json`) | Verified (`--full-auto`, bypass flag) | High |
| Cursor Agent CLI | Partially verified | Partially verified | Partially verified | Medium |

## 11. Source Links

- Claude Code docs: https://docs.anthropic.com/en/docs/claude-code/overview
- Claude Code settings/permissions: https://docs.anthropic.com/en/docs/claude-code/settings
- Claude CLI reference: https://docs.anthropic.com/fr/docs/claude-code/cli-reference
- Codex CLI guide: https://developers.openai.com/codex/cli
- Codex source/CLI docs: https://github.com/openai/codex
- Cursor CLI headless docs: https://docs.cursor.com/en/cli/headless
- Cursor CLI parameter reference: https://docs.cursor.com/cli/reference/parameters
- Cursor output format docs: https://docs.cursor.com/en/cli/reference/output-format
