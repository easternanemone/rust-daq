//! Generic VISA Instrument V2 Implementation
//!
//! This module provides a V2 implementation of a generic VISA instrument
//! using the new three-tier architecture:
//! - VisaAdapter for VISA communication (GPIB, USB, Ethernet)
//! - Instrument trait for state management
//! - Flexible command execution for arbitrary VISA commands
//!
//! VISA (Virtual Instrument Software Architecture) is a standard API for
//! communicating with test and measurement instruments. This generic
//! implementation supports any VISA-compliant instrument.
//!
//! ## Configuration Example
//!
//! ```toml
//! [instruments.visa_multimeter]
//! type = "visa_v2"
//! resource = "GPIB0::5::INSTR"  # or "USB0::...", "TCPIP0::..."
//! timeout_ms = 5000
//! enable_streaming = false
//! streaming_command = "MEAS:VOLT:DC?"
//! streaming_rate_hz = 1.0
//! ```

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use daq_core::{
    AdapterConfig, DataPoint, Instrument, InstrumentCommand, InstrumentState, Measurement,
    MeasurementReceiver, MeasurementSender,
};
use log::{debug, error, info};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

use crate::adapters::visa_adapter::VisaAdapter;

/// Generic VISA instrument implementation using VISA adapter
pub struct VisaInstrumentV2 {
    /// Instrument identifier
    id: String,

    /// VISA adapter for communication (Arc<Mutex> for shared mutable access)
    adapter: Arc<Mutex<VisaAdapter>>,

    /// Current instrument state
    state: InstrumentState,

    /// Cached instrument identity (*IDN? response)
    identity: Option<String>,

    /// Streaming configuration
    enable_streaming: bool,
    streaming_command: String,
    streaming_rate_hz: f64,

    /// Data streaming (zero-copy with Arc)
    measurement_tx: MeasurementSender,
    _measurement_rx_keeper: MeasurementReceiver,

    /// Acquisition task management
    task_handle: Option<JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl VisaInstrumentV2 {
    /// Create a new generic VISA instrument with VisaAdapter and default capacity (1024)
    ///
    /// # Arguments
    /// * `id` - Unique instrument identifier
    /// * `resource` - VISA resource string (e.g., "GPIB0::1::INSTR")
    pub fn new(id: String, resource: String) -> Self {
        Self::with_capacity(id, resource, 1024)
    }

    /// Create a new generic VISA instrument with VisaAdapter and specified capacity
    ///
    /// # Arguments
    /// * `id` - Unique instrument identifier
    /// * `resource` - VISA resource string (e.g., "GPIB0::1::INSTR")
    /// * `capacity` - Broadcast channel capacity for data distribution
    pub fn with_capacity(id: String, resource: String, capacity: usize) -> Self {
        let (tx, rx) = broadcast::channel(capacity);
        let adapter = VisaAdapter::new(resource);

        Self {
            id,
            adapter: Arc::new(Mutex::new(adapter)),
            state: InstrumentState::Disconnected,
            identity: None,
            enable_streaming: false,
            streaming_command: "MEAS:VOLT:DC?".to_string(),
            streaming_rate_hz: 1.0,
            measurement_tx: tx,
            _measurement_rx_keeper: rx,
            task_handle: None,
            shutdown_tx: None,
        }
    }

    /// Configure streaming parameters
    ///
    /// # Arguments
    /// * `enabled` - Enable continuous polling
    /// * `command` - VISA query command to poll (e.g., "MEAS:VOLT:DC?")
    /// * `rate_hz` - Polling rate in Hz
    pub fn with_streaming(mut self, enabled: bool, command: String, rate_hz: f64) -> Self {
        self.enable_streaming = enabled;
        self.streaming_command = command;
        self.streaming_rate_hz = rate_hz;
        self
    }

    /// Send a VISA command to the instrument
    ///
    /// For query commands (ending with ?), returns the response.
    /// For write commands, returns an empty string.
    pub async fn send_command(&self, command: &str) -> Result<String> {
        let adapter_guard = self.adapter.lock().await;
        adapter_guard.send_command(command).await
    }

    /// Send a VISA write command (no response expected)
    pub async fn send_write(&self, command: &str) -> Result<()> {
        let adapter_guard = self.adapter.lock().await;
        adapter_guard.send_write(command).await
    }

    /// Query the instrument identity (*IDN?)
    async fn query_identity(&mut self) -> Result<String> {
        let response = self
            .send_command("*IDN?")
            .await
            .context("Failed to query instrument identity")?;

        info!("VISA instrument '{}' identity: {}", self.id, response);
        self.identity = Some(response.clone());
        Ok(response)
    }

    /// Reset the instrument (*RST)
    pub async fn reset_instrument(&mut self) -> Result<()> {
        self.send_write("*RST")
            .await
            .context("Failed to send *RST command")?;

        info!("VISA instrument '{}' reset", self.id);

        // Wait for reset to complete
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    /// Clear status (*CLS)
    pub async fn clear_status(&mut self) -> Result<()> {
        self.send_write("*CLS")
            .await
            .context("Failed to send *CLS command")?;

        info!("VISA instrument '{}' status cleared", self.id);
        Ok(())
    }

    /// Query operation complete (*OPC?)
    pub async fn operation_complete(&self) -> Result<bool> {
        let response = self
            .send_command("*OPC?")
            .await
            .context("Failed to query operation complete")?;

        Ok(response.trim() == "1")
    }

    /// Spawn polling task for continuous measurement streaming
    fn spawn_polling_task(&mut self) {
        if !self.enable_streaming {
            return;
        }

        let tx = self.measurement_tx.clone();
        let id = self.id.clone();
        let polling_rate = self.streaming_rate_hz;
        let command = self.streaming_command.clone();

        // Clone the Arc (not the adapter) for the spawned task
        // This shares the same connected adapter instance
        let adapter = self.adapter.clone();

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        self.task_handle = Some(tokio::spawn(async move {
            let interval_duration = Duration::from_secs_f64(1.0 / polling_rate);
            let mut interval = tokio::time::interval(interval_duration);

            info!(
                "VISA instrument '{}' polling task started at {} Hz",
                id, polling_rate
            );

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Query the instrument via adapter
                        let result = {
                            let adapter_guard = adapter.lock().await;
                            adapter_guard.send_command(&command).await
                        };

                        match result {
                            Ok(response) => {
                                // Parse response as f64
                                if let Ok(value) = response.trim().parse::<f64>() {
                                    let datapoint = DataPoint {
                                        timestamp: Utc::now(),
                                        channel: id.clone(),
                                        value,
                                        unit: "V".to_string(), // Generic unit
                                    };
                                    let measurement = Measurement::Scalar(datapoint);
                                    let _ = tx.send(measurement); // Ignore channel errors
                                } else {
                                    debug!("Failed to parse VISA response as f64: {}", response);
                                }
                            }
                            Err(e) => {
                                error!("VISA polling query failed: {}", e);
                                // Continue polling despite errors
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        info!("VISA instrument '{}' polling task shutting down", id);
                        break;
                    }
                }
            }
        }));
    }
}

#[async_trait]
impl Instrument for VisaInstrumentV2 {
    fn id(&self) -> &str {
        &self.id
    }

