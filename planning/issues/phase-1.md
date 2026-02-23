# Phase 1 Tickets (Core Orchestrator + Single Agent) Issue Bodies

Last updated: 2026-02-22

Generated from `planning/implementation-checklist.md`.

Global label prefix: `hydra`

## [M1.1] Core Workspace Scaffold

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: S
- Dependencies: none

### Issue Body (Markdown)

```md
## Problem
No Rust workspace exists yet. All subsequent features need a compilable crate structure with shared error handling and logging.

## Scope
Create `hydra-core` library crate and `hydra-cli` binary crate in a Cargo workspace. Wire `tracing` for structured logging and `thiserror`/`anyhow` for error handling. Set up CI for Linux and Windows compilation.

## Acceptance Criteria
- [ ] Workspace builds with hydra-core and hydra-cli crates.
- [ ] Logging and error crates wired consistently.
- [ ] CI compiles on Linux and Windows.

## Out of Scope
Runtime logic, config parsing, adapter code.

## Dependencies
- none

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M1.2] Config Parser and Defaults

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: S
- Dependencies: M1.1

### Issue Body (Markdown)

```md
## Problem
Hydra needs a user-editable configuration file to control scoring weights, adapter preferences, timeout values, and artifact retention. Without a config parser, all behavior must be hardcoded or passed via CLI flags.

## Scope
Implement `hydra.toml` parser using `serde` + `toml` crate. Define the full configuration schema with typed fields, defaults for all optional values, and actionable validation error messages.

## Acceptance Criteria
- [ ] hydra.toml parses with schema validation.
- [ ] Missing optional fields get deterministic defaults.
- [ ] Invalid config returns actionable error messages.

## Out of Scope
GUI config editor; runtime config reload.

## Dependencies
- M1.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M1.3] Worktree Lifecycle Service

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: M
- Dependencies: M1.1

### Issue Body (Markdown)

```md
## Problem
Agents must run in isolated worktrees to prevent file collisions. Worktree creation, tracking, and cleanup (including on interrupt and failure) must be reliable to avoid orphaned directories and branches.

## Scope
Implement worktree create/list/remove operations via git CLI. Add interrupt-safe cleanup using signal handlers. Ensure paths are valid on both Linux and Windows.

## Acceptance Criteria
- [ ] Create/list/remove worktree operations are implemented.
- [ ] Interrupt-safe cleanup path exists.
- [ ] Windows path handling tests pass.

## Out of Scope
Garbage collection scheduler; sparse checkout support.

## Dependencies
- M1.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M1.4] Process Supervisor (Single Agent)

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: M
- Dependencies: M1.1

### Issue Body (Markdown)

```md
## Problem
Agent CLI processes need lifecycle management: spawning with correct arguments, streaming stdout/stderr, enforcing timeouts, and graceful cancellation. Without supervision, hung agents consume resources indefinitely and produce no usable artifacts.

## Scope
Build a single-agent process supervisor with start, stream, timeout (idle + hard), and cancel support. Implement bounded output buffering and emit normalized lifecycle events to the event bus.

## Acceptance Criteria
- [ ] Supports start, stream, timeout, cancel.
- [ ] Bounded output buffering prevents memory blowups.
- [ ] Emits normalized lifecycle events.

## Out of Scope
Parallel supervision (Phase 2); PTY layer (handled separately).

## Dependencies
- M1.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M1.5] Claude Adapter Runtime Path

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-adapter, type-feature
- Estimate: M
- Dependencies: M0.2, M1.4

### Issue Body (Markdown)

```md
## Problem
The Claude adapter probe (M0.2) validates binary presence and flags, but the actual runtime path (spawning Claude in a worktree, parsing its stream-json output, mapping events to the normalized schema) has not been implemented.

## Scope
Implement `build_command()` and `parse_line()`/`parse_raw()` for the Claude adapter. Wire it through the process supervisor. Cover timeout and cancellation with integration tests.

## Acceptance Criteria
- [ ] claude runs in isolated worktree.
- [ ] Stream parser maps key events to normalized schema.
- [ ] Timeout and cancellation are covered by tests.

## Out of Scope
Multi-agent orchestration; scoring.

## Dependencies
- M0.2, M1.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M1.6] Codex Adapter Runtime Path

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-adapter, type-feature
- Estimate: M
- Dependencies: M0.3, M1.4

### Issue Body (Markdown)

```md
## Problem
The Codex adapter probe (M0.3) validates binary presence and flags, but the runtime path (spawning `codex exec` in a worktree, parsing JSON stream output, handling flag variants) has not been implemented.

## Scope
Implement `build_command()` and `parse_line()`/`parse_raw()` for the Codex adapter. Handle known flag variants gracefully. Wire through process supervisor with integration tests.

## Acceptance Criteria
- [ ] codex exec works in isolated worktree.
- [ ] JSON stream parser maps events and usage data.
- [ ] Unsupported flag fallback logic is tested.

## Out of Scope
Multi-agent orchestration; scoring.

## Dependencies
- M0.3, M1.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M1.7] CLI Race Command (Single Agent)

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: S
- Dependencies: M1.2, M1.3, M1.5

### Issue Body (Markdown)

```md
## Problem
There is no end-to-end CLI command that ties together config parsing, worktree creation, and agent execution. Users need a single command to run an agent on a task and get results.

## Scope
Implement `hydra race --agents <agent>` command using clap. Wire config -> worktree -> adapter -> supervisor -> artifact output into a single flow. Output run summary with branch name and artifact path.

## Acceptance Criteria
- [ ] hydra race --agents claude completes end-to-end.
- [ ] Run summary includes branch and artifact path.
- [ ] Non-zero exit codes on fatal failures.

## Out of Scope
Multi-agent parallel execution; scoring.

## Dependencies
- M1.2, M1.3, M1.5

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M1.8] Interrupt and Recovery Tests

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-test, type-test
- Estimate: M
- Dependencies: M1.3, M1.4, M1.7

### Issue Body (Markdown)

```md
## Problem
Interrupted runs (Ctrl+C, process kill, system crash) can leave orphaned worktrees, stale branches, and incomplete artifacts. These failure paths must be tested to ensure cleanup is reliable.

## Scope
Write integration tests for interrupt scenarios: Ctrl+C during agent execution, agent process crash, partial completion. Verify worktree and branch cleanup, artifact integrity, and absence of orphaned resources.

## Acceptance Criteria
- [ ] Ctrl+C cleanup test passes.
- [ ] Partial failure leaves usable artifacts.
- [ ] No orphan worktrees after test run.

## Out of Scope
Crash recovery metadata (Phase 6); Windows-specific interrupt behavior (Phase 6).

## Dependencies
- M1.3, M1.4, M1.7

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```
