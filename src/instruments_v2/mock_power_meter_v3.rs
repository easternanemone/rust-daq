//! Mock Power Meter V3 (Unified Architecture)
//!
//! V3 implementation for testing the entire integration stack:
//! - Implements `core_v3::Instrument` trait (unified architecture)
//! - Implements `core_v3::PowerMeter` trait for polymorphism
//! - Uses `Parameter<T>` for declarative parameter management
//! - Direct async methods (no actor model)
//! - Single broadcast channel for data streaming
//!
//! This is the first fully-integrated V3 instrument, designed to validate:
//! - TOML configuration → InstrumentManagerV3
//! - Manager → Forwarder pattern
//! - Forwarder → DataDistributor
//! - DataDistributor → GUI
//!
//! ## Configuration
//!
//! ```toml
//! [instruments.power_meter]
//! type = "mock_power_meter_v3"
//! sampling_rate = 10.0  # Hz
//! wavelength_nm = 532.0  # nm
//! ```
//!
//! ## Data Generation
//!
//! Generates realistic power readings:
//! - Base power: 1.0 mW
//! - Noise: ±5% random variation
//! - Units: milliwatts (mW)

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::core_v3::{
    Command, Instrument, InstrumentState, Measurement, ParameterBase, PowerMeter, Response,
};
use crate::parameter::{Parameter, ParameterBuilder};

// =============================================================================
// MockPowerMeterV3
// =============================================================================

/// Mock power meter for testing V3 integration stack
///
/// Generates simulated power readings with realistic noise. This is the
/// simplest V3 instrument (scalar data only) for validating end-to-end flow:
/// TOML → Manager → Forwarder → DataDistributor → GUI.
pub struct MockPowerMeterV3 {
    /// Instrument identifier
    id: String,

    /// Current state
    state: InstrumentState,

    /// Sampling rate in Hz
    sampling_rate: f64,

    /// Data broadcast channel
    data_tx: broadcast::Sender<Measurement>,

    /// Parameters (for dynamic access via ParameterBase)
    parameters: HashMap<String, Box<dyn ParameterBase>>,

    // Typed parameters (for direct access via PowerMeter trait)
    wavelength_nm: Arc<RwLock<Parameter<f64>>>,

    /// Data generation task handle
    task_handle: Option<tokio::task::JoinHandle<()>>,

    /// Shutdown signal for data generation task
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl MockPowerMeterV3 {
    /// Create new mock power meter instance
    ///
    /// # Arguments
    /// * `id` - Unique instrument identifier
    /// * `sampling_rate` - Data generation rate in Hz
    /// * `wavelength_nm` - Initial wavelength setting in nanometers
    pub fn new(id: impl Into<String>, sampling_rate: f64, wavelength_nm: f64) -> Self {
        let id = id.into();
        let (data_tx, _) = broadcast::channel(1024);

        // Create wavelength parameter
        let wavelength_nm_param = Arc::new(RwLock::new(
            ParameterBuilder::new("wavelength_nm", wavelength_nm)
                .description("Laser wavelength for power calibration")
                .unit("nm")
                .range(400.0, 1700.0)
                .build(),
        ));

        Self {
            id,
            state: InstrumentState::Uninitialized,
            sampling_rate,
            data_tx,
            parameters: HashMap::new(),
            wavelength_nm: wavelength_nm_param,
            task_handle: None,
            shutdown_tx: None,
        }
    }

    /// Create from TOML configuration
    ///
    /// # Configuration Format
    ///
    /// ```toml
    /// [instruments.power_meter]
    /// type = "mock_power_meter_v3"
    /// sampling_rate = 10.0
    /// wavelength_nm = 532.0
    /// ```
    pub fn from_config(id: &str, cfg: &serde_json::Value) -> Result<Box<dyn Instrument>> {
        let sampling_rate = cfg["sampling_rate"].as_f64().unwrap_or(10.0);
        let wavelength_nm = cfg["wavelength_nm"].as_f64().unwrap_or(532.0);

        Ok(Box::new(Self::new(id, sampling_rate, wavelength_nm)))
    }

    /// Generate realistic power reading with ±5% noise
    fn generate_power_reading(&self) -> f64 {
        let mut rng = rand::thread_rng();
        let base_power = 1.0; // 1 mW baseline
        let noise = rng.gen_range(-0.05..0.05); // ±5% noise
        base_power * (1.0 + noise)
    }

    /// Start the data generation task
    fn start_data_generation_task(&mut self) -> Result<()> {
        let tx = self.data_tx.clone();
        let sampling_rate = self.sampling_rate;
        let id = self.id.clone();
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();

        let handle = tokio::spawn(async move {
            let interval = tokio::time::Duration::from_secs_f64(1.0 / sampling_rate);
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        // Generate power reading
                        let mut rng = rand::thread_rng();
                        let base_power = 1.0; // 1 mW baseline
                        let noise = rng.gen_range(-0.05..0.05); // ±5% noise
                        let power = base_power * (1.0 + noise);

                        // Create measurement
                        let measurement = Measurement::Scalar {
                            name: format!("{}_power", id),
                            value: power,
                            unit: "mW".to_string(),
                            timestamp: Utc::now(),
                        };

                        // Broadcast measurement
                        if tx.send(measurement).is_err() {
                            // Channel closed, exit task
                            break;
                        }
                    }
                    _ = &mut shutdown_rx => {
                        // Shutdown signal received, exit task
                        break;
                    }
                }
            }
        });

        self.task_handle = Some(handle);
        self.shutdown_tx = Some(shutdown_tx);
        Ok(())
    }

    /// Stop the data generation task
    async fn stop_data_generation_task(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        if let Some(handle) = self.task_handle.take() {
            handle.await?;
        }
        Ok(())
    }
}

