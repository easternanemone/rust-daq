# GUI Design: PyMoDAQ/DynExp-Inspired Remote Interface

**Issue:** bd-cmgi
**Status:** Design Phase
**Dependencies:** bd-x9tp (Headless-First Remote Architecture)

---

## Major Architectural Issues (Must Address Before Implementation)

### Issue 1: No Device Registry

**Problem:** There is no central registry to manage hardware devices at runtime. The daemon can't enumerate what devices are connected or expose them to remote clients.

**Current State:**
- Hardware drivers exist (`ell14`, `esp300`, `pvcam`, `maitai`, `newport_1830c`)
- Each driver implements capability traits (`Movable`, `Readable`, etc.)
- No mechanism to instantiate, register, or discover devices dynamically

**Required:**
```rust
/// Central device registry for runtime management
pub struct DeviceRegistry {
    devices: HashMap<DeviceId, Arc<dyn Device>>,
}

trait Device: Send + Sync {
    fn id(&self) -> &DeviceId;
    fn name(&self) -> &str;
    fn capabilities(&self) -> Vec<Capability>;  // What traits it implements
    fn as_movable(&self) -> Option<&dyn Movable>;
    fn as_readable(&self) -> Option<&dyn Readable>;
    fn as_triggerable(&self) -> Option<&dyn Triggerable>;
    // etc.
}
```

**Impact:** Without this, GUI cannot discover available hardware.

---

### Issue 2: gRPC API Gap - No Direct Hardware Control

**Problem:** Current gRPC API (`daq.proto`) only handles script execution. There's no way for a remote GUI to directly control hardware.

**Current API:**
- `UploadScript`, `StartScript`, `StopScript` - Script lifecycle
- `StreamMeasurements` - Passive data streaming
- `GetDaemonInfo` - Returns hardcoded `["MockStage", "MockCamera"]`

**Missing API (Required for GUI):**
```protobuf
// HardwareService - Direct device control
service HardwareService {
  // Discovery
  rpc ListDevices(Empty) returns (ListDevicesResponse);
  rpc GetDevice(GetDeviceRequest) returns (DeviceInfo);

  // Motion Control (Movable devices)
  rpc MoveAbsolute(MoveRequest) returns (MoveResponse);
  rpc MoveRelative(MoveRequest) returns (MoveResponse);
  rpc GetPosition(PositionRequest) returns (PositionResponse);
  rpc StopMotion(StopMotionRequest) returns (Empty);
  rpc StreamPosition(StreamPositionRequest) returns (stream PositionUpdate);

  // Scalar Readout (Readable devices)
  rpc ReadValue(ReadRequest) returns (ReadResponse);
  rpc StreamValues(StreamValuesRequest) returns (stream ValueUpdate);

  // Camera Control (FrameProducer + ExposureControl)
  rpc SetExposure(SetExposureRequest) returns (Empty);
  rpc GetExposure(GetExposureRequest) returns (ExposureResponse);
  rpc StartAcquisition(StartAcquisitionRequest) returns (Empty);
  rpc StopAcquisition(StopAcquisitionRequest) returns (Empty);
  rpc StreamFrames(StreamFramesRequest) returns (stream Frame);

  // Trigger Control (Triggerable devices)
  rpc Arm(ArmRequest) returns (Empty);
  rpc Trigger(TriggerRequest) returns (Empty);
}
```

**Impact:** GUI would have to wrap all operations in Rhai scripts, which is inefficient and loses type safety.

---

### Issue 3: No Capability Introspection

**Problem:** Even if we add device listing, the GUI needs to know what each device can do to render appropriate controls.

**Required Data Structure:**
```protobuf
message DeviceInfo {
  string id = 1;
  string name = 2;
  string driver_type = 3;  // "ell14", "esp300", "pvcam", etc.

  // Capabilities as flags
  bool is_movable = 10;
  bool is_readable = 11;
  bool is_triggerable = 12;
  bool is_frame_producer = 13;
  bool is_exposure_controllable = 14;

  // Capability-specific metadata
  optional MovableInfo movable_info = 20;
  optional ReadableInfo readable_info = 21;
  optional FrameProducerInfo frame_producer_info = 22;
}

message MovableInfo {
  double min_position = 1;
  double max_position = 2;
  string units = 3;  // "mm", "degrees", etc.
  double resolution = 4;  // Minimum step size
}
```

