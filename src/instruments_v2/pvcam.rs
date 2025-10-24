//! Photometrics PVCAM camera driver V2 (PrimeBSI)
//!
//! V2 implementation using the new trait hierarchy and Measurement enum.
//! This enables native PixelBuffer::U16 broadcasting for 4× memory savings.
//!
//! ## Configuration
//!
//! ```toml
//! [instruments.prime_bsi]
//! type = "pvcam_v2"
//! camera_name = "PrimeBSI"
//! exposure_ms = 100.0
//! roi = [0, 0, 2048, 2048]  # [x, y, width, height]
//! binning = [1, 1]  # [x_bin, y_bin]
//! polling_rate_hz = 10.0
//! ```

use async_trait::async_trait;
use chrono::Utc;
use daq_core::{
    arc_measurement, measurement_channel, Camera, DaqError, DataPoint, HardwareAdapter, ImageData,
    Instrument, InstrumentCommand, InstrumentState, Measurement, MeasurementReceiver,
    MeasurementSender, PixelBuffer, Result, ROI,
};
use tokio::task::JoinHandle;

use crate::adapters::MockAdapter;

/// PVCAM camera V2 implementation
///
/// Uses PixelBuffer::U16 for native 16-bit camera data (4× memory savings vs f64).
pub struct PVCAMInstrumentV2 {
    id: String,
    adapter: Box<dyn HardwareAdapter>,
    state: InstrumentState,

    // Camera configuration
    camera_name: String,
    exposure_ms: f64,
    roi: ROI,
    binning: (u16, u16),
    sensor_size: (u32, u32),

    // Data streaming
    measurement_tx: MeasurementSender,
    _measurement_rx_keeper: MeasurementReceiver,

    // Task management
    task_handle: Option<JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl PVCAMInstrumentV2 {
    /// Create a new PVCAM instrument with default capacity (1024)
    pub fn new(id: String) -> Self {
        Self::with_capacity(id, 1024)
    }

    /// Create a new PVCAM instrument with specified broadcast capacity
    pub fn with_capacity(id: String, capacity: usize) -> Self {
        Self::with_adapter_and_capacity(id, Box::new(MockAdapter::new()), capacity)
    }

    /// Create a new PVCAM instrument with custom adapter and capacity
    pub fn with_adapter_and_capacity(
        id: String,
        adapter: Box<dyn HardwareAdapter>,
        capacity: usize,
    ) -> Self {
        let (measurement_tx, measurement_rx) = measurement_channel(capacity);

        Self {
            id,
            adapter,
            state: InstrumentState::Disconnected,

            // Default PVCAM configuration
            camera_name: "PrimeBSI".to_string(),
            exposure_ms: 100.0,
            roi: ROI {
                x: 0,
                y: 0,
                width: 2048,
                height: 2048,
            },
            binning: (1, 1),
            sensor_size: (2048, 2048),

            measurement_tx,
            _measurement_rx_keeper: measurement_rx,

            task_handle: None,
            shutdown_tx: None,
        }
    }

    /// Simulate frame data generation (placeholder for PVCAM SDK)
    ///
    /// TODO: Replace with actual PVCAM SDK calls:
    /// - pl_exp_start_seq()
    /// - pl_exp_check_status()
    /// - pl_exp_get_latest_frame()
    fn simulate_frame_data(&self, width: u32, height: u32) -> Vec<u16> {
        let mut frame = vec![0u16; (width * height) as usize];

        // Simple gradient pattern for testing
        for y in 0..height {
            for x in 0..width {
                let value = ((x + y) % 256) as u16 * 256;
                frame[(y * width + x) as usize] = value;
            }
        }

        frame
    }

    /// Calculate frame statistics
    fn calculate_frame_stats(&self, frame: &[u16]) -> (f64, f64, f64) {
        if frame.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let sum: u64 = frame.iter().map(|&v| v as u64).sum();
        let mean = sum as f64 / frame.len() as f64;

        let min = *frame.iter().min().unwrap_or(&0) as f64;
        let max = *frame.iter().max().unwrap_or(&0) as f64;

        (mean, min, max)
    }
}

#[async_trait]
impl Instrument for PVCAMInstrumentV2 {
    fn id(&self) -> &str {
        &self.id
    }

    fn instrument_type(&self) -> &str {
        "pvcam_v2"
    }

