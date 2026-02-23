# Technology Stack

## Decision Framework

Every technology choice is evaluated against three criteria:
1. **Fitness for purpose** — does it solve the specific problem well?
2. **Cross-platform** — does it work on Linux and Windows (macOS as bonus)?
3. **Weight** — Hydra runs alongside resource-hungry AI agents; every MB and every CPU cycle matters.

---

## Backend: Rust

### Choice: Rust (2021 edition)

### Justification

Hydra's backend manages concurrent child processes, streams their output in real time, handles git operations, and evaluates code quality. This is systems-level work that demands:

- **Concurrent process management** — spawning and supervising 3+ CLI agent processes simultaneously
- **Stream processing** — parsing JSONL output from multiple agents in real time without blocking
- **Low overhead** — the backend must be lightweight since the agents themselves consume significant CPU/RAM
- **Safety** — process handles, file descriptors, and git operations must not leak

Rust meets all four. The ownership system prevents resource leaks. Tokio provides async concurrency without goroutine-style GC pressure. Compiled binaries are small (~5-10MB) and start instantly.

### Alternatives Considered

| Alternative | Why rejected |
|---|---|
| **Go** | Claude Squad uses Go successfully, but Go's error handling is verbose for the adapter pattern, and Tauri's backend must be Rust — using Go would mean two languages in the backend or IPC overhead |
| **TypeScript/Node.js** | Parallel Code uses this, but Node's single-threaded event loop is awkward for managing multiple PTY streams; also adds V8 memory overhead alongside the agents |
| **Python** | Poor process management primitives; GIL limits true concurrency; not suitable for a desktop app backend |

---

## Async Runtime: Tokio

### Choice: tokio 1.x

### Justification

Hydra needs to:
- Spawn multiple child processes concurrently
- Stream stdout/stderr from each process without blocking others
- Handle timeouts (watchdog timers for hung agents)
- Fan out events to multiple subscribers (GUI, CLI, scoring engine)

Tokio is the standard async runtime for Rust and provides all of this:
- `tokio::process::Command` for async child process management
- `tokio::sync::broadcast` for the EventBus fan-out pattern
- `tokio::time::timeout` for agent watchdog timers
- `tokio::join!` / `tokio::select!` for concurrent agent execution

### Alternatives Considered

| Alternative | Why rejected |
|---|---|
| **async-std** | Smaller ecosystem; `tokio::process` is more mature and better documented for PTY/child process use cases |
| **smol** | Lightweight but lacks the process management primitives Hydra needs out of the box |
| **Synchronous (std threads)** | Too much boilerplate for concurrent stream processing; no built-in timeout support |

---

## Desktop Framework: Tauri v2

### Choice: Tauri v2 (2.6.x)

### Justification

