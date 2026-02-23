# Sprint 1 Cut (Dependency-Safe, First 8 Tickets)

Last updated: 2026-02-23

## 1. Sprint Goal

Establish a reliable readiness and foundation layer:
- adapter probe system for Tier-1 launch adapters (`claude`, `codex`)
- experimental probe path for `cursor-agent`
- deterministic run artifact convention
- initial core workspace and config parser

## 2. Ticket Set (First 8)

Execution order (dependency-safe):
1. `M0.1` Adapter Probe Framework
2. `M0.2` Claude Probe Implementation
3. `M0.3` Codex Probe Implementation
4. `M0.4` Cursor Experimental Probe
5. `M0.5` Run Artifact Convention
6. `M0.6` Doctor Command MVP
7. `M1.1` Core Workspace Scaffold
8. `M1.2` Config Parser and Defaults

## 3. Why This Order

1. `M0.1` is required by all probe implementations.
2. `M0.2` and `M0.3` establish Tier-1 readiness checks early.
3. `M0.4` adds experimental gating without blocking Tier-1.
4. `M0.5` creates deterministic artifact shape needed by later commands.
5. `M0.6` ships immediate operator value (`hydra doctor`).
6. `M1.1` and `M1.2` prepare the codebase for orchestration features in sprint 2.

## 4. Parallelization Plan

Lane A (adapter readiness):
- `M0.1` -> (`M0.2`, `M0.3`, `M0.4`) -> `M0.6`

Lane B (core scaffolding):
- `M0.5` -> `M1.1` -> `M1.2`

Cross-lane sync points:
- `M0.6` should consume artifact/config conventions from lane B where relevant.

## 5. Per-Ticket Definition of Ready

For each of the 8 tickets:
1. Acceptance criteria copied from `research/implementation-checklist.md`.
2. Test strategy stated (unit/fixture/integration).
3. Owner and estimate assigned.
4. Dependencies linked in issue.

## 6. Sprint Exit Criteria

Sprint 1 is done when:
1. `hydra doctor` reports Tier-1 readiness with stable JSON output.
2. Cursor probe is classified experimental and never default-enabled.
3. `.hydra/runs/<run_id>/manifest.json` and `events.jsonl` are consistently produced.
4. Workspace and config parser compile and pass CI on Linux and Windows.

## 7. Risks and Controls

| Risk | Impact | Control |
|---|---|---|
| Probe parsing brittle across CLI versions | High | Fixture-based parser tests and explicit unknown-state handling |
| Scope overlap between doctor output and config parser | Medium | Shared output schema and one owner for schema changes |
| Experimental adapter accidentally enabled by default | High | Startup policy test that enforces opt-in gate |

## 8. Suggested Sprint 2 Seed (Not In Scope)

- `M1.3` Worktree Lifecycle Service
- `M1.4` Process Supervisor (Single Agent)
- `M1.5` Claude Adapter Runtime Path
- `M1.6` Codex Adapter Runtime Path
