# System Architecture

## Overview

Hydra follows a **layered architecture** with a shared Rust core (`hydra-core`) consumed by two front-ends: a CLI binary (`hydra-cli`) and a Tauri v2 desktop application (`hydra-app`). All orchestration logic — worktree management, agent spawning, event routing, scoring, and merging — lives in `hydra-core` so that both interfaces share identical behavior.

---

## Crate Structure

```
hydra/
├── Cargo.toml                 # Workspace root
├── hydra.toml                 # Example project configuration
├── crates/
│   ├── hydra-core/            # Library crate — the engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs          # hydra.toml parsing and defaults
│   │       ├── worktree.rs        # Git worktree lifecycle
│   │       ├── adapter/
│   │       │   ├── mod.rs         # AgentAdapter trait
│   │       │   ├── claude.rs      # Claude Code adapter
│   │       │   ├── codex.rs       # Codex CLI adapter
│   │       │   └── cursor.rs      # Cursor CLI adapter
│   │       ├── runner.rs          # Agent process spawning and supervision
│   │       ├── events.rs          # Event types and EventBus
│   │       ├── scoring.rs         # Scoring engine
│   │       ├── merge.rs           # Merge engine
│   │       └── workflow.rs        # Collaboration workflow definitions
│   │
│   ├── hydra-cli/             # Binary crate — terminal interface
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/
│   │       │   ├── race.rs        # `hydra race` subcommand
│   │       │   ├── collab.rs      # `hydra collab` subcommand
│   │       │   ├── merge.rs       # `hydra merge` subcommand
│   │       │   └── status.rs      # `hydra status` subcommand
│   │       └── output.rs         # Terminal formatting and progress bars
│   │
│   └── hydra-app/             # Tauri v2 binary crate — desktop GUI
│       ├── Cargo.toml
│       ├── tauri.conf.json
│       ├── build.rs
│       └── src/
│           ├── main.rs
│           └── commands.rs    # Tauri command handlers (IPC bridge)
│
├── frontend/                  # React + TypeScript + TailwindCSS
│   ├── package.json
│   ├── tsconfig.json
│   ├── src/
│   │   ├── App.tsx
│   │   ├── components/
│   │   │   ├── Terminal.tsx        # xterm.js wrapper
│   │   │   ├── AgentGrid.tsx      # Multi-terminal grid layout
│   │   │   ├── DiffViewer.tsx     # Side-by-side diff display
│   │   │   ├── ScoreBoard.tsx     # Scoring results dashboard
│   │   │   ├── MergePanel.tsx     # One-click merge controls
│   │   │   └── TaskInput.tsx      # Prompt input + agent selection
│   │   ├── hooks/
│   │   │   ├── useAgent.ts        # Agent state management
│   │   │   └── useEvents.ts       # Tauri event listener bindings
│   │   └── lib/
│   │       ├── tauri.ts           # Tauri invoke wrappers
│   │       └── types.ts           # Shared TypeScript types
│   └── index.html
│
└── research/                  # Documentation (this folder)
```

---

## Core Components

### WorktreeManager

Manages the lifecycle of git worktrees for agent isolation.

```rust
pub struct WorktreeManager {
    repo_root: PathBuf,
    workspace_dir: PathBuf,  // .hydra-workspaces/
}

impl WorktreeManager {
    pub fn create(&self, agent_id: &str, base_branch: &str) -> Result<Worktree>;
    pub fn list(&self) -> Result<Vec<Worktree>>;
    pub fn remove(&self, worktree: &Worktree) -> Result<()>;
    pub fn cleanup_all(&self) -> Result<()>;
}

pub struct Worktree {
    pub path: PathBuf,
    pub branch: String,
    pub agent_id: String,
    pub base_branch: String,
}
```

Internally calls `git worktree add` / `git worktree remove` via `std::process::Command`. Worktrees live in `.hydra-workspaces/<agent_id>-<timestamp>/` relative to the repo root.

### AgentAdapter Trait

The abstraction boundary between hydra-core and any CLI agent.

