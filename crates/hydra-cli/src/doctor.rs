use hydra_core::adapter::ProbeReport;
use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub adapters: ProbeReport,
    pub git: GitChecks,
    pub all_tier1_ready: bool,
    pub git_ok: bool,
}

impl DoctorReport {
    pub fn new(adapters: ProbeReport, git: GitChecks) -> Self {
        let all_tier1_ready = adapters.all_tier1_ready;
        let git_ok = git.is_repo && git.has_commits;
        Self {
            adapters,
            git,
            all_tier1_ready,
            git_ok,
        }
    }

    pub fn healthy(&self) -> bool {
        self.all_tier1_ready && self.git_ok
    }
}

#[derive(Debug, Serialize)]
pub struct GitChecks {
    pub is_repo: bool,
    pub has_commits: bool,
    pub current_branch: Option<String>,
    pub clean_working_tree: bool,
    pub error: Option<String>,
}

pub fn check_git_repo() -> GitChecks {
    let is_repo = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_repo {
        return GitChecks {
            is_repo: false,
            has_commits: false,
            current_branch: None,
            clean_working_tree: false,
            error: Some("not inside a git repository".to_string()),
        };
    }

    let has_commits = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let current_branch = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let branch = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if branch.is_empty() {
                    None
                } else {
                    Some(branch)
                }
            } else {
                None
            }
        });

    let clean_working_tree = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map(|o| o.status.success() && o.stdout.is_empty())
        .unwrap_or(false);

    GitChecks {
        is_repo,
        has_commits,
        current_branch,
        clean_working_tree,
        error: None,
    }
}

pub fn print_human_report(report: &DoctorReport) {
    println!("Hydra Doctor Report");
    println!("===================");
    println!();

    println!("Git Repository:");
    if report.git.is_repo {
        println!("  Status: OK");
        if let Some(branch) = &report.git.current_branch {
            println!("  Branch: {branch}");
        }
        println!(
            "  Working tree: {}",
            if report.git.clean_working_tree {
                "clean"
            } else {
                "dirty"
            }
        );
        println!(
            "  Has commits: {}",
            if report.git.has_commits { "yes" } else { "no" }
        );
    } else {
        println!("  Status: NOT A GIT REPO");
        if let Some(err) = &report.git.error {
            println!("  Error: {err}");
        }
    }

    println!();
    println!("Adapter Readiness:");
    println!(
        "  All Tier-1 ready: {}",
        if report.all_tier1_ready { "yes" } else { "NO" }
    );
    println!();

    for r in &report.adapters.results {
        let tier_label = match r.tier {
            hydra_core::adapter::AdapterTier::Tier1 => "tier-1",
            hydra_core::adapter::AdapterTier::Experimental => "experimental",
        };
        let status = r.detect.status_label();
        println!("  [{tier_label}] {key} ({status})", key = r.adapter_key);
        if let Some(path) = &r.detect.binary_path {
            println!("    binary: {}", path.display());
        }
        if let Some(v) = &r.detect.version {
            println!("    version: {v}");
        }
        if !r.detect.supported_flags.is_empty() {
            println!("    flags: {}", r.detect.supported_flags.join(", "));
        }
        if let Some(err) = &r.detect.error {
            println!("    error: {err}");
        }
    }

    println!();
    if report.healthy() {
        println!("Overall: HEALTHY");
    } else {
        println!("Overall: UNHEALTHY");
        if !report.all_tier1_ready {
            println!("  - One or more Tier-1 adapters are not ready");
        }
        if !report.git_ok {
            println!("  - Git repository checks failed");
        }
    }
}
