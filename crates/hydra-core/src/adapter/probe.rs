use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Output;

/// Adapter tier classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterTier {
    Tier1,
    Experimental,
}

/// Confidence level for detected capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Verified,
    Observed,
    Unknown,
}

/// Result of probing an adapter's availability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub adapter_key: String,
    pub tier: AdapterTier,
    pub status: ProbeStatus,
    pub binary_path: Option<PathBuf>,
    pub version: Option<String>,
    pub capabilities: CapabilitySet,
    pub confidence: Confidence,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    Ready,
    Blocked,
    Missing,
    ExperimentalReady,
    ExperimentalBlocked,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub json_stream: bool,
    pub plain_text: bool,
    pub force_edit_mode: bool,
    pub sandbox_controls: bool,
    pub approval_controls: bool,
    pub session_resume: bool,
    pub emits_usage: bool,
}

/// Trait that all adapter probes must implement.
pub trait AdapterProbe: Send + Sync {
    /// Unique key for this adapter (e.g., "claude", "codex", "cursor-agent").
    fn key(&self) -> &'static str;

    /// The tier classification of this adapter.
    fn tier(&self) -> AdapterTier;

    /// Probe the system for this adapter's availability and capabilities.
    fn probe(&self) -> ProbeResult;
}

/// Aggregate probe report for all adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeReport {
    pub adapters: Vec<ProbeResult>,
    pub tier1_ready: bool,
    pub experimental_available: Vec<String>,
}

impl ProbeReport {
    pub fn from_results(results: Vec<ProbeResult>) -> Self {
        let tier1_ready = results
            .iter()
            .filter(|r| r.tier == AdapterTier::Tier1)
            .all(|r| r.status == ProbeStatus::Ready);
        let experimental_available = results
            .iter()
            .filter(|r| {
                r.tier == AdapterTier::Experimental && r.status == ProbeStatus::ExperimentalReady
            })
            .map(|r| r.adapter_key.clone())
            .collect();
        Self {
            adapters: results,
            tier1_ready,
            experimental_available,
        }
    }
}

/// Abstraction over command execution, allowing probes to be tested without
/// invoking real binaries.
pub trait CommandRunner: Send + Sync {
    /// Run a command with the given program and arguments.
    /// Returns `Ok(Output)` if the process was spawned (even if it exited non-zero),
    /// or `Err` if the binary could not be found / executed.
    fn run(&self, program: &str, args: &[&str]) -> std::io::Result<Output>;
}

/// Default implementation that shells out to real processes.
pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> std::io::Result<Output> {
        std::process::Command::new(program).args(args).output()
    }
}

/// Build a [`ProbeResult`] for a missing binary.
pub fn missing_result(key: &str, tier: AdapterTier) -> ProbeResult {
    ProbeResult {
        adapter_key: key.to_string(),
        tier,
        status: ProbeStatus::Missing,
        binary_path: None,
        version: None,
        capabilities: CapabilitySet::default(),
        confidence: Confidence::Unknown,
        message: Some(format!("{key} binary not found in PATH")),
    }
}

/// Find the absolute path of a binary using `which`.
pub fn which_binary(runner: &dyn CommandRunner, name: &str) -> Option<PathBuf> {
    runner
        .run("which", &[name])
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_report_all_tier1_ready() {
        let results = vec![
            ProbeResult {
                adapter_key: "a".into(),
                tier: AdapterTier::Tier1,
                status: ProbeStatus::Ready,
                binary_path: None,
                version: None,
                capabilities: CapabilitySet::default(),
                confidence: Confidence::Verified,
                message: None,
            },
            ProbeResult {
                adapter_key: "b".into(),
                tier: AdapterTier::Tier1,
                status: ProbeStatus::Ready,
                binary_path: None,
                version: None,
                capabilities: CapabilitySet::default(),
                confidence: Confidence::Verified,
                message: None,
            },
        ];
        let report = ProbeReport::from_results(results);
        assert!(report.tier1_ready);
        assert!(report.experimental_available.is_empty());
    }

    #[test]
    fn probe_report_tier1_blocked() {
        let results = vec![
            ProbeResult {
                adapter_key: "a".into(),
                tier: AdapterTier::Tier1,
                status: ProbeStatus::Ready,
                binary_path: None,
                version: None,
                capabilities: CapabilitySet::default(),
                confidence: Confidence::Verified,
                message: None,
            },
            ProbeResult {
                adapter_key: "b".into(),
                tier: AdapterTier::Tier1,
                status: ProbeStatus::Blocked,
                binary_path: None,
                version: None,
                capabilities: CapabilitySet::default(),
                confidence: Confidence::Observed,
                message: Some("missing flag".into()),
            },
        ];
        let report = ProbeReport::from_results(results);
        assert!(!report.tier1_ready);
    }

    #[test]
    fn probe_report_experimental_available() {
        let results = vec![ProbeResult {
            adapter_key: "cursor-agent".into(),
            tier: AdapterTier::Experimental,
            status: ProbeStatus::ExperimentalReady,
            binary_path: None,
            version: None,
            capabilities: CapabilitySet::default(),
            confidence: Confidence::Observed,
            message: None,
        }];
        let report = ProbeReport::from_results(results);
        // No tier1 adapters means the predicate `all(...)` on an empty iterator is true.
        assert!(report.tier1_ready);
        assert_eq!(report.experimental_available, vec!["cursor-agent"]);
    }

    #[test]
    fn missing_result_builds_correctly() {
        let r = missing_result("test", AdapterTier::Tier1);
        assert_eq!(r.status, ProbeStatus::Missing);
        assert_eq!(r.adapter_key, "test");
        assert!(r.message.unwrap().contains("not found"));
    }

    #[test]
    fn capability_set_defaults_to_false() {
        let c = CapabilitySet::default();
        assert!(!c.json_stream);
        assert!(!c.plain_text);
        assert!(!c.force_edit_mode);
        assert!(!c.sandbox_controls);
        assert!(!c.approval_controls);
        assert!(!c.session_resume);
        assert!(!c.emits_usage);
    }

    #[test]
    fn serde_round_trip_probe_result() {
        let r = ProbeResult {
            adapter_key: "claude".into(),
            tier: AdapterTier::Tier1,
            status: ProbeStatus::Ready,
            binary_path: Some(PathBuf::from("/usr/bin/claude")),
            version: Some("1.0.0".into()),
            capabilities: CapabilitySet {
                json_stream: true,
                plain_text: true,
                ..Default::default()
            },
            confidence: Confidence::Verified,
            message: None,
        };
        let json = serde_json::to_string(&r).unwrap();
        let deser: ProbeResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.adapter_key, "claude");
        assert_eq!(deser.tier, AdapterTier::Tier1);
        assert_eq!(deser.status, ProbeStatus::Ready);
        assert!(deser.capabilities.json_stream);
    }
}
