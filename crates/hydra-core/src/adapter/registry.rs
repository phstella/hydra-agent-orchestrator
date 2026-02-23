use tracing::info;

use super::claude::{ClaudeAdapter, ClaudeProbe};
use super::codex::{CodexAdapter, CodexProbe};
use super::cursor::{CursorAdapter, CursorProbe};
use super::probe::{
    AdapterProbe, AdapterTier, CommandRunner, ProbeResult, ProbeStatus, RealCommandRunner,
};
use super::runtime::AdapterRuntime;
use crate::{HydraError, Result};

/// Entry in the adapter registry.
pub struct RegisteredAdapter {
    pub key: String,
    pub tier: AdapterTier,
    pub probe_result: ProbeResult,
    /// The runtime implementation (available only if probe succeeded).
    runtime: Option<Box<dyn AdapterRuntime>>,
}

impl RegisteredAdapter {
    /// Whether this adapter is available for use in a run.
    pub fn is_available(&self) -> bool {
        matches!(
            self.probe_result.status,
            ProbeStatus::Ready | ProbeStatus::ExperimentalReady
        )
    }
}

/// Central registry managing adapter discovery and tier policy.
pub struct AdapterRegistry {
    adapters: Vec<RegisteredAdapter>,
    allow_experimental: bool,
}

impl AdapterRegistry {
    /// Create a registry with the default real command runner.
    pub fn new(allow_experimental: bool) -> Self {
        let runner = RealCommandRunner;
        Self::with_runner(allow_experimental, &runner)
    }

    /// Create a registry using the given command runner (useful for testing).
    pub fn with_runner(allow_experimental: bool, runner: &dyn CommandRunner) -> Self {
        let mut adapters = Vec::new();

        // Probe Claude (Tier 1)
        let claude_probe = ClaudeProbe::new(runner);
        let claude_result = claude_probe.probe();
        let claude_available = matches!(claude_result.status, ProbeStatus::Ready);
        info!(
            adapter = "claude",
            status = ?claude_result.status,
            "probed adapter"
        );
        adapters.push(RegisteredAdapter {
            key: "claude".to_string(),
            tier: AdapterTier::Tier1,
            probe_result: claude_result,
            runtime: if claude_available {
                Some(Box::new(ClaudeAdapter))
            } else {
                None
            },
        });

        // Probe Codex (Tier 1)
        let codex_probe = CodexProbe::new(runner);
        let codex_result = codex_probe.probe();
        let codex_available = matches!(codex_result.status, ProbeStatus::Ready);
        info!(
            adapter = "codex",
            status = ?codex_result.status,
            "probed adapter"
        );
        adapters.push(RegisteredAdapter {
            key: "codex".to_string(),
            tier: AdapterTier::Tier1,
            probe_result: codex_result,
            runtime: if codex_available {
                Some(Box::new(CodexAdapter))
            } else {
                None
            },
        });

        // Probe Cursor (Experimental)
        let cursor_probe = CursorProbe::new(runner);
        let cursor_result = cursor_probe.probe();
        let cursor_available = matches!(cursor_result.status, ProbeStatus::ExperimentalReady);
        info!(
            adapter = "cursor-agent",
            status = ?cursor_result.status,
            "probed adapter"
        );
        adapters.push(RegisteredAdapter {
            key: "cursor-agent".to_string(),
            tier: AdapterTier::Experimental,
            probe_result: cursor_result,
            runtime: if cursor_available {
                Some(Box::new(CursorAdapter))
            } else {
                None
            },
        });

        Self {
            adapters,
            allow_experimental,
        }
    }

    /// Get adapters available for a run (respecting tier policy).
    ///
    /// Returns only:
    /// - Tier-1 adapters with `Ready` status
    /// - Experimental adapters with `ExperimentalReady` status ONLY if `allow_experimental` is true
    pub fn available_adapters(&self) -> Vec<&RegisteredAdapter> {
        self.adapters
            .iter()
            .filter(|a| match a.tier {
                AdapterTier::Tier1 => a.probe_result.status == ProbeStatus::Ready,
                AdapterTier::Experimental => {
                    self.allow_experimental
                        && a.probe_result.status == ProbeStatus::ExperimentalReady
                }
            })
            .collect()
    }

