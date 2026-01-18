//! Serial Port Abstractions for Driver Crates
//!
//! This module provides shared types for async serial communication that can be used
//! by driver crates without duplicating definitions.
//!
//! # Feature Flag
//!
//! This module requires the `serial` feature to be enabled:
//!
//! ```toml
//! [dependencies]
//! daq-core = { path = "../daq-core", features = ["serial"] }
//! ```
//!
//! # Types
//!
//! - [`SerialPortIO`]: Trait alias combining AsyncRead + AsyncWrite for serial ports
//! - [`DynSerial`]: Type-erased boxed serial port
//! - [`SharedPort`]: Thread-safe shared serial port with buffered reading
//! - [`SharedPortUnbuffered`]: Thread-safe shared serial port without buffering
//!
//! # Example
//!
//! ```rust,ignore
//! use daq_core::serial::{SerialPortIO, SharedPort};
//! use std::sync::Arc;
//! use tokio::sync::Mutex;
//! use tokio::io::BufReader;
//!
//! // Open a serial port and wrap it for sharing
//! let port = tokio_serial::new("/dev/ttyUSB0", 9600)
//!     .open_native_async()?;
//! let shared: SharedPort = Arc::new(Mutex::new(BufReader::new(Box::new(port))));
//!
//! // Use in multiple tasks
//! let port_clone = shared.clone();
//! tokio::spawn(async move {
//!     let mut guard = port_clone.lock().await;
//!     // Read and write operations
//! });
//! ```

use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, BufReader};
use tokio::sync::Mutex;

// =============================================================================
// Serial Port Trait
// =============================================================================

/// Trait alias for async serial port I/O.
///
/// Any type implementing `AsyncRead + AsyncWrite + Unpin + Send` can be used
/// as a serial port. This includes:
/// - `tokio_serial::SerialStream` (real hardware)
/// - `tokio::io::DuplexStream` (testing)
/// - Any mock implementing the async I/O traits
pub trait SerialPortIO: AsyncRead + AsyncWrite + Unpin + Send {}

// Blanket implementation for all types meeting the requirements
impl<T: AsyncRead + AsyncWrite + Unpin + Send> SerialPortIO for T {}

// =============================================================================
// Type Aliases
// =============================================================================

/// Type-erased boxed serial port.
///
/// Use this when you need to store a serial port without knowing its concrete type.
///
/// # Example
///
/// ```rust,ignore
/// use daq_core::serial::DynSerial;
///
/// fn create_port(path: &str) -> anyhow::Result<DynSerial> {
///     let port = tokio_serial::new(path, 9600).open_native_async()?;
///     Ok(Box::new(port))
/// }
/// ```
pub type DynSerial = Box<dyn SerialPortIO>;

/// Thread-safe shared serial port with buffered reading.
///
/// This is the primary type for sharing a serial port between multiple async tasks.
/// The `BufReader` wrapper enables efficient line-by-line reading.
///
/// # Why BufReader?
///
/// Many serial protocols (RS-232, ASCII command/response) use line-delimited messages.
/// `BufReader` enables `read_line()` and `read_until()` methods that accumulate data
/// until a delimiter is found.
///
/// # Example
///
/// ```rust,ignore
/// use daq_core::serial::SharedPort;
/// use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
///
/// async fn send_query(port: &SharedPort, command: &str) -> anyhow::Result<String> {
///     let mut guard = port.lock().await;
///
///     // Write command
///     guard.get_mut().write_all(format!("{}\r\n", command).as_bytes()).await?;
///
///     // Read response
///     let mut response = String::new();
///     guard.read_line(&mut response).await?;
///
///     Ok(response.trim().to_string())
/// }
/// ```
pub type SharedPort = Arc<Mutex<BufReader<DynSerial>>>;

/// Thread-safe shared serial port without buffering.
///
/// Use this when you need direct byte-level access without `BufReader` overhead.
/// Suitable for binary protocols or when managing buffering manually.
///
/// # Example
///
/// ```rust,ignore
/// use daq_core::serial::SharedPortUnbuffered;
/// use tokio::io::{AsyncReadExt, AsyncWriteExt};
///
/// async fn read_bytes(port: &SharedPortUnbuffered, buf: &mut [u8]) -> anyhow::Result<usize> {
///     let mut guard = port.lock().await;
///     let n = guard.read(buf).await?;
///     Ok(n)
/// }
/// ```
pub type SharedPortUnbuffered = Arc<Mutex<DynSerial>>;

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a SharedPort from a type-erased serial port.
///
/// # Example
///
/// ```rust,ignore
/// use daq_core::serial::{DynSerial, SharedPort, wrap_shared};
///
/// let port: DynSerial = Box::new(tokio_serial::new("/dev/ttyUSB0", 9600).open_native_async()?);
/// let shared: SharedPort = wrap_shared(port);
/// ```
pub fn wrap_shared(port: DynSerial) -> SharedPort {
    Arc::new(Mutex::new(BufReader::new(port)))
}

/// Create a SharedPortUnbuffered from a type-erased serial port.
///
/// # Example
///
/// ```rust,ignore
/// use daq_core::serial::{DynSerial, SharedPortUnbuffered, wrap_shared_unbuffered};
///
/// let port: DynSerial = Box::new(tokio_serial::new("/dev/ttyUSB0", 9600).open_native_async()?);
/// let shared: SharedPortUnbuffered = wrap_shared_unbuffered(port);
/// ```
pub fn wrap_shared_unbuffered(port: DynSerial) -> SharedPortUnbuffered {
    Arc::new(Mutex::new(port))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_shared_port_with_duplex() {
        // Create a duplex stream for testing
        let (mut host, device) = tokio::io::duplex(64);
        let port: SharedPort = wrap_shared(Box::new(device));

        // Write from host side
        host.write_all(b"Hello\n").await.unwrap();

        // Read from shared port side
        let mut guard = port.lock().await;
        let mut line = String::new();
        guard.read_line(&mut line).await.unwrap();

        assert_eq!(line.trim(), "Hello");
    }

    #[tokio::test]
    async fn test_shared_port_unbuffered_with_duplex() {
        // Create a duplex stream for testing
        let (mut host, device) = tokio::io::duplex(64);
        let port: SharedPortUnbuffered = wrap_shared_unbuffered(Box::new(device));

        // Write from host side
        host.write_all(b"test").await.unwrap();

        // Read from shared port side
        let mut guard = port.lock().await;
        let mut buf = [0u8; 4];
        let n = guard.read(&mut buf).await.unwrap();

        assert_eq!(n, 4);
        assert_eq!(&buf, b"test");
    }

    #[tokio::test]
    async fn test_shared_port_clone() {
        // Verify that SharedPort can be cloned for use in multiple tasks
        let (mut host, device) = tokio::io::duplex(64);
        let port: SharedPort = wrap_shared(Box::new(device));

        // Clone for another task
        let port_clone = port.clone();

        // Write from host
        host.write_all(b"data\n").await.unwrap();

        // Read from clone - should work since it's the same underlying port
        let mut guard = port_clone.lock().await;
        let mut line = String::new();
        guard.read_line(&mut line).await.unwrap();

        assert_eq!(line.trim(), "data");
    }
}
