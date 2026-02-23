use std::collections::HashMap;

use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::single::{
    AgentCommand, ProcessStatus, ProcessSupervisor, SupervisorConfig, SupervisorEvent,
    SupervisorHandle, SupervisorResult,
};
use crate::{HydraError, Result};

/// Events from all agents, tagged with agent key.
#[derive(Debug, Clone)]
pub struct TaggedEvent {
    pub agent_key: String,
    pub event: SupervisorEvent,
}

/// Aggregated result from parallel execution.
#[derive(Debug)]
pub struct ParallelResult {
    pub results: HashMap<String, SupervisorResult>,
    pub all_completed: bool,
    pub failed_agents: Vec<String>,
}

/// Handle for cancelling all or individual agents.
pub struct ParallelHandle {
    handles: HashMap<String, SupervisorHandle>,
}

impl ParallelHandle {
    /// Cancel a specific agent.
    pub fn cancel_agent(&mut self, agent_key: &str) {
        if let Some(handle) = self.handles.remove(agent_key) {
            debug!(agent_key, "cancelling agent");
            handle.cancel();
        } else {
            warn!(
                agent_key,
                "no handle found for agent (already cancelled or completed)"
            );
        }
    }

    /// Cancel all agents.
    pub fn cancel_all(self) {
        for (key, handle) in self.handles {
            debug!(agent_key = %key, "cancelling agent");
            handle.cancel();
        }
    }
}

/// Manages multiple concurrent agent processes.
pub struct ParallelSupervisor {
    configs: HashMap<String, SupervisorConfig>,
}

impl Default for ParallelSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelSupervisor {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    /// Add an agent to supervise.
    pub fn add_agent(&mut self, config: SupervisorConfig) {
        let key = config.agent_key.clone();
        self.configs.insert(key, config);
    }