    fn state(&self) -> InstrumentState {
        self.state.clone()
    }

    async fn initialize(&mut self) -> Result<()> {
        if self.state != InstrumentState::Disconnected {
            return Err(anyhow::anyhow!("Already initialized"));
        }

        self.state = InstrumentState::Connecting;

        // TODO: Initialize PVCAM SDK
        // pl_pvcam_init()
        // pl_cam_open()
        // Configure ROI, binning, exposure time, etc.

        match self.adapter.connect(&Default::default()).await {
            Ok(()) => {
                self.state = InstrumentState::Ready;
                log::info!(
                    "PVCAM camera '{}' ({}) initialized",
                    self.id,
                    self.camera_name
                );
                log::warn!("PVCAM SDK integration not yet implemented - using simulated data");
                Ok(())
            }
            Err(e) => {
                self.state = InstrumentState::Error(DaqError {
                    message: e.to_string(),
                    can_recover: true,
                });
                Err(e)
            }
        }
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state = InstrumentState::ShuttingDown;

        // Stop any running acquisition
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }

        // TODO: Cleanup PVCAM SDK
        // pl_cam_close()
        // pl_pvcam_uninit()

        self.adapter.disconnect().await?;

        self.state = InstrumentState::Disconnected;
        log::info!("PVCAM camera '{}' shut down", self.id);
        Ok(())
    }

    fn measurement_stream(&self) -> MeasurementReceiver {
        self.measurement_tx.subscribe()
    }

    async fn handle_command(&mut self, cmd: InstrumentCommand) -> Result<()> {
        match cmd {
            InstrumentCommand::Shutdown => self.shutdown().await,
            InstrumentCommand::StartAcquisition => self.start_live().await,
            InstrumentCommand::StopAcquisition => self.stop_live().await,
            InstrumentCommand::SetParameter { name, value } => match name.as_str() {
                "exposure_ms" => {
                    if let Some(ms) = value.as_f64() {
                        self.set_exposure_ms(ms).await
                    } else {
                        Err(anyhow::anyhow!("Invalid exposure value"))
                    }
                }
                "gain" => {
                    // TODO: Set camera gain via PVCAM SDK
                    log::info!("PVCAM gain set to {}", value);
                    Ok(())
                }
                "binning" => {
                    // TODO: Set camera binning via PVCAM SDK
                    log::info!("PVCAM binning set to {}", value);
                    Ok(())
                }
                _ => Err(anyhow::anyhow!("Unknown parameter: {}", name)),
            },
            InstrumentCommand::GetParameter { .. } => {
                // Not implemented yet
                Ok(())
            }
            InstrumentCommand::Recover => self.recover().await,
        }
    }

    async fn recover(&mut self) -> Result<()> {
        match &self.state {
            InstrumentState::Error(daq_error) if daq_error.can_recover => {
                log::info!("Attempting recovery for PVCAM '{}'", self.id);

                let _ = self.adapter.disconnect().await;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                self.adapter.connect(&Default::default()).await?;

                self.state = InstrumentState::Ready;
                log::info!("Recovery successful for PVCAM '{}'", self.id);
                Ok(())
            }
            InstrumentState::Error(_) => {
                Err(anyhow::anyhow!("Cannot recover from unrecoverable error"))
            }
            _ => Err(anyhow::anyhow!(
                "Cannot recover from state: {:?}",
                self.state
            )),
        }
    }
}

#[async_trait]
impl Camera for PVCAMInstrumentV2 {
    async fn snap(&mut self) -> Result<ImageData> {
        if self.state != InstrumentState::Ready {
            return Err(anyhow::anyhow!(
                "PVCAM camera not ready, state: {:?}",
                self.state
            ));
        }

        self.state = InstrumentState::Acquiring;

        // Simulate acquisition delay
        tokio::time::sleep(std::time::Duration::from_millis(self.exposure_ms as u64)).await;

        let width = (self.roi.width / self.binning.0) as u32;
        let height = (self.roi.height / self.binning.1) as u32;

        // Generate u16 frame data (native camera format)
        let frame_data = self.simulate_frame_data(width, height);

        // ✅ KEY FEATURE: Use PixelBuffer::U16 for 4× memory savings!
        let image = ImageData {
            timestamp: Utc::now(),
            channel: format!("{}_image", self.id),
            width,
            height,
            pixels: PixelBuffer::U16(frame_data), // 4× memory reduction vs Vec<f64>
            unit: "counts".to_string(),
            metadata: Some(serde_json::json!({
                "camera_name": self.camera_name,
                "exposure_ms": self.exposure_ms,
                "roi": self.roi,
                "binning": self.binning,
            })),
        };

        // Emit measurement
        let measurement = arc_measurement(Measurement::Image(image.clone()));
        let _ = self.measurement_tx.send(measurement);

        self.state = InstrumentState::Ready;

        Ok(image)
    }

