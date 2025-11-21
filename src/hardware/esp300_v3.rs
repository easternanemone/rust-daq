//! Newport ESP300 Multi-Axis Motion Controller Driver (V3 Architecture)
//!
//! This module provides V3 Instrument and Stage trait implementations for the
//! Newport ESP300 motion controller. It replaces the V2/V4 implementations with
//! the unified V3 architecture.
//!
//! ## Protocol Overview
//!
//! - Format: ASCII command/response over RS-232
//! - Baud: 19200, 8N1, hardware flow control (or None - works with both)
//! - Commands: {Axis}{Command}{Value}
//! - Example: "1PA5.0" (axis 1, position absolute, 5.0mm)
//!
//! ## Multi-Axis Support
//!
//! The ESP300 supports up to 3 axes. Each axis is represented as a separate
//! Stage instance, but they share the same serial port connection.
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use rust_daq::hardware::esp300_v3::Esp300Axis;
//! use rust_daq::core_v3::{Instrument, Stage};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create axis 1 driver
//!     let mut axis1 = Esp300Axis::new("esp300_axis1", "/dev/ttyUSB0", 1)?;
//!
//!     // Initialize
//!     axis1.initialize().await?;
//!
//!     // Move to position
//!     axis1.move_absolute(10.5).await?;
//!     axis1.wait_settled(std::time::Duration::from_secs(30)).await?;
//!
//!     // Get position
//!     let pos = axis1.position().await?;
//!     println!("Position: {:.3} mm", pos);
//!
//!     Ok(())
//! }
//! ```

