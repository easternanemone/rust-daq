# Instrument Control and Integration Guide

## Overview

This guide covers implementing instrument control capabilities for the scientific data acquisition application, including SCPI protocol support, serial communication, USB/Ethernet interfaces, and plugin architecture for different instrument types.

## Core Instrument Architecture

### Base Instrument Trait
```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[async_trait]
pub trait Instrument: Send + Sync {
    type Config: Serialize + for<'de> Deserialize<'de> + Send + Clone;
    type Data: Send + Clone + Serialize + for<'de> Deserialize<'de>;
    type Error: std::error::Error + Send + Sync + 'static;

    /// Initialize the instrument with given configuration
    async fn initialize(&mut self, config: Self::Config) -> Result<(), Self::Error>;

    /// Configure instrument parameters
    async fn configure(&mut self, params: HashMap<String, serde_json::Value>) -> Result<(), Self::Error>;

    /// Start data acquisition
    async fn start_acquisition(&mut self) -> Result<(), Self::Error>;

    /// Stop data acquisition
    async fn stop_acquisition(&mut self) -> Result<(), Self::Error>;

    /// Read data from the instrument
    async fn read_data(&mut self) -> Result<Self::Data, Self::Error>;

    /// Send a command to the instrument
    async fn send_command(&mut self, command: &str) -> Result<String, Self::Error>;

    /// Check if instrument is connected
    async fn is_connected(&self) -> bool;

    /// Get instrument status
    async fn get_status(&self) -> Result<InstrumentStatus, Self::Error>;

    /// Shutdown the instrument connection
    async fn shutdown(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentStatus {
    pub connected: bool,
    pub acquiring: bool,
    pub error_state: Option<String>,
    pub last_data_time: Option<std::time::SystemTime>,
    pub data_rate: f64, // samples per second
}
```

