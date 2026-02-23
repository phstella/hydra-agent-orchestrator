# Technology Stack

Last updated: 2026-02-23

## 1. Decision Criteria

Every stack decision is scored against:
1. Linux reliability
2. Windows feasibility
3. Runtime overhead while agents are active
4. Implementation complexity for a small team
5. Ecosystem stability

## 2. Core Language and Runtime

### Choice

- Rust (edition 2021)
- Tokio async runtime

### Why

Hydra is fundamentally a systems-orchestration product:
- process spawning
- stream handling
- timeout and cancellation
- filesystem and git operations

Rust + Tokio provides strong control with low overhead and robust error modeling.

## 3. App Surfaces

### CLI

- `clap` for commands and typed args
- `tracing` for structured logs

### Desktop GUI

- Tauri v2 backend bridge to `hydra-core`
- React + TypeScript frontend for rapid operator UI

Rationale:
- one engine, two interfaces
- no duplicate orchestration logic

## 4. Process and Terminal Layer

### Choice

- `portable-pty` for PTY abstraction
- fallback non-PTY stream mode when PTY is unavailable

### Linux notes

- PTY path is mature and preferred.

### Windows notes

- ConPTY support exists but requires extra compatibility testing.
- Keep fallback plain stream mode for reliability.

## 5. Configuration and Data

### Configuration

- `hydra.toml` parsed via `serde` + `toml`

### Artifact storage

- file-first artifact model under `.hydra/runs/<run_id>/`
- optional SQLite index for history and GUI queries

Rationale:
- file artifacts are transparent and portable
- SQLite gives query ergonomics without service dependency

## 6. Git Integration

### Choice

- shell out to git CLI (`std::process`/Tokio process) for v1

### Why not libgit2 first

- git CLI behavior aligns better with existing user environment
- easier debugging and parity with user expectations

Future option:
- selective migration to git library where shelling out causes portability issues

## 7. Frontend Libraries

### Recommended baseline

- React + TypeScript
- state: lightweight store (e.g., Zustand) or React context
- diff view: Monaco diff or equivalent high-fidelity viewer
- terminal rendering: xterm.js

### Styling

- utility CSS with tokenized theme variables
- avoid design debt by defining component-level style primitives early

## 8. Testing Stack

### Rust

- unit tests for adapter parsing and scoring formulas
- integration tests for workflow execution in temp repos

### Frontend

- component tests for score/diff panels
- smoke tests for end-to-end launch and event rendering

### Cross-platform

- CI matrix on Linux and Windows for critical orchestration tests

## 9. Versioning and Upgrade Policy

### Policy

- pin core dependencies in lockfile
- introduce adapter compatibility matrix by external CLI version
- run compatibility probes at startup

### Why

External agent CLIs evolve quickly; Hydra must degrade gracefully instead of hard-failing.

## 10. Tradeoff Table

| Decision | Benefit | Cost |
|---|---|---|
| Rust + Tokio | performance and safety | steeper development curve |
| Tauri + React | lightweight cross-platform GUI | webview complexity |
| Worktree isolation | strong collision prevention | extra disk usage and cleanup logic |
| File artifacts + SQLite index | auditability + queryability | dual storage management |

## 11. Deferred Decisions

1. Plugin ABI for third-party adapters (post-v1 stabilization).
2. Native git library adoption vs shell-out permanence.
3. Optional remote execution backend.

## 12. External References

- Tauri project: https://github.com/tauri-apps/tauri
- Codex CLI docs/source: https://developers.openai.com/codex/cli and https://github.com/openai/codex
- Claude Code docs: https://docs.anthropic.com/en/docs/claude-code/overview
- Cursor CLI docs: https://docs.cursor.com/en/cli/headless
