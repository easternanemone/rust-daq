//! Newport ESP300 3-axis Motion Controller V3 (Unified Architecture)
//!
//! V3 implementation using the unified core_v3 traits:
//! - Implements `core_v3::Instrument` trait (replaces V1/V2 split)
//! - Implements `core_v3::Stage` trait for motion control polymorphism
//! - Uses `Parameter<T>` for declarative parameter management
//! - Direct async methods (no InstrumentCommand message passing)
//! - Single broadcast channel (no double-broadcast overhead)
//!
//! ## Configuration
//!
//! ```toml
//! [instruments.stage]
//! type = "esp300_v3"
//! port = "/dev/ttyUSB0"
//! baud_rate = 19200
//! axis = 1
//! velocity_mm_s = 5.0
//! acceleration_mm_s2 = 10.0
//! min_position_mm = 0.0
//! max_position_mm = 100.0
//! sdk_mode = "mock"  # or "real" for actual hardware
//! ```
//!
//! ## Protocol
//!
//! ESP300 uses RS-232 serial communication with SCPI-like commands:
//! - Baud: 19200, 8N1, hardware flow control
//! - Commands: "1PA{pos}" (move absolute), "1TP?" (query position), "1OR" (home)
//! - Responses: ASCII text terminated by "\r\n"
//!
//! ## Migration from V2
//!
//! V3 eliminates the SerialAdapter and actor model:
//! - V2: SerialAdapter + handle_command() → Complex message passing
//! - V3: Direct trait methods (move_absolute(), position(), etc.)

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
    position: f64,
    velocity: f64,
    is_moving: bool,
    last_command: String,
}

impl MockSerialPort {
    fn new() -> Self {
        Self {
            position: 0.0,
            velocity: 5.0,
            is_moving: false,
            last_command: String::new(),
        }
    }
}