### SCPI Instrument Implementation
```rust
use scpi::prelude::*;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::time::Duration;

pub struct ScpiInstrument {
    connection: Option<TcpStream>,
    config: ScpiConfig,
    status: InstrumentStatus,
    command_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScpiConfig {
    pub address: String,
    pub port: u16,
    pub timeout_ms: u64,
    pub termination: String, // "\n", "\r\n", etc.
    pub encoding: String,    // "ascii", "utf8"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScpiData {
    pub timestamp: std::time::SystemTime,
    pub values: Vec<f64>,
    pub units: Vec<String>,
    pub channels: Vec<String>,
}

impl ScpiInstrument {
    pub fn new(config: ScpiConfig) -> Self {
        Self {
            connection: None,
            config,
            status: InstrumentStatus::default(),
            command_timeout: Duration::from_millis(1000),
        }
    }

    async fn connect(&mut self) -> Result<(), ScpiError> {
        let addr = format!("{}:{}", self.config.address, self.config.port);
        let stream = TcpStream::connect(&addr).await?;
        
        self.connection = Some(stream);
        self.status.connected = true;
        
        // Send identification query
        let idn = self.query("*IDN?").await?;
        tracing::info!("Connected to instrument: {}", idn);
        
        Ok(())
    }

    async fn query(&mut self, command: &str) -> Result<String, ScpiError> {
        let stream = self.connection.as_mut()
            .ok_or(ScpiError::NotConnected)?;

        // Send command
        let cmd_with_term = format!("{}{}", command, self.config.termination);
        stream.write_all(cmd_with_term.as_bytes()).await?;

        // Read response
        let mut buffer = vec![0u8; 1024];
        let timeout = tokio::time::timeout(self.command_timeout, stream.read(&mut buffer)).await??;
        
        let response = String::from_utf8_lossy(&buffer[..timeout])
            .trim()
            .to_string();
        
        Ok(response)
    }

    async fn write(&mut self, command: &str) -> Result<(), ScpiError> {
        let stream = self.connection.as_mut()
            .ok_or(ScpiError::NotConnected)?;

        let cmd_with_term = format!("{}{}", command, self.config.termination);
        stream.write_all(cmd_with_term.as_bytes()).await?;
        
        Ok(())
    }
}

#[async_trait]
impl Instrument for ScpiInstrument {
    type Config = ScpiConfig;
    type Data = ScpiData;
    type Error = ScpiError;

    async fn initialize(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        self.config = config;
        self.connect().await?;
        
        // Perform instrument-specific initialization
        self.write("*RST").await?; // Reset
        self.write("*CLS").await?; // Clear status
        
        Ok(())
    }

    async fn configure(&mut self, params: HashMap<String, serde_json::Value>) -> Result<(), Self::Error> {
        for (param, value) in params {
            match param.as_str() {
                "sample_rate" => {
                    if let Some(rate) = value.as_f64() {
                        self.write(&format!("SAMP:RATE {}", rate)).await?;
                    }
                }
                "range" => {
                    if let Some(range) = value.as_f64() {
                        self.write(&format!("VOLT:RANG {}", range)).await?;
                    }
                }
                "trigger_source" => {
                    if let Some(source) = value.as_str() {
                        self.write(&format!("TRIG:SOUR {}", source)).await?;
                    }
                }
                _ => {
                    tracing::warn!("Unknown parameter: {}", param);
                }
            }
        }
        
        Ok(())
    }

    async fn start_acquisition(&mut self) -> Result<(), Self::Error> {
        self.write("INIT").await?;
        self.status.acquiring = true;
        Ok(())
    }

    async fn stop_acquisition(&mut self) -> Result<(), Self::Error> {
        self.write("ABOR").await?;
        self.status.acquiring = false;
        Ok(())
    }

    async fn read_data(&mut self) -> Result<Self::Data, Self::Error> {
        let response = self.query("FETC?").await?;
        
        // Parse SCPI response (comma-separated values)
        let values: Result<Vec<f64>, _> = response
            .split(',')
            .map(|s| s.trim().parse::<f64>())
            .collect();
        
        let values = values.map_err(|e| ScpiError::ParseError(e.to_string()))?;
        
        Ok(ScpiData {
            timestamp: std::time::SystemTime::now(),
            values,
            units: vec!["V".to_string(); values.len()],
            channels: (0..values.len()).map(|i| format!("CH{}", i + 1)).collect(),
        })
    }

    async fn send_command(&mut self, command: &str) -> Result<String, Self::Error> {
        if command.ends_with('?') {
            self.query(command).await
        } else {
            self.write(command).await?;
            Ok(String::new())
        }
    }

    async fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    async fn get_status(&self) -> Result<InstrumentStatus, Self::Error> {
        Ok(self.status.clone())
    }

    async fn shutdown(&mut self) -> Result<(), Self::Error> {
        if let Some(mut stream) = self.connection.take() {
            let _ = stream.shutdown().await;
        }
        self.status.connected = false;
        self.status.acquiring = false;
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ScpiError {
    #[error("Not connected to instrument")]
    NotConnected,
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Timeout error")]
    Timeout(#[from] tokio::time::error::Elapsed),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Command error: {0}")]
    CommandError(String),
}
```

