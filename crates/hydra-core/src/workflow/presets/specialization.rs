//! Specialization workflow preset.
//!
//! Multiple agents work on separate scopes in parallel, then an optional
//! integration agent merges the results.

use crate::workflow::engine::{NodeType, WorkflowDefinition, WorkflowNode};

/// Create a specialization workflow.
///
/// Each scope gets its own build node running in parallel. An optional
/// integration agent merges all scope branches, followed by scoring.
///
/// # Arguments
///
/// * `scopes` - Vec of `(scope_name, agent_key, prompt)` tuples. Each scope
///   runs as an independent parallel build node.
/// * `integration_agent` - Optional agent key for the merge/integration step.
///   If `None`, the merge node runs without an agent (deterministic merge).
pub fn specialization(
    scopes: Vec<(String, String, String)>,
    integration_agent: Option<String>,
) -> WorkflowDefinition {
    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut scope_ids: Vec<String> = Vec::new();

    // Create parallel build nodes for each scope.
    for (scope_name, agent_key, prompt) in &scopes {
        let node_id = format!("scope-{scope_name}");
        scope_ids.push(node_id.clone());

        nodes.push(WorkflowNode {
            id: node_id,
            node_type: NodeType::Build,
            agent_key: Some(agent_key.clone()),
            prompt_template: prompt.clone(),
            depends_on: vec![],
            timeout_seconds: Some(600),
            max_retries: 0,
        });
    }

    // Integration merge node that depends on all scopes.
    nodes.push(WorkflowNode {
        id: "integrate".into(),
        node_type: NodeType::Merge,
        agent_key: integration_agent,
        prompt_template: "Merge all scope branches and resolve any conflicts. \
                          Verify that changes from each scope do not conflict \
                          with each other."
            .into(),
        depends_on: scope_ids,
        timeout_seconds: Some(300),
        max_retries: 0,
    });

    // Final scoring node.
    nodes.push(WorkflowNode {
        id: "score".into(),
        node_type: NodeType::Score,
        agent_key: None,
        prompt_template: String::new(),
        depends_on: vec!["integrate".into()],
        timeout_seconds: Some(120),
        max_retries: 0,
    });

    let scope_count = scopes.len();
    WorkflowDefinition {
        name: "specialization".to_string(),
        description: format!(
            "Parallel specialization across {scope_count} scopes with integration"
        ),
        nodes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_with_two_scopes() {
        let wf = specialization(
            vec![
                ("frontend".into(), "claude".into(), "build the UI".into()),
                ("backend".into(), "codex".into(), "build the API".into()),
            ],
            Some("claude".into()),
        );

        assert_eq!(wf.name, "specialization");
        // 2 scope nodes + integrate + score = 4
        assert_eq!(wf.nodes.len(), 4);

        // Scope nodes are parallel (no dependencies).
        let fe = wf.nodes.iter().find(|n| n.id == "scope-frontend").unwrap();
        assert!(fe.depends_on.is_empty());
        assert_eq!(fe.agent_key.as_deref(), Some("claude"));
        assert_eq!(fe.node_type, NodeType::Build);

        let be = wf.nodes.iter().find(|n| n.id == "scope-backend").unwrap();
        assert!(be.depends_on.is_empty());
        assert_eq!(be.agent_key.as_deref(), Some("codex"));

        // Integrate depends on both scopes.
        let integrate = wf.nodes.iter().find(|n| n.id == "integrate").unwrap();
        assert_eq!(integrate.node_type, NodeType::Merge);
        assert!(integrate.depends_on.contains(&"scope-frontend".to_string()));
        assert!(integrate.depends_on.contains(&"scope-backend".to_string()));
        assert_eq!(integrate.agent_key.as_deref(), Some("claude"));

        // Score depends on integrate.
        let score = wf.nodes.iter().find(|n| n.id == "score").unwrap();
        assert_eq!(score.depends_on, vec!["integrate"]);
        assert_eq!(score.node_type, NodeType::Score);
    }

    #[test]
    fn preset_without_integration_agent() {
        let wf = specialization(
            vec![("db".into(), "codex".into(), "migrate db".into())],
            None,
        );

        let integrate = wf.nodes.iter().find(|n| n.id == "integrate").unwrap();
        assert!(integrate.agent_key.is_none());
    }

    #[test]
    fn preset_three_scopes() {
        let wf = specialization(
            vec![
                ("auth".into(), "claude".into(), "auth module".into()),
                ("api".into(), "codex".into(), "api module".into()),
                ("ui".into(), "claude".into(), "ui module".into()),
            ],
            Some("claude".into()),
        );

        // 3 scopes + integrate + score = 5
        assert_eq!(wf.nodes.len(), 5);

        let integrate = wf.nodes.iter().find(|n| n.id == "integrate").unwrap();
        assert_eq!(integrate.depends_on.len(), 3);
    }
}
