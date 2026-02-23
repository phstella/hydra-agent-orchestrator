//! Builder-Reviewer-Refiner workflow preset.
//!
//! A three-stage pipeline: build code, review it, then refine based on feedback.

use crate::workflow::engine::{NodeType, WorkflowDefinition, WorkflowNode};

/// Create a builder-reviewer-refiner workflow.
///
/// Stages:
/// 1. **build** - Agent produces initial code from the task prompt.
/// 2. **review** - A different agent reviews the build output.
/// 3. **refine** - A third agent applies the reviewer's feedback.
/// 4. **score** - Deterministic scoring of the refined result.
pub fn builder_reviewer_refiner(
    builder_agent: &str,
    reviewer_agent: &str,
    refiner_agent: &str,
    task_prompt: &str,
) -> WorkflowDefinition {
    WorkflowDefinition {
        name: "builder-reviewer-refiner".to_string(),
        description: "Build, review, then refine code".to_string(),
        nodes: vec![
            WorkflowNode {
                id: "build".into(),
                node_type: NodeType::Build,
                agent_key: Some(builder_agent.into()),
                prompt_template: task_prompt.into(),
                depends_on: vec![],
                timeout_seconds: Some(600),
                max_retries: 0,
            },
            WorkflowNode {
                id: "review".into(),
                node_type: NodeType::Review,
                agent_key: Some(reviewer_agent.into()),
                prompt_template: concat!(
                    "Review the code changes from the build step. ",
                    "Provide structured feedback on: correctness, code quality, ",
                    "test coverage, and potential issues. Output a JSON rubric ",
                    "with scores and specific improvement suggestions."
                )
                .into(),
                depends_on: vec!["build".into()],
                timeout_seconds: Some(300),
                max_retries: 0,
            },
            WorkflowNode {
                id: "refine".into(),
                node_type: NodeType::Refine,
                agent_key: Some(refiner_agent.into()),
                prompt_template: concat!(
                    "Apply the reviewer's feedback to improve the code. ",
                    "Focus on the specific issues identified."
                )
                .into(),
                depends_on: vec!["review".into()],
                timeout_seconds: Some(600),
                max_retries: 0,
            },
            WorkflowNode {
                id: "score".into(),
                node_type: NodeType::Score,
                agent_key: None,
                prompt_template: String::new(),
                depends_on: vec!["refine".into()],
                timeout_seconds: Some(120),
                max_retries: 0,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_structure() {
        let wf = builder_reviewer_refiner("claude", "codex", "claude", "implement feature X");

        assert_eq!(wf.name, "builder-reviewer-refiner");
        assert_eq!(wf.nodes.len(), 4);

        // Verify node types.
        assert_eq!(wf.nodes[0].node_type, NodeType::Build);
        assert_eq!(wf.nodes[1].node_type, NodeType::Review);
        assert_eq!(wf.nodes[2].node_type, NodeType::Refine);
        assert_eq!(wf.nodes[3].node_type, NodeType::Score);

        // Verify agent assignments.
        assert_eq!(wf.nodes[0].agent_key.as_deref(), Some("claude"));
        assert_eq!(wf.nodes[1].agent_key.as_deref(), Some("codex"));
        assert_eq!(wf.nodes[2].agent_key.as_deref(), Some("claude"));
        assert!(wf.nodes[3].agent_key.is_none()); // Score is agentless.

        // Verify dependency chain.
        assert!(wf.nodes[0].depends_on.is_empty());
        assert_eq!(wf.nodes[1].depends_on, vec!["build"]);
        assert_eq!(wf.nodes[2].depends_on, vec!["review"]);
        assert_eq!(wf.nodes[3].depends_on, vec!["refine"]);

        // Verify task prompt is in the build node.
        assert_eq!(wf.nodes[0].prompt_template, "implement feature X");
    }

    #[test]
    fn preset_timeouts() {
        let wf = builder_reviewer_refiner("claude", "codex", "claude", "task");

        assert_eq!(wf.nodes[0].timeout_seconds, Some(600));
        assert_eq!(wf.nodes[1].timeout_seconds, Some(300));
        assert_eq!(wf.nodes[2].timeout_seconds, Some(600));
        assert_eq!(wf.nodes[3].timeout_seconds, Some(120));
    }
}
