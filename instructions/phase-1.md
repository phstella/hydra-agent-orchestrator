# Phase 1: Core Orchestrator + Single Agent

**Goal**: Stable end-to-end loop for one agent in an isolated worktree.

**Duration estimate**: 2-3 weeks

## Milestones

| ID | Title | Estimate | Dependencies |
|----|-------|----------|--------------|
| M1.1 | Core Workspace Scaffold | S | none |
| M1.2 | Config Parser and Defaults | S | M1.1 |
| M1.3 | Worktree Lifecycle Service | M | M1.1 |
| M1.4 | Process Supervisor (Single Agent) | M | M1.1 |
| M1.5 | Claude Adapter Runtime Path | M | M0.2, M1.4 |
| M1.6 | Codex Adapter Runtime Path | M | M0.3, M1.4 |
| M1.7 | CLI Race Command (Single Agent) | S | M1.2, M1.3, M1.5 |
| M1.8 | Interrupt and Recovery Tests | M | M1.3, M1.4, M1.7 |

## Parallelization

After M1.1 (workspace scaffold), three streams can run in parallel:

- **Config + Worktree**: M1.2, M1.3
- **Process Supervisor + Adapters**: M1.4 -> (M1.5, M1.6)
- M1.7 merges the config, worktree, and adapter streams.
- M1.8 is the integration test gate.

## What to Build

- **Workspace scaffold** (M1.1): Cargo workspace with `hydra-core` lib crate and
  `hydra-cli` binary crate. Wire `tracing`, `thiserror`, `anyhow`. Set up CI.

- **Config parser** (M1.2): `hydra.toml` via `serde` + `toml`. Full schema with
  defaults. See `docs/scoring-engine.md` section 12 for example config.

- **Worktree service** (M1.3): Create/list/remove worktrees via git CLI.
  Interrupt-safe cleanup. See `docs/architecture.md` section 3 for isolation model.

- **Process supervisor** (M1.4): Single-agent lifecycle: spawn, stream, timeout,
  cancel. Bounded output buffering. See `docs/architecture.md` section 4.3.

- **Adapter runtime** (M1.5, M1.6): `build_command()` and `parse_line()`/`parse_raw()`
  for Claude and Codex. See `docs/agent-adapters.md` sections 4-5.

- **Race command** (M1.7): `hydra race --agents <agent>` end-to-end flow.

- **Interrupt tests** (M1.8): Ctrl+C, crash, partial completion scenarios.

## Exit Criteria

1. Run starts from any valid git repo.
2. Worktree isolation works correctly.
3. Cleanup is deterministic on success, failure, and interrupt.

## References

- Acceptance criteria: `planning/implementation-checklist.md` section 4
- Issue bodies: `planning/issues/phase-1.md`