### Serial Instrument Implementation
```rust
use serialport::{SerialPort, SerialPortBuilder};
use std::time::Duration;

pub struct SerialInstrument {
    port: Option<Box<dyn SerialPort>>,
    config: SerialConfig,
    status: InstrumentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: serialport::DataBits,
    pub parity: serialport::Parity,
    pub stop_bits: serialport::StopBits,
    pub flow_control: serialport::FlowControl,
    pub timeout: Duration,
}

impl SerialInstrument {
    pub fn new(config: SerialConfig) -> Self {
        Self {
            port: None,
            config,
            status: InstrumentStatus::default(),
        }
    }

    async fn connect(&mut self) -> Result<(), SerialError> {
        let port = serialport::new(&self.config.port_name, self.config.baud_rate)
            .data_bits(self.config.data_bits)
            .parity(self.config.parity)
            .stop_bits(self.config.stop_bits)
            .flow_control(self.config.flow_control)
            .timeout(self.config.timeout)
            .open()?;

        self.port = Some(port);
        self.status.connected = true;
        
        Ok(())
    }

    async fn write_read(&mut self, command: &str) -> Result<String, SerialError> {
        let port = self.port.as_mut()
            .ok_or(SerialError::NotConnected)?;

        // Write command
        port.write_all(command.as_bytes())?;
        port.write_all(b"\r\n")?;

        // Read response
        let mut buffer = vec![0u8; 1024];
        let mut response = String::new();
        
        loop {
            match port.read(&mut buffer) {
                Ok(bytes_read) => {
                    let chunk = String::from_utf8_lossy(&buffer[..bytes_read]);
                    response.push_str(&chunk);
                    
                    if response.contains('\n') {
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    break;
                }
                Err(e) => return Err(SerialError::Io(e)),
            }
        }
        
        Ok(response.trim().to_string())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SerialError {
    #[error("Not connected to instrument")]
    NotConnected,
    
    #[error("Serial port error: {0}")]
    SerialPort(#[from] serialport::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Mock Instrument for Testing
```rust
use rand::Rng;
use std::time::{Duration, SystemTime};

