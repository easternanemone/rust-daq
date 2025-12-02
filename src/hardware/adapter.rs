//! Hardware adapter abstraction for generic instrument communication.
//!
//! This module provides a trait-based interface for communicating with laboratory
//! instruments through various connection methods (serial, USB, VISA, etc.).
//!
//! The `HardwareAdapter` trait defines a common interface for:
//! - Configuration validation
//! - Connection lifecycle management
//! - Command/query operations
//!
//! # Architecture
//!
//! Adapters serve as a bridge between high-level capability traits (`Movable`,
//! `Readable`, etc.) and low-level communication protocols. Each concrete adapter
//! implementation handles protocol-specific details like:
//! - Connection establishment
//! - Command formatting
//! - Response parsing
//! - Error handling
//!
//! # Example
//!
//! ```rust,ignore
//! use rust_daq::hardware::adapter::HardwareAdapter;
//! use serde_json::json;
//!
//! // Adapter implements HardwareAdapter trait
//! let mut adapter = MySerialAdapter::new();
//!
//! // Configure and connect
//! let config = json!({
//!     "port": "/dev/ttyUSB0",
//!     "baud_rate": 9600,
//! });
//! adapter.connect(&config).await?;
//!
//! // Send commands
//! adapter.send("*IDN?\r\n").await?;
//! let response = adapter.query("READ?\r\n").await?;
//! ```

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// An error that can occur when interacting with a hardware adapter.
///
/// Categorizes common failure modes when communicating with laboratory instruments.
///
/// # Error Handling Strategy
///
/// - `InvalidConfig` and `NotConnected` indicate programming errors - fix the calling code
/// - `ConnectionFailed`, `SendFailed`, `QueryFailed` may be transient - retry with backoff
/// - `Io` errors usually indicate hardware/cable issues - check physical connections
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// Configuration validation failed.
    ///
    /// The provided configuration JSON is missing required fields,
    /// contains invalid values, or has type mismatches.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rust_daq::hardware::adapter::AdapterError;
    /// let error = AdapterError::InvalidConfig(
    ///     "Missing required field 'port'".to_string()
    /// );
    /// ```
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Operation attempted without establishing connection first.
    ///
    /// Call `connect()` before attempting to send commands or queries.
    #[error("Not connected")]
    NotConnected,

    /// Failed to establish connection to the hardware.
    ///
    /// Check physical connections, port availability, and permissions.
    ///
    /// # Common Causes
    /// - Serial port already in use
    /// - Incorrect port name (e.g., `/dev/ttyUSB0` vs `/dev/ttyUSB1`)
    /// - Insufficient permissions (try `sudo usermod -aG dialout $USER`)
    /// - Device not powered on
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Failed to send command to hardware.
    ///
    /// Device may have disconnected or stopped responding.
    /// Consider implementing automatic reconnection logic.
    #[error("Send failed: {0}")]
    SendFailed(String),

    /// Failed to query response from hardware.
    ///
    /// Device may be unresponsive, or response format is unexpected.
    /// Check device manual for correct query syntax.
    #[error("Query failed: {0}")]
    QueryFailed(String),

    /// Low-level I/O error occurred.
    ///
    /// Wraps standard I/O errors like timeouts, broken pipes, or permission issues.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Catchall for other errors.
    ///
    /// Used for errors that don't fit other categories.
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Trait for generic hardware communication adapters.
///
/// Defines the interface for connecting to and communicating with laboratory
/// instruments. Implementations handle protocol-specific details while presenting
/// a uniform API.
///
/// # Implementation Notes
///
/// - All methods are async to support non-blocking I/O
/// - Configuration uses JSON for flexibility across different adapter types
/// - Implementations should validate configuration in `validate_config()`
/// - Connection state should be tracked internally
///
/// # Example Implementation
///
/// ```rust,ignore
/// use rust_daq::hardware::adapter::{HardwareAdapter, AdapterError};
/// use async_trait::async_trait;
/// use serde_json::Value;
///
/// struct MySerialAdapter {
///     port: Option<SerialPort>,
/// }
///
/// #[async_trait]
/// impl HardwareAdapter for MySerialAdapter {
///     fn name(&self) -> &str { "MySerialAdapter" }
///     
///     fn default_config(&self) -> Value {
///         json!({
///             "port": "/dev/ttyUSB0",
///             "baud_rate": 9600,
///         })
///     }
///     
///     fn validate_config(&self, config: &Value) -> Result<()> {
///         config.get("port")
///             .ok_or_else(|| anyhow!("Missing 'port'"))?;
///         Ok(())
///     }
///     
///     async fn connect(&mut self, config: &Value) -> Result<()> {
///         // Open serial port...
///         Ok(())
///     }
///     
///     // ... implement other methods
/// }
/// ```
#[async_trait]
pub trait HardwareAdapter: Send + Sync {
    /// Get the name of the adapter.
    ///
    /// Used for logging and identification purposes.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let adapter = MySerialAdapter::new();
    /// assert_eq!(adapter.name(), "MySerialAdapter");
    /// ```
    fn name(&self) -> &str;

