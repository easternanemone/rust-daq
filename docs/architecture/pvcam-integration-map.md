# PVCAM FFI Integration Map

This document maps all integration points between the PVCAM FFI bindings and the rust-daq ecosystem.

## Overview

The PVCAM driver (`daq-driver-pvcam`) provides Rust bindings for Teledyne's PVCAM C SDK, enabling control of Prime BSI and other Photometrics cameras. The integration uses a layered architecture with FFI bindings, a component layer, and trait implementations.

## Crate Dependencies

```
pvcam-sys (FFI bindings via bindgen)
    └── daq-driver-pvcam (driver implementation)
            ├── daq-hardware (HAL integration)
            ├── rust-daq (prelude re-export)
            └── daq-egui (GUI integration)
```

### Import Graph

| Crate | Import | Purpose |
|-------|--------|---------|
| `daq-hardware` | `daq_driver_pvcam::PvcamDriver` | Hardware registry integration |
| `rust-daq` | `daq_driver_pvcam` | Prelude re-export |
| `daq-egui` | `daq_driver_pvcam::PvcamDriver` | GUI camera panel |

## Trait Implementation Matrix

PvcamDriver implements the following capability traits from `daq-hardware`:

| Trait | Status | Notes |
|-------|--------|-------|
| `ExposureControl` | Implemented | Controls exposure time, modes |
| `Triggerable` | Implemented | Internal/external trigger support |
| `FrameProducer` | Implemented | Continuous acquisition, frame delivery |
| `MeasurementSource` | Implemented | Frame metadata as measurements |
| `Parameterized` | Implemented | Parameter discovery and access |
| `Commandable` | Implemented | Device commands (connect, disconnect) |

## Component Architecture

The driver is organized into components under `src/components/`:

```
PvcamDriver
    ├── PvcamConnection (connection.rs)
    │   ├── SDK initialization with reference counting
    │   ├── Camera open/close
    │   └── list_available_cameras() - Dynamic camera discovery
    │
    ├── PvcamFeatures (features.rs)
    │   ├── 30+ Parameter<T> fields for camera settings
    │   ├── list_exposure_modes() - Dynamic exposure mode discovery
    │   ├── list_clear_modes() - Dynamic clear mode discovery
    │   ├── list_expose_out_modes() - Dynamic expose-out mode discovery
    │   ├── list_speed_modes() - Speed table enumeration
    │   ├── list_readout_ports() - Port enumeration
    │   ├── list_gain_modes() - Gain table enumeration
    │   └── list_pp_features() / list_pp_params() - Post-processing
    │
    └── PvcamAcquisition (acquisition.rs)
        ├── Continuous acquisition loop
        ├── Frame callback handling (EOF callbacks)
        └── Ring buffer integration
```

## Parameter<T> Bindings

Key parameters exposed via `PvcamFeatures`:

### Temperature Control
- `temperature` - Current sensor temperature (read-only)
- `temperature_setpoint` - Target temperature (read-write)
- `fan_speed` - Cooling fan speed setting

### Exposure Settings
- `exposure_time` - Exposure duration in milliseconds
- `exposure_mode` - Timed, External, Trigger-first modes
- `clear_mode` - Sensor clearing strategy
- `expose_out_mode` - Expose output signal timing

### Readout Configuration
- `readout_port` - Port selection (if multiple available)
- `speed_index` - Readout speed selection
- `gain_index` - Gain/bit-depth selection
- `binning_serial` / `binning_parallel` - Pixel binning

### Shutter Control
- `shutter_mode` - Normal, open, closed
- `shutter_open_delay` / `shutter_close_delay` - Timing delays

### Smart Streaming
- `smart_stream_enabled` - Enable/disable smart streaming
- `smart_stream_mode` - Exposures or frames mode

## Dynamic Discovery Functions

These functions query hardware for available options at runtime:

| Function | PARAM_ID | Returns |
|----------|----------|---------|
| `list_available_cameras()` | N/A | `Vec<String>` camera names |
| `list_exposure_modes()` | `PARAM_EXPOSURE_MODE` | `Vec<(i32, String)>` |
| `list_clear_modes()` | `PARAM_CLEAR_MODE` | `Vec<(i32, String)>` |
| `list_expose_out_modes()` | `PARAM_EXPOSE_OUT_MODE` | `Vec<(i32, String)>` |
| `list_speed_modes()` | `PARAM_SPDTAB_INDEX` | `Vec<SpeedMode>` |
| `list_readout_ports()` | `PARAM_READOUT_PORT` | `Vec<ReadoutPort>` |
| `list_gain_modes()` | `PARAM_GAIN_INDEX` | `Vec<GainMode>` |
| `list_pp_features()` | `PARAM_PP_INDEX` | `Vec<PpFeature>` |

### Prime BSI Hardware Discovery Results

Actual values discovered on Prime BSI camera:

**Exposure Modes:**
- Internal Trigger (1792) - Camera controls timing
- Edge Trigger (2304) - External edge trigger
- Trigger first (2048) - Trigger on first frame

**Clear Modes:**
- Auto (only mode available)

**Expose Out Modes:**
- Rolling Shutter (3)
- First Row (0)
- Any Row (2)

## gRPC Integration

The driver integrates with `daq-server` via `HardwareService`:

```
HardwareService
    └── DeviceRegistry
            └── PvcamDriver instance
                    ├── get_parameter() / set_parameter()
                    ├── execute_command()
                    └── subscribe_measurements()
```

### Current gRPC Endpoints Affected
- `GetDevices` - Lists registered camera
- `GetDeviceParameters` - Returns parameter values
- `SetDeviceParameter` - Updates settings
- `ExecuteCommand` - Connect/disconnect commands
- `StreamMeasurements` - Frame data streaming

### Future gRPC Extensions
The new list functions are not yet exposed via gRPC. Potential additions:
- `ListAvailableCameras` RPC
- `ListExposureModes` RPC
- `ListClearModes` RPC
- `ListExposeOutModes` RPC

## Feature Flags

| Flag | Effect |
|------|--------|
| `pvcam_hardware` | Enable real hardware FFI calls |
| `pvcam-sdk` (pvcam-sys) | Generate bindgen FFI bindings |

Without `pvcam_hardware`, the driver operates in mock mode with simulated responses.

## SDK Pattern Compliance (bd-sk6z)

All parameter access follows the SDK pattern:
1. Check `is_param_available(handle, PARAM_ID)` first
2. Only access parameters if available
3. Return appropriate errors for unavailable parameters

This ensures compatibility across different camera models with varying feature sets.

## Related ADRs

- [adr-pvcam-driver-architecture.md](adr-pvcam-driver-architecture.md) - Driver design decisions
- [adr-pvcam-continuous-acquisition.md](adr-pvcam-continuous-acquisition.md) - Buffer modes and acquisition
- [adr-pvcam-sdk-pattern-compliance.md](adr-pvcam-sdk-pattern-compliance.md) - SDK compliance patterns
