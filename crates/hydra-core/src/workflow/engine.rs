//! Workflow engine: DAG-based execution of multi-agent collaboration workflows.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::HydraConfig;
use crate::{HydraError, Result};

// ---------------------------------------------------------------------------
// Workflow definition types
// ---------------------------------------------------------------------------

/// A node in the workflow DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub node_type: NodeType,
    pub agent_key: Option<String>,
    pub prompt_template: String,
    pub depends_on: Vec<String>,
    pub timeout_seconds: Option<u64>,
    pub max_retries: u32,
}

/// The kind of operation a node performs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Build,
    Review,
    Refine,
    Score,
    Merge,
    Custom,
}

/// Workflow definition as a DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub description: String,
    pub nodes: Vec<WorkflowNode>,
}

// ---------------------------------------------------------------------------
// Execution result types
// ---------------------------------------------------------------------------

/// Result of executing a single node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    pub node_id: String,
    pub status: NodeStatus,
    pub artifact_id: Option<Uuid>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub retries_used: u32,
}

/// Status of a workflow node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Retrying,
}

/// Result of a complete workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub workflow_name: String,
    pub run_id: Uuid,
    pub status: WorkflowStatus,
    pub node_results: Vec<NodeResult>,
    pub final_score: Option<f64>,
    pub duration_ms: u64,
}

/// Overall status of a workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Completed,
    PartialFailure,
    Failed,
    Cancelled,
}

// ---------------------------------------------------------------------------
// Workflow context (shared between nodes)
// ---------------------------------------------------------------------------