    /// Get the default configuration for the adapter.
    ///
    /// Returns a JSON object with default values for all configuration parameters.
    /// Users can modify this template to create custom configurations.
    ///
    /// # Returns
    ///
    /// JSON object with default configuration values
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let adapter = MySerialAdapter::new();
    /// let config = adapter.default_config();
    /// println!("Default config: {}", config);
    /// // {"port": "/dev/ttyUSB0", "baud_rate": 9600}
    /// ```
    fn default_config(&self) -> Value;

    /// Validate a configuration for the adapter.
    ///
    /// Checks that the configuration contains all required fields with valid values.
    /// Called before `connect()` to catch configuration errors early.
    ///
    /// # Arguments
    ///
    /// * `config` - JSON configuration to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` if configuration is valid
    /// * `Err` if configuration is invalid or incomplete
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    ///
    /// let adapter = MySerialAdapter::new();
    /// let config = json!({"port": "/dev/ttyUSB0", "baud_rate": 115200});
    /// adapter.validate_config(&config)?;
    /// ```
    fn validate_config(&self, config: &Value) -> Result<()>;

    /// Connect to the hardware.
    ///
    /// Establishes connection to the instrument using the provided configuration.
    /// Must be called before attempting to send commands or queries.
    ///
    /// # Arguments
    ///
    /// * `config` - JSON configuration specifying connection parameters
    ///
    /// # Errors
    ///
    /// * `AdapterError::InvalidConfig` - Configuration is invalid
    /// * `AdapterError::ConnectionFailed` - Failed to establish connection
    /// * `AdapterError::Io` - Low-level I/O error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    ///
    /// let mut adapter = MySerialAdapter::new();
    /// let config = json!({
    ///     "port": "/dev/ttyUSB0",
    ///     "baud_rate": 115200,
    /// });
    /// adapter.connect(&config).await?;
    /// ```
    async fn connect(&mut self, config: &Value) -> Result<()>;

    /// Disconnect from the hardware.
    ///
    /// Closes the connection and releases hardware resources.
    /// Safe to call even if not currently connected.
    ///
    /// # Errors
    ///
    /// * `AdapterError::Io` - Error while closing connection
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// adapter.disconnect().await?;
    /// ```
    async fn disconnect(&mut self) -> Result<()>;

    /// Send a command to the hardware.
    ///
    /// Transmits a command string to the instrument without waiting for a response.
    /// Use `query()` for commands that return data.
    ///
    /// # Arguments
    ///
    /// * `command` - Command string to send (typically with terminator like `\r\n`)
    ///
    /// # Errors
    ///
    /// * `AdapterError::NotConnected` - Not connected to hardware
    /// * `AdapterError::SendFailed` - Failed to transmit command
    /// * `AdapterError::Io` - Low-level I/O error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// adapter.send("*RST\r\n").await?;  // Reset instrument
    /// adapter.send("OUTPUT ON\r\n").await?;  // Enable output
    /// ```
    async fn send(&mut self, command: &str) -> Result<()>;

    /// Query the hardware.
    ///
    /// Sends a query command and waits for the response.
    /// Combines send + receive in one operation.
    ///
    /// # Arguments
    ///
    /// * `query` - Query string to send (typically with terminator like `\r\n`)
    ///
    /// # Returns
    ///
    /// Response string from the instrument
    ///
    /// # Errors
    ///
    /// * `AdapterError::NotConnected` - Not connected to hardware
    /// * `AdapterError::QueryFailed` - Failed to receive response or parse it
    /// * `AdapterError::Io` - Low-level I/O error or timeout
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let id = adapter.query("*IDN?\r\n").await?;
    /// println!("Instrument: {}", id);
    ///
    /// let power = adapter.query("READ:POWER?\r\n").await?;
    /// let power_value: f64 = power.trim().parse()?;
    /// ```
    async fn query(&mut self, query: &str) -> Result<String>;
}
