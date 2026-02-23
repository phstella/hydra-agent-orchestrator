use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::ProbeResult;
use super::AgentAdapter;

/// Aggregated probe report for all registered adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeReport {
    pub timestamp: DateTime<Utc>,
    pub results: Vec<ProbeResult>,
    pub all_tier1_ready: bool,
}

/// Runs probes against a set of registered adapters and produces a unified report.
pub struct ProbeRunner {
    adapters: Vec<Box<dyn AgentAdapter>>,
}

impl ProbeRunner {
    pub fn new(adapters: Vec<Box<dyn AgentAdapter>>) -> Self {
        Self { adapters }
    }

    pub fn run(&self) -> ProbeReport {
        let timestamp = Utc::now();
        let mut results = Vec::with_capacity(self.adapters.len());
        let mut all_tier1_ready = true;

        for adapter in &self.adapters {
            let detect = adapter.detect();
            let capabilities = adapter.capabilities();
            let tier = adapter.tier();

            if tier == super::types::AdapterTier::Tier1 && !detect.status.is_available() {
                all_tier1_ready = false;
            }

            results.push(ProbeResult {
                adapter_key: adapter.key().to_string(),
                tier,
                detect,
                capabilities,
            });
        }

        ProbeReport {
            timestamp,
            results,
            all_tier1_ready,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::types::*;
    use crate::adapter::AgentAdapter;

    struct FakeAdapter {
        key: &'static str,
        tier: AdapterTier,
        status: DetectStatus,
    }

    impl AgentAdapter for FakeAdapter {
        fn key(&self) -> &'static str {
            self.key
        }

        fn tier(&self) -> AdapterTier {
            self.tier
        }

        fn detect(&self) -> DetectResult {
            DetectResult {
                status: self.status.clone(),
                binary_path: Some("/usr/bin/fake".into()),
                version: Some("1.0.0".to_string()),
                supported_flags: vec!["--json".to_string()],
                confidence: CapabilityConfidence::Verified,
                error: None,
            }
        }

        fn capabilities(&self) -> CapabilitySet {
            CapabilitySet {
                json_stream: CapabilityEntry::verified(true),
                plain_text: CapabilityEntry::verified(true),
                force_edit_mode: CapabilityEntry::verified(false),
                sandbox_controls: CapabilityEntry::unknown(),
                approval_controls: CapabilityEntry::unknown(),
                session_resume: CapabilityEntry::unknown(),
                emits_usage: CapabilityEntry::unknown(),
            }
        }
    }

    #[test]
    fn probe_report_with_all_tier1_ready() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(FakeAdapter {
                key: "claude",
                tier: AdapterTier::Tier1,
                status: DetectStatus::Ready,
            }),
            Box::new(FakeAdapter {
                key: "codex",
                tier: AdapterTier::Tier1,
                status: DetectStatus::Ready,
            }),
        ];

        let runner = ProbeRunner::new(adapters);
        let report = runner.run();

        assert!(report.all_tier1_ready);
        assert_eq!(report.results.len(), 2);
    }

    #[test]
    fn probe_report_with_blocked_tier1() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(FakeAdapter {
                key: "claude",
                tier: AdapterTier::Tier1,
                status: DetectStatus::Blocked,
            }),
            Box::new(FakeAdapter {
                key: "codex",
                tier: AdapterTier::Tier1,
                status: DetectStatus::Ready,
            }),
        ];

        let runner = ProbeRunner::new(adapters);
        let report = runner.run();

        assert!(!report.all_tier1_ready);
        assert_eq!(report.results[0].detect.status, DetectStatus::Blocked);
    }

    #[test]
    fn experimental_adapter_does_not_affect_tier1_readiness() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![
            Box::new(FakeAdapter {
                key: "claude",
                tier: AdapterTier::Tier1,
                status: DetectStatus::Ready,
            }),
            Box::new(FakeAdapter {
                key: "cursor-agent",
                tier: AdapterTier::Experimental,
                status: DetectStatus::ExperimentalBlocked,
            }),
        ];

        let runner = ProbeRunner::new(adapters);
        let report = runner.run();

        assert!(report.all_tier1_ready);
        assert_eq!(
            report.results[1].detect.status,
            DetectStatus::ExperimentalBlocked
        );
    }

    #[test]
    fn empty_adapter_set_produces_valid_report() {
        let runner = ProbeRunner::new(vec![]);
        let report = runner.run();

        assert!(report.all_tier1_ready);
        assert!(report.results.is_empty());
    }

    #[test]
    fn probe_report_serializes_to_json() {
        let adapters: Vec<Box<dyn AgentAdapter>> = vec![Box::new(FakeAdapter {
            key: "claude",
            tier: AdapterTier::Tier1,
            status: DetectStatus::Ready,
        })];

        let runner = ProbeRunner::new(adapters);
        let report = runner.run();

        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("\"adapter_key\": \"claude\""));
        assert!(json.contains("\"all_tier1_ready\": true"));
        assert!(json.contains("\"tier\": \"tier1\""));
    }

    #[test]
    fn detect_status_is_available_logic() {
        assert!(DetectStatus::Ready.is_available());
        assert!(DetectStatus::ExperimentalReady.is_available());
        assert!(!DetectStatus::Blocked.is_available());
        assert!(!DetectStatus::ExperimentalBlocked.is_available());
        assert!(!DetectStatus::Missing.is_available());
    }
}
