//! DriverFactory implementations for Comedi DAQ devices.
//!
//! This module provides registry-compatible factories for integrating Comedi
//! devices into the rust-daq ecosystem via the DriverFactory pattern.
//!
//! # Supported Drivers
//!
//! - [`ComediAnalogInputFactory`] - Multi-channel analog input for power monitoring
//! - [`ComediAnalogOutputFactory`] - Analog output for feedback control
//!
//! # Hardware Tested
//!
//! - NI PCI-MIO-16XE-10 (16-ch AI, 2-ch AO, DIO, counters)
//!
//! # Example Configuration
//!
//! ```toml
//! [[devices]]
//! id = "ni_daq_ai"
//! type = "comedi_analog_input"
//! enabled = true
//!
//! [devices.config]
//! device = "/dev/comedi0"
//! channel = 0
//! range_index = 0
//! ```

use anyhow::{Context, Result};
use async_trait::async_trait;
use daq_core::capabilities::{Parameterized, Readable};
use daq_core::driver::{Capability, DeviceComponents, DeviceMetadata, DriverFactory};
use daq_core::observable::ParameterSet;
use daq_core::parameter::Parameter;
use futures::future::BoxFuture;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::device::ComediDevice;
use crate::subsystem::analog_input::AnalogInput;
use crate::subsystem::analog_output::AnalogOutput;
use crate::subsystem::Range;

// =============================================================================
// Configuration Types
// =============================================================================

/// Configuration for Comedi analog input driver.
#[derive(Debug, Clone, Deserialize)]
pub struct ComediAnalogInputConfig {
    /// Path to Comedi device (e.g., "/dev/comedi0")
    #[serde(default = "default_device")]
    pub device: String,

    /// Analog input channel number (0-15 for 16-ch cards)
    #[serde(default)]
    pub channel: u32,

    /// Voltage range index (0 = default, typically ±10V)
    #[serde(default)]
    pub range_index: u32,

    /// Human-readable name for the measurement
    #[serde(default)]
    pub measurement_name: Option<String>,

    /// Measurement units (default: "V")
    #[serde(default = "default_units")]
    pub units: String,

    /// Enable mock mode for testing without hardware
    #[serde(default)]
    pub mock: bool,
}

fn default_device() -> String {
    "/dev/comedi0".to_string()
}

fn default_units() -> String {
    "V".to_string()
}

/// Configuration for Comedi analog output driver.
#[derive(Debug, Clone, Deserialize)]
pub struct ComediAnalogOutputConfig {
    /// Path to Comedi device
    #[serde(default = "default_device")]
    pub device: String,

    /// Analog output channel number
    #[serde(default)]
    pub channel: u32,

    /// Voltage range index
    #[serde(default)]
    pub range_index: u32,

    /// Output units (default: "V")
    #[serde(default = "default_units")]
    pub units: String,

    /// Enable mock mode
    #[serde(default)]
    pub mock: bool,
}

// =============================================================================
// Mock Implementations (for testing without hardware)
// =============================================================================

/// Mock analog input for testing.
struct MockAnalogInput {
    channel: u32,
    value: RwLock<f64>,
}

impl MockAnalogInput {
    fn new(channel: u32) -> Self {
        Self {
            channel,
            value: RwLock::new(0.5 + 0.1 * channel as f64), // Simulated offset per channel
        }
    }

    async fn read_voltage(&self) -> Result<f64> {
        // Simulate noise
        let base = *self.value.read().await;
        let noise = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos() as f64
            / 1e9
            - 0.5)
            * 0.01;
        Ok(base + noise)
    }
}

/// Mock analog output for testing.
struct MockAnalogOutput {
    #[allow(dead_code)]
    channel: u32,
    value: RwLock<f64>,
}

impl MockAnalogOutput {
    fn new(channel: u32) -> Self {
        Self {
            channel,
            value: RwLock::new(0.0),
        }
    }

    async fn write_voltage(&self, voltage: f64) -> Result<()> {
        *self.value.write().await = voltage;
        Ok(())
    }

    async fn read_voltage(&self) -> Result<f64> {
        Ok(*self.value.read().await)
    }
}

// =============================================================================
// ComediAnalogInputDriver
// =============================================================================