pub struct MockInstrument {
    config: MockConfig,
    status: InstrumentStatus,
    sample_count: usize,
    start_time: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockConfig {
    pub channels: usize,
    pub sample_rate: f64,
    pub amplitude: f64,
    pub frequency: f64,
    pub noise_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockData {
    pub timestamp: SystemTime,
    pub values: Vec<f64>,
    pub sample_number: usize,
}

impl MockInstrument {
    pub fn new(config: MockConfig) -> Self {
        Self {
            config,
            status: InstrumentStatus {
                connected: false,
                acquiring: false,
                error_state: None,
                last_data_time: None,
                data_rate: 0.0,
            },
            sample_count: 0,
            start_time: None,
        }
    }

    fn generate_sample_data(&mut self) -> MockData {
        let mut rng = rand::thread_rng();
        let time_elapsed = self.start_time
            .map(|start| start.elapsed().unwrap_or_default().as_secs_f64())
            .unwrap_or(0.0);

        let mut values = Vec::new();
        
        for channel in 0..self.config.channels {
            // Generate sine wave with noise
            let phase = channel as f64 * std::f64::consts::PI / 4.0;
            let signal = self.config.amplitude * 
                (2.0 * std::f64::consts::PI * self.config.frequency * time_elapsed + phase).sin();
            
            let noise = rng.gen_range(-self.config.noise_level..self.config.noise_level);
            values.push(signal + noise);
        }

        self.sample_count += 1;
        
        MockData {
            timestamp: SystemTime::now(),
            values,
            sample_number: self.sample_count,
        }
    }
}

#[async_trait]
impl Instrument for MockInstrument {
    type Config = MockConfig;
    type Data = MockData;
    type Error = MockError;

    async fn initialize(&mut self, config: Self::Config) -> Result<(), Self::Error> {
        self.config = config;
        self.status.connected = true;
        
        // Simulate connection delay
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok(())
    }

    async fn start_acquisition(&mut self) -> Result<(), Self::Error> {
        self.status.acquiring = true;
        self.start_time = Some(SystemTime::now());
        self.sample_count = 0;
        Ok(())
    }

    async fn stop_acquisition(&mut self) -> Result<(), Self::Error> {
        self.status.acquiring = false;
        Ok(())
    }

    async fn read_data(&mut self) -> Result<Self::Data, Self::Error> {
        if !self.status.acquiring {
            return Err(MockError::NotAcquiring);
        }

        // Simulate acquisition time
        let sample_interval = Duration::from_secs_f64(1.0 / self.config.sample_rate);
        tokio::time::sleep(sample_interval).await;

        let data = self.generate_sample_data();
        self.status.last_data_time = Some(data.timestamp);
        self.status.data_rate = self.config.sample_rate;

        Ok(data)
    }

    async fn configure(&mut self, params: HashMap<String, serde_json::Value>) -> Result<(), Self::Error> {
        for (key, value) in params {
            match key.as_str() {
                "sample_rate" => {
                    if let Some(rate) = value.as_f64() {
                        self.config.sample_rate = rate;
                    }
                }
                "amplitude" => {
                    if let Some(amp) = value.as_f64() {
                        self.config.amplitude = amp;
                    }
                }
                "frequency" => {
                    if let Some(freq) = value.as_f64() {
                        self.config.frequency = freq;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn send_command(&mut self, command: &str) -> Result<String, Self::Error> {
        // Mock responses to common commands
        match command {
            "*IDN?" => Ok("Mock Instrument,Model 1234,SN12345,v1.0".to_string()),
            "*TST?" => Ok("0".to_string()),
            _ => Ok("OK".to_string()),
        }
    }

    async fn is_connected(&self) -> bool {
        self.status.connected
    }

    async fn get_status(&self) -> Result<InstrumentStatus, Self::Error> {
        Ok(self.status.clone())
    }

    async fn shutdown(&mut self) -> Result<(), Self::Error> {
        self.status.connected = false;
        self.status.acquiring = false;
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MockError {
    #[error("Instrument not acquiring data")]
    NotAcquiring,
    
    #[error("Configuration error: {0}")]
    Configuration(String),
}
```

### Instrument Manager
```rust
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;

pub struct InstrumentManager {
    instruments: HashMap<String, Box<dyn Instrument<Data = InstrumentData, Error = InstrumentError>>>,
    data_sender: mpsc::Sender<(String, InstrumentData)>,
    command_receiver: mpsc::Receiver<InstrumentCommand>,
}

#[derive(Debug, Clone)]
pub enum InstrumentCommand {
    Connect { instrument_id: String },
    Disconnect { instrument_id: String },
    StartAcquisition { instrument_id: String },
    StopAcquisition { instrument_id: String },
    Configure { instrument_id: String, params: HashMap<String, serde_json::Value> },
    SendCommand { instrument_id: String, command: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstrumentData {
    Scpi(ScpiData),
    Mock(MockData),
    // Add other instrument data types
}

impl InstrumentManager {
    pub fn new(
        data_sender: mpsc::Sender<(String, InstrumentData)>,
        command_receiver: mpsc::Receiver<InstrumentCommand>,
    ) -> Self {
        Self {
            instruments: HashMap::new(),
            data_sender,
            command_receiver,
        }
    }

    pub async fn run(&mut self) -> Result<(), InstrumentError> {
        while let Some(command) = self.command_receiver.recv().await {
            if let Err(e) = self.handle_command(command).await {
                tracing::error!("Error handling instrument command: {}", e);
            }
        }
        Ok(())
    }

    async fn handle_command(&mut self, command: InstrumentCommand) -> Result<(), InstrumentError> {
        match command {
            InstrumentCommand::Connect { instrument_id } => {
                if let Some(instrument) = self.instruments.get_mut(&instrument_id) {
                    // Configuration should come from config manager
                    instrument.initialize(/* config */).await?;
                }
            }
            InstrumentCommand::StartAcquisition { instrument_id } => {
                if let Some(instrument) = self.instruments.get_mut(&instrument_id) {
                    instrument.start_acquisition().await?;
                    
                    // Start data acquisition loop
                    self.start_data_loop(&instrument_id).await;
                }
            }
            // Handle other commands...
            _ => {}
        }
        Ok(())
    }

    async fn start_data_loop(&mut self, instrument_id: &str) {
        let instrument_id = instrument_id.to_string();
        let data_sender = self.data_sender.clone();
        
        tokio::spawn(async move {
            // This would need access to the instrument
            // Implementation depends on how you structure the async access
        });
    }
}
```

This instrument control guide provides a comprehensive foundation for integrating various types of scientific instruments into your Rust DAQ application, with support for multiple communication protocols and a flexible plugin architecture.