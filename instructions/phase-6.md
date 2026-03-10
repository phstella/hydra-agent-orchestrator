# Phase 6: Windows Parity + Release Hardening

**Goal**: Stabilize Windows runtime behavior and achieve release readiness.

**Duration estimate**: 2 weeks

## Milestones

| ID | Title | Estimate | Dependencies |
|----|-------|----------|--------------|
| M6.1 | ConPTY and Process Control Validation | M | M4.6 |
| M6.2 | Path and Filesystem Edge Cases | M | M1.3 |
| M6.3 | Crash Recovery and Resume Metadata | M | M2.10 |
| M6.4 | Packaging and Release Automation | M | M6.1, M6.2 |
| M6.5 | Release Candidate Acceptance Suite | M | M6.1, M6.2, M6.3, M6.4 |
| M6.6 | Artifact and Schema Migration Strategy | M | M2.12, M6.3 |

## Parallelization

Three items can start in parallel (given their Phase 2/4 dependencies are met):

- M6.1 (ConPTY validation)
- M6.2 (path edge cases)
- M6.3 (crash recovery)

Then: M6.4 follows M6.1+M6.2, M6.5 follows all, M6.6 follows M6.3+M2.12.

## What to Build

- **ConPTY validation** (M6.1): Test PTY and fallback stream paths on Windows.
  Verify cancel/timeout behavior. See `docs/architecture.md` section 7 for
  cross-platform notes.

- **Path edge cases** (M6.2): Long paths, spaces, Unicode, file locking on Windows.

- **Crash recovery** (M6.3): Recovery metadata in manifests, stale state cleanup tool.

- **Release pipeline** (M6.4): CI/CD for Linux + Windows, versioned binaries,
  checksums, release notes.

- **Acceptance suite** (M6.5): Full product surface validation before RC cut.

- **Schema migration** (M6.6): Versioned manifests, migration tool,
  forward/backward compatibility tests.

## Exit Criteria

1. Parity acceptance suite passes on Linux and Windows.
2. Schema migration tool upgrades v1 artifacts to current format.
3. Forward/backward compatibility tests pass.

## References

- Acceptance criteria: `planning/implementation-checklist.md` section 9
- Issue bodies: `planning/issues/phase-6.md`
- Risk register: `planning/roadmap.md` section 11