    /// Spawn all agents concurrently, returning a merged event stream and a handle.
    pub async fn spawn_all(
        &self,
        commands: HashMap<String, AgentCommand>,
    ) -> Result<(mpsc::Receiver<TaggedEvent>, ParallelHandle)> {
        if self.configs.is_empty() {
            return Err(HydraError::Process(
                "no agents configured in parallel supervisor".to_string(),
            ));
        }

        let (merged_tx, merged_rx) = mpsc::channel::<TaggedEvent>(256);
        let mut handles = HashMap::new();

        for (key, config) in &self.configs {
            let command = commands.get(key).ok_or_else(|| {
                HydraError::Process(format!("no command provided for agent '{key}'"))
            })?;

            let supervisor = ProcessSupervisor::new(config.clone());
            let (mut agent_rx, agent_handle) = supervisor.spawn(command.clone()).await?;

            handles.insert(key.clone(), agent_handle);

            let agent_key = key.clone();
            let tx = merged_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = agent_rx.recv().await {
                    let tagged = TaggedEvent {
                        agent_key: agent_key.clone(),
                        event,
                    };
                    if tx.send(tagged).await.is_err() {
                        break;
                    }
                }
            });
        }

        // Drop our copy of the sender so the receiver closes when all forwarders complete.
        drop(merged_tx);

        info!(
            agent_count = self.configs.len(),
            "spawned all agents in parallel"
        );

        Ok((merged_rx, ParallelHandle { handles }))
    }

    /// Run all agents to completion, returning aggregated results.
    pub async fn run_all_to_completion(
        &self,
        commands: HashMap<String, AgentCommand>,
    ) -> Result<ParallelResult> {
        if self.configs.is_empty() {
            return Err(HydraError::Process(
                "no agents configured in parallel supervisor".to_string(),
            ));
        }

        // Spawn each agent independently and collect futures.
        let mut join_handles = Vec::new();

        for (key, config) in &self.configs {
            let command = commands.get(key).ok_or_else(|| {
                HydraError::Process(format!("no command provided for agent '{key}'"))
            })?;

            let supervisor = ProcessSupervisor::new(config.clone());
            let cmd = command.clone();
            let agent_key = key.clone();

            let jh = tokio::spawn(async move {
                let result = supervisor.run_to_completion(cmd).await;
                (agent_key, result)
            });

            join_handles.push(jh);
        }

        let mut results = HashMap::new();
        let mut failed_agents = Vec::new();

        for jh in join_handles {
            let (agent_key, result) = jh
                .await
                .map_err(|e| HydraError::Process(format!("agent task panicked: {e}")))?;

            match result {
                Ok(supervisor_result) => {
                    if supervisor_result.status != ProcessStatus::Completed {
                        failed_agents.push(agent_key.clone());
                    }
                    results.insert(agent_key, supervisor_result);
                }
                Err(e) => {
                    warn!(agent_key = %agent_key, error = %e, "agent failed to execute");
                    failed_agents.push(agent_key);
                }
            }
        }

        let all_completed = failed_agents.is_empty() && results.len() == self.configs.len();

        info!(
            total = self.configs.len(),
            completed = results.len(),
            failed = failed_agents.len(),
            "parallel execution finished"
        );

        Ok(ParallelResult {
            results,
            all_completed,
            failed_agents,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use uuid::Uuid;

    fn test_config(agent_key: &str) -> SupervisorConfig {
        SupervisorConfig {
            run_id: Uuid::new_v4(),
            agent_key: agent_key.to_string(),
            idle_timeout: Duration::from_secs(10),
            hard_timeout: Duration::from_secs(30),
            max_output_bytes: 1024 * 1024,
        }
    }

    fn echo_command(msg: &str) -> AgentCommand {
        AgentCommand {
            program: "echo".to_string(),
            args: vec![msg.to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
        }
    }

    fn sleep_command(secs: &str) -> AgentCommand {
        AgentCommand {
            program: "sleep".to_string(),
            args: vec![secs.to_string()],
            env: vec![],
            cwd: std::env::temp_dir(),
        }
    }

    fn failing_command(exit_code: i32) -> AgentCommand {
        AgentCommand {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), format!("exit {exit_code}")],
            env: vec![],
            cwd: std::env::temp_dir(),
        }
    }

    #[tokio::test]
    async fn two_agents_run_concurrently_and_complete() {
        let mut ps = ParallelSupervisor::new();
        ps.add_agent(test_config("agent-a"));
        ps.add_agent(test_config("agent-b"));

        let mut commands = HashMap::new();
        commands.insert("agent-a".to_string(), echo_command("hello-a"));
        commands.insert("agent-b".to_string(), echo_command("hello-b"));

        let result = ps.run_all_to_completion(commands).await.unwrap();

        assert!(result.all_completed);
        assert!(result.failed_agents.is_empty());
        assert_eq!(result.results.len(), 2);

        let a = &result.results["agent-a"];
        assert_eq!(a.status, ProcessStatus::Completed);
        assert!(a.stdout_lines.contains(&"hello-a".to_string()));

        let b = &result.results["agent-b"];
        assert_eq!(b.status, ProcessStatus::Completed);
        assert!(b.stdout_lines.contains(&"hello-b".to_string()));
    }

    #[tokio::test]
    async fn one_failure_does_not_kill_the_other() {
        let mut ps = ParallelSupervisor::new();
        ps.add_agent(test_config("good-agent"));
        ps.add_agent(test_config("bad-agent"));

        let mut commands = HashMap::new();
        commands.insert("good-agent".to_string(), echo_command("success"));
        commands.insert("bad-agent".to_string(), failing_command(1));

        let result = ps.run_all_to_completion(commands).await.unwrap();

        assert!(!result.all_completed);
        assert!(result.failed_agents.contains(&"bad-agent".to_string()));

        let good = &result.results["good-agent"];
        assert_eq!(good.status, ProcessStatus::Completed);

        let bad = &result.results["bad-agent"];
        assert_eq!(bad.status, ProcessStatus::Failed);
    }

    #[tokio::test]
    async fn cancel_individual_agent() {
        let mut ps = ParallelSupervisor::new();
        let mut config_a = test_config("cancel-me");
        config_a.hard_timeout = Duration::from_secs(60);
        config_a.idle_timeout = Duration::from_secs(60);
        ps.add_agent(config_a);

        let mut config_b = test_config("keep-running");
        config_b.hard_timeout = Duration::from_secs(60);
        config_b.idle_timeout = Duration::from_secs(60);
        ps.add_agent(config_b);

        let mut commands = HashMap::new();
        commands.insert("cancel-me".to_string(), sleep_command("999"));
        commands.insert("keep-running".to_string(), echo_command("done"));

        let (mut rx, mut handle) = ps.spawn_all(commands).await.unwrap();

        // Wait for at least one Started event from "cancel-me"
        let mut saw_started = false;
        while !saw_started {
            if let Some(evt) = rx.recv().await {
                if evt.agent_key == "cancel-me"
                    && matches!(evt.event, SupervisorEvent::Started { .. })
                {
                    saw_started = true;
                }
            }
        }

        // Cancel just the one agent
        handle.cancel_agent("cancel-me");

        // Drain events and verify we get a Cancelled for cancel-me
        let mut saw_cancelled = false;
        while let Some(evt) = rx.recv().await {
            if evt.agent_key == "cancel-me"
                && matches!(evt.event, SupervisorEvent::Cancelled { .. })
            {
                saw_cancelled = true;
            }
        }
        assert!(saw_cancelled, "expected Cancelled event for cancel-me");
    }

    #[tokio::test]
    async fn cancel_all_agents() {
        let mut ps = ParallelSupervisor::new();
        let mut config_a = test_config("agent-1");
        config_a.hard_timeout = Duration::from_secs(60);
        config_a.idle_timeout = Duration::from_secs(60);
        ps.add_agent(config_a);

        let mut config_b = test_config("agent-2");
        config_b.hard_timeout = Duration::from_secs(60);
        config_b.idle_timeout = Duration::from_secs(60);
        ps.add_agent(config_b);

        let mut commands = HashMap::new();
        commands.insert("agent-1".to_string(), sleep_command("999"));
        commands.insert("agent-2".to_string(), sleep_command("999"));

        let (mut rx, handle) = ps.spawn_all(commands).await.unwrap();

        // Wait for both to start
        let mut started_count = 0;
        while started_count < 2 {
            if let Some(evt) = rx.recv().await {
                if matches!(evt.event, SupervisorEvent::Started { .. }) {
                    started_count += 1;
                }
            }
        }

        // Cancel all
        handle.cancel_all();

        // Drain and count cancellations
        let mut cancelled = std::collections::HashSet::new();
        while let Some(evt) = rx.recv().await {
            if matches!(evt.event, SupervisorEvent::Cancelled { .. }) {
                cancelled.insert(evt.agent_key);
            }
        }
        assert_eq!(cancelled.len(), 2);
        assert!(cancelled.contains("agent-1"));
        assert!(cancelled.contains("agent-2"));
    }

    #[tokio::test]
    async fn merged_event_stream_contains_events_from_all_agents() {
        let mut ps = ParallelSupervisor::new();
        ps.add_agent(test_config("stream-a"));
        ps.add_agent(test_config("stream-b"));

        let mut commands = HashMap::new();
        commands.insert("stream-a".to_string(), echo_command("from-a"));
        commands.insert("stream-b".to_string(), echo_command("from-b"));

        let (mut rx, _handle) = ps.spawn_all(commands).await.unwrap();

        let mut agent_keys = std::collections::HashSet::new();
        while let Some(evt) = rx.recv().await {
            agent_keys.insert(evt.agent_key);
        }

        assert!(agent_keys.contains("stream-a"));
        assert!(agent_keys.contains("stream-b"));
    }

    #[tokio::test]
    async fn aggregate_status_correct_mixed() {
        let mut ps = ParallelSupervisor::new();
        ps.add_agent(test_config("ok-agent"));

        let mut timeout_config = test_config("timeout-agent");
        timeout_config.hard_timeout = Duration::from_millis(200);
        timeout_config.idle_timeout = Duration::from_secs(60);
        ps.add_agent(timeout_config);

        let mut commands = HashMap::new();
        commands.insert("ok-agent".to_string(), echo_command("hi"));
        commands.insert("timeout-agent".to_string(), sleep_command("999"));

        let result = ps.run_all_to_completion(commands).await.unwrap();

        assert!(!result.all_completed);
        assert!(result.failed_agents.contains(&"timeout-agent".to_string()));
        assert!(!result.failed_agents.contains(&"ok-agent".to_string()));
    }

    #[tokio::test]
    async fn empty_supervisor_errors() {
        let ps = ParallelSupervisor::new();
        let commands = HashMap::new();

        let result = ps.run_all_to_completion(commands).await;
        assert!(result.is_err());
    }
}
