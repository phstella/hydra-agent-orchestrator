use super::DimensionScore;
use crate::scoring::baseline::TestResult;

/// Score the test dimension using the regression-aware formula.
///
/// Formula (from docs/scoring-engine.md section 5.2):
///   pass_rate = A_pass / A_total
///   regression = max(0, B_pass - A_pass)
///   reg_penalty = if B_pass == 0 then 0 else (regression / B_pass) * 60
///   new_test_bonus = if new_tests > 0 then min(10, new_tests * 0.5) else 0
///   score = clamp(pass_rate * 100 - reg_penalty + new_test_bonus, 0, 100)
pub fn score_tests(baseline: Option<&TestResult>, agent_result: &TestResult) -> DimensionScore {
    let a_pass = agent_result.passed as f64;
    let a_total = agent_result.total as f64;

    let pass_rate = if a_total == 0.0 {
        0.0
    } else {
        a_pass / a_total
    };

    let (b_pass, b_total) = baseline
        .map(|b| (b.passed as f64, b.total as f64))
        .unwrap_or((0.0, 0.0));

    let regression = (b_pass - a_pass).max(0.0);
    let reg_penalty = if b_pass == 0.0 {
        0.0
    } else {
        (regression / b_pass) * 60.0
    };

    let new_tests = (a_total - b_total).max(0.0);
    let new_test_bonus = if new_tests > 0.0 {
        (new_tests * 0.5).min(10.0)
    } else {
        0.0
    };

    let raw_score = pass_rate * 100.0 - reg_penalty + new_test_bonus;
    let score = raw_score.clamp(0.0, 100.0);

    let test_drop = if b_total > 0.0 {
        Some(a_total < b_total * 0.8)
    } else {
        None
    };

    DimensionScore {
        name: "tests".to_string(),
        score,
        evidence: serde_json::json!({
            "agent_passed": agent_result.passed,
            "agent_failed": agent_result.failed,
            "agent_total": agent_result.total,
            "baseline_passed": baseline.map(|b| b.passed),
            "baseline_total": baseline.map(|b| b.total),
            "pass_rate": pass_rate,
            "regression": regression as u32,
            "reg_penalty": reg_penalty,
            "new_test_bonus": new_test_bonus,
            "test_drop_detected": test_drop,
        }),
    }
}

#[cfg(test)]
mod test_scoring {
    use super::*;
    use crate::scoring::baseline::{CommandResult, TestResult};

    fn make_test_result(passed: u32, failed: u32) -> TestResult {
        TestResult {
            command_result: CommandResult {
                command: "cargo test".to_string(),
                success: failed == 0,
                exit_code: if failed == 0 { 0 } else { 1 },
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 100,
            },
            passed,
            failed,
            total: passed + failed,
        }
    }

    #[test]
    fn perfect_score_no_baseline() {
        let agent = make_test_result(10, 0);
        let score = score_tests(None, &agent);
        assert!((score.score - 100.0).abs() < 0.01);
    }

    #[test]
    fn perfect_score_matching_baseline() {
        let baseline = make_test_result(10, 0);
        let agent = make_test_result(10, 0);
        let score = score_tests(Some(&baseline), &agent);
        assert!((score.score - 100.0).abs() < 0.01);
    }

    #[test]
    fn regression_penalty_applied() {
        let baseline = make_test_result(10, 0);
        let agent = make_test_result(8, 2);
        let score = score_tests(Some(&baseline), &agent);
        // pass_rate = 0.8, reg_penalty = (2/10)*60 = 12, score = 80 - 12 = 68
        assert!((score.score - 68.0).abs() < 0.01);
    }

    #[test]
    fn new_test_bonus_applied() {
        let baseline = make_test_result(10, 0);
        let agent = make_test_result(14, 0);
        let score = score_tests(Some(&baseline), &agent);
        // pass_rate=1.0, no regression, new_tests=4, bonus=min(10,4*0.5)=2.0
        // score = 100 + 2 = 102 -> clamped to 100
        assert!((score.score - 100.0).abs() < 0.01);
    }

    #[test]
    fn new_test_bonus_capped_at_10() {
        let baseline = make_test_result(10, 0);
        let agent = make_test_result(40, 0);
        let score = score_tests(Some(&baseline), &agent);
        // new_tests=30, bonus=min(10, 15)=10
        assert!((score.score - 100.0).abs() < 0.01);
    }

    #[test]
    fn zero_total_scores_zero() {
        let agent = make_test_result(0, 0);
        let score = score_tests(None, &agent);
        assert!((score.score - 0.0).abs() < 0.01);
    }

    #[test]
    fn full_regression_heavily_penalized() {
        let baseline = make_test_result(10, 0);
        let agent = make_test_result(0, 10);
        let score = score_tests(Some(&baseline), &agent);
        // pass_rate=0, reg_penalty=(10/10)*60=60, score = 0-60 = -60 -> clamped to 0
        assert!((score.score - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_drop_detected() {
        let baseline = make_test_result(10, 0);
        let agent = make_test_result(5, 0);
        let score = score_tests(Some(&baseline), &agent);
        assert_eq!(score.evidence["test_drop_detected"], true);
    }

    #[test]
    fn no_test_drop_when_tests_maintained() {
        let baseline = make_test_result(10, 0);
        let agent = make_test_result(9, 0);
        let score = score_tests(Some(&baseline), &agent);
        assert_eq!(score.evidence["test_drop_detected"], false);
    }

    #[test]
    fn zero_baseline_no_regression_penalty() {
        let baseline = make_test_result(0, 0);
        let agent = make_test_result(5, 0);
        let score = score_tests(Some(&baseline), &agent);
        assert!((score.score - 100.0).abs() < 1.0);
    }
}
