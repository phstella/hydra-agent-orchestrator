# System Architecture

Last updated: 2026-02-23

## 1. Goals and Constraints

### Primary goals

1. Run multiple coding agents concurrently on one repository without file collisions.
2. Make execution deterministic enough to compare outputs fairly.
3. Provide both CLI and GUI control surfaces backed by the same orchestration core.
4. Preserve auditability: each run must be replayable from recorded metadata.

### Hard constraints

- Linux-first operation must be robust.
- Windows must be supported without requiring WSL for the Hydra runtime itself.
- macOS support is a plus, not a launch gate.
- Agent CLIs are external dependencies with changing interfaces.

### Non-goals (v1)

- Executing agents on remote workers.
- Stateful agent conversations shared across unrelated runs.
- Automatic conflict resolution with no human approval.

## 2. Runtime Topology

Hydra should be a shared engine with two frontends:

- `hydra-core` (Rust library): orchestration, isolation, scoring, merge safety
- `hydra-cli` (Rust binary): automation-first, scripting-friendly
- `hydra-app` (Tauri v2 + React): interactive monitoring and comparison

```text
User (CLI or GUI)
    -> Task API (start_race/start_workflow)
        -> Run Orchestrator
            -> Worktree Service
            -> Adapter Manager
            -> Process Supervisor
            -> Event Bus
            -> Scoring Engine
            -> Merge Coordinator
```

## 3. Repository and Branch Isolation

### Isolation model

Hydra uses **git worktrees** as the default isolation primitive.

Per run:
1. Determine base ref (`HEAD` by default or user-selected branch).
2. Create run namespace: `hydra/<run_id>/...`.
3. Create one worktree per agent:
   - `.hydra/worktrees/<run_id>/<agent_key>/`
4. Execute each agent with `cwd` set to its worktree path.

### Branch naming convention

- Base snapshot: `hydra/<run_id>/base`
- Agent branches: `hydra/<run_id>/agent/<agent_key>`
- Integration branch (for composed workflows): `hydra/<run_id>/integration`

### Cleanup policy

- Default: keep worktrees for failed runs, cleanup successful run worktrees after merge.
- Configurable retention:
  - `retain = none | failed | all`
  - time-based garbage collection (e.g., 7 days)

## 4. Core Components

### 4.1 Orchestrator

Coordinates run lifecycle:
- validate config
- prepare worktrees
- spawn agents
- collect events and artifacts
- trigger scoring
- expose merge candidates

### 4.2 Adapter Manager

Manages installed agent adapters and capability detection.

Responsibilities:
- binary discovery (`PATH`, configured absolute path)
- version probing (`--version` or equivalent)
- capability probing (`--help` parse and/or static manifest)
- adapter-specific invocation building

### 4.3 Process Supervisor

Per-agent supervision with:
- start deadline
- idle timeout
- hard timeout
- cancellation support (`SIGTERM` then `SIGKILL` on Unix; terminate fallback on Windows)
- bounded output buffering (prevent unbounded memory)

### 4.4 Event Bus

Normalizes events from all agents and hydra subsystems.

Event categories:
- run events (`run_started`, `run_completed`, `run_failed`)
- agent lifecycle (`agent_started`, `agent_completed`, `agent_failed`)
- agent stream (`agent_stdout`, `agent_stderr`, parsed semantic events)
- scoring (`score_started`, `score_finished`)
- merge (`merge_ready`, `merge_succeeded`, `merge_conflict`)

### 4.5 Scoring Engine

Post-run evaluator with configurable dimensions (build/tests/lint/diff/speed).

Design rule:
- score dimensions must be deterministic from recorded artifacts whenever possible.

### 4.6 Merge Coordinator

Safety-first merge operations:
- pre-merge checks
- dry-run support
- conflict detection and reporting
- optional auto-merge with threshold and policy gates

## 5. Data Model (Core Records)

### Run record

```rust
struct RunRecord {
    run_id: Uuid,
    repo_root: PathBuf,
    base_ref: String,
    task_prompt_hash: String,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    status: RunStatus,
}
```

