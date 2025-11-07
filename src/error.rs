//! Custom error types for the application.
//!
//! This module defines the primary error type, `DaqError`, for the entire application.
//! Using the `thiserror` crate, it provides a centralized and consistent way to handle
//! different kinds of errors that can occur, from I/O and configuration issues to
//! instrument-specific problems.
//!
//! ## Error Hierarchy
//!
//! `DaqError` is an enum that consolidates various error sources:
//!
//! - **`Config`**: Wraps errors from the `config` crate, typically related to file parsing
//!   or format issues in the configuration files.
//! - **`Configuration`**: Represents semantic errors in the configuration, such as invalid
//!   values that pass parsing but are logically incorrect (e.g., an invalid IP address format).
//!   These are usually caught during the validation step.
//! - **`Io`**: Wraps standard `std::io::Error`, covering all file and network I/O issues.
//! - **`Tokio`**: Specifically for errors related to the Tokio runtime, though it also wraps
//!   `std::io::Error` as Tokio I/O operations are a common source.
//! - **`Instrument`**: A general category for errors originating from instrument drivers. This
//!   could be anything from a communication failure to an invalid command sent to the hardware.
//! - **`Processing`**: For errors that occur during data processing stages, such as filtering
//!   or analysis.
//! - **`FeatureNotEnabled`**: A specific error used when the code attempts to use functionality
//!   (like a specific instrument driver or storage format) that was not included at compile
//!   time via feature flags. This provides a clear message to the user on how to enable it.
//!
//! By using `#[from]`, `DaqError` can be seamlessly created from underlying error types,
//! simplifying error handling throughout the application with the `?` operator.

use daq_core::DaqError as CoreDaqError;
use thiserror::Error;

/// Convenience alias for results using the application error type.
pub type AppResult<T> = std::result::Result<T, DaqError>;

#[derive(Error, Debug)]
pub enum DaqError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Configuration validation error: {0}")]
    Configuration(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tokio runtime error: {0}")]
    Tokio(std::io::Error),

    #[error("Instrument error: {0}")]
    Instrument(String),

    #[error("Serial port not connected")]
    SerialPortNotConnected,

    #[error("Unexpected EOF from serial port")]
    SerialUnexpectedEof,

    #[error("Serial support not enabled. Rebuild with --features instrument_serial")]
    SerialFeatureDisabled,

    #[error("Data processing error: {0}")]
    Processing(String),

    #[error("Module does not support operation: {0}")]
    ModuleOperationNotSupported(String),

    #[error("Module is busy during operation")]
    ModuleBusyDuringOperation,

    #[error("No camera assigned to module")]
    CameraNotAssigned,

    #[error("Feature '{0}' is not enabled. Please build with --features {0}")]
    FeatureNotEnabled(String),

    #[error("Feature '{0}' is enabled but not yet implemented. {1}")]
    FeatureIncomplete(String, String),

    #[error("Shutdown failed with errors")]
    ShutdownFailed(Vec<DaqError>),

    #[error("Failed to send value update (no subscribers)")]
    ParameterNoSubscribers,

    #[error("Parameter is read-only")]
    ParameterReadOnly,

    #[error("Invalid choice for parameter")]
    ParameterInvalidChoice,

    #[error("No hardware reader connected")]
    ParameterNoHardwareReader,
}

impl From<DaqError> for CoreDaqError {
    fn from(value: DaqError) -> Self {
        match value {
            DaqError::Config(err) => CoreDaqError {
                message: err.to_string(),
                can_recover: false,
            },
            DaqError::Configuration(msg)
            | DaqError::Processing(msg)
            | DaqError::Instrument(msg) => CoreDaqError {
                message: msg,
                can_recover: true,
            },
            DaqError::ModuleOperationNotSupported(operation) => CoreDaqError {
                message: format!("Module does not support operation: {operation}"),
                can_recover: true,
            },
            DaqError::ModuleBusyDuringOperation => CoreDaqError {
                message: "Module is busy during operation".into(),
                can_recover: true,
            },
            DaqError::CameraNotAssigned => CoreDaqError {
                message: "No camera assigned to module".into(),
                can_recover: true,
            },
            DaqError::Io(err) | DaqError::Tokio(err) => CoreDaqError {
                message: err.to_string(),
                can_recover: false,
            },
            DaqError::SerialPortNotConnected => CoreDaqError {
                message: "Serial port not connected".into(),
                can_recover: true,
            },
            DaqError::SerialUnexpectedEof => CoreDaqError {
                message: "Unexpected EOF from serial port".into(),
                can_recover: true,
            },
            DaqError::SerialFeatureDisabled => CoreDaqError {
                message: "Serial support not enabled. Rebuild with --features instrument_serial"
                    .into(),
                can_recover: false,
            },
            DaqError::FeatureNotEnabled(feature) => CoreDaqError {
                message: format!("Feature '{feature}' is not enabled"),
                can_recover: false,
            },
            DaqError::FeatureIncomplete(feature, note) => CoreDaqError {
                message: format!("Feature '{feature}' is incomplete: {note}"),
                can_recover: false,
            },
            DaqError::ShutdownFailed(errors) => {
                let combined = errors
                    .into_iter()
                    .map(|err| err.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                CoreDaqError {
                    message: format!("Shutdown failed: {combined}"),
                    can_recover: false,
                }
            }
            DaqError::ParameterNoSubscribers
            | DaqError::ParameterReadOnly
            | DaqError::ParameterInvalidChoice => CoreDaqError {
                message: value.to_string(),
                can_recover: true,
            },
            DaqError::ParameterNoHardwareReader => CoreDaqError {
                message: value.to_string(),
                can_recover: true,
            },
        }
    }
}

impl From<CoreDaqError> for DaqError {
    fn from(value: CoreDaqError) -> Self {
        if value.can_recover {
            DaqError::Processing(value.message)
        } else {
            DaqError::Instrument(value.message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_app_instrument_error_to_core() {
        let app_err = DaqError::Instrument("laser failed".to_string());
        let core: CoreDaqError = app_err.into();
        assert_eq!(core.message, "laser failed");
        assert!(core.can_recover);
    }

    #[test]
    fn converts_shutdown_failure_to_core() {
        let app_err = DaqError::ShutdownFailed(vec![
            DaqError::Instrument("camera timeout".into()),
            DaqError::Processing("buffer drain".into()),
        ]);
        let core: CoreDaqError = app_err.into();
        assert!(!core.can_recover);
        assert!(core.message.contains("camera timeout"));
        assert!(core.message.contains("buffer drain"));
    }

    #[test]
    fn converts_core_error_back_to_app() {
        let core = CoreDaqError {
            message: "recoverable".into(),
            can_recover: true,
        };
        let app: DaqError = core.into();
        match app {
            DaqError::Processing(msg) => assert_eq!(msg, "recoverable"),
            other => panic!("unexpected variant: {:?}", other),
        }
    }
}