use serde::{Deserialize, Serialize};

use super::DimensionScore;
use crate::config::DiffScopeConfig;

/// Statistics from a git diff between agent worktree and base ref.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffStats {
    pub files_changed: u32,
    pub lines_added: u32,
    pub lines_removed: u32,
    pub paths: Vec<String>,
}

impl DiffStats {
    pub fn total_churn(&self) -> u32 {
        self.lines_added + self.lines_removed
    }
}

/// Parse `git diff --numstat` output into DiffStats.
pub fn parse_numstat(output: &str) -> DiffStats {
    let mut stats = DiffStats::default();
    for line in output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let added: u32 = parts[0].parse().unwrap_or(0);
            let removed: u32 = parts[1].parse().unwrap_or(0);
            let path = parts[2].to_string();
            stats.lines_added += added;
            stats.lines_removed += removed;
            stats.files_changed += 1;
            stats.paths.push(path);
        }
    }
    stats
}

/// Score the diff scope dimension.
///
/// Heuristics (from docs/scoring-engine.md section 5.4):
/// - Modest churn scores highest
/// - Broad unrelated edits penalized
/// - Out-of-scope path edits trigger hard penalty (cap at 30)
pub fn score_diff_scope(stats: &DiffStats, config: &DiffScopeConfig) -> DimensionScore {
    let churn = stats.total_churn() as f64;
    let files = stats.files_changed as f64;

    let max_churn = config.max_churn_soft as f64;
    let max_files = config.max_files_soft as f64;

    // Churn score: full marks up to max_churn_soft, linear decay beyond
    let churn_score = if max_churn <= 0.0 || churn <= max_churn {
        100.0
    } else {
        let excess_ratio = (churn - max_churn) / max_churn;
        (100.0 - excess_ratio * 50.0).max(0.0)
    };

    // Files score: full marks up to max_files_soft, linear decay beyond
    let files_score = if max_files <= 0.0 || files <= max_files {
        100.0
    } else {
        let excess_ratio = (files - max_files) / max_files;
        (100.0 - excess_ratio * 50.0).max(0.0)
    };

    // Protected path check
    let protected_violation = !config.protected_paths.is_empty()
        && stats
            .paths
            .iter()
            .any(|p| config.protected_paths.iter().any(|pp| p.starts_with(pp)));

    let raw_score = (churn_score * 0.5 + files_score * 0.5).min(100.0);

    let score = if protected_violation {
        raw_score.min(30.0)
    } else {
        raw_score
    };

    DimensionScore {
        name: "diff_scope".to_string(),
        score,
        evidence: serde_json::json!({
            "files_changed": stats.files_changed,
            "lines_added": stats.lines_added,
            "lines_removed": stats.lines_removed,
            "total_churn": stats.total_churn(),
            "churn_score": churn_score,
            "files_score": files_score,
            "protected_violation": protected_violation,
        }),
    }
}

/// Compute diff stats by running git in the given worktree.
pub async fn compute_diff_stats(
    worktree_path: &std::path::Path,
    base_ref: &str,
) -> Result<DiffStats, std::io::Error> {
    let output = tokio::process::Command::new("git")
        .args(["diff", "--numstat", base_ref])
        .current_dir(worktree_path)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(std::io::Error::other(format!("git diff failed: {stderr}")));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut stats = parse_numstat(&stdout);

    let untracked = list_untracked_files(worktree_path).await?;
    for rel_path in untracked {
        let output = tokio::process::Command::new("git")
            .args([
                "-C",
                &worktree_path.to_string_lossy(),
                "diff",
                "--numstat",
                "--no-index",
                "--",
                "/dev/null",
                &rel_path,
            ])
            .output()
            .await?;

        // `git diff --no-index` exits with status 1 when differences are present.
        if !output.status.success() && output.status.code() != Some(1) {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(std::io::Error::other(format!(
                "git diff --no-index failed: {stderr}"
            )));
        }

        let mut extra = parse_numstat(&String::from_utf8_lossy(&output.stdout));
        if extra.files_changed > 0 {
            extra.paths = vec![rel_path];
        }
        merge_stats(&mut stats, extra);
    }

    Ok(stats)
}

async fn list_untracked_files(
    worktree_path: &std::path::Path,
) -> Result<Vec<String>, std::io::Error> {
    let output = tokio::process::Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(worktree_path)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(std::io::Error::other(format!(
            "git ls-files failed: {stderr}"
        )));
    }

    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect();
    Ok(files)
}

