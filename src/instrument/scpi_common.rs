//! Common SCPI communication abstractions.
//!
//! This module provides reusable components for SCPI-based instruments,
//! eliminating code duplication across serial instrument drivers.

use crate::config::Settings;
use crate::core::DataPoint;
use crate::error::DaqError;
use crate::instrument::serial_helper;
use crate::adapters::serial::SerialAdapter;
use anyhow::{Context, Result};
use async_trait::async_trait;
use daq_core::Measurement;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::{interval, sleep};

/// Trait for SCPI communication transports.
///
/// Abstracts the underlying communication mechanism (serial, VISA, TCP)
/// to enable protocol-agnostic SCPI operations.
#[async_trait]
pub trait ScpiTransport: Send + Sync {
    /// Send a query command and return the response.
    async fn query(&self, command: &str) -> Result<String>;
    
    /// Send a command without expecting a response.
    async fn command(&self, command: &str) -> Result<()>;
}

/// Serial-based SCPI transport implementation.
///
/// Wraps SerialAdapter with SCPI-specific communication parameters.
pub struct SerialScpiTransport {
    adapter: Arc<SerialAdapter>,
    terminator: &'static str,
    delimiter: u8,
    timeout: Duration,
}

impl SerialScpiTransport {
    /// Create transport for standard RS-232 SCPI instruments.
    ///
    /// Uses:
    /// - Terminator: "\r\n"
    /// - Delimiter: b'\n'
    /// - Timeout: 1 second
    pub fn new_rs232(adapter: SerialAdapter, settings: &Settings) -> Self {
        let timeout = Duration::from_millis(settings.application.timeouts.serial_read_timeout_ms);
        Self {
            adapter: Arc::new(adapter),
            terminator: "\r\n",
            delimiter: b'\n',
            timeout,
        }
    }
    
    /// Create transport for MaiTai laser (non-standard parameters).
    ///
    /// Uses:
    /// - Terminator: "\r"
    /// - Delimiter: b'\r'
    /// - Timeout: 2 seconds
    pub fn new_maitai(adapter: SerialAdapter, settings: &Settings) -> Self {
        let timeout = Duration::from_millis(settings.application.timeouts.scpi_command_timeout_ms);
        Self {
            adapter: Arc::new(adapter),
            terminator: "\r",
            delimiter: b'\r',
            timeout,
        }
    }
}

#[async_trait]
impl ScpiTransport for SerialScpiTransport {
    async fn query(&self, command: &str) -> Result<String> {
        serial_helper::send_command_async(
            &self.adapter,
            command,
            self.terminator,
            self.delimiter,
            self.timeout,
        )
        .await
        .context("SCPI query failed")
    }
    
    async fn command(&self, command: &str) -> Result<()> {
        self.query(command).await?;
        Ok(())
    }
}

/// Open a serial instrument and extract configuration.
///
/// Helper function to eliminate duplication of serial port opening logic.
///
/// Returns: (SerialAdapter, instrument config HashMap)
pub fn open_serial_instrument(
    settings: &Settings,
    id: &str,
) -> Result<(SerialAdapter, HashMap<String, toml::Value>)> {
    let instrument_config = settings
        .instruments
        .get(id)
        .ok_or_else(|| anyhow::anyhow!("Instrument '{}' not found in settings", id))?
        .clone();

    let port = instrument_config
        .get("port")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'port' in instrument config"))?;

    let baud_rate = instrument_config
        .get("baud_rate")
        .and_then(|v| v.as_integer())
        .unwrap_or(9600) as u32;

    let serial_port = serialport::new(port, baud_rate)
        .timeout(Duration::from_millis(100))
        .open()
        .with_context(|| format!("Failed to open serial port {}", port))?;

    let adapter = SerialAdapter::new(serial_port);
    Ok((adapter, instrument_config))
}

/// Parse a floating-point response from SCPI query.
///
/// Handles common SCPI response formats and error messages.
pub fn parse_f64_response(response: &str) -> Result<f64> {
    response
        .trim()
        .parse::<f64>()
        .with_context(|| format!("Failed to parse SCPI response as f64: '{}'", response))
}

/// Spawn a polling task for periodic SCPI queries.
///
/// Generic helper to eliminate duplication of polling task structure.
///
/// # Arguments
/// * `interval_duration` - Time between polls
/// * `broadcast_tx` - Channel for sending measurements
/// * `query_fn` - Async function that performs the query and returns a measurement
pub fn spawn_polling_task<F, Fut>(
    interval_duration: Duration,
    retry_delay: Duration,
    broadcast_tx: broadcast::Sender<Measurement>,
    mut query_fn: F,
) -> tokio::task::JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Measurement>> + Send,
{
    tokio::spawn(async move {
        let mut ticker = interval(interval_duration);
        loop {
            ticker.tick().await;
            
            match query_fn().await {
                Ok(measurement) => {
                    if broadcast_tx.send(measurement).is_err() {
                        log::warn!("No active receivers, stopping polling");
                        break;
                    }
                }
                Err(e) => {
                    log::error!("Polling query failed: {:?}", e);
                    sleep(retry_delay).await;
                }
            }
        }
    })
}
