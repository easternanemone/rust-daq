Based on the rust-daq project structure and the provided instrument manuals, here is a comprehensive set of hardware modules.

These implementations are designed to fit into src/hardware/ and implement the capability traits found in src/hardware/capabilities.rs. I have improved the existing implementations (found in the context) by adding critical features found in the manuals, such as error checking, warmup monitoring, and wavelength calibration.

1. Thorlabs Elliptec ELL14 Driver (src/hardware/ell14.rs)

This module implements the Movable trait. I have enhanced it to check the Status/Error codes defined in the manual  rather than just the "Moving" bit.

Rust
//! Thorlabs Elliptec ELL14 Rotation Mount Driver
//!
//! Reference: ELLx modules protocol manual Issue 7-6
//!
//! Protocol Overview:
//! - Format: [Address][Command][Data] [cite: 66]
//! - Error handling: Status byte returned in 'GS' response 
//! - Timing: 2 second timeout [cite: 73]

use crate::hardware::capabilities::Movable;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

pub struct Ell14Driver {
    port: Mutex<SerialStream>,
    address: String,
    pulses_per_degree: f64,
}

impl Ell14Driver {
    pub fn new(port_path: &str, address: &str) -> Result<Self> {
        let port = tokio_serial::new(port_path, 9600)
            .open_native_async()
            .context("Failed to open ELL14 serial port")?;

        Ok(Self {
            port: Mutex::new(port),
            address: address.to_string(),
            pulses_per_degree: 398.2222, // 143360 pulses / 360 degrees [cite: 109]
        })
    }

    pub async fn home(&self) -> Result<()> {
        // "ho" command: Homing [cite: 547]
        let _ = self.transaction("ho").await?;
        self.wait_settled().await
    }

    async fn transaction(&self, command: &str) -> Result<String> {
        let mut port = self.port.lock().await;
        
        // Protocol: {Address}{Command} [cite: 81]
        let payload = format!("{}{}", self.address, command);
        port.write_all(payload.as_bytes()).await
            .context("ELL14 write failed")?;

        let mut buf = [0u8; 1024];
        let read_len = tokio::time::timeout(Duration::from_millis(500), port.read(&mut buf))
            .await
            .context("ELL14 read timeout")?
            .context("ELL14 read error")?;

        let response = std::str::from_utf8(&buf[..read_len])
            .context("Invalid UTF-8 from ELL14")?
            .trim();
            
        // Check for status/error response "GS" [cite: 164]
        if let Some(idx) = response.find("GS") {
             let hex_str = &response[idx+2..].trim();
             if let Ok(status) = u32::from_str_radix(hex_str, 16) {
                 self.check_error(status)?;
             }
        }

        Ok(response.to_string())
    }
    
    /// Check Status/Error codes 
    fn check_error(&self, status: u32) -> Result<()> {
        match status {
            0 => Ok(()), // OK, no error
            1 => Err(anyhow!("ELL14 Communication timeout")),
            2 => Err(anyhow!("ELL14 Mechanical timeout")),
            3 => Err(anyhow!("ELL14 Command error")),
            4 => Err(anyhow!("ELL14 Value out of range")),
            5 => Err(anyhow!("ELL14 Module isolated")),
            6 => Err(anyhow!("ELL14 Module out of isolation")),
            7 => Err(anyhow!("ELL14 Initialization error")),
            8 => Err(anyhow!("ELL14 Thermal error")),
            9 => Ok(()), // Busy is not an error condition for this check
            10 => Err(anyhow!("ELL14 Sensor Error")),
            11 => Err(anyhow!("ELL14 Motor Error")),
            12 => Err(anyhow!("ELL14 Out of Range")),
            13 => Err(anyhow!("ELL14 Over Current")),
            _ => Err(anyhow!("ELL14 Unknown error: {}", status)),
        }
    }

    fn parse_position_response(&self, response: &str) -> Result<f64> {
        // Format: {Address}PO{8-char Hex} [cite: 566]
        if let Some(idx) = response.find("PO") {
             let hex_str = &response[idx+2..].trim();
             let hex_clean = if hex_str.len() > 8 { &hex_str[..8] } else { hex_str };
             
             let pulses = i32::from_str_radix(hex_clean, 16)
                .context("Failed to parse position hex")?;
             return Ok(pulses as f64 / self.pulses_per_degree);
        }
        Err(anyhow!("Invalid position response: {}", response))
    }
}

#[async_trait]
impl Movable for Ell14Driver {
    async fn move_abs(&self, position_deg: f64) -> Result<()> {
        let pulses = (position_deg * self.pulses_per_degree) as i32;
        // "ma" command: Move Absolute [cite: 606]
        self.transaction(&format!("ma{:08X}", pulses)).await?;
        Ok(())
    }

    async fn move_rel(&self, distance_deg: f64) -> Result<()> {
        let pulses = (distance_deg * self.pulses_per_degree) as i32;
        // "mr" command: Move Relative [cite: 637]
        self.transaction(&format!("mr{:08X}", pulses)).await?;
        Ok(())
    }

