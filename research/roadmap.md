# Development Roadmap

## Overview

Hydra is built in four phases, each delivering a usable increment. Phase 1 produces a working CLI that can run a single agent in an isolated worktree. Phase 4 delivers the full vision: multi-agent collaboration, automated scoring, and a polished desktop GUI.

Estimated timeline: 10-14 weeks for a solo developer working full-time. Phases can overlap where noted.

---

## Phase 1 — Foundation (hydra-core + single-agent CLI)

**Goal:** Prove the core loop works — spawn one CLI agent in an isolated worktree, stream its output, and capture the result.

**Duration:** 2-3 weeks

### Milestones

| ID | Milestone | Deliverable | Dependencies |
|---|---|---|---|
| 1.1 | Rust workspace scaffold | `Cargo.toml` workspace with `hydra-core` and `hydra-cli` crates compiling | None |
| 1.2 | Configuration system | `hydra.toml` parsing with serde + toml crate, defaults for missing fields | 1.1 |
| 1.3 | Git worktree manager | `WorktreeManager` that creates/lists/removes worktrees in `.hydra-workspaces/` | 1.1 |
| 1.4 | Agent adapter trait | `AgentAdapter` trait definition with `AgentEvent` enum | 1.1 |
| 1.5 | Claude Code adapter | Working adapter that spawns `claude -p` in a worktree, parses `stream-json` output | 1.3, 1.4 |
| 1.6 | Agent runner | `Runner` that ties worktree creation → agent spawn → event streaming → cleanup | 1.3, 1.5 |
| 1.7 | CLI: `hydra race` (single agent) | `hydra race --task "..." --agents claude` — runs one agent, prints streaming output, cleans up | 1.2, 1.6 |

### Exit Criteria

- `hydra race --task "add a hello world endpoint" --agents claude` runs in any git repo
- Creates a worktree, runs Claude Code, streams output to the terminal, and cleans up on completion
- Non-zero exit code if the agent fails or the worktree can't be created

### Key Technical Work

- Tokio async runtime setup
- `std::process::Command` wrapper for git operations (worktree add/remove, branch create)
- JSONL line-by-line parser for Claude Code's `stream-json` output
- Signal handling (Ctrl+C cleans up worktrees before exiting)
- `clap` CLI framework for argument parsing

---

## Phase 2 — Multi-Agent Racing

**Goal:** Run multiple agents in parallel, each in their own worktree, and score the results.

**Duration:** 3-4 weeks

### Milestones

| ID | Milestone | Deliverable | Dependencies |
|---|---|---|---|
| 2.1 | Codex CLI adapter | Working adapter for `codex exec` with JSONL parsing | 1.4 |
| 2.2 | Cursor CLI adapter | Working adapter for `cursor --print` with watchdog timeout | 1.4 |
| 2.3 | Parallel agent spawning | `AgentPool` that spawns N agents concurrently via `tokio::join!` | 1.6, 2.1, 2.2 |
| 2.4 | EventBus | `broadcast::Sender` fan-out of `HydraEvent` to multiple subscribers | 1.6 |
| 2.5 | Baseline capture | Run build/test/lint commands in original repo before agents start | 1.2 |
| 2.6 | Scoring engine: build + tests | `score_build` and `score_tests` with test-runner output parsers | 2.5 |
| 2.7 | Scoring engine: lint + diff + speed | `score_lint`, `score_diff_size`, `score_speed` | 2.5 |
| 2.8 | Composite scoring + ranking | Weighted score calculation, ranking, CLI output table | 2.6, 2.7 |
| 2.9 | CLI: `hydra merge` | `hydra merge <branch>` merges the winner's branch and cleans up worktrees | 1.3 |
| 2.10 | CLI: `hydra race` (multi-agent) | Full race flow: spawn → stream → score → rank → suggest merge | 2.3, 2.8 |

### Exit Criteria

- `hydra race --task "implement JWT auth" --agents claude,codex,cursor` runs all three agents in parallel
- Each agent works in an isolated worktree — no file collisions
- After all agents complete, the scoring engine runs and prints a ranked table with scores
- `hydra merge <branch>` merges the winning branch into the current branch

### Key Technical Work

- Per-agent JSONL parsers (Claude, Codex, and Cursor each have different event schemas)
- Test runner output parsers (jest, cargo test, pytest, go test)
- Lint output parsers (eslint, clippy, pylint — at least 2 to start)
- Concurrent process management with tokio (handling one agent failing while others continue)
- Graceful timeout: configurable per-agent timeout that kills hung processes

---

## Phase 3 — Tauri GUI

**Goal:** Build the desktop application with real-time terminal output, diff viewer, and merge controls.

**Duration:** 3-4 weeks (can overlap with Phase 2 starting at milestone 2.4)

### Milestones

| ID | Milestone | Deliverable | Dependencies |
|---|---|---|---|
| 3.1 | Tauri v2 app scaffold | `hydra-app` crate with React + TailwindCSS frontend building and launching | 1.1 |
| 3.2 | Tauri IPC bridge | Rust command handlers that call `hydra-core` functions, TypeScript invoke wrappers | 2.10 |
| 3.3 | Task input component | `TaskInput.tsx` — prompt field, agent multi-select, "Start Race" button | 3.1 |
| 3.4 | xterm.js terminal integration | `Terminal.tsx` wrapping xterm.js, connected to `tauri-plugin-pty` for real agent output | 3.1 |
| 3.5 | Agent grid layout | `AgentGrid.tsx` — responsive grid of terminal panels, one per agent, with status badges | 3.4 |
| 3.6 | Event streaming to frontend | Tauri events pushed from Rust → React, displayed in terminals and status indicators | 2.4, 3.4 |
| 3.7 | Diff viewer | `DiffViewer.tsx` — side-by-side diff display comparing agent output vs base branch | 3.2 |
| 3.8 | Scoreboard | `ScoreBoard.tsx` — ranked cards with score breakdowns, bar charts, color coding | 2.8, 3.2 |
| 3.9 | Merge panel | `MergePanel.tsx` — one-click merge button, conflict warnings, cleanup confirmation | 2.9, 3.2 |
| 3.10 | Polish: loading states, error handling, responsive layout | Production-quality UX | 3.3-3.9 |

