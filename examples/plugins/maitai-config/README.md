# MaiTai Config-Only Plugin

A config-only plugin example for the Spectra-Physics MaiTai Ti:Sapphire tunable laser.

## Overview

This example demonstrates creating a device driver plugin using **only configuration files** - no native code, scripts, or WASM required. The `GenericSerialDriver` interprets the TOML configuration to communicate with the hardware.

## What This Example Demonstrates

### Capabilities

Unlike the ELL14 example (which demonstrates `Movable`), this plugin showcases:

- **WavelengthTunable** - Set and query output wavelength (690-1040 nm)
- **ShutterControl** - Open/close beam shutter, query state
- **Readable** - Read laser output power

### Protocol Features

- **Software flow control (XON/XOFF)** - Required for MaiTai communication
- **Mixed terminators** - CR+LF for commands, LF for responses
- **Response parsing** - Handles formats like "820nm", "3.00W", "0"/"1"

## Files

| File | Purpose |
|------|---------|
| `plugin.toml` | Plugin manifest with metadata and module definition |
| `device.toml` | Complete MaiTai protocol specification |
| `README.md` | This documentation |

## Usage

### Loading the Plugin

```rust
use daq_hardware::plugin::discovery::PluginRegistry;

let mut registry = PluginRegistry::new();
registry.add_search_path("examples/plugins/");
registry.scan();

// Find the MaiTai plugin
let maitai = registry.get_latest("maitai-laser").unwrap();
println!("Loaded: {} v{}", maitai.name(), maitai.version);
```

### Creating a Driver Instance

```rust
use daq_hardware::plugin::driver::GenericSerialDriver;
use std::path::Path;

let driver = GenericSerialDriver::from_config(
    Path::new("examples/plugins/maitai-config/device.toml"),
    "/dev/ttyUSB0",
).await?;
```

### Using WavelengthTunable Trait

```rust
use daq_hardware::capabilities::WavelengthTunable;

// Set wavelength
driver.set_wavelength(800.0).await?;

// Query wavelength
let current = driver.get_wavelength().await?;
println!("Current wavelength: {} nm", current);
```

### Using ShutterControl Trait

```rust
use daq_hardware::capabilities::ShutterControl;

// Open shutter
driver.open_shutter().await?;

// Check state
if driver.is_shutter_open().await? {
    println!("Shutter is open");
}

// Close shutter
driver.close_shutter().await?;
```

## Protocol Reference

### Connection Settings

| Setting | Value |
|---------|-------|
| Baud Rate | 9600 |
| Data Bits | 8 |
| Parity | None |
| Stop Bits | 1 |
| Flow Control | Software (XON/XOFF) |
| Timeout | 5000 ms |

### Commands

| Command | Template | Description |
|---------|----------|-------------|
| Set wavelength | `WAVELENGTH:{nm}` | Set output wavelength |
| Get wavelength | `WAVELENGTH?` | Query current wavelength |
| Open shutter | `SHUTter:1` | Open beam shutter |
| Close shutter | `SHUTter:0` | Close beam shutter |
| Get shutter | `SHUTTER?` | Query shutter state |
| Get power | `POWER?` | Query output power |

### Response Formats

- **Wavelength**: `820nm` or `820NM`
- **Shutter state**: `0` (closed) or `1` (open)
- **Power**: `3.00W`, `100mW`, or `50%`

## Comparison with ELL14 Example

| Feature | ELL14 | MaiTai |
|---------|-------|--------|
| Primary Capability | Movable | WavelengthTunable |
| Flow Control | None | Software (XON/XOFF) |
| Bus Type | RS-485 multidrop | Point-to-point |
| Response Format | Hex-encoded | ASCII with units |
| Calibration | Degrees ↔ Pulses | Direct nm values |

## Hardware Reference

- **Manufacturer**: Spectra-Physics (Newport/MKS)
- **Model**: MaiTai HP / MaiTai XF
- **Type**: Mode-locked Ti:Sapphire laser
- **Tuning Range**: 690-1040 nm
- **Manual**: MaiTai HP/XF User's Manual
