//! V4 Serial Hardware Adapter
//!
//! Lightweight wrapper around the existing SerialAdapter for V4 actors.
//! Provides async serial communication for instruments like Newport 1830-C.

use crate::adapters::SerialAdapter as LegacySerialAdapter;
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Builder for constructing SerialAdapterV4 with custom configuration
///
/// Provides a safe, fluent interface for configuring serial adapters
/// while preserving sensible defaults.
///
/// # Example
/// ```no_run
/// use std::time::Duration;
/// use rust_daq::hardware::SerialAdapterV4Builder;
///
/// let adapter = SerialAdapterV4Builder::new("/dev/ttyUSB0".to_string(), 9600)
///     .with_timeout(Duration::from_millis(500))
///     .build();
/// ```
pub struct SerialAdapterV4Builder {
    port_name: String,
    baud_rate: u32,
    timeout: Duration,
    line_terminator: String,
    response_delimiter: char,
}

impl SerialAdapterV4Builder {
    /// Create a new builder with required parameters
    ///
    /// # Arguments
    /// * `port_name` - Serial port path (e.g., "/dev/ttyUSB0", "COM3")
    /// * `baud_rate` - Communication speed (e.g., 9600, 115200)
    ///
    /// Default configuration:
    /// * timeout: 1 second
    /// * line_terminator: "\r\n"
    /// * response_delimiter: '\n'
    pub fn new(port_name: String, baud_rate: u32) -> Self {
        Self {
            port_name,
            baud_rate,
            timeout: Duration::from_secs(1),
            line_terminator: "\r\n".to_string(),
            response_delimiter: '\n',
        }
    }

    /// Set the read timeout duration
    ///
    /// Default: 1 second
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the line terminator string for commands
    ///
    /// Default: "\r\n" (CRLF)
    pub fn with_line_terminator(mut self, terminator: String) -> Self {
        self.line_terminator = terminator;
        self
    }

    /// Set the response delimiter character
    ///
    /// Default: '\n' (newline)
    pub fn with_response_delimiter(mut self, delimiter: char) -> Self {
        self.response_delimiter = delimiter;
        self
    }

    /// Build the SerialAdapterV4 with the configured settings
    pub fn build(self) -> SerialAdapterV4 {
        let inner = LegacySerialAdapter::new(self.port_name, self.baud_rate)
            .with_timeout(self.timeout)
            .with_line_terminator(self.line_terminator)
            .with_response_delimiter(self.response_delimiter);

        SerialAdapterV4 {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

/// V4 Serial adapter for RS-232/USB-Serial instruments
///
/// This wraps the existing SerialAdapter to provide a V4-compatible
/// interface for Kameo actors. It maintains async I/O using Tokio's
/// blocking task executor.
#[derive(Clone)]
pub struct SerialAdapterV4 {
    /// The underlying legacy serial adapter
    inner: Arc<Mutex<LegacySerialAdapter>>,
}

impl SerialAdapterV4 {
    /// Create a new V4 serial adapter with default configuration
    ///
    /// # Arguments
    /// * `port_name` - Serial port path (e.g., "/dev/ttyUSB0", "COM3")
    /// * `baud_rate` - Communication speed (e.g., 9600, 115200)
    ///
    /// # Default Configuration
    /// * timeout: 1 second
    /// * line_terminator: "\r\n"
    /// * response_delimiter: '\n'
    ///
    /// For custom configuration, use `SerialAdapterV4Builder`.
    ///
    /// # Example
    /// ```no_run
    /// use rust_daq::hardware::SerialAdapterV4;
    ///
    /// let adapter = SerialAdapterV4::new("/dev/ttyUSB0".to_string(), 9600);
    /// ```
    pub fn new(port_name: String, baud_rate: u32) -> Self {
        SerialAdapterV4Builder::new(port_name, baud_rate).build()
    }

    /// Connect to the serial port
    pub async fn connect(&self) -> Result<()> {
        use daq_core::{AdapterConfig, HardwareAdapter};

        let mut adapter = self.inner.lock().await;
        adapter
            .connect(&AdapterConfig::default())
            .await
            .context("Failed to connect serial adapter")
    }

    /// Disconnect from the serial port
    pub async fn disconnect(&self) -> Result<()> {
        use daq_core::HardwareAdapter;

        let mut adapter = self.inner.lock().await;
        adapter
            .disconnect()
            .await
            .context("Failed to disconnect serial adapter")
    }

    /// Send a command and read the response
    ///
    /// # Arguments
    /// * `command` - The command string to send (line terminator added automatically)
    ///
    /// # Returns
    /// The trimmed response string
    ///
    /// # Example (Newport 1830-C)
    /// ```no_run
    /// # async fn example() -> anyhow::Result<()> {
    /// # use rust_daq::hardware::SerialAdapterV4;
    /// let adapter = SerialAdapterV4::new("/dev/ttyUSB0".to_string(), 9600);
    /// adapter.connect().await?;
    ///
    /// // Set wavelength to 780 nm
    /// adapter.send_command("PM:Lambda 780").await?;
    ///
    /// // Read power
    /// let response = adapter.send_command("PM:Power?").await?;
    /// let power: f64 = response.parse()?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_command(&self, command: &str) -> Result<String> {
        self.inner
            .lock()
            .await
            .send_command(command)
            .await
            .context("Serial command failed")
    }

    /// Check if the adapter is connected
    pub async fn is_connected(&self) -> bool {
        use daq_core::HardwareAdapter;
        self.inner.lock().await.is_connected()
    }

    /// Get adapter information
    pub async fn info(&self) -> String {
        use daq_core::HardwareAdapter;
        self.inner.lock().await.info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serial_adapter_creation() {
        let adapter = SerialAdapterV4::new("/dev/ttyUSB0".to_string(), 9600);
        // Should not panic
        drop(adapter);
    }

    #[test]
    fn test_serial_adapter_builder_defaults() {
        let adapter = SerialAdapterV4Builder::new("/dev/ttyUSB0".to_string(), 9600).build();
        // Should create successfully with defaults
        drop(adapter);
    }

    #[test]
    fn test_serial_adapter_builder_with_custom_timeout() {
        let adapter = SerialAdapterV4Builder::new("/dev/ttyUSB0".to_string(), 9600)
            .with_timeout(Duration::from_millis(500))
            .build();
        // Should create successfully with custom timeout
        drop(adapter);
    }

    #[test]
    fn test_serial_adapter_builder_full_customization() {
        let adapter = SerialAdapterV4Builder::new("/dev/ttyUSB0".to_string(), 9600)
            .with_timeout(Duration::from_millis(500))
            .with_line_terminator("\n".to_string())
            .with_response_delimiter('\r')
            .build();
        // Should create successfully with all custom settings
        drop(adapter);
    }

    #[test]
    fn test_serial_adapter_builder_no_panic_after_clone() {
        let adapter = SerialAdapterV4Builder::new("/dev/ttyUSB0".to_string(), 9600)
            .with_timeout(Duration::from_millis(500))
            .build();
        let _cloned = adapter.clone();
        // No panic should occur after cloning
        drop(adapter);
        drop(_cloned);
    }
}