#[async_trait]
impl SerialPort for MockSerialPort {
    async fn write(&mut self, data: &str) -> Result<()> {
        let cmd = data.trim();
        self.last_command = cmd.to_string();

        // Parse ESP300 commands (axis number is first character)
        if let Some(axis_char) = cmd.chars().next() {
            if axis_char == '1' {
                let cmd_type = &cmd[1..];

                // Handle queries (commands ending with ?)
                if cmd_type.ends_with('?') {
                    // Query commands don't modify state, just store for response
                    return Ok(());
                }

                // Handle commands with values
                let (cmd_prefix, value_str) = if cmd_type.len() >= 2 {
                    (&cmd_type[0..2], &cmd_type[2..])
                } else {
                    (cmd_type, "")
                };

                match cmd_prefix {
                    "PA" => {
                        // Move absolute
                        self.position = value_str.parse()?;
                        self.is_moving = true;
                    }
                    "PR" => {
                        // Move relative
                        let delta: f64 = value_str.parse()?;
                        self.position += delta;
                        self.is_moving = true;
                    }
                    "VA" => {
                        // Set velocity
                        self.velocity = value_str.parse()?;
                    }
                    "ST" => {
                        // Stop motion
                        self.is_moving = false;
                    }
                    "OR" => {
                        // Home (find origin)
                        self.position = 0.0;
                        self.is_moving = false;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    async fn read_line(&mut self) -> Result<String> {
        // Simulate motion settling
        if self.is_moving {
            self.is_moving = false;
        }

        // Return mock responses based on last command
        let response = if self.last_command.contains("TP?") {
            // Position query
            format!("{:.6}\r\n", self.position)
        } else if self.last_command.contains("MD?") {
            // Motion done query - return 0 if not moving, 1 if moving
            if self.is_moving {
                "1\r\n".to_string()
            } else {
                "0\r\n".to_string()
            }
        } else if self.last_command.contains("VE?") {
            // Version query
            "ESP300 Version 3.0\r\n".to_string()
        } else {
            // Default response
            "OK\r\n".to_string()
        };

        Ok(response)
    }
}

/// Real serial port implementation using synchronous I/O
///
/// Note: Uses std::io blocking I/O wrapped in Mutex rather than tokio_serial.
/// This is acceptable for ESP300's command-response protocol.
/// Future enhancement: Consider tokio_serial for high-frequency polling.
#[cfg(feature = "instrument_serial")]
struct RealSerialPort {
    port: std::sync::Mutex<Box<dyn serialport::SerialPort>>,
}

#[cfg(feature = "instrument_serial")]
#[async_trait]
impl SerialPort for RealSerialPort {
    async fn write(&mut self, data: &str) -> Result<()> {
        use std::io::Write;
        let mut port = self.port.lock().unwrap();
        port.write_all(data.as_bytes())?;
        Ok(())
    }

    async fn read_line(&mut self) -> Result<String> {
        use std::io::Read;
        let mut port = self.port.lock().unwrap();
        let mut buffer = vec![0u8; 128];
        let n = port.read(&mut buffer)?;
        let line = String::from_utf8_lossy(&buffer[..n]).to_string();
        Ok(line)
    }
}

// =============================================================================
// SDK Mode Selection
// =============================================================================

/// SDK mode for ESP300
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ESP300SdkKind {
    /// Mock serial for testing
    Mock,
    /// Real serial hardware
    Real,
}

// =============================================================================
// ESP300 V3
// =============================================================================

/// Newport ESP300 Motion Controller V3 implementation
///
/// Unified architecture implementation demonstrating:
/// - Direct `Instrument` + `Stage` trait implementation
/// - `Parameter<T>` for declarative settings
/// - Single broadcast channel for data streaming
/// - Direct async methods (no message passing)
/// - Serial abstraction layer (Mock/Real)
pub struct ESP300V3 {
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
    sdk_kind: ESP300SdkKind,

    // Stage parameters (for direct access via Stage trait)
    axis: u32,
    velocity_mm_s: Arc<RwLock<Parameter<f64>>>,
    acceleration_mm_s2: Arc<RwLock<Parameter<f64>>>,
    min_position_mm: Arc<RwLock<Parameter<f64>>>,
    max_position_mm: Arc<RwLock<Parameter<f64>>>,

    // Cached position (updated on queries)
    cached_position: Arc<RwLock<f64>>,
}

impl ESP300V3 {
    /// Create new ESP300 V3 instance
    ///
    /// # Arguments
    /// * `id` - Unique instrument identifier
    /// * `port_path` - Serial port path (e.g., "/dev/ttyUSB0")
    /// * `sdk_kind` - Mock or Real serial mode
    /// * `axis` - Axis number (1-3)
    pub fn new(
        id: impl Into<String>,
        port_path: impl Into<String>,
        sdk_kind: ESP300SdkKind,
        axis: u32,
    ) -> Self {
        let id = id.into();
        let (data_tx, _) = broadcast::channel(1024);

        // Create parameters
        let velocity_mm_s = Arc::new(RwLock::new(
            ParameterBuilder::new("velocity_mm_s", 5.0)
                .description("Stage velocity")
                .unit("mm/s")
                .range(0.001, 300.0)
                .build(),
        ));

        let acceleration_mm_s2 = Arc::new(RwLock::new(
            ParameterBuilder::new("acceleration_mm_s2", 10.0)
                .description("Stage acceleration")
                .unit("mm/s²")
                .range(0.001, 1000.0)
                .build(),
        ));

        let min_position_mm = Arc::new(RwLock::new(
            ParameterBuilder::new("min_position_mm", 0.0)
                .description("Minimum position limit")
                .unit("mm")
                .build(),
        ));

        let max_position_mm = Arc::new(RwLock::new(
            ParameterBuilder::new("max_position_mm", 100.0)
                .description("Maximum position limit")
                .unit("mm")
                .build(),
        ));

        Self {
            id,
            state: InstrumentState::Uninitialized,
            data_tx,
            parameters: HashMap::new(),
            serial_port: Arc::new(Mutex::new(None)),
            port_path: port_path.into(),
            baud_rate: 19200,
            sdk_kind,
            axis,
            velocity_mm_s,
            acceleration_mm_s2,
            min_position_mm,
            max_position_mm,
            cached_position: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Send command to ESP300
    async fn send_command(&self, cmd: &str) -> Result<()> {
        let mut port = self.serial_port.lock().await;
        if let Some(port) = &mut *port {
            let command = format!("{}\r\n", cmd);
            port.write(&command).await
        } else {
            Err(anyhow!("Serial port not initialized"))
        }
    }

    /// Read response from ESP300
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
            unit: "mm".to_string(),
            timestamp: Utc::now(),
        };
        let _ = self.data_tx.send(measurement);

        Ok(())
    }
}

// =============================================================================
// Instrument Trait Implementation
// =============================================================================

#[async_trait]
impl Instrument for ESP300V3 {
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
            ESP300SdkKind::Mock => {
                let mut port = self.serial_port.lock().await;
                *port = Some(Box::new(MockSerialPort::new()));
            }
            #[cfg(feature = "instrument_serial")]
            ESP300SdkKind::Real => {
                let serial = serialport::new(&self.port_path, self.baud_rate)
                    .timeout(std::time::Duration::from_millis(100))
                    .open()
                    .map_err(|e| anyhow!("Failed to open {}: {}", self.port_path, e))?;
                let mut port = self.serial_port.lock().await;
                *port = Some(Box::new(RealSerialPort {
                    port: std::sync::Mutex::new(serial),
                }));
            }
            #[cfg(not(feature = "instrument_serial"))]
            ESP300SdkKind::Real => {
                return Err(anyhow!(
                    "Real serial not available - enable 'instrument_serial' feature"
                ));
            }
        }

        // Query version
        self.send_command("VE?").await?;
        let _version = self.read_response().await?;

        // Configure axis with initial parameters
        let velocity = self.velocity_mm_s.read().await.get();
        self.send_command(&format!("{}VA{}", self.axis, velocity))
            .await?;

        let acceleration = self.acceleration_mm_s2.read().await.get();
        self.send_command(&format!("{}AC{}", self.axis, acceleration))
            .await?;

        // Set units to millimeters (1 = mm)
        self.send_command(&format!("{}SN1", self.axis)).await?;

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
impl Stage for ESP300V3 {
    async fn move_absolute(&mut self, position_mm: f64) -> Result<()> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        // Check position limits
        let min = self.min_position_mm.read().await.get();
        let max = self.max_position_mm.read().await.get();

        if position_mm < min || position_mm > max {
            return Err(anyhow!(
                "Position {} mm out of range [{}, {}]",
                position_mm,
                min,
                max
            ));
        }

        // Send move absolute command
        self.send_command(&format!("{}PA{}", self.axis, position_mm))
            .await?;

        // Update cached position
        {
            let mut cached = self.cached_position.write().await;
            *cached = position_mm;
        }

        // Broadcast position update
        self.update_position().await?;

        Ok(())
    }

    async fn move_relative(&mut self, distance_mm: f64) -> Result<()> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        // Send move relative command
        self.send_command(&format!("{}PR{}", self.axis, distance_mm))
            .await?;

        // Update cached position
        {
            let mut cached = self.cached_position.write().await;
            *cached += distance_mm;
        }

        // Broadcast position update
        self.update_position().await?;

        Ok(())
    }

    async fn position(&self) -> Result<f64> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        // Query position
        self.send_command(&format!("{}TP?", self.axis)).await?;
        let response = self.read_response().await?;

        let position: f64 = response
            .trim()
            .parse()
            .map_err(|e| anyhow!("Failed to parse position '{}': {}", response, e))?;

        // Update cached position
        let mut cached = self.cached_position.write().await;
        *cached = position;

        Ok(position)
    }

    async fn stop_motion(&mut self) -> Result<()> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        self.send_command(&format!("{}ST", self.axis)).await?;
        Ok(())
    }

    async fn is_moving(&self) -> Result<bool> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        // Query motion done status
        self.send_command(&format!("{}MD?", self.axis)).await?;
        let response = self.read_response().await?;

        // MD? returns 0 if motion done, non-zero if moving
        let status: i32 = response
            .trim()
            .parse()
            .map_err(|e| anyhow!("Failed to parse motion status '{}': {}", response, e))?;

        Ok(status != 0)
    }

