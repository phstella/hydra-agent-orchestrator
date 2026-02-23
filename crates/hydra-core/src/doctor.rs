use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, warn};

use crate::adapter::claude::ClaudeProbe;
use crate::adapter::codex::CodexProbe;
use crate::adapter::cursor::CursorProbe;
use crate::adapter::probe::{
    AdapterProbe, AdapterTier, CommandRunner, ProbeReport, ProbeStatus, RealCommandRunner,
};

/// Full doctor report aggregating git checks and adapter probes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub git: GitCheck,
    pub adapters: ProbeReport,
    pub overall_ready: bool,
}

/// Result of checking git availability and repository status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCheck {
    pub git_available: bool,
    pub git_version: Option<String>,
    pub in_git_repo: bool,
    pub repo_root: Option<String>,
}

impl DoctorReport {
    /// Run all doctor checks using real system commands.
    pub fn run(repo_path: Option<&Path>) -> Self {
        let runner = RealCommandRunner;
        Self::run_with_runner(&runner, repo_path)
    }

    /// Run all doctor checks with a custom command runner (for testing).
    pub fn run_with_runner(runner: &dyn CommandRunner, repo_path: Option<&Path>) -> Self {
        let git = check_git(runner, repo_path);
        let adapters = probe_all_adapters(runner);
        let overall_ready = git.git_available && git.in_git_repo && adapters.tier1_ready;
        Self {
            git,
            adapters,
            overall_ready,
        }
    }

    /// Return a human-readable summary of reasons the system is not ready.
    /// Empty if `overall_ready` is true.
    pub fn not_ready_reasons(&self) -> Vec<String> {
        let mut reasons = Vec::new();

        if !self.git.git_available {
            reasons.push("git is not installed or not in PATH".to_string());
        }
        if !self.git.in_git_repo {
            reasons.push("not inside a git repository".to_string());
        }

        for adapter in &self.adapters.adapters {
            if adapter.tier == AdapterTier::Tier1 && adapter.status != ProbeStatus::Ready {
                let status_label = match adapter.status {
                    ProbeStatus::Missing => "missing",
                    ProbeStatus::Blocked => "blocked",
                    _ => "not ready",
                };
                reasons.push(format!(
                    "Tier-1 adapter \"{}\" is {}",
                    adapter.adapter_key, status_label
                ));
            }
        }

        reasons
    }
}

/// Check git availability and whether we are in a git repo.
fn check_git(runner: &dyn CommandRunner, repo_path: Option<&Path>) -> GitCheck {
    // Check git --version
    let version_output = runner.run("git", &["--version"]);
    let (git_available, git_version) = match version_output {
        Ok(output) if output.status.success() => {
            let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // Parse "git version X.Y.Z" -> "X.Y.Z"
            let version = raw
                .strip_prefix("git version ")
                .map(|v| v.to_string())
                .unwrap_or(raw);
            debug!(version = %version, "git found");
            (true, Some(version))
        }
        Ok(_) => {
            warn!("git --version exited non-zero");
            (false, None)
        }
        Err(e) => {
            warn!(error = %e, "git not found");
            (false, None)
        }
    };

    if !git_available {
        return GitCheck {
            git_available: false,
            git_version: None,
            in_git_repo: false,
            repo_root: None,
        };
    }

    // Check if inside a git work tree
    let mut rev_parse_args = vec!["rev-parse", "--show-toplevel"];
    let dir_arg: String;
    if let Some(path) = repo_path {
        dir_arg = format!("-C{}", path.display());
        rev_parse_args.insert(0, &dir_arg);
    }

    let arg_refs: Vec<&str> = rev_parse_args.iter().map(|s| s.as_ref()).collect();
    let repo_output = runner.run("git", &arg_refs);
    let (in_git_repo, repo_root) = match repo_output {
        Ok(output) if output.status.success() => {
            let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            debug!(root = %root, "inside git repo");
            (true, Some(root))
        }
        _ => {
            debug!("not inside a git repository");
            (false, None)
        }
    };

    GitCheck {
        git_available,
        git_version,
        in_git_repo,
        repo_root,
    }
}