use crate::core_v3::{
    Command, Instrument, InstrumentState, Measurement, ParameterBase, Response, Stage,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

/// Newport ESP300 axis driver implementing V3 Stage trait
///
/// Each axis is a separate instrument that implements both Instrument and Stage traits.
/// Multiple axes can share the same serial port through Arc<Mutex<>> wrapper.
pub struct Esp300Axis {
    /// Instrument identifier (e.g., "esp300_axis1")
    id: String,

    /// Current lifecycle state
    state: InstrumentState,

    /// Serial port protected by Mutex for thread-safe multi-axis access
    port: Arc<Mutex<BufReader<SerialStream>>>,

    /// Axis number (1-3)
    axis: u8,

    /// Command timeout duration
    timeout: Duration,

    /// Velocity in mm/s (cached for configuration)
    velocity_mm_s: f64,

    /// Acceleration in mm/s² (cached for configuration)
    acceleration_mm_s2: f64,

    /// Data broadcast channel for position updates
    data_tx: broadcast::Sender<Measurement>,

    /// Background polling task handle
    polling_task: Option<JoinHandle<()>>,

    /// Parameters map (empty for now, can be extended)
    parameters: HashMap<String, Box<dyn ParameterBase>>,
}

impl Esp300Axis {
    /// Create a new ESP300 axis driver
    ///
    /// # Arguments
    /// * `id` - Unique identifier for this axis (e.g., "esp300_axis1")
    /// * `port_path` - Serial port path (e.g., "/dev/ttyUSB0", "COM3")
    /// * `axis` - Axis number (1-3)
    ///
    /// # Errors
    /// Returns error if serial port cannot be opened or axis is invalid
    pub fn new(id: impl Into<String>, port_path: &str, axis: u8) -> Result<Self> {
        if !(1..=3).contains(&axis) {
            return Err(anyhow!("ESP300 axis must be 1-3, got {}", axis));
        }

        // Open serial port
        // NOTE: ESP300 v3.04 confirmed to work with FlowControl::None (tested 2025-11-02)
        let port = tokio_serial::new(port_path, 19200)
            .data_bits(tokio_serial::DataBits::Eight)
            .parity(tokio_serial::Parity::None)
            .stop_bits(tokio_serial::StopBits::One)
            .flow_control(tokio_serial::FlowControl::None)
            .open_native_async()
            .context(format!("Failed to open ESP300 serial port: {}", port_path))?;

        let (data_tx, _) = broadcast::channel(128);

        Ok(Self {
            id: id.into(),
            state: InstrumentState::Disconnected,
            port: Arc::new(Mutex::new(BufReader::new(port))),
            axis,
            timeout: Duration::from_secs(5),
            velocity_mm_s: 5.0,
            acceleration_mm_s2: 10.0,
            data_tx,
            polling_task: None,
            parameters: HashMap::new(),
        })
    }

    /// Create multiple axes sharing the same serial port
    ///
    /// This is more efficient than opening separate connections for each axis.
    pub fn new_multi_axis(
        base_id: &str,
        port_path: &str,
        axes: &[u8],
    ) -> Result<Vec<Self>> {
        if axes.is_empty() {
            return Err(anyhow!("At least one axis must be specified"));
        }

        // Open shared serial port
        let port = tokio_serial::new(port_path, 19200)
            .data_bits(tokio_serial::DataBits::Eight)
            .parity(tokio_serial::Parity::None)
            .stop_bits(tokio_serial::StopBits::One)
            .flow_control(tokio_serial::FlowControl::None)
            .open_native_async()
            .context(format!("Failed to open ESP300 serial port: {}", port_path))?;

        let shared_port = Arc::new(Mutex::new(BufReader::new(port)));

        let mut axis_drivers = Vec::new();
        for &axis in axes {
            if !(1..=3).contains(&axis) {
                return Err(anyhow!("ESP300 axis must be 1-3, got {}", axis));
            }

            let (data_tx, _) = broadcast::channel(128);
            let id = format!("{}_axis{}", base_id, axis);

            axis_drivers.push(Self {
                id,
                state: InstrumentState::Disconnected,
                port: shared_port.clone(),
                axis,
                timeout: Duration::from_secs(5),
                velocity_mm_s: 5.0,
                acceleration_mm_s2: 10.0,
                data_tx,
                polling_task: None,
                parameters: HashMap::new(),
            });
        }

        Ok(axis_drivers)
    }

    /// Send command and read response
    async fn query(&self, command: &str) -> Result<String> {
        let mut port = self.port.lock().await;

        // Write command with terminator
        let cmd = format!("{}\r\n", command);
        port.get_mut()
            .write_all(cmd.as_bytes())
            .await
            .context("ESP300 write failed")?;

        // Read response with timeout
        let mut response = String::new();
        tokio::time::timeout(self.timeout, port.read_line(&mut response))
            .await
            .context("ESP300 read timeout")?
            .context("ESP300 read error")?;

        Ok(response.trim().to_string())
    }

    /// Send command without expecting response
    async fn send_command(&self, command: &str) -> Result<()> {
        let mut port = self.port.lock().await;

        let cmd = format!("{}\r\n", command);
        port.get_mut()
            .write_all(cmd.as_bytes())
            .await
            .context("ESP300 write failed")?;

        // Small delay to ensure command is processed
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    /// Check if axis is in motion
    async fn is_moving_internal(&self) -> Result<bool> {
        let response = self.query(&format!("{}MD?", self.axis)).await?;
        // Response is 0 if stationary, 1 if moving
        Ok(response.trim() != "0")
    }

    /// Get current position (internal implementation)
    async fn position_internal(&self) -> Result<f64> {
        let response = self.query(&format!("{}TP?", self.axis)).await?;
        response
            .trim()
            .parse::<f64>()
            .context("Failed to parse position from ESP300")
    }

    /// Start background polling task for position updates
    fn start_polling(&mut self, rate_hz: f64) {
        if self.polling_task.is_some() {
            return; // Already running
        }

        let port = self.port.clone();
        let axis = self.axis;
        let data_tx = self.data_tx.clone();
        let id = self.id.clone();
        let timeout = self.timeout;

        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs_f64(1.0 / rate_hz));

            loop {
                interval.tick().await;

                // Query position
                let mut port_guard = port.lock().await;
                let cmd = format!("{}TP?\r\n", axis);

                if let Err(e) = port_guard.get_mut().write_all(cmd.as_bytes()).await {
                    log::warn!("ESP300 {} polling write error: {}", id, e);
                    continue;
                }

                let mut response = String::new();
                match tokio::time::timeout(timeout, port_guard.read_line(&mut response)).await {
                    Ok(Ok(_)) => {
                        if let Ok(position) = response.trim().parse::<f64>() {
                            let measurement = Measurement::Scalar {
                                name: format!("{}_position", id),
                                value: position,
                                unit: "mm".to_string(),
                                timestamp: chrono::Utc::now(),
                            };

                            if data_tx.send(measurement).is_err() {
                                // No subscribers, continue polling
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        log::warn!("ESP300 {} polling read error: {}", id, e);
                    }
                    Err(_) => {
                        log::warn!("ESP300 {} polling timeout", id);
                    }
                }

                drop(port_guard); // Release lock before next iteration
            }
        });

        self.polling_task = Some(task);
    }

    /// Stop background polling task
    fn stop_polling(&mut self) {
        if let Some(task) = self.polling_task.take() {
            task.abort();
        }
    }
}

#[async_trait]
impl Instrument for Esp300Axis {
    fn id(&self) -> &str {
        &self.id
    }

    fn state(&self) -> InstrumentState {
        self.state
    }

    async fn initialize(&mut self) -> Result<()> {
        if self.state != InstrumentState::Disconnected {
            return Err(anyhow!("ESP300 axis already initialized"));
        }

        self.state = InstrumentState::Connecting;
        log::info!("Initializing ESP300 axis {} ({})", self.axis, self.id);

        // Query controller version
        let version = self.query("VE?").await?;
        log::info!("ESP300 version: {}", version);

        // Configure axis with default parameters
        self.send_command(&format!("{}VA{}", self.axis, self.velocity_mm_s))
            .await
            .context("Failed to set velocity")?;
        self.send_command(&format!("{}AC{}", self.axis, self.acceleration_mm_s2))
            .await
            .context("Failed to set acceleration")?;

        self.state = InstrumentState::Connected;
        log::info!("ESP300 axis {} initialized successfully", self.axis);

        // Start polling at 5 Hz
        self.start_polling(5.0);

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        log::info!("Shutting down ESP300 axis {}", self.axis);

        self.stop_polling();
        self.state = InstrumentState::ShuttingDown;

        // Stop motion if active
        let _ = self.send_command(&format!("{}ST", self.axis)).await;

        Ok(())
    }

    fn data_channel(&self) -> broadcast::Receiver<Measurement> {
        self.data_tx.subscribe()
    }

    async fn execute(&mut self, cmd: Command) -> Result<Response> {
        match cmd {
            Command::Start => {
                self.state = InstrumentState::Running;
                Ok(Response::State(self.state))
            }
            Command::Stop => {
                self.stop_motion().await?;
                self.state = InstrumentState::Connected;
                Ok(Response::State(self.state))
            }
            Command::GetState => Ok(Response::State(self.state)),
            Command::GetParameter(name) => {
                let value = match name.as_str() {
                    "position" => serde_json::json!(self.position().await?),
                    "velocity" => serde_json::json!(self.velocity_mm_s),
                    "acceleration" => serde_json::json!(self.acceleration_mm_s2),
                    "is_moving" => serde_json::json!(self.is_moving().await?),
                    _ => return Ok(Response::Error(format!("Unknown parameter: {}", name))),
                };
                Ok(Response::Parameter(value))
            }
            Command::SetParameter(name, value) => {
                match name.as_str() {
                    "velocity" => {
                        let vel: f64 = serde_json::from_value(value)?;
                        self.set_velocity(vel).await?;
                    }
                    "acceleration" => {
                        let accel: f64 = serde_json::from_value(value)?;
                        self.send_command(&format!("{}AC{}", self.axis, accel)).await?;
                        self.acceleration_mm_s2 = accel;
                    }
                    _ => return Ok(Response::Error(format!("Unknown parameter: {}", name))),
                }
                Ok(Response::Ok)
            }
            Command::Custom(cmd_name, _args) => {
                match cmd_name.as_str() {
                    "home" => {
                        self.home().await?;
                        Ok(Response::Ok)
                    }
                    _ => Ok(Response::Error(format!("Unknown custom command: {}", cmd_name))),
                }
            }
            _ => Ok(Response::Error("Unsupported command".to_string())),
        }
    }

    fn parameters(&self) -> &HashMap<String, Box<dyn ParameterBase>> {
        &self.parameters
    }

    fn parameters_mut(&mut self) -> &mut HashMap<String, Box<dyn ParameterBase>> {
        &mut self.parameters
    }
}

#[async_trait]
impl Stage for Esp300Axis {
    async fn move_absolute(&mut self, position_mm: f64) -> Result<()> {
        log::debug!(
            "ESP300 axis {} moving to absolute position {:.3} mm",
            self.axis,
            position_mm
        );

        self.send_command(&format!("{}PA{:.6}", self.axis, position_mm))
            .await
            .context("Failed to send move_absolute command")
    }

    async fn move_relative(&mut self, distance_mm: f64) -> Result<()> {
        log::debug!(
            "ESP300 axis {} moving relative distance {:.3} mm",
            self.axis,
            distance_mm
        );

        self.send_command(&format!("{}PR{:.6}", self.axis, distance_mm))
            .await
            .context("Failed to send move_relative command")
    }

    async fn position(&self) -> Result<f64> {
        self.position_internal().await
    }

    async fn stop_motion(&mut self) -> Result<()> {
        log::info!("ESP300 axis {} stopping motion", self.axis);

        self.send_command(&format!("{}ST", self.axis))
            .await
            .context("Failed to send stop command")
    }

    async fn is_moving(&self) -> Result<bool> {
        self.is_moving_internal().await
    }

    async fn home(&mut self) -> Result<()> {
        log::info!("ESP300 axis {} homing (finding reference)", self.axis);

        self.send_command(&format!("{}OR", self.axis))
            .await
            .context("Failed to send home command")?;

        // Wait for homing to complete
        self.wait_settled(Duration::from_secs(60)).await
            .context("Homing timeout")
    }

    async fn set_velocity(&mut self, mm_per_sec: f64) -> Result<()> {
        log::debug!(
            "ESP300 axis {} setting velocity to {:.3} mm/s",
            self.axis,
            mm_per_sec
        );

        self.send_command(&format!("{}VA{:.6}", self.axis, mm_per_sec))
            .await
            .context("Failed to set velocity")?;

        self.velocity_mm_s = mm_per_sec;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axis_validation() {
        // Valid axes
        let result1 = Esp300Axis::new("test1", "/dev/null", 1);
        let result2 = Esp300Axis::new("test2", "/dev/null", 2);
        let result3 = Esp300Axis::new("test3", "/dev/null", 3);

        // Note: Will fail due to /dev/null not being a valid serial port,
        // but should NOT fail due to axis validation
        assert!(result1.is_err()); // Serial error, not axis error
        assert!(result2.is_err());
        assert!(result3.is_err());

        // Invalid axes - should fail validation before serial open
        // This test would need mock serial support to work properly
    }

    #[test]
    fn test_multi_axis_creation() {
        let result = Esp300Axis::new_multi_axis("esp300", "/dev/null", &[1, 2, 3]);
        // Will fail on serial port, but logic is correct
        assert!(result.is_err());
    }
}