/// Driver for Comedi analog input channels.
///
/// This driver wraps a single analog input channel and exposes it via the
/// rust-daq `Readable` trait for integration with the experiment engine.
///
/// # Use Cases
///
/// - Power monitoring via photodiode signal
/// - Voltage measurements
/// - General analog signal acquisition
///
/// # Example
///
/// ```rust,ignore
/// let driver = ComediAnalogInputDriver::new_async("/dev/comedi0", 0, 0, false).await?;
/// let voltage = driver.read().await?;
/// println!("Channel 0: {:.3} V", voltage);
/// ```
pub struct ComediAnalogInputDriver {
    /// Real hardware (None if mock mode)
    device: Option<ComediDevice>,
    analog_input: Option<AnalogInput>,

    /// Mock implementation (Some if mock mode)
    mock: Option<MockAnalogInput>,

    /// Channel being read
    channel: u32,

    /// Range index
    range_index: u32,

    /// Parameter registry
    params: Arc<ParameterSet>,

    /// Voltage reading parameter (updated on each read)
    voltage: Parameter<f64>,

    /// Channel parameter (read-only info)
    channel_param: Parameter<f64>,
}

impl ComediAnalogInputDriver {
    /// Create a new analog input driver.
    ///
    /// # Arguments
    /// * `device_path` - Path to Comedi device (e.g., "/dev/comedi0")
    /// * `channel` - Analog input channel number
    /// * `range_index` - Voltage range index
    /// * `mock` - If true, use mock implementation
    pub async fn new_async(
        device_path: &str,
        channel: u32,
        range_index: u32,
        mock: bool,
    ) -> Result<Arc<Self>> {
        let mut params = ParameterSet::new();

        // Create parameters
        let voltage = Parameter::new("voltage", 0.0)
            .with_description("Last voltage reading")
            .with_unit("V");

        let channel_param =
            Parameter::new("channel", channel as f64).with_description("Analog input channel");

        params.register(voltage.clone());
        params.register(channel_param.clone());

        let driver = if mock {
            info!("Creating mock Comedi analog input driver (channel={})", channel);
            Self {
                device: None,
                analog_input: None,
                mock: Some(MockAnalogInput::new(channel)),
                channel,
                range_index,
                params: Arc::new(params),
                voltage,
                channel_param,
            }
        } else {
            info!(
                "Opening Comedi device {} for analog input (channel={}, range={})",
                device_path, channel, range_index
            );

            // Open device in blocking task (FFI is blocking)
            let path = device_path.to_string();
            let device = tokio::task::spawn_blocking(move || ComediDevice::open(&path))
                .await
                .context("Task join error")?
                .context("Failed to open Comedi device")?;

            let ai = device
                .analog_input()
                .context("Failed to get analog input subsystem")?;

            // Validate channel
            let n_channels = ai.n_channels();
            if channel >= n_channels {
                anyhow::bail!(
                    "Channel {} out of range (device has {} channels)",
                    channel,
                    n_channels
                );
            }

            info!(
                "Opened {} ({}), channel {}/{}, {}-bit resolution",
                device.board_name(),
                device.driver_name(),
                channel,
                n_channels,
                ai.resolution_bits()
            );

            Self {
                device: Some(device),
                analog_input: Some(ai),
                mock: None,
                channel,
                range_index,
                params: Arc::new(params),
                voltage,
                channel_param,
            }
        };

        Ok(Arc::new(driver))
    }

    /// Read voltage from the configured channel.
    async fn read_voltage(&self) -> Result<f64> {
        if let Some(mock) = &self.mock {
            return mock.read_voltage().await;
        }

        let ai = self
            .analog_input
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No analog input subsystem"))?;

        let channel = self.channel;
        // Use default range (±10V) with specified index
        let range = Range {
            index: self.range_index,
            ..Range::default()
        };

        // Comedi FFI is blocking, run in blocking task
        let ai_clone = ai.clone();
        let voltage = tokio::task::spawn_blocking(move || ai_clone.read_voltage(channel, range))
            .await
            .context("Task join error")?
            .context("Failed to read voltage")?;

        debug!("Read voltage: channel={}, value={:.4}V", self.channel, voltage);
        Ok(voltage)
    }

    /// Get the channel number.
    pub fn channel(&self) -> u32 {
        self.channel
    }

    /// Get the device board name.
    pub fn board_name(&self) -> String {
        self.device
            .as_ref()
            .map(|d| d.board_name())
            .unwrap_or_else(|| "mock".to_string())
    }
}

impl Parameterized for ComediAnalogInputDriver {
    fn parameters(&self) -> &ParameterSet {
        &self.params
    }
}

#[async_trait]
impl Readable for ComediAnalogInputDriver {
    async fn read(&self) -> Result<f64> {
        self.read_voltage().await
    }
}

