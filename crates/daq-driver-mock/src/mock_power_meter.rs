//! Mock power meter implementation.

use anyhow::Result;
use async_trait::async_trait;
use daq_core::capabilities::{Parameterized, Readable};
use daq_core::driver::{Capability, DeviceComponents, DriverFactory};
use daq_core::observable::ParameterSet;
use daq_core::parameter::Parameter;
use futures::future::BoxFuture;
use serde::Deserialize;
use std::sync::Arc;

// =============================================================================
// MockPowerMeterFactory - DriverFactory implementation
// =============================================================================

/// Configuration for MockPowerMeter driver
#[derive(Debug, Clone, Deserialize)]
pub struct MockPowerMeterConfig {
    /// Base power reading in Watts (default: 1.0)
    #[serde(default = "default_base_power")]
    pub base_power: f64,
}

fn default_base_power() -> f64 {
    1.0
}

impl Default for MockPowerMeterConfig {
    fn default() -> Self {
        Self { base_power: 1.0 }
    }
}

/// Factory for creating MockPowerMeter instances.
pub struct MockPowerMeterFactory;

/// Static capabilities for MockPowerMeter
static MOCK_POWER_METER_CAPABILITIES: &[Capability] =
    &[Capability::Readable, Capability::Parameterized];

impl DriverFactory for MockPowerMeterFactory {
    fn driver_type(&self) -> &'static str {
        "mock_power_meter"
    }

    fn name(&self) -> &'static str {
        "Mock Power Meter"
    }

    fn capabilities(&self) -> &'static [Capability] {
        MOCK_POWER_METER_CAPABILITIES
    }

    fn validate(&self, config: &toml::Value) -> Result<()> {
        let _: MockPowerMeterConfig = config.clone().try_into()?;
        Ok(())
    }

    fn build(&self, config: toml::Value) -> BoxFuture<'static, Result<DeviceComponents>> {
        Box::pin(async move {
            let cfg: MockPowerMeterConfig = config.try_into().unwrap_or_default();

            let meter = Arc::new(MockPowerMeter::new(cfg.base_power));

            Ok(DeviceComponents {
                readable: Some(meter.clone()),
                parameterized: Some(meter),
                ..Default::default()
            })
        })
    }
}

// =============================================================================
// MockPowerMeter - Simulated Power Meter
// =============================================================================

/// Mock power meter with simulated readings.
///
/// Simulates a power meter with:
/// - Configurable base power value
/// - Small random noise simulation (~1% variation)
/// - Units in Watts
///
/// # Example
///
/// ```rust,ignore
/// let meter = MockPowerMeter::new(2.5);
/// let reading = meter.read().await?;
/// assert!((reading - 2.5).abs() < 0.1);
/// ```
pub struct MockPowerMeter {
    base_power: Parameter<f64>,
    params: ParameterSet,
}

impl MockPowerMeter {
    /// Create new mock power meter with specified base power (Watts).
    ///
    /// # Arguments
    /// * `base_power` - Base power reading in Watts
    pub fn new(base_power: f64) -> Self {
        let mut params = ParameterSet::new();
        let power_param = Parameter::new("base_power", base_power)
            .with_description("Base power reading for simulated measurements")
            .with_unit("W")
            .with_range(0.0, 10.0); // 0 to 10W range

        params.register(power_param.clone());

        Self {
            base_power: power_param,
            params,
        }
    }

    /// Set the base power reading.
    pub async fn set_base_power(&self, power: f64) -> Result<()> {
        self.base_power.set(power).await
    }

    /// Get the current base power setting.
    pub fn get_base_power(&self) -> f64 {
        self.base_power.get()
    }
}

impl Parameterized for MockPowerMeter {
    fn parameters(&self) -> &ParameterSet {
        &self.params
    }
}

impl Default for MockPowerMeter {
    fn default() -> Self {
        Self::new(1.0)
    }
}

#[async_trait]
impl Readable for MockPowerMeter {
    async fn read(&self) -> Result<f64> {
        let base = self.base_power.get();

        // Add small noise (~1% variation) for realism
        // Use simple deterministic noise based on time
        let noise_factor = 1.0
            + (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                % 200) as f64
                / 10000.0
            - 0.01;

        let reading = base * noise_factor;
        Ok(reading)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_power_meter_read() {
        let meter = MockPowerMeter::new(2.5);

        // Read should return approximately the base value
        let reading = meter.read().await.unwrap();
        assert!(
            reading > 2.4 && reading < 2.6,
            "Reading {} not in expected range",
            reading
        );
    }

    #[tokio::test]
    async fn test_mock_power_meter_set_power() {
        let meter = MockPowerMeter::new(1.0);

        // Initial reading around 1.0
        let reading1 = meter.read().await.unwrap();
        assert!(reading1 > 0.9 && reading1 < 1.1);

        // Change base power
        meter.set_base_power(5.0).await.unwrap();
        assert_eq!(meter.get_base_power(), 5.0);

        // Reading should now be around 5.0
        let reading2 = meter.read().await.unwrap();
        assert!(
            reading2 > 4.9 && reading2 < 5.1,
            "Reading {} not in expected range",
            reading2
        );
    }

    #[tokio::test]
    async fn test_mock_power_meter_default() {
        let meter = MockPowerMeter::default();
        assert_eq!(meter.get_base_power(), 1.0);
    }

    #[tokio::test]
    async fn test_factory_creates_power_meter() {
        let factory = MockPowerMeterFactory;

        assert_eq!(factory.driver_type(), "mock_power_meter");

        let config = toml::Value::Table(toml::map::Map::new());
        let components = factory.build(config).await.unwrap();

        assert!(components.readable.is_some());
        assert!(components.parameterized.is_some());
    }
}