    /// Get a specific adapter by key.
    pub fn get(&self, key: &str) -> Option<&RegisteredAdapter> {
        self.adapters.iter().find(|a| a.key == key)
    }

    /// Get the runtime for a specific adapter.
    pub fn get_runtime(&self, key: &str) -> Result<&dyn AdapterRuntime> {
        let adapter = self
            .get(key)
            .ok_or_else(|| HydraError::Adapter(format!("adapter not found: {key}")))?;

        // Check tier policy: experimental adapters blocked unless allowed
        if adapter.tier == AdapterTier::Experimental && !self.allow_experimental {
            return Err(HydraError::Adapter(format!(
                "adapter '{key}' is experimental and experimental adapters are not enabled"
            )));
        }

        adapter
            .runtime
            .as_deref()
            .ok_or_else(|| HydraError::Adapter(format!("adapter '{key}' is not available")))
    }

    /// List all registered adapters (including unavailable).
    pub fn all(&self) -> &[RegisteredAdapter] {
        &self.adapters
    }

    /// Check if all Tier-1 adapters are ready.
    pub fn tier1_ready(&self) -> bool {
        self.adapters
            .iter()
            .filter(|a| a.tier == AdapterTier::Tier1)
            .all(|a| a.probe_result.status == ProbeStatus::Ready)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::probe::CommandRunner;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};

    struct MockRunner {
        responses: std::collections::HashMap<String, std::io::Result<Output>>,
    }

    impl MockRunner {
        fn new() -> Self {
            Self {
                responses: std::collections::HashMap::new(),
            }
        }

        fn register(&mut self, cmd: &str, result: std::io::Result<Output>) {
            self.responses.insert(cmd.to_string(), result);
        }
    }