fn merge_stats(target: &mut DiffStats, extra: DiffStats) {
    target.files_changed += extra.files_changed;
    target.lines_added += extra.lines_added;
    target.lines_removed += extra.lines_removed;
    target.paths.extend(extra.paths);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn default_config() -> DiffScopeConfig {
        DiffScopeConfig {
            max_files_soft: 20,
            max_churn_soft: 800,
            protected_paths: vec![],
        }
    }

    #[test]
    fn parse_numstat_basic() {
        let output = "10\t5\tsrc/main.rs\n20\t3\tsrc/lib.rs\n";
        let stats = parse_numstat(output);
        assert_eq!(stats.files_changed, 2);
        assert_eq!(stats.lines_added, 30);
        assert_eq!(stats.lines_removed, 8);
        assert_eq!(stats.paths, vec!["src/main.rs", "src/lib.rs"]);
    }

    #[test]
    fn parse_numstat_empty() {
        let stats = parse_numstat("");
        assert_eq!(stats.files_changed, 0);
        assert_eq!(stats.total_churn(), 0);
    }

    #[test]
    fn modest_change_scores_high() {
        let config = default_config();
        let stats = DiffStats {
            files_changed: 3,
            lines_added: 50,
            lines_removed: 10,
            paths: vec!["src/a.rs".into(), "src/b.rs".into(), "src/c.rs".into()],
        };
        let score = score_diff_scope(&stats, &config);
        assert!((score.score - 100.0).abs() < 0.01);
    }

    #[test]
    fn excessive_churn_penalized() {
        let config = default_config();
        let stats = DiffStats {
            files_changed: 5,
            lines_added: 1200,
            lines_removed: 400,
            paths: (0..5).map(|i| format!("src/{i}.rs")).collect(),
        };
        let score = score_diff_scope(&stats, &config);
        assert!(score.score < 100.0);
    }

    #[test]
    fn excessive_files_penalized() {
        let config = default_config();
        let stats = DiffStats {
            files_changed: 50,
            lines_added: 100,
            lines_removed: 50,
            paths: (0..50).map(|i| format!("src/{i}.rs")).collect(),
        };
        let score = score_diff_scope(&stats, &config);
        assert!(score.score < 100.0);
    }

    #[test]
    fn protected_path_caps_at_30() {
        let mut config = default_config();
        config.protected_paths = vec!["infra/".to_string()];
        let stats = DiffStats {
            files_changed: 2,
            lines_added: 10,
            lines_removed: 5,
            paths: vec!["src/main.rs".into(), "infra/deploy.sh".into()],
        };
        let score = score_diff_scope(&stats, &config);
        assert!(score.score <= 30.0);
    }

    #[test]
    fn no_protected_paths_no_cap() {
        let config = default_config();
        let stats = DiffStats {
            files_changed: 2,
            lines_added: 10,
            lines_removed: 5,
            paths: vec!["src/main.rs".into(), "infra/deploy.sh".into()],
        };
        let score = score_diff_scope(&stats, &config);
        assert!((score.score - 100.0).abs() < 0.01);
    }

    #[test]
    fn empty_diff_scores_100() {
        let config = default_config();
        let stats = DiffStats::default();
        let score = score_diff_scope(&stats, &config);
        assert!((score.score - 100.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn compute_diff_stats_includes_untracked_new_files() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path();

        fn git(repo: &std::path::Path, args: &[&str]) {
            let output = Command::new("git")
                .args(args)
                .current_dir(repo)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        git(repo, &["init"]);
        git(repo, &["config", "user.email", "test@example.com"]);
        git(repo, &["config", "user.name", "Test User"]);
        std::fs::write(repo.join("README.md"), "base\n").unwrap();
        git(repo, &["add", "README.md"]);
        git(repo, &["commit", "-m", "init"]);

        std::fs::write(repo.join("snake.py"), "print('snake')\n").unwrap();

        let stats = compute_diff_stats(repo, "HEAD").await.unwrap();
        assert_eq!(stats.files_changed, 1);
        assert_eq!(stats.lines_added, 1);
        assert_eq!(stats.lines_removed, 0);
        assert!(stats.paths.iter().any(|p| p == "snake.py"));
    }
}