```rust
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    fn name(&self) -> &str;

    fn capabilities(&self) -> AgentCapabilities;

    async fn spawn(
        &self,
        task: &Task,
        worktree: &Worktree,
        config: &AgentConfig,
    ) -> Result<AgentProcess>;

    async fn stream_events(
        &self,
        process: &mut AgentProcess,
    ) -> Result<Pin<Box<dyn Stream<Item = AgentEvent> + Send>>>;

    async fn interrupt(&self, process: &mut AgentProcess) -> Result<()>;
}

pub struct AgentCapabilities {
    pub supports_streaming: bool,
    pub supports_json_output: bool,
    pub supports_tool_restrictions: bool,
    pub supports_auto_approve: bool,
}
```

### AgentProcess

Wraps a running child process with metadata.

```rust
pub struct AgentProcess {
    pub id: Uuid,
    pub agent_name: String,
    pub worktree: Worktree,
    pub child: Child,            // tokio::process::Child
    pub started_at: Instant,
    pub status: AgentStatus,
}

pub enum AgentStatus {
    Running,
    Completed { duration: Duration },
    Failed { error: String, duration: Duration },
    Interrupted,
}
```

### EventBus

Central pub/sub bus routing events from agents to all subscribers (GUI, CLI, scoring engine).

```rust
pub struct EventBus {
    sender: broadcast::Sender<HydraEvent>,
}

pub enum HydraEvent {
    AgentStarted { agent_id: Uuid, agent_name: String },
    AgentOutput { agent_id: Uuid, line: String, stream: OutputStream },
    AgentCompleted { agent_id: Uuid, duration: Duration },
    AgentFailed { agent_id: Uuid, error: String },
    ScoringStarted,
    ScoringResult { rankings: Vec<AgentScore> },
    MergeCompleted { winner: Uuid, branch: String },
}

pub enum OutputStream {
    Stdout,
    Stderr,
}
```

Uses `tokio::sync::broadcast` for fan-out to multiple consumers. The Tauri app subscribes via `tauri::Manager::emit` to push events to the React frontend.

### TaskRouter

Entry point for all user-initiated operations. Coordinates worktree creation, agent spawning, and post-completion scoring.

```rust
pub struct TaskRouter {
    worktree_mgr: WorktreeManager,
    adapters: HashMap<String, Box<dyn AgentAdapter>>,
    event_bus: EventBus,
    scoring: ScoringEngine,
    merge: MergeEngine,
}

impl TaskRouter {
    pub async fn race(&self, task: Task, agent_names: Vec<String>) -> Result<RaceResult>;
    pub async fn collaborate(&self, workflow: Workflow) -> Result<CollabResult>;
    pub async fn merge_winner(&self, race_id: Uuid) -> Result<()>;
    pub async fn status(&self) -> Result<Vec<AgentProcess>>;
}
```

### ScoringEngine

Evaluates agent outputs after completion. See `research/scoring-engine.md` for full algorithm.

```rust
pub struct ScoringEngine {
    config: ScoringConfig,
}

impl ScoringEngine {
    pub async fn evaluate(&self, worktrees: &[Worktree]) -> Result<Vec<AgentScore>>;
}

pub struct AgentScore {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub total: f64,
    pub breakdown: ScoreBreakdown,
}

pub struct ScoreBreakdown {
    pub build: Option<f64>,
    pub tests: Option<f64>,
    pub lint: Option<f64>,
    pub diff_size: Option<f64>,
    pub speed: Option<f64>,
}
```

### MergeEngine

Handles merging the winning agent's worktree branch back into the base branch.

```rust
pub struct MergeEngine;

impl MergeEngine {
    pub fn merge_branch(
        &self,
        repo_root: &Path,
        source_branch: &str,
        target_branch: &str,
    ) -> Result<MergeOutcome>;

    pub fn generate_diff(
        &self,
        worktree: &Worktree,
    ) -> Result<String>;
}

pub enum MergeOutcome {
    FastForward,
    Merged { conflicts: usize },
    Conflict { files: Vec<PathBuf> },
}
```

---

## Data Flow

### Race Mode

