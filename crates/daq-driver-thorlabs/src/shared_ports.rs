//! Shared port management for RS-485 multidrop bus devices.
//!
//! Multiple ELL14 devices can share a single serial port. This module
//! provides a static registry to track and reuse open ports.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;

/// Trait for types that can be used as async serial ports.
pub trait SerialPortIO: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> SerialPortIO for T {}

/// Dynamic serial port type.
pub type DynSerial = Box<dyn SerialPortIO>;

/// Shared serial port wrapped in async mutex.
pub type SharedPort = Arc<Mutex<DynSerial>>;

/// Module-local registry for shared serial ports.
static SHARED_PORTS: OnceLock<RwLock<HashMap<String, SharedPort>>> = OnceLock::new();

fn port_registry() -> &'static RwLock<HashMap<String, SharedPort>> {
    SHARED_PORTS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Get an existing shared port if one is already open for the given path.
pub fn get_existing_port(port_path: &str) -> Option<SharedPort> {
    let registry = port_registry().read();
    registry.get(port_path).cloned()
}

/// Register a newly opened port in the shared registry.
pub fn register_port(port_path: &str, port: SharedPort) {
    let mut registry = port_registry().write();
    registry.insert(port_path.to_string(), port);
    tracing::info!(port = port_path, "Registered new ELL14 shared port");
}

/// Remove a port from the registry (e.g., when it becomes stale).
pub fn remove_port(port_path: &str) -> bool {
    let mut registry = port_registry().write();
    let removed = registry.remove(port_path).is_some();
    if removed {
        tracing::info!(
            port = port_path,
            "Removed stale ELL14 shared port from registry"
        );
    }
    removed
}

/// Get or create a shared port for the given path with default timeout (500ms).
///
/// If a port is already open for this path, returns the existing connection.
/// Otherwise, opens a new port and registers it.
pub async fn get_or_open_port(port_path: &str) -> anyhow::Result<SharedPort> {
    get_or_open_port_with_timeout(port_path, std::time::Duration::from_millis(500)).await
}

/// Get or create a shared port for the given path with custom timeout.
///
/// If a port is already open for this path, performs a health check and returns
/// the existing connection if healthy. Otherwise, opens a new port and registers it.
pub async fn get_or_open_port_with_timeout(
    port_path: &str,
    timeout: std::time::Duration,
) -> anyhow::Result<SharedPort> {
    use tokio::io::AsyncWriteExt;

    // Check if already open
    if let Some(port) = get_existing_port(port_path) {
        // Health check: try to flush the port to verify it's still connected
        let health_check = async {
            let mut guard = port.lock().await;
            guard.flush().await
        };

        match tokio::time::timeout(std::time::Duration::from_millis(100), health_check).await {
            Ok(Ok(())) => {
                tracing::debug!(port = port_path, "Reusing healthy ELL14 shared port");
                return Ok(port);
            }
            Ok(Err(e)) => {
                tracing::warn!(port = port_path, error = %e, "ELL14 shared port health check failed, reopening");
                remove_port(port_path);
            }
            Err(_) => {
                tracing::warn!(
                    port = port_path,
                    "ELL14 shared port health check timed out, reopening"
                );
                remove_port(port_path);
            }
        }
    }

    // Open new port
    let port_path_owned = port_path.to_string();
    let port =
        tokio::task::spawn_blocking(move || open_serial_port(&port_path_owned, timeout)).await??;

    let shared: SharedPort = Arc::new(Mutex::new(Box::new(port)));

    // Store in registry
    register_port(port_path, shared.clone());

    Ok(shared)
}

/// Open a serial port with ELL14 default settings.
fn open_serial_port(
    port_path: &str,
    timeout: std::time::Duration,
) -> anyhow::Result<tokio_serial::SerialStream> {
    use tokio_serial::SerialPortBuilderExt;

    let port = tokio_serial::new(port_path, 9600)
        .data_bits(tokio_serial::DataBits::Eight)
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .flow_control(tokio_serial::FlowControl::None)
        .timeout(timeout)
        .open_native_async()?;

    tracing::info!(port = port_path, timeout_ms = ?timeout.as_millis(), "Opened ELL14 serial port");
    Ok(port)
}

/// Close all shared ports (for cleanup/testing).
pub fn close_all_ports() {
    if let Some(registry) = SHARED_PORTS.get() {
        let mut guard = registry.write();
        let count = guard.len();
        guard.clear();
        tracing::info!(count, "Closed all shared ELL14 ports");
    }
}

/// Get the number of currently open shared ports.
pub fn port_count() -> usize {
    SHARED_PORTS.get().map(|r| r.read().len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_registry_initializes() {
        // Just verify we can access the registry without panicking
        // port_count() returns usize, so it's always >= 0
        let _count = port_count();
    }

    #[test]
    fn test_close_all_ports() {
        // Should not panic even if no ports are open
        close_all_ports();
    }
}
