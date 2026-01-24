//! NI DAQ Service implementation for Comedi hardware control (bd-czem)
//!
//! This module provides gRPC endpoints for NI PCI-MIO-16XE-10 data acquisition
//! hardware via the Comedi Linux driver. Extends basic HardwareService with
//! DAQ-specific capabilities:
//! - Multi-channel analog input streaming
//! - Digital I/O control
//! - Counter/timer operations
//! - Hardware triggering
//!
//! # Architecture
//!
//! Unlike HardwareService which uses capability traits (Readable, Movable, etc.),
//! NiDaqService accesses Comedi drivers directly for low-level DAQ operations.

use anyhow::Error as AnyError;
use daq_core::limits::RPC_TIMEOUT;
use daq_hardware::registry::DeviceRegistry;
use daq_proto::ni_daq::ni_daq_service_server::NiDaqService;
use daq_proto::ni_daq::*;
use std::future::Future;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status};
use tracing::instrument;

// =============================================================================
// NI DAQ Service Implementation
// =============================================================================

/// Implementation of NiDaqService gRPC interface.
///
/// Provides direct access to NI PCI-MIO-16XE-10 capabilities through Comedi drivers.
/// Complements HardwareService for DAQ-specific operations.
pub struct NiDaqServiceImpl {
    /// Device registry for looking up Comedi devices
    registry: Arc<DeviceRegistry>,
}

impl NiDaqServiceImpl {
    /// Create a new NI DAQ service instance.
    pub fn new(registry: Arc<DeviceRegistry>) -> Self {
        Self { registry }
    }

    /// Execute an async operation with timeout (pattern from HardwareService).
    async fn await_with_timeout<F, T>(&self, operation: &str, fut: F) -> Result<T, Status>
    where
        F: Future<Output = Result<T, AnyError>> + Send,
        T: Send,
    {
        match tokio::time::timeout(RPC_TIMEOUT, fut).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(err)) => Err(Status::internal(err.to_string())),
            Err(_) => Err(Status::deadline_exceeded(format!(
                "{} timed out after {:?}",
                operation, RPC_TIMEOUT
            ))),
        }
    }
}

/// Get current timestamp in nanoseconds since UNIX epoch.
#[allow(dead_code)]
fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

#[tonic::async_trait]
impl NiDaqService for NiDaqServiceImpl {
    // ==========================================================================
    // Analog Input (Streaming)
    // ==========================================================================

    #[instrument(skip(self))]
    async fn stream_analog_input(
        &self,
        _request: Request<StreamAnalogInputRequest>,
    ) -> Result<Response<Self::StreamAnalogInputStream>, Status> {
        Err(Status::unimplemented(
            "StreamAnalogInput not yet implemented (Phase 2)",
        ))
    }

    type StreamAnalogInputStream =
        tokio_stream::wrappers::ReceiverStream<Result<AnalogInputData, Status>>;

