# Hydra Agent Orchestrator

Read this file at the start of every session. It is the single source of
truth for project structure, conventions, and current state.

## What is Hydra

Hydra is a local code-agent orchestration control center. It runs multiple
coding agents (Claude, Codex, and experimental adapters) on isolated git
worktrees, scores their outputs deterministically, and guides the user
toward a safe merge of the best result.

## Repo Layout

```
CLAUDE.md              <- you are here (read every session)
PROGRESS.md            <- living state file (update every session)
research.md            <- research index and product summary
docs/                  <- reference documentation (architecture, adapters, scoring, etc.)
planning/              <- project management (roadmap, checklist, issues, sprints)
  issues/              <- per-phase issue packs
instructions/          <- per-phase implementation guidance for agents
```

## Target Crate Structure

```
Cargo.toml             (workspace root)
crates/
  hydra-core/          (library: orchestration, scoring, merge, adapters)
  hydra-cli/           (binary: CLI frontend via clap)
  hydra-app/           (Tauri v2 + React: GUI frontend, Phase 3)
```

## Coding Conventions

### Rust
- Edition 2021, stable toolchain.
- `tokio` async runtime. Avoid `block_on` inside async contexts.
- Error handling: `thiserror` for library errors, `anyhow` for CLI.
- Logging: `tracing` with structured fields. No `println!` in library code.
- Formatting: `cargo fmt` before every commit. `cargo clippy --all-targets -- -D warnings` must pass.
- Tests: `#[cfg(test)]` unit tests in-file; integration tests in `tests/`.

### Git
- Branch naming: `feat/<short-name>`, `fix/<short-name>`, `chore/<short-name>`.
- Commit messages: imperative mood, reference milestone ID when applicable
  (e.g., `M0.1: implement adapter probe framework`).
- One logical change per commit. Squash fixups before merge.

### Artifacts
- Run artifacts live under `.hydra/runs/<run_id>/`.
- Source of truth is JSONL (`events.jsonl`); SQLite index is derived (Phase 3+).
- Never commit `.hydra/` to the repo.

## Locked Design Decisions

1. Git worktrees for agent isolation (not clones).
2. Short-lived process model for CLI; GUI embeds `hydra-core` directly.
3. JSONL source of truth; SQLite derived index from Phase 3.
4. Tier-1 adapters: `claude`, `codex`. Experimental: `cursor-agent`.
5. Scoring is deterministic from captured artifacts.
6. Merge is explicit by default; auto-merge is policy-gated.

## Milestone ID Format

`M<phase>.<sequence>` - e.g., `M0.1`, `M2.11`, `M5.6`.
Use as commit prefix and issue title prefix.

## PROGRESS.md Protocol

1. Read `PROGRESS.md` at session start to understand current state.
2. Before starting work, update `In-Progress Work` section.
3. After completing a milestone, move it to `Completed Milestones`.
4. Record any non-obvious decisions in `Decisions Made`.
5. Leave clear `Instructions for Next Agent` at session end.

## Key Reference Paths

- Architecture & ADRs: `docs/architecture.md`
- Adapter contracts: `docs/agent-adapters.md`
- Scoring formulas: `docs/scoring-engine.md`
- Roadmap & phases: `planning/roadmap.md`
- Full checklist (47 milestones): `planning/implementation-checklist.md`
- Sprint 1 cut (10 tickets): `planning/sprint-1-cut.md`
- Phase instructions: `instructions/phase-0.md` through `phase-5.md`

## Workflow Orchestration

### 1. Plan Node Default
- Enter plan mode for ANY non-trivial task (3+ steps or architectural decisions)
- If something goes sideways, STOP and re-plan immediately - don't keep pushing
- Use plan mode for verification steps, not just building
- Write detailed specs upfront to reduce ambiguity

### 2. Subagent Strategy
- Use subagents liberally to keep main context window clean
- Offload research, exploration, and parallel analysis to subagents
- For complex problems, throw more compute at it via subagents
- One task per subagent for focused execution

### 3. Self-Improvement Loop
- After ANY correction from the user: update `tasks/lessons.md` with the pattern
- Write rules for yourself that prevent the same mistake
- Ruthlessly iterate on these lessons until mistake rate drops
- Review lessons at session start for relevant project

### 4. Verification Before Done
- Never mark a task complete without proving it works
- Diff behavior between main and your changes when relevant
- Ask yourself: "Would a staff engineer approve this?"
- Run tests, check logs, demonstrate correctness

### 5. Demand Elegance (Balanced)
- For non-trivial changes: pause and ask "is there a more elegant way?"
- If a fix feels hacky: "Knowing everything I know now, implement the elegant solution"
- Skip this for simple, obvious fixes - don't over-engineer
- Challenge your own work before presenting it

### 6. Autonomous Bug Fixing
- When given a bug report: just fix it. Don't ask for hand-holding
- Point at logs, errors, failing tests - then resolve them
- Zero context switching required from the user
- Go fix failing CI tests without being told how

## Task Management

1. **Plan First**: Write plan to `tasks/todo.md` with checkable items
2. **Verify Plan**: Check in before starting implementation
3. **Track Progress**: Mark items complete as you go
4. **Explain Changes**: High-level summary at each step
5. **Document Results**: Add review section to `tasks/todo.md`
6. **Capture Lessons**: Update `tasks/lessons.md` after corrections

## Core Principles

- **Simplicity First**: Make every change as simple as possible. Impact minimal code.
- **No Laziness**: Find root causes. No temporary fixes. Senior developer standards.
- **Minimal Impact**: Changes should only touch what's necessary. Avoid introducing bugs.