/// Context passed between nodes during execution.
#[derive(Debug, Clone, Default)]
pub struct WorkflowContext {
    pub run_id: Uuid,
    pub artifacts: HashMap<String, Uuid>,
    pub node_outputs: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Node executor trait
// ---------------------------------------------------------------------------

/// Trait for executing individual workflow nodes.
///
/// The default implementation is a no-op that simulates execution.
/// Real adapter-backed execution is plugged in via this trait.
pub trait NodeExecutor: Send + Sync {
    fn execute(
        &self,
        node: &WorkflowNode,
        context: &WorkflowContext,
    ) -> impl std::future::Future<Output = NodeResult> + Send;
}

/// Default executor that simulates node execution (for testing and dry-runs).
pub struct SimulatedExecutor;

impl NodeExecutor for SimulatedExecutor {
    async fn execute(&self, node: &WorkflowNode, _context: &WorkflowContext) -> NodeResult {
        let start = Instant::now();

        // Score nodes produce a numeric result.
        let output = match node.node_type {
            NodeType::Score => Some("85.0".to_string()),
            _ => Some(format!("simulated output for node '{}'", node.id)),
        };

        let artifact_id = if node.node_type != NodeType::Score {
            Some(Uuid::new_v4())
        } else {
            None
        };

        NodeResult {
            node_id: node.id.clone(),
            status: NodeStatus::Completed,
            artifact_id,
            output,
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
            retries_used: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Workflow engine
// ---------------------------------------------------------------------------

/// Workflow engine executes DAG-based workflows.
pub struct WorkflowEngine<E: NodeExecutor = SimulatedExecutor> {
    repo_root: PathBuf,
    config: HydraConfig,
    executor: E,
}

impl WorkflowEngine<SimulatedExecutor> {
    pub fn new(repo_root: PathBuf, config: HydraConfig) -> Self {
        Self {
            repo_root,
            config,
            executor: SimulatedExecutor,
        }
    }
}

impl<E: NodeExecutor> WorkflowEngine<E> {
    /// Create a workflow engine with a custom node executor.
    pub fn with_executor(repo_root: PathBuf, config: HydraConfig, executor: E) -> Self {
        Self {
            repo_root,
            config,
            executor,
        }
    }

    /// Return the repo root path.
    pub fn repo_root(&self) -> &PathBuf {
        &self.repo_root
    }

    /// Return a reference to the config.
    pub fn config(&self) -> &HydraConfig {
        &self.config
    }

    /// Execute a workflow definition.
    pub async fn execute(&self, workflow: &WorkflowDefinition) -> Result<WorkflowResult> {
        let run_id = Uuid::new_v4();
        let start = Instant::now();

        info!(
            %run_id,
            workflow = %workflow.name,
            node_count = workflow.nodes.len(),
            "starting workflow execution"
        );

        // Resolve execution order via topological sort.
        let levels = self.resolve_order(workflow)?;

        let mut context = WorkflowContext {
            run_id,
            ..Default::default()
        };

        let mut all_results: Vec<NodeResult> = Vec::new();
        let mut failed_nodes: HashMap<String, bool> = HashMap::new();

        // Build a node lookup for quick access.
        let node_map: HashMap<&str, &WorkflowNode> =
            workflow.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

        for (level_idx, level) in levels.iter().enumerate() {
            debug!(level = level_idx, nodes = ?level, "executing workflow level");

            // Determine which nodes in this level can actually run.
            let mut runnable = Vec::new();
            let mut skipped = Vec::new();

            for node_id in level {
                let node = node_map.get(node_id.as_str()).ok_or_else(|| {
                    HydraError::Config(format!("workflow node '{node_id}' not found in definition"))
                })?;

                // Check if any dependency failed.
                let dep_failed = node
                    .depends_on
                    .iter()
                    .any(|dep| failed_nodes.contains_key(dep));

                if dep_failed {
                    skipped.push(node_id.clone());
                } else {
                    runnable.push(*node);
                }
            }

            // Record skipped nodes.
            for node_id in &skipped {
                warn!(node_id, "skipping node due to failed dependency");
                failed_nodes.insert(node_id.clone(), true);
                all_results.push(NodeResult {
                    node_id: node_id.clone(),
                    status: NodeStatus::Skipped,
                    artifact_id: None,
                    output: None,
                    error: Some("skipped: upstream dependency failed".into()),
                    duration_ms: 0,
                    retries_used: 0,
                });
            }

            // Execute runnable nodes (sequentially for now; parallel nodes
            // at the same level are handled via tokio::join in execute_level).
            let results = self.execute_level(runnable, &context).await;

            // Process results: update context, track failures.
            for result in results {
                if result.status == NodeStatus::Completed {
                    if let Some(ref output) = result.output {
                        context
                            .node_outputs
                            .insert(result.node_id.clone(), output.clone());
                    }
                    if let Some(artifact_id) = result.artifact_id {
                        context
                            .artifacts
                            .insert(result.node_id.clone(), artifact_id);
                    }
                } else {
                    failed_nodes.insert(result.node_id.clone(), true);
                }

                all_results.push(result);
            }
        }

        // Compute workflow status.
        let has_failures = all_results.iter().any(|r| r.status == NodeStatus::Failed);
        let has_skips = all_results.iter().any(|r| r.status == NodeStatus::Skipped);
        let all_failed = all_results
            .iter()
            .all(|r| r.status == NodeStatus::Failed || r.status == NodeStatus::Skipped);

        let status = if all_failed {
            WorkflowStatus::Failed
        } else if has_failures || has_skips {
            WorkflowStatus::PartialFailure
        } else {
            WorkflowStatus::Completed
        };

        // Extract final score from last Score node if available.
        let final_score = all_results
            .iter()
            .rev()
            .find(|r| {
                node_map
                    .get(r.node_id.as_str())
                    .is_some_and(|n| n.node_type == NodeType::Score)
                    && r.status == NodeStatus::Completed
            })
            .and_then(|r| r.output.as_ref())
            .and_then(|o| o.parse::<f64>().ok());

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            %run_id,
            workflow = %workflow.name,
            ?status,
            ?final_score,
            duration_ms,
            "workflow execution complete"
        );

        Ok(WorkflowResult {
            workflow_name: workflow.name.clone(),
            run_id,
            status,
            node_results: all_results,
            final_score,
            duration_ms,
        })
    }

    /// Execute a level of independent nodes.
    ///
    /// Nodes at the same level have no inter-dependencies and are executed
    /// sequentially here (parallel tokio::spawn would require `E: 'static`).
    async fn execute_level(
        &self,
        nodes: Vec<&WorkflowNode>,
        context: &WorkflowContext,
    ) -> Vec<NodeResult> {
        let mut results = Vec::with_capacity(nodes.len());

        for node in nodes {
            let result = self.execute_node(node, context).await;
            results.push(result);
        }

        results
    }

    /// Execute a single node with timeout and retries.
    async fn execute_node(&self, node: &WorkflowNode, context: &WorkflowContext) -> NodeResult {
        let timeout_secs = node
            .timeout_seconds
            .unwrap_or(self.config.general.default_timeout_seconds);
        let max_retries = node.max_retries;
        let start = Instant::now();
        let mut retries_used = 0;

        loop {
            let timeout = tokio::time::Duration::from_secs(timeout_secs);
            let result = tokio::time::timeout(timeout, self.executor.execute(node, context)).await;

            match result {
                Ok(mut node_result) => {
                    node_result.retries_used = retries_used;

                    if node_result.status == NodeStatus::Completed || retries_used >= max_retries {
                        node_result.duration_ms = start.elapsed().as_millis() as u64;
                        return node_result;
                    }

                    retries_used += 1;
                    debug!(
                        node_id = %node.id,
                        retries_used,
                        max_retries,
                        "retrying failed node"
                    );
                }
                Err(_) => {
                    warn!(node_id = %node.id, timeout_secs, "node execution timed out");

                    if retries_used >= max_retries {
                        return NodeResult {
                            node_id: node.id.clone(),
                            status: NodeStatus::Failed,
                            artifact_id: None,
                            output: None,
                            error: Some(format!(
                                "timed out after {timeout_secs}s (retries: {retries_used}/{max_retries})"
                            )),
                            duration_ms: start.elapsed().as_millis() as u64,
                            retries_used,
                        };
                    }

                    retries_used += 1;
                }
            }
        }
    }

    /// Resolve execution order from DAG via topological sort.
    ///
    /// Returns levels where each level contains nodes that can execute in parallel.
    /// Nodes in level N+1 depend only on nodes in levels 0..N.
    pub fn resolve_order(&self, workflow: &WorkflowDefinition) -> Result<Vec<Vec<String>>> {
        let node_ids: Vec<&str> = workflow.nodes.iter().map(|n| n.id.as_str()).collect();

        // Build adjacency: node -> set of dependents.
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

        for id in &node_ids {
            in_degree.insert(id, 0);
            dependents.insert(id, Vec::new());
        }

        for node in &workflow.nodes {
            for dep in &node.depends_on {
                if !in_degree.contains_key(dep.as_str()) {
                    return Err(HydraError::Config(format!(
                        "workflow node '{}' depends on unknown node '{dep}'",
                        node.id
                    )));
                }
                *in_degree.get_mut(node.id.as_str()).unwrap() += 1;
                dependents.get_mut(dep.as_str()).unwrap().push(&node.id);
            }
        }

        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        // Sort for deterministic ordering.
        queue.sort();

        let mut processed = 0;

        while !queue.is_empty() {
            let current_level: Vec<String> = queue.iter().map(|s| s.to_string()).collect();
            let mut next_queue: Vec<&str> = Vec::new();

            for node_id in &queue {
                processed += 1;
                if let Some(deps) = dependents.get(node_id) {
                    for dep in deps {
                        let deg = in_degree.get_mut(dep).unwrap();
                        *deg -= 1;
                        if *deg == 0 {
                            next_queue.push(dep);
                        }
                    }
                }
            }

            levels.push(current_level);
            next_queue.sort();
            queue = next_queue;
        }

        if processed != node_ids.len() {
            return Err(HydraError::Config(
                "workflow contains a dependency cycle".into(),
            ));
        }

        Ok(levels)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_workflow() -> WorkflowDefinition {
        WorkflowDefinition {
            name: "test-workflow".into(),
            description: "A test workflow".into(),
            nodes: vec![
                WorkflowNode {
                    id: "a".into(),
                    node_type: NodeType::Build,
                    agent_key: Some("claude".into()),
                    prompt_template: "build something".into(),
                    depends_on: vec![],
                    timeout_seconds: Some(60),
                    max_retries: 0,
                },
                WorkflowNode {
                    id: "b".into(),
                    node_type: NodeType::Review,
                    agent_key: Some("codex".into()),
                    prompt_template: "review it".into(),
                    depends_on: vec!["a".into()],
                    timeout_seconds: Some(60),
                    max_retries: 0,
                },
                WorkflowNode {
                    id: "c".into(),
                    node_type: NodeType::Score,
                    agent_key: None,
                    prompt_template: String::new(),
                    depends_on: vec!["b".into()],
                    timeout_seconds: Some(60),
                    max_retries: 0,
                },
            ],
        }
    }

    fn parallel_workflow() -> WorkflowDefinition {
        WorkflowDefinition {
            name: "parallel-test".into(),
            description: "Nodes b and c run in parallel".into(),
            nodes: vec![
                WorkflowNode {
                    id: "a".into(),
                    node_type: NodeType::Build,
                    agent_key: Some("claude".into()),
                    prompt_template: "start".into(),
                    depends_on: vec![],
                    timeout_seconds: None,
                    max_retries: 0,
                },
                WorkflowNode {
                    id: "b".into(),
                    node_type: NodeType::Review,
                    agent_key: Some("claude".into()),
                    prompt_template: "review b".into(),
                    depends_on: vec!["a".into()],
                    timeout_seconds: None,
                    max_retries: 0,
                },
                WorkflowNode {
                    id: "c".into(),
                    node_type: NodeType::Review,
                    agent_key: Some("codex".into()),
                    prompt_template: "review c".into(),
                    depends_on: vec!["a".into()],
                    timeout_seconds: None,
                    max_retries: 0,
                },
                WorkflowNode {
                    id: "d".into(),
                    node_type: NodeType::Merge,
                    agent_key: None,
                    prompt_template: "merge".into(),
                    depends_on: vec!["b".into(), "c".into()],
                    timeout_seconds: None,
                    max_retries: 0,
                },
            ],
        }
    }

    #[test]
    fn topo_sort_linear() {
        let engine = WorkflowEngine::new(PathBuf::from("/tmp"), HydraConfig::default());
        let wf = simple_workflow();
        let levels = engine.resolve_order(&wf).unwrap();

        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec!["a"]);
        assert_eq!(levels[1], vec!["b"]);
        assert_eq!(levels[2], vec!["c"]);
    }

    #[test]
    fn topo_sort_parallel() {
        let engine = WorkflowEngine::new(PathBuf::from("/tmp"), HydraConfig::default());
        let wf = parallel_workflow();
        let levels = engine.resolve_order(&wf).unwrap();

        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec!["a"]);
        assert_eq!(levels[1], vec!["b", "c"]);
        assert_eq!(levels[2], vec!["d"]);
    }

    #[test]
    fn topo_sort_cycle_detection() {
        let engine = WorkflowEngine::new(PathBuf::from("/tmp"), HydraConfig::default());
        let wf = WorkflowDefinition {
            name: "cycle".into(),
            description: "has a cycle".into(),
            nodes: vec![
                WorkflowNode {
                    id: "a".into(),
                    node_type: NodeType::Build,
                    agent_key: None,
                    prompt_template: String::new(),
                    depends_on: vec!["b".into()],
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
            ],
        };

        let err = engine.resolve_order(&wf).unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }

    #[test]
    fn topo_sort_unknown_dependency() {
        let engine = WorkflowEngine::new(PathBuf::from("/tmp"), HydraConfig::default());
        let wf = WorkflowDefinition {
            name: "bad-dep".into(),
            description: "references unknown node".into(),
            nodes: vec![WorkflowNode {
                id: "a".into(),
                node_type: NodeType::Build,
                agent_key: None,
                prompt_template: String::new(),
                depends_on: vec!["nonexistent".into()],
                timeout_seconds: None,
                max_retries: 0,
            }],
        };

        let err = engine.resolve_order(&wf).unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[tokio::test]
    async fn execute_simple_workflow() {
        let engine = WorkflowEngine::new(PathBuf::from("/tmp"), HydraConfig::default());
        let wf = simple_workflow();
        let result = engine.execute(&wf).await.unwrap();

        assert_eq!(result.workflow_name, "test-workflow");
        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.node_results.len(), 3);

        for nr in &result.node_results {
            assert_eq!(
                nr.status,
                NodeStatus::Completed,
                "node {} failed",
                nr.node_id
            );
        }

        assert!(result.final_score.is_some());
    }

    #[tokio::test]
    async fn execute_parallel_workflow() {
        let engine = WorkflowEngine::new(PathBuf::from("/tmp"), HydraConfig::default());
        let wf = parallel_workflow();
        let result = engine.execute(&wf).await.unwrap();

        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.node_results.len(), 4);
    }

    #[tokio::test]
    async fn node_failure_skips_dependents() {
        struct FailingExecutor {
            fail_node: String,
        }

        impl NodeExecutor for FailingExecutor {
            async fn execute(&self, node: &WorkflowNode, _context: &WorkflowContext) -> NodeResult {
                if node.id == self.fail_node {
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
                    NodeResult {
                        node_id: node.id.clone(),
                        status: NodeStatus::Completed,
                        artifact_id: Some(Uuid::new_v4()),
                        output: Some("ok".into()),
                        error: None,
                        duration_ms: 0,
                        retries_used: 0,
                    }
                }
            }
        }

        let engine = WorkflowEngine::with_executor(
            PathBuf::from("/tmp"),
            HydraConfig::default(),
            FailingExecutor {
                fail_node: "a".into(),
            },
        );

        let wf = simple_workflow();
        let result = engine.execute(&wf).await.unwrap();

        assert_eq!(result.status, WorkflowStatus::Failed);

        let a_result = result
            .node_results
            .iter()
            .find(|r| r.node_id == "a")
            .unwrap();
        assert_eq!(a_result.status, NodeStatus::Failed);

        let b_result = result
            .node_results
            .iter()
            .find(|r| r.node_id == "b")
            .unwrap();
        assert_eq!(b_result.status, NodeStatus::Skipped);

        let c_result = result
            .node_results
            .iter()
            .find(|r| r.node_id == "c")
            .unwrap();
        assert_eq!(c_result.status, NodeStatus::Skipped);
    }

    #[tokio::test]
    async fn artifact_passing_between_nodes() {
        let engine = WorkflowEngine::new(PathBuf::from("/tmp"), HydraConfig::default());
        let wf = simple_workflow();
        let result = engine.execute(&wf).await.unwrap();

        let a_result = result
            .node_results
            .iter()
            .find(|r| r.node_id == "a")
            .unwrap();
        assert!(a_result.artifact_id.is_some());

        let b_result = result
            .node_results
            .iter()
            .find(|r| r.node_id == "b")
            .unwrap();
        assert!(b_result.artifact_id.is_some());

        // Score node does not produce artifacts.
        let c_result = result
            .node_results
            .iter()
            .find(|r| r.node_id == "c")
            .unwrap();
        assert!(c_result.artifact_id.is_none());
    }
}
