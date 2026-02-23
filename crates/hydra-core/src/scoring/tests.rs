use serde::{Deserialize, Serialize};

/// Detailed test scoring breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScore {
    pub score: f64,
    pub pass_rate: f64,
    pub regression_penalty: f64,
    pub new_test_bonus: f64,
    pub anti_gaming_penalty: f64,
    pub agent_passed: u32,
    pub agent_failed: u32,
    pub agent_total: u32,
    pub baseline_passed: u32,
    pub baseline_total: u32,
}

/// Input parameters for test scoring.
pub struct TestInput {
    pub agent_passed: u32,
    pub agent_total: u32,
    pub baseline_passed: u32,
    pub baseline_total: u32,
}

/// Score test results against baseline.
///
/// Formula (from scoring-engine.md):
/// ```text
/// pass_rate = if A_total == 0 then 0 else A_pass / A_total
/// regression = max(0, B_pass - A_pass)
/// reg_penalty = if B_pass == 0 then 0 else (regression / B_pass) * 60
/// new_test_bonus = if new_tests > 0 then min(10, new_tests * 0.5) else 0
/// score = clamp((pass_rate * 100) - reg_penalty + new_test_bonus, 0, 100)
/// ```
///
/// Anti-gaming: penalize if test count drops significantly from baseline.
pub fn score_tests(input: &TestInput) -> TestScore {
    let a_pass = input.agent_passed as f64;
    let a_total = input.agent_total as f64;
    let b_pass = input.baseline_passed as f64;
    let b_total = input.baseline_total as f64;

    // Pass rate
    let pass_rate = if a_total == 0.0 {
        0.0
    } else {
        a_pass / a_total
    };

    // Regression penalty
    let regression = (b_pass - a_pass).max(0.0);
    let reg_penalty = if b_pass == 0.0 {
        0.0
    } else {
        (regression / b_pass) * 60.0
    };

    // New test bonus
    let new_tests = (a_total - b_total).max(0.0);
    let new_test_bonus = if new_tests > 0.0 {
        (new_tests * 0.5).min(10.0)
    } else {
        0.0
    };

    // Anti-gaming: if test count dropped by more than 20% from baseline,
    // apply a penalty proportional to the drop.
    let anti_gaming_penalty = if b_total > 0.0 && a_total < b_total {
        let drop_ratio = (b_total - a_total) / b_total;
        if drop_ratio > 0.2 {
            // Penalty: up to 40 points for large drops
            (drop_ratio * 40.0).min(40.0)
        } else {
            0.0
        }
    } else {
        0.0
    };

    let raw = (pass_rate * 100.0) - reg_penalty + new_test_bonus - anti_gaming_penalty;
    let score = raw.clamp(0.0, 100.0);

    TestScore {
        score,
        pass_rate,
        regression_penalty: reg_penalty,
        new_test_bonus,
        anti_gaming_penalty,
        agent_passed: input.agent_passed,
        agent_failed: input.agent_total.saturating_sub(input.agent_passed),
        agent_total: input.agent_total,
        baseline_passed: input.baseline_passed,
        baseline_total: input.baseline_total,
    }
}

#[cfg(test)]
mod test_scoring {
    use super::*;

    #[test]
    fn perfect_score_no_regression() {
        let s = score_tests(&TestInput {
            agent_passed: 10,
            agent_total: 10,
            baseline_passed: 10,
            baseline_total: 10,
        });
        assert_eq!(s.score, 100.0);
        assert_eq!(s.pass_rate, 1.0);
        assert_eq!(s.regression_penalty, 0.0);
        assert_eq!(s.new_test_bonus, 0.0);
        assert_eq!(s.anti_gaming_penalty, 0.0);
    }

    #[test]
    fn regression_penalty() {
        // Baseline: 10 passed. Agent: 8 passed, 10 total.
        // regression = 2, reg_penalty = (2/10)*60 = 12
        // pass_rate = 0.8, score = 80 - 12 = 68
        let s = score_tests(&TestInput {
            agent_passed: 8,
            agent_total: 10,
            baseline_passed: 10,
            baseline_total: 10,
        });
        assert!((s.score - 68.0).abs() < 0.01);
        assert!((s.regression_penalty - 12.0).abs() < 0.01);
    }