### Agent run record

```rust
struct AgentRunRecord {
    run_id: Uuid,
    agent_key: String,
    adapter_version: Option<String>,
    worktree_path: PathBuf,
    branch: String,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    status: AgentStatus,
    token_usage: Option<TokenUsage>,
    cost_estimate_usd: Option<f64>,
}
```

### Artifact record

```rust
struct ArtifactRecord {
    run_id: Uuid,
    agent_key: String,
    kind: ArtifactKind, // diff, logs, score_json, test_output, lint_output
    path: PathBuf,
    sha256: String,
}
```

## 6. Execution Flows

### 6.1 Race mode (parallel)

1. Validate all selected adapters are installed.
2. Capture baseline metrics (build/test/lint on base).
3. Create one worktree per agent.
4. Spawn all agent processes concurrently.
5. Stream events to CLI/GUI.
6. Wait for completion/timeouts.
7. Score each agent output.
8. Publish ranked results and merge options.

### 6.2 Collaboration workflow mode (staged)

1. Build workflow graph from preset or user-defined DSL.
2. Execute node-by-node with explicit artifact passing.
3. Record transition artifacts (e.g., reviewer feedback).
4. Score terminal node outputs.

## 7. Cross-Platform Architecture Notes

### Linux (first-class target)

- PTY path: `portable-pty` with Unix backend.
- Signal model: full `SIGTERM`/`SIGKILL` support.
- Locking: file-lock guard for `.hydra/state` to avoid concurrent mutating runs.

### Windows

- PTY path: ConPTY via `portable-pty`.
- Process termination semantics differ from Unix signals.
- Path handling must normalize separators and long paths.
- Terminal rendering differences can impact ANSI stream behavior.

### macOS (bonus)

- Similar to Linux process behavior.
- Not a launch blocker; parity tracked after Linux/Windows milestone stability.

## 8. Safety and Security Model

### Trust boundaries

Hydra orchestrates external tools that can edit files and run shell commands. Hydra itself must:
- isolate writes to agent worktrees unless user opts out
- avoid executing arbitrary shell snippets from untrusted output
- redact secrets from logs where feasible

### Permission strategy

- Default run policy: isolated worktree + explicit adapter arguments.
- Optional elevated mode must be opt-in per run.
- Adapter command lines are always persisted for audit.

## 9. Failure Modes and Recovery

| Failure | Detection | Recovery |
|---|---|---|
| Worktree creation fails | non-zero git exit | mark agent failed, continue others if possible |
| Agent hangs | idle/hard timeout | terminate process, mark timed out |
| Adapter parse drift | JSON parse errors spike | fall back to raw stream mode + warning |
| Scoring command fails | missing tool or non-zero | dimension marked unavailable, weights renormalized |
| Merge conflicts | git merge conflict exit | produce conflict report and keep branches |

## 10. Observability

Minimum telemetry for each run:
- run id, agent set, base ref, timing
- per-agent duration, exit code, timeout reason
- score dimension breakdown and normalized weights
- merge outcome and conflict file list

Suggested implementation:
- `tracing` for structured logs
- JSON logs persisted under `.hydra/runs/<run_id>/events.jsonl`
- optional SQLite index for history UI

## 11. Architecture Decisions (ADR-lite)

1. Use git worktrees, not multiple clones.
2. Keep orchestration logic in shared Rust core.
3. Prefer normalized event schema over adapter-specific UI handling.
4. Treat scoring as pluggable, with baseline deterministic dimensions first.
5. Keep merge operation explicit by default; auto-merge is policy-gated.

## 12. Open Architecture Questions

1. Should Hydra run as short-lived process per command, or optional background daemon for GUI and CLI sharing?
2. Should event storage be append-only JSONL only, or dual-write to SQLite in v1?
3. Should we support "remote repo" orchestration before local parity is complete?

## 13. Source Notes

Architecture here combines product design choices with external constraints from current agent CLIs and tooling ecosystems. CLI capability assumptions are detailed in `research/agent-adapters.md` with confidence markers and links.
