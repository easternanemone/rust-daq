//! Custom error types for the application.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaqError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tokio runtime error: {0}")]
    Tokio(std::io::Error),

    #[error("Instrument error: {0}")]
    Instrument(String),

    #[cfg(feature = "instrument_visa")]
    #[error("VISA error: {0}")]
    Visa(#[from] visa_rs::Error),

    #[error("Data processing error: {0}")]
    Processing(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Feature '{0}' is not enabled. Please build with --features {0}")]
    FeatureNotEnabled(String),
}