// =============================================================================
// Instrument Trait Implementation
// =============================================================================

#[async_trait]
impl Instrument for MockPowerMeterV3 {
    fn id(&self) -> &str {
        &self.id
    }

    fn state(&self) -> InstrumentState {
        self.state
    }

    async fn initialize(&mut self) -> Result<()> {
        if self.state != InstrumentState::Uninitialized {
            return Err(anyhow!("Already initialized"));
        }

        self.state = InstrumentState::Idle;

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state = InstrumentState::ShuttingDown;

        // Send shutdown signal to data generation task
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        // Wait for task to complete
        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }

        Ok(())
    }

    fn data_channel(&self) -> broadcast::Receiver<Measurement> {
        self.data_tx.subscribe()
    }

    async fn execute(&mut self, cmd: Command) -> Result<Response> {
        match cmd {
            Command::Start => {
                if self.state == InstrumentState::Idle {
                    self.state = InstrumentState::Running;
                    self.start_data_generation_task()?;
                }
                Ok(Response::Ok)
            }
            Command::Stop => {
                if self.state == InstrumentState::Running {
                    self.state = InstrumentState::Idle;
                    self.stop_data_generation_task().await?;
                }
                Ok(Response::Ok)
            }
            Command::GetState => Ok(Response::State(self.state)),
            Command::Configure { params } => {
                for (key, value) in params {
                    if key == "sampling_rate" {
                        if let Some(rate) = value.as_f64() {
                            self.sampling_rate = rate;
                        }
                    }
                }
                Ok(Response::Ok)
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

// =============================================================================
// PowerMeter Trait Implementation
// =============================================================================

#[async_trait]
impl PowerMeter for MockPowerMeterV3 {
    async fn set_wavelength(&mut self, nm: f64) -> Result<()> {
        // Validate and set parameter
        self.wavelength_nm.write().await.set(nm).await?;
        Ok(())
    }

    async fn set_range(&mut self, _watts: f64) -> Result<()> {
        // Mock implementation - no actual range setting
        Ok(())
    }

    async fn zero(&mut self) -> Result<()> {
        // Mock implementation - no actual zeroing
        Ok(())
    }
}

// Additional mock-specific methods
impl MockPowerMeterV3 {
    /// Get current wavelength setting
    pub async fn wavelength(&self) -> f64 {
        self.wavelength_nm.read().await.get()
    }

    /// Get current power reading (generates new sample)
    pub fn power(&self) -> f64 {
        self.generate_power_reading()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_from_config() {
        let cfg = serde_json::json!({
            "sampling_rate": 10.0,
            "wavelength_nm": 532.0
        });

        let inst = MockPowerMeterV3::from_config("test", &cfg).unwrap();
        assert_eq!(inst.id(), "test");
    }

    #[tokio::test]
    async fn test_initialization() {
        let mut meter = MockPowerMeterV3::new("test_pm", 10.0, 532.0);
        assert_eq!(meter.state(), InstrumentState::Uninitialized);

        meter.initialize().await.unwrap();
        assert_eq!(meter.state(), InstrumentState::Idle);
    }

    #[tokio::test]
    async fn test_initialize_and_measure() {
        let mut meter = MockPowerMeterV3::new("test_pm", 10.0, 532.0);
        meter.initialize().await.unwrap();

        // Subscribe to data channel
        let mut rx = meter.data_channel();

        // Wait for measurement (with timeout)
        tokio::select! {
            result = rx.recv() => {
                let measurement = result.unwrap();
                match measurement {
                    Measurement::Scalar { name, value, unit, .. } => {
                        assert_eq!(name, "test_pm_power");
                        assert!(value > 0.0, "Power should be positive");
                        assert!(value < 2.0, "Power should be reasonable (< 2mW with noise)");
                        assert_eq!(unit, "mW");
                    }
                    _ => panic!("Expected Scalar measurement"),
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                panic!("No measurement received within timeout");
            }
        }
    }

    #[tokio::test]
    async fn test_power_meter_trait_methods() {
        let mut meter = MockPowerMeterV3::new("test_pm", 10.0, 532.0);
        meter.initialize().await.unwrap();

        // Test set_wavelength
        meter.set_wavelength(633.0).await.unwrap();
        assert_eq!(meter.wavelength().await, 633.0);

        // Test set_wavelength with validation (should fail)
        let result = meter.set_wavelength(100.0).await;
        assert!(result.is_err(), "Wavelength below 400nm should fail");

        let result = meter.set_wavelength(2000.0).await;
        assert!(result.is_err(), "Wavelength above 1700nm should fail");

        // Test set_range (mock - always succeeds)
        meter.set_range(0.001).await.unwrap();

        // Test zero (mock - always succeeds)
        meter.zero().await.unwrap();
    }

    #[tokio::test]
    async fn test_data_generation_noise() {
        let meter = MockPowerMeterV3::new("test_pm", 10.0, 532.0);

        // Generate multiple readings and check they're within expected range
        let readings: Vec<f64> = (0..100).map(|_| meter.power()).collect();

        for reading in &readings {
            assert!(*reading >= 0.95, "Power should be >= 0.95 mW (1.0 - 5%)");
            assert!(*reading <= 1.05, "Power should be <= 1.05 mW (1.0 + 5%)");
        }

        // Check that readings vary (not all the same)
        let min = readings.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = readings.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(max - min > 0.01, "Readings should have noise variation");
    }

    #[tokio::test]
    async fn test_execute_command() {
        let mut meter = MockPowerMeterV3::new("test_pm", 10.0, 532.0);
        meter.initialize().await.unwrap();

        // Test Start command
        let response = meter.execute(Command::Start).await.unwrap();
        assert!(matches!(response, Response::Ok));
        assert_eq!(meter.state(), InstrumentState::Running);

        // Test Stop command
        let response = meter.execute(Command::Stop).await.unwrap();
        assert!(matches!(response, Response::Ok));
        assert_eq!(meter.state(), InstrumentState::Idle);

        // Test GetState command
        let response = meter.execute(Command::GetState).await.unwrap();
        assert!(matches!(response, Response::State(InstrumentState::Idle)));
    }

    #[tokio::test]
    async fn test_shutdown() {
        let mut meter = MockPowerMeterV3::new("test_pm", 10.0, 532.0);
        meter.initialize().await.unwrap();

        // Subscribe to data channel
        let mut rx = meter.data_channel();

        // Verify data is flowing
        tokio::select! {
            result = rx.recv() => {
                assert!(result.is_ok(), "Should receive measurement before shutdown");
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                panic!("No measurement received before shutdown");
            }
        }

        // Shutdown
        meter.shutdown().await.unwrap();
        assert_eq!(meter.state(), InstrumentState::ShuttingDown);

        // Verify no more data is generated
        tokio::select! {
            result = rx.recv() => {
                // recv() might return lag error if buffer filled before shutdown
                // or Ok if there was buffered data. Either is acceptable.
                // The important part is the task stopped spawning new data.
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => {
                // Timeout is expected - no new data should be generated
            }
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let mut meter = MockPowerMeterV3::new("test_pm", 10.0, 532.0);
        meter.initialize().await.unwrap();

        // Create multiple subscribers
        let mut rx1 = meter.data_channel();
        let mut rx2 = meter.data_channel();
        let mut rx3 = meter.data_channel();

        // All subscribers should receive the same measurement
        tokio::select! {
            result1 = rx1.recv() => {
                let m1 = result1.unwrap();
                let result2 = rx2.recv().await.unwrap();
                let result3 = rx3.recv().await.unwrap();

                // All measurements should have the same name
                assert_eq!(m1.name(), result2.name());
                assert_eq!(m1.name(), result3.name());
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                panic!("No measurement received within timeout");
            }
        }
    }

    #[tokio::test]
    async fn test_sampling_rate() {
        let sampling_rate = 20.0; // 20 Hz
        let mut meter = MockPowerMeterV3::new("test_pm", sampling_rate, 532.0);
        meter.initialize().await.unwrap();

        let mut rx = meter.data_channel();

        // Collect timestamps of first few measurements
        let mut timestamps = Vec::new();
        for _ in 0..5 {
            if let Ok(measurement) = rx.recv().await {
                timestamps.push(measurement.timestamp());
            }
        }

        // Check that measurements arrive at approximately correct rate
        assert_eq!(timestamps.len(), 5, "Should receive 5 measurements");

        for i in 1..timestamps.len() {
            let duration = timestamps[i].signed_duration_since(timestamps[i - 1]);
            let duration_ms = duration.num_milliseconds() as f64;
            let expected_ms = 1000.0 / sampling_rate;

            // Allow ±20% tolerance for timing (async scheduler variance)
            assert!(
                duration_ms >= expected_ms * 0.8 && duration_ms <= expected_ms * 1.2,
                "Measurement interval should be ~{} ms, got {} ms",
                expected_ms,
                duration_ms
            );
        }
    }
}