    async fn start_live(&mut self) -> Result<()> {
        if self.state != InstrumentState::Ready {
            return Err(anyhow::anyhow!(
                "Cannot start live from state: {:?}",
                self.state
            ));
        }

        self.state = InstrumentState::Acquiring;

        let tx = self.measurement_tx.clone();
        let id = self.id.clone();
        let camera_name = self.camera_name.clone();
        let exposure_ms = self.exposure_ms;
        let roi = self.roi;
        let binning = self.binning;

        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        // Spawn continuous acquisition task
        self.task_handle = Some(tokio::spawn(async move {
            let width = (roi.width / binning.0) as u32;
            let height = (roi.height / binning.1) as u32;
            let mut frame_count = 0u64;

            log::info!("PVCAM live acquisition started for '{}'", id);

            loop {
                tokio::select! {
                    // Simulate frame acquisition
                    _ = tokio::time::sleep(std::time::Duration::from_millis(exposure_ms as u64)) => {
                        frame_count += 1;

                        // Generate u16 frame (simulating PVCAM SDK output)
                        let mut frame_data = vec![0u16; (width * height) as usize];
                        for y in 0..height {
                            for x in 0..width {
                                // Add timestamp variation for dynamic pattern
                                let offset = (Utc::now().timestamp() % 256) as u32;
                                let value = ((x + y + offset) % 256) as u16 * 256;
                                frame_data[(y * width + x) as usize] = value;
                            }
                        }

                        // Calculate statistics
                        let sum: u64 = frame_data.iter().map(|&v| v as u64).sum();
                        let mean = sum as f64 / frame_data.len() as f64;
                        let min = *frame_data.iter().min().unwrap_or(&0) as f64;
                        let max = *frame_data.iter().max().unwrap_or(&0) as f64;

                        let timestamp = Utc::now();

                        // Emit image measurement with PixelBuffer::U16
                        let image = ImageData {
                            timestamp,
                            channel: format!("{}_image", id),
                            width,
                            height,
                            pixels: PixelBuffer::U16(frame_data), // ✅ 4× memory savings
                            unit: "counts".to_string(),
                            metadata: Some(serde_json::json!({
                                "camera_name": camera_name,
                                "exposure_ms": exposure_ms,
                                "roi": roi,
                                "binning": binning,
                                "frame": frame_count,
                            })),
                        };

                        let measurement = arc_measurement(Measurement::Image(image));
                        if tx.send(measurement.clone()).is_err() {
                            log::info!("No receivers for image, stopping acquisition");
                            break;
                        }

                        // Also emit scalar statistics
                        let dp_mean = DataPoint {
                            timestamp,
                            channel: format!("{}_mean_intensity", id),
                            value: mean,
                            unit: "counts".to_string(),
                        };
                        let _ = tx.send(arc_measurement(Measurement::Scalar(dp_mean)));

                        let dp_min = DataPoint {
                            timestamp,
                            channel: format!("{}_min_intensity", id),
                            value: min,
                            unit: "counts".to_string(),
                        };
                        let _ = tx.send(arc_measurement(Measurement::Scalar(dp_min)));

                        let dp_max = DataPoint {
                            timestamp,
                            channel: format!("{}_max_intensity", id),
                            value: max,
                            unit: "counts".to_string(),
                        };
                        let _ = tx.send(arc_measurement(Measurement::Scalar(dp_max)));
                    }
                    // Shutdown signal
                    _ = &mut shutdown_rx => {
                        log::info!("PVCAM acquisition shutdown requested for '{}'", id);
                        break;
                    }
                }
            }

            log::info!("PVCAM live acquisition stopped for '{}'", id);
        }));

        Ok(())
    }

    async fn stop_live(&mut self) -> Result<()> {
        if self.state != InstrumentState::Acquiring {
            return Ok(()); // Already stopped
        }

        // Signal shutdown
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Wait for task to finish
        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }

        self.state = InstrumentState::Ready;
        log::info!("Stopped live acquisition for PVCAM '{}'", self.id);
        Ok(())
    }

    async fn set_exposure_ms(&mut self, ms: f64) -> Result<()> {
        if self.state == InstrumentState::Acquiring {
            return Err(anyhow::anyhow!("Cannot change exposure while acquiring"));
        }
        self.exposure_ms = ms;
        // TODO: Apply to PVCAM hardware
        Ok(())
    }

    async fn get_exposure_ms(&self) -> f64 {
        self.exposure_ms
    }

    async fn set_roi(&mut self, roi: ROI) -> Result<()> {
        if self.state == InstrumentState::Acquiring {
            return Err(anyhow::anyhow!("Cannot change ROI while acquiring"));
        }
        self.roi = roi;
        // TODO: Apply to PVCAM hardware
        Ok(())
    }

    async fn get_roi(&self) -> ROI {
        self.roi
    }

    async fn set_binning(&mut self, x: u16, y: u16) -> Result<()> {
        if self.state == InstrumentState::Acquiring {
            return Err(anyhow::anyhow!("Cannot change binning while acquiring"));
        }
        self.binning = (x, y);
        // TODO: Apply to PVCAM hardware
        Ok(())
    }

    async fn get_binning(&self) -> (u16, u16) {
        self.binning
    }

    fn get_sensor_size(&self) -> (u32, u32) {
        self.sensor_size
    }

    fn get_pixel_size_um(&self) -> (f64, f64) {
        (6.5, 6.5) // PrimeBSI pixel size
    }

    fn supports_hardware_trigger(&self) -> bool {
        true // PVCAM supports hardware triggering
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pvcam_lifecycle() {
        let mut camera = PVCAMInstrumentV2::new("test_pvcam".to_string());

        assert_eq!(camera.state(), InstrumentState::Disconnected);

        camera.initialize().await.unwrap();
        assert_eq!(camera.state(), InstrumentState::Ready);

        camera.shutdown().await.unwrap();
        assert_eq!(camera.state(), InstrumentState::Disconnected);
    }

    #[tokio::test]
    async fn test_pvcam_snap_uses_pixelbuffer_u16() {
        let mut camera = PVCAMInstrumentV2::new("test_pvcam".to_string());
        camera.initialize().await.unwrap();

        let image = camera.snap().await.unwrap();

        // Verify PixelBuffer::U16 is used
        match image.pixels {
            PixelBuffer::U16(ref data) => {
                assert_eq!(data.len(), 2048 * 2048);
                // Verify u16 values are in valid range
                assert!(data.iter().all(|&v| v <= u16::MAX));
            }
            _ => panic!("Expected PixelBuffer::U16, got {:?}", image.pixels),
        }

        // Verify memory savings
        let actual_bytes = image.pixels.memory_bytes();
        let expected_bytes = 2048 * 2048 * 2; // u16 = 2 bytes/pixel
        assert_eq!(actual_bytes, expected_bytes);

        // Compare to old Vec<f64> approach
        let old_bytes = 2048 * 2048 * 8; // f64 = 8 bytes/pixel
        let savings = old_bytes - actual_bytes;
        assert_eq!(savings, 25165824); // 24 MB savings per frame!
    }

    #[tokio::test]
    async fn test_pvcam_live_acquisition() {
        let mut camera = PVCAMInstrumentV2::new("test_pvcam".to_string());
        camera.initialize().await.unwrap();

        let mut rx = camera.measurement_stream();

        camera.start_live().await.unwrap();
        assert_eq!(camera.state(), InstrumentState::Acquiring);

        // Receive a few measurements (mix of Image and Scalar)
        for _ in 0..10 {
            let measurement = rx.recv().await.unwrap();
            match &*measurement {
                Measurement::Image(image) => {
                    // Verify PixelBuffer::U16
                    assert!(matches!(image.pixels, PixelBuffer::U16(_)));
                }
                Measurement::Scalar(data) => {
                    // Statistics channels
                    assert!(
                        data.channel.contains("mean")
                            || data.channel.contains("min")
                            || data.channel.contains("max")
                    );
                }
                _ => panic!("Unexpected measurement type"),
            }
        }

        camera.stop_live().await.unwrap();
        assert_eq!(camera.state(), InstrumentState::Ready);
    }
}