    #[instrument(skip(self))]
    async fn configure_analog_input(
        &self,
        request: Request<ConfigureAnalogInputRequest>,
    ) -> Result<Response<ConfigureAnalogInputResponse>, Status> {
        let req = request.into_inner();

        // Validate device exists in registry
        if req.device_id.is_empty() {
            return Err(Status::invalid_argument("device_id is required"));
        }

        // Validate channel configs
        if req.channel_configs.is_empty() {
            return Err(Status::invalid_argument(
                "At least one channel configuration is required",
            ));
        }

        // Validate each channel configuration
        for config in &req.channel_configs {
            // Channel validation (NI PCI-MIO-16XE-10 has 16 AI channels: 0-15)
            if config.channel > 15 {
                return Err(Status::invalid_argument(format!(
                    "Invalid channel {}. NI PCI-MIO-16XE-10 supports channels 0-15",
                    config.channel
                )));
            }

            // Range validation (0-13 for NI PCI-MIO-16XE-10)
            if config.range_index > 13 {
                return Err(Status::invalid_argument(format!(
                    "Invalid range_index {}. Valid ranges: 0-13",
                    config.range_index
                )));
            }

            // Validate analog reference mode
            let reference = AnalogReference::try_from(config.reference).map_err(|_| {
                Status::invalid_argument(format!(
                    "Invalid analog reference mode: {}",
                    config.reference
                ))
            })?;

            // Note: DIFFERENTIAL mode on NI PCI-MIO-16XE-10 reduces available channels
            // (uses channel pairs: 0+8, 1+9, etc., so only 8 channels available)
            if reference == AnalogReference::Differential && config.channel >= 8 {
                return Err(Status::invalid_argument(format!(
                    "Channel {} not available in DIFFERENTIAL mode. Only channels 0-7 available (paired with 8-15)",
                    config.channel
                )));
            }
        }

        // Validate timing configuration if present
        if let Some(timing) = &req.timing {
            if timing.sample_rate_hz <= 0.0 {
                return Err(Status::invalid_argument("sample_rate_hz must be positive"));
            }

            // NI PCI-MIO-16XE-10 max sample rate is ~100 kS/s aggregate
            if timing.sample_rate_hz > 100_000.0 {
                return Err(Status::invalid_argument(
                    "sample_rate_hz exceeds maximum (100 kS/s) for NI PCI-MIO-16XE-10",
                ));
            }

            // Validate clock source
            let _clock_source = ClockSource::try_from(timing.clock_source).map_err(|_| {
                Status::invalid_argument(format!("Invalid clock source: {}", timing.clock_source))
            })?;
        }

        // For Phase 1: Configuration is accepted and validated
        // Phase 2 will implement actual hardware reconfiguration via driver

        // Calculate timing values
        // For internal clock on NI PCI-MIO-16XE-10:
        // - Base clock: 20 MHz
        // - Convert time: controlled by CONVERT_arg (min ~500 ns)
        // - Scan interval: time between scans (all channels)

        let actual_sample_rate_hz = req
            .timing
            .as_ref()
            .map(|t| t.sample_rate_hz)
            .unwrap_or(1000.0); // Default 1 kHz if not specified

        let n_channels = req.channel_configs.len() as f64;

        // Calculate intervals (simplified for Phase 1)
        // Convert interval: time per sample
        let convert_interval_ns = ((1.0 / actual_sample_rate_hz) * 1_000_000_000.0) as u32;

        // Scan interval: time for all channels in a scan
        let scan_interval_ns = (convert_interval_ns as f64 * n_channels) as u32;

        // Return success with validated configuration
        let response = ConfigureAnalogInputResponse {
            success: true,
            error_message: String::new(),
            actual_sample_rate_hz,
            actual_scan_interval_ns: scan_interval_ns,
            actual_convert_interval_ns: convert_interval_ns,
        };

        Ok(Response::new(response))
    }

    #[instrument(skip(self))]
    async fn read_analog_input(
        &self,
        _request: Request<ReadAnalogInputRequest>,
    ) -> Result<Response<ReadAnalogInputResponse>, Status> {
        Err(Status::unimplemented(
            "ReadAnalogInput not yet implemented (use HardwareService.ReadValue instead)",
        ))
    }

    // ==========================================================================
    // Analog Output
    // ==========================================================================