Hydra needs a cross-platform desktop GUI that:
- Runs on Linux (X11 + Wayland) and Windows
- Is lightweight — users are already running 3 AI agents consuming significant resources
- Can embed terminal emulators (xterm.js)
- Has native Rust backend integration (Hydra's core is Rust)

Tauri v2 is the natural fit:
- Ships apps at **2-5MB** (uses the OS web renderer, not a bundled Chromium)
- Backend is Rust — `hydra-core` integrates directly, no IPC serialization overhead
- `tauri-plugin-pty` (v0.2.1) provides PTY support for terminal emulation
- Native event system for Rust → frontend communication
- Supports Linux (X11, Wayland), Windows, macOS, Android, iOS

### Alternatives Considered

| Alternative | Why rejected |
|---|---|
| **Electron** | 150MB+ bundle size; bundles Chromium; high memory usage — unacceptable when running alongside AI agents. Mux by Coder uses Electron and is noticeably heavy. |
| **GTK4 (native)** | Excellent on Linux, poor on Windows. No web-based terminal emulator support — would need to build a custom terminal widget. |
| **Slint** | Promising Rust-native UI, but too immature for the complexity Hydra needs (no xterm.js, limited component ecosystem) |
| **CLI-only (no GUI)** | Claude Squad proves this works for some users, but the plan calls for a visual diff viewer, scoreboard, and merge controls that benefit enormously from a GUI |

---

## Frontend: React + TypeScript + TailwindCSS

### Choice: React 19 + TypeScript 5.x + TailwindCSS 4.x

### Justification

The Tauri frontend is a web view. The frontend must render:
- Multiple xterm.js terminal instances in a responsive grid
- A diff viewer comparing agent outputs
- A scoreboard with charts and color-coded rankings
- Form controls for task input and agent selection

React is chosen for:
- The largest component ecosystem (xterm.js React wrappers, diff viewers, chart libraries all exist)
- TypeScript for type-safe Tauri IPC (typed invoke wrappers matching Rust command signatures)
- TailwindCSS for rapid, consistent styling without a heavy CSS framework

### Alternatives Considered

| Alternative | Why rejected |
|---|---|
| **Svelte** | Excellent DX, but smaller ecosystem — fewer xterm.js integration examples, fewer diff viewer components |
| **Vue** | Viable, but React has more Tauri community examples and the xterm.js ecosystem is React-heavy |
| **Vanilla JS** | Too much boilerplate for the state management needed (3+ terminal streams, scores, diffs, race lifecycle) |
| **Solid** | Performant but ecosystem is too small for the specialized components Hydra needs |

---

## Terminal Emulator: xterm.js

### Choice: xterm.js 5.x with `@xterm/addon-fit`

### Justification

Hydra must render live terminal output from CLI agents, including:
- ANSI color codes (agents use color extensively)
- Interactive prompt rendering
- Real-time streaming (output appears as the agent works, not after completion)

xterm.js is what VS Code uses for its integrated terminal. It handles all of the above and runs in a web view, which is exactly Tauri's frontend environment. The `addon-fit` module auto-sizes the terminal to its container, essential for the responsive grid layout.

### Alternatives Considered

| Alternative | Why rejected |
|---|---|
| **Raw `<pre>` rendering** | Cannot render ANSI codes, cursor movement, or interactive elements |
| **Ghostty embedding** | cmux uses this — macOS only, not embeddable in a web view |
| **Custom canvas renderer** | Massive engineering effort to replicate what xterm.js already does |

---

## PTY Management: portable-pty + tauri-plugin-pty

### Choice: `portable-pty` 0.9.x (via `tauri-plugin-pty` 0.2.1)

### Justification

CLI agents expect to run in a real terminal (PTY). Without a PTY, they may:
- Disable color output
- Skip interactive prompts
- Behave differently than when run manually

`portable-pty` is a cross-platform Rust crate that allocates pseudo-terminals on Linux (via `/dev/ptmx`) and Windows (via ConPTY). `tauri-plugin-pty` wraps it for Tauri's plugin system, providing frontend APIs to spawn PTY processes and connect them to xterm.js.

For the CLI-only mode (`hydra-cli`), `portable-pty` is used directly without the Tauri plugin.

### Alternatives Considered

| Alternative | Why rejected |
|---|---|
| **pty-process** | Tokio-native but Linux-only (POSIX `forkpty`); no Windows support |
| **Raw `std::process::Command`** | No PTY allocation — agents don't get a terminal and may behave differently |
| **nix crate (low-level)** | Too low-level; `portable-pty` provides the right abstraction |

---

## CLI Framework: clap

### Choice: `clap` 4.x with derive macros

### Justification

`hydra-cli` needs subcommands (`race`, `collab`, `merge`, `status`), typed arguments, and auto-generated help text. `clap` is the standard Rust CLI framework and supports all of this with minimal boilerplate via derive macros.

```rust
#[derive(Parser)]
#[command(name = "hydra")]
enum Cli {
    Race(RaceArgs),
    Collab(CollabArgs),
    Merge(MergeArgs),
    Status(StatusArgs),
}
```

### Alternatives Considered

| Alternative | Why rejected |
|---|---|
| **argh** | Simpler but lacks `clap`'s ecosystem (completions, man page generation) |
| **structopt** | Merged into clap 4 — use clap directly |

---

## Configuration: TOML

### Choice: `toml` crate with `serde` deserialization

### Justification

`hydra.toml` sits at the project root and configures agents, scoring weights, collaboration workflows, and general settings. TOML is:
- Human-readable and human-editable
- The standard config format in the Rust ecosystem (`Cargo.toml`)
- Supports nested tables, arrays, and inline tables — sufficient for Hydra's config structure

### Alternatives Considered

| Alternative | Why rejected |
|---|---|
| **YAML** | More ambiguous parsing rules; indentation-sensitive; the "Norway problem" (`NO` parsed as boolean) |
| **JSON** | No comments; verbose; unfriendly for human editing |
| **RON** | Rust-specific format; unfamiliar to most users |

---

## Diff Engine: similar (Rust) + Monaco Editor (GUI)

### Choice: `similar` crate for programmatic diff; Monaco Editor diff view for the GUI

### Justification

Hydra computes diffs in two contexts:
1. **Scoring** — programmatic diff to count lines changed, files touched (Rust-side)
2. **Display** — visual side-by-side diff for the user to review agent outputs (GUI-side)

For scoring, `git diff --stat` is sufficient and avoids additional dependencies. For detailed programmatic diff (e.g., feeding diffs to reviewer agents), the `similar` crate provides a pure-Rust unified diff implementation.

For the GUI, Monaco Editor (the editor component from VS Code) includes a built-in diff view that handles syntax highlighting, line-level and word-level diffs, and large files. It runs in the web view and is well-documented.

### Alternatives Considered

| Alternative | Why rejected |
|---|---|
| **diff crate** | Less maintained than `similar` |
| **react-diff-viewer** | Lightweight but no syntax highlighting; Monaco is more capable |
| **CodeMirror** | Viable, but Monaco's diff view is more mature and closer to what developers expect |

---

## Session Storage: SQLite

### Choice: `rusqlite` (via `r2d2-sqlite` connection pool)

### Justification

Phase 4 adds session history — storing past races with their task, agents, scores, timestamps, diffs, and costs. SQLite is:
- Embedded (no external database server)
- Single-file storage (`.hydra/history.db`)
- Well-supported in Rust via `rusqlite`
- Sufficient for the read/write patterns (write once per race, read for history view)

### Alternatives Considered

| Alternative | Why rejected |
|---|---|
| **JSON files** | Hard to query; grows unwieldy with hundreds of races |
| **sled** | Embedded KV store, but SQL queries are more natural for history browsing (filter by date, agent, score range) |
| **PostgreSQL/MySQL** | Overkill; requires external server; defeats the "lightweight tool" goal |

---

## Summary

| Layer | Choice | Key Crate/Package |
|---|---|---|
| Language | Rust (2021 edition) | — |
| Async runtime | Tokio | `tokio` 1.x |
| Desktop framework | Tauri v2 | `tauri` 2.6.x |
| Frontend framework | React + TypeScript | `react` 19, `typescript` 5.x |
| Styling | TailwindCSS | `tailwindcss` 4.x |
| Terminal emulator | xterm.js | `@xterm/xterm` 5.x |
| PTY management | portable-pty | `portable-pty` 0.9.x, `tauri-plugin-pty` 0.2.x |
| CLI framework | clap | `clap` 4.x |
| Configuration | TOML | `toml`, `serde` |
| Diff engine (backend) | similar | `similar` |
| Diff viewer (frontend) | Monaco Editor | `@monaco-editor/react` |
| Session storage | SQLite | `rusqlite`, `r2d2` |
| Serialization | serde + serde_json | `serde`, `serde_json` |
| UUID generation | uuid | `uuid` 1.x |
| Logging | tracing | `tracing`, `tracing-subscriber` |
