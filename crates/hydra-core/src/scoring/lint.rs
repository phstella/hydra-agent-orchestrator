use serde::{Deserialize, Serialize};

/// Detailed lint scoring breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintScore {
    pub score: f64,
    pub new_errors: u32,
    pub new_warnings: u32,
    pub resolved: u32,
    pub agent_errors: u32,
    pub agent_warnings: u32,
    pub baseline_errors: u32,
    pub baseline_warnings: u32,
}

/// Input parameters for lint scoring.
pub struct LintInput {
    pub agent_errors: u32,
    pub agent_warnings: u32,
    pub baseline_errors: u32,
    pub baseline_warnings: u32,
}

/// Score lint results against baseline.
///
/// Formula:
/// ```text
/// new_errors = max(0, A_errors - B_errors)
/// new_warnings = max(0, A_warnings - B_warnings)
/// resolved = max(0, (B_errors + B_warnings) - (A_errors + A_warnings))
/// score = clamp(100 - (new_errors * 12) - (new_warnings * 2) + (resolved * 1), 0, 100)
/// ```
pub fn score_lint(input: &LintInput) -> LintScore {
    let new_errors = input.agent_errors.saturating_sub(input.baseline_errors);
    let new_warnings = input.agent_warnings.saturating_sub(input.baseline_warnings);

    let baseline_total = input.baseline_errors + input.baseline_warnings;
    let agent_total = input.agent_errors + input.agent_warnings;
    let resolved = baseline_total.saturating_sub(agent_total);

    let raw = 100.0 - (new_errors as f64 * 12.0) - (new_warnings as f64 * 2.0) + (resolved as f64);
    let score = raw.clamp(0.0, 100.0);

    LintScore {
        score,
        new_errors,
        new_warnings,
        resolved,
        agent_errors: input.agent_errors,
        agent_warnings: input.agent_warnings,
        baseline_errors: input.baseline_errors,
        baseline_warnings: input.baseline_warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_slate() {
        let s = score_lint(&LintInput {
            agent_errors: 0,
            agent_warnings: 0,
            baseline_errors: 0,
            baseline_warnings: 0,
        });
        assert_eq!(s.score, 100.0);
    }

    #[test]
    fn new_errors_penalized() {
        // 2 new errors: 100 - 24 = 76
        let s = score_lint(&LintInput {
            agent_errors: 2,
            agent_warnings: 0,
            baseline_errors: 0,
            baseline_warnings: 0,
        });
        assert!((s.score - 76.0).abs() < 0.01);
        assert_eq!(s.new_errors, 2);
    }

    #[test]
    fn new_warnings_penalized() {
        // 5 new warnings: 100 - 10 = 90
        let s = score_lint(&LintInput {
            agent_errors: 0,
            agent_warnings: 5,
            baseline_errors: 0,
            baseline_warnings: 0,
        });
        assert!((s.score - 90.0).abs() < 0.01);
        assert_eq!(s.new_warnings, 5);
    }

    #[test]
    fn resolved_issues_bonus() {
        // Baseline: 3 errors, 5 warnings. Agent: 0 errors, 0 warnings.
        // resolved = 8, score = 100 + 8 = 108 -> clamped to 100
        let s = score_lint(&LintInput {
            agent_errors: 0,
            agent_warnings: 0,
            baseline_errors: 3,
            baseline_warnings: 5,
        });
        assert_eq!(s.score, 100.0);
        assert_eq!(s.resolved, 8);
    }

    #[test]
    fn mixed_new_and_resolved() {
        // Baseline: 2e 4w (total 6). Agent: 1e 2w (total 3).
        // new_errors = 0, new_warnings = 0, resolved = 3
        // score = 100 + 3 = 103 -> 100
        let s = score_lint(&LintInput {
            agent_errors: 1,
            agent_warnings: 2,
            baseline_errors: 2,
            baseline_warnings: 4,
        });
        assert_eq!(s.score, 100.0);
        assert_eq!(s.resolved, 3);
    }

    #[test]
    fn clamped_at_zero() {
        // 10 new errors: 100 - 120 = -20 -> 0
        let s = score_lint(&LintInput {
            agent_errors: 10,
            agent_warnings: 0,
            baseline_errors: 0,
            baseline_warnings: 0,
        });
        assert_eq!(s.score, 0.0);
    }

    #[test]
    fn pre_existing_issues_not_penalized() {
        // Baseline already has 5 errors. Agent still has 5 errors.
        // new_errors = 0, score = 100
        let s = score_lint(&LintInput {
            agent_errors: 5,
            agent_warnings: 0,
            baseline_errors: 5,
            baseline_warnings: 0,
        });
        assert_eq!(s.score, 100.0);
        assert_eq!(s.new_errors, 0);
    }

    #[test]
    fn partial_resolution() {
        // Baseline: 5e 10w. Agent: 3e 8w.
        // new_errors = 0, new_warnings = 0, resolved = (15 - 11) = 4
        // score = 100 + 4 = 104 -> 100
        let s = score_lint(&LintInput {
            agent_errors: 3,
            agent_warnings: 8,
            baseline_errors: 5,
            baseline_warnings: 10,
        });
        assert_eq!(s.score, 100.0);
        assert_eq!(s.resolved, 4);
    }

    #[test]
    fn worsened_with_resolved() {
        // Baseline: 0e 5w. Agent: 1e 2w.
        // new_errors = 1, new_warnings = 0, resolved = 5 - 3 = 2
        // score = 100 - 12 + 2 = 90
        let s = score_lint(&LintInput {
            agent_errors: 1,
            agent_warnings: 2,
            baseline_errors: 0,
            baseline_warnings: 5,
        });
        assert!((s.score - 90.0).abs() < 0.01);
    }
}