    #[instrument(skip(self))]
    async fn set_analog_output(
        &self,
        request: Request<SetAnalogOutputRequest>,
    ) -> Result<Response<SetAnalogOutputResponse>, Status> {
        let req = request.into_inner();

        // Validate inputs
        if req.device_id.is_empty() {
            return Err(Status::invalid_argument("device_id is required"));
        }

        // NI PCI-MIO-16XE-10 has 2 analog output channels (DAC0, DAC1)
        if req.channel > 1 {
            return Err(Status::invalid_argument(format!(
                "Invalid channel {}. NI PCI-MIO-16XE-10 supports channels 0-1",
                req.channel
            )));
        }

        // Look up device in registry
        let settable = self.registry.get_settable(&req.device_id).ok_or_else(|| {
            Status::not_found(format!(
                "Device '{}' not found or does not support analog output",
                req.device_id
            ))
        })?;

        // Construct parameter name based on channel
        // Comedi SettableAnalogOutput supports "voltage" (channel 0) or "voltage_N" (channel N)
        let param_name = if req.channel == 0 {
            "voltage".to_string()
        } else {
            format!("voltage_{}", req.channel)
        };

        // Set the voltage via Settable trait
        let voltage_json = serde_json::Value::Number(
            serde_json::Number::from_f64(req.voltage)
                .ok_or_else(|| Status::invalid_argument("Invalid voltage value"))?,
        );

        self.await_with_timeout("SetAnalogOutput", async {
            settable.set_value(&param_name, voltage_json).await
        })
        .await
        .map_err(|e| Status::internal(format!("Failed to set voltage: {}", e)))?;

        // Read back the actual voltage
        // Note: Comedi Settable doesn't implement get_value for voltage yet,
        // so we'll return the requested voltage as the actual voltage.
        // In a real implementation, we'd query the hardware to verify.
        let actual_voltage = req.voltage;

        Ok(Response::new(SetAnalogOutputResponse {
            success: true,
            error_message: String::new(),
            actual_voltage,
        }))
    }

    #[instrument(skip(self))]
    async fn get_analog_output(
        &self,
        _request: Request<GetAnalogOutputRequest>,
    ) -> Result<Response<GetAnalogOutputResponse>, Status> {
        Err(Status::unimplemented(
            "GetAnalogOutput not yet implemented (Phase 1)",
        ))
    }

    #[instrument(skip(self))]
    async fn configure_analog_output(
        &self,
        _request: Request<ConfigureAnalogOutputRequest>,
    ) -> Result<Response<ConfigureAnalogOutputResponse>, Status> {
        Err(Status::unimplemented(
            "ConfigureAnalogOutput not yet implemented (Phase 1)",
        ))
    }

    // ==========================================================================
    // Digital I/O
    // ==========================================================================

    #[instrument(skip(self))]
    async fn configure_digital_io(
        &self,
        _request: Request<ConfigureDigitalIoRequest>,
    ) -> Result<Response<ConfigureDigitalIoResponse>, Status> {
        Err(Status::unimplemented(
            "ConfigureDigitalIO not yet implemented (Phase 3)",
        ))
    }

    #[instrument(skip(self))]
    async fn read_digital_io(
        &self,
        _request: Request<ReadDigitalIoRequest>,
    ) -> Result<Response<ReadDigitalIoResponse>, Status> {
        Err(Status::unimplemented(
            "ReadDigitalIO not yet implemented (Phase 3)",
        ))
    }

    #[instrument(skip(self))]
    async fn write_digital_io(
        &self,
        _request: Request<WriteDigitalIoRequest>,
    ) -> Result<Response<WriteDigitalIoResponse>, Status> {
        Err(Status::unimplemented(
            "WriteDigitalIO not yet implemented (Phase 3)",
        ))
    }

    #[instrument(skip(self))]
    async fn read_digital_port(
        &self,
        _request: Request<ReadDigitalPortRequest>,
    ) -> Result<Response<ReadDigitalPortResponse>, Status> {
        Err(Status::unimplemented(
            "ReadDigitalPort not yet implemented (Phase 3)",
        ))
    }

    #[instrument(skip(self))]
    async fn write_digital_port(
        &self,
        _request: Request<WriteDigitalPortRequest>,
    ) -> Result<Response<WriteDigitalPortResponse>, Status> {
        Err(Status::unimplemented(
            "WriteDigitalPort not yet implemented (Phase 3)",
        ))
    }

    // ==========================================================================
    // Counter/Timer
    // ==========================================================================

    #[instrument(skip(self))]
    async fn read_counter(
        &self,
        _request: Request<ReadCounterRequest>,
    ) -> Result<Response<ReadCounterResponse>, Status> {
        Err(Status::unimplemented(
            "ReadCounter not yet implemented (Phase 4)",
        ))
    }

