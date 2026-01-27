//! Client error types.

use thiserror::Error;

/// Result type alias using ClientError.
pub type Result<T> = std::result::Result<T, ClientError>;

/// Errors that can occur when using the DAQ client.
#[derive(Error, Debug)]
pub enum ClientError {
    /// Invalid URL format.
    #[error("Invalid URL: {0}")]
    UrlParse(#[from] url::ParseError),

    /// gRPC transport error (connection failed, TLS error, etc.).
    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    /// gRPC status error (server returned an error).
    #[error("gRPC status error: {0}")]
    RpcStatus(#[from] tonic::Status),

    /// Connection failed with a descriptive message.
    #[error("Connection failed: {0}")]
    Connection(String),

    /// Timeout waiting for operation.
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Device not found.
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
