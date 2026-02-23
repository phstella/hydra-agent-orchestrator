# Product Roadmap

Last updated: 2026-02-23

## 1. Planning Assumptions

- Team assumption: solo developer or very small team.
- Priority: Linux-first reliability, then Windows parity.
- Delivery style: incremental vertical slices with working CLI at every phase.

## 2. Definition of Done (Global)

A phase is complete when:
1. Exit criteria are met.
2. Automated tests pass for new scope.
3. Known risks are documented with mitigation or accepted debt.

## 3. Phase 0 (1 week): Validation and Guardrails

Goal:
- lock adapter assumptions
- verify local run architecture on Linux

Deliverables:
- adapter probe command for Claude/Codex/Cursor
- run artifact directory convention
- baseline `hydra doctor` command

Exit criteria:
- `hydra doctor` reports adapter readiness and repo prerequisites.

## 4. Phase 1 (2-3 weeks): Core Orchestrator + Single Agent

Goal:
- stable end-to-end loop for one agent in isolated worktree

Deliverables:
- `hydra-core` crate scaffold
- config parser (`hydra.toml`)
- worktree lifecycle service
- single adapter execution (start with Claude or Codex)
- streamed events in CLI

Exit criteria:
- run starts from any valid git repo
- worktree isolation works
- cleanup is deterministic on success/failure/interrupt

## 5. Phase 2 (3-4 weeks): Multi-Agent Race + Scoring

Goal:
- concurrent runs and objective ranking

Deliverables:
- multi-adapter registry
- parallel process supervisor
- scoring engine v1 (build/tests/lint/diff/speed)
- mergeable flag and ranking output
- CLI merge command with dry-run

Exit criteria:
- 2-3 agents run concurrently without collisions
- score output includes breakdown and artifacts
- merge dry-run and real merge both tested

## 6. Phase 3 (3-4 weeks): GUI Alpha (Tauri)

Goal:
- visual monitoring and result review

Deliverables:
- task launch UI
- per-agent live output panels
- score dashboard
- diff viewer
- merge action panel

Exit criteria:
- Linux GUI can start and monitor multi-agent race
- results are equivalent to CLI data

## 7. Phase 4 (2-3 weeks): Collaboration Workflows

Goal:
- move beyond race mode into structured cooperation

Deliverables:
- builder/reviewer/refiner preset
- specialization preset
- iterative refinement preset
- workflow artifact timeline in CLI and GUI

Exit criteria:
- each preset has one golden integration test
- workflow failures degrade gracefully with clear status

## 8. Phase 5 (2 weeks): Windows Parity + Hardening

Goal:
- stabilize Windows runtime behavior and release readiness

Deliverables:
- ConPTY validation
- path/permission edge-case fixes
- crash recovery and artifact integrity checks
- packaging and release automation

Exit criteria:
- parity acceptance suite passes on Linux and Windows

## 9. Milestone Risk Register

| Risk | Phase | Impact | Mitigation |
|---|---|---|---|
| Adapter flag drift | 0-5 | High | runtime capability probes + versioned fixtures |
| PTY instability on Windows | 3-5 | High | fallback raw stream mode and adapter-specific toggles |
| Scoring false positives | 2-4 | Medium | baseline normalization + per-repo profiles |
| Merge automation distrust | 2-5 | Medium | default dry-run and explicit human gate |
| Scope creep from workflow editor | 4 | Medium | ship presets first, postpone graph editor |

## 10. Metrics by Phase

### Engineering metrics

- run success rate
- median orchestration overhead (excluding agent runtime)
- adapter parse error rate
- merge conflict detection accuracy

### Product metrics

- time-to-first-ranked-result
- percent of runs ending in mergeable candidate
- user override rate (when user picks non-top score winner)

## 11. Suggested Backlog Order (Immediate)

1. `hydra doctor` and adapter probes
2. worktree service with strong cleanup semantics
3. one robust adapter end-to-end in CLI
4. artifact persistence and replay primitives
5. scoring engine baseline capture

## 12. Release Gates

Pre-release checklist:
1. Linux and Windows smoke tests green.
2. Adapter probes documented and version-stamped.
3. Scoring outputs reproducible from saved artifacts.
4. No known data-loss paths in cleanup/merge logic.

## 13. Open Roadmap Questions

1. Should Windows parity happen before GUI alpha, if early users are mixed-OS teams?
2. Should cost tracking move earlier (Phase 2) since it affects run-policy decisions?
3. Should we publish plugin API in v1 or keep adapters internal until stabilized?
