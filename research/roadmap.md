# Product Roadmap

Last updated: 2026-02-23

## 1. Planning Assumptions

- Team assumption: solo developer or very small team.
- Priority: Linux-first reliability, then Windows parity.
- Delivery style: incremental vertical slices with working CLI at every phase.
- Launch adapter tiering is fixed: Tier-1 = `claude`, `codex`; `cursor-agent` remains experimental.

## 2. Adapter Tier Policy

Tier definitions:
- Tier-1: enabled by default, release-blocking reliability requirements.
- Experimental: disabled by default, explicit opt-in, non-blocking for release.

Policy rules:
1. Tier-1 adapters require passing probe + conformance test suite.
2. Experimental adapters cannot become default without explicit milestone decision.
3. Experimental adapter failures must not impact Tier-1 run reliability.

## 3. Definition of Done (Global)

A phase is complete when:
1. Exit criteria are met.
2. Automated tests pass for new scope.
3. Known risks are documented with mitigation or accepted debt.

## 4. Phase 0 (1 week): Validation and Guardrails

Goal:
- lock adapter assumptions
- verify local run architecture on Linux

Deliverables:
- adapter probe command for Claude/Codex (Tier-1)
- experimental probe path for Cursor
- run artifact directory convention
- baseline `hydra doctor` command
- security baseline: secret redaction, worktree sandbox enforcement, unsafe-mode guardrails
- architecture decision lock: process model and storage model documented as ADRs

Exit criteria:
- `hydra doctor` reports adapter readiness and repo prerequisites.
- Secret redaction tests pass with known fixture patterns.
- ADR entries for process model and storage model are finalized.

## 5. Phase 1 (2-3 weeks): Core Orchestrator + Single Agent

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

## 6. Phase 2 (3-4 weeks): Multi-Agent Race + Scoring

Goal:
- concurrent runs and objective ranking

Deliverables:
- multi-adapter registry
- parallel process supervisor
- scoring engine v1 (build/tests/lint/diff/speed)
- mergeable flag and ranking output
- CLI merge command with dry-run
- experimental adapter opt-in gate (`--allow-experimental-adapters`)
- cost and budget engine: token usage capture, cost aggregation, budget stop conditions
- observability contract: versioned event schema, run health metrics, stable artifact format

Exit criteria:
- 2+ Tier-1 agents run concurrently without collisions
- score output includes breakdown and artifacts
- merge dry-run and real merge both tested
- token usage captured and cost summary displayed for adapters that emit usage data
- event schema versioned and manifest includes `schema_version` field

## 7. Phase 3 (3-4 weeks): GUI Alpha (Tauri)

Goal:
- visual monitoring and result review

Deliverables:
- task launch UI
- per-agent live output panels
- score dashboard
- diff viewer
- merge action panel
- explicit experimental-adapter warnings and opt-in UX

Exit criteria:
- Linux GUI can start and monitor multi-agent race
- results are equivalent to CLI data

## 8. Phase 4 (2-3 weeks): Collaboration Workflows

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

## 9. Phase 5 (2 weeks): Windows Parity + Hardening

Goal:
- stabilize Windows runtime behavior and release readiness

Deliverables:
- ConPTY validation
- path/permission edge-case fixes
- crash recovery and artifact integrity checks
- packaging and release automation
- artifact and schema migration strategy with forward/backward compatibility tests

Exit criteria:
- parity acceptance suite passes on Linux and Windows
- schema migration tool upgrades v1 artifacts to current format
- forward/backward compatibility tests pass

## 10. Milestone Risk Register

| Risk | Phase | Impact | Mitigation |
|---|---|---|---|
| Adapter flag drift | 0-5 | High | runtime capability probes + versioned fixtures |
| PTY instability on Windows | 3-5 | High | fallback raw stream mode and adapter-specific toggles |
| Scoring false positives | 2-4 | Medium | baseline normalization + per-repo profiles |
| Merge automation distrust | 2-5 | Medium | default dry-run and explicit human gate |
| Scope creep from workflow editor | 4 | Medium | ship presets first, postpone graph editor |
| Secret leakage in logs/artifacts | 0-5 | High | secret redaction rules + log scrubbing tests (M0.7) |
| Uncontrolled API cost in race mode | 2-5 | High | budget stop conditions + cost visibility in output (M2.11) |
| Schema drift breaking run history | 3-5 | Medium | versioned event schema + migration tool (M2.12, M5.6) |
| Competitor adds scoring feature | 2-3 | High | prioritize Phases 0-2 as single push to market |

## 11. Metrics by Phase

### Engineering metrics

| Metric | Target (v1) |
|---|---|
| Run success rate (no Hydra-caused failures) | >= 95% |
| Median orchestration overhead (excluding agent runtime) | < 5 seconds |
| Adapter parse error rate | < 1% of streamed events |
| Merge conflict detection accuracy | 100% (no silent conflicts) |
| Worktree cleanup reliability (no orphans) | 100% |

### Product metrics

| Metric | Target (v1) |
|---|---|
| Time-to-first-ranked-result (after agents finish) | < 30 seconds |
| Percent of runs ending in at least one mergeable candidate | >= 70% |
| User override rate (picks non-top score winner) | tracked, no target yet |
| Cost visibility coverage (runs with cost data when adapter supports it) | 100% |

## 12. Suggested Backlog Order (Immediate)

1. `hydra doctor` and adapter probes
2. worktree service with strong cleanup semantics
3. one robust adapter end-to-end in CLI
4. artifact persistence and replay primitives
5. scoring engine baseline capture

## 13. Release Gates

Pre-release checklist:
1. Linux and Windows smoke tests green.
2. Adapter probes documented and version-stamped.
3. Scoring outputs reproducible from saved artifacts.
4. No known data-loss paths in cleanup/merge logic.

## 14. Issue Tracking Note

- Milestone-to-ticket breakdown is maintained in `research/implementation-checklist.md`.
- Use milestone IDs (`M0.1`, `M1.1`, etc.) as canonical issue prefixes.

## 15. Resolved Roadmap Questions

1. ~~Should cost tracking move earlier (Phase 2) since it affects run-policy decisions?~~ **Decided: Yes.** Cost and budget engine added to Phase 2 as M2.11. Race mode multiplies API cost; users need visibility before workflows add further complexity.

## 16. Open Roadmap Questions

1. Should Windows parity happen before GUI alpha, if early users are mixed-OS teams?
2. Should we publish plugin API in v1 or keep adapters internal until stabilized?