    fn instrument_type(&self) -> &str {
        "visa_v2"
    }

    fn state(&self) -> InstrumentState {
        self.state.clone()
    }

    async fn initialize(&mut self) -> Result<()> {
        if self.state != InstrumentState::Disconnected {
            return Err(anyhow!("Cannot initialize from state: {:?}", self.state));
        }

        info!("Initializing VISA instrument '{}'", self.id);
        self.state = InstrumentState::Connecting;

        // Connect hardware adapter with empty config (adapter already has resource string)
        let config = AdapterConfig::default();
        self.adapter
            .lock()
            .await
            .connect(&config)
            .await
            .context("Failed to connect VISA adapter")?;

        // Query identity
        self.query_identity().await?;

        self.state = InstrumentState::Ready;
        info!("VISA instrument '{}' initialized successfully", self.id);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down VISA instrument '{}'", self.id);
        self.state = InstrumentState::ShuttingDown;

        // Stop acquisition task if running
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }

        // Disconnect adapter
        self.adapter.lock().await.disconnect().await?;

        self.state = InstrumentState::Disconnected;
        info!("VISA instrument '{}' shut down successfully", self.id);
        Ok(())
    }

    async fn recover(&mut self) -> Result<()> {
        match &self.state {
            InstrumentState::Error(daq_error) if daq_error.can_recover => {
                info!("Attempting to recover VISA instrument '{}'", self.id);

                // Disconnect and wait
                let _ = self.adapter.lock().await.disconnect().await;
                tokio::time::sleep(Duration::from_millis(500)).await;

                // Reconnect
                let config = AdapterConfig::default();
                self.adapter.lock().await.connect(&config).await?;

                // Re-query identity
                self.query_identity().await?;

                self.state = InstrumentState::Ready;

                info!("VISA instrument '{}' recovered successfully", self.id);
                Ok(())
            }
            InstrumentState::Error(_) => Err(anyhow!("Cannot recover from unrecoverable error")),
            _ => Err(anyhow!("Cannot recover from state: {:?}", self.state)),
        }
    }

    fn measurement_stream(&self) -> MeasurementReceiver {
        self.measurement_tx.subscribe()
    }