**Impact:** Without introspection, GUI can't dynamically build device-appropriate panels.

---

### Issue 4: State Synchronization Protocol

**Problem:** GUI needs real-time updates of device state (positions, values, acquisition status) without polling.

**PyMoDAQ Pattern:** Uses Qt signals/slots for UI updates
**DynExp Pattern:** Event queues between modules and UI thread

**Required for rust-daq:**
- Server-side push for state changes
- Bidirectional streaming or server-sent events
- Subscription mechanism (subscribe to specific devices/channels)

**Proposed Protocol:**
```protobuf
// Subscribe to device state changes
rpc SubscribeDeviceState(SubscriptionRequest) returns (stream DeviceStateUpdate);

message SubscriptionRequest {
  repeated string device_ids = 1;  // Empty = all devices
  uint32 max_rate_hz = 2;  // Rate limiting
}

message DeviceStateUpdate {
  string device_id = 1;
  uint64 timestamp_ns = 2;
  oneof state {
    PositionState position = 10;
    ValueState value = 11;
    AcquisitionState acquisition = 12;
    ErrorState error = 13;
  }
}
```

---

### Issue 5: Scan Orchestration Architecture

**Problem:** PyMoDAQ's DAQ_Scan coordinates multi-axis scans with synchronized acquisition. This doesn't exist in rust-daq.

**Current State:**
- Scripts can implement scans manually in Rhai
- No reusable scan abstraction
- No scan progress tracking
- No scan configuration persistence

**Required Abstraction:**
```rust
/// Scan configuration
pub struct ScanConfig {
    /// Axes to scan (device_id + start/end/steps)
    pub axes: Vec<ScanAxis>,
    /// Detectors to acquire from
    pub detectors: Vec<DeviceId>,
    /// Acquisition settings per point
    pub acquisition: AcquisitionConfig,
    /// Data output configuration
    pub output: OutputConfig,
}

pub struct ScanAxis {
    pub device_id: DeviceId,
    pub start: f64,
    pub end: f64,
    pub steps: usize,
    pub mode: ScanMode,  // Linear, Bidirectional, Random
}
```

**gRPC API:**
```protobuf
service ScanService {
  rpc CreateScan(CreateScanRequest) returns (CreateScanResponse);
  rpc StartScan(StartScanRequest) returns (stream ScanProgress);
  rpc PauseScan(PauseScanRequest) returns (Empty);
  rpc ResumeScan(ResumeScanRequest) returns (Empty);
  rpc AbortScan(AbortScanRequest) returns (Empty);
  rpc GetScanResult(GetScanResultRequest) returns (ScanResult);
}
```

---

### Issue 6: Preset/Configuration System

**Problem:** No mechanism to save/load experimental configurations (device settings, scan parameters, GUI layout).

**PyMoDAQ Pattern:** XML-based presets stored per setup
**DynExp Pattern:** Project files containing all configurations

**Required:**
```rust
pub struct Preset {
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,

    /// Device configurations (positions, settings)
    pub devices: HashMap<DeviceId, DevicePreset>,

    /// Default scan configurations
    pub scans: Vec<ScanConfig>,
}
```

---

## Resolution Priority

| Issue | bd Issue | Priority | Blocks GUI? | Complexity |
|-------|----------|----------|-------------|------------|
| 1. Device Registry | `bd-h3si` | P0 | Yes | Medium |
| 2. Hardware Control API | `bd-4x6q` | P0 | Yes | High |
| 3. Capability Introspection | `bd-of3i` | P0 | Yes | Low |
| 4. State Synchronization | `bd-6uba` | P1 | Partial | Medium |
| 5. Scan Orchestration | `bd-4le6` | P1 | Yes (for DAQ_Scan) | High |
| 6. Preset System | `bd-akcm` | P2 | No | Medium |

