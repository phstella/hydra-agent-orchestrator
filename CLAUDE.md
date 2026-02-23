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

`M<phase>.<sequence>` - e.g., `M0.1`, `M2.11`, `M6.6`.
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
- Full checklist (53 milestones): `planning/implementation-checklist.md`
- Sprint 1 cut (10 tickets): `planning/sprint-1-cut.md`
- Phase instructions: `instructions/phase-0.md` through `phase-6.md`
