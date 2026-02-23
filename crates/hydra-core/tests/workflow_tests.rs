//! Integration tests for the workflow engine and presets.

use std::path::PathBuf;

use hydra_core::config::HydraConfig;
use hydra_core::workflow::engine::WorkflowContext;
use hydra_core::workflow::{
    builder_reviewer_refiner, iterative_refinement, should_stop_iterating, specialization,
    NodeExecutor, NodeResult, NodeStatus, NodeType, WorkflowDefinition, WorkflowEngine,
    WorkflowNode, WorkflowStatus,
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_engine() -> WorkflowEngine {
    WorkflowEngine::new(PathBuf::from("/tmp"), HydraConfig::default())
}

/// An executor that fails a specific node by ID.
struct FailOnNode {
    fail_id: String,
}

impl NodeExecutor for FailOnNode {
    async fn execute(&self, node: &WorkflowNode, _ctx: &WorkflowContext) -> NodeResult {
        if node.id == self.fail_id {
            NodeResult {
                node_id: node.id.clone(),
                status: NodeStatus::Failed,
                artifact_id: None,
                output: None,
                error: Some("intentional failure".into()),
                duration_ms: 0,
                retries_used: 0,
            }
        } else {
            let output = match node.node_type {
                NodeType::Score => Some("82.5".to_string()),
                _ => Some(format!("output from {}", node.id)),
            };
            NodeResult {
                node_id: node.id.clone(),
                status: NodeStatus::Completed,
                artifact_id: Some(Uuid::new_v4()),
                output,
                error: None,
                duration_ms: 1,
                retries_used: 0,
            }
        }
    }
}

/// An executor that retries and eventually succeeds after N failures.
struct RetryExecutor {
    fail_count: std::sync::atomic::AtomicU32,
    max_failures: u32,
}

impl NodeExecutor for RetryExecutor {
    async fn execute(&self, node: &WorkflowNode, _ctx: &WorkflowContext) -> NodeResult {
        let count = self
            .fail_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if count < self.max_failures {
            NodeResult {
                node_id: node.id.clone(),
                status: NodeStatus::Failed,
                artifact_id: None,
                output: None,
                error: Some(format!("failure #{}", count + 1)),
                duration_ms: 0,
                retries_used: 0,
            }
        } else {
            NodeResult {
                node_id: node.id.clone(),
                status: NodeStatus::Completed,
                artifact_id: Some(Uuid::new_v4()),
                output: Some("success after retries".into()),
                error: None,
                duration_ms: 0,
                retries_used: 0,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Topological sort tests
// ---------------------------------------------------------------------------

#[test]
fn topo_sort_empty_workflow() {
    let engine = default_engine();
    let wf = WorkflowDefinition {
        name: "empty".into(),
        description: "no nodes".into(),
        nodes: vec![],
    };
    let levels = engine.resolve_order(&wf).unwrap();
    assert!(levels.is_empty());
}

#[test]
fn topo_sort_single_node() {
    let engine = default_engine();
    let wf = WorkflowDefinition {
        name: "single".into(),
        description: "one node".into(),
        nodes: vec![WorkflowNode {
            id: "only".into(),
            node_type: NodeType::Build,
            agent_key: Some("claude".into()),
            prompt_template: "do it".into(),
            depends_on: vec![],
            timeout_seconds: None,
            max_retries: 0,
        }],
    };
    let levels = engine.resolve_order(&wf).unwrap();
    assert_eq!(levels, vec![vec!["only".to_string()]]);
}

#[test]
fn topo_sort_diamond() {
    let engine = default_engine();
    let wf = WorkflowDefinition {
        name: "diamond".into(),
        description: "diamond shape DAG".into(),
        nodes: vec![
            WorkflowNode {
                id: "a".into(),
                node_type: NodeType::Build,
                agent_key: None,
                prompt_template: String::new(),
                depends_on: vec![],
                timeout_seconds: None,
                max_retries: 0,
            },
            WorkflowNode {
                id: "b".into(),
                node_type: NodeType::Build,
                agent_key: None,
                prompt_template: String::new(),
                depends_on: vec!["a".into()],
                timeout_seconds: None,
                max_retries: 0,
            },
            WorkflowNode {
                id: "c".into(),
                node_type: NodeType::Build,
                agent_key: None,
                prompt_template: String::new(),
                depends_on: vec!["a".into()],
                timeout_seconds: None,
                max_retries: 0,
            },
            WorkflowNode {
                id: "d".into(),
                node_type: NodeType::Merge,
                agent_key: None,
                prompt_template: String::new(),
                depends_on: vec!["b".into(), "c".into()],
                timeout_seconds: None,
                max_retries: 0,
            },
        ],
    };

    let levels = engine.resolve_order(&wf).unwrap();
    assert_eq!(levels.len(), 3);
    assert_eq!(levels[0], vec!["a"]);
    assert_eq!(levels[1], vec!["b", "c"]);
    assert_eq!(levels[2], vec!["d"]);
}

#[test]
fn topo_sort_cycle_detected() {
    let engine = default_engine();
    let wf = WorkflowDefinition {
        name: "cycle".into(),
        description: "cycle".into(),
        nodes: vec![
            WorkflowNode {
                id: "x".into(),
                node_type: NodeType::Build,
                agent_key: None,
                prompt_template: String::new(),
                depends_on: vec!["y".into()],
                timeout_seconds: None,
                max_retries: 0,
            },
            WorkflowNode {
                id: "y".into(),
                node_type: NodeType::Build,
                agent_key: None,
                prompt_template: String::new(),
                depends_on: vec!["z".into()],
                timeout_seconds: None,
                max_retries: 0,
            },
            WorkflowNode {
                id: "z".into(),
                node_type: NodeType::Build,
                agent_key: None,
                prompt_template: String::new(),
                depends_on: vec!["x".into()],
                timeout_seconds: None,
                max_retries: 0,
            },
        ],
    };

    let err = engine.resolve_order(&wf).unwrap_err();
    assert!(
        err.to_string().contains("cycle"),
        "expected cycle error, got: {err}"
    );
}

#[test]
fn topo_sort_unknown_dep() {
    let engine = default_engine();
    let wf = WorkflowDefinition {
        name: "bad".into(),
        description: "bad dep".into(),
        nodes: vec![WorkflowNode {
            id: "a".into(),
            node_type: NodeType::Build,
            agent_key: None,
            prompt_template: String::new(),
            depends_on: vec!["ghost".into()],
            timeout_seconds: None,
            max_retries: 0,
        }],
    };

    let err = engine.resolve_order(&wf).unwrap_err();
    assert!(
        err.to_string().contains("ghost"),
        "expected error about ghost, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// Builder-Reviewer-Refiner preset tests
// ---------------------------------------------------------------------------

#[test]
fn builder_reviewer_preset_structure() {
    let wf = builder_reviewer_refiner("claude", "codex", "claude", "implement X");

    assert_eq!(wf.name, "builder-reviewer-refiner");
    assert_eq!(wf.nodes.len(), 4);
    assert_eq!(wf.nodes[0].node_type, NodeType::Build);
    assert_eq!(wf.nodes[1].node_type, NodeType::Review);
    assert_eq!(wf.nodes[2].node_type, NodeType::Refine);
    assert_eq!(wf.nodes[3].node_type, NodeType::Score);

    // Dependency chain is linear.
    assert!(wf.nodes[0].depends_on.is_empty());
    assert_eq!(wf.nodes[1].depends_on, vec!["build"]);
    assert_eq!(wf.nodes[2].depends_on, vec!["review"]);
    assert_eq!(wf.nodes[3].depends_on, vec!["refine"]);
}

#[tokio::test]
async fn builder_reviewer_executes_successfully() {
    let engine = default_engine();
    let wf = builder_reviewer_refiner("claude", "codex", "claude", "build a feature");
    let result = engine.execute(&wf).await.unwrap();

    assert_eq!(result.status, WorkflowStatus::Completed);
    assert_eq!(result.node_results.len(), 4);
    assert!(result.final_score.is_some());
}

// ---------------------------------------------------------------------------
// Specialization preset tests
// ---------------------------------------------------------------------------

#[test]
fn specialization_preset_two_scopes() {
    let wf = specialization(
        vec![
            ("frontend".into(), "claude".into(), "build UI".into()),
            ("backend".into(), "codex".into(), "build API".into()),
        ],
        Some("claude".into()),
    );

    assert_eq!(wf.name, "specialization");
    assert_eq!(wf.nodes.len(), 4); // 2 scopes + integrate + score

    // Parallel scope nodes.
    let fe = wf.nodes.iter().find(|n| n.id == "scope-frontend").unwrap();
    assert!(fe.depends_on.is_empty());

    let be = wf.nodes.iter().find(|n| n.id == "scope-backend").unwrap();
    assert!(be.depends_on.is_empty());

    // Integrate depends on both.
    let integrate = wf.nodes.iter().find(|n| n.id == "integrate").unwrap();
    assert!(integrate.depends_on.contains(&"scope-frontend".to_string()));
    assert!(integrate.depends_on.contains(&"scope-backend".to_string()));
}

#[tokio::test]
async fn specialization_executes_successfully() {
    let engine = default_engine();
    let wf = specialization(
        vec![
            ("a".into(), "claude".into(), "scope a".into()),
            ("b".into(), "codex".into(), "scope b".into()),
        ],
        Some("claude".into()),
    );
    let result = engine.execute(&wf).await.unwrap();

    assert_eq!(result.status, WorkflowStatus::Completed);
    assert_eq!(result.node_results.len(), 4);
}

// ---------------------------------------------------------------------------
// Iterative refinement preset tests
// ---------------------------------------------------------------------------

#[test]
fn iterative_preset_structure() {
    let wf = iterative_refinement("claude", "task", 3, 90.0);

    assert_eq!(wf.name, "iterative-refinement");
    assert_eq!(wf.nodes.len(), 6); // 3 * (build + score)

    // First iteration: build_0 has no deps.
    assert!(wf.nodes[0].depends_on.is_empty());
    assert_eq!(wf.nodes[0].node_type, NodeType::Build);

    // Subsequent iterations: build_N depends on score_(N-1).
    assert_eq!(wf.nodes[2].depends_on, vec!["score_0"]);
    assert_eq!(wf.nodes[2].node_type, NodeType::Refine);
}

#[tokio::test]
async fn iterative_executes_all_iterations() {
    let engine = default_engine();
    let wf = iterative_refinement("claude", "improve this", 2, 99.0);
    let result = engine.execute(&wf).await.unwrap();

    assert_eq!(result.status, WorkflowStatus::Completed);
    assert_eq!(result.node_results.len(), 4); // 2 * (build + score)
}

// ---------------------------------------------------------------------------
// Convergence guard tests
// ---------------------------------------------------------------------------

#[test]
fn convergence_threshold_met() {
    assert!(should_stop_iterating(&[70.0, 80.0, 91.0], 90.0, 5));
}

#[test]
fn convergence_two_consecutive_decreases() {
    assert!(should_stop_iterating(&[85.0, 80.0, 75.0], 95.0, 10));
}

#[test]
fn convergence_single_decrease_continues() {
    assert!(!should_stop_iterating(&[85.0, 80.0], 95.0, 10));
}

#[test]
fn convergence_patience_exhausted() {
    assert!(should_stop_iterating(&[90.0, 80.0, 80.0, 80.0], 95.0, 3));
}

#[test]
fn convergence_improving_within_patience() {
    assert!(!should_stop_iterating(&[60.0, 70.0, 80.0], 95.0, 5));
}

// ---------------------------------------------------------------------------
// Node failure handling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn failed_node_skips_dependents() {
    let engine = WorkflowEngine::with_executor(
        PathBuf::from("/tmp"),
        HydraConfig::default(),
        FailOnNode {
            fail_id: "build".into(),
        },
    );

    let wf = builder_reviewer_refiner("claude", "codex", "claude", "task");
    let result = engine.execute(&wf).await.unwrap();

    // Build failed => review, refine, score should be skipped.
    let build_r = result
        .node_results
        .iter()
        .find(|r| r.node_id == "build")
        .unwrap();
    assert_eq!(build_r.status, NodeStatus::Failed);

    let review_r = result
        .node_results
        .iter()
        .find(|r| r.node_id == "review")
        .unwrap();
    assert_eq!(review_r.status, NodeStatus::Skipped);

    let refine_r = result
        .node_results
        .iter()
        .find(|r| r.node_id == "refine")
        .unwrap();
    assert_eq!(refine_r.status, NodeStatus::Skipped);

    let score_r = result
        .node_results
        .iter()
        .find(|r| r.node_id == "score")
        .unwrap();
    assert_eq!(score_r.status, NodeStatus::Skipped);

    // Overall status is Failed.
    assert_eq!(result.status, WorkflowStatus::Failed);
}

#[tokio::test]
async fn partial_failure_in_specialization() {
    // Fail one scope but succeed on the other.
    let engine = WorkflowEngine::with_executor(
        PathBuf::from("/tmp"),
        HydraConfig::default(),
        FailOnNode {
            fail_id: "scope-backend".into(),
        },
    );

    let wf = specialization(
        vec![
            ("frontend".into(), "claude".into(), "build UI".into()),
            ("backend".into(), "codex".into(), "build API".into()),
        ],
        Some("claude".into()),
    );

    let result = engine.execute(&wf).await.unwrap();

    // Frontend succeeds, backend fails, integrate is skipped (depends on backend).
    let fe_r = result
        .node_results
        .iter()
        .find(|r| r.node_id == "scope-frontend")
        .unwrap();
    assert_eq!(fe_r.status, NodeStatus::Completed);

    let be_r = result
        .node_results
        .iter()
        .find(|r| r.node_id == "scope-backend")
        .unwrap();
    assert_eq!(be_r.status, NodeStatus::Failed);

    let int_r = result
        .node_results
        .iter()
        .find(|r| r.node_id == "integrate")
        .unwrap();
    assert_eq!(int_r.status, NodeStatus::Skipped);

    // Overall is PartialFailure (not all failed).
    assert_eq!(result.status, WorkflowStatus::PartialFailure);
}

// ---------------------------------------------------------------------------
// Workflow status computation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn all_completed_gives_completed_status() {
    let engine = default_engine();
    let wf = builder_reviewer_refiner("claude", "claude", "claude", "task");
    let result = engine.execute(&wf).await.unwrap();
    assert_eq!(result.status, WorkflowStatus::Completed);
}

#[tokio::test]
async fn all_failed_gives_failed_status() {
    let engine = WorkflowEngine::with_executor(
        PathBuf::from("/tmp"),
        HydraConfig::default(),
        FailOnNode {
            fail_id: "build".into(),
        },
    );

    let wf = builder_reviewer_refiner("claude", "claude", "claude", "task");
    let result = engine.execute(&wf).await.unwrap();
    assert_eq!(result.status, WorkflowStatus::Failed);
}

// ---------------------------------------------------------------------------
// Artifact passing between nodes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn artifacts_propagated_through_workflow() {
    let engine = default_engine();
    let wf = builder_reviewer_refiner("claude", "codex", "claude", "task");
    let result = engine.execute(&wf).await.unwrap();

    // Build and review nodes should have artifact IDs.
    let build_r = result
        .node_results
        .iter()
        .find(|r| r.node_id == "build")
        .unwrap();
    assert!(build_r.artifact_id.is_some());

    let review_r = result
        .node_results
        .iter()
        .find(|r| r.node_id == "review")
        .unwrap();
    assert!(review_r.artifact_id.is_some());

    // Score node has no artifact.
    let score_r = result
        .node_results
        .iter()
        .find(|r| r.node_id == "score")
        .unwrap();
    assert!(score_r.artifact_id.is_none());
}

// ---------------------------------------------------------------------------
// Retry behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
async fn node_retries_on_failure() {
    let engine = WorkflowEngine::with_executor(
        PathBuf::from("/tmp"),
        HydraConfig::default(),
        RetryExecutor {
            fail_count: std::sync::atomic::AtomicU32::new(0),
            max_failures: 2,
        },
    );

    // Single node with max_retries = 3 (should succeed after 2 failures).
    let wf = WorkflowDefinition {
        name: "retry-test".into(),
        description: "test retries".into(),
        nodes: vec![WorkflowNode {
            id: "build".into(),
            node_type: NodeType::Build,
            agent_key: Some("claude".into()),
            prompt_template: "build".into(),
            depends_on: vec![],
            timeout_seconds: Some(60),
            max_retries: 3,
        }],
    };

    let result = engine.execute(&wf).await.unwrap();
    assert_eq!(result.status, WorkflowStatus::Completed);

    let build_r = result
        .node_results
        .iter()
        .find(|r| r.node_id == "build")
        .unwrap();
    assert_eq!(build_r.status, NodeStatus::Completed);
    assert_eq!(build_r.retries_used, 2);
}

// ---------------------------------------------------------------------------
// Serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn workflow_definition_serialization() {
    let wf = builder_reviewer_refiner("claude", "codex", "claude", "test prompt");
    let json = serde_json::to_string_pretty(&wf).expect("serialize");
    let deserialized: WorkflowDefinition = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.name, wf.name);
    assert_eq!(deserialized.nodes.len(), wf.nodes.len());
    assert_eq!(deserialized.nodes[0].id, "build");
}

#[tokio::test]
async fn workflow_result_serialization() {
    let engine = default_engine();
    let wf = builder_reviewer_refiner("claude", "codex", "claude", "task");
    let result = engine.execute(&wf).await.unwrap();

    let json = serde_json::to_string_pretty(&result).expect("serialize result");
    let deserialized: hydra_core::workflow::WorkflowResult =
        serde_json::from_str(&json).expect("deserialize result");

    assert_eq!(deserialized.workflow_name, result.workflow_name);
    assert_eq!(deserialized.status, result.status);
    assert_eq!(deserialized.node_results.len(), result.node_results.len());
}
