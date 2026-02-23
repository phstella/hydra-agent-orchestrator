pub mod paths;
pub mod process;

/// Returns `true` when compiled for Windows.
pub fn is_windows() -> bool {
    cfg!(windows)
}

/// Returns `true` when compiled for a Unix-family OS.
pub fn is_unix() -> bool {
    cfg!(unix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_detection_is_consistent() {
        // On any given build, exactly one of these should be true.
        assert!(is_windows() || is_unix());
    }

    #[cfg(unix)]
    #[test]
    fn unix_detected_on_unix() {
        assert!(is_unix());
        assert!(!is_windows());
    }

    #[cfg(windows)]
    #[test]
    fn windows_detected_on_windows() {
        assert!(is_windows());
        assert!(!is_unix());
    }
}
