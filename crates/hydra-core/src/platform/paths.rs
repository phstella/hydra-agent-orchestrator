//! Platform-aware path normalisation and validation.
//!
//! Handles Windows long-path prefixes, invalid characters, file locking,
//! and Unicode safety checks.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tracing::{debug, warn};

use crate::{HydraError, Result};

/// Maximum retries for [`safe_write`] when a file is locked.
const WRITE_MAX_RETRIES: u32 = 5;

/// Base delay between retries (doubles each attempt).
const WRITE_RETRY_BASE: Duration = Duration::from_millis(100);

/// Normalise a path for the current OS.
///
/// On Windows, prepends the `\\?\` long-path prefix for paths exceeding
/// 260 characters. On other platforms this is a no-op.
pub fn normalize_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let path_str = path.to_string_lossy();
        if path_str.len() > 260 && !path_str.starts_with("\\\\?\\") {
            PathBuf::from(format!("\\\\?\\{}", path_str))
        } else {
            path.to_path_buf()
        }
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

/// Validate that a path is safe for artifact writes.
///
/// Checks for platform-specific invalid characters and excessively long
/// path components.
pub fn validate_artifact_path(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();

    // Reject empty paths.
    if path_str.is_empty() {
        return Err(HydraError::Artifact("path is empty".into()));
    }

    // Check for null bytes (invalid on all platforms).
    if path_str.contains('\0') {
        return Err(HydraError::Artifact("path contains null byte".into()));
    }

    #[cfg(windows)]
    {
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
        for c in invalid_chars {
            // Skip the drive letter colon (e.g. "C:\...").
            if c == ':' && path_str.len() >= 2 {
                let rest = &path_str[2..];
                if rest.contains(c) {
                    return Err(HydraError::Artifact(format!(
                        "path contains invalid Windows character '{c}'"
                    )));
                }
                continue;
            }
            if path_str.contains(c) {
                return Err(HydraError::Artifact(format!(
                    "path contains invalid Windows character '{c}'"
                )));
            }
        }
    }

    // Check individual component lengths (255 byte limit on most filesystems).
    for component in path.components() {
        let name = component.as_os_str().to_string_lossy();
        if name.len() > 255 {
            return Err(HydraError::Artifact(format!(
                "path component '{}...' exceeds 255 byte limit",
                &name[..40]
            )));
        }
    }

    Ok(())
}

/// Write `contents` to `path` with retry-on-lock.
///
/// If the write fails due to a permission or lock error, retries up to
/// [`WRITE_MAX_RETRIES`] times with exponential backoff.
pub async fn safe_write(path: &Path, contents: &[u8]) -> Result<()> {
    let normalized = normalize_path(path);
    validate_artifact_path(&normalized)?;

    // Ensure parent directory exists.
    if let Some(parent) = normalized.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            HydraError::Artifact(format!(
                "failed to create parent directories for {}: {e}",
                normalized.display()
            ))
        })?;
    }

    let mut delay = WRITE_RETRY_BASE;
    for attempt in 0..=WRITE_MAX_RETRIES {
        match tokio::fs::write(&normalized, contents).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                let retryable = matches!(
                    e.kind(),
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::WouldBlock
                );
                if retryable && attempt < WRITE_MAX_RETRIES {
                    debug!(
                        path = %normalized.display(),
                        attempt,
                        error = %e,
                        "write failed, retrying after backoff"
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                } else {
                    return Err(HydraError::Artifact(format!(
                        "failed to write {}: {e}",
                        normalized.display()
                    )));
                }
            }
        }
    }

    unreachable!()
}

/// Check whether a path contains characters outside the ASCII printable range
/// that may cause portability issues across platforms.
///
/// Returns `true` if the path is safe (ASCII-only or well-formed Unicode),
/// `false` if it contains replacement characters or other indicators of
/// encoding problems.
pub fn check_unicode_safety(path: &Path) -> bool {
    let s = path.to_string_lossy();

    // Replacement character indicates lossy conversion.
    if s.contains('\u{FFFD}') {
        warn!(path = %s, "path contains Unicode replacement characters");
        return false;
    }

    // Check for control characters (except path separators).
    for ch in s.chars() {
        if ch.is_control() && ch != '/' && ch != '\\' {
            warn!(path = %s, char = ?ch, "path contains control character");
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_short_path_unchanged() {
        let p = Path::new("/home/user/project/file.txt");
        assert_eq!(normalize_path(p), p.to_path_buf());
    }

    #[test]
    fn validate_rejects_empty_path() {
        let result = validate_artifact_path(Path::new(""));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn validate_rejects_null_byte() {
        let result = validate_artifact_path(Path::new("foo\0bar"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("null"));
    }

    #[test]
    fn validate_rejects_long_component() {
        let long_name = "a".repeat(256);
        let path = PathBuf::from(&long_name);
        let result = validate_artifact_path(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("255"));
    }

    #[test]
    fn validate_accepts_normal_path() {
        let result = validate_artifact_path(Path::new("/tmp/hydra/runs/abc/manifest.json"));
        assert!(result.is_ok());
    }

    #[test]
    fn unicode_safety_ascii_ok() {
        assert!(check_unicode_safety(Path::new("/tmp/hydra/test")));
    }

    #[test]
    fn unicode_safety_valid_unicode_ok() {
        assert!(check_unicode_safety(Path::new("/tmp/hydra/tést")));
    }

    #[test]
    fn unicode_safety_control_chars_rejected() {
        assert!(!check_unicode_safety(Path::new("/tmp/hydra/te\x01st")));
    }

    #[tokio::test]
    async fn safe_write_creates_parent_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("deep").join("nested").join("file.txt");
        safe_write(&path, b"hello").await.unwrap();

        let contents = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(contents, "hello");
    }

    #[tokio::test]
    async fn safe_write_overwrites_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("file.txt");
        safe_write(&path, b"first").await.unwrap();
        safe_write(&path, b"second").await.unwrap();

        let contents = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(contents, "second");
    }
}
