use hydra_core::adapter::ProbeReport;
use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub adapters: ProbeReport,
    pub git: GitChecks,
    pub all_tier1_ready: bool,
    pub git_ok: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdapterPathOverrides {
    pub claude: Option<String>,
    pub codex: Option<String>,
    pub cursor: Option<String>,
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

pub fn load_adapter_path_overrides() -> AdapterPathOverrides {
    load_adapter_path_overrides_from(Path::new("hydra.toml"))
}

fn load_adapter_path_overrides_from(path: &Path) -> AdapterPathOverrides {
    if !path.exists() {
        return AdapterPathOverrides::default();
    }

    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(err) => {
            tracing::warn!(
                config = %path.display(),
                error = %err,
                "Failed to read hydra.toml adapter path overrides"
            );
            return AdapterPathOverrides::default();
        }
    };

    match parse_adapter_path_overrides_toml(&data) {
        Ok(paths) => paths,
        Err(err) => {
            tracing::warn!(
                config = %path.display(),
                error = %err,
                "Failed to parse hydra.toml adapter path overrides"
            );
            AdapterPathOverrides::default()
        }
    }
}

fn parse_adapter_path_overrides_toml(data: &str) -> Result<AdapterPathOverrides, String> {
    let mut in_adapters_section = false;
    let mut parsed = AdapterPathOverrides::default();
    let mut cursor_fallback: Option<String> = None;

    for (idx, raw_line) in data.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_adapters_section = line == "[adapters]";
            continue;
        }

        if !in_adapters_section {
            continue;
        }

        let (key, value_raw) = line
            .split_once('=')
            .ok_or_else(|| format!("invalid adapters entry at line {}", idx + 1))?;
        let key = key.trim();
        let value = parse_toml_string_value(value_raw.trim())
            .ok_or_else(|| format!("invalid value for '{key}' at line {}", idx + 1))?;

        match key {
            "claude" => parsed.claude = Some(value),
            "codex" => parsed.codex = Some(value),
            "cursor_agent" => parsed.cursor = Some(value),
            "cursor" => cursor_fallback = Some(value),
            _ => {}
        }
    }

    if parsed.cursor.is_none() {
        parsed.cursor = cursor_fallback;
    }

    Ok(parsed)
}

fn parse_toml_string_value(input: &str) -> Option<String> {
    let value_without_comment = input.split('#').next()?.trim();
    if value_without_comment.is_empty() {
        return None;
    }

    if let Some(inner) = value_without_comment
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
    {
        return Some(inner.replace("\\\"", "\""));
    }

    if let Some(inner) = value_without_comment
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
    {
        return Some(inner.to_string());
    }

    Some(value_without_comment.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parse_adapter_paths_from_toml() {
        let data = r#"
[adapters]
claude = "/opt/claude"
codex = "/opt/codex"
cursor = "/opt/cursor"
"#;

        let parsed = parse_adapter_path_overrides_toml(data).unwrap();
        assert_eq!(parsed.claude.as_deref(), Some("/opt/claude"));
        assert_eq!(parsed.codex.as_deref(), Some("/opt/codex"));
        assert_eq!(parsed.cursor.as_deref(), Some("/opt/cursor"));
    }

    #[test]
    fn parse_prefers_cursor_agent_field() {
        let data = r#"
[adapters]
cursor = "/opt/cursor"
cursor_agent = "/opt/cursor-agent"
"#;

        let parsed = parse_adapter_path_overrides_toml(data).unwrap();
        assert_eq!(parsed.cursor.as_deref(), Some("/opt/cursor-agent"));
    }

    #[test]
    fn missing_config_file_returns_default_paths() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("hydra.toml");
        let parsed = load_adapter_path_overrides_from(&missing);
        assert_eq!(parsed, AdapterPathOverrides::default());
    }

    #[test]
    fn parser_ignores_non_adapter_sections_and_inline_comments() {
        let data = r#"
[scoring]
profile = "rust"

[adapters]
claude = "/opt/claude" # comment
codex = '/opt/codex'
"#;

        let parsed = parse_adapter_path_overrides_toml(data).unwrap();
        assert_eq!(parsed.claude.as_deref(), Some("/opt/claude"));
        assert_eq!(parsed.codex.as_deref(), Some("/opt/codex"));
        assert_eq!(parsed.cursor, None);
    }
}