    async fn handle_command(&mut self, cmd: InstrumentCommand) -> Result<()> {
        match cmd {
            InstrumentCommand::StartAcquisition => self.start_streaming().await,
            InstrumentCommand::StopAcquisition => self.stop_streaming().await,
            InstrumentCommand::Shutdown => self.shutdown().await,
            InstrumentCommand::Recover => self.recover().await,
            InstrumentCommand::SetParameter { name, value } => {
                // Generic parameter setting via VISA command
                let visa_cmd = value
                    .as_str()
                    .ok_or_else(|| anyhow!("Parameter value must be a VISA command string"))?;

                info!(
                    "Setting VISA parameter '{}' with command: {}",
                    name, visa_cmd
                );
                self.send_command(visa_cmd).await?;
                Ok(())
            }
            InstrumentCommand::GetParameter { name } => {
                // For GetParameter, we'd need a way to return the value
                // For now, just log it
                info!("Get parameter request for '{}' (not implemented)", name);
                Ok(())
            }
        }
    }
}

// Additional VISA-specific methods
impl VisaInstrumentV2 {
    /// Start continuous measurement streaming
    async fn start_streaming(&mut self) -> Result<()> {
        if self.state != InstrumentState::Ready {
            return Err(anyhow!(
                "Cannot start streaming from state: {:?}",
                self.state
            ));
        }

        if !self.enable_streaming {
            return Err(anyhow!(
                "Streaming not enabled. Configure with with_streaming()"
            ));
        }

        self.spawn_polling_task();
        self.state = InstrumentState::Acquiring;

        info!("VISA instrument '{}' started streaming", self.id);
        Ok(())
    }

    /// Stop continuous measurement streaming
    async fn stop_streaming(&mut self) -> Result<()> {
        if self.state != InstrumentState::Acquiring {
            return Err(anyhow!("Not currently acquiring"));
        }

        // Stop polling task
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }

        self.state = InstrumentState::Ready;
        info!("VISA instrument '{}' stopped streaming", self.id);
        Ok(())
    }

    /// Get the cached instrument identity
    pub fn get_identity(&self) -> Option<&str> {
        self.identity.as_deref()
    }

    /// Execute a VISA query and return the response
    pub async fn query(&self, command: &str) -> Result<String> {
        if self.state != InstrumentState::Ready && self.state != InstrumentState::Acquiring {
            return Err(anyhow!("Cannot query from state: {:?}", self.state));
        }

        self.send_command(command).await
    }

    /// Execute a VISA write command
    pub async fn write(&mut self, command: &str) -> Result<()> {
        if self.state != InstrumentState::Ready && self.state != InstrumentState::Acquiring {
            return Err(anyhow!("Cannot write from state: {:?}", self.state));
        }

        self.send_write(command).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visa_creation() {
        let instrument =
            VisaInstrumentV2::new("test_visa".to_string(), "GPIB0::1::INSTR".to_string());

        assert_eq!(instrument.id(), "test_visa");
        assert_eq!(instrument.instrument_type(), "visa_v2");
        assert_eq!(instrument.state(), InstrumentState::Disconnected);

        assert!(!instrument.enable_streaming);
    }

    #[test]
    fn test_visa_with_streaming() {
        let instrument = VisaInstrumentV2::new(
            "test_visa".to_string(),
            "TCPIP0::192.168.1.100::INSTR".to_string(),
        )
        .with_streaming(true, "MEAS:CURR:DC?".to_string(), 5.0);

        assert_eq!(instrument.id(), "test_visa");
        assert!(instrument.enable_streaming);
        assert_eq!(instrument.streaming_command, "MEAS:CURR:DC?");
        assert_eq!(instrument.streaming_rate_hz, 5.0);
    }

    #[test]
    fn test_identity_storage() {
        let mut instrument = VisaInstrumentV2::new(
            "test_visa".to_string(),
            "USB0::0x1234::0x5678::SERIAL::INSTR".to_string(),
        );

        // Initially no identity
        assert!(instrument.get_identity().is_none());

        // Simulate setting identity (normally done during initialization)
        instrument.identity = Some("Manufacturer,Model,Serial,Version".to_string());
        assert!(instrument.get_identity().is_some());
        assert_eq!(
            instrument.get_identity().unwrap(),
            "Manufacturer,Model,Serial,Version"
        );
    }

    #[test]
    fn test_state_transitions() {
        let instrument =
            VisaInstrumentV2::new("test_visa".to_string(), "GPIB0::5::INSTR".to_string());

        // Starts disconnected
        assert_eq!(instrument.state(), InstrumentState::Disconnected);

        // Would transition through Connecting -> Ready during initialization
        // (tested in integration tests with actual hardware)
    }

    // Note: Integration tests with actual VISA hardware would go in tests/ directory
    // These unit tests verify the structure and basic functionality without hardware
}
