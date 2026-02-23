//! Iterative refinement workflow preset.
//!
//! An agent repeatedly builds and gets scored until the score exceeds a
//! threshold or the convergence guard triggers.

use crate::workflow::engine::{NodeType, WorkflowDefinition, WorkflowNode};

/// Create an iterative refinement workflow.
///
/// The workflow consists of repeating build-score cycles. The engine's
/// convergence guard (checked at execution time) stops iteration when:
///
/// 1. The score exceeds `score_threshold`.
/// 2. The score decreases for two consecutive iterations.
/// 3. `max_iterations` is reached.
///
/// The workflow is represented as a flat DAG with chained iterations:
/// `build_0 -> score_0 -> build_1 -> score_1 -> ... -> build_N -> score_N`
///
/// At execution time, the engine can short-circuit remaining nodes when
/// the convergence guard fires (nodes are marked Skipped).
pub fn iterative_refinement(
    agent_key: &str,
    task_prompt: &str,
    max_iterations: u32,
    score_threshold: f64,
) -> WorkflowDefinition {
    let mut nodes: Vec<WorkflowNode> = Vec::new();

    for i in 0..max_iterations {
        let build_id = format!("build_{i}");
        let score_id = format!("score_{i}");

        // Build node depends on previous score (except first iteration).
        let build_deps = if i == 0 {
            vec![]
        } else {
            vec![format!("score_{}", i - 1)]
        };

        let prompt = if i == 0 {
            task_prompt.to_string()
        } else {
            format!(
                "Iteration {}: refine your previous solution based on the scoring feedback. \
                 Target score: {score_threshold}. Original task: {task_prompt}",
                i + 1
            )
        };

        nodes.push(WorkflowNode {
            id: build_id.clone(),
            node_type: if i == 0 {
                NodeType::Build
            } else {
                NodeType::Refine
            },
            agent_key: Some(agent_key.into()),
            prompt_template: prompt,
            depends_on: build_deps,
            timeout_seconds: Some(600),
            max_retries: 0,
        });

        nodes.push(WorkflowNode {
            id: score_id,
            node_type: NodeType::Score,
            agent_key: None,
            prompt_template: String::new(),
            depends_on: vec![build_id],
            timeout_seconds: Some(120),
            max_retries: 0,
        });
    }

    WorkflowDefinition {
        name: "iterative-refinement".to_string(),
        description: format!(
            "Iterative refinement: up to {max_iterations} iterations, \
             threshold {score_threshold}"
        ),
        nodes,
    }
}

/// Check the convergence guard for iterative refinement.
///
/// Returns `true` if iteration should stop based on the score history.
///
/// Convergence rules:
/// 1. Score exceeds threshold.
/// 2. Score decreased for two consecutive iterations.
/// 3. No improvement over the best score for `patience` iterations.
pub fn should_stop_iterating(scores: &[f64], threshold: f64, patience: usize) -> bool {
    if scores.is_empty() {
        return false;
    }

    let latest = scores[scores.len() - 1];

    // Rule 1: exceeded threshold.
    if latest >= threshold {
        return true;
    }

    // Rule 2: two consecutive decreases.
    if scores.len() >= 3 {
        let n = scores.len();
        if scores[n - 1] < scores[n - 2] && scores[n - 2] < scores[n - 3] {
            return true;
        }
    }

    // Rule 3: no improvement over best for `patience` iterations.
    if scores.len() > patience {
        let best = scores.iter().copied().reduce(f64::max).unwrap_or(0.0);
        let recent_best = scores[scores.len() - patience..]
            .iter()
            .copied()
            .reduce(f64::max)
            .unwrap_or(0.0);

        if recent_best <= best && scores.len() > patience {
            // Check if the best was achieved before the patience window.
            let pre_patience_best = scores[..scores.len() - patience]
                .iter()
                .copied()
                .reduce(f64::max)
                .unwrap_or(0.0);
            if pre_patience_best >= recent_best {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_structure() {
        let wf = iterative_refinement("claude", "build a feature", 3, 90.0);

        assert_eq!(wf.name, "iterative-refinement");
        // 3 iterations * 2 nodes each = 6 nodes
        assert_eq!(wf.nodes.len(), 6);

        // First iteration: build_0 -> score_0
        assert_eq!(wf.nodes[0].id, "build_0");
        assert_eq!(wf.nodes[0].node_type, NodeType::Build);
        assert!(wf.nodes[0].depends_on.is_empty());

        assert_eq!(wf.nodes[1].id, "score_0");
        assert_eq!(wf.nodes[1].node_type, NodeType::Score);
        assert_eq!(wf.nodes[1].depends_on, vec!["build_0"]);

        // Second iteration: build_1 depends on score_0
        assert_eq!(wf.nodes[2].id, "build_1");
        assert_eq!(wf.nodes[2].node_type, NodeType::Refine);
        assert_eq!(wf.nodes[2].depends_on, vec!["score_0"]);

        assert_eq!(wf.nodes[3].id, "score_1");
        assert_eq!(wf.nodes[3].depends_on, vec!["build_1"]);

        // Third iteration: build_2 depends on score_1
        assert_eq!(wf.nodes[4].id, "build_2");
        assert_eq!(wf.nodes[4].depends_on, vec!["score_1"]);
    }

    #[test]
    fn single_iteration() {
        let wf = iterative_refinement("codex", "task", 1, 50.0);
        assert_eq!(wf.nodes.len(), 2);
    }

    #[test]
    fn convergence_guard_threshold_met() {
        assert!(should_stop_iterating(&[60.0, 75.0, 91.0], 90.0, 3));
    }

    #[test]
    fn convergence_guard_not_met() {
        assert!(!should_stop_iterating(&[60.0, 75.0, 80.0], 90.0, 3));
    }

    #[test]
    fn convergence_guard_two_consecutive_decreases() {
        // 80 -> 75 -> 70: two consecutive decreases
        assert!(should_stop_iterating(&[80.0, 75.0, 70.0], 90.0, 10));
    }

    #[test]
    fn convergence_guard_single_decrease_ok() {
        // 80 -> 75: only one decrease, should continue
        assert!(!should_stop_iterating(&[80.0, 75.0], 90.0, 10));
    }

    #[test]
    fn convergence_guard_empty_scores() {
        assert!(!should_stop_iterating(&[], 90.0, 3));
    }

    #[test]
    fn convergence_guard_patience_exhausted() {
        // Best was 85 at index 0, then 3 iterations without improvement.
        assert!(should_stop_iterating(&[85.0, 80.0, 80.0, 80.0], 90.0, 3));
    }

    #[test]
    fn convergence_guard_patience_not_exhausted() {
        // Improving within patience window.
        assert!(!should_stop_iterating(&[60.0, 70.0, 80.0], 90.0, 3));
    }
}
