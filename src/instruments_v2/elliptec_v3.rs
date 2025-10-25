//! Thorlabs Elliptec ELL14 Rotation Stage V3 (Unified Architecture)
//!
//! V3 implementation using the unified core_v3 traits:
//! - Implements `core_v3::Instrument` trait (replaces V1/V2 split)
//! - Implements `core_v3::Stage` trait for motion control polymorphism
//! - Uses `Parameter<T>` for declarative parameter management
//! - Direct async methods (no InstrumentCommand message passing)
//! - Single broadcast channel (no double-broadcast overhead)
//!
//! ## Key Validation Points
//!
//! This is the SECOND Stage implementation (after ESP300 V3), validating that:
//! - Same `Stage` trait works for different hardware (Newport ESP300 vs Thorlabs Elliptec)
//! - Binary protocol (Elliptec) vs ASCII protocol (ESP300) both fit the trait
//! - Trait abstraction enables hardware-agnostic motion control code
//!
//! ## Configuration
//!
//! ```toml
//! [instruments.rotator]
//! type = "elliptec_v3"
//! port = "/dev/ttyUSB0"
//! baud_rate = 9600
//! device_address = 0
//! min_position_deg = 0.0
//! max_position_deg = 360.0
//! sdk_mode = "mock"  # or "real" for actual hardware
//! ```
//!
//! ## Elliptec Protocol (Official Thorlabs Specification)
//!
//! Binary protocol over RS-232 serial:
//! - Baud: 9600, 8N1, no flow control
//! - Commands: ASCII format `<address><cmd>[data]\r`
//! - Responses: ASCII format `<address><status>[data]\r`
//! - ELL14 pulses per revolution: 136,533 (official specification)
//! - Timing: 100ms delay after command, 100ms after response (200ms cycle minimum)
//!
//! Common commands:
//! - `gp` - Get position (response: `PO<8-hex-digits>`)
//! - `ma<8-hex>` - Move absolute to position
//! - `ho` - Home (find reference position)
//! - `gs` - Get status (response: `GS<4-hex-status>`)
//!
//! ## Migration from V2
//!
//! V3 eliminates the MotionController trait and actor model:
//! - V2: MotionController trait with axis numbering + handle_command()
//! - V3: Direct Stage trait methods (same as ESP300 V3)

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::core_v3::{
    Command, Instrument, InstrumentState, Measurement, ParameterBase, Response, Stage,
};
use crate::parameter::{Parameter, ParameterBuilder};

// =============================================================================
// Serial Port Abstraction (for testing)
// =============================================================================

/// Serial port abstraction trait for testing
#[async_trait]
trait SerialPort: Send + Sync {
    async fn write(&mut self, data: &str) -> Result<()>;
    async fn read_line(&mut self) -> Result<String>;
}

/// Mock serial port for testing
struct MockSerialPort {
    position_counts: u32,
    is_moving: bool,
    is_homed: bool,
    last_command: String,
}

impl MockSerialPort {
    fn new() -> Self {
        Self {
            position_counts: 0,
            is_moving: false,
            is_homed: false,
            last_command: String::new(),
        }
    }

    fn counts_to_degrees(counts: u32) -> f64 {
        // ELL14 official specification: 136,533 counts = 360 degrees
        (counts as f64 / 136533.0) * 360.0
    }

    fn degrees_to_counts(degrees: f64) -> u32 {
        // Normalize to 0-360 range
        let normalized = degrees.rem_euclid(360.0);
        ((normalized / 360.0) * 136533.0) as u32
    }
}