    #[test]
    fn new_test_bonus() {
        // Baseline: 10/10. Agent: 14/14 (4 new tests).
        // new_test_bonus = min(10, 4*0.5) = 2.0
        // score = 100 + 2 = 102 -> clamped to 100
        let s = score_tests(&TestInput {
            agent_passed: 14,
            agent_total: 14,
            baseline_passed: 10,
            baseline_total: 10,
        });
        assert_eq!(s.score, 100.0);
        assert!((s.new_test_bonus - 2.0).abs() < 0.01);
    }

    #[test]
    fn new_test_bonus_capped() {
        // 30 new tests => bonus = min(10, 30*0.5) = 10
        let s = score_tests(&TestInput {
            agent_passed: 40,
            agent_total: 40,
            baseline_passed: 10,
            baseline_total: 10,
        });
        assert!((s.new_test_bonus - 10.0).abs() < 0.01);
    }

    #[test]
    fn zero_tests_agent() {
        let s = score_tests(&TestInput {
            agent_passed: 0,
            agent_total: 0,
            baseline_passed: 10,
            baseline_total: 10,
        });
        // pass_rate = 0, regression = 10, reg_penalty = 60
        // anti_gaming: drop 100% > 20%, penalty = 1.0 * 40 = 40
        // score = 0 - 60 - 40 = -100 -> clamped to 0
        assert_eq!(s.score, 0.0);
    }

    #[test]
    fn zero_baseline() {
        // No baseline tests. Agent adds 5 new tests, all passing.
        let s = score_tests(&TestInput {
            agent_passed: 5,
            agent_total: 5,
            baseline_passed: 0,
            baseline_total: 0,
        });
        // pass_rate = 1.0, reg_penalty = 0 (B_pass=0), new_test_bonus = min(10, 5*0.5)=2.5
        // score = 100 + 2.5 = 102.5 -> clamped to 100
        assert_eq!(s.score, 100.0);
        assert!((s.new_test_bonus - 2.5).abs() < 0.01);
    }

    #[test]
    fn anti_gaming_dropped_tests() {
        // Baseline: 20 tests. Agent: 5 tests (dropped 75%).
        // All 5 pass. drop_ratio = 15/20 = 0.75, penalty = 0.75*40 = 30
        let s = score_tests(&TestInput {
            agent_passed: 5,
            agent_total: 5,
            baseline_passed: 20,
            baseline_total: 20,
        });
        // pass_rate = 1.0, regression = 15, reg_penalty = (15/20)*60 = 45
        // anti_gaming = 30
        // score = 100 - 45 - 30 = 25
        assert!((s.score - 25.0).abs() < 0.01);
    }

    #[test]
    fn anti_gaming_small_drop_no_penalty() {
        // Drop 10% (below 20% threshold)
        let s = score_tests(&TestInput {
            agent_passed: 9,
            agent_total: 9,
            baseline_passed: 10,
            baseline_total: 10,
        });
        assert_eq!(s.anti_gaming_penalty, 0.0);
    }

    #[test]
    fn partial_pass_rate() {
        // Agent: 7 out of 10. Baseline: 7/10.
        // No regression, no new tests.
        // score = 70
        let s = score_tests(&TestInput {
            agent_passed: 7,
            agent_total: 10,
            baseline_passed: 7,
            baseline_total: 10,
        });
        assert!((s.score - 70.0).abs() < 0.01);
    }

    #[test]
    fn score_clamped_at_zero() {
        // Massive regression
        let s = score_tests(&TestInput {
            agent_passed: 0,
            agent_total: 10,
            baseline_passed: 10,
            baseline_total: 10,
        });
        // pass_rate = 0, reg_penalty = (10/10)*60 = 60
        // score = 0 - 60 = -60 -> 0
        assert_eq!(s.score, 0.0);
    }
}