// =============================================================================
// ComediAnalogOutputDriver
// =============================================================================

/// Driver for Comedi analog output channels.
///
/// This driver wraps a single analog output channel for control applications
/// such as PID feedback loops.
pub struct ComediAnalogOutputDriver {
    /// Real hardware
    device: Option<ComediDevice>,
    analog_output: Option<AnalogOutput>,

    /// Mock implementation
    mock: Option<MockAnalogOutput>,

    /// Channel
    channel: u32,

    /// Range index
    range_index: u32,

    /// Parameter registry
    params: Arc<ParameterSet>,

    /// Output voltage parameter
    output: Parameter<f64>,
}

impl ComediAnalogOutputDriver {
    /// Create a new analog output driver.
    pub async fn new_async(
        device_path: &str,
        channel: u32,
        range_index: u32,
        mock: bool,
    ) -> Result<Arc<Self>> {
        let mut params = ParameterSet::new();

        let output = Parameter::new("output", 0.0)
            .with_description("Output voltage")
            .with_unit("V")
            .with_range(-10.0, 10.0);

        params.register(output.clone());

        let driver = if mock {
            info!("Creating mock Comedi analog output driver (channel={})", channel);
            Self {
                device: None,
                analog_output: None,
                mock: Some(MockAnalogOutput::new(channel)),
                channel,
                range_index,
                params: Arc::new(params),
                output,
            }
        } else {
            info!(
                "Opening Comedi device {} for analog output (channel={})",
                device_path, channel
            );

            let path = device_path.to_string();
            let device = tokio::task::spawn_blocking(move || ComediDevice::open(&path))
                .await
                .context("Task join error")?
                .context("Failed to open Comedi device")?;

            let ao = device
                .analog_output()
                .context("Failed to get analog output subsystem")?;

            Self {
                device: Some(device),
                analog_output: Some(ao),
                mock: None,
                channel,
                range_index,
                params: Arc::new(params),
                output,
            }
        };

        Ok(Arc::new(driver))
    }

    /// Write voltage to the output channel.
    pub async fn write_voltage(&self, voltage: f64) -> Result<()> {
        if let Some(mock) = &self.mock {
            return mock.write_voltage(voltage).await;
        }

        let ao = self
            .analog_output
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No analog output subsystem"))?;

        let channel = self.channel;
        let range = Range {
            index: self.range_index,
            ..Range::default()
        };

        let ao_clone = ao.clone();
        tokio::task::spawn_blocking(move || ao_clone.write_voltage(channel, voltage, range))
            .await
            .context("Task join error")?
            .context("Failed to write voltage")?;

        debug!("Wrote voltage: channel={}, value={:.4}V", self.channel, voltage);
        Ok(())
    }

    /// Read current output value (from cache, not hardware).
    ///
    /// Note: Comedi AO doesn't support hardware readback on all devices.
    /// This returns the cached value from the output parameter.
    pub async fn read_output(&self) -> Result<f64> {
        if let Some(mock) = &self.mock {
            return mock.read_voltage().await;
        }

        // Return cached parameter value (no hardware readback for AO)
        Ok(self.output.get())
    }

    /// Get output parameter for external control.
    pub fn output(&self) -> &Parameter<f64> {
        &self.output
    }
}

impl Parameterized for ComediAnalogOutputDriver {
    fn parameters(&self) -> &ParameterSet {
        &self.params
    }
}

// =============================================================================
// Factories
// =============================================================================

/// Factory for Comedi analog input drivers.
///
/// Creates single-channel analog input drivers suitable for power monitoring,
/// voltage measurements, and general signal acquisition.
pub struct ComediAnalogInputFactory;

static AI_CAPABILITIES: &[Capability] = &[Capability::Readable, Capability::Parameterized];

impl DriverFactory for ComediAnalogInputFactory {
    fn driver_type(&self) -> &'static str {
        "comedi_analog_input"
    }

    fn name(&self) -> &'static str {
        "Comedi Analog Input"
    }

    fn capabilities(&self) -> &'static [Capability] {
        AI_CAPABILITIES
    }

    fn validate(&self, config: &toml::Value) -> Result<()> {
        let cfg: ComediAnalogInputConfig = config
            .clone()
            .try_into()
            .context("Invalid Comedi analog input config")?;

        if cfg.device.is_empty() {
            anyhow::bail!("'device' path cannot be empty");
        }

        // Channel validation will happen at build time when we know device capabilities
        Ok(())
    }

    fn build(&self, config: toml::Value) -> BoxFuture<'static, Result<DeviceComponents>> {
        Box::pin(async move {
            let cfg: ComediAnalogInputConfig = config
                .try_into()
                .context("Invalid Comedi analog input config")?;

            let driver =
                ComediAnalogInputDriver::new_async(&cfg.device, cfg.channel, cfg.range_index, cfg.mock)
                    .await?;

            Ok(DeviceComponents {
                readable: Some(driver.clone()),
                parameterized: Some(driver),
                metadata: DeviceMetadata {
                    measurement_units: Some(cfg.units),
                    ..Default::default()
                },
                ..Default::default()
            })
        })
    }
}

