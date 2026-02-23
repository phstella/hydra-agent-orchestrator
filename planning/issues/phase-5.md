# Phase 5 Tickets (Windows Parity and Release Hardening) Issue Bodies

Last updated: 2026-02-22

Generated from `planning/implementation-checklist.md`.

Global label prefix: `hydra`

## [M5.1] ConPTY and Process Control Validation

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-core, type-test
- Estimate: M
- Dependencies: M3.7

### Issue Body (Markdown)

```md
## Problem
PTY behavior on Windows (ConPTY) differs from Unix and has not been validated under real workloads. Process termination semantics, orphan process prevention, and ANSI rendering may behave differently than on Linux.

## Scope
Validate PTY and fallback stream paths on Windows. Test cancel/timeout behavior with real agent CLIs. Verify no orphan processes remain after cancellation. Document any Windows-specific behavior differences.

## Acceptance Criteria
- [ ] PTY and fallback stream paths both tested.
- [ ] Cancel/timeout behavior verified on Windows.
- [ ] No orphan process remains after cancellation.

## Out of Scope
macOS PTY testing; custom terminal emulator support.

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
Windows has distinct path length limits (260 chars default), separator conventions, and file locking behavior that can cause failures in worktree creation, artifact writes, and cleanup operations.

## Scope
Test and fix long path handling, paths with spaces and Unicode characters, and artifact writes under locked file conditions. Ensure all filesystem operations use OS-safe path construction.

## Acceptance Criteria
- [ ] Long path handling tests pass.
- [ ] Space/Unicode path cases are covered.
- [ ] Artifact writes are robust under locked files.

## Out of Scope
Network drive support; junction point handling.

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
Interrupted runs (system crash, power loss, OOM kill) can leave the `.hydra/` directory in an inconsistent state with stale worktrees, partial artifacts, and incomplete manifests. Users need tools to inspect and recover from these states.

## Scope
Add recovery metadata to run manifests. Implement a cleanup tool that detects and reconciles stale state (orphaned worktrees, incomplete runs). Ensure interrupted runs are inspectable post-crash.

## Acceptance Criteria
- [ ] Interrupted runs can be inspected post-crash.
- [ ] Cleanup tool can reconcile stale state.
- [ ] Recovery metadata is included in run manifest.

## Out of Scope
Automatic run resumption; partial result scoring.

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
There is no automated pipeline for producing versioned release artifacts. Manual packaging is error-prone and blocks release cadence.

## Scope
Set up CI/CD release pipeline for Linux and Windows. Produce versioned binaries with checksums. Generate release notes from milestone labels. Define version numbering scheme.

## Acceptance Criteria
- [ ] Versioned builds produced for Linux and Windows.
- [ ] Release artifacts include checksums.
- [ ] Release notes generated from milestone labels.

## Out of Scope
macOS builds; Homebrew formula; auto-update mechanism.

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
- Dependencies: M5.1, M5.2, M5.3, M5.4

### Issue Body (Markdown)

```md
## Problem
There is no comprehensive acceptance test that validates the full product surface before release. Without a release gate, regressions in core flows could ship to users.

## Scope
Write an acceptance test suite covering Tier-1 race and merge paths on Linux and Windows. Verify experimental adapter behavior remains opt-in. Confirm no P0 bugs are open at RC cut.

## Acceptance Criteria
- [ ] Tier-1 race and merge path pass on Linux/Windows.
- [ ] Experimental adapter behavior remains opt-in.
- [ ] No P0 bugs open at RC cut.

## Out of Scope
Performance regression tests; security audit.

## Dependencies
- M5.1, M5.2, M5.3, M5.4

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## [M5.6] Artifact and Schema Migration Strategy

- Phase: Phase 5 Tickets (Windows Parity and Release Hardening)
- Labels: hydra, phase-5, area-core, type-feature
- Estimate: M
- Dependencies: M2.12, M5.3

### Issue Body (Markdown)

```md
## Problem
As Hydra evolves, the artifact format (manifest.json, events.jsonl, score output) and configuration schema (hydra.toml) will change. Without a migration strategy, users upgrading Hydra may encounter broken run history, unreadable artifacts, or invalid configuration files.

## Scope
Implement versioned manifest and event schema with forward-compatibility rules. Add a migration tool that upgrades older artifacts/configs to current schema. Write forward/backward compatibility tests for at least one schema transition. Document upgrade path in release notes.

## Acceptance Criteria
- [ ] Schema version is checked on artifact read and config parse.
- [ ] Migration tool upgrades v1 artifacts/configs to current format.
- [ ] Forward and backward compatibility tests pass for at least one schema transition.
- [ ] Upgrade path is documented.

## Out of Scope
Automatic background migration; multi-version concurrent support.

## Dependencies
- M2.12, M5.3

## Notes
- Tier-1 launch adapters are claude and codex.
- Experimental adapters require explicit opt-in.
```


## Coverage Check

- Total issues generated: 6
- Expected range: `M5.1` through `M5.6`
