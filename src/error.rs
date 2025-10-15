//! Custom error types for the application.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaqError {
    #[error("Instrument error: {0}")]
    Instrument(String),

    #[error("Data processing error: {0}")]
    Processing(String),

    #[error("Feature '{0}' is not enabled. Please build with --features {0}")]
    FeatureNotEnabled(String),
}