/// Factory for Comedi analog output drivers.
pub struct ComediAnalogOutputFactory;

static AO_CAPABILITIES: &[Capability] = &[Capability::Parameterized];

impl DriverFactory for ComediAnalogOutputFactory {
    fn driver_type(&self) -> &'static str {
        "comedi_analog_output"
    }

    fn name(&self) -> &'static str {
        "Comedi Analog Output"
    }

    fn capabilities(&self) -> &'static [Capability] {
        AO_CAPABILITIES
    }

    fn validate(&self, config: &toml::Value) -> Result<()> {
        let cfg: ComediAnalogOutputConfig = config
            .clone()
            .try_into()
            .context("Invalid Comedi analog output config")?;

        if cfg.device.is_empty() {
            anyhow::bail!("'device' path cannot be empty");
        }

        Ok(())
    }

    fn build(&self, config: toml::Value) -> BoxFuture<'static, Result<DeviceComponents>> {
        Box::pin(async move {
            let cfg: ComediAnalogOutputConfig = config
                .try_into()
                .context("Invalid Comedi analog output config")?;

            let driver =
                ComediAnalogOutputDriver::new_async(&cfg.device, cfg.channel, cfg.range_index, cfg.mock)
                    .await?;

            Ok(DeviceComponents {
                parameterized: Some(driver),
                metadata: DeviceMetadata {
                    measurement_units: Some(cfg.units),
                    ..Default::default()
                },
                ..Default::default()
            })
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_factory_type() {
        let factory = ComediAnalogInputFactory;
        assert_eq!(factory.driver_type(), "comedi_analog_input");
        assert_eq!(factory.name(), "Comedi Analog Input");
    }

    #[test]
    fn test_ao_factory_type() {
        let factory = ComediAnalogOutputFactory;
        assert_eq!(factory.driver_type(), "comedi_analog_output");
        assert_eq!(factory.name(), "Comedi Analog Output");
    }

    #[test]
    fn test_ai_capabilities() {
        let factory = ComediAnalogInputFactory;
        let caps = factory.capabilities();
        assert!(caps.contains(&Capability::Readable));
        assert!(caps.contains(&Capability::Parameterized));
    }

    #[tokio::test]
    async fn test_ai_factory_validate() {
        let factory = ComediAnalogInputFactory;

        // Valid config
        let valid = toml::toml! {
            device = "/dev/comedi0"
            channel = 0
        };
        assert!(factory.validate(&toml::Value::Table(valid)).is_ok());

        // Empty device
        let empty_device = toml::toml! {
            device = ""
        };
        assert!(factory.validate(&toml::Value::Table(empty_device)).is_err());
    }

    #[tokio::test]
    async fn test_ai_mock_driver() {
        let driver = ComediAnalogInputDriver::new_async("/dev/comedi0", 0, 0, true)
            .await
            .expect("Failed to create mock driver");

        // Read should work
        let voltage = driver.read().await.expect("Failed to read");
        assert!(voltage.is_finite());

        // Parameters should be registered
        let params = driver.parameters();
        assert!(params.names().contains(&"voltage"));
        assert!(params.names().contains(&"channel"));
    }

    #[tokio::test]
    async fn test_ao_mock_driver() {
        let driver = ComediAnalogOutputDriver::new_async("/dev/comedi0", 0, 0, true)
            .await
            .expect("Failed to create mock driver");

        // Write and read back
        driver.write_voltage(2.5).await.expect("Failed to write");
        let readback = driver.read_output().await.expect("Failed to read");
        assert!((readback - 2.5).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_ai_factory_build_mock() {
        let factory = ComediAnalogInputFactory;

        let config = toml::toml! {
            device = "/dev/comedi0"
            channel = 0
            mock = true
        };

        let result = factory.build(toml::Value::Table(config)).await;
        assert!(result.is_ok());

        let components = result.unwrap();
        assert!(components.readable.is_some());
        assert!(components.parameterized.is_some());
    }
}
