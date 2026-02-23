# Phase 5 Tickets (Windows Parity and Release Hardening) Issue Bodies

Last updated: 2026-02-23

Generated from `research/github-issues.md`.

Global label prefix: `hydra`

## [M5.1] ConPTY and Process Control Validation

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-core, type-test
- Estimate: M
- Dependencies: M3.7

### Issue Body (Markdown)

```md
## Problem
Implement milestone M5.1 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M5.1.

## Acceptance Criteria
- [ ] PTY and fallback stream paths both tested.
- [ ] Cancel/timeout behavior verified on Windows.
- [ ] No orphan process remains after cancellation.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

## Dependencies
- M3.7

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M5.2] Path and Filesystem Edge Cases

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-core, type-feature
- Estimate: M
- Dependencies: M1.3

### Issue Body (Markdown)

```md
## Problem
Implement milestone M5.2 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M5.2.

## Acceptance Criteria
- [ ] Long path handling tests pass.
- [ ] Space/Unicode path cases are covered.
- [ ] Artifact writes are robust under locked files.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

## Dependencies
- M1.3

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M5.3] Crash Recovery and Resume Metadata

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-core, type-feature
- Estimate: M
- Dependencies: M2.10

### Issue Body (Markdown)

```md
## Problem
Implement milestone M5.3 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M5.3.

## Acceptance Criteria
- [ ] Interrupted runs can be inspected post-crash.
- [ ] Cleanup tool can reconcile stale state.
- [ ] Recovery metadata is included in run manifest.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

## Dependencies
- M2.10

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M5.4] Packaging and Release Automation

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-release, type-feature
- Estimate: M
- Dependencies: M5.1, M5.2

### Issue Body (Markdown)

```md
## Problem
Implement milestone M5.4 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M5.4.

## Acceptance Criteria
- [ ] Versioned builds produced for Linux and Windows.
- [ ] Release artifacts include checksums.
- [ ] Release notes generated from milestone labels.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

## Dependencies
- M5.1, M5.2

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M5.5] Release Candidate Acceptance Suite

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-test, type-test
- Estimate: M
- Dependencies: M5.1 to M5.4

### Issue Body (Markdown)

```md
## Problem
Implement milestone M5.5 to advance Hydra roadmap execution.

## Scope
Deliver the implementation needed to satisfy the acceptance criteria for M5.5.

## Acceptance Criteria
- [ ] Tier-1 race and merge path pass on Linux/Windows.
- [ ] Experimental adapter behavior remains opt-in.
- [ ] No P0 bugs open at RC cut.

## Out of Scope
Any work not required to satisfy this ticket's acceptance criteria.

## Dependencies
- M5.1 to M5.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```

## Coverage Check

- Total issues generated: 42
- Expected range: `M0.1` through `M5.5`