    impl CommandRunner for MockRunner {
        fn run(&self, program: &str, args: &[&str]) -> std::io::Result<Output> {
            let key = format!("{program} {}", args.join(" "));
            match self.responses.get(&key) {
                Some(Ok(output)) => Ok(output.clone()),
                Some(Err(_)) => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "mock error",
                )),
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("no mock for: {key}"),
                )),
            }
        }
    }

    fn success_output(stdout: &str) -> Output {
        Output {
            status: ExitStatus::from_raw(0),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    /// Create a mock runner where both Claude and Codex are ready, Cursor is ready.
    fn all_ready_mock() -> MockRunner {
        let mut mock = MockRunner::new();

        // Claude
        mock.register("which claude", Ok(success_output("/usr/bin/claude\n")));
        mock.register("claude --version", Ok(success_output("claude 1.0.0\n")));
        mock.register(
            "claude --help",
            Ok(success_output(
                "Usage: claude\n  -p, --print\n  --output-format\n  --resume\n",
            )),
        );

        // Codex
        mock.register("which codex", Ok(success_output("/usr/bin/codex\n")));
        mock.register("codex --version", Ok(success_output("codex 1.0.0\n")));
        mock.register(
            "codex exec --help",
            Ok(success_output("Usage: codex exec\n  --json\n  --sandbox\n")),
        );

        // Cursor
        mock.register(
            "which cursor-agent",
            Ok(success_output("/usr/bin/cursor-agent\n")),
        );
        mock.register(
            "cursor-agent --version",
            Ok(success_output("cursor-agent 0.3.0\n")),
        );
        mock.register(
            "cursor-agent --help",
            Ok(success_output("Usage: cursor-agent\n  --json\n")),
        );

        mock
    }

    /// Create a mock runner where no adapters are found.
    fn none_found_mock() -> MockRunner {
        MockRunner::new()
    }

    /// Create a mock runner where Claude is ready, Codex is blocked, Cursor is missing.
    fn mixed_mock() -> MockRunner {
        let mut mock = MockRunner::new();

        // Claude: ready
        mock.register("which claude", Ok(success_output("/usr/bin/claude\n")));
        mock.register("claude --version", Ok(success_output("claude 1.0.0\n")));
        mock.register(
            "claude --help",
            Ok(success_output(
                "Usage: claude\n  -p, --print\n  --output-format\n",
            )),
        );

        // Codex: found but blocked (no --json)
        mock.register("which codex", Ok(success_output("/usr/bin/codex\n")));
        mock.register("codex --version", Ok(success_output("codex 0.1.0\n")));
        mock.register(
            "codex exec --help",
            Ok(success_output("Usage: codex exec\n  --verbose\n")),
        );

        // Cursor: not found (no mock entries = not found)

        mock
    }

    #[test]
    fn registry_discovers_all_adapters() {
        let mock = all_ready_mock();
        let registry = AdapterRegistry::with_runner(false, &mock);

        assert_eq!(registry.all().len(), 3);
        assert!(registry.get("claude").is_some());
        assert!(registry.get("codex").is_some());
        assert!(registry.get("cursor-agent").is_some());
    }

    #[test]
    fn available_excludes_experimental_by_default() {
        let mock = all_ready_mock();
        let registry = AdapterRegistry::with_runner(false, &mock);

        let available = registry.available_adapters();
        // Only Claude and Codex (Tier-1)
        assert_eq!(available.len(), 2);
        let keys: Vec<&str> = available.iter().map(|a| a.key.as_str()).collect();
        assert!(keys.contains(&"claude"));
        assert!(keys.contains(&"codex"));
        assert!(!keys.contains(&"cursor-agent"));
    }

    #[test]
    fn available_includes_experimental_when_allowed() {
        let mock = all_ready_mock();
        let registry = AdapterRegistry::with_runner(true, &mock);

        let available = registry.available_adapters();
        assert_eq!(available.len(), 3);
        let keys: Vec<&str> = available.iter().map(|a| a.key.as_str()).collect();
        assert!(keys.contains(&"cursor-agent"));
    }

    #[test]
    fn get_runtime_returns_error_for_missing_adapter() {
        let mock = all_ready_mock();
        let registry = AdapterRegistry::with_runner(false, &mock);

        let result = registry.get_runtime("nonexistent");
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("adapter not found"));
    }

    #[test]
    fn get_runtime_blocks_experimental_when_not_allowed() {
        let mock = all_ready_mock();
        let registry = AdapterRegistry::with_runner(false, &mock);

        let result = registry.get_runtime("cursor-agent");
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("experimental"));
    }

    #[test]
    fn get_runtime_allows_experimental_when_enabled() {
        let mock = all_ready_mock();
        let registry = AdapterRegistry::with_runner(true, &mock);

        let result = registry.get_runtime("cursor-agent");
        assert!(result.is_ok());
    }

    #[test]
    fn get_runtime_returns_error_for_unavailable_adapter() {
        let mock = mixed_mock();
        let registry = AdapterRegistry::with_runner(false, &mock);

        // Codex is blocked, so runtime should not be available
        let result = registry.get_runtime("codex");
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("not available"));
    }

    #[test]
    fn tier1_ready_when_all_tier1_ready() {
        let mock = all_ready_mock();
        let registry = AdapterRegistry::with_runner(false, &mock);

        assert!(registry.tier1_ready());
    }

    #[test]
    fn tier1_not_ready_when_one_blocked() {
        let mock = mixed_mock();
        let registry = AdapterRegistry::with_runner(false, &mock);

        // Codex is blocked, so tier1_ready should be false
        assert!(!registry.tier1_ready());
    }

    #[test]
    fn tier1_not_ready_when_all_missing() {
        let mock = none_found_mock();
        let registry = AdapterRegistry::with_runner(false, &mock);

        // All missing, but the predicate checks all Tier1 are Ready.
        // With Missing status, tier1_ready should be false.
        assert!(!registry.tier1_ready());
    }

    #[test]
    fn available_adapters_empty_when_none_ready() {
        let mock = none_found_mock();
        let registry = AdapterRegistry::with_runner(true, &mock);

        assert!(registry.available_adapters().is_empty());
    }

    #[test]
    fn registered_adapter_is_available() {
        let mock = all_ready_mock();
        let registry = AdapterRegistry::with_runner(true, &mock);

        assert!(registry.get("claude").unwrap().is_available());
        assert!(registry.get("codex").unwrap().is_available());
        assert!(registry.get("cursor-agent").unwrap().is_available());
    }

    #[test]
    fn registered_adapter_not_available_when_blocked() {
        let mock = mixed_mock();
        let registry = AdapterRegistry::with_runner(false, &mock);

        assert!(registry.get("claude").unwrap().is_available());
        assert!(!registry.get("codex").unwrap().is_available());
    }
}
