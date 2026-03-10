use super::DimensionScore;
use crate::scoring::baseline::LintResult;

/// Score the lint dimension using the delta formula.
///
/// Formula (from docs/scoring-engine.md section 5.3):
///   new_errors = max(0, A_errors - B_errors)
///   new_warnings = max(0, A_warnings - B_warnings)
///   resolved = max(0, (B_errors + B_warnings) - (A_errors + A_warnings))
///   score = clamp(100 - (new_errors * 12) - (new_warnings * 2) + (resolved * 1), 0, 100)
pub fn score_lint(baseline: Option<&LintResult>, agent_result: &LintResult) -> DimensionScore {
    let (b_errors, b_warnings) = baseline
        .map(|b| (b.errors as i64, b.warnings as i64))
        .unwrap_or((0, 0));

    let a_errors = agent_result.errors as i64;
    let a_warnings = agent_result.warnings as i64;

    let new_errors = (a_errors - b_errors).max(0);
    let new_warnings = (a_warnings - b_warnings).max(0);
    let resolved = ((b_errors + b_warnings) - (a_errors + a_warnings)).max(0);

    let raw = 100 - (new_errors * 12) - (new_warnings * 2) + resolved;
    let score = (raw as f64).clamp(0.0, 100.0);

    DimensionScore {
        name: "lint".to_string(),
        score,
        evidence: serde_json::json!({
            "agent_errors": agent_result.errors,
            "agent_warnings": agent_result.warnings,
            "baseline_errors": baseline.map(|b| b.errors),
            "baseline_warnings": baseline.map(|b| b.warnings),
            "new_errors": new_errors,
            "new_warnings": new_warnings,
            "resolved": resolved,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::baseline::{CommandResult, LintResult};

    fn make_lint(errors: u32, warnings: u32) -> LintResult {
        LintResult {
            command_result: CommandResult {
                command: "cargo clippy".to_string(),
                success: errors == 0,
                exit_code: if errors == 0 { 0 } else { 1 },
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 50,
            },
            errors,
            warnings,
        }
    }

    #[test]
    fn clean_lint_scores_100() {
        let agent = make_lint(0, 0);
        let score = score_lint(None, &agent);
        assert!((score.score - 100.0).abs() < 0.01);
    }

    #[test]
    fn new_errors_penalized_heavily() {
        let baseline = make_lint(0, 0);
        let agent = make_lint(3, 0);
        let score = score_lint(Some(&baseline), &agent);
        // 100 - 3*12 = 64
        assert!((score.score - 64.0).abs() < 0.01);
    }

    #[test]
    fn new_warnings_penalized_lightly() {
        let baseline = make_lint(0, 0);
        let agent = make_lint(0, 5);
        let score = score_lint(Some(&baseline), &agent);
        // 100 - 5*2 = 90
        assert!((score.score - 90.0).abs() < 0.01);
    }

    #[test]
    fn resolved_issues_give_bonus() {
        let baseline = make_lint(2, 5);
        let agent = make_lint(0, 0);
        let score = score_lint(Some(&baseline), &agent);
        // 100 + (2+5 - 0-0) = 100 + 7 -> clamped to 100
        assert!((score.score - 100.0).abs() < 0.01);
    }

    #[test]
    fn mixed_new_and_resolved() {
        let baseline = make_lint(2, 5);
        let agent = make_lint(1, 8);
        let score = score_lint(Some(&baseline), &agent);
        // new_errors = max(0, 1-2) = 0
        // new_warnings = max(0, 8-5) = 3
        // resolved = max(0, (2+5)-(1+8)) = max(0, -2) = 0
        // 100 - 0*12 - 3*2 + 0 = 94
        assert!((score.score - 94.0).abs() < 0.01);
    }

    #[test]
    fn score_clamped_at_zero() {
        let baseline = make_lint(0, 0);
        let agent = make_lint(20, 0);
        let score = score_lint(Some(&baseline), &agent);
        // 100 - 20*12 = -140 -> clamped to 0
        assert!((score.score - 0.0).abs() < 0.01);
    }

    #[test]
    fn no_baseline_treats_as_zero() {
        let agent = make_lint(1, 2);
        let score = score_lint(None, &agent);
        // 100 - 1*12 - 2*2 = 84
        assert!((score.score - 84.0).abs() < 0.01);
    }
}
