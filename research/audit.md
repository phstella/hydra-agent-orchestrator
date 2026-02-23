# Research Audit

Last updated: 2026-02-23

## 1. Purpose

This file records the quality audit of the research package and the concrete improvements made in this revision.

## 2. What Was Audited

- `research.md`
- `research/architecture.md`
- `research/agent-adapters.md`
- `research/scoring-engine.md`
- `research/collaboration-workflows.md`
- `research/tech-stack.md`
- `research/competitive-analysis.md`
- `research/roadmap.md`
- `research/implementation-checklist.md`
- `research/github-issues.md`
- `research/sprint-1-cut.md`
- `research/issues/README.md`
- `research/issues/phase-0.md`
- `research/issues/phase-1.md`
- `research/issues/phase-2.md`
- `research/issues/phase-3.md`
- `research/issues/phase-4.md`
- `research/issues/phase-5.md`

## 3. Key Gaps Found Before Revision

1. Some adapter details relied on assumptions that can drift quickly (especially Codex/Cursor).
2. Competitive analysis mixed strong claims with weak evidence labels.
3. Cross-platform behavior differences (Linux vs Windows) were not consistently documented.
4. Workflow and scoring docs had strong concepts but lacked explicit artifact contracts and policy gates.
5. Top-level `research.md` did not act as a navigable research index.

## 4. Improvements Applied (Initial Revision)

1. Added source-backed adapter sections with confidence annotations.
2. Added explicit architecture constraints, failure modes, and safety boundaries.
3. Added artifact-driven workflow model and node contracts.
4. Expanded scoring with baseline normalization, anti-gaming controls, and gate logic.
5. Rebuilt roadmap with phase gates, metrics, and risk register.
6. Reframed tech stack as decision record with tradeoff table.
7. Rewrote top-level `research.md` as index + scope anchor.
8. Added issue-ready milestone checklist with ticket templates and acceptance criteria.
9. Locked launch adapter tiering to Claude/Codex (Tier-1) and Cursor experimental opt-in.
10. Generated copy-paste GitHub issue bodies for all milestones (`M0.1` to `M5.5`).
11. Produced dependency-safe sprint-1 cut for the first 8 tickets.
12. Split the issue pack into per-phase files for faster triage.

## 5. Improvements Applied (Review Revision, 2026-02-22)

Triggered by external review (`research-review-2026-02-23.md`) that identified high-priority gaps in security, cost governance, architecture decision locking, and issue body quality.

### High-priority gaps addressed

1. **Architecture decisions locked.** Resolved all three open architecture questions in `research/architecture.md`:
   - ADR 6: Short-lived process model for v1 (no daemon).
   - ADR 7: JSONL source of truth, SQLite derived index from Phase 3.
   - Remote repo orchestration confirmed out of scope for v1.

2. **Security baseline added as milestone.** Added `M0.7` (Security Baseline Implementation) to Phase 0 covering secret redaction, worktree sandbox enforcement, and unsafe-mode guardrails. Added to sprint-1 cut.

3. **Cost governance promoted to Phase 2.** Added `M2.11` (Cost and Budget Engine) covering token usage capture, cost aggregation, and budget stop conditions. Resolved open roadmap question about cost tracking timing.

4. **Observability contract added.** Added `M2.12` (Observability Contract) covering versioned event schema, run health metrics, and stable artifact format.

5. **Schema migration strategy added.** Added `M5.6` (Artifact and Schema Migration Strategy) covering versioned manifests, migration tooling, and forward/backward compatibility tests.

### Medium-priority gaps addressed

6. **Issue body quality improved.** Replaced generic "Implement milestone M#.# to advance Hydra roadmap execution" boilerplate across all Phase 1-5 issue bodies with specific Problem, Scope, and Out of Scope descriptions. Updated all per-phase issue packs and github-issues.md.

7. **Dependency notation disambiguated.** Changed `M2.2 to M2.8` and `M5.1 to M5.4` to explicit enumerated lists in implementation-checklist.md, github-issues.md, and phase issue files.

8. **NFR thresholds added.** Replaced metric-name-only engineering and product metrics in roadmap with concrete v1 targets (e.g., run success rate >= 95%, orchestration overhead < 5s, merge conflict detection 100%).

9. **Sprint-1 cut expanded.** Added `M0.7` and `M0.8` to sprint-1 (now 10 tickets). Added Lane C (architecture governance) to parallelization plan. Added new risks and exit criteria.

10. **Risk register expanded.** Added secret leakage, uncontrolled API cost, schema drift, and competitor scoring risks to roadmap risk register.

### Total milestone count

- Previous: 42 milestones (`M0.1` to `M5.5`)
- Current: 47 milestones (`M0.1` to `M5.6`, including `M0.7`, `M0.8`, `M2.11`, `M2.12`, `M5.6`)

## 6. Remaining Uncertainties

1. Cursor CLI command/flag stability across versions still needs runtime probe validation in implementation.
2. PTY behavior parity on Windows must be tested with real workload before final UI assumptions.
3. Market differentiation should be re-validated each release cycle (space is changing quickly).
4. CLI command tree design does not have a dedicated document yet; commands are documented across architecture, roadmap, and checklist files.
5. Full `hydra.toml` configuration schema reference is not centralized (snippets exist in scoring-engine.md and tech-stack.md).
6. API key discovery and validation strategy is not explicitly documented (covered partially by M0.7 security baseline).

## 7. Recommended Ongoing Process

1. Add a monthly research refresh cadence.
2. Keep adapter capability fixtures versioned in-repo.
3. Require source links for any new competitor or CLI capability claim.
4. Monitor competitor scoring features bi-weekly during active development.
5. Regenerate issue packs from `implementation-checklist.md` after any milestone changes to avoid drift.

## 7. Sources Used in This Audit Cycle

- https://developers.openai.com/codex/cli
- https://github.com/openai/codex
- https://docs.anthropic.com/en/docs/claude-code/overview
- https://docs.anthropic.com/en/docs/claude-code/settings
- https://docs.anthropic.com/fr/docs/claude-code/cli-reference
- https://docs.cursor.com/en/cli/headless
- https://docs.cursor.com/cli/reference/parameters
- https://docs.cursor.com/en/cli/reference/output-format
- https://github.com/smtg-ai/claude-squad
- https://github.com/johannesjo/parallel-code
- https://github.com/coder/mux
- https://github.com/manaflow-ai/cmux