    async fn home(&mut self) -> Result<()> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        // Send home/origin search command
        self.send_command(&format!("{}OR", self.axis)).await?;

        // Wait for homing to complete
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Update position
        self.update_position().await?;

        Ok(())
    }

    async fn set_velocity(&mut self, mm_per_sec: f64) -> Result<()> {
        if self.state == InstrumentState::Uninitialized {
            return Err(anyhow!("Stage not initialized"));
        }

        // Validate and set parameter
        self.velocity_mm_s.write().await.set(mm_per_sec).await?;

        // Send to hardware
        self.send_command(&format!("{}VA{}", self.axis, mm_per_sec))
            .await?;

        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_esp300_v3_initialization() {
        let mut stage = ESP300V3::new("test_stage", "/dev/tty.mock", ESP300SdkKind::Mock, 1);
        assert_eq!(stage.state(), InstrumentState::Uninitialized);

        stage.initialize().await.unwrap();
        assert_eq!(stage.state(), InstrumentState::Idle);
    }

    #[tokio::test]
    async fn test_esp300_v3_absolute_move() {
        let mut stage = ESP300V3::new("test_stage", "/dev/tty.mock", ESP300SdkKind::Mock, 1);
        stage.initialize().await.unwrap();

        // Move to position
        stage.move_absolute(50.0).await.unwrap();

        // Verify position
        let pos = stage.position().await.unwrap();
        assert!((pos - 50.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_esp300_v3_relative_move() {
        let mut stage = ESP300V3::new("test_stage", "/dev/tty.mock", ESP300SdkKind::Mock, 1);
        stage.initialize().await.unwrap();

        // Move to known position
        stage.move_absolute(10.0).await.unwrap();

        // Move relative
        stage.move_relative(5.0).await.unwrap();

        // Verify position
        let pos = stage.position().await.unwrap();
        assert!((pos - 15.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_esp300_v3_position_query() {
        let mut stage = ESP300V3::new("test_stage", "/dev/tty.mock", ESP300SdkKind::Mock, 1);
        stage.initialize().await.unwrap();

        // Query initial position
        let pos = stage.position().await.unwrap();
        assert!(pos >= 0.0);
    }

    #[tokio::test]
    async fn test_esp300_v3_motion_status() {
        let mut stage = ESP300V3::new("test_stage", "/dev/tty.mock", ESP300SdkKind::Mock, 1);
        stage.initialize().await.unwrap();

        // Check if moving (should be stationary initially)
        let moving = stage.is_moving().await.unwrap();
        assert!(!moving);
    }

    #[tokio::test]
    async fn test_esp300_v3_homing() {
        let mut stage = ESP300V3::new("test_stage", "/dev/tty.mock", ESP300SdkKind::Mock, 1);
        stage.initialize().await.unwrap();

        // Move away from zero
        stage.move_absolute(50.0).await.unwrap();

        // Home the stage
        stage.home().await.unwrap();

        // Verify at home position
        let pos = stage.position().await.unwrap();
        assert!((pos - 0.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_esp300_v3_parameter_validation() {
        let mut stage = ESP300V3::new("test_stage", "/dev/tty.mock", ESP300SdkKind::Mock, 1);
        stage.initialize().await.unwrap();

        // Invalid position (out of range)
        let result = stage.move_absolute(200.0).await;
        assert!(result.is_err());

        // Valid position
        stage.move_absolute(50.0).await.unwrap();
    }

    #[tokio::test]
    async fn test_esp300_v3_shutdown() {
        let mut stage = ESP300V3::new("test_stage", "/dev/tty.mock", ESP300SdkKind::Mock, 1);
        stage.initialize().await.unwrap();

        stage.shutdown().await.unwrap();
        assert_eq!(stage.state(), InstrumentState::ShuttingDown);
    }
}
