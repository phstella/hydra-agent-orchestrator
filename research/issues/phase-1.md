# Phase 1 Tickets (Core Orchestrator + Single Agent) Issue Bodies

Last updated: 2026-02-23

Generated from `research/github-issues.md`.

Global label prefix: `hydra`

## [M1.1] Core Workspace Scaffold

- Phase: Phase 1 Tickets (Core Orchestrator + Single Agent)
- Labels: hydra, phase-1, area-core, type-feature
- Estimate: S
- Dependencies: none

### Issue Body (Markdown)

```md
## Problem
Implement milestone M1.1 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.1.

## Acceptance Criteria
- [ ] Workspace builds with hydra-core and hydra-cli crates.
- [ ] Logging and error crates wired consistently.
- [ ] CI compiles on Linux and Windows.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.2 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.2.

## Acceptance Criteria
- [ ] hydra.toml parses with schema validation.
- [ ] Missing optional fields get deterministic defaults.
- [ ] Invalid config returns actionable error messages.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.3 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.3.

## Acceptance Criteria
- [ ] Create/list/remove worktree operations are implemented.
- [ ] Interrupt-safe cleanup path exists.
- [ ] Windows path handling tests pass.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.4 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.4.

## Acceptance Criteria
- [ ] Supports start, stream, timeout, cancel.
- [ ] Bounded output buffering prevents memory blowups.
- [ ] Emits normalized lifecycle events.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.5 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.5.

## Acceptance Criteria
- [ ] claude runs in isolated worktree.
- [ ] Stream parser maps key events to normalized schema.
- [ ] Timeout and cancellation are covered by tests.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.6 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.6.

## Acceptance Criteria
- [ ] codex exec works in isolated worktree.
- [ ] JSON stream parser maps events and usage data.
- [ ] Unsupported flag fallback logic is tested.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.7 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.7.

## Acceptance Criteria
- [ ] hydra race --agents claude completes end-to-end.
- [ ] Run summary includes branch and artifact path.
- [ ] Non-zero exit codes on fatal failures.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

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
Implement milestone M1.8 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M1.8.

## Acceptance Criteria
- [ ] Ctrl+C cleanup test passes.
- [ ] Partial failure leaves usable artifacts.
- [ ] No orphan worktrees after test run.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

## Dependencies
- M1.3, M1.4, M1.7

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


