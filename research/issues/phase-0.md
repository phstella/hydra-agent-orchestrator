# Phase 0 Tickets (Validation and Guardrails) Issue Bodies

Last updated: 2026-02-22

Generated from `research/implementation-checklist.md`.

Global label prefix: `hydra`

## [M0.1] Adapter Probe Framework

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-adapter, type-feature
- Estimate: M
- Dependencies: none

### Issue Body (Markdown)

```md
## Problem
Adapter assumptions can drift silently.

## Scope
Add probe interface and unified probe report model.

## Acceptance Criteria
- [ ] hydra doctor emits JSON report with adapter probe status.
- [ ] Probe output includes binary path, version, supported flags, confidence.
- [ ] Unknown adapters do not crash doctor command.

## Out of Scope
Full adapter execution.

## Dependencies
- none

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M0.2] Claude Probe Implementation

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-adapter, type-feature
- Estimate: S
- Dependencies: M0.1

### Issue Body (Markdown)

```md
## Problem
Tier-1 adapter must be validated at startup.

## Scope
Implement Claude probe for required headless flags.

## Acceptance Criteria
- [ ] Probe verifies -p and --output-format support.
- [ ] Probe result status is ready or blocked with clear reason.
- [ ] Fixture-based probe test passes in CI.

## Out of Scope
runtime parsing logic.

## Dependencies
- M0.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M0.3] Codex Probe Implementation

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-adapter, type-feature
- Estimate: S
- Dependencies: M0.1

### Issue Body (Markdown)

```md
## Problem
Tier-1 adapter must be validated at startup.

## Scope
Implement Codex probe for exec, --json, and approval mode flags.

## Acceptance Criteria
- [ ] Probe verifies exec subcommand exists.
- [ ] Probe verifies JSON output flag support.
- [ ] Probe handles known flag variants without panic.

## Out of Scope
full scoring integration.

## Dependencies
- M0.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M0.4] Cursor Experimental Probe

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-adapter, type-feature
- Estimate: M
- Dependencies: M0.1

### Issue Body (Markdown)

```md
## Problem
Cursor interface variability must not break default flows.

## Scope
Add Cursor probe with experimental classification.

## Acceptance Criteria
- [ ] Cursor probe never promotes adapter to Tier-1.
- [ ] Probe result can be experimental-ready, experimental-blocked, or missing.
- [ ] UI and CLI mark adapter as experimental.

## Out of Scope
enabling Cursor by default.

## Dependencies
- M0.1

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M0.5] Run Artifact Convention

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-core, type-feature
- Estimate: S
- Dependencies: none

### Issue Body (Markdown)

```md
## Problem
Runs need deterministic artifact paths for replay.

## Scope
Define .hydra/runs/<run_id>/ structure and write metadata manifest.

## Acceptance Criteria
- [ ] Every run writes manifest.json and events.jsonl.
- [ ] Artifact paths are OS-safe on Linux and Windows.
- [ ] Cleanup policy respects retention config.

## Out of Scope
GUI history viewer.

## Dependencies
- none

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M0.6] Doctor Command MVP

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-core, type-feature
- Estimate: S
- Dependencies: M0.1, M0.2, M0.3, M0.4

### Issue Body (Markdown)

```md
## Problem
Users need quick readiness check before run.

## Scope
Implement hydra doctor summary + JSON output mode.

## Acceptance Criteria
- [ ] Exit code is non-zero when Tier-1 prerequisites fail.
- [ ] Output includes git repo checks and adapter readiness.
- [ ] --json output is stable and parseable.

## Out of Scope
auto-fix behavior.

## Dependencies
- M0.1, M0.2, M0.3, M0.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M0.7] Security Baseline Implementation

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-core, type-feature
- Estimate: M
- Dependencies: M0.5

### Issue Body (Markdown)

```md
## Problem
Security intent exists in architecture docs but has no implementation milestones. Agent processes inherit environment variables including API keys; logs and artifacts can capture secrets from agent output.

## Scope
Implement secret redaction rules for logs and artifacts, sandbox policy enforcement for agent worktrees, and unsafe-mode guardrails.

## Acceptance Criteria
- [ ] Known secret patterns (API keys, tokens) are redacted from persisted logs and artifacts.
- [ ] Agent processes cannot write outside their assigned worktree unless unsafe mode is explicitly enabled.
- [ ] Unsafe mode requires explicit per-run opt-in flag and emits a visible warning.
- [ ] Log scrubbing unit tests pass with known secret fixtures.

## Out of Scope
full threat model document; runtime network sandboxing.

## Dependencies
- M0.5

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M0.8] Architecture Decision Lock

- Phase: Phase 0 Tickets (Validation and Guardrails)
- Labels: hydra, phase-0, area-core, type-chore
- Estimate: S
- Dependencies: none

### Issue Body (Markdown)

```md
## Problem
Two architecture decisions (process model and storage model) were deferred as open questions but affect implementation choices in Phase 1 and Phase 3.

## Scope
Document locked decisions for process model (short-lived CLI, embedded GUI) and storage model (JSONL source of truth, SQLite derived index from Phase 3) in architecture.md.

## Acceptance Criteria
- [ ] ADR entries 6 and 7 are present in research/architecture.md.
- [ ] Open questions section is updated to reflect resolved status.
- [ ] No implementation is blocked by unresolved architecture questions.

## Out of Scope
implementing the SQLite index (Phase 3).

## Dependencies
- none

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


---

## Coverage Check

Phase 0 issues in this file: **8** (M0.1, M0.2, M0.3, M0.4, M0.5, M0.6, M0.7, M0.8)
Phase 0 tickets in checklist: **8**
Coverage: **complete**
