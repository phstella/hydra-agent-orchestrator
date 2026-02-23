use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{ScoringGates, ScoringWeights};

/// Per-dimension score breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub build: Option<f64>,
    pub tests: Option<f64>,
    pub lint: Option<f64>,
    pub diff_scope: Option<f64>,
    pub speed: Option<f64>,
}

/// Complete score for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentScore {
    pub agent_key: String,
    pub total: f64,
    pub breakdown: ScoreBreakdown,
    pub mergeable: bool,
    pub gate_failures: Vec<String>,
}

/// Full ranking result for a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingResult {
    pub run_id: Uuid,
    pub rankings: Vec<AgentScore>,
}

/// Input for scoring a single agent.
pub struct AgentInput {
    pub agent_key: String,
    pub build_score: Option<f64>,
    pub build_passed: bool,
    pub test_score: Option<f64>,
    pub test_regression_percent: f64,
    pub lint_score: Option<f64>,
    pub diff_scope_score: Option<f64>,
    pub speed_score: Option<f64>,
}

/// Compute composite scores and rank agents.
///
/// Composite = sum(dimension_score * dimension_weight) / sum(active_weights)
/// where active_weights excludes None dimensions (renormalization).
///
/// Gating rules:
/// - Build must pass (if gate enabled)
/// - Test regression below threshold (if gate enabled)
pub fn rank_agents(
    run_id: Uuid,
    agents: Vec<AgentInput>,
    weights: &ScoringWeights,
    gates: &ScoringGates,
) -> RankingResult {
    let mut rankings: Vec<AgentScore> = agents
        .into_iter()
        .map(|agent| score_agent(agent, weights, gates))
        .collect();

    // Sort by total descending (stable sort for determinism with ties)
    rankings.sort_by(|a, b| {
        b.total
            .partial_cmp(&a.total)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    RankingResult { run_id, rankings }
}

fn score_agent(input: AgentInput, weights: &ScoringWeights, gates: &ScoringGates) -> AgentScore {
    let breakdown = ScoreBreakdown {
        build: input.build_score,
        tests: input.test_score,
        lint: input.lint_score,
        diff_scope: input.diff_scope_score,
        speed: input.speed_score,
    };

    // Compute weighted composite with renormalization
    let mut weighted_sum = 0.0;
    let mut active_weight_sum = 0.0;

    if let Some(s) = breakdown.build {
        weighted_sum += s * weights.build as f64;
        active_weight_sum += weights.build as f64;
    }
    if let Some(s) = breakdown.tests {
        weighted_sum += s * weights.tests as f64;
        active_weight_sum += weights.tests as f64;
    }
    if let Some(s) = breakdown.lint {
        weighted_sum += s * weights.lint as f64;
        active_weight_sum += weights.lint as f64;
    }
    if let Some(s) = breakdown.diff_scope {
        weighted_sum += s * weights.diff_scope as f64;
        active_weight_sum += weights.diff_scope as f64;
    }
    if let Some(s) = breakdown.speed {
        weighted_sum += s * weights.speed as f64;
        active_weight_sum += weights.speed as f64;
    }

    let total = if active_weight_sum > 0.0 {
        weighted_sum / active_weight_sum
    } else {
        0.0
    };

    // Check gating rules
    let mut gate_failures = Vec::new();

    if gates.require_build_pass && !input.build_passed {
        gate_failures.push("build_failed".to_string());
    }

    if input.test_regression_percent > gates.max_test_regression_percent {
        gate_failures.push(format!(
            "test_regression_{:.1}%_exceeds_max_{:.1}%",
            input.test_regression_percent, gates.max_test_regression_percent
        ));
    }

    let mergeable = gate_failures.is_empty();

    AgentScore {
        agent_key: input.agent_key,
        total,
        breakdown,
        mergeable,
        gate_failures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_weights() -> ScoringWeights {
        ScoringWeights::default()
    }

    fn default_gates() -> ScoringGates {
        ScoringGates::default()
    }

    fn make_agent(
        key: &str,
        build: f64,
        tests: f64,
        lint: f64,
        diff: f64,
        speed: f64,
    ) -> AgentInput {
        AgentInput {
            agent_key: key.to_string(),
            build_score: Some(build),
            build_passed: build == 100.0,
            test_score: Some(tests),
            test_regression_percent: 0.0,
            lint_score: Some(lint),
            diff_scope_score: Some(diff),
            speed_score: Some(speed),
        }
    }

    #[test]
    fn perfect_scores() {
        let result = rank_agents(
            Uuid::nil(),
            vec![make_agent("agent1", 100.0, 100.0, 100.0, 100.0, 100.0)],
            &default_weights(),
            &default_gates(),
        );
        assert_eq!(result.rankings.len(), 1);
        let agent = &result.rankings[0];
        assert_eq!(agent.total, 100.0);
        assert!(agent.mergeable);
        assert!(agent.gate_failures.is_empty());
    }

    #[test]
    fn ranking_order() {
        let result = rank_agents(
            Uuid::nil(),
            vec![
                make_agent("slow", 100.0, 80.0, 90.0, 85.0, 50.0),
                make_agent("fast", 100.0, 95.0, 95.0, 90.0, 100.0),
                make_agent("mid", 100.0, 90.0, 85.0, 88.0, 75.0),
            ],
            &default_weights(),
            &default_gates(),
        );
        assert_eq!(result.rankings[0].agent_key, "fast");
        assert_eq!(result.rankings[1].agent_key, "mid");
        assert_eq!(result.rankings[2].agent_key, "slow");
    }

    #[test]
    fn build_gate_failure() {
        let result = rank_agents(
            Uuid::nil(),
            vec![AgentInput {
                agent_key: "broken".to_string(),
                build_score: Some(0.0),
                build_passed: false,
                test_score: Some(100.0),
                test_regression_percent: 0.0,
                lint_score: Some(100.0),
                diff_scope_score: Some(100.0),
                speed_score: Some(100.0),
            }],
            &default_weights(),
            &default_gates(),
        );
        let agent = &result.rankings[0];
        assert!(!agent.mergeable);
        assert!(agent.gate_failures.contains(&"build_failed".to_string()));
    }

    #[test]
    fn test_regression_gate() {
        let result = rank_agents(
            Uuid::nil(),
            vec![AgentInput {
                agent_key: "regressed".to_string(),
                build_score: Some(100.0),
                build_passed: true,
                test_score: Some(70.0),
                test_regression_percent: 15.0,
                lint_score: Some(100.0),
                diff_scope_score: Some(100.0),
                speed_score: Some(100.0),
            }],
            &default_weights(),
            &ScoringGates {
                require_build_pass: true,
                max_test_regression_percent: 10.0,
            },
        );
        let agent = &result.rankings[0];
        assert!(!agent.mergeable);
        assert!(agent
            .gate_failures
            .iter()
            .any(|f| f.contains("test_regression")));
    }

    #[test]
    fn weight_renormalization_missing_dimensions() {
        // Only build and tests provided. Weights: build=30, tests=30.
        // build=100, tests=50 => composite = (100*30 + 50*30) / 60 = 4500/60 = 75
        let result = rank_agents(
            Uuid::nil(),
            vec![AgentInput {
                agent_key: "partial".to_string(),
                build_score: Some(100.0),
                build_passed: true,
                test_score: Some(50.0),
                test_regression_percent: 0.0,
                lint_score: None,
                diff_scope_score: None,
                speed_score: None,
            }],
            &default_weights(),
            &default_gates(),
        );
        let agent = &result.rankings[0];
        assert!((agent.total - 75.0).abs() < 0.01);
        assert!(agent.breakdown.lint.is_none());
        assert!(agent.breakdown.diff_scope.is_none());
        assert!(agent.breakdown.speed.is_none());
    }

    #[test]
    fn all_dimensions_missing() {
        let result = rank_agents(
            Uuid::nil(),
            vec![AgentInput {
                agent_key: "empty".to_string(),
                build_score: None,
                build_passed: false,
                test_score: None,
                test_regression_percent: 0.0,
                lint_score: None,
                diff_scope_score: None,
                speed_score: None,
            }],
            &default_weights(),
            &default_gates(),
        );
        let agent = &result.rankings[0];
        assert_eq!(agent.total, 0.0);
    }

    #[test]
    fn custom_weights() {
        // All weight on tests
        let weights = ScoringWeights {
            build: 0,
            tests: 100,
            lint: 0,
            diff_scope: 0,
            speed: 0,
        };
        let result = rank_agents(
            Uuid::nil(),
            vec![make_agent("agent1", 0.0, 85.0, 0.0, 0.0, 0.0)],
            &weights,
            &ScoringGates {
                require_build_pass: false,
                max_test_regression_percent: 100.0,
            },
        );
        let agent = &result.rankings[0];
        assert!((agent.total - 85.0).abs() < 0.01);
    }

    #[test]
    fn composite_calculation_correctness() {
        // build=100 w=30, tests=80 w=30, lint=90 w=15, diff=70 w=15, speed=60 w=10
        // composite = (100*30 + 80*30 + 90*15 + 70*15 + 60*10) / 100
        //           = (3000 + 2400 + 1350 + 1050 + 600) / 100
        //           = 8400 / 100 = 84
        let result = rank_agents(
            Uuid::nil(),
            vec![make_agent("agent1", 100.0, 80.0, 90.0, 70.0, 60.0)],
            &default_weights(),
            &default_gates(),
        );
        let agent = &result.rankings[0];
        assert!((agent.total - 84.0).abs() < 0.01);
    }

    #[test]
    fn build_gate_disabled() {
        let result = rank_agents(
            Uuid::nil(),
            vec![AgentInput {
                agent_key: "no_gate".to_string(),
                build_score: Some(0.0),
                build_passed: false,
                test_score: Some(100.0),
                test_regression_percent: 0.0,
                lint_score: None,
                diff_scope_score: None,
                speed_score: None,
            }],
            &default_weights(),
            &ScoringGates {
                require_build_pass: false,
                max_test_regression_percent: 100.0,
            },
        );
        let agent = &result.rankings[0];
        assert!(agent.mergeable);
    }

    #[test]
    fn deterministic_ordering_with_ties() {
        // Two agents with identical scores
        let result = rank_agents(
            Uuid::nil(),
            vec![
                make_agent("alpha", 100.0, 100.0, 100.0, 100.0, 100.0),
                make_agent("beta", 100.0, 100.0, 100.0, 100.0, 100.0),
            ],
            &default_weights(),
            &default_gates(),
        );
        // Stable sort preserves input order for ties
        assert_eq!(result.rankings[0].agent_key, "alpha");
        assert_eq!(result.rankings[1].agent_key, "beta");
    }
}