**Dependency Chain:**
- GUI Epic (`bd-cmgi`) depends on Headless Architecture (`bd-x9tp`)
- All architectural issues block GUI implementation

---

## Overview

This document defines the GUI architecture for rust-daq, synthesizing best practices from two established data acquisition frameworks:

- **PyMoDAQ** - Python-based, Qt5/PyQtGraph, modular plugin architecture
- **DynExp** - C++ with Qt, three-tier abstraction, gRPC remote control

The GUI follows rust-daq's **headless-first principle**: the GUI is a remote client that connects to the headless daemon via gRPC.

## Framework Comparison

### PyMoDAQ Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Dashboard                        │
│  (Central coordination, presets, hardware grouping) │
├─────────────────┬───────────────────────────────────┤
│   DAQ_Move      │           DAQ_Viewer              │
│  (Actuators)    │         (Detectors)               │
├─────────────────┴───────────────────────────────────┤
│              DAQ_Scan / DAQ_Logger                  │
│  (Automated acquisition, parameter logging)         │
├─────────────────────────────────────────────────────┤
│                Plugin Layer                         │
│  (Hardware-specific implementations)                │
└─────────────────────────────────────────────────────┘
```

**Key Concepts:**
- **Dashboard**: Central hub that bundles actuators + detectors into experimental setups
- **DAQ_Move**: Independent module for motion control (stages, rotation mounts)
- **DAQ_Viewer**: Independent module for data acquisition (cameras, spectrometers)
- **DAQ_Scan**: Extension for automated scans (sweep parameters, acquire data)
- **Presets**: Saved configurations for reproducible experiments
- **Dynamic GUI Generation**: GUIs built programmatically from device capabilities

### DynExp Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Modules                           │
│  (Experiment controllers, hardware-agnostic)        │
├─────────────────────────────────────────────────────┤
│               Meta Instruments                      │
│  (AnalogOut, DataStreamInstrument, FunctionGen)     │
├─────────────────────────────────────────────────────┤
│            Hardware Instruments                     │
│  (NIDAQAnalogOut, ZurichMFLI, etc.)                │
├─────────────────────────────────────────────────────┤
│              Hardware Adapters                      │
│  (NIDAQ, VISA, Serial port access)                 │
└─────────────────────────────────────────────────────┘
```

