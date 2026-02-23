//! Artifact schema migration tool.
//!
//! Provides version checking and migration of run artifacts between
//! schema versions.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{HydraError, Result};

/// Current schema version for artifacts.
pub const CURRENT_SCHEMA_VERSION: &str = "1.0.0";

/// Parsed semver-like schema version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SchemaVersion {
    /// Parse a version string like `"1.0.0"`.
    pub fn parse(version_str: &str) -> Result<Self> {
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() != 3 {
            return Err(HydraError::Artifact(format!(
                "invalid schema version '{version_str}': expected 'major.minor.patch'"
            )));
        }

        let major = parts[0].parse::<u32>().map_err(|_| {
            HydraError::Artifact(format!(
                "invalid major version '{}' in '{version_str}'",
                parts[0]
            ))
        })?;
        let minor = parts[1].parse::<u32>().map_err(|_| {
            HydraError::Artifact(format!(
                "invalid minor version '{}' in '{version_str}'",
                parts[1]
            ))
        })?;
        let patch = parts[2].parse::<u32>().map_err(|_| {
            HydraError::Artifact(format!(
                "invalid patch version '{}' in '{version_str}'",
                parts[2]
            ))
        })?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    /// Check if this version is compatible with another.
    ///
    /// Compatibility rules:
    /// - Same major version is required.
    /// - Minor version of `other` must be <= `self` (forward-compatible reads).
    pub fn is_compatible_with(&self, other: &SchemaVersion) -> bool {
        self.major == other.major && other.minor <= self.minor
    }

    /// Format as a dotted string.
    pub fn to_string_version(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Return the current built-in schema version.
    pub fn current() -> Result<Self> {
        Self::parse(CURRENT_SCHEMA_VERSION)
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Summary of a migration operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub from_version: String,
    pub to_version: String,
    pub files_migrated: u32,
    pub warnings: Vec<String>,
}

/// Tool for migrating artifacts between schema versions.
pub struct MigrationTool;

impl MigrationTool {
    /// Check whether artifacts at `manifest_path` need migration.
    ///
    /// Reads the manifest's `schema_version` field and compares it to
    /// [`CURRENT_SCHEMA_VERSION`].
    pub fn needs_migration(manifest_path: &Path) -> Result<bool> {
        let data = std::fs::read_to_string(manifest_path).map_err(|e| {
            HydraError::Artifact(format!(
                "failed to read manifest {}: {e}",
                manifest_path.display()
            ))
        })?;

        let value: serde_json::Value = serde_json::from_str(&data)
            .map_err(|e| HydraError::Artifact(format!("failed to parse manifest: {e}")))?;

        let version_str = value
            .get("schema_version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                HydraError::Artifact("manifest missing 'schema_version' field".into())
            })?;

        let artifact_version = SchemaVersion::parse(version_str)?;
        let current = SchemaVersion::current()?;

        Ok(!current.is_compatible_with(&artifact_version) || artifact_version != current)
    }

    /// Migrate artifacts in `artifact_dir` to the current schema version.
    ///
    /// Currently only updates the `schema_version` field in manifests, since
    /// v1.0.0 is the initial version. Future versions will add transformation
    /// logic here.
    pub fn migrate(artifact_dir: &Path) -> Result<MigrationReport> {
        let manifest_path = artifact_dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(HydraError::Artifact(format!(
                "no manifest.json found in {}",
                artifact_dir.display()
            )));
        }

        let data = std::fs::read_to_string(&manifest_path)
            .map_err(|e| HydraError::Artifact(format!("failed to read manifest: {e}")))?;

        let mut value: serde_json::Value = serde_json::from_str(&data)
            .map_err(|e| HydraError::Artifact(format!("failed to parse manifest: {e}")))?;

        let from_version = value
            .get("schema_version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let mut warnings = Vec::new();
        let mut files_migrated = 0u32;

        // Perform version-specific migrations.
        let from = SchemaVersion::parse(&from_version).unwrap_or(SchemaVersion {
            major: 0,
            minor: 0,
            patch: 0,
        });
        let current = SchemaVersion::current()?;

        if from == current {
            debug!("artifacts already at current schema version");
            return Ok(MigrationReport {
                from_version: from_version.clone(),
                to_version: CURRENT_SCHEMA_VERSION.to_string(),
                files_migrated: 0,
                warnings: vec![],
            });
        }

        // Update schema_version in the manifest.
        value["schema_version"] = serde_json::Value::String(CURRENT_SCHEMA_VERSION.to_string());

        let json = serde_json::to_string_pretty(&value).map_err(|e| {
            HydraError::Artifact(format!("failed to serialize migrated manifest: {e}"))
        })?;
        std::fs::write(&manifest_path, json)
            .map_err(|e| HydraError::Artifact(format!("failed to write migrated manifest: {e}")))?;
        files_migrated += 1;

        // Check events file for any needed transformations.
        let events_path = artifact_dir.join("events.jsonl");
        if events_path.exists() {
            // For v1.0.0 the events format is stable; just note its existence.
            debug!("events.jsonl found; no migration needed for events at this version");
        } else {
            warnings.push("no events.jsonl found in artifact directory".into());
        }

        info!(
            from = %from_version,
            to = CURRENT_SCHEMA_VERSION,
            files_migrated,
            "migration complete"
        );

        Ok(MigrationReport {
            from_version,
            to_version: CURRENT_SCHEMA_VERSION.to_string(),
            files_migrated,
            warnings,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_version() {
        let v = SchemaVersion::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn parse_current_version() {
        let v = SchemaVersion::current().unwrap();
        assert_eq!(v, SchemaVersion::parse(CURRENT_SCHEMA_VERSION).unwrap());
    }

    #[test]
    fn parse_invalid_version_not_enough_parts() {
        let result = SchemaVersion::parse("1.0");
        assert!(result.is_err());
    }

    #[test]
    fn parse_invalid_version_non_numeric() {
        let result = SchemaVersion::parse("1.x.0");
        assert!(result.is_err());
    }

    #[test]
    fn compatibility_same_version() {
        let v1 = SchemaVersion::parse("1.0.0").unwrap();
        let v2 = SchemaVersion::parse("1.0.0").unwrap();
        assert!(v1.is_compatible_with(&v2));
    }

    #[test]
    fn compatibility_newer_minor_reads_older() {
        let current = SchemaVersion::parse("1.2.0").unwrap();
        let older = SchemaVersion::parse("1.1.0").unwrap();
        assert!(current.is_compatible_with(&older));
    }

    #[test]
    fn incompatible_different_major() {
        let v1 = SchemaVersion::parse("2.0.0").unwrap();
        let v2 = SchemaVersion::parse("1.0.0").unwrap();
        assert!(!v1.is_compatible_with(&v2));
    }

    #[test]
    fn incompatible_older_reading_newer_minor() {
        let older = SchemaVersion::parse("1.0.0").unwrap();
        let newer = SchemaVersion::parse("1.2.0").unwrap();
        assert!(!older.is_compatible_with(&newer));
    }

    #[test]
    fn version_display() {
        let v = SchemaVersion::parse("1.2.3").unwrap();
        assert_eq!(v.to_string(), "1.2.3");
        assert_eq!(v.to_string_version(), "1.2.3");
    }

    #[test]
    fn migration_report_serde_round_trip() {
        let report = MigrationReport {
            from_version: "0.9.0".into(),
            to_version: "1.0.0".into(),
            files_migrated: 2,
            warnings: vec!["some warning".into()],
        };
        let json = serde_json::to_string(&report).unwrap();
        let deser: MigrationReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.from_version, "0.9.0");
        assert_eq!(deser.to_version, "1.0.0");
        assert_eq!(deser.files_migrated, 2);
        assert_eq!(deser.warnings.len(), 1);
    }

    #[test]
    fn needs_migration_current_version() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest_path = tmp.path().join("manifest.json");
        let manifest = serde_json::json!({
            "schema_version": CURRENT_SCHEMA_VERSION,
            "run_id": "550e8400-e29b-41d4-a716-446655440000",
        });
        std::fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

        assert!(!MigrationTool::needs_migration(&manifest_path).unwrap());
    }

    #[test]
    fn needs_migration_old_version() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest_path = tmp.path().join("manifest.json");
        let manifest = serde_json::json!({
            "schema_version": "0.9.0",
            "run_id": "550e8400-e29b-41d4-a716-446655440000",
        });
        std::fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

        assert!(MigrationTool::needs_migration(&manifest_path).unwrap());
    }

    #[test]
    fn migrate_updates_version() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest_path = tmp.path().join("manifest.json");
        let manifest = serde_json::json!({
            "schema_version": "0.9.0",
            "run_id": "550e8400-e29b-41d4-a716-446655440000",
        });
        std::fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

        let report = MigrationTool::migrate(tmp.path()).unwrap();
        assert_eq!(report.from_version, "0.9.0");
        assert_eq!(report.to_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(report.files_migrated, 1);

        // Verify the file was updated.
        let data = std::fs::read_to_string(&manifest_path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(value["schema_version"], CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrate_no_op_when_current() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest_path = tmp.path().join("manifest.json");
        let manifest = serde_json::json!({
            "schema_version": CURRENT_SCHEMA_VERSION,
            "run_id": "550e8400-e29b-41d4-a716-446655440000",
        });
        std::fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

        let report = MigrationTool::migrate(tmp.path()).unwrap();
        assert_eq!(report.files_migrated, 0);
    }

    #[test]
    fn migrate_missing_manifest_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let result = MigrationTool::migrate(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no manifest.json"));
    }
}
