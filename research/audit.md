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

## 3. Key Gaps Found Before Revision

1. Some adapter details relied on assumptions that can drift quickly (especially Codex/Cursor).
2. Competitive analysis mixed strong claims with weak evidence labels.
3. Cross-platform behavior differences (Linux vs Windows) were not consistently documented.
4. Workflow and scoring docs had strong concepts but lacked explicit artifact contracts and policy gates.
5. Top-level `research.md` did not act as a navigable research index.

## 4. Improvements Applied

1. Added source-backed adapter sections with confidence annotations.
2. Added explicit architecture constraints, failure modes, and safety boundaries.
3. Added artifact-driven workflow model and node contracts.
4. Expanded scoring with baseline normalization, anti-gaming controls, and gate logic.
5. Rebuilt roadmap with phase gates, metrics, and risk register.
6. Reframed tech stack as decision record with tradeoff table.
7. Rewrote top-level `research.md` as index + scope anchor.

## 5. Remaining Uncertainties

1. Cursor CLI command/flag stability across versions still needs runtime probe validation in implementation.
2. PTY behavior parity on Windows must be tested with real workload before final UI assumptions.
3. Market differentiation should be re-validated each release cycle (space is changing quickly).

## 6. Recommended Ongoing Process

1. Add a monthly research refresh cadence.
2. Keep adapter capability fixtures versioned in-repo.
3. Require source links for any new competitor or CLI capability claim.

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