#[async_trait]
impl SerialPort for MockSerialPort {
    async fn write(&mut self, data: &str) -> Result<()> {
        let cmd = data.trim();
        self.last_command = cmd.to_string();

        // Parse Elliptec commands
        if cmd.len() < 2 {
            return Err(anyhow!("Command too short: {}", cmd));
        }

        // Extract address (first character, should be hex digit)
        let _address = &cmd[0..1];
        let command_part = &cmd[1..];

        // Parse command type (first 2 chars after address)
        if command_part.len() >= 2 {
            let cmd_type = &command_part[0..2];

            match cmd_type {
                "ma" => {
                    // Move absolute - expect 8 hex digits
                    if command_part.len() >= 10 {
                        let hex_pos = &command_part[2..10];
                        self.position_counts = u32::from_str_radix(hex_pos, 16)?;
                        self.is_moving = true;
                    }
                }
                "ho" => {
                    // Home command
                    self.position_counts = 0;
                    self.is_homed = true;
                    self.is_moving = false;
                }
                "gp" | "gs" => {
                    // Query commands - no state change
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn read_line(&mut self) -> Result<String> {
        // Simulate motion settling
        if self.is_moving {
            self.is_moving = false;
        }

        // Generate response based on last command
        let response = if self.last_command.contains("gp") {
            // Position query - format: "0PO12345678\r"
            format!("0PO{:08X}\r", self.position_counts)
        } else if self.last_command.contains("gs") {
            // Status query - format: "0GS1234\r"
            // Bit 8 (0x0100) = homed, Bit 1 (0x0002) = moving
            let mut status: u16 = 0;
            if self.is_homed {
                status |= 0x0100;
            }
            if self.is_moving {
                status |= 0x0002;
            }
            format!("0GS{:04X}\r", status)
        } else {
            // Generic OK response
            "0GS0000\r".to_string()
        };

        Ok(response)
    }
}

/// Real serial port implementation using synchronous I/O
#[cfg(feature = "instrument_serial")]
struct RealSerialPort {
    port: std::sync::Mutex<Box<dyn serialport::SerialPort>>,
}

#[cfg(feature = "instrument_serial")]
#[async_trait]
impl SerialPort for RealSerialPort {
    async fn write(&mut self, data: &str) -> Result<()> {
        use std::io::Write;
        {
            let mut port = self.port.lock().unwrap();
            port.write_all(data.as_bytes())?;
        } // MutexGuard dropped here

        // Official timing requirement: 100ms delay after sending
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Ok(())
    }

    async fn read_line(&mut self) -> Result<String> {
        use std::io::Read;
        let line = {
            let mut port = self.port.lock().unwrap();
            let mut buffer = vec![0u8; 128];
            let n = port.read(&mut buffer)?;
            String::from_utf8_lossy(&buffer[..n]).to_string()
        }; // MutexGuard dropped here

        // Official timing requirement: 100ms delay after receiving
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Ok(line)
    }
}

// =============================================================================
// SDK Mode Selection
// =============================================================================

/// SDK mode for Elliptec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElliptecSdkKind {
    /// Mock serial for testing
    Mock,
    /// Real serial hardware
    Real,
}

// =============================================================================
// Elliptec V3
// =============================================================================

/// Thorlabs Elliptec ELL14 Rotation Stage V3 implementation
///
/// Unified architecture implementation demonstrating:
/// - Direct `Instrument` + `Stage` trait implementation (same as ESP300 V3)
/// - `Parameter<T>` for declarative settings
/// - Single broadcast channel for data streaming
/// - Direct async methods (no message passing)
/// - Binary protocol abstraction layer (Mock/Real)
///
/// **Validation**: This is the SECOND Stage implementation, proving that the
/// Stage trait works identically for different hardware (Elliptec vs ESP300).
pub struct ElliptecV3 {
    /// Instrument identifier
    id: String,

    /// Current state
    state: InstrumentState,

    /// Data broadcast channel
    data_tx: broadcast::Sender<Measurement>,

    /// Parameters (for dynamic access via ParameterBase)
    parameters: HashMap<String, Box<dyn ParameterBase>>,

    // Serial abstraction (wrapped in Arc<Mutex> for interior mutability)
    serial_port: Arc<Mutex<Option<Box<dyn SerialPort>>>>,
    port_path: String,
    baud_rate: u32,
    sdk_kind: ElliptecSdkKind,

    // Device configuration
    device_address: u8,

    // Stage parameters (for direct access via Stage trait)
    min_position_deg: Arc<RwLock<Parameter<f64>>>,
    max_position_deg: Arc<RwLock<Parameter<f64>>>,

    // Cached position (updated on queries)
    cached_position: Arc<RwLock<f64>>,

    // Elliptec-specific constants
    counts_per_rotation: f64, // ELL14: 136,533 counts = 360 degrees
}

impl ElliptecV3 {
    /// Create new Elliptec V3 instance
    ///
    /// # Arguments
    /// * `id` - Unique instrument identifier
    /// * `port_path` - Serial port path (e.g., "/dev/ttyUSB0")
    /// * `sdk_kind` - Mock or Real serial mode
    /// * `device_address` - Elliptec device address (0-15, hex digit)
    pub fn new(
        id: impl Into<String>,
        port_path: impl Into<String>,
        sdk_kind: ElliptecSdkKind,
        device_address: u8,
    ) -> Self {
        let id = id.into();
        let (data_tx, _) = broadcast::channel(1024);

        // Validate device address (must be 0-15 for hex encoding)
        if device_address > 15 {
            panic!("Device address must be 0-15, got {}", device_address);
        }

        // Create parameters
        let min_position_deg = Arc::new(RwLock::new(
            ParameterBuilder::new("min_position_deg", 0.0)
                .description("Minimum position limit")
                .unit("deg")
                .build(),
        ));

        let max_position_deg = Arc::new(RwLock::new(
            ParameterBuilder::new("max_position_deg", 360.0)
                .description("Maximum position limit")
                .unit("deg")
                .build(),
        ));

        Self {
            id,
            state: InstrumentState::Uninitialized,
            data_tx,
            parameters: HashMap::new(),
            serial_port: Arc::new(Mutex::new(None)),
            port_path: port_path.into(),
            baud_rate: 9600, // Elliptec standard baud rate
            sdk_kind,
            device_address,
            min_position_deg,
            max_position_deg,
            cached_position: Arc::new(RwLock::new(0.0)),
            counts_per_rotation: 136533.0, // ELL14 official specification
        }
    }

    /// Send command to Elliptec device
    async fn send_command(&self, cmd: &str) -> Result<()> {
        let mut port = self.serial_port.lock().await;
        if let Some(port) = &mut *port {
            let command = format!("{:X}{}\r", self.device_address, cmd);
            port.write(&command).await
        } else {
            Err(anyhow!("Serial port not initialized"))
        }
    }

    /// Read response from Elliptec device
    async fn read_response(&self) -> Result<String> {
        let mut port = self.serial_port.lock().await;
        if let Some(port) = &mut *port {
            let response = port.read_line().await?;
            Ok(response.trim().to_string())
        } else {
            Err(anyhow!("Serial port not initialized"))
        }
    }

    /// Query and broadcast current position
    async fn update_position(&mut self) -> Result<()> {
        let pos = self.position().await?;

        // Broadcast measurement
        let measurement = Measurement::Scalar {
            name: format!("{}_position", self.id),
            value: pos,
            unit: "deg".to_string(),
            timestamp: Utc::now(),
        };
        let _ = self.data_tx.send(measurement);

        Ok(())
    }

    /// Check status for errors and homing state
    async fn check_status(&self) -> Result<u16> {
        self.send_command("gs").await?;
        let response = self.read_response().await?;

        // Response format: "0GS1234" where 1234 is 4-char hex status word
        if response.len() < 7 || &response[1..3] != "GS" {
            return Err(anyhow!("Invalid status response format: {}", response));
        }

        let status_hex = &response[3..7];
        let status = u16::from_str_radix(status_hex, 16).map_err(|e| {
            anyhow!(
                "Failed to parse status word '{}': {} (response: {})",
                status_hex,
                e,
                response
            )
        })?;

        // Check error bit 9 (0x0200)
        if (status & 0x0200) != 0 {
            return Err(anyhow!(
                "Elliptec device {} error detected (status: 0x{:04X})",
                self.device_address,
                status
            ));
        }

        Ok(status)
    }

    /// Convert counts to degrees
    fn counts_to_degrees(&self, counts: u32) -> f64 {
        (counts as f64 / self.counts_per_rotation) * 360.0
    }

    /// Convert degrees to counts
    fn degrees_to_counts(&self, degrees: f64) -> u32 {
        let normalized = degrees.rem_euclid(360.0);
        ((normalized / 360.0) * self.counts_per_rotation) as u32
    }
}

// =============================================================================
// Instrument Trait Implementation
// =============================================================================

#[async_trait]
impl Instrument for ElliptecV3 {
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

        // Initialize serial port based on SDK kind
        match self.sdk_kind {
            ElliptecSdkKind::Mock => {
                let mut port = self.serial_port.lock().await;
                *port = Some(Box::new(MockSerialPort::new()));
            }
            #[cfg(feature = "instrument_serial")]
            ElliptecSdkKind::Real => {
                let serial = serialport::new(&self.port_path, self.baud_rate)
                    .timeout(std::time::Duration::from_millis(500))
                    .open()
                    .map_err(|e| anyhow!("Failed to open {}: {}", self.port_path, e))?;
                let mut port = self.serial_port.lock().await;
                *port = Some(Box::new(RealSerialPort {
                    port: std::sync::Mutex::new(serial),
                }));
            }
            #[cfg(not(feature = "instrument_serial"))]
            ElliptecSdkKind::Real => {
                return Err(anyhow!(
                    "Real serial not available - enable 'instrument_serial' feature"
                ));
            }
        }

        // Query device status
        let status = self.check_status().await?;
        log::info!(
            "Elliptec device {} status: 0x{:04X}",
            self.device_address,
            status
        );

        self.state = InstrumentState::Idle;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.state = InstrumentState::ShuttingDown;
        let mut port = self.serial_port.lock().await;
        *port = None;
        Ok(())
    }

    fn data_channel(&self) -> broadcast::Receiver<Measurement> {
        self.data_tx.subscribe()
    }

    async fn execute(&mut self, cmd: Command) -> Result<Response> {
        match cmd {
            Command::Start => {
                self.state = InstrumentState::Running;
                Ok(Response::Ok)
            }
            Command::Stop => {
                self.state = InstrumentState::Idle;
                Ok(Response::Ok)
            }
            _ => Ok(Response::Ok),
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
// Stage Trait Implementation
// =============================================================================

#[async_trait]
impl Stage for ElliptecV3 {
    async fn move_absolute(&mut self, position_deg: f64) -> Result<()> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        // Check position limits
        let min = self.min_position_deg.read().await.get();
        let max = self.max_position_deg.read().await.get();

        if position_deg < min || position_deg > max {
            return Err(anyhow!(
                "Position {} deg out of range [{}, {}]",
                position_deg,
                min,
                max
            ));
        }

        // Convert to counts
        let counts = self.degrees_to_counts(position_deg);

        // Send move absolute command
        self.send_command(&format!("ma{:08X}", counts)).await?;

        // Update cached position
        {
            let mut cached = self.cached_position.write().await;
            *cached = position_deg;
        }

        // Broadcast position update
        self.update_position().await?;

        Ok(())
    }

    async fn move_relative(&mut self, distance_deg: f64) -> Result<()> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        let current_pos = self.position().await?;
        self.move_absolute(current_pos + distance_deg).await
    }

    async fn position(&self) -> Result<f64> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        // Query position
        self.send_command("gp").await?;
        let response = self.read_response().await?;

        // Response format: "0PO12345678"
        if response.len() < 11 || &response[1..3] != "PO" {
            return Err(anyhow!("Invalid position response: {}", response));
        }

        let hex_pos = &response[3..11];
        let counts = u32::from_str_radix(hex_pos, 16).map_err(|e| {
            anyhow!(
                "Failed to parse hex position '{}': {} (response: {})",
                hex_pos,
                e,
                response
            )
        })?;

        let position = self.counts_to_degrees(counts);

        // Update cached position
        let mut cached = self.cached_position.write().await;
        *cached = position;

        Ok(position)
    }

    async fn stop_motion(&mut self) -> Result<()> {
        // ELL14 doesn't have a stop command (position-based moves)
        Err(anyhow!("ELL14 does not support stop command"))
    }

    async fn is_moving(&self) -> Result<bool> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        // Query status
        let status = self.check_status().await?;

        // Bit 1 (0x0002) indicates moving
        Ok((status & 0x0002) != 0)
    }

    async fn home(&mut self) -> Result<()> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        // Send home command
        self.send_command("ho").await?;

        // Wait for homing to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Poll status until homing complete
        for _ in 0..50 {
            let status = self.check_status().await?;

            // Bit 8 (0x0100) = homed, Bit 7 (0x0080) = homing in progress
            if (status & 0x0100) != 0 && (status & 0x0080) == 0 {
                log::debug!(
                    "Elliptec device {} homed (status: 0x{:04X})",
                    self.device_address,
                    status
                );

                // Update position
                self.update_position().await?;
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        Err(anyhow!("Elliptec homing timeout"))
    }

    async fn set_velocity(&mut self, _deg_per_sec: f64) -> Result<()> {
        // ELL14 doesn't support velocity control (fixed speed)
        Err(anyhow!("ELL14 does not support velocity control"))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_elliptec_v3_initialization() {
        let mut stage = ElliptecV3::new("test_elliptec", "/dev/tty.mock", ElliptecSdkKind::Mock, 0);
        assert_eq!(stage.state(), InstrumentState::Uninitialized);

        stage.initialize().await.unwrap();
        assert_eq!(stage.state(), InstrumentState::Idle);
    }

    #[tokio::test]
    async fn test_elliptec_v3_absolute_move() {
        let mut stage = ElliptecV3::new("test_elliptec", "/dev/tty.mock", ElliptecSdkKind::Mock, 0);
        stage.initialize().await.unwrap();

        // Move to position
        stage.move_absolute(90.0).await.unwrap();

        // Verify position
        let pos = stage.position().await.unwrap();
        assert!((pos - 90.0).abs() < 1.0); // Within 1 degree
    }

    #[tokio::test]
    async fn test_elliptec_v3_relative_move() {
        let mut stage = ElliptecV3::new("test_elliptec", "/dev/tty.mock", ElliptecSdkKind::Mock, 0);
        stage.initialize().await.unwrap();

        // Move to known position
        stage.move_absolute(45.0).await.unwrap();

        // Move relative
        stage.move_relative(45.0).await.unwrap();

        // Verify position
        let pos = stage.position().await.unwrap();
        assert!((pos - 90.0).abs() < 1.0);
    }

    #[tokio::test]
    async fn test_elliptec_v3_position_query() {
        let mut stage = ElliptecV3::new("test_elliptec", "/dev/tty.mock", ElliptecSdkKind::Mock, 0);
        stage.initialize().await.unwrap();

        // Query initial position
        let pos = stage.position().await.unwrap();
        assert!(pos >= 0.0 && pos < 360.0);
    }

    #[tokio::test]
    async fn test_elliptec_v3_homing() {
        let mut stage = ElliptecV3::new("test_elliptec", "/dev/tty.mock", ElliptecSdkKind::Mock, 0);
        stage.initialize().await.unwrap();

        // Move away from zero
        stage.move_absolute(180.0).await.unwrap();

        // Home the stage
        stage.home().await.unwrap();

        // Verify at home position
        let pos = stage.position().await.unwrap();
        assert!((pos - 0.0).abs() < 1.0);
    }

    #[tokio::test]
    async fn test_elliptec_v3_parameter_validation() {
        let mut stage = ElliptecV3::new("test_elliptec", "/dev/tty.mock", ElliptecSdkKind::Mock, 0);
        stage.initialize().await.unwrap();

        // Invalid position (out of range)
        let result = stage.move_absolute(400.0).await;
        assert!(result.is_err());

        // Valid position
        stage.move_absolute(180.0).await.unwrap();
    }

    #[tokio::test]
    async fn test_elliptec_v3_shutdown() {
        let mut stage = ElliptecV3::new("test_elliptec", "/dev/tty.mock", ElliptecSdkKind::Mock, 0);
        stage.initialize().await.unwrap();

        stage.shutdown().await.unwrap();
        assert_eq!(stage.state(), InstrumentState::ShuttingDown);
    }

    #[tokio::test]
    async fn test_elliptec_v3_motion_status() {
        let mut stage = ElliptecV3::new("test_elliptec", "/dev/tty.mock", ElliptecSdkKind::Mock, 0);
        stage.initialize().await.unwrap();

        // Check if moving (should be stationary initially)
        let moving = stage.is_moving().await.unwrap();
        assert!(!moving);
    }

    #[tokio::test]
    async fn test_elliptec_v3_counts_conversion() {
        let stage = ElliptecV3::new("test", "/dev/tty.mock", ElliptecSdkKind::Mock, 0);

        // Test full rotation (136,533 counts = 360 degrees)
        let counts = 136533u32;
        let degrees = stage.counts_to_degrees(counts);
        assert!((degrees - 360.0).abs() < 0.01);

        // Test half rotation
        let counts = 68266u32; // Approximately half
        let degrees = stage.counts_to_degrees(counts);
        assert!((degrees - 180.0).abs() < 0.5);

        // Test degrees to counts conversion
        let counts = stage.degrees_to_counts(180.0);
        let degrees = stage.counts_to_degrees(counts);
        assert!((degrees - 180.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_elliptec_v3_stage_trait_compatibility() {
        // This test validates that Elliptec V3 implements the same Stage trait as ESP300 V3
        let mut stage: Box<dyn Stage> = Box::new(ElliptecV3::new(
            "test_elliptec",
            "/dev/tty.mock",
            ElliptecSdkKind::Mock,
            0,
        ));

        stage.initialize().await.unwrap();

        // Test that Stage trait methods work identically to ESP300 V3
        stage.move_absolute(90.0).await.unwrap();
        let pos = stage.position().await.unwrap();
        assert!((pos - 90.0).abs() < 1.0);

        stage.home().await.unwrap();
        let pos = stage.position().await.unwrap();
        assert!((pos - 0.0).abs() < 1.0);
    }
}