    #[instrument(skip(self))]
    async fn reset_counter(
        &self,
        _request: Request<ResetCounterRequest>,
    ) -> Result<Response<ResetCounterResponse>, Status> {
        Err(Status::unimplemented(
            "ResetCounter not yet implemented (Phase 4)",
        ))
    }

    #[instrument(skip(self))]
    async fn arm_counter(
        &self,
        _request: Request<ArmCounterRequest>,
    ) -> Result<Response<ArmCounterResponse>, Status> {
        Err(Status::unimplemented(
            "ArmCounter not yet implemented (Phase 4)",
        ))
    }

    #[instrument(skip(self))]
    async fn disarm_counter(
        &self,
        _request: Request<DisarmCounterRequest>,
    ) -> Result<Response<DisarmCounterResponse>, Status> {
        Err(Status::unimplemented(
            "DisarmCounter not yet implemented (Phase 4)",
        ))
    }

    #[instrument(skip(self))]
    async fn configure_counter(
        &self,
        _request: Request<ConfigureCounterRequest>,
    ) -> Result<Response<ConfigureCounterResponse>, Status> {
        Err(Status::unimplemented(
            "ConfigureCounter not yet implemented (Phase 4)",
        ))
    }

    // ==========================================================================
    // Triggering
    // ==========================================================================

    #[instrument(skip(self))]
    async fn configure_trigger(
        &self,
        _request: Request<ConfigureTriggerRequest>,
    ) -> Result<Response<ConfigureTriggerResponse>, Status> {
        Err(Status::unimplemented(
            "ConfigureTrigger not yet implemented (Phase 2+)",
        ))
    }

    #[instrument(skip(self))]
    async fn get_trigger_config(
        &self,
        _request: Request<GetTriggerConfigRequest>,
    ) -> Result<Response<TriggerConfig>, Status> {
        Err(Status::unimplemented(
            "GetTriggerConfig not yet implemented (Phase 2+)",
        ))
    }

    // ==========================================================================
    // Device Status
    // ==========================================================================