    async fn position(&self) -> Result<f64> {
        // "gp" command: Get Position [cite: 841]
        let resp = self.transaction("gp").await?;
        self.parse_position_response(&resp)
    }

    async fn wait_settled(&self) -> Result<()> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(15) {
                return Err(anyhow!("ELL14 timeout"));
            }
            // "gs" command: Get Status [cite: 145]
            let resp = self.transaction("gs").await?;
            // Bit 0 of status indicates "Moving" [cite: 170] is implied by code 9 "Busy"
            if !resp.contains("GS09") { // 09 is Busy
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
2. Spectra-Physics Mai Tai Driver (src/hardware/maitai.rs)

I have added the read_warmup_status method, as the manual states the laser cannot be turned on until warmup is 100%.

Rust
//! Spectra-Physics MaiTai Driver
//!
//! Reference: Mai Tai User's Manual
//! Protocol: ASCII over RS-232, 9600 8N1 [cite: 2947]

use crate::hardware::capabilities::Readable;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

pub struct MaiTaiDriver {
    port: Mutex<BufReader<SerialStream>>,
}

impl MaiTaiDriver {
    pub fn new(port_path: &str) -> Result<Self> {
        let port = tokio_serial::new(port_path, 9600)
            .data_bits(tokio_serial::DataBits::Eight)
            .parity(tokio_serial::Parity::None)
            .stop_bits(tokio_serial::StopBits::One)
            .flow_control(tokio_serial::FlowControl::Software) // XON/XOFF [cite: 2948]
            .open_native_async()?;

        Ok(Self {
            port: Mutex::new(BufReader::new(port)),
        })
    }

    async fn send_command(&self, cmd: &str) -> Result<()> {
        let mut port = self.port.lock().await;
        // Terminators: CR or LF. Responses end in LF [cite: 2957]
        port.get_mut().write_all(format!("{}\r", cmd).as_bytes()).await?;
        Ok(())
    }

    async fn query(&self, cmd: &str) -> Result<String> {
        let mut port = self.port.lock().await;
        port.get_mut().write_all(format!("{}\r", cmd).as_bytes()).await?;
        
        let mut response = String::new();
        tokio::time::timeout(Duration::from_secs(2), port.read_line(&mut response))
            .await??;
            
        Ok(response.trim().to_string())
    }

    /// Check system warmup status [cite: 3083]
    /// Returns percentage (0-100). Laser can only be turned ON when 100%.
    pub async fn get_warmup_status(&self) -> Result<f64> {
        let resp = self.query("READ:PCTWarmedup?").await?;
        // Response format: "050%"
        let val = resp.trim_matches('%').trim();
        val.parse::<f64>().context("Failed to parse warmup %")
    }

    pub async fn set_wavelength(&self, nm: f64) -> Result<()> {
        // Range 750-850 or 780-920 depending on model [cite: 2772]
        self.send_command(&format!("WAVelength {:.0}", nm)).await
    }
    
    pub async fn get_wavelength(&self) -> Result<f64> {
        let resp = self.query("READ:WAVelength?").await?;
        resp.parse().context("Failed to parse wavelength")
    }

    pub async fn set_shutter(&self, open: bool) -> Result<()> {
        // SHUTter 1 (open), SHUTter 0 (close) [cite: 3108]
        let val = if open { 1 } else { 0 };
        self.send_command(&format!("SHUTter {}", val)).await
    }

    pub async fn set_laser_on(&self, on: bool) -> Result<()> {
        // ON/OFF commands [cite: 3036]
        if on {
            // Check warmup first
            if self.get_warmup_status().await? < 100.0 {
                return Err(anyhow!("Laser not warmed up"));
            }
            self.send_command("ON").await
        } else {
            self.send_command("OFF").await
        }
    }
}

#[async_trait]
impl Readable for MaiTaiDriver {
    async fn read(&self) -> Result<f64> {
        // READ:POWer? returns 0 to 2.00 W [cite: 3100]
        let resp = self.query("READ:POWer?").await?;
        resp.parse().context("Failed to parse power")
    }
}
3. Newport 1830-C Driver (src/hardware/newport_1830c.rs)

The manual highlights the necessity of setting the wavelength and using ZERO for background subtraction.


Rust
//! Newport 1830-C Optical Power Meter Driver
//!
//! Reference: Newport 1830-C User's Manual
//! Protocol: ASCII, 9600 8N1 [cite: 4548]
//! Terminator: LF (\n) [cite: 4553]

use crate::hardware::capabilities::Readable;
use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

pub struct Newport1830CDriver {
    port: Mutex<BufReader<SerialStream>>,
}

impl Newport1830CDriver {
    pub fn new(port_path: &str) -> Result<Self> {
        let port = tokio_serial::new(port_path, 9600)
            .open_native_async()?;
        Ok(Self {
            port: Mutex::new(BufReader::new(port)),
        })
    }

    async fn send_cmd(&self, cmd: &str) -> Result<()> {
        let mut port = self.port.lock().await;
        // Terminator is LF [cite: 4553]
        port.get_mut().write_all(format!("{}\n", cmd).as_bytes()).await?;
        Ok(())
    }
    
    async fn query(&self, cmd: &str) -> Result<String> {
        let mut port = self.port.lock().await;
        port.get_mut().write_all(format!("{}\n", cmd).as_bytes()).await?;
        
        let mut response = String::new();
        tokio::time::timeout(std::time::Duration::from_secs(1), port.read_line(&mut response))
            .await??;
        Ok(response.trim().to_string())
    }

    /// Set calibration wavelength [cite: 5223]
    pub async fn set_wavelength(&self, nm: usize) -> Result<()> {
        self.send_cmd(&format!("W{}", nm)).await
    }

    /// Set Attenuator mode (A1 = on, A0 = off) [cite: 4862]
    pub async fn set_attenuator(&self, on: bool) -> Result<()> {
        let val = if on { 1 } else { 0 };
        self.send_cmd(&format!("A{}", val)).await
    }

    /// Zero the meter (subtract background) [cite: 5255]
    pub async fn zero(&self) -> Result<()> {
        self.send_cmd("Z1").await
    }
}

#[async_trait]
impl Readable for Newport1830CDriver {
    async fn read(&self) -> Result<f64> {
        // D? command returns data [cite: 4936]
        // Format: ±d.ddddE±dd [cite: 4943]
        let resp = self.query("D?").await?;
        resp.parse().context("Failed to parse power reading")
    }
}
4. Newport ESP300 Driver (src/hardware/esp300.rs)

I have implemented the standard motion commands PA, TP and incorporated the error checking command TB?  to ensure safe operation.

Rust
//! Newport ESP300 Motion Controller Driver
//!
//! Reference: ESP300 Motion Controller/Driver User's Manual
//! Protocol: ASCII, 19200 8N1 [cite: 6129]

use crate::hardware::capabilities::Movable;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tokio_serial::{SerialPortBuilderExt, SerialStream};

pub struct Esp300Driver {
    port: Mutex<BufReader<SerialStream>>,
    axis: u8,
}

impl Esp300Driver {
    pub fn new(port_path: &str, axis: u8) -> Result<Self> {
        // ESP300 supports axes 1 to 3 [cite: 6112]
        if !(1..=3).contains(&axis) {
            return Err(anyhow!("Invalid axis number"));
        }
        
        let port = tokio_serial::new(port_path, 19200) // Default baud 19200 [cite: 6129]
            .open_native_async()?;
            
        Ok(Self {
            port: Mutex::new(BufReader::new(port)),
            axis,
        })
    }

    async fn send_cmd(&self, cmd: &str) -> Result<()> {
        let mut port = self.port.lock().await;
        // Command terminator is CR [cite: 6662]
        port.get_mut().write_all(format!("{}\r", cmd).as_bytes()).await?;
        Ok(())
    }

    async fn query(&self, cmd: &str) -> Result<String> {
        let mut port = self.port.lock().await;
        port.get_mut().write_all(format!("{}\r", cmd).as_bytes()).await?;
        
        let mut response = String::new();
        // Response terminator is CR/LF [cite: 6680]
        tokio::time::timeout(Duration::from_secs(1), port.read_line(&mut response))
            .await??;
        Ok(response.trim().to_string())
    }

    pub async fn motor_on(&self) -> Result<()> {
        self.send_cmd(&format!("{}MO", self.axis)).await // [cite: 7722]
    }

    pub async fn set_velocity(&self, units_per_sec: f64) -> Result<()> {
        self.send_cmd(&format!("{}VA{}", self.axis, units_per_sec)).await // [cite: 8257]
    }
    
    /// Check for errors 
    pub async fn check_errors(&self) -> Result<()> {
        let resp = self.query("TB?").await?;
        if !resp.starts_with('0') {
            return Err(anyhow!("ESP300 Error: {}", resp));
        }
        Ok(())
    }
}

#[async_trait]
impl Movable for Esp300Driver {
    async fn move_abs(&self, position: f64) -> Result<()> {
        // PA: Position Absolute [cite: 7811]
        self.send_cmd(&format!("{}PA{}", self.axis, position)).await
    }

    async fn move_rel(&self, distance: f64) -> Result<()> {
        // PR: Position Relative [cite: 7863]
        self.send_cmd(&format!("{}PR{}", self.axis, distance)).await
    }

    async fn position(&self) -> Result<f64> {
        // TP: Tell Position [cite: 8191]
        let resp = self.query(&format!("{}TP", self.axis)).await?;
        resp.parse().context("Failed to parse position")
    }

    async fn wait_settled(&self) -> Result<()> {
        // MD: Motion Done? 1=Done, 0=Moving [cite: 7704]
        loop {
            let resp = self.query(&format!("{}MD?", self.axis)).await?;
            if resp.trim() == "1" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Ok(())
    }
}