/// Probe all known adapters and return an aggregate report.
fn probe_all_adapters(runner: &dyn CommandRunner) -> ProbeReport {
    let claude = ClaudeProbe::new(runner);
    let codex = CodexProbe::new(runner);
    let cursor = CursorProbe::new(runner);

    let probes: Vec<&dyn AdapterProbe> = vec![&claude, &codex, &cursor];
    let results: Vec<_> = probes.iter().map(|p| p.probe()).collect();

    ProbeReport::from_results(results)
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

    fn fail_output() -> Output {
        Output {
            status: ExitStatus::from_raw(1 << 8), // exit code 1
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    /// Registers all adapter mock responses as missing (no binaries found).
    fn register_all_adapters_missing(mock: &mut MockRunner) {
        for name in &["claude", "codex", "cursor-agent", "cursor"] {
            mock.register(
                &format!("which {name}"),
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                )),
            );
        }
    }

    /// Registers mock responses for both Tier-1 adapters as ready.
    fn register_tier1_adapters_ready(mock: &mut MockRunner) {
        // Claude
        mock.register("which claude", Ok(success_output("/usr/bin/claude\n")));
        mock.register("claude --version", Ok(success_output("claude 1.2.3\n")));
        mock.register(
            "claude --help",
            Ok(success_output(
                "Usage: claude\n  -p, --print\n  --output-format json\n  --resume\n",
            )),
        );
        // Codex
        mock.register("which codex", Ok(success_output("/usr/bin/codex\n")));
        mock.register("codex --version", Ok(success_output("codex 0.5.0\n")));
        mock.register(
            "codex exec --help",
            Ok(success_output("Usage: codex exec\n  --json\n  --sandbox\n")),
        );
    }

    #[test]
    fn doctor_report_serde_round_trip() {
        let report = DoctorReport {
            git: GitCheck {
                git_available: true,
                git_version: Some("2.43.0".to_string()),
                in_git_repo: true,
                repo_root: Some("/home/user/project".to_string()),
            },
            adapters: ProbeReport::from_results(vec![]),
            overall_ready: true,
        };

        let json = serde_json::to_string(&report).unwrap();
        let deser: DoctorReport = serde_json::from_str(&json).unwrap();

        assert!(deser.overall_ready);
        assert!(deser.git.git_available);
        assert_eq!(deser.git.git_version.as_deref(), Some("2.43.0"));
        assert!(deser.git.in_git_repo);
        assert_eq!(deser.git.repo_root.as_deref(), Some("/home/user/project"));
    }

    #[test]
    fn git_check_with_git_available() {
        let mut mock = MockRunner::new();
        mock.register("git --version", Ok(success_output("git version 2.43.0\n")));
        mock.register(
            "git rev-parse --show-toplevel",
            Ok(success_output("/home/user/project\n")),
        );

        let result = check_git(&mock, None);
        assert!(result.git_available);
        assert_eq!(result.git_version.as_deref(), Some("2.43.0"));
        assert!(result.in_git_repo);
        assert_eq!(result.repo_root.as_deref(), Some("/home/user/project"));
    }

    #[test]
    fn git_check_with_git_missing() {
        let mock = MockRunner::new();
        let result = check_git(&mock, None);
        assert!(!result.git_available);
        assert!(result.git_version.is_none());
        assert!(!result.in_git_repo);
        assert!(result.repo_root.is_none());
    }

    #[test]
    fn git_check_available_but_not_in_repo() {
        let mut mock = MockRunner::new();
        mock.register("git --version", Ok(success_output("git version 2.43.0\n")));
        mock.register("git rev-parse --show-toplevel", Ok(fail_output()));

        let result = check_git(&mock, None);
        assert!(result.git_available);
        assert!(!result.in_git_repo);
        assert!(result.repo_root.is_none());
    }

    #[test]
    fn overall_ready_false_when_git_missing() {
        let mut mock = MockRunner::new();
        // Git missing
        register_all_adapters_missing(&mut mock);

        let report = DoctorReport::run_with_runner(&mock, None);
        assert!(!report.overall_ready);
        assert!(!report.git.git_available);
    }

    #[test]
    fn overall_ready_false_when_not_in_repo() {
        let mut mock = MockRunner::new();
        mock.register("git --version", Ok(success_output("git version 2.43.0\n")));
        mock.register("git rev-parse --show-toplevel", Ok(fail_output()));
        register_tier1_adapters_ready(&mut mock);
        // Cursor missing
        for name in &["cursor-agent", "cursor"] {
            mock.register(
                &format!("which {name}"),
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                )),
            );
        }

        let report = DoctorReport::run_with_runner(&mock, None);
        assert!(!report.overall_ready);
        assert!(!report.git.in_git_repo);
    }

    #[test]
    fn overall_ready_false_when_tier1_adapter_blocked() {
        let mut mock = MockRunner::new();
        mock.register("git --version", Ok(success_output("git version 2.43.0\n")));
        mock.register(
            "git rev-parse --show-toplevel",
            Ok(success_output("/repo\n")),
        );
        // Claude: ready
        mock.register("which claude", Ok(success_output("/usr/bin/claude\n")));
        mock.register("claude --version", Ok(success_output("claude 1.0.0\n")));
        mock.register(
            "claude --help",
            Ok(success_output(
                "Usage: claude\n  -p, --print\n  --output-format json\n",
            )),
        );
        // Codex: missing
        mock.register(
            "which codex",
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not found",
            )),
        );
        // Cursor: missing
        for name in &["cursor-agent", "cursor"] {
            mock.register(
                &format!("which {name}"),
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                )),
            );
        }

        let report = DoctorReport::run_with_runner(&mock, None);
        assert!(!report.overall_ready);
        assert!(!report.adapters.tier1_ready);
    }

    #[test]
    fn overall_ready_true_when_all_tier1_ready_and_git_ok() {
        let mut mock = MockRunner::new();
        mock.register("git --version", Ok(success_output("git version 2.43.0\n")));
        mock.register(
            "git rev-parse --show-toplevel",
            Ok(success_output("/repo\n")),
        );
        register_tier1_adapters_ready(&mut mock);
        // Cursor: missing (experimental, doesn't affect overall_ready)
        for name in &["cursor-agent", "cursor"] {
            mock.register(
                &format!("which {name}"),
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                )),
            );
        }

        let report = DoctorReport::run_with_runner(&mock, None);
        assert!(report.overall_ready);
        assert!(report.git.git_available);
        assert!(report.git.in_git_repo);
        assert!(report.adapters.tier1_ready);
    }

    #[test]
    fn not_ready_reasons_when_git_missing() {
        let report = DoctorReport {
            git: GitCheck {
                git_available: false,
                git_version: None,
                in_git_repo: false,
                repo_root: None,
            },
            adapters: ProbeReport::from_results(vec![]),
            overall_ready: false,
        };

        let reasons = report.not_ready_reasons();
        assert!(reasons.iter().any(|r| r.contains("git")));
    }

    #[test]
    fn not_ready_reasons_when_adapter_missing() {
        use crate::adapter::probe::{CapabilitySet, Confidence, ProbeResult};

        let report = DoctorReport {
            git: GitCheck {
                git_available: true,
                git_version: Some("2.43.0".to_string()),
                in_git_repo: true,
                repo_root: Some("/repo".to_string()),
            },
            adapters: ProbeReport::from_results(vec![ProbeResult {
                adapter_key: "codex".to_string(),
                tier: AdapterTier::Tier1,
                status: ProbeStatus::Missing,
                binary_path: None,
                version: None,
                capabilities: CapabilitySet::default(),
                confidence: Confidence::Unknown,
                message: None,
            }]),
            overall_ready: false,
        };

        let reasons = report.not_ready_reasons();
        assert!(reasons
            .iter()
            .any(|r| r.contains("codex") && r.contains("missing")));
    }
}