### Exit Criteria

- Desktop app launches on Linux and Windows
- User can type a task, select agents, and start a race from the GUI
- Real-time terminal output streams for each agent in a grid layout
- After completion, diff viewer shows each agent's changes
- Scoreboard displays ranked results with breakdowns
- One-click merge applies the winner's changes

### Key Technical Work

- `tauri-plugin-pty` integration for real pseudo-terminal rendering in xterm.js
- Tauri event system: Rust `app.emit("agent-output", payload)` → React `listen("agent-output")`
- xterm.js addon: `@xterm/addon-fit` for auto-resizing terminals to their container
- Diff rendering: either Monaco Editor's diff view or a dedicated library like `react-diff-viewer`
- TailwindCSS responsive grid that handles 1-4 terminals gracefully
- State management: zustand or React context for agent states, scores, and race lifecycle

---

## Phase 4 — Collaboration & Polish

**Goal:** Add collaboration workflows, cost tracking, session history, and production polish.

**Duration:** 2-3 weeks

### Milestones

| ID | Milestone | Deliverable | Dependencies |
|---|---|---|---|
| 4.1 | Builder/Reviewer workflow | `hydra collab builder-reviewer` command + GUI workflow wizard | 2.10 |
| 4.2 | Specialization workflow | `hydra collab specialize` command with directory scoping | 2.10 |
| 4.3 | Iterative refinement | `hydra collab refine` command with score-threshold loop | 2.8 |
| 4.4 | Workflow composition | Multi-step workflow definitions in `hydra.toml` | 4.1-4.3 |
| 4.5 | Cost/token tracking | Per-agent, per-run, cumulative token/cost display in CLI and GUI | 2.4 |
| 4.6 | Session history | SQLite database storing past races: task, agents, scores, timestamps, costs | 2.10 |
| 4.7 | Session replay | GUI view of past races with stored scores and diffs | 4.6, 3.7 |
| 4.8 | GUI: collaboration workflow UI | Visual workflow builder or preset selector in the GUI | 4.1-4.3, 3.2 |
| 4.9 | Cross-platform testing | Validated on Linux (X11 + Wayland), Windows 10/11 | 3.10 |
| 4.10 | Documentation + release | README, user guide, `hydra --help` polish, binary releases | All |

### Exit Criteria

- All three collaboration workflows functional in CLI and GUI
- Cost tracking shows per-agent token usage and estimated dollar cost
- Session history persists across runs — users can review past races
- App works on Linux (X11 and Wayland) and Windows
- Documentation covers installation, configuration, and all workflows

---

## Dependency Graph

```
Phase 1                    Phase 2                    Phase 3              Phase 4
========                   ========                   ========             ========

1.1 Scaffold ──────┬──── 2.1 Codex adapter            3.1 Tauri scaffold    4.1 Builder/Reviewer
                   │                                       │
1.2 Config ────────┤     2.2 Cursor adapter            3.3 TaskInput        4.2 Specialization
                   │                                       │
1.3 Worktrees ─────┤     2.3 AgentPool ──────────────  3.4 xterm.js         4.3 Iterative
                   │          │                            │
1.4 Trait ─────────┤     2.4 EventBus ─────────────── 3.5 AgentGrid        4.4 Composition
                   │          │                            │
1.5 Claude ────────┤     2.5 Baseline capture          3.6 Event stream     4.5 Cost tracking
                   │          │                            │
1.6 Runner ────────┘     2.6 Score: build+tests        3.7 DiffViewer       4.6 Session history
                              │                            │
1.7 CLI (single) ─────  2.7 Score: lint+diff+speed    3.8 ScoreBoard       4.7 Session replay
                              │                            │
                         2.8 Composite scoring         3.9 MergePanel       4.8 Collab GUI
                              │                            │
                         2.9 CLI: merge                3.10 Polish          4.9 Cross-platform
                              │                                              │
                         2.10 CLI: race (multi) ──────────────────────── 4.10 Docs + release
```

---

## Risk Mitigation by Phase

| Phase | Risk | Mitigation |
|---|---|---|
| 1 | Git worktree operations fail on unusual repo states (submodules, LFS) | Test against repos with submodules; document limitations |
| 1 | Claude Code CLI changes its output format | Pin to known version; adapter version check on startup |
| 2 | Cursor CLI hangs in headless mode | Watchdog timer with configurable timeout; graceful degradation |
| 2 | Test runner output parsing is fragile | Start with exit-code-only scoring; add parsers incrementally |
| 3 | `tauri-plugin-pty` doesn't support Windows | Fall back to raw stdout rendering (no PTY colors) on Windows |
| 3 | xterm.js performance with 4 concurrent terminals | Limit to 4 visible terminals; virtualise hidden ones |
| 4 | Collaboration prompts produce poor results | Ship sensible defaults; let users customize prompt templates |
| 4 | Session history database grows large | Auto-prune entries older than 30 days; configurable retention |