```
User submits task + agent list
         │
         ▼
    TaskRouter.race()
         │
         ├── WorktreeManager.create()  ×N agents
         │         │
         │         ▼
         │   .hydra-workspaces/claude-<ts>/
         │   .hydra-workspaces/codex-<ts>/
         │   .hydra-workspaces/cursor-<ts>/
         │
         ├── AgentAdapter.spawn()  ×N (concurrent via tokio::join!)
         │         │
         │         ▼
         │   claude -p "task" --output-format stream-json  (in worktree-claude)
         │   codex exec "task" --json --full-auto           (in worktree-codex)
         │   cursor --print "task"                          (in worktree-cursor)
         │
         ├── AgentAdapter.stream_events()  ×N
         │         │
         │         ▼
         │   EventBus broadcasts AgentOutput events
         │   (GUI renders in xterm.js, CLI prints to stdout)
         │
         ├── All agents complete (or timeout)
         │
         ├── ScoringEngine.evaluate()
         │         │
         │         ▼
         │   Run build_command in each worktree
         │   Run test_command in each worktree
         │   Run lint_command in each worktree
         │   Compute diff size for each worktree
         │   Calculate weighted scores
         │
         ├── EventBus broadcasts ScoringResult
         │
         └── User reviews scores → MergeEngine.merge_branch()
                   │
                   ▼
             WorktreeManager.cleanup_all()
```

### Collaboration Mode

```
User submits workflow definition
         │
         ▼
    TaskRouter.collaborate()
         │
         ├── Phase 1: Builder
         │   ├── WorktreeManager.create() for builder agent
         │   ├── AgentAdapter.spawn() with original task
         │   └── Wait for completion → capture diff
         │
         ├── Phase 2: Reviewer
         │   ├── WorktreeManager.create() for reviewer agent
         │   ├── AgentAdapter.spawn() with diff as context +
         │   │   review prompt
         │   └── Wait for completion → capture review feedback
         │
         ├── Phase 3: Refinement (optional loop)
         │   ├── Feed review feedback back to builder agent
         │   └── Repeat until scoring threshold met or max iterations
         │
         └── ScoringEngine.evaluate() on final output
```

---

## IPC: Tauri Bridge

The Tauri app exposes hydra-core functionality to the React frontend via Tauri commands:

```rust
// crates/hydra-app/src/commands.rs

#[tauri::command]
async fn start_race(
    state: State<'_, AppState>,
    task: String,
    agents: Vec<String>,
) -> Result<Uuid, String>;

#[tauri::command]
async fn get_scores(
    state: State<'_, AppState>,
    race_id: Uuid,
) -> Result<Vec<AgentScore>, String>;

#[tauri::command]
async fn merge_winner(
    state: State<'_, AppState>,
    race_id: Uuid,
) -> Result<(), String>;

#[tauri::command]
async fn get_diff(
    state: State<'_, AppState>,
    agent_id: Uuid,
) -> Result<String, String>;
```

Events flow from Rust to React via Tauri's event system:

```typescript
// frontend/src/hooks/useEvents.ts
import { listen } from "@tauri-apps/api/event";

listen<AgentOutputPayload>("agent-output", (event) => {
  // Append to xterm.js terminal for the corresponding agent
});

listen<ScoringResultPayload>("scoring-result", (event) => {
  // Update scoreboard component
});
```

---

## Configuration

Project-level configuration via `hydra.toml` at the repository root:

```toml
[general]
workspace_dir = ".hydra-workspaces"
max_concurrent_agents = 3
timeout_seconds = 600

[agents.claude]
binary = "claude"
extra_args = ["--allowedTools", "Edit,Write,Bash"]

[agents.codex]
binary = "codex"
extra_args = ["--full-auto"]

[agents.cursor]
binary = "cursor"
extra_args = ["--force"]
timeout_seconds = 300  # Override: Cursor CLI can hang

[scoring]
weights = { build = 30, tests = 30, lint = 15, diff_size = 15, speed = 10 }
build_command = "npm run build"
test_command = "npm test"
lint_command = "npm run lint"

[collaboration]
max_iterations = 3
review_prompt_template = "Review this diff for bugs, security issues, and code quality:\n\n{diff}"
```

Parsed by `hydra-core/src/config.rs` using the `toml` crate with `serde` deserialization into strongly-typed structs.