    #[instrument(skip(self))]
    async fn get_daq_status(
        &self,
        request: Request<GetDaqStatusRequest>,
    ) -> Result<Response<DaqStatus>, Status> {
        let req = request.into_inner();
        let device_id = req.device_id;

        // Look up device in registry to verify it exists
        let _device_info = self
            .registry
            .get_device_info(&device_id)
            .ok_or_else(|| Status::not_found(format!("Device '{}' not found", device_id)))?;

        // Comedi support is only available when the 'comedi' feature is enabled
        #[cfg(feature = "comedi")]
        {
            // For now, we open a new connection to query device info
            // TODO: Store device path in registry metadata or driver state
            let device_path = "/dev/comedi0";

            let status = self
                .await_with_timeout("GetDAQStatus", async {
                    // Open device to query info (spawn_blocking for FFI)
                    let path = device_path.to_string();
                    let device = tokio::task::spawn_blocking(move || {
                        use daq_hardware::drivers::comedi::ComediDevice;
                        ComediDevice::open(&path)
                    })
                    .await
                    .map_err(|e| anyhow::anyhow!("Task join error: {}", e))??;

                    // Get comprehensive device info
                    let info = device.info()?;

                    // Build subdevice summaries
                    let subdevices: Vec<SubdeviceSummary> = info
                        .subdevices
                        .iter()
                        .map(|sub| {
                            use daq_hardware::drivers::comedi::SubdeviceType;
                            let type_str = match sub.subdev_type {
                                SubdeviceType::AnalogInput => "ai",
                                SubdeviceType::AnalogOutput => "ao",
                                SubdeviceType::DigitalIO => "dio",
                                SubdeviceType::Counter => "counter",
                                SubdeviceType::Timer => "timer",
                                _ => "other",
                            };

                            SubdeviceSummary {
                                index: sub.index,
                                r#type: type_str.to_string(),
                                n_channels: sub.n_channels,
                                busy: sub.is_busy(),
                                supports_commands: sub.supports_commands(),
                            }
                        })
                        .collect();

                    // Count channels by type
                    use daq_hardware::drivers::comedi::SubdeviceType;
                    let ai_info = info
                        .subdevices
                        .iter()
                        .find(|s| s.subdev_type == SubdeviceType::AnalogInput);
                    let ao_info = info
                        .subdevices
                        .iter()
                        .find(|s| s.subdev_type == SubdeviceType::AnalogOutput);
                    let dio_info = info
                        .subdevices
                        .iter()
                        .find(|s| s.subdev_type == SubdeviceType::DigitalIO);
                    let counter_count = info
                        .subdevices
                        .iter()
                        .filter(|s| s.subdev_type == SubdeviceType::Counter)
                        .count();

                    let ai_channels = ai_info.map(|s| s.n_channels).unwrap_or(0);
                    let ao_channels = ao_info.map(|s| s.n_channels).unwrap_or(0);
                    let dio_channels = dio_info.map(|s| s.n_channels).unwrap_or(0);

                    // Get AI/AO resolution
                    let ai_resolution = ai_info.map(|s| s.resolution_bits()).unwrap_or(0);
                    let ao_resolution = ao_info.map(|s| s.resolution_bits()).unwrap_or(0);

                    // Get voltage ranges for AI
                    let ai_ranges = if let Some(ai_sub) = ai_info {
                        let ai = device.analog_input_subdevice(ai_sub.index)?;
                        ai.ranges(0)?
                            .into_iter()
                            .map(|r| VoltageRange {
                                index: r.index,
                                min_voltage: r.min,
                                max_voltage: r.max,
                                unit: match r.unit {
                                    0 => "V".to_string(),
                                    1 => "mA".to_string(),
                                    _ => "".to_string(),
                                },
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };

                    // Get voltage ranges for AO
                    let ao_ranges = if ao_info.is_some() {
                        if let Ok(ao) = device.analog_output() {
                            ao.ranges(0)?
                                .into_iter()
                                .map(|r| VoltageRange {
                                    index: r.index,
                                    min_voltage: r.min,
                                    max_voltage: r.max,
                                    unit: match r.unit {
                                        0 => "V".to_string(),
                                        1 => "mA".to_string(),
                                        _ => "".to_string(),
                                    },
                                })
                                .collect()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };

                    Ok(DaqStatus {
                        device_id: device_id.clone(),
                        board_name: info.board_name,
                        driver_name: info.driver_name,
                        online: true,
                        n_subdevices: info.n_subdevices,
                        subdevices,
                        ai_busy: ai_info.map(|s| s.is_busy()).unwrap_or(false),
                        ao_busy: ao_info.map(|s| s.is_busy()).unwrap_or(false),
                        dio_configured: dio_info.is_some(),
                        active_counters: 0, // TODO: Track active counters in state
                        ai_channels,
                        ao_channels,
                        dio_channels,
                        counter_channels: counter_count as u32,
                        ai_resolution_bits: ai_resolution,
                        ao_resolution_bits: ao_resolution,
                        ai_ranges,
                        ao_ranges,
                    })
                })
                .await?;

            Ok(Response::new(status))
        }

        #[cfg(not(feature = "comedi"))]
        {
            Err(Status::unimplemented(
                "GetDAQStatus requires 'comedi' feature to be enabled",
            ))
        }
    }

    #[instrument(skip(self))]
    async fn get_timing_capabilities(
        &self,
        _request: Request<GetTimingCapabilitiesRequest>,
    ) -> Result<Response<TimingCapabilities>, Status> {
        Err(Status::unimplemented(
            "GetTimingCapabilities not yet implemented (Phase 2+)",
        ))
    }

    #[instrument(skip(self))]
    async fn get_calibration_status(
        &self,
        _request: Request<GetCalibrationStatusRequest>,
    ) -> Result<Response<CalibrationStatus>, Status> {
        Err(Status::unimplemented(
            "GetCalibrationStatus not yet implemented (future)",
        ))
    }
}
