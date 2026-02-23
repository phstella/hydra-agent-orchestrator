use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::DimensionScore;
use crate::config::{GatesConfig, WeightsConfig};

/// Full score for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentScore {
    pub agent_key: String,
    pub dimensions: Vec<DimensionScore>,
    pub composite: f64,
    pub mergeable: bool,
    pub gate_failures: Vec<String>,
}

/// Rank agents by weighted composite score with mergeability gates.
pub fn rank_agents(
    mut agent_dimensions: Vec<(String, Vec<DimensionScore>)>,
    weights: &WeightsConfig,
    gates: &GatesConfig,
    durations: &HashMap<String, Duration>,
) -> Vec<AgentScore> {
    let fastest_ms = durations
        .values()
        .map(|d| d.as_millis() as f64)
        .filter(|d| *d > 0.0)
        .fold(f64::MAX, f64::min);

    let mut scores: Vec<AgentScore> = agent_dimensions
        .drain(..)
        .map(|(agent_key, mut dims)| {
            if let Some(dur) = durations.get(&agent_key) {
                let agent_ms = dur.as_millis() as f64;
                let speed_score = if agent_ms > 0.0 {
                    (fastest_ms / agent_ms * 100.0).min(100.0)
                } else {
                    100.0
                };
                dims.push(DimensionScore {
                    name: "speed".to_string(),
                    score: speed_score,
                    evidence: serde_json::json!({
                        "agent_duration_ms": agent_ms as u64,
                        "fastest_ms": fastest_ms as u64,
                    }),
                });
            }

            let composite = compute_composite(&dims, weights);
            let (mergeable, gate_failures) = check_gates(&dims, gates);

            AgentScore {
                agent_key,
                dimensions: dims,
                composite,
                mergeable,
                gate_failures,
            }
        })
        .collect();

    scores.sort_by(|a, b| {
        b.composite
            .partial_cmp(&a.composite)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scores
}

fn weight_for_dimension(name: &str, weights: &WeightsConfig) -> u32 {
    match name {
        "build" => weights.build,
        "tests" => weights.tests,
        "lint" => weights.lint,
        "diff_scope" => weights.diff_scope,
        "speed" => weights.speed,
        _ => 0,
    }
}

fn compute_composite(dimensions: &[DimensionScore], weights: &WeightsConfig) -> f64 {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;

    for dim in dimensions {
        let w = weight_for_dimension(&dim.name, weights) as f64;
        if w > 0.0 {
            weighted_sum += dim.score * w;
            total_weight += w;
        }
    }

    if total_weight == 0.0 {
        return 0.0;
    }

    weighted_sum / total_weight
}

fn check_gates(dimensions: &[DimensionScore], gates: &GatesConfig) -> (bool, Vec<String>) {
    let mut failures = Vec::new();

    if gates.require_build_pass {
        if let Some(build) = dimensions.iter().find(|d| d.name == "build") {
            if build.score < 100.0 {
                failures.push("build failed".to_string());
            }
        }
    }

    if gates.max_test_regression_percent >= 0.0 {
        if let Some(tests) = dimensions.iter().find(|d| d.name == "tests") {
            if let Some(regression) = tests.evidence.get("regression") {
                if let Some(reg_count) = regression.as_u64() {
                    if let Some(baseline_passed) = tests
                        .evidence
                        .get("baseline_passed")
                        .and_then(|v| v.as_u64())
                    {
                        if baseline_passed > 0 && reg_count > 0 {
                            let reg_pct = (reg_count as f64 / baseline_passed as f64) * 100.0;
                            if reg_pct > gates.max_test_regression_percent {
                                failures.push(format!(
                                    "test regression {reg_pct:.1}% exceeds max {:.1}%",
                                    gates.max_test_regression_percent
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    let mergeable = failures.is_empty();
    (mergeable, failures)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dim(name: &str, score: f64) -> DimensionScore {
        DimensionScore {
            name: name.to_string(),
            score,
            evidence: serde_json::json!({}),
        }
    }

    fn default_weights() -> WeightsConfig {
        WeightsConfig::default()
    }

    fn default_gates() -> GatesConfig {
        GatesConfig::default()
    }

    #[test]
    fn composite_weighted_correctly() {
        let dims = vec![
            make_dim("build", 100.0),
            make_dim("tests", 80.0),
            make_dim("lint", 90.0),
            make_dim("diff_scope", 70.0),
        ];
        let composite = compute_composite(&dims, &default_weights());
        // (100*30 + 80*30 + 90*15 + 70*15) / (30+30+15+15) = (3000+2400+1350+1050)/90 = 86.67
        assert!((composite - 86.67).abs() < 0.1);
    }

    #[test]
    fn missing_dimensions_renormalize() {
        let dims = vec![make_dim("build", 100.0), make_dim("tests", 80.0)];
        let composite = compute_composite(&dims, &default_weights());
        // Only build(30) + tests(30) active, total_weight=60
        // (100*30 + 80*30) / 60 = 5400/60 = 90
        assert!((composite - 90.0).abs() < 0.01);
    }

    #[test]
    fn no_dimensions_scores_zero() {
        let dims = vec![];
        let composite = compute_composite(&dims, &default_weights());
        assert!((composite - 0.0).abs() < 0.01);
    }

    #[test]
    fn build_gate_fails_when_build_zero() {
        let dims = vec![make_dim("build", 0.0), make_dim("tests", 100.0)];
        let (mergeable, failures) = check_gates(&dims, &default_gates());
        assert!(!mergeable);
        assert!(failures.iter().any(|f| f.contains("build failed")));
    }

    #[test]
    fn build_gate_passes_when_build_100() {
        let dims = vec![make_dim("build", 100.0), make_dim("tests", 100.0)];
        let (mergeable, _) = check_gates(&dims, &default_gates());
        assert!(mergeable);
    }

    #[test]
    fn rank_agents_sorted_by_composite() {
        let agents = vec![
            (
                "codex".to_string(),
                vec![make_dim("build", 100.0), make_dim("tests", 70.0)],
            ),
            (
                "claude".to_string(),
                vec![make_dim("build", 100.0), make_dim("tests", 90.0)],
            ),
        ];
        let durations = HashMap::new();
        let ranked = rank_agents(agents, &default_weights(), &default_gates(), &durations);
        assert_eq!(ranked[0].agent_key, "claude");
        assert_eq!(ranked[1].agent_key, "codex");
    }

    #[test]
    fn speed_dimension_added_from_durations() {
        let agents = vec![
            ("fast".to_string(), vec![make_dim("build", 100.0)]),
            ("slow".to_string(), vec![make_dim("build", 100.0)]),
        ];
        let mut durations = HashMap::new();
        durations.insert("fast".to_string(), Duration::from_secs(10));
        durations.insert("slow".to_string(), Duration::from_secs(30));

        let ranked = rank_agents(agents, &default_weights(), &default_gates(), &durations);
        let fast_speed = ranked
            .iter()
            .find(|a| a.agent_key == "fast")
            .unwrap()
            .dimensions
            .iter()
            .find(|d| d.name == "speed")
            .unwrap();
        let slow_speed = ranked
            .iter()
            .find(|a| a.agent_key == "slow")
            .unwrap()
            .dimensions
            .iter()
            .find(|d| d.name == "speed")
            .unwrap();

        assert!((fast_speed.score - 100.0).abs() < 0.01);
        assert!((slow_speed.score - 33.33).abs() < 0.5);
    }

    #[test]
    fn unmergeable_agent_still_ranked() {
        let agents = vec![
            (
                "bad".to_string(),
                vec![make_dim("build", 0.0), make_dim("tests", 100.0)],
            ),
            (
                "good".to_string(),
                vec![make_dim("build", 100.0), make_dim("tests", 80.0)],
            ),
        ];
        let durations = HashMap::new();
        let ranked = rank_agents(agents, &default_weights(), &default_gates(), &durations);
        assert_eq!(ranked.len(), 2);
        assert!(
            !ranked
                .iter()
                .find(|a| a.agent_key == "bad")
                .unwrap()
                .mergeable
        );
        assert!(
            ranked
                .iter()
                .find(|a| a.agent_key == "good")
                .unwrap()
                .mergeable
        );
    }
}
