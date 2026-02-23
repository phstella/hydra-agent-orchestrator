# Hydra Research Index

Last updated: 2026-02-22

## Product Summary

Hydra is a local **code-agent orchestration control center**. It runs multiple coding agents on isolated git workspaces, compares their outcomes, and helps users merge the best result safely.

Launch adapter policy:
1. Tier-1 adapters: `claude`, `codex`
2. Tier-2 experimental: `cursor-agent` (opt-in only until probe and conformance gates pass)

Primary platform priority:
1. Linux (first-class)
2. Windows (first-class after Linux parity)
3. macOS (best effort in early phases)

## Core Hypothesis

Teams using multiple coding agents lose time in manual orchestration:
- manually creating branches/worktrees
- manually launching each agent
- manually diffing and evaluating outputs
- manually merging winner changes

Hydra should reduce this overhead by making parallel execution, comparison, and merge decisions deterministic, auditable, and fast.

## Scope Baseline

In-scope for initial product:
- CLI-agent orchestration (headless/non-interactive mode)
- worktree-per-agent isolation
- race mode (same task, multiple agents)
- score-based ranking and merge guidance
- Linux-first runtime and UX

Out-of-scope for initial product:
- cloud-hosted remote execution fabric
- long-lived autonomous agent swarms
- natural-language-only git conflict resolution without human review

## Document Map

### Reference Documentation (`docs/`)

- `docs/architecture.md`: system and runtime design (includes locked ADRs for process model and storage model)
- `docs/agent-adapters.md`: per-agent adapter contracts and CLI specifics
- `docs/scoring-engine.md`: evaluation model and calibration
- `docs/collaboration-workflows.md`: multi-agent workflow definitions
- `docs/tech-stack.md`: technology decisions and tradeoffs
- `docs/competitive-analysis.md`: market scan and differentiation

### Project Management (`planning/`)

- `planning/roadmap.md`: phased delivery plan, gates, risks, and NFR targets
- `planning/implementation-checklist.md`: issue-ready execution checklist (47 milestones, `M0.1` to `M5.6`)
- `planning/github-issues.md`: copy-paste-ready GitHub issue bodies (47 issues)
- `planning/sprint-1-cut.md`: dependency-safe first sprint ticket cut (10 tickets)
- `planning/issues/README.md`: per-phase issue packs index
- `planning/audit.md`: quality audit summary, revision history, and unresolved gaps

### Agent Guidance

- `CLAUDE.md`: agent entry point - project overview, conventions, locked decisions
- `PROGRESS.md`: living state file - current phase, completed work, next steps
- `instructions/`: per-phase implementation guidance (`phase-0.md` through `phase-5.md`)

## Quality Bar For This Research Package

This folder is considered "release ready" when:
1. Every major product claim maps to an implementation section.
2. CLI integration assumptions are either source-verified or explicitly marked uncertain.
3. Linux and Windows behavior differences are documented for each critical subsystem.
4. Open questions are explicit and assigned to a discovery phase or milestone.

## Open Product Questions

1. Should `aider` be the next experimental adapter after `cursor-agent`?
2. Should merge automation default to "suggest only" or "auto-merge above threshold"?
3. Do we persist complete run artifacts by default, or keep only summaries unless opted in?
4. Should workflow composition ship in v1, or after race-mode hardening?

## External Sources Used During This Update

- https://developers.openai.com/codex/cli
- https://docs.anthropic.com/en/docs/claude-code/overview
- https://docs.anthropic.com/en/docs/claude-code/settings
- https://docs.cursor.com/en/cli/headless
- https://docs.cursor.com/cli/reference/parameters
- https://docs.cursor.com/en/cli/reference/output-format
- https://github.com/openai/codex
- https://github.com/smtg-ai/claude-squad
- https://github.com/johannesjo/parallel-code
- https://github.com/coder/mux
- https://github.com/manaflow-ai/cmux
