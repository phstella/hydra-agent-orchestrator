pub mod adapter;
pub mod artifact;
pub mod config;
pub mod doctor;
pub mod error;
pub mod scoring;
pub mod security;
pub mod supervisor;
pub mod worktree;

pub use error::{HydraError, Result};

use tracing::info;

/// Initialise a default tracing subscriber for the library consumer.
///
/// Call this once at program start. Uses `RUST_LOG` env var for filtering,
/// defaulting to `info` level.
pub fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt().with_env_filter(filter).init();

    info!("hydra tracing initialised");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = HydraError::Config("missing field".into());
        assert_eq!(err.to_string(), "config error: missing field");
    }

    #[test]
    fn io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let hydra_err: HydraError = io_err.into();
        assert!(matches!(hydra_err, HydraError::Io(_)));
    }
}