**Key Concepts:**
- **Three-tier abstraction**: HardwareAdapters → Instruments → Modules
- **Meta instruments**: Hardware-agnostic interfaces (like rust-daq's capability traits)
- **Threaded task queues**: Each instrument runs its own event loop
- **Runtime assignment**: Instruments can be reassigned to modules without code changes
- **gRPC integration**: Language-agnostic remote control

## Synthesis: rust-daq GUI Architecture

### Design Principles

1. **Remote-First**: GUI connects to daemon via gRPC; no direct hardware access
2. **Capability-Driven UI**: GUI modules dynamically adapt based on device capabilities
3. **Modular Panels**: Independent panels for motion, detection, scanning
4. **State Synchronization**: Real-time bidirectional sync between GUI and daemon
5. **Preset Management**: Save/load experimental configurations
6. **Script Integration**: Execute and monitor Rhai scripts from GUI

### Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                        GUI Application                          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                     Dashboard Panel                       │  │
│  │  - Connected devices overview                            │  │
│  │  - Preset management (save/load configurations)          │  │
│  │  - Script execution controls                             │  │
│  │  - System status (memory, active scans)                  │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌────────────────────┐  ┌────────────────────────────────────┐│
│  │   Move Panel       │  │         Viewer Panel               ││
│  │  (Movable devices) │  │     (Readable/FrameProducer)       ││
│  │                    │  │                                    ││
│  │  - Position readout│  │  - Live value display              ││
│  │  - Jog controls    │  │  - Image viewer (cameras)          ││
│  │  - Move to target  │  │  - Waveform plots                  ││
│  │  - Limits display  │  │  - Trigger controls                ││
│  └────────────────────┘  └────────────────────────────────────┘│
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                      Scan Panel                          │  │
│  │  - Scan configuration (start, end, steps)                │  │
│  │  - Multi-axis scanning                                   │  │
│  │  - Live data visualization                               │  │
│  │  - Progress tracking                                     │  │
│  └──────────────────────────────────────────────────────────┘  │
├────────────────────────────────────────────────────────────────┤
│                      gRPC Client Layer                         │
│  - Bidirectional streaming for measurements                   │
│  - Unary calls for commands                                   │
│  - Connection management & reconnection                       │
└────────────────────────────────────────────────────────────────┘
                              │
                              │ gRPC (TCP/Unix Socket)
                              ▼
┌────────────────────────────────────────────────────────────────┐
│                    rust-daq Daemon                             │
│  (Headless server, hardware control, script execution)         │
└────────────────────────────────────────────────────────────────┘
```

### Mapping rust-daq Capabilities to GUI Panels

| Capability Trait   | GUI Panel    | PyMoDAQ Equiv | DynExp Equiv          |
|--------------------|--------------|---------------|----------------------|
| `Movable`          | Move Panel   | DAQ_Move      | PositionerModule     |
| `Readable`         | Viewer Panel | DAQ_Viewer    | DataStreamInstrument |
| `FrameProducer`    | Viewer Panel | DAQ_Viewer    | Camera module        |
| `Triggerable`      | Viewer Panel | DAQ_Viewer    | TriggerModule        |
| `ExposureControl`  | Viewer Panel | DAQ_Viewer    | Camera settings      |

### gRPC API Extensions Required

The current gRPC API supports script execution and measurement streaming. For GUI support, we need:

```protobuf
// Hardware Discovery & Control
service HardwareService {
  // List connected devices with their capabilities
  rpc ListDevices(ListDevicesRequest) returns (ListDevicesResponse);

  // Get device details (position, settings, status)
  rpc GetDeviceState(DeviceStateRequest) returns (DeviceStateResponse);

  // Move commands (for Movable devices)
  rpc MoveAbsolute(MoveRequest) returns (MoveResponse);
  rpc MoveRelative(MoveRequest) returns (MoveResponse);
  rpc StopMotion(StopMotionRequest) returns (StopMotionResponse);

  // Streaming position updates
  rpc StreamPosition(StreamPositionRequest) returns (stream PositionUpdate);

  // Trigger commands (for Triggerable devices)
  rpc Arm(ArmRequest) returns (ArmResponse);
  rpc Trigger(TriggerRequest) returns (TriggerResponse);

  // Exposure control
  rpc SetExposure(SetExposureRequest) returns (SetExposureResponse);

  // Frame streaming (for FrameProducer devices)
  rpc StreamFrames(StreamFramesRequest) returns (stream Frame);
}

// Scan Orchestration
service ScanService {
  // Configure and start a scan
  rpc CreateScan(CreateScanRequest) returns (CreateScanResponse);
  rpc StartScan(StartScanRequest) returns (StartScanResponse);
  rpc StopScan(StopScanRequest) returns (StopScanResponse);

  // Stream scan progress and data
  rpc StreamScanProgress(ScanProgressRequest) returns (stream ScanProgress);
}

// Preset Management
service PresetService {
  rpc SavePreset(SavePresetRequest) returns (SavePresetResponse);
  rpc LoadPreset(LoadPresetRequest) returns (LoadPresetResponse);
  rpc ListPresets(ListPresetsRequest) returns (ListPresetsResponse);
}
```

## Technology Stack Options

### Option A: Tauri + React/Vue (Recommended)

**Architecture**: Rust backend + Web frontend

```
┌─────────────────────────────────────┐
│        Tauri Application            │
│  ┌───────────────────────────────┐  │
│  │    React/Vue Frontend         │  │
│  │  - Modern UI components       │  │
│  │  - Real-time charts (Plotly)  │  │
│  │  - WebSocket for streaming    │  │
│  └───────────────────────────────┘  │
│  ┌───────────────────────────────┐  │
│  │    Rust Backend (Tauri)       │  │
│  │  - gRPC client to daemon      │  │
│  │  - Native performance         │  │
│  │  - Cross-platform             │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
```

**Pros:**
- Rust backend for type safety with daemon protos
- Modern web UI with extensive component libraries
- Cross-platform (Windows, macOS, Linux)
- Smaller binary than Electron
- Hot-reload for rapid UI development

**Cons:**
- Two language stacks (Rust + JS/TS)
- Less mature than Qt ecosystem

### Option B: egui (Pure Rust)

**Architecture**: Pure Rust with immediate-mode GUI

```
┌─────────────────────────────────────┐
│        egui Application             │
│  - Native Rust                      │
│  - Immediate-mode rendering         │
│  - egui_plot for visualization      │
│  - Direct gRPC integration          │
└─────────────────────────────────────┘
```

**Pros:**
- Single language (pure Rust)
- Direct integration with existing codebase
- Fast iteration with hot-reload
- Smaller cognitive overhead

**Cons:**
- Less polished UI components
- Limited widget ecosystem
- Custom styling more difficult
- Plotting capabilities less mature than web alternatives

### Option C: Slint (Declarative Rust GUI)

**Architecture**: Declarative UI with Rust backend

```
┌─────────────────────────────────────┐
│        Slint Application            │
│  ┌───────────────────────────────┐  │
│  │    .slint UI definitions      │  │
│  │  - Declarative, Qt-like       │  │
│  │  - Live preview               │  │
│  └───────────────────────────────┘  │
│  ┌───────────────────────────────┐  │
│  │    Rust Logic                 │  │
│  │  - gRPC client                │  │
│  │  - State management           │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
```

**Pros:**
- Declarative syntax (familiar to Qt/QML users)
- Good performance
- Live preview during development
- Growing ecosystem

**Cons:**
- Newer framework, smaller community
- Limited third-party components
- Plotting requires external integration

### Option D: Python + Qt (PyMoDAQ-Compatible)

**Architecture**: Python GUI with rust-daq as backend

```
┌─────────────────────────────────────┐
│     Python Qt Application           │
│  - PyQt5/PySide6                    │
│  - PyQtGraph for visualization      │
│  - grpcio for daemon communication  │
│  - Could share plugins with PyMoDAQ │
└─────────────────────────────────────┘
```

**Pros:**
- Maximum compatibility with PyMoDAQ ecosystem
- Mature plotting (PyQtGraph)
- Could potentially use PyMoDAQ plugins directly
- Large community

**Cons:**
- Separate language from daemon
- Python deployment complexity
- Performance limitations for high-rate visualization

## Recommendation

**Primary:** Option A (Tauri + React) for production GUI
- Best balance of UX polish, performance, and development velocity
- Cross-platform desktop application
- Modern charting libraries (Plotly, Chart.js, Apache ECharts)

**Secondary:** Option B (egui) for embedded/debug tools
- Quick visualization tools bundled with daemon
- Development/debugging utilities
- Low dependency footprint

## Implementation Phases

### Phase 1: gRPC API Extensions
1. Define protobuf messages for hardware control
2. Implement HardwareService in daemon
3. Add device capability introspection
4. Test with grpcurl/Postman

### Phase 2: Core GUI Framework
1. Set up Tauri project structure
2. Implement gRPC client in Rust backend
3. Create React component library
4. Build Dashboard panel

### Phase 3: Hardware Panels
1. Implement Move Panel (Movable devices)
2. Implement Viewer Panel (Readable/FrameProducer)
3. Real-time visualization (positions, values, frames)

### Phase 4: Scan System
1. Implement Scan configuration UI
2. Live scan progress visualization
3. Data export integration

### Phase 5: Preset & Script Integration
1. Preset save/load functionality
2. Script editor integration
3. Script execution monitoring

## References

- [PyMoDAQ Documentation](http://pymodaq.cnrs.fr/en/latest/)
- [PyMoDAQ GitHub](https://github.com/PyMoDAQ/PyMoDAQ)
- [DynExp Documentation](https://jbopp.github.io/dynexp/doc/)
- [Tauri Documentation](https://tauri.app/)
- [egui Documentation](https://github.com/emilk/egui)
- [Slint Documentation](https://slint.dev/